//! Shared source imports with consumer-owned Rust generation, independent of Cargo links.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use super::{BuildError, BuildOutput, GraphModule, GraphRecord, GRAPH_SCHEMA, ModuleSources, required_path, write_changed, write_module_bindings};
use crate::generate::RustModulePaths;

/// Generates selected modules directly from a common IDL namespace tree.
///
/// Each consuming crate runs its own build and owns its generated files. No
/// package, Cargo `links` owner, `DEP_*` exchange, runtime or registry is created.
/// Imported source is resolved from the same root and participates in the normal
/// canonical ABI computation. Rust locations affect generated paths, never ABI
/// identity. Two consumers may therefore generate the same contract independently.
///
/// Register every reachable non-bundled module either with [`Self::module`] to
/// emit local bindings, or [`Self::import`] to use Rust definitions already
/// exposed by an ordinary library. The latter is useful when a Rust helper API
/// passes its own generated values directly; it is not needed for COM calls.
#[derive(Clone, Debug)]
pub struct SharedSources {
    /// Shared namespace root, relative to the consuming manifest unless absolute.
    root: PathBuf,
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
        Self { root: root.into(), bindings: Vec::new() }
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
    /// Source still comes from the shared root and is checked in this build.
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
        let root = required_path("CARGO_MANIFEST_DIR")?.join(&self.root);
        self.build_at(&root, &required_path("OUT_DIR")?)
    }

    /// Generates at explicit paths, with no dependency on process environment.
    /// All source/path/graph admission completes before generated output changes.
    fn build_at(&self, root: &Path, output: &Path) -> Result<Vec<BuildOutput>, BuildError> {
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
        let mut pending = local.iter().cloned().collect::<Vec<_>>();
        while let Some(name) = pending.pop() {
            if crate::is_bundled_module(&name) || modules.contains_key(&name) {
                continue;
            }
            if !policies.contains_key(&name) {
                return Err(BuildError::Idl(format!("shared IDL import {name:?} has no local or imported Rust location")));
            }
            let relative = PathBuf::from(name.replace('.', "/"));
            // Watch both candidates: adding a multipart directory must invalidate
            // a previous file build. A directory and file may not claim one name.
            let file = relative.with_extension("ncidl");
            println!("cargo::rerun-if-changed={}", root.join(&relative).display());
            println!("cargo::rerun-if-changed={}", root.join(&file).display());
            if root.join(&relative).is_dir() && root.join(&file).is_file() {
                return Err(BuildError::Idl(format!("shared module {name:?} has both file and directory sources")));
            }
            let selected = if root.join(&relative).is_dir() { &relative } else { &file };
            let sources = ModuleSources::load(root, selected)?;
            for (part, _) in sources.parts() {
                println!("cargo::rerun-if-changed={}", root.join(part).display());
            }
            let parsed = sources.parse()?;
            pending.extend(parsed.externals.iter().map(|external| external.module.clone()));
            modules.insert(name, (sources, parsed));
        }
        let resolved = crate::resolve_modules(modules.values().map(|(_, module)| module.clone()).collect()).map_err(BuildError::Resolve)?;
        let roots = modules.keys().map(|name| (name.clone(), root.to_owned())).collect();
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
        let mut request = SharedSources::new(&root);
        for (index, name) in names.into_iter().enumerate() {
            let source = root.join(name.replace('.', "/")).with_extension("ncidl");
            fs::create_dir_all(source.parent().unwrap()).unwrap();
            fs::write(&source, "#[version(1)] struct Value { value: u32 }").unwrap();
            request = request.module(name, format!("first::module{index}"));
        }
        let output = work.path().join("generated");
        let first = request.build_at(&root, &output).unwrap();
        let second = names
            .into_iter()
            .enumerate()
            .fold(SharedSources::new(&root), |request, (index, name)| {
                request.module(name, format!("second::module{index}"))
            })
            .build_at(&root, &work.path().join("another consumer"))
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
                .build_at(&root, &output)
                .unwrap_err()
                .to_string()
                .contains("no local or imported Rust location")
        );
        assert!(!output.exists());
        fs::create_dir(root.join("proof/root")).unwrap();
        let request = request.import("proof.value", "values::api");
        assert!(request.build_at(&root, &output).unwrap_err().to_string().contains("both file and directory"));
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
            assert!(request.build_at(work.path(), &work.path().join("output")).is_err());
            assert!(!work.path().join("output").exists());
        }
    }
}
