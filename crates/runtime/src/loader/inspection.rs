//! Owned offline views of factory plugin and canonical IDL metadata.
//!
//! Inspection opens a trusted library without registering it, validates the
//! same factory description and contract graph used by live activation, and
//! copies every reported value into owned storage. The opened module remains
//! pinned until the inspector process exits because returned descriptions and
//! contract getters point into native modules.

use std::mem::ManuallyDrop;
use std::path::Path;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{
    NC_CONTRACT_DECLARATION_CLASS, NC_CONTRACT_DECLARATION_ENUM, NC_CONTRACT_DECLARATION_FACTORY, NC_CONTRACT_DECLARATION_INTERFACE,
    NC_CONTRACT_DECLARATION_STRUCT, NC_E_INVALID_ARG, NcAbiThreadingMode, NcAbiValidationMode, NcContractFingerprint, NcFactoryPluginDescription, NcGuid,
    NcStaticUtf8,
};

use crate::contract_registry::RegisteredContractModule;
use crate::factory_registration::build_factory_plugin_registration_batch;

use super::{LoadedModule, PluginLoadError, PluginLoadErrorKind, factory_plugin_description_from_entrypoint};

/// Owned metadata extracted from one factory plugin shared library.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginInspection {
    /// Core ABI version advertised by the plugin description.
    pub core_abi_version: u32,
    /// ABI signature representation used by the plugin and its contracts.
    pub validation_mode: NcAbiValidationMode,
    /// Stable plugin identifier derived from its qualified name.
    pub plugin_id: NcGuid,
    /// Canonical qualified plugin name copied into owned storage.
    pub qualified_name: String,
    /// Plugin version text copied into owned storage.
    pub plugin_version: String,
    /// Stable identities of hard factory-plugin dependencies.
    pub dependencies: Vec<NcGuid>,
    /// Typed root factory and complete canonical contract graph.
    pub factory: FactoryInspection,
}

impl PluginInspection {
    /// Returns the only activation model supported by this runtime generation.
    pub const fn activation_model(&self) -> &'static str {
        "factory"
    }
}

/// Owned metadata for a plugin's root factory and canonical contract graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactoryInspection {
    /// IDL declaration identity returned by the plugin bootstrap operation.
    pub root_factory_id: NcGuid,
    /// Threading capability promised by the returned factory object.
    pub root_factory_threading: NcAbiThreadingMode,
    /// Exact module revisions referenced directly by generated plugin bindings.
    pub direct_contracts: Vec<ContractReferenceInspection>,
    /// Complete validated graph in dependency-first order.
    pub modules: Vec<ContractModuleInspection>,
}

/// Owned identity of one exact native contract module revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContractReferenceInspection {
    /// Stable logical module identity.
    pub module_id: NcGuid,
    /// Complete transitive ABI fingerprint.
    pub abi_fingerprint: NcContractFingerprint,
}

/// Owned canonical description of one IDL contract module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractModuleInspection {
    /// Stable logical module identity.
    pub module_id: NcGuid,
    /// Complete transitive ABI fingerprint.
    pub abi_fingerprint: NcContractFingerprint,
    /// Exact dotted logical module name.
    pub module_name: String,
    /// Direct exact dependencies in canonical module-name order.
    pub dependencies: Vec<ContractReferenceInspection>,
    /// Top-level declarations retained in source order.
    pub declarations: Vec<ContractDeclarationInspection>,
    /// Canonical `.ncidl` source stored by the contract library.
    pub source: String,
}

/// Owned identity and capability of one top-level IDL declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractDeclarationInspection {
    /// Stable derived or explicitly pinned declaration identity.
    pub declaration_id: NcGuid,
    /// Positive IDL declaration version.
    pub version: u32,
    /// Source-level declaration category.
    pub kind: ContractDeclarationKind,
    /// Capability for classes/factories and `None` for data/interfaces.
    pub threading: Option<NcAbiThreadingMode>,
    /// Exact dotted module and declared name.
    pub qualified_name: String,
}

/// Source-level kind of one canonical IDL declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractDeclarationKind {
    /// C-layout value record.
    Struct,
    /// Tagged value union.
    Enum,
    /// COM interface trait.
    Interface,
    /// Factory-created object contract.
    Class,
    /// Plugin or service construction interface.
    Factory,
}

