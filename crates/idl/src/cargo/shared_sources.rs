//! Shared source imports with consumer-owned Rust generation, independent of Cargo links.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use super::{BuildError, BuildOutput, GraphModule, GraphRecord, GRAPH_SCHEMA, ModuleSources, required_path, write_changed, write_module_bindings};
use crate::generate::RustModulePaths;

/// Generates selected modules directly from shared IDL namespace trees.
///
/// Each consuming crate runs its own build and owns its generated files. No
/// package, Cargo `links` owner, `DEP_*` exchange, runtime or registry is created.
/// Imported source is resolved from the configured roots and participates in the normal
/// canonical ABI computation. Rust locations affect generated paths, never ABI
/// identity. Two consumers may therefore generate the same contract independently.
///
/// Register every reachable non-bundled module either with [`Self::module`] to
/// emit local bindings, or [`Self::import`] to use Rust definitions already
/// exposed by an ordinary library. The latter is useful when a Rust helper API
/// passes its own generated values directly; it is not needed for COM calls.
#[derive(Clone, Debug)]
pub struct SharedSources {
    /// Source roots, relative to the consuming manifest unless absolute.
    roots: Vec<PathBuf>,
    /// Requested local emissions and explicit external Rust binding locations.
    bindings: Vec<Binding>,
}

/// One declared Rust location with an explicit source-generation policy.
#[derive(Clone, Debug)]
struct Binding {
    /// Exact dotted IDL module name; also determines its source path.
    module: String,
    /// Validated by the generator before any output is written.
    rust_path: String,
    /// Whether this consumer emits the module rather than importing Rust types.
    emission: Emission,
}

/// Distinguishes local generation from ordinary Rust imports without hidden fallback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Emission {
    /// Generate this consumer's independent Rust definitions.
    Local,
    /// Resolve source for ABI identity but use an existing library's Rust types.
    Imported,
}

