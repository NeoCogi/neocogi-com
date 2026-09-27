//! Cargo-specific loading, graph exchange, and generated-file ownership.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fmt, fs, io,
    path::{Path, PathBuf},
};

use crate::resolve_modules;
use serde::{Deserialize, Serialize};

use crate::generate::{RustModulePaths, generate_rust_with_paths_and_source_map};

mod source;
mod source_map;
pub use source::ModuleSources;

/// Declarative build request for exactly one contract module crate.
#[derive(Clone, Debug)]
pub struct Module {
    /// Directory containing the crate's IDL namespace tree.
    idl_root: PathBuf,
    /// File or directory that defines the logical module and its source parts.
    input: ModuleInput,
    /// Optional Rust location published to consumers, independent of IDL identity.
    rust_path: Option<String>,
}

/// Source layout selected for one logical contract module.
#[derive(Clone, Debug)]
enum ModuleInput {
    /// One legacy `.ncidl` file whose path defines module identity.
    File(PathBuf),
    /// One directory whose path defines identity and whose files are parts.
    Directory(PathBuf),
}

impl Module {
    /// Creates one module build request from a root and normalized source path.
    pub fn new(idl_root: impl Into<PathBuf>, source: impl Into<PathBuf>) -> Self {
        Self {
            idl_root: idl_root.into(),
            input: ModuleInput::File(source.into()),
            rust_path: None,
        }
    }

    /// Creates one module build request from every `.ncidl` part in a directory.
    ///
    /// The directory path relative to `idl_root` defines module identity. Part
    /// names, nesting, and filesystem enumeration order do not affect the ABI.
    pub fn directory(idl_root: impl Into<PathBuf>, module_directory: impl Into<PathBuf>) -> Self {
        Self {
            idl_root: idl_root.into(),
            input: ModuleInput::Directory(module_directory.into()),
            rust_path: None,
        }
    }

    /// Publishes declarations from a crate-rooted Rust module instead of a dedicated crate.
    ///
    /// For example, `neocogi_com::events` keeps the logical contract identity
    /// `neocogi.com.events` while exposing its bindings inside the COM facade.
    /// Validation occurs during build, before any generated output is written.
    pub fn rust_path(mut self, path: impl Into<String>) -> Self {
        self.rust_path = Some(path.into());
        self
    }

