//! Owned module generation separates compiler rejection from filesystem work.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use super::{BuildError, BuildOutput, source_map, write_changed};
use crate::{
    GeneratedRust, ResolvedModules,
    generate::{RustModulePaths, generate_rust_with_paths_and_source_map},
};

/// One completed compiler result; physical source locations are finished later.
/// No filesystem operation occurs while constructing this owner. All requested
/// modules must reach this boundary before an invocation starts writing output.
pub(super) struct ModuleEmission {
    /// Paired formatted Rust and its logical provenance, from exactly one generation.
    generated: GeneratedRust,
    /// Normal-module include entry with explicit exports and selected physical name.
    include_text: String,
    /// Selected output identities, returned publicly only after publication succeeds.
    output: BuildOutput,
}

/// Generates one module and selects its Cargo filenames without touching output.
/// The authoritative generator admits complete declaration paths and owns all
/// syntax work; this result retains its exact text/map instead of regenerating.
pub(super) fn generate_module_emission(
    resolved: &ResolvedModules,
    module_name: &str,
    paths: &RustModulePaths,
    output_dir: &Path,
    stem: &str,
    ownership_note: &str,
) -> Result<ModuleEmission, BuildError> {
    let resolved_module = resolved
        .module(module_name)
        .ok_or_else(|| BuildError::Idl(format!("resolved graph lost module {module_name:?}")))?;
    let mut bindings = generate_rust_with_paths_and_source_map(resolved, module_name, paths)
        .map_err(|source| BuildError::Idl(format!("cannot generate module {module_name:?}: {source}")))?;
    let generated_file = format!("{stem}.rs");
    let bindings_path = output_dir.join(&generated_file);
    // Cargo owns physical naming, while the pure generator supplies only its
    // logical-name placeholder. The finished map must describe this exact file.
    bindings.source_map.generated_file = generated_file;
    let include_path = output_dir.join(format!("{stem}.include.rs"));
    let source_map_path = output_dir.join(format!("{stem}.rs.map"));
    // Only this small entry is macro-included. A normal Rust module gives
    // attribute-macro definitions an actual source file for IDE navigation.
    // Its relative path also keeps the pair relocatable within OUT_DIR.
    // Explicit imports retain their original precedence over consumer glob
    // re-exports. In particular, another contract's module metadata must not
    // make this contract's MODULE_ID, ABI_FINGERPRINT or dependency routes
    // ambiguous.
    let explicit_exports = std::iter::once("MODULE_ID, ABI_FINGERPRINT, MODULE_NAME".to_owned())
        .chain(resolved_module.source.imports.iter().map(|import| import.name.clone()))
        .chain(paths.dependency_namespace().map(|name| name.to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    let entry = format!(
        "// Generated module entry for IDL module `{module_name}`.\n\
             {ownership_note}\n\
             // The generated filename below is local to this package, not a global identity.\n\
             #[path = \"{stem}.rs\"]\n\
             mod __nc_idl_{stem};\n\
             // Preserve the public surface, including in private consumer modules.\n\
             #[allow(unused_imports)]\n\
             pub use self::__nc_idl_{stem}::*;\n\
             #[allow(unused_imports)]\n\
             pub use self::__nc_idl_{stem}::{{{explicit_exports}}};\n"
    );
    Ok(ModuleEmission {
        generated: bindings,
        include_text: entry,
        output: BuildOutput {
            module_name: module_name.to_owned(),
            module_id: resolved_module.module_id,
            abi_fingerprint: resolved_module.abi_fingerprint,
            bindings: bindings_path,
            include: include_path,
            source_map: source_map_path,
            dependency_graph: output_dir.join("neocogi-idl-graph.json"),
        },
    })
}

impl ModuleEmission {
    /// Finishes and serializes provenance without publishing any main artifact.
    /// This phase may create bundled navigation copies before a later error.
    /// Logical maps are discarded after serialization rather than retained twice.
    fn finish(mut self, roots: &BTreeMap<String, PathBuf>, resolved: &ResolvedModules, output: &Path) -> Result<ReadyEmission, BuildError> {
        source_map::resolve_locations(&mut self.generated.source_map, output, roots, resolved)?;
        let map_bytes = serde_json::to_vec_pretty(&self.generated.source_map).map_err(|source| BuildError::Json {
            operation: "encode Rust source map",
            path: self.output.source_map.clone(),
            source,
        })?;
        Ok(ReadyEmission {
            source: self.generated.source,
            source_map: map_bytes,
            include_text: self.include_text,
            output: self.output,
        })
    }
}

/// A module whose compiler and physical provenance work has completed.
/// Only contextual filesystem writes remain; this owner retains no logical map
/// or syntax tree and cannot invoke generation during publication.
struct ReadyEmission {
    /// The exact formatted Rust produced by the original generator invocation.
    source: String,
    /// Serialized map with final basename and canonical physical locations.
    source_map: Vec<u8>,
    /// Already selected normal-module loader and explicit public re-exports.
    include_text: String,
    /// Successful result descriptors, withheld until the complete batch publishes.
    output: BuildOutput,
}

impl ReadyEmission {
    /// Publishes one fully finished module using existing write-if-changed rules.
    /// A write failure can leave partial files; no rollback is promised.
    fn write(self) -> Result<BuildOutput, BuildError> {
        write_changed(&self.output.bindings, self.source.as_bytes())?;
        write_changed(&self.output.source_map, &self.source_map)?;
        write_changed(&self.output.include, self.include_text.as_bytes())?;
        Ok(self.output)
    }
}

/// Writes completed compiler results and their already encoded source graph.
/// This shared filesystem owner never regenerates or serializes the source graph.
/// Every map must finish before any main artifact is changed. Bundled-source
/// copies/directories may change during finishing; actual publication can be
/// partial on IO failure. Callers advertise success only after this returns.
pub(super) fn write_emissions(
    emissions: Vec<ModuleEmission>,
    roots: &BTreeMap<String, PathBuf>,
    resolved: &ResolvedModules,
    output: &Path,
    graph_bytes: &[u8],
) -> Result<Vec<BuildOutput>, BuildError> {
    let ready = emissions
        .into_iter()
        .map(|emission| emission.finish(roots, resolved, output))
        .collect::<Result<Vec<_>, _>>()?;
    // This second batch barrier is separate from generation: a later source
    // can disappear or fail canonicalization after all Rust generation succeeds.
    let results = ready.into_iter().map(ReadyEmission::write).collect::<Result<Vec<_>, _>>()?;
    write_changed(&output.join("neocogi-idl-graph.json"), graph_bytes)?;
    Ok(results)
}

#[cfg(test)]
mod tests;