impl SharedSources {
    /// Selects a shared directory containing file and multipart module sources.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            roots: vec![root.into()],
            bindings: Vec::new(),
        }
    }

    /// Adds a source root owned by another subsystem.
    ///
    /// Paths are relative to the consuming manifest unless absolute. Every
    /// reachable module must have exactly one source owner across all roots;
    /// root order never chooses between conflicting definitions. Repeating the
    /// same canonical root is harmless.
    pub fn include_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.roots.push(root.into());
        self
    }

    /// Emits a module at the supplied crate-rooted Rust location.
    /// Use an explicit self alias when the path names the consuming crate itself.
    pub fn module(mut self, module: impl Into<String>, rust_path: impl Into<String>) -> Self {
        self.bindings.push(Binding {
            module: module.into(),
            rust_path: rust_path.into(),
            emission: Emission::Local,
        });
        self
    }

    /// Uses an ordinary dependency's generated Rust definitions for an import.
    /// Source still comes from the shared roots and is checked in this build.
    pub fn import(mut self, module: impl Into<String>, rust_path: impl Into<String>) -> Self {
        self.bindings.push(Binding {
            module: module.into(),
            rust_path: rust_path.into(),
            emission: Emission::Imported,
        });
        self
    }

    /// Builds from Cargo's manifest directory into this consumer's `OUT_DIR`.
    /// Emits source rerun instructions, but no Cargo ownership metadata.
    pub fn build(self) -> Result<Vec<BuildOutput>, BuildError> {
        let manifest = required_path("CARGO_MANIFEST_DIR")?;
        let roots = self.roots.iter().map(|root| manifest.join(root)).collect::<Vec<_>>();
        self.build_at(&roots, &required_path("OUT_DIR")?)
    }

    /// Generates at explicit paths, with no dependency on process environment.
    /// All source/path/graph admission completes before generated output changes.
    fn build_at(&self, source_roots: &[PathBuf], output: &Path) -> Result<Vec<BuildOutput>, BuildError> {
        let mut paths = RustModulePaths::default();
        let mut local = BTreeSet::new();
        let mut policies = BTreeMap::new();
        for binding in &self.bindings {
            paths
                .insert(&binding.module, &binding.rust_path)
                .map_err(|error| BuildError::Idl(error.to_string()))?;
            if policies
                .insert(binding.module.clone(), binding.emission)
                .is_some_and(|prior| prior != binding.emission)
            {
                return Err(BuildError::Idl(format!(
                    "module {:?} cannot be both locally generated and imported",
                    binding.module
                )));
            }
            if binding.emission == Emission::Local {
                local.insert(binding.module.clone());
            }
        }
        if local.is_empty() {
            return Err(BuildError::Idl("shared source build requires at least one local module".into()));
        }
        let mut modules = BTreeMap::new();
        let mut roots = BTreeMap::new();
        let mut pending = local.iter().cloned().collect::<Vec<_>>();
        while let Some(name) = pending.pop() {
            if crate::is_bundled_module(&name) || modules.contains_key(&name) {
                continue;
            }
            if !policies.contains_key(&name) {
                return Err(BuildError::Idl(format!("shared IDL import {name:?} has no local or imported Rust location")));
            }
            let (root, sources) = load_sources(source_roots, &name)?;
            roots.insert(name.clone(), root);
            let parsed = sources.parse()?;
            pending.extend(parsed.externals.iter().map(|external| external.module.clone()));
            modules.insert(name, (sources, parsed));
        }
        let resolved = crate::resolve_modules(modules.values().map(|(_, module)| module.clone()).collect()).map_err(BuildError::Resolve)?;
        // A repository may declare locations for several independent graphs.
        // Only the reachable closure belongs to this generation invocation.
        let mut paths = RustModulePaths::default();
        for binding in &self.bindings {
            if resolved.module(&binding.module).is_some() {
                paths
                    .insert(&binding.module, &binding.rust_path)
                    .map_err(|error| BuildError::Idl(error.to_string()))?;
            }
        }
        fs::create_dir_all(output).map_err(|source| BuildError::Io {
            operation: "create generated output directory",
            path: output.to_owned(),
            source,
        })?;
        let mut results = Vec::new();
        for name in local {
            // Module UUIDs are already the canonical logical identity. Bounded,
            // lowercase filenames distinguish separator/case variants on Windows.
            let stem = format!(
                "module_{}",
                resolved
                    .module(&name)
                    .expect("local module was resolved")
                    .module_id
                    .to_string()
                    .replace('-', "")
            );
            results.push(write_module_bindings(
                &resolved,
                &name,
                &paths,
                &roots,
                output,
                &stem,
                "// Consumer-owned bindings from shared IDL sources; no Cargo links owner.",
            )?);
        }
        let graph = GraphRecord {
            schema: GRAPH_SCHEMA,
            modules: modules
                .into_iter()
                .map(|(name, (sources, _))| {
                    let rust_path = self
                        .bindings
                        .iter()
                        .find(|binding| binding.module == name)
                        .map(|binding| binding.rust_path.clone());
                    (name, GraphModule { sources, rust_path })
                })
                .collect(),
            source_roots: roots,
        };
        let path = output.join("neocogi-idl-graph.json");
        let bytes = serde_json::to_vec_pretty(&graph).map_err(|source| BuildError::Json {
            operation: "encode shared IDL graph",
            path: path.clone(),
            source,
        })?;
        write_changed(&path, &bytes)?;
        Ok(results)
    }
}

