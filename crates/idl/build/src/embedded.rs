//! Generated COM built-in descriptions embedded in the one shared runtime.
//!
//! This path resolves the same authored NCIDL used by the typed contract crate,
//! but emits a Rust static using the canonical `neocogi-com-core` layouts. It
//! neither compiles nor links a second native description library.

use std::{env, fmt::Write as _, path::PathBuf, string::String};

use neocogi_com_core::NcGuid;
use neocogi_idl::{COM_EVENTS_MODULE_NAME, ResolvedModule, resolve_modules};

use crate::{
    ModuleSources,
    cargo::{BuildError, write_changed},
    native::{declaration_kind, declaration_threading},
};

/// Cargo build request for a generated runtime-owned description.
pub struct EmbeddedModule {
    /// Directory containing the canonical NCIDL module path.
    idl_root: PathBuf,
    /// Relative module directory whose path determines its logical name.
    module_directory: PathBuf,
}

impl EmbeddedModule {
    /// Selects one multipart IDL module below its canonical source root.
    pub fn directory(idl_root: impl Into<PathBuf>, module_directory: impl Into<PathBuf>) -> Self {
        Self {
            idl_root: idl_root.into(),
            module_directory: module_directory.into(),
        }
    }

    /// Renders the runtime-owned description without requiring a Cargo build.
    ///
    /// Source-packaging tests can use this pure path while production build
    /// scripts use [`Self::build`] for tracking and output ownership.
    pub fn generate(&self) -> Result<String, BuildError> {
        let sources = ModuleSources::load_directory(&self.idl_root, &self.module_directory)?;
        render_sources(&sources)
    }

    /// Generates immutable COM events metadata into this runtime build's `OUT_DIR`.
    ///
    /// The caller includes the returned source file in the runtime crate. An
    /// unrelated module or an events module with foreign dependencies is
    /// rejected rather than silently becoming a second runtime built-in.
    pub fn build(self) -> Result<PathBuf, BuildError> {
        let manifest = env::var_os("CARGO_MANIFEST_DIR").ok_or(BuildError::MissingEnvironment("CARGO_MANIFEST_DIR"))?;
        let output = env::var_os("OUT_DIR").ok_or(BuildError::MissingEnvironment("OUT_DIR"))?;
        let root = PathBuf::from(manifest).join(self.idl_root);
        let path = root.join(&self.module_directory);
        println!("cargo::rerun-if-changed={}", path.display());
        let sources = ModuleSources::load_directory(&root, &self.module_directory)?;
        for (part, _) in sources.parts() {
            println!("cargo::rerun-if-changed={}", root.join(part).display());
        }
        let source = render_sources(&sources)?;
        let generated = PathBuf::from(output).join("com_builtin_events.rs");
        write_changed(&generated, source.as_bytes())?;
        Ok(generated)
    }
}

/// Resolves the canonical source parts and rejects foreign runtime dependencies.
fn render_sources(sources: &ModuleSources) -> Result<String, BuildError> {
    let parsed = sources.parse()?;
    if parsed.name.to_string() != COM_EVENTS_MODULE_NAME {
        return Err(BuildError::Idl(format!("only {COM_EVENTS_MODULE_NAME} is an embedded COM runtime contract")));
    }
    let graph = resolve_modules(vec![parsed]).map_err(|error| BuildError::Idl(error.to_string()))?;
    let module = graph
        .module(COM_EVENTS_MODULE_NAME)
        .ok_or_else(|| BuildError::Idl("resolved graph lost COM events".into()))?;
    if !module.dependencies.is_empty() {
        return Err(BuildError::Idl(
            "the runtime-owned COM events contract cannot depend on a foreign description library".into(),
        ));
    }
    Ok(render_description(module))
}

/// Renders one immutable description from resolved NCIDL identities.
///
/// Its generated statics use the public ABI layouts directly, so there is no
/// handwritten mirror of the runtime-owned events description.
fn render_description(module: &ResolvedModule) -> String {
    let mut source = String::from("// Generated from neocogi.com.events NCIDL; edit the authored .ncidl instead.\n\n");
    source.push_str("use neocogi_com_core::{NC_CONTRACT_DESCRIPTION_ABI_VERSION, NcContractDeclarationDescription, NcContractFingerprint, NcContractModuleDescription, NcGuid, NcStaticUtf8};\n\n");
    writeln!(
        source,
        "/// Stable ID of the runtime-owned COM events module.\npub(crate) const MODULE_ID: NcGuid = {};",
        core_guid(module.module_id)
    )
    .expect("writing to String cannot fail");
    writeln!(
        source,
        "/// Exact IDL ABI revision compiled into this runtime.\npub(crate) const ABI_FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new({});\n",
        bytes(&module.abi_fingerprint)
    )
    .expect("writing to String cannot fail");
    source.push_str("/// Source-ordered declarations in the intrinsic COM events module.\nstatic DECLARATIONS: &[NcContractDeclarationDescription] = &[\n");
    for (identity, declaration) in module.declarations.iter().zip(&module.source.declarations) {
        writeln!(
            source,
            "    NcContractDeclarationDescription {{ declaration_id: {}, version: {}, kind: {}, threading: {}, qualified_name: NcStaticUtf8::from_static({:?}) }},",
            core_guid(identity.id),
            identity.version,
            declaration_kind(&declaration.kind),
            declaration_threading(&declaration.kind),
            identity.qualified_name,
        )
        .expect("writing to String cannot fail");
    }
    source.push_str("];\n\n/// Canonical immutable description owned by the shared COM runtime.\nstatic DESCRIPTION: NcContractModuleDescription = NcContractModuleDescription {\n");
    writeln!(source, "    description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,").expect("writing to String cannot fail");
    source.push_str("    module_id: MODULE_ID,\n    abi_fingerprint: ABI_FINGERPRINT,\n");
    writeln!(source, "    module_name: NcStaticUtf8::from_static({COM_EVENTS_MODULE_NAME:?}),").expect("writing to String cannot fail");
    source.push_str("    dependency_count: 0,\n    dependencies: core::ptr::null(),\n");
    writeln!(source, "    declaration_count: {},", module.declarations.len()).expect("writing to String cannot fail");
    source.push_str("    declarations: DECLARATIONS.as_ptr(),\n};\n\n");
    source.push_str("/// Borrows the runtime-owned description without invoking a foreign getter.\npub(crate) fn description() -> &'static NcContractModuleDescription {\n    &DESCRIPTION\n}\n");
    source
}

/// Renders a public-core GUID constructor from resolved canonical fields.
fn core_guid(guid: NcGuid) -> String {
    let (data1, data2, data3, data4) = guid.fields();
    format!("NcGuid::from_fields({data1}, {data2}, {data3}, {})", bytes(&data4))
}

/// Renders an integer byte array without creating ABI-owned duplicate types.
fn bytes<const N: usize>(value: &[u8; N]) -> String {
    format!("[{}]", value.iter().map(u8::to_string).collect::<Vec<_>>().join(", "))
}