    /// Generates bindings and publishes the resolved graph to dependent builds.
    ///
    /// Cargo invokes this from the contract crate's `build.rs`. The crate must
    /// declare the exact conventionally derived `links` value; this prevents a
    /// package from advertising another module's generated artifacts.
    pub fn build(self) -> Result<BuildOutput, BuildError> {
        let manifest_dir = required_path("CARGO_MANIFEST_DIR")?;
        let output_dir = required_path("OUT_DIR")?;
        let source_root = manifest_dir.join(&self.idl_root);
        // Emit the selected ownership path before loading so a failed build still
        // tells Cargo which authored path should trigger the next attempt.
        let own_sources = match &self.input {
            ModuleInput::File(relative_path) => {
                println!("cargo::rerun-if-changed={}", source_root.join(relative_path).display());
                ModuleSources::load_file(&source_root, relative_path)?
            }
            ModuleInput::Directory(relative_directory) => {
                println!("cargo::rerun-if-changed={}", source_root.join(relative_directory).display());
                let sources = ModuleSources::load_directory(&source_root, relative_directory)?;
                for (relative_path, _) in sources.parts() {
                    println!("cargo::rerun-if-changed={}", source_root.join(relative_path).display());
                }
                sources
            }
        };
        let own_record = GraphModule {
            sources: own_sources,
            rust_path: self.rust_path,
        };

        let own_module = own_record.sources.parse()?;
        let module_name = own_module.name.to_string();
        let expected_links = links_name(&module_name);
        let actual_links = env::var("CARGO_MANIFEST_LINKS").map_err(|_| BuildError::MissingEnvironment("CARGO_MANIFEST_LINKS"))?;
        if actual_links != expected_links {
            return Err(BuildError::LinksMismatch {
                module: module_name,
                expected: expected_links,
                actual: actual_links,
            });
        }

        let mut records = BTreeMap::new();
        records.insert(own_module.name.to_string(), own_record);
        let dependency_modules = own_module.externals.iter().map(|external| external.module.clone()).collect::<BTreeSet<_>>();
        for dependency in dependency_modules {
            // Bundled source is injected and fingerprinted by the resolver; it
            // has no separately built Rust crate or Cargo DEP_* exchange file.
            if crate::is_bundled_module(&dependency) {
                continue;
            }
            let variable = dependency_graph_variable(&dependency);
            println!("cargo::rerun-if-env-changed={variable}");
            let graph_path = env::var_os(&variable).ok_or_else(|| BuildError::MissingDependency {
                module: dependency.clone(),
                variable: variable.clone(),
            })?;
            let graph_path = PathBuf::from(graph_path);
            let graph_source = fs::read_to_string(&graph_path).map_err(|source_error| BuildError::Io {
                operation: "read dependency IDL graph",
                path: graph_path.clone(),
                source: source_error,
            })?;
            let graph: GraphRecord = serde_json::from_str(&graph_source).map_err(|source_error| BuildError::Json {
                operation: "decode dependency IDL graph",
                path: graph_path,
                source: source_error,
            })?;
            if graph.schema != GRAPH_SCHEMA {
                return Err(BuildError::Idl(format!(
                    "dependency module {dependency:?} uses unsupported graph schema {}",
                    graph.schema
                )));
            }
            if !graph.modules.contains_key(&dependency) {
                return Err(BuildError::Idl(format!("dependency graph for {dependency:?} does not contain that module")));
            }
            merge_records(&mut records, graph.modules)?;
        }

        let parsed = records
            .iter()
            .map(|(name, record)| {
                let module = record.sources.parse()?;
                if module.name.to_string() != *name {
                    return Err(BuildError::Idl(format!(
                        "dependency graph key {name:?} disagrees with source-derived module {}",
                        module.name
                    )));
                }
                Ok(module)
            })
            .collect::<Result<Vec<_>, BuildError>>()?;
        let resolved = resolve_modules(parsed).map_err(|source| BuildError::Idl(source.to_string()))?;
        let resolved_module = resolved
            .module(&own_module.name.to_string())
            .ok_or_else(|| BuildError::Idl("resolved graph lost the crate's own module".into()))?;
        // Rust locations travel with the source owner but never enter resolution
        // or hashing. Every transitive consumer uses the same validated mapping.
        let mut paths = RustModulePaths::default();
        for (name, record) in &records {
            if let Some(path) = &record.rust_path {
                paths.insert(name, path).map_err(|source| BuildError::Idl(source.to_string()))?;
            }
        }
        let mut bindings =
            generate_rust_with_paths_and_source_map(&resolved, &own_module.name.to_string(), &paths).map_err(|source| BuildError::Idl(source.to_string()))?;
        let stem = own_module.name.segments().join("_");
        let bindings_path = output_dir.join(format!("{stem}.rs"));
        let include_path = output_dir.join(format!("{stem}.include.rs"));
        let source_map_path = output_dir.join(format!("{stem}.rs.map"));
        bindings.source_map.source_root = Some(source_map::relative_source_root(&output_dir, &source_root)?);
        let map_bytes = serde_json::to_vec_pretty(&bindings.source_map).map_err(|source_error| BuildError::Json {
            operation: "encode Rust source map",
            path: source_map_path.clone(),
            source: source_error,
        })?;
        write_changed(&bindings_path, bindings.source.as_bytes())?;
        write_changed(&source_map_path, &map_bytes)?;
        // Only this small entry is macro-included. A normal Rust module gives
        // attribute-macro definitions an actual source file for IDE navigation.
        // Its relative path also keeps the pair relocatable within OUT_DIR.
        // Explicit imports retain their original precedence over consumer glob
        // re-exports. In particular, another contract's module metadata must not
        // make this contract's MODULE_ID or ABI_FINGERPRINT ambiguous.
        let explicit_exports = std::iter::once("MODULE_ID, ABI_FINGERPRINT, MODULE_NAME".to_owned())
            .chain(resolved_module.source.imports.iter().flat_map(|import| import.names.iter()).cloned())
            .collect::<Vec<_>>()
            .join(", ");
        let entry = format!(
            "// Generated module entry. Include this file from the contract's Rust module.\n\
             #[path = \"{stem}.rs\"]\n\
             mod __nc_idl_{stem};\n\
             // Preserve the contract's public Rust paths at the include site.\n\
             pub use self::__nc_idl_{stem}::*;\n\
             pub use self::__nc_idl_{stem}::{{{explicit_exports}}};\n"
        );
        write_changed(&include_path, entry.as_bytes())?;

        let graph = GraphRecord { schema: GRAPH_SCHEMA, modules: records };
        let graph_bytes = serde_json::to_vec_pretty(&graph).map_err(|source_error| BuildError::Json {
            operation: "encode IDL graph",
            path: output_dir.clone(),
            source: source_error,
        })?;
        let graph_path = output_dir.join("neocogi-idl-graph.json");
        write_changed(&graph_path, &graph_bytes)?;

        println!("cargo::metadata=idl_graph={}", graph_path.display());
        println!("cargo::metadata=idl_source_map={}", source_map_path.display());
        println!("cargo::metadata=idl_include={}", include_path.display());
        println!("cargo::metadata=module_name={}", own_module.name);
        println!("cargo::metadata=module_id={}", resolved_module.module_id);
        println!("cargo::metadata=abi_fingerprint={}", resolved_module.fingerprint_hex());

        Ok(BuildOutput {
            module_name: own_module.name.to_string(),
            module_id: resolved_module.module_id,
            abi_fingerprint: resolved_module.abi_fingerprint,
            bindings: bindings_path,
            include: include_path,
            source_map: source_map_path,
            dependency_graph: graph_path,
        })
    }
}

