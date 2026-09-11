//! Owned, offline views of plugin descriptor metadata.
//!
//! Offline inspection opens the library but never registers the plugin. Every
//! string and relationship is copied into owned storage. Once opened, the
//! module remains pinned until the short-lived inspector process exits;
//! platform loader side effects may still occur merely by opening it. The same
//! validated-and-pinned descriptor helper backs the shared runtime's synchronous
//! callback API, while tooling copies the complete graph into the owned records
//! defined here.

use std::mem::ManuallyDrop;
use std::path::Path;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{
    NC_E_INVALID_ARG, NC_E_POINTER, NcAbiSignature, NcAbiThreadingMode, NcAbiValidationMode, NcClassDescriptor, NcClassInterfaceDescriptor, NcGuid,
    NcInterfaceDescriptor, NcMethodDescriptor, NcPluginDescriptor, NcStaticUtf8, NcStatus, format_abi_signature, format_method_signature,
    validate_plugin_descriptor_with_issues,
};

use super::{LoadedModule, PluginLoadError, PluginLoadErrorKind, plugin_descriptor_from_module};

/// Owned metadata extracted from an neocogi-com plugin shared library.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginInspection {
    /// Core ABI version advertised by the plugin descriptor.
    pub core_abi_version: u32,
    /// ABI signature representation declared for the entire plugin graph.
    pub validation_mode: NcAbiValidationMode,
    /// Stable plugin identifier.
    pub plugin_id: NcGuid,
    /// Static plugin qualified name copied into owned storage.
    pub qualified_name: String,
    /// Static plugin version metadata copied into owned storage.
    pub plugin_version: String,
    /// Stable identities of hard plugin dependencies.
    pub dependencies: Vec<NcGuid>,
    /// Constructible classes exported by the plugin.
    pub classes: Vec<ClassInspection>,
}

/// Owned metadata for one constructible class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassInspection {
    /// Stable class identifier.
    pub class_id: NcGuid,
    /// Static class qualified name copied into owned storage.
    pub qualified_name: String,
    /// Static class version metadata copied into owned storage.
    pub class_version: String,
    /// Owned constructor-frame ABI mangle, or an empty string for no arguments.
    pub constructor_abi_signature: String,
    /// Threading capability promised by constructed objects.
    pub threading: NcAbiThreadingMode,
    /// Interfaces implemented by this class.
    pub interfaces: Vec<ClassInterfaceInspection>,
}

/// Owned metadata for one class/interface relationship.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassInterfaceInspection {
    /// Interface ID advertised by the class.
    pub interface_id: NcGuid,
    /// Full interface descriptor copied from the descriptor graph.
    pub interface: InterfaceInspection,
}

/// Owned identity of one optional base-interface contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfaceIdentityInspection {
    /// Stable interface GUID, or nil when no base is present.
    pub iid: NcGuid,
    /// Owned canonical ABI mangle, or empty when no base is present.
    pub abi_signature: String,
}

impl InterfaceIdentityInspection {
    /// Returns whether this is the canonical absent-base identity.
    pub fn is_none(&self) -> bool {
        self.iid.is_nil() && self.abi_signature.is_empty()
    }
}

/// Owned metadata for one ABI interface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfaceInspection {
    /// Stable interface ID.
    pub interface_id: NcGuid,
    /// Normative qualified interface name copied into owned storage.
    pub qualified_name: String,
    /// Owned canonical mangle of the complete lowered interface ABI contract.
    pub abi_signature: String,
    /// Explicit base contract, or NONE for the implicit IUnknown prefix.
    pub base: InterfaceIdentityInspection,
    /// Locally introduced method metadata; inherited slots belong to `base`.
    pub methods: Vec<MethodInspection>,
}

/// Owned metadata for one ABI method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodInspection {
    /// Zero-based local method index after the complete inherited prefix.
    pub index: u32,
    /// Static method name copied into owned storage.
    pub name: String,
    /// Diagnostic typed ABI signature text rendered from method descriptors.
    pub signature: String,
}

/// Loads an neocogi-com plugin shared library and extracts its descriptor metadata.
///
/// This does not register the plugin in the process-global runtime, construct
/// plugin objects, or load declared neocogi-com dependencies. It still loads the
/// dynamic library, so normal platform loader behavior for that library can run
/// before the descriptor symbol is queried. Once opened, the library is pinned
/// until process exit, including when symbol or metadata inspection fails. This
/// operation is intended for a short-lived inspector process.
///
/// # Safety
///
/// `path` must identify a trusted neocogi-com plugin library. Loading a dynamic
/// library can run arbitrary platform loader code, and the exported descriptor
/// symbol is trusted FFI metadata until validation rejects it.
pub unsafe fn inspect_plugin_library(path: impl AsRef<Path>) -> Result<PluginInspection, PluginLoadError> {
    let path = path.as_ref();
    // SAFETY: The caller trusts this library as an neocogi-com plugin.
    let plugin = unsafe { inspect_plugin_descriptor(path)? };

    // SAFETY: Validation proved the descriptor graph is readable. The module
    // was pinned by `inspect_plugin_descriptor`, and the copied inspection
    // value owns all text.
    unsafe { inspection_from_descriptor(path, &*plugin) }
}

