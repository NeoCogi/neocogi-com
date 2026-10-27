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
    let mut bindings = generate_rust_with_paths_and_source_map(resolved, module_name, paths).map_err(|source| BuildError::Idl(source.to_string()))?;
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
    /// Finishes actual-file provenance and publishes this completed generation.
    /// Filesystem errors can leave partial output; no compiler work is repeated.
    pub(super) fn write(mut self, roots: &BTreeMap<String, PathBuf>, resolved: &ResolvedModules, output: &Path) -> Result<BuildOutput, BuildError> {
        source_map::resolve_locations(&mut self.generated.source_map, output, roots, resolved)?;
        let map_bytes = serde_json::to_vec_pretty(&self.generated.source_map).map_err(|source| BuildError::Json {
            operation: "encode Rust source map",
            path: self.output.source_map.clone(),
            source,
        })?;
        write_changed(&self.output.bindings, self.generated.source.as_bytes())?;
        write_changed(&self.output.source_map, &map_bytes)?;
        write_changed(&self.output.include, self.include_text.as_bytes())?;
        Ok(self.output)
    }
}
