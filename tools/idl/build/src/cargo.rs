//! Cargo-specific loading, graph exchange, and generated-file ownership.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fmt, fs, io,
    path::{Path, PathBuf},
};

use neocogi_idl::{ModuleSource, parse_module, parse_module_parts, resolve_modules};
use serde::{Deserialize, Serialize};

use crate::{generate_rust, native};

/// Schema version for one native contract artifact record.
const ARTIFACT_SCHEMA: u32 = 1;

/// Declarative build request for exactly one contract module crate.
#[derive(Clone, Debug)]
pub struct Module {
    /// Directory containing the crate's IDL namespace tree.
    idl_root: PathBuf,
    /// File or directory that defines the logical module and its source parts.
    input: ModuleInput,
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
        }
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
        let own_record = load_sources(&source_root, &self.input)?;
        println!("cargo::rerun-if-env-changed=NEOCOGI_CONTRACT_LINKAGE");
        println!("cargo::rerun-if-env-changed=NEOCOGI_CONTRACT_NATIVE_TARGET");

        let own_module = parse_record(&own_record)?;
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
        let mut dependency_artifacts = BTreeMap::new();
        let dependency_modules = own_module.externals.iter().map(|external| external.module.clone()).collect::<BTreeSet<_>>();
        for dependency in dependency_modules {
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
            let artifact = graph
                .modules
                .get(&dependency)
                .and_then(|module| module.artifact.clone())
                .ok_or_else(|| BuildError::Idl(format!("dependency module {dependency:?} did not publish a native artifact record")))?;
            dependency_artifacts.insert(dependency, artifact);
            merge_records(&mut records, graph.modules)?;
        }

        let parsed = records
            .iter()
            .map(|(name, record)| {
                let module = parse_record(record)?;
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
        let bindings = generate_rust(&resolved, &own_module.name.to_string()).map_err(|source| BuildError::Idl(source.to_string()))?;
        let stem = own_module.name.segments().join("_");
        let bindings_path = output_dir.join(format!("{stem}.rs"));
        write_changed(&bindings_path, bindings.as_bytes())?;

        let exact_dependencies = resolved_module
            .dependencies
            .iter()
            .map(|dependency| {
                let artifact = dependency_artifacts
                    .get(&dependency.module_name)
                    .ok_or_else(|| BuildError::Idl(format!("resolved dependency {:?} has no native artifact record", dependency.module_name)))?;
                artifact.validate_dependency(dependency)?;
                Ok(artifact.clone())
            })
            .collect::<Result<Vec<_>, BuildError>>()?;
        let artifact = native::build(resolved_module, &exact_dependencies, &output_dir)?;
        let artifact_bytes = serde_json::to_vec_pretty(&artifact).map_err(|source_error| BuildError::Json {
            operation: "encode native contract artifact record",
            path: output_dir.clone(),
            source: source_error,
        })?;
        let artifact_path = output_dir.join("neocogi-contract-artifact.json");
        write_changed(&artifact_path, &artifact_bytes)?;
        let own_record = records
            .get_mut(&module_name)
            .ok_or_else(|| BuildError::Idl("resolved graph lost the crate's mutable source record".into()))?;
        own_record.artifact = Some(artifact.clone());

        let graph = GraphRecord { schema: GRAPH_SCHEMA, modules: records };
        let graph_bytes = serde_json::to_vec_pretty(&graph).map_err(|source_error| BuildError::Json {
            operation: "encode IDL graph",
            path: output_dir.clone(),
            source: source_error,
        })?;
        let graph_path = output_dir.join("neocogi-idl-graph.json");
        write_changed(&graph_path, &graph_bytes)?;

        println!("cargo::metadata=idl_graph={}", graph_path.display());
        println!("cargo::metadata=artifact={}", artifact_path.display());
        println!("cargo::metadata=module_name={}", own_module.name);
        println!("cargo::metadata=module_id={}", resolved_module.module_id);
        println!("cargo::metadata=abi_fingerprint={}", resolved_module.fingerprint_hex());

        Ok(BuildOutput {
            module_name: own_module.name.to_string(),
            module_id: resolved_module.module_id,
            abi_fingerprint: resolved_module.abi_fingerprint,
            bindings: bindings_path,
            dependency_graph: graph_path,
            artifact,
            artifact_record: artifact_path,
        })
    }
}

/// Machine-readable identity and path of one exact native contract artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactRecord {
    /// Record schema version checked by build and assembly tooling.
    pub schema: u32,
    /// Exact dotted logical module name.
    pub module: String,
    /// Stable UUID string derived from the logical module name.
    pub module_id: String,
    /// Complete lowercase SHA-256 ABI fingerprint.
    pub abi_fingerprint: String,
    /// Rust target triple for which the artifact was compiled.
    pub target: String,
    /// Native linkage mode, either `dynamic` or `static`.
    pub linkage: String,
    /// Fingerprint-qualified linker name without platform decoration.
    pub link_name: String,
    /// Absolute path to the runtime shared library or static archive.
    pub library: PathBuf,
    /// Platform linker companion files, such as a Windows import library.
    pub linker_files: Vec<PathBuf>,
    /// Fingerprint-qualified exported description getter.
    pub getter: String,
}