/// Opens and validates a plugin descriptor without registering it.
///
/// A successfully opened library is pinned until process exit, including when
/// symbol lookup or descriptor validation subsequently fails.
///
/// # Safety
///
/// `path` must identify a trusted neocogi-com plugin library. Loading a dynamic
/// library can run arbitrary platform loader code, and the returned pointer is
/// meaningful only while the pinned module remains mapped.
pub(crate) unsafe fn inspect_plugin_descriptor(path: &Path) -> Result<*const NcPluginDescriptor, PluginLoadError> {
    // Inspection uses process exit as its unload boundary. `ManuallyDrop` is
    // established immediately after a successful open so later errors and
    // unwinding cannot call `dlclose`/`FreeLibrary` while plugin code may have
    // escaped from loader hooks or the descriptor entrypoint.
    let module = ManuallyDrop::new(LoadedModule::open(path)?);
    // SAFETY: The caller trusts this library as an neocogi-com plugin.
    let plugin = unsafe { plugin_descriptor_from_module(&module, path)? };

    // SAFETY: `plugin` came from the loaded module's descriptor entrypoint.
    // Validation only reads the descriptor graph.
    if let Err(status) = unsafe { validate_descriptor(plugin) } {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(status),
            "plugin descriptor failed inspection validation",
        ));
    }

    Ok(plugin)
}

/// Runs the shared whole-graph descriptor validation gate without collecting issues.
pub(super) unsafe fn validate_descriptor(plugin: *const NcPluginDescriptor) -> Result<(), NcStatus> {
    // Loading and offline inspection intentionally share this validation gate;
    // they must not disagree about which descriptor graphs are readable.
    // SAFETY: `plugin` came from the loaded module's descriptor entrypoint and
    // validation only reads the descriptor graph.
    let status = unsafe { validate_plugin_descriptor_with_issues(plugin, |_| {}) };
    if status.is_err() {
        return Err(status);
    }
    Ok(())
}

/// Copies one validated plugin root and all reachable metadata into owned records.
unsafe fn inspection_from_descriptor(path: &Path, plugin: &NcPluginDescriptor) -> Result<PluginInspection, PluginLoadError> {
    // SAFETY: The caller keeps the module live and validated this descriptor.
    let qualified_name = unsafe { copy_static_utf8(path, plugin.qualified_name, "plugin qualified name is not valid static UTF-8")? };
    // SAFETY: Same validated descriptor graph as above.
    let plugin_version = unsafe { copy_static_utf8(path, plugin.plugin_version, "plugin version is not valid static UTF-8")? };

    // SAFETY: Validation proved the dependency table is readable while the
    // inspection process keeps the module pinned.
    let dependencies = unsafe { plugin.dependencies() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "plugin dependency table is invalid"))?
        .iter()
        .map(|dependency| dependency.plugin_id)
        .collect();

    // SAFETY: The descriptor was validated before this copy.
    let classes = unsafe { copy_classes(path, plugin)? };

    Ok(PluginInspection {
        core_abi_version: plugin.core_abi_version,
        validation_mode: plugin.validation_mode,
        plugin_id: plugin.plugin_id,
        qualified_name,
        plugin_version,
        dependencies,
        classes,
    })
}

/// Copies the validated class table in descriptor order.
unsafe fn copy_classes(path: &Path, plugin: &NcPluginDescriptor) -> Result<Vec<ClassInspection>, PluginLoadError> {
    if plugin.class_table.is_null() {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(NC_E_POINTER),
            "plugin class table is null",
        ));
    }
    // SAFETY: Validation proved this class table pointer is readable.
    let class_table = unsafe { &*plugin.class_table };
    // SAFETY: The class table was validated before this copy.
    let classes = unsafe { class_table.classes() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "plugin class table is invalid"))?;
    let mut out = Vec::with_capacity(classes.len());
    for class in classes {
        out.push(copy_class(path, class)?);
    }
    Ok(out)
}

/// Copies one class's text, constructor contract, threading, and interfaces.
fn copy_class(path: &Path, class: &NcClassDescriptor) -> Result<ClassInspection, PluginLoadError> {
    // SAFETY: The descriptor was validated before this copy.
    let qualified_name = unsafe { copy_static_utf8(path, class.qualified_name, "class qualified name is not valid static UTF-8")? };
    // SAFETY: The descriptor was validated before this copy.
    let class_version = unsafe { copy_static_utf8(path, class.class_version, "class version is not valid static UTF-8")? };

    let interfaces = unsafe { class.interfaces() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "class interface table is invalid"))?;
    let mut copied_interfaces = Vec::with_capacity(interfaces.len());
    for interface in interfaces {
        copied_interfaces.push(copy_class_interface(path, interface)?);
    }

    let threading = NcAbiThreadingMode::from_raw(class.threading)
        .ok_or_else(|| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(NC_E_INVALID_ARG), "class threading mode is invalid"))?;
    Ok(ClassInspection {
        class_id: class.class_id,
        qualified_name,
        class_version,
        constructor_abi_signature: copy_abi_signature(path, class.constructor_abi_signature, "class constructor ABI signature is invalid")?,
        threading,
        interfaces: copied_interfaces,
    })
}