impl ContractDeclarationKind {
    /// Returns the exact IDL keyword represented by this kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "trait",
            Self::Class => "class",
            Self::Factory => "factory",
        }
    }
}

/// Loads a trusted factory plugin and copies its canonical metadata.
///
/// This operation does not register the plugin, activate its root factory, or
/// load declared plugin dependencies. Platform loader initialization may still
/// run while opening the library. The module is pinned even when later symbol
/// lookup or metadata validation fails.
///
/// # Safety
///
/// `path` must identify trusted native plugin code. Opening a dynamic library
/// can execute arbitrary platform initialization.
pub unsafe fn inspect_plugin_library(path: impl AsRef<Path>) -> Result<PluginInspection, PluginLoadError> {
    let path = path.as_ref();
    let module = ManuallyDrop::new(LoadedModule::open(path)?);
    // SAFETY: The caller trusts the module and the symbol has the fixed factory
    // description ABI defined by neocogi-com-core.
    let get = unsafe { module.get_factory_plugin_description(path)? };
    // SAFETY: The pinned module backs the returned root and linked getters.
    let plugin = unsafe { factory_plugin_description_from_entrypoint(get, path)? };
    // SAFETY: The pinned module keeps the complete graph live during copying.
    unsafe { inspection_from_factory_description(path, plugin) }
}

/// Opens and validates a factory description for a synchronous C callback.
///
/// A successfully opened module is pinned until process exit, including when
/// symbol lookup or graph validation subsequently fails.
///
/// # Safety
///
/// `path` must identify trusted native plugin code. The returned pointer may be
/// used only according to the immutable factory-description ABI.
pub(crate) unsafe fn inspect_factory_plugin_description(path: &Path) -> Result<*const NcFactoryPluginDescription, PluginLoadError> {
    let module = ManuallyDrop::new(LoadedModule::open(path)?);
    // SAFETY: The trusted pinned module owns the required fixed-name symbol.
    let get = unsafe { module.get_factory_plugin_description(path)? };
    // SAFETY: The module remains pinned after this helper returns.
    let plugin = unsafe { factory_plugin_description_from_entrypoint(get, path)? };
    // SAFETY: The pinned module and linked contract libraries retain the graph.
    unsafe { build_factory_plugin_registration_batch(plugin) }.map_err(|status| {
        PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(status),
            "factory plugin description failed inspection validation",
        )
    })?;
    Ok(plugin)
}

/// Validates and copies one factory description plus its full contract graph.
unsafe fn inspection_from_factory_description(path: &Path, plugin: *const NcFactoryPluginDescription) -> Result<PluginInspection, PluginLoadError> {
    // SAFETY: The pinned module and native dependencies retain every pointer
    // and getter reached by batch construction.
    let batch = unsafe { build_factory_plugin_registration_batch(plugin) }.map_err(|status| {
        PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(status),
            "factory plugin description failed inspection validation",
        )
    })?;
    // SAFETY: Successful validation rejected null and retained this root in the
    // pinned module.
    let plugin = unsafe { &*plugin };
    // SAFETY: Factory validation established readable UTF-8 metadata.
    let qualified_name = unsafe { copy_static_utf8(path, plugin.qualified_name, "plugin qualified name is not valid static UTF-8")? };
    // SAFETY: Factory validation established readable UTF-8 metadata.
    let plugin_version = unsafe { copy_static_utf8(path, plugin.plugin_version, "plugin version is not valid static UTF-8")? };
    // SAFETY: Factory validation established the dependency table shape.
    let dependencies = unsafe { plugin.dependencies() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "plugin dependency table is invalid"))?
        .iter()
        .map(|dependency| dependency.plugin_id)
        .collect();
    // SAFETY: Factory validation established the direct reference table shape.
    let direct_contracts = unsafe { plugin.contracts() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "plugin contract table is invalid"))?
        .iter()
        .map(|reference| ContractReferenceInspection {
            module_id: reference.module_id,
            abi_fingerprint: reference.abi_fingerprint,
        })
        .collect();
    let mut modules = Vec::with_capacity(batch.modules().len());
    for module in batch.modules() {
        modules.push(copy_contract_module(path, module)?);
    }
    let root_factory_threading = NcAbiThreadingMode::from_raw(plugin.root_factory_threading).ok_or_else(|| {
        PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(NC_E_INVALID_ARG),
            "root factory threading mode is invalid",
        )
    })?;

    Ok(PluginInspection {
        core_abi_version: plugin.core_abi_version,
        validation_mode: plugin.validation_mode,
        plugin_id: plugin.plugin_id,
        qualified_name,
        plugin_version,
        dependencies,
        factory: FactoryInspection {
            root_factory_id: plugin.root_factory_id,
            root_factory_threading,
            direct_contracts,
            modules,
        },
    })
}