/// Deterministic outputs owned by one successful contract build script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildOutput {
    /// Exact dotted logical module name.
    pub module_name: String,
    /// Stable identity derived from the module name.
    pub module_id: neocogi_com_core::NcGuid,
    /// Transitive canonical ABI fingerprint.
    pub abi_fingerprint: [u8; 32],
    /// Generated Rust definitions loaded as a normal module by `include`.
    pub bindings: PathBuf,
    /// Small `.include.rs` entry that the contract crate includes and re-exports.
    /// Loading the definitions through a module preserves editor navigation.
    pub include: PathBuf,
    /// Readable provenance sidecar for exactly the generated bindings.
    pub source_map: PathBuf,
    /// Machine-readable source graph exported through Cargo metadata.
    pub dependency_graph: PathBuf,
}

/// Build-script failure with contextual paths and dependency names.
#[derive(Debug)]
pub enum BuildError {
    /// A required Cargo build-script variable was absent.
    MissingEnvironment(
        /// Exact variable expected from Cargo.
        &'static str,
    ),
    /// A direct imported contract crate did not publish graph metadata.
    MissingDependency {
        /// Logical module named by the IDL import.
        module: String,
        /// Cargo `DEP_*` variable expected from its `links` package.
        variable: String,
    },
    /// The package's `links` value does not own the source module.
    LinksMismatch {
        /// Logical module derived from the `.ncidl` path.
        module: String,
        /// Required conventional `links` value.
        expected: String,
        /// Value actually supplied by the package manifest.
        actual: String,
    },
    /// Filesystem operation failed at a concrete path.
    Io {
        /// Human-readable operation being attempted.
        operation: &'static str,
        /// File involved in the failed operation.
        path: PathBuf,
        /// Original operating-system I/O failure.
        source: io::Error,
    },
    /// JSON graph serialization failed at a concrete path.
    Json {
        /// Human-readable encode or decode operation.
        operation: &'static str,
        /// File involved in the failed operation.
        path: PathBuf,
        /// Original JSON codec failure.
        source: serde_json::Error,
    },
    /// Parsing, resolution, or generation rejected the module graph.
    Idl(
        /// Complete source-level diagnostic.
        String,
    ),
}

impl fmt::Display for BuildError {
    /// Writes a concise build-script diagnostic with concrete context.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnvironment(variable) => write!(formatter, "Cargo did not provide required build variable {variable}"),
            Self::MissingDependency { module, variable } => {
                write!(formatter, "IDL import {module:?} requires a contract dependency that publishes {variable}")
            }
            Self::LinksMismatch { module, expected, actual } => {
                write!(formatter, "contract crate for {module:?} must declare links = {expected:?}, found {actual:?}")
            }
            Self::Io { operation, path, source } => write!(formatter, "failed to {operation} {}: {source}", path.display()),
            Self::Json { operation, path, source } => write!(formatter, "failed to {operation} {}: {source}", path.display()),
            Self::Idl(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for BuildError {
    /// Retains concrete I/O and JSON causes for build tooling.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Schema version for Cargo-to-Cargo IDL source graph exchange.
const GRAPH_SCHEMA: u32 = 5;

/// Serializable transitive module graph produced by a dependency build.
#[derive(Deserialize, Serialize)]
struct GraphRecord {
    /// Record schema version checked before reading any module fields.
    schema: u32,
    /// Modules keyed by exact dotted logical name.
    modules: BTreeMap<String, GraphModule>,
}

/// Source record sufficient to repeat semantic resolution in a dependent crate.
#[derive(Deserialize, Eq, PartialEq, Serialize)]
struct GraphModule {
    /// Canonically loaded source topology, flattened in graph schema v4.
    #[serde(flatten)]
    sources: ModuleSources,
    /// Optional absolute Rust location; omitted for conventional contract crates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rust_path: Option<String>,
}

/// Merges dependency records while rejecting two sources for one logical name.
fn merge_records(target: &mut BTreeMap<String, GraphModule>, incoming: BTreeMap<String, GraphModule>) -> Result<(), BuildError> {
    for (name, record) in incoming {
        if target.get(&name).is_some_and(|existing| existing != &record) {
            return Err(BuildError::Idl(format!(
                "dependency graphs provide conflicting sources or Rust locations for module {name:?}"
            )));
        }
        target.entry(name).or_insert(record);
    }
    Ok(())
}

/// Returns one required Cargo path variable without lossy Unicode conversion.
fn required_path(variable: &'static str) -> Result<PathBuf, BuildError> {
    env::var_os(variable).map(PathBuf::from).ok_or(BuildError::MissingEnvironment(variable))
}

/// Derives the stable Cargo metadata owner for one logical module.
fn links_name(module: &str) -> String {
    format!("nc_contract_{}", module.replace('.', "_"))
}

/// Derives Cargo's metadata environment variable for one imported module.
fn dependency_graph_variable(module: &str) -> String {
    format!("DEP_{}_IDL_GRAPH", links_name(module).to_ascii_uppercase())
}

/// Replaces one generated file only when its contents changed.
pub(crate) fn write_changed(path: &Path, contents: &[u8]) -> Result<(), BuildError> {
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    fs::write(path, contents).map_err(|source| BuildError::Io {
        operation: "write generated output",
        path: path.to_owned(),
        source,
    })
}