/// Copies one class/interface edge and its referenced interface metadata.
fn copy_class_interface(path: &Path, class_interface: &NcClassInterfaceDescriptor) -> Result<ClassInterfaceInspection, PluginLoadError> {
    if class_interface.interface_descriptor.is_null() {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(NC_E_POINTER),
            "class interface descriptor pointer is null",
        ));
    }

    // SAFETY: Validation proved this descriptor pointer is readable.
    let interface = unsafe { &*class_interface.interface_descriptor };
    let interface = copy_interface(path, interface)?;
    Ok(ClassInterfaceInspection {
        interface_id: interface.interface_id,
        interface,
    })
}

/// Copies one interface identity, ABI signature, name, and ordered methods.
fn copy_interface(path: &Path, interface: &NcInterfaceDescriptor) -> Result<InterfaceInspection, PluginLoadError> {
    // SAFETY: The descriptor was validated before this copy.
    let qualified_name = unsafe { copy_static_utf8(path, interface.qualified_name, "interface qualified name is not valid static UTF-8")? };
    let methods = unsafe { interface.methods() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "interface method table is invalid"))?;
    let mut copied_methods = Vec::with_capacity(methods.len());
    for (index, method) in methods.iter().enumerate() {
        copied_methods.push(copy_method(path, index as u32, method)?);
    }

    Ok(InterfaceInspection {
        interface_id: interface.interface_id,
        qualified_name,
        abi_signature: copy_abi_signature(path, interface.abi_signature, "interface ABI signature is invalid")?,
        base: InterfaceIdentityInspection {
            iid: interface.base.iid,
            abi_signature: copy_abi_signature(path, interface.base.abi_signature, "base-interface ABI signature is invalid")?,
        },
        methods: copied_methods,
    })
}

/// Copies one method name and renders its validated diagnostic signature.
fn copy_method(path: &Path, index: u32, method: &NcMethodDescriptor) -> Result<MethodInspection, PluginLoadError> {
    // SAFETY: The descriptor was validated before this copy.
    let name = unsafe { copy_static_utf8(path, method.name, "method name is not valid static UTF-8")? };
    // SAFETY: The loaded module remains mapped for this copy, and validation
    // established the method descriptor's complete pointer graph.
    let signature = unsafe { format_method_signature(method) }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "method typed ABI descriptors are invalid"))?;

    Ok(MethodInspection { index, name, signature })
}

/// Copies one validated component-owned ABI signature into offline storage.
fn copy_abi_signature(path: &Path, signature: NcAbiSignature, message: &'static str) -> Result<String, PluginLoadError> {
    // SAFETY: Descriptor validation established the signature view, and the
    // exporting module remains pinned throughout this synchronous copy.
    unsafe { format_abi_signature(signature) }.map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), message))
}

/// Copies validated module-owned static UTF-8 into independent `String` storage.
unsafe fn copy_static_utf8(path: &Path, value: NcStaticUtf8, message: &'static str) -> Result<String, PluginLoadError> {
    // SAFETY: The caller guarantees the descriptor graph remains live for this
    // copy. UTF-8 validity is checked by `as_str`.
    unsafe { value.as_str() }
        .map(str::to_owned)
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), message))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use neocogi_com_core::NcAbiSignature;

    use super::copy_abi_signature;

    /// Inspection preserves textual signatures and renders binary validation
    /// fingerprints as stable algorithm-prefixed hexadecimal text.
    #[test]
    fn copied_signatures_support_every_validation_representation() {
        static SHA_256: [u8; 32] = [0xff; 32];
        static FNV_1A_128: [u8; 16] = [0x80; 16];
        let path = Path::new("plugin");

        assert_eq!(copy_abi_signature(path, NcAbiSignature::EMPTY, "invalid").unwrap(), "");
        assert_eq!(copy_abi_signature(path, NcAbiSignature::UNCHECKED, "invalid").unwrap(), "ncabi1;unchecked");
        assert_eq!(
            copy_abi_signature(path, NcAbiSignature::from_static(&SHA_256), "invalid").unwrap(),
            format!("sha-256:{}", "ff".repeat(32))
        );
        assert_eq!(
            copy_abi_signature(path, NcAbiSignature::from_static(&FNV_1A_128), "invalid").unwrap(),
            format!("fnv-1a-128:{}", "80".repeat(16))
        );
    }
}