/// Finds one authoritative module across subsystem roots before any output is
/// written. File/directory conflicts and distinct roots claiming the same name
/// are errors, rather than order-dependent shadowing. Filesystem failures retain
/// their IO cause; only an absent candidate is ignored.
fn load_sources(roots: &[PathBuf], name: &str) -> Result<(PathBuf, ModuleSources), BuildError> {
    let relative = PathBuf::from(name.replace('.', "/"));
    let file = relative.with_extension("ncidl");
    let mut owner: Option<(PathBuf, PathBuf)> = None;
    for root in roots {
        for (candidate, directory) in [(&relative, true), (&file, false)] {
            let path = root.join(candidate);
            // Watch absent candidates too: adding another owner invalidates the
            // build and must cause the same ambiguity error as a clean build.
            println!("cargo::rerun-if-changed={}", path.display());
            let metadata = match fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(source) => {
                    return Err(BuildError::Io {
                        operation: "inspect shared IDL source",
                        path,
                        source,
                    });
                }
            };
            if metadata.is_dir() != directory || (!directory && !metadata.is_file()) {
                return Err(BuildError::Idl(format!("shared IDL source has unexpected file kind: {}", path.display())));
            }
            let canonical_root = fs::canonicalize(root).map_err(|source| BuildError::Io {
                operation: "canonicalize shared IDL root",
                path: root.clone(),
                source,
            })?;
            if let Some((prior_root, prior_candidate)) = &owner {
                if prior_root == &canonical_root && prior_candidate == candidate {
                    continue;
                }
                return Err(BuildError::Idl(format!(
                    "shared module {name:?} has ambiguous source owners: {} and {} (including both file and directory forms)",
                    prior_root.join(prior_candidate).display(),
                    path.display(),
                )));
            }
            owner = Some((canonical_root, candidate.clone()));
        }
    }
    let (root, candidate) = owner.ok_or_else(|| BuildError::Idl(format!("shared module {name:?} has no source in the configured roots")))?;
    let sources = ModuleSources::load(&root, &candidate)?;
    for (part, _) in sources.parts() {
        println!("cargo::rerun-if-changed={}", root.join(part).display());
    }
    Ok((root, sources))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Multi-module outputs remain distinct for separator and case variants,
    /// including on case-insensitive filesystems. Rust paths never affect ABI.
    #[test]
    fn independent_outputs_preserve_exact_module_names_and_source_provenance() {
        let work = tempfile::tempdir().unwrap();
        let root = work.path().join("idl");
        let names = ["a_b.c", "a.b_c", "Case.same", "case.same"];
        let mut request = SharedSources::new(&root).import("unused.module", "unused::module");
        for (index, name) in names.into_iter().enumerate() {
            let source = root.join(name.replace('.', "/")).with_extension("ncidl");
            fs::create_dir_all(source.parent().unwrap()).unwrap();
            fs::write(&source, "#[version(1)] struct Value { value: u32 }").unwrap();
            request = request.module(name, format!("first::module{index}"));
        }
        let output = work.path().join("generated");
        let first = request.build_at(std::slice::from_ref(&root), &output).unwrap();
        let second = names
            .into_iter()
            .enumerate()
            .fold(SharedSources::new(&root), |request, (index, name)| {
                request.module(name, format!("second::module{index}"))
            })
            .build_at(std::slice::from_ref(&root), &work.path().join("another consumer"))
            .unwrap();
        let filenames = first
            .iter()
            .map(|module| module.include.file_name().unwrap().to_str().unwrap().to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        assert_eq!(filenames.len(), 4);
        for (left, right) in first.iter().zip(&second) {
            assert_eq!(left.module_name, right.module_name);
            assert_eq!(left.module_id, right.module_id);
            assert_eq!(left.abi_fingerprint, right.abi_fingerprint);
            let map: crate::RustSourceMap = serde_json::from_slice(&fs::read(&left.source_map).unwrap()).unwrap();
            for source in map.sources {
                assert!(output.join(source.location.unwrap()).exists());
            }
        }
    }

    /// Missing explicit Rust imports and ambiguous filesystem owners fail before output.
    #[test]
    fn rejects_unmapped_imports_and_ambiguous_source_owners() {
        let work = tempfile::tempdir().unwrap();
        let root = work.path().join("idl");
        fs::create_dir_all(root.join("proof")).unwrap();
        fs::write(
            root.join("proof/root.ncidl"),
            "extern proof.value; use proof.value::Value; #[version(1)] struct Root { value: Value }",
        )
        .unwrap();
        fs::write(root.join("proof/value.ncidl"), "#[version(1)] struct Value { value: u32 }").unwrap();
        let output = work.path().join("output");
        let request = SharedSources::new(&root).module("proof.root", "consumer::root");
        assert!(
            request
                .build_at(std::slice::from_ref(&root), &output)
                .unwrap_err()
                .to_string()
                .contains("no local or imported Rust location")
        );
        assert!(!output.exists());
        fs::create_dir(root.join("proof/root")).unwrap();
        let request = request.import("proof.value", "values::api");
        assert!(
            request
                .build_at(std::slice::from_ref(&root), &output)
                .unwrap_err()
                .to_string()
                .contains("both file and directory")
        );
        assert!(!output.exists());
    }

    /// Local/import conflicts and different Rust paths cannot silently select a winner.
    #[test]
    fn rejects_conflicting_binding_policies_and_paths() {
        let work = tempfile::tempdir().unwrap();
        for request in [
            SharedSources::new(work.path())
                .module("proof.value", "one::value")
                .import("proof.value", "one::value"),
            SharedSources::new(work.path())
                .module("proof.value", "one::value")
                .module("proof.value", "two::value"),
            SharedSources::new(work.path()).module("../escape", "one::value"),
            SharedSources::new(work.path()).module("proof.value", "crate::value"),
            SharedSources::new(work.path()).import("proof.value", "one::value"),
        ] {
            assert!(request.build_at(&[work.path().to_owned()], &work.path().join("output")).is_err());
            assert!(!work.path().join("output").exists());
        }
    }
    /// Separate subsystem roots resolve one graph with accurate source locations;
    /// duplicate roots are harmless, while a competing owner fails before output.
    #[test]
    fn subsystem_roots_resolve_imports_without_shadowing() {
        let work = tempfile::tempdir().unwrap();
        let com = work.path().join("com/schemas");
        let ui = work.path().join("ui/schemas");
        fs::create_dir_all(com.join("proof")).unwrap();
        fs::create_dir_all(ui.join("proof")).unwrap();
        fs::write(com.join("proof/value.ncidl"), "#[version(1)] struct Value { value: u32 }").unwrap();
        fs::write(
            ui.join("proof/root.ncidl"),
            "extern proof.value; use proof.value::Value; #[version(1)] struct Root { value: Value }",
        )
        .unwrap();
        let request = SharedSources::new(&ui)
            .include_root(&com)
            .include_root(ui.join("."))
            .module("proof.root", "consumer::root")
            .module("proof.value", "consumer::value");
        let output = work.path().join("output");
        request.build_at(&request.roots, &output).unwrap();
        let graph: GraphRecord = serde_json::from_slice(&fs::read(output.join("neocogi-idl-graph.json")).unwrap()).unwrap();
        assert_eq!(graph.source_roots["proof.root"], fs::canonicalize(&ui).unwrap());
        assert_eq!(graph.source_roots["proof.value"], fs::canonicalize(&com).unwrap());
        let before = fs::read(output.join("neocogi-idl-graph.json")).unwrap();
        fs::copy(com.join("proof/value.ncidl"), ui.join("proof/value.ncidl")).unwrap();
        let error = request.build_at(&request.roots, &output).unwrap_err();
        assert!(error.to_string().contains("ambiguous source owners"), "{error}");
        assert_eq!(fs::read(output.join("neocogi-idl-graph.json")).unwrap(), before);
        let fresh = work.path().join("fresh-output");
        assert!(request.build_at(&request.roots, &fresh).is_err());
        assert!(!fresh.exists());
    }

    /// IO failures stay distinguishable from a missing source namespace.
    #[cfg(unix)]
    #[test]
    fn source_inspection_preserves_io_failures() {
        let work = tempfile::tempdir().unwrap();
        let blocker = work.path().join("not-a-directory");
        fs::write(&blocker, "file").unwrap();
        let error = load_sources(&[blocker], "proof.value").unwrap_err();
        assert!(matches!(error, BuildError::Io { .. }), "{error}");
        let missing = load_sources(&[work.path().join("missing")], "proof.value").unwrap_err();
        assert!(matches!(missing, BuildError::Idl(_)), "{missing}");
    }
}