/// Copies one already validated canonical module record into owned storage.
fn copy_contract_module(path: &Path, module: &RegisteredContractModule) -> Result<ContractModuleInspection, PluginLoadError> {
    // SAFETY: Registration batch validation retained only readable immutable roots.
    let description = unsafe { &*module.description };
    // SAFETY: Whole-module validation established the dependency table shape.
    let dependencies = unsafe { description.dependencies() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "contract dependency table is invalid"))?
        .iter()
        .map(|dependency| ContractReferenceInspection {
            module_id: dependency.module_id,
            abi_fingerprint: dependency.abi_fingerprint,
        })
        .collect();
    // SAFETY: Whole-module validation established the declaration table shape.
    let raw_declarations = unsafe { description.declarations() }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "contract declaration table is invalid"))?;
    let mut declarations = Vec::with_capacity(raw_declarations.len());
    for declaration in raw_declarations {
        let kind = contract_declaration_kind(path, declaration.kind)?;
        let threading = match kind {
            ContractDeclarationKind::Class | ContractDeclarationKind::Factory => {
                Some(NcAbiThreadingMode::from_raw(declaration.threading).ok_or_else(|| {
                    PluginLoadError::new(
                        PluginLoadErrorKind::Metadata,
                        path,
                        Some(NC_E_INVALID_ARG),
                        "contract declaration threading mode is invalid",
                    )
                })?)
            }
            ContractDeclarationKind::Struct | ContractDeclarationKind::Enum | ContractDeclarationKind::Interface => None,
        };
        // SAFETY: Whole-module validation established declaration-name UTF-8.
        let qualified_name = unsafe {
            copy_static_utf8(
                path,
                declaration.qualified_name,
                "contract declaration qualified name is not valid static UTF-8",
            )?
        };
        declarations.push(ContractDeclarationInspection {
            declaration_id: declaration.declaration_id,
            version: declaration.version,
            kind,
            threading,
            qualified_name,
        });
    }
    // SAFETY: Whole-module validation established module source UTF-8.
    let source = unsafe { copy_static_utf8(path, description.source, "contract module source is not valid static UTF-8")? };
    Ok(ContractModuleInspection {
        module_id: module.module_id,
        abi_fingerprint: module.abi_fingerprint,
        module_name: module.module_name.clone(),
        dependencies,
        declarations,
        source,
    })
}

/// Converts a validated raw declaration tag to its source-level enum.
fn contract_declaration_kind(path: &Path, kind: u32) -> Result<ContractDeclarationKind, PluginLoadError> {
    match kind {
        NC_CONTRACT_DECLARATION_STRUCT => Ok(ContractDeclarationKind::Struct),
        NC_CONTRACT_DECLARATION_ENUM => Ok(ContractDeclarationKind::Enum),
        NC_CONTRACT_DECLARATION_INTERFACE => Ok(ContractDeclarationKind::Interface),
        NC_CONTRACT_DECLARATION_CLASS => Ok(ContractDeclarationKind::Class),
        NC_CONTRACT_DECLARATION_FACTORY => Ok(ContractDeclarationKind::Factory),
        _ => Err(PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(NC_E_INVALID_ARG),
            "contract declaration kind is invalid",
        )),
    }
}

/// Copies validated module-owned static UTF-8 into independent storage.
unsafe fn copy_static_utf8(path: &Path, value: NcStaticUtf8, message: &'static str) -> Result<String, PluginLoadError> {
    // SAFETY: The caller guarantees the graph remains live for this copy;
    // `as_str` performs the UTF-8 validation.
    unsafe { value.as_str() }
        .map(str::to_owned)
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), message))
}