impl ArtifactRecord {
    /// Selects the file passed to a native linker for this artifact.
    pub(crate) fn linker_file(&self) -> Result<&Path, BuildError> {
        if self.linkage == "dynamic" && self.target.contains("windows") {
            return self
                .linker_files
                .first()
                .map(PathBuf::as_path)
                .ok_or_else(|| BuildError::Idl(format!("dynamic Windows contract artifact {:?} has no import library", self.module)));
        }
        Ok(&self.library)
    }

    /// Verifies that a direct IDL edge uses the artifact Cargo actually built.
    fn validate_dependency(&self, dependency: &neocogi_idl::ResolvedDependency) -> Result<(), BuildError> {
        if self.schema != ARTIFACT_SCHEMA {
            return Err(BuildError::Idl(format!(
                "dependency module {:?} uses unsupported artifact schema {}",
                dependency.module_name, self.schema
            )));
        }
        let expected_id = dependency.module_id.to_string();
        let expected_fingerprint = fingerprint_hex(&dependency.abi_fingerprint);
        if self.module != dependency.module_name || self.module_id != expected_id || self.abi_fingerprint != expected_fingerprint {
            return Err(BuildError::Idl(format!(
                "dependency artifact for {:?} does not match the resolved IDL identity",
                dependency.module_name
            )));
        }
        Ok(())
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
    /// Generated Rust source included by the module crate.
    pub bindings: PathBuf,
    /// Machine-readable source graph exported through Cargo metadata.
    pub dependency_graph: PathBuf,
    /// Exact native contract library built for this module revision.
    pub artifact: ArtifactRecord,
    /// Machine-readable record exported for distribution assembly.
    pub artifact_record: PathBuf,
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
    /// A platform C compiler or linker rejected generated native code.
    Command {
        /// Human-readable compilation operation being attempted.
        operation: &'static str,
        /// Process exit code, or `None` if the process ended by signal.
        status: Option<i32>,
        /// Complete standard output captured from the command.
        stdout: String,
        /// Complete standard error captured from the command.
        stderr: String,
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
            Self::Command { operation, status, stdout, stderr } => {
                write!(formatter, "failed to {operation} (exit {status:?})\nstdout:\n{stdout}\nstderr:\n{stderr}")
            }
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
const GRAPH_SCHEMA: u32 = 2;

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
    /// Normalized file or directory path that defines the logical module name.
    module_path: String,
    /// Source parts keyed by normalized path relative to the IDL root.
    sources: BTreeMap<String, String>,
    /// Exact target artifact when this module record originated at its owner.
    artifact: Option<ArtifactRecord>,
}

/// Loads a single-file or directory-based module into its graph representation.
fn load_sources(idl_root: &Path, input: &ModuleInput) -> Result<GraphModule, BuildError> {
    match input {
        ModuleInput::File(relative_path) => {
            let source_path = idl_root.join(relative_path);
            println!("cargo::rerun-if-changed={}", source_path.display());
            let source = read_source(&source_path)?;
            Ok(GraphModule {
                module_path: path_text(relative_path)?,
                sources: BTreeMap::from([(path_text(relative_path)?, source)]),
                artifact: None,
            })
        }
        ModuleInput::Directory(relative_directory) => {
            let module_directory = idl_root.join(relative_directory);
            println!("cargo::rerun-if-changed={}", module_directory.display());
            let mut paths = Vec::new();
            discover_parts(&module_directory, &mut paths)?;
            paths.sort();
            let mut sources = BTreeMap::new();
            for source_path in paths {
                println!("cargo::rerun-if-changed={}", source_path.display());
                let relative_path = source_path.strip_prefix(idl_root).map_err(|_| {
                    BuildError::Idl(format!(
                        "discovered IDL source {} outside configured root {}",
                        source_path.display(),
                        idl_root.display()
                    ))
                })?;
                sources.insert(path_text(relative_path)?, read_source(&source_path)?);
            }
            Ok(GraphModule {
                module_path: path_text(relative_directory)?,
                sources,
                artifact: None,
            })
        }
    }
}

/// Recursively discovers `.ncidl` source parts without assigning order meaning.
fn discover_parts(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), BuildError> {
    let entries = fs::read_dir(directory).map_err(|source| BuildError::Io {
        operation: "read IDL module directory",
        path: directory.to_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| BuildError::Io {
            operation: "read IDL module directory entry",
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| BuildError::Io {
            operation: "inspect IDL module directory entry",
            path: path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            discover_parts(&path, output)?;
        } else if file_type.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some("ncidl") {
            output.push(path);
        }
    }
    Ok(())
}

/// Reads one UTF-8 IDL part with its concrete filesystem path in failures.
fn read_source(path: &Path) -> Result<String, BuildError> {
    fs::read_to_string(path).map_err(|source| BuildError::Io {
        operation: "read IDL source",
        path: path.to_owned(),
        source,
    })
}

/// Parses one graph record through its file or directory identity form.
fn parse_record(record: &GraphModule) -> Result<neocogi_idl::Module, BuildError> {
    let module_path = Path::new(&record.module_path);
    if module_path.extension().and_then(|extension| extension.to_str()) == Some("ncidl") {
        if record.sources.len() != 1 {
            return Err(BuildError::Idl(format!(
                "single-file IDL module {:?} contains {} source records",
                record.module_path,
                record.sources.len()
            )));
        }
        let source = record.sources.values().next().expect("single-file record length was checked");
        return parse_module(module_path, source).map_err(|source| BuildError::Idl(source.to_string()));
    }

    let parts = record
        .sources
        .iter()
        .map(|(path, source)| ModuleSource { path: Path::new(path), text: source })
        .collect::<Vec<_>>();
    parse_module_parts(module_path, &parts).map_err(|source| BuildError::Idl(source.to_string()))
}

/// Merges dependency records while rejecting two sources for one logical name.
fn merge_records(target: &mut BTreeMap<String, GraphModule>, incoming: BTreeMap<String, GraphModule>) -> Result<(), BuildError> {
    for (name, record) in incoming {
        if target.get(&name).is_some_and(|existing| !same_module_record(existing, &record)) {
            return Err(BuildError::Idl(format!("dependency graphs provide conflicting sources for module {name:?}")));
        }
        target.entry(name).or_insert(record);
    }
    Ok(())
}

/// Compares logical module content while ignoring build-directory path aliases.
fn same_module_record(left: &GraphModule, right: &GraphModule) -> bool {
    if left.module_path != right.module_path || left.sources != right.sources {
        return false;
    }
    match (&left.artifact, &right.artifact) {
        (Some(left), Some(right)) => {
            left.schema == right.schema
                && left.module == right.module
                && left.module_id == right.module_id
                && left.abi_fingerprint == right.abi_fingerprint
                && left.target == right.target
                && left.linkage == right.linkage
                && left.link_name == right.link_name
                && left.getter == right.getter
        }
        (None, None) => true,
        _ => false,
    }
}

/// Renders a fingerprint as its complete lowercase hexadecimal identity.
fn fingerprint_hex(fingerprint: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in fingerprint {
        use fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

/// Returns one required Cargo path variable without lossy Unicode conversion.
fn required_path(variable: &'static str) -> Result<PathBuf, BuildError> {
    env::var_os(variable).map(PathBuf::from).ok_or(BuildError::MissingEnvironment(variable))
}

/// Converts a normalized source path into portable metadata text.
fn path_text(path: &Path) -> Result<String, BuildError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| BuildError::Idl(format!("IDL source path {} is not UTF-8", path.display())))
}

/// Derives the stable native/Cargo link owner for one logical module.
fn links_name(module: &str) -> String {
    format!("nc_contract_{}", module.replace('.', "_"))
}

/// Derives Cargo's metadata environment variable for one imported module.
fn dependency_graph_variable(module: &str) -> String {
    format!("DEP_{}_IDL_GRAPH", links_name(module).to_ascii_uppercase())
}

/// Replaces one generated file only when its contents changed.
fn write_changed(path: &Path, contents: &[u8]) -> Result<(), BuildError> {
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    fs::write(path, contents).map_err(|source| BuildError::Io {
        operation: "write generated output",
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    //! Filesystem-focused tests for directory module discovery and graph loading.

    use std::{fs, path::Path};

    use tempfile::tempdir;

    use super::{ModuleInput, load_sources, parse_record};

    /// Confirms recursive discovery ignores unrelated files and produces one
    /// path-derived logical module with canonical declarations.
    #[test]
    fn loads_directory_parts_as_one_module() {
        let temporary = tempdir().unwrap();
        let root = temporary.path().join("idl");
        let module = root.join("neocogi/ui");
        let nested = module.join("input");
        fs::create_dir_all(&nested).unwrap();
        fs::write(module.join("widgets.ncidl"), "#[version(1)] trait IWidget {}").unwrap();
        fs::write(nested.join("geometry.ncidl"), "#[version(1)] struct Size { width: u32 }").unwrap();
        fs::write(module.join("README.md"), "not IDL").unwrap();

        let record = load_sources(&root, &ModuleInput::Directory(Path::new("neocogi/ui").to_owned())).unwrap();
        let parsed = parse_record(&record).unwrap();

        assert_eq!(record.module_path, "neocogi/ui");
        assert_eq!(
            record.sources.keys().map(String::as_str).collect::<Vec<_>>(),
            ["neocogi/ui/input/geometry.ncidl", "neocogi/ui/widgets.ncidl"]
        );
        assert_eq!(parsed.name.to_string(), "neocogi.ui");
        assert_eq!(
            parsed.declarations.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
            ["IWidget", "Size"]
        );
    }
}
