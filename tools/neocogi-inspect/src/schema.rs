//! Versioned non-reconstructive inspection artifact and semantic validation.
//!
//! The serialized model is deliberately separate from runtime inspection
//! structs. It records compatibility identities and derived diagnostics, but
//! does not expose structured method or aggregate definitions as a stable
//! reflection contract.

use std::collections::{BTreeMap, BTreeSet};

use neocogi_com_core::{IID_IUNKNOWN, IUNKNOWN_QUALIFIED_NAME, NcAbiThreadingMode, NcAbiValidationMode, NcGuid, guid_from_qualified_name};
#[cfg(test)]
use neocogi_com_core::{NcAbiSignature, NcInterfaceId, format_abi_signature};
use nc_com_shared_runtime::{ContractDeclarationKind, PluginInspection};
use serde::{Deserialize, Serialize};

pub(crate) const SCHEMA: &str = "neocogi-com.inspection";
pub(crate) const SCHEMA_VERSION: u32 = 6;
const PRODUCER: &str = "neocogi-inspect";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionArtifact {
    pub(crate) schema: String,
    pub(crate) schema_version: u32,
    pub(crate) producer: ArtifactProducer,
    pub(crate) target: ArtifactTarget,
    pub(crate) core_abi_version: u32,
    /// ABI signature representation used by every contract in this artifact.
    pub(crate) com_validation: ArtifactValidationMode,
    pub(crate) plugin: ArtifactPlugin,
    /// Mutually exclusive plugin activation metadata.
    pub(crate) activation: ArtifactActivation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactProducer {
    pub(crate) name: String,
    pub(crate) version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactTarget {
    pub(crate) triple: String,
    pub(crate) pointer_width: u8,
    pub(crate) endianness: ArtifactEndianness,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactEndianness {
    Little,
    Big,
}

/// Serialized spelling of one assembly-wide COM ABI validation mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ArtifactValidationMode {
    /// Structural descriptor validation without ABI signature comparison.
    #[serde(rename = "none")]
    None,
    /// SHA-256 digests of canonical ABI contract records.
    #[serde(rename = "sha-256")]
    Sha256,
    /// FNV-1a-128 fingerprints of canonical ABI contract records.
    #[serde(rename = "fnv-1a-128")]
    Fnv1a128,
    /// Complete canonical ABI contract records without hashing.
    #[serde(rename = "mangling")]
    Mangling,
}

impl ArtifactValidationMode {
    /// Converts a validated ABI descriptor mode to its serialized counterpart.
    fn from_abi(mode: NcAbiValidationMode) -> Option<Self> {
        match mode {
            NcAbiValidationMode::NONE => Some(Self::None),
            NcAbiValidationMode::SHA_256 => Some(Self::Sha256),
            NcAbiValidationMode::FNV_1A_128 => Some(Self::Fnv1a128),
            NcAbiValidationMode::MANGLING => Some(Self::Mangling),
            _ => None,
        }
    }

    /// Returns the canonical assembly/configuration spelling.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sha256 => "sha-256",
            Self::Fnv1a128 => "fnv-1a-128",
            Self::Mangling => "mangling",
        }
    }
}

impl std::fmt::Display for ArtifactValidationMode {
    /// Formats the mode exactly as it appears in JSON and assembly configuration.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactPlugin {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) dependencies: Vec<String>,
}

/// Serialized activation model with no meaningless cross-model fields.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactActivation {
    /// One typed root factory backed by canonical IDL modules.
    Factory(ArtifactFactory),
    /// Historical runtime-constructible class table.
    LegacyClassTable {
        /// Constructible classes retained in stable descriptor order.
        classes: Vec<ArtifactClass>,
    },
}

/// Serialized factory identity and complete canonical module graph.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactFactory {
    /// Stable IDL identity returned by plugin bootstrap.
    pub(crate) root_factory_id: String,
    /// Threading capability promised by the root factory.
    pub(crate) root_factory_threading: ArtifactThreading,
    /// Exact module revisions directly referenced by plugin bindings.
    pub(crate) direct_contracts: Vec<ArtifactContractReference>,
    /// Complete dependency-first canonical module graph.
    pub(crate) modules: Vec<ArtifactContractModule>,
}

/// Serialized identity of one exact contract revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactContractReference {
    /// Stable logical module identity.
    pub(crate) module_id: String,
    /// Lowercase hexadecimal SHA-256 ABI fingerprint.
    pub(crate) abi_fingerprint: String,
}

/// Serialized canonical description of one contract module.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactContractModule {
    /// Stable logical module identity.
    pub(crate) id: String,
    /// Lowercase hexadecimal SHA-256 ABI fingerprint.
    pub(crate) abi_fingerprint: String,
    /// Exact dotted logical module name.
    pub(crate) name: String,
    /// Direct exact dependencies in canonical order.
    pub(crate) dependencies: Vec<ArtifactContractReference>,
    /// Top-level declarations retained in source order.
    pub(crate) declarations: Vec<ArtifactContractDeclaration>,
    /// Complete canonical `.ncidl` source stored by the module library.
    pub(crate) source: String,
}

/// Serialized identity and capability of one IDL declaration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactContractDeclaration {
    /// Stable derived or explicitly pinned identity.
    pub(crate) id: String,
    /// Positive source declaration version.
    pub(crate) version: u32,
    /// Rust-like IDL declaration category.
    pub(crate) kind: ArtifactContractDeclarationKind,
    /// Class/factory capability; absent for data and interfaces.
    pub(crate) threading: Option<ArtifactThreading>,
    /// Exact dotted module and declared name.
    pub(crate) name: String,
}

/// Serialized source-level declaration category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactContractDeclarationKind {
    /// C-layout value record.
    Struct,
    /// Tagged value union.
    Enum,
    /// COM interface trait.
    Trait,
    /// Factory-created object contract.
    Class,
    /// Plugin or service construction interface.
    Factory,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactClass {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) constructor_abi_signature: String,
    pub(crate) threading: ArtifactThreading,
    pub(crate) interfaces: Vec<ArtifactInterface>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactInterfaceId {
    pub(crate) iid: String,
    pub(crate) abi_signature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactInterface {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) abi_signature: String,
    /// Base contract identity; local methods follow this complete prefix.
    pub(crate) base: ArtifactInterfaceId,
    pub(crate) methods: Vec<ArtifactMethod>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactMethod {
    pub(crate) index: u32,
    pub(crate) name: String,
    /// Derived diagnostic text, not a stable machine-readable ABI definition.
    pub(crate) signature: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactThreading {
    ThreadLocal,
    ThreadSafe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArtifactValidationError {
    path: String,
    message: String,
}

impl ArtifactValidationError {
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ArtifactValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for ArtifactValidationError {}

impl From<&PluginInspection> for InspectionArtifact {
    fn from(value: &PluginInspection) -> Self {
        let mut dependencies: Vec<_> = value.dependencies.iter().map(ToString::to_string).collect();
        dependencies.sort();
        let factory = &value.factory;
        let mut direct_contracts: Vec<_> = factory.direct_contracts.iter().map(artifact_contract_reference).collect();
        direct_contracts.sort_by(|left, right| left.module_id.cmp(&right.module_id));
        let activation = ArtifactActivation::Factory(ArtifactFactory {
            root_factory_id: factory.root_factory_id.to_string(),
            root_factory_threading: artifact_threading(factory.root_factory_threading),
            direct_contracts,
            modules: factory
                .modules
                .iter()
                .map(|module| ArtifactContractModule {
                    id: module.module_id.to_string(),
                    abi_fingerprint: format_fingerprint(module.abi_fingerprint.bytes),
                    name: module.module_name.clone(),
                    dependencies: module.dependencies.iter().map(artifact_contract_reference).collect(),
                    declarations: module
                        .declarations
                        .iter()
                        .map(|declaration| ArtifactContractDeclaration {
                            id: declaration.declaration_id.to_string(),
                            version: declaration.version,
                            kind: artifact_declaration_kind(declaration.kind),
                            threading: declaration.threading.map(artifact_threading),
                            name: declaration.qualified_name.clone(),
                        })
                        .collect(),
                    source: module.source.clone(),
                })
                .collect(),
        });

        Self {
            schema: SCHEMA.to_string(),
            schema_version: SCHEMA_VERSION,
            producer: ArtifactProducer {
                name: PRODUCER.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            target: ArtifactTarget {
                triple: env!("NC_COM_INSPECT_TARGET").to_string(),
                pointer_width: (core::mem::size_of::<usize>() * 8) as u8,
                endianness: if cfg!(target_endian = "little") {
                    ArtifactEndianness::Little
                } else {
                    ArtifactEndianness::Big
                },
            },
            core_abi_version: value.core_abi_version,
            com_validation: ArtifactValidationMode::from_abi(value.validation_mode).expect("plugin inspection must contain a descriptor-validated ABI mode"),
            plugin: ArtifactPlugin {
                id: value.plugin_id.to_string(),
                name: value.qualified_name.clone(),
                version: value.plugin_version.clone(),
                dependencies,
            },
            activation,
        }
    }
}

/// Converts one typed exact module reference to stable text fields.
fn artifact_contract_reference(value: &nc_com_shared_runtime::ContractReferenceInspection) -> ArtifactContractReference {
    ArtifactContractReference {
        module_id: value.module_id.to_string(),
        abi_fingerprint: format_fingerprint(value.abi_fingerprint.bytes),
    }
}

/// Converts one runtime declaration kind to its schema spelling.
fn artifact_declaration_kind(value: ContractDeclarationKind) -> ArtifactContractDeclarationKind {
    match value {
        ContractDeclarationKind::Struct => ArtifactContractDeclarationKind::Struct,
        ContractDeclarationKind::Enum => ArtifactContractDeclarationKind::Enum,
        ContractDeclarationKind::Interface => ArtifactContractDeclarationKind::Trait,
        ContractDeclarationKind::Class => ArtifactContractDeclarationKind::Class,
        ContractDeclarationKind::Factory => ArtifactContractDeclarationKind::Factory,
    }
}

/// Formats a complete fingerprint as fixed-width lowercase hexadecimal text.
fn format_fingerprint(bytes: [u8; 32]) -> String {
    use std::fmt::Write as _;

    let mut text = String::with_capacity(64);
    for byte in bytes {
        write!(text, "{byte:02x}").expect("writing into String cannot fail");
    }
    text
}

impl InspectionArtifact {
    pub(crate) fn validate(&self) -> Result<(), ArtifactValidationError> {
        self.validate_envelope()?;

        let plugin_id = parse_guid(&self.plugin.id, "plugin.id")?;
        validate_named_guid(plugin_id, &self.plugin.name, "plugin")?;
        validate_version(&self.plugin.version, "plugin.version")?;
        validate_dependencies(plugin_id, &self.plugin.dependencies)?;

        let mut id_to_name = BTreeMap::<[u8; 16], &str>::new();
        let mut name_to_id = BTreeMap::<&str, NcGuid>::new();
        insert_identity(&mut id_to_name, &mut name_to_id, plugin_id, &self.plugin.name, "plugin")?;

        match &self.activation {
            ArtifactActivation::Factory(factory) => validate_factory(factory, &mut id_to_name, &mut name_to_id)?,
            ArtifactActivation::LegacyClassTable { classes } => validate_classes(classes, self.com_validation, &mut id_to_name, &mut name_to_id)?,
        }
        Ok(())
    }

    /// Validates schema provenance and target-wide ABI facts.
    fn validate_envelope(&self) -> Result<(), ArtifactValidationError> {
        if self.schema != SCHEMA {
            return Err(ArtifactValidationError::new("schema", format!("expected {SCHEMA:?}, found {:?}", self.schema)));
        }
        if self.schema_version != SCHEMA_VERSION {
            return Err(ArtifactValidationError::new(
                "schema_version",
                format!("unsupported inspection schema version {}; expected {SCHEMA_VERSION}", self.schema_version),
            ));
        }
        if self.producer.name != PRODUCER {
            return Err(ArtifactValidationError::new("producer.name", format!("expected {PRODUCER:?}")));
        }
        validate_version(&self.producer.version, "producer.version")?;
        if self.target.triple.is_empty() || self.target.triple.len() > 255 || !self.target.triple.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(ArtifactValidationError::new(
                "target.triple",
                "target triple must contain 1 through 255 printable non-whitespace ASCII bytes",
            ));
        }
        if !matches!(self.target.pointer_width, 32 | 64) {
            return Err(ArtifactValidationError::new("target.pointer_width", "pointer width must be 32 or 64"));
        }
        if self.core_abi_version == 0 {
            return Err(ArtifactValidationError::new("core_abi_version", "core ABI version must not be zero"));
        }
        Ok(())
    }
}

/// Validates all legacy constructible class and interface relationships.
fn validate_classes<'a>(
    classes: &'a [ArtifactClass],
    validation_mode: ArtifactValidationMode,
    id_to_name: &mut BTreeMap<[u8; 16], &'a str>,
    name_to_id: &mut BTreeMap<&'a str, NcGuid>,
) -> Result<(), ArtifactValidationError> {
    let mut class_ids = BTreeSet::new();
    for (class_index, class) in classes.iter().enumerate() {
        let class_path = format!("classes[{class_index}]");
        let class_id = parse_guid(&class.id, &format!("{class_path}.id"))?;
        validate_named_guid(class_id, &class.name, &class_path)?;
        validate_version(&class.version, &format!("{class_path}.version"))?;
        if !class_ids.insert(class_id.to_uuid_bytes()) {
            return Err(ArtifactValidationError::new(format!("{class_path}.id"), "duplicate class ID"));
        }
        insert_identity(id_to_name, name_to_id, class_id, &class.name, &class_path)?;
        validate_signature(
            &class.constructor_abi_signature,
            &format!("{class_path}.constructor_abi_signature"),
            validation_mode,
        )?;

        let mut interface_ids = BTreeSet::new();
        let mut iunknown_count = 0;
        for (interface_index, interface) in class.interfaces.iter().enumerate() {
            let interface_path = format!("{class_path}.interfaces[{interface_index}]");
            let iid = validate_interface(interface, &interface_path, validation_mode)?;
            if !interface_ids.insert(iid.to_uuid_bytes()) {
                return Err(ArtifactValidationError::new(format!("{interface_path}.id"), "duplicate interface ID in class"));
            }
            if iid == IID_IUNKNOWN {
                iunknown_count += 1;
            }
            insert_identity(id_to_name, name_to_id, iid, &interface.name, &interface_path)?;
        }
        // Every inherited identity must resolve in this same class. Bound
        // traversal by table length so malformed artifacts cannot loop.
        for interface in &class.interfaces {
            let mut current = interface;
            let mut hops = 0;
            while !validate_interface_id(&current.base, &class_path, true, validation_mode)?.is_none() {
                hops += 1;
                if hops >= class.interfaces.len() {
                    return Err(ArtifactValidationError::new(&class_path, "cyclic interface inheritance"));
                }
                current = class
                    .interfaces
                    .iter()
                    .find(|candidate| candidate.id == current.base.iid && candidate.abi_signature == current.base.abi_signature)
                    .ok_or_else(|| ArtifactValidationError::new(&class_path, "base interface contract is absent or incompatible"))?;
            }
        }
        if iunknown_count != 1 {
            return Err(ArtifactValidationError::new(
                format!("{class_path}.interfaces"),
                "class must contain exactly one canonical IUnknown interface",
            ));
        }
    }
    Ok(())
}

/// Validates a factory root against one complete canonical module graph.
fn validate_factory<'a>(
    factory: &'a ArtifactFactory,
    id_to_name: &mut BTreeMap<[u8; 16], &'a str>,
    name_to_id: &mut BTreeMap<&'a str, NcGuid>,
) -> Result<(), ArtifactValidationError> {
    let root_factory_id = parse_guid(&factory.root_factory_id, "activation.factory.root_factory_id")?;
    if root_factory_id.is_nil() {
        return Err(ArtifactValidationError::new(
            "activation.factory.root_factory_id",
            "root factory ID must not be nil",
        ));
    }
    if factory.direct_contracts.is_empty() {
        return Err(ArtifactValidationError::new(
            "activation.factory.direct_contracts",
            "factory plugin must directly reference at least one contract module",
        ));
    }

    let mut modules_by_id = BTreeMap::<[u8; 16], (usize, &ArtifactContractModule)>::new();
    let mut module_names = BTreeSet::<&str>::new();
    let mut declaration_ids = BTreeSet::<[u8; 16]>::new();
    let mut matching_roots = 0usize;
    for (module_index, module) in factory.modules.iter().enumerate() {
        let path = format!("activation.factory.modules[{module_index}]");
        let module_id = parse_guid(&module.id, &format!("{path}.id"))?;
        validate_named_guid(module_id, &module.name, &path)?;
        validate_fingerprint(&module.abi_fingerprint, &format!("{path}.abi_fingerprint"))?;
        if modules_by_id.insert(module_id.to_uuid_bytes(), (module_index, module)).is_some() {
            return Err(ArtifactValidationError::new(format!("{path}.id"), "duplicate contract module ID"));
        }
        if !module_names.insert(module.name.as_str()) {
            return Err(ArtifactValidationError::new(format!("{path}.name"), "duplicate contract module name"));
        }
        insert_identity(id_to_name, name_to_id, module_id, &module.name, &path)?;

        let mut dependency_ids = BTreeSet::new();
        for (dependency_index, dependency) in module.dependencies.iter().enumerate() {
            let dependency_path = format!("{path}.dependencies[{dependency_index}]");
            let dependency_id = validate_contract_reference(dependency, &dependency_path)?;
            if !dependency_ids.insert(dependency_id.to_uuid_bytes()) {
                return Err(ArtifactValidationError::new(
                    format!("{dependency_path}.module_id"),
                    "duplicate direct contract dependency",
                ));
            }
            let Some((resolved_index, resolved)) = modules_by_id.get(&dependency_id.to_uuid_bytes()) else {
                return Err(ArtifactValidationError::new(
                    dependency_path,
                    "dependency is absent or does not precede its dependent module",
                ));
            };
            if *resolved_index >= module_index || resolved.abi_fingerprint != dependency.abi_fingerprint {
                return Err(ArtifactValidationError::new(
                    dependency_path,
                    "dependency revision does not match the canonical module graph",
                ));
            }
        }

        for (declaration_index, declaration) in module.declarations.iter().enumerate() {
            let declaration_path = format!("{path}.declarations[{declaration_index}]");
            let declaration_id = parse_guid(&declaration.id, &format!("{declaration_path}.id"))?;
            if declaration_id.is_nil() {
                return Err(ArtifactValidationError::new(format!("{declaration_path}.id"), "declaration ID must not be nil"));
            }
            validate_qualified_name(&declaration.name, &format!("{declaration_path}.name"))?;
            if declaration.version == 0 {
                return Err(ArtifactValidationError::new(
                    format!("{declaration_path}.version"),
                    "declaration version must be positive",
                ));
            }
            let expects_threading = matches!(
                declaration.kind,
                ArtifactContractDeclarationKind::Class | ArtifactContractDeclarationKind::Factory
            );
            if declaration.threading.is_some() != expects_threading {
                return Err(ArtifactValidationError::new(
                    format!("{declaration_path}.threading"),
                    "threading is required only for class and factory declarations",
                ));
            }
            if !declaration_ids.insert(declaration_id.to_uuid_bytes()) {
                return Err(ArtifactValidationError::new(format!("{declaration_path}.id"), "duplicate declaration ID"));
            }
            insert_identity(id_to_name, name_to_id, declaration_id, &declaration.name, &declaration_path)?;
            if declaration_id == root_factory_id {
                matching_roots += 1;
                if declaration.kind != ArtifactContractDeclarationKind::Factory || declaration.threading != Some(factory.root_factory_threading) {
                    return Err(ArtifactValidationError::new(
                        declaration_path,
                        "root identity does not name a factory with matching threading",
                    ));
                }
            }
        }
    }

    let mut direct_ids = BTreeSet::new();
    for (index, reference) in factory.direct_contracts.iter().enumerate() {
        let path = format!("activation.factory.direct_contracts[{index}]");
        let module_id = validate_contract_reference(reference, &path)?;
        if !direct_ids.insert(module_id.to_uuid_bytes()) {
            return Err(ArtifactValidationError::new(format!("{path}.module_id"), "duplicate direct contract reference"));
        }
        let Some((_, module)) = modules_by_id.get(&module_id.to_uuid_bytes()) else {
            return Err(ArtifactValidationError::new(path, "direct contract is absent from the canonical module graph"));
        };
        if module.abi_fingerprint != reference.abi_fingerprint {
            return Err(ArtifactValidationError::new(
                path,
                "direct contract revision does not match the canonical module graph",
            ));
        }
    }
    if matching_roots != 1 {
        return Err(ArtifactValidationError::new(
            "activation.factory.root_factory_id",
            "root factory must occur exactly once in the canonical module graph",
        ));
    }
    Ok(())
}

/// Validates one exact contract reference and returns its parsed module ID.
fn validate_contract_reference(reference: &ArtifactContractReference, path: &str) -> Result<NcGuid, ArtifactValidationError> {
    let module_id = parse_guid(&reference.module_id, &format!("{path}.module_id"))?;
    if module_id.is_nil() {
        return Err(ArtifactValidationError::new(format!("{path}.module_id"), "contract module ID must not be nil"));
    }
    validate_fingerprint(&reference.abi_fingerprint, &format!("{path}.abi_fingerprint"))?;
    Ok(module_id)
}

/// Requires one nonzero lowercase 32-byte hexadecimal digest.
fn validate_fingerprint(value: &str, path: &str) -> Result<(), ArtifactValidationError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(ArtifactValidationError::new(path, "fingerprint must be 64 lowercase hexadecimal characters"));
    }
    if value.bytes().all(|byte| byte == b'0') {
        return Err(ArtifactValidationError::new(path, "fingerprint must not be zero"));
    }
    Ok(())
}

fn validate_dependencies(plugin_id: NcGuid, values: &[String]) -> Result<(), ArtifactValidationError> {
    let mut dependencies = BTreeSet::new();
    for (index, dependency) in values.iter().enumerate() {
        let path = format!("plugin.dependencies[{index}]");
        let dependency = parse_guid(dependency, &path)?;
        if dependency.is_nil() {
            return Err(ArtifactValidationError::new(path, "dependency ID must not be nil"));
        }
        if dependency == plugin_id {
            return Err(ArtifactValidationError::new(path, "plugin cannot depend on itself"));
        }
        if !dependencies.insert(dependency.to_uuid_bytes()) {
            return Err(ArtifactValidationError::new(path, "duplicate dependency ID"));
        }
    }
    Ok(())
}

/// Validates one serialized interface using the artifact-wide representation.
fn validate_interface(interface: &ArtifactInterface, path: &str, mode: ArtifactValidationMode) -> Result<NcGuid, ArtifactValidationError> {
    let iid = parse_guid(&interface.id, &format!("{path}.id"))?;
    let advertised_is_empty = validate_signature(&interface.abi_signature, &format!("{path}.abi_signature"), mode)?;
    let base = validate_interface_id(&interface.base, &format!("{path}.base"), true, mode)?;
    if base.iid == IID_IUNKNOWN {
        return Err(ArtifactValidationError::new(path, "IUnknown is an implicit prefix, not an explicit base"));
    }
    if interface.name == IUNKNOWN_QUALIFIED_NAME {
        if !base.is_none() {
            return Err(ArtifactValidationError::new(path, "IUnknown cannot have a base"));
        }

        if iid != IID_IUNKNOWN {
            return Err(ArtifactValidationError::new(path, "IUnknown must use its reserved IID"));
        }
        if !interface.methods.is_empty() {
            return Err(ArtifactValidationError::new(
                format!("{path}.methods"),
                "IUnknown artifact metadata must not list derived root methods",
            ));
        }
        return Ok(iid);
    }

    validate_named_guid(iid, &interface.name, path)?;
    if advertised_is_empty {
        return Err(ArtifactValidationError::new(
            format!("{path}.abi_signature"),
            "interface ABI signature must not be empty",
        ));
    }
    for (method_index, method) in interface.methods.iter().enumerate() {
        if method.index != method_index as u32 {
            return Err(ArtifactValidationError::new(
                format!("{path}.methods[{method_index}].index"),
                format!("expected contiguous method index {method_index}"),
            ));
        }
    }
    Ok(iid)
}

/// Validates one optional interface identity and its mode-specific signature.
fn validate_interface_id(
    value: &ArtifactInterfaceId,
    path: &str,
    allow_none: bool,
    mode: ArtifactValidationMode,
) -> Result<ValidatedInterfaceId, ArtifactValidationError> {
    let iid = parse_guid(&value.iid, &format!("{path}.iid"))?;
    let signature_is_empty = validate_signature(&value.abi_signature, &format!("{path}.abi_signature"), mode)?;
    let identity = ValidatedInterfaceId {
        iid,
        is_none: iid.is_nil() && signature_is_empty,
    };
    if allow_none && identity.is_none() {
        return Ok(identity);
    }
    if iid.is_nil() || signature_is_empty {
        return Err(ArtifactValidationError::new(
            path,
            "interface identity requires a non-nil IID and non-empty ABI signature",
        ));
    }
    Ok(identity)
}

/// Parsed identity facts needed while validating an artifact ancestry graph.
struct ValidatedInterfaceId {
    /// Parsed canonical interface GUID.
    iid: NcGuid,
    /// Whether both identity fields use their canonical absent values.
    is_none: bool,
}

impl ValidatedInterfaceId {
    /// Returns whether this is the absent-base sentinel.
    fn is_none(&self) -> bool {
        self.is_none
    }
}

fn validate_named_guid(id: NcGuid, name: &str, path: &str) -> Result<(), ArtifactValidationError> {
    validate_qualified_name(name, &format!("{path}.name"))?;
    if id.is_nil() {
        return Err(ArtifactValidationError::new(format!("{path}.id"), "identity must not be nil"));
    }
    if id != guid_from_qualified_name(name) {
        return Err(ArtifactValidationError::new(
            format!("{path}.id"),
            "GUID does not match the normative qualified name",
        ));
    }
    Ok(())
}

fn insert_identity<'a>(
    id_to_name: &mut BTreeMap<[u8; 16], &'a str>,
    name_to_id: &mut BTreeMap<&'a str, NcGuid>,
    id: NcGuid,
    name: &'a str,
    path: &str,
) -> Result<(), ArtifactValidationError> {
    if let Some(existing) = id_to_name.insert(id.to_uuid_bytes(), name) {
        if existing != name {
            return Err(ArtifactValidationError::new(
                path,
                format!("GUID is already mapped to qualified name {existing:?}"),
            ));
        }
    }
    if let Some(existing) = name_to_id.insert(name, id) {
        if existing != id {
            return Err(ArtifactValidationError::new(
                path,
                format!("qualified name is already mapped to GUID {existing}"),
            ));
        }
    }
    Ok(())
}

fn parse_guid(value: &str, path: &str) -> Result<NcGuid, ArtifactValidationError> {
    let guid = value
        .parse::<NcGuid>()
        .map_err(|error| ArtifactValidationError::new(path, format!("invalid GUID: {error}")))?;
    if guid.to_string() != value {
        return Err(ArtifactValidationError::new(path, "GUID must use lowercase canonical UUID text"));
    }
    Ok(guid)
}

/// Validates one rendered signature against the artifact's declared mode.
fn validate_signature(value: &str, path: &str, mode: ArtifactValidationMode) -> Result<bool, ArtifactValidationError> {
    if value.is_empty() {
        return Ok(true);
    }
    match mode {
        ArtifactValidationMode::None if value == "ncabi1;unchecked" => {}
        ArtifactValidationMode::Sha256 => {
            let digest = value
                .strip_prefix("sha-256:")
                .ok_or_else(|| ArtifactValidationError::new(path, "ABI signature does not use the declared sha-256 mode"))?;
            validate_hex_signature(digest, 64, path)?;
        }
        ArtifactValidationMode::Fnv1a128 => {
            let fingerprint = value
                .strip_prefix("fnv-1a-128:")
                .ok_or_else(|| ArtifactValidationError::new(path, "ABI signature does not use the declared fnv-1a-128 mode"))?;
            validate_hex_signature(fingerprint, 32, path)?;
        }
        ArtifactValidationMode::Mangling if value != "ncabi1;unchecked" && value.starts_with("ncabi1;") && !value.chars().any(char::is_control) => {}
        ArtifactValidationMode::None => {
            return Err(ArtifactValidationError::new(path, "ABI signature does not use the declared none mode"));
        }
        ArtifactValidationMode::Mangling => {
            return Err(ArtifactValidationError::new(path, "ABI signature does not use the declared mangling mode"));
        }
    }
    Ok(false)
}

/// Validates one fixed-width lowercase hexadecimal artifact representation.
fn validate_hex_signature(value: &str, expected_len: usize, path: &str) -> Result<(), ArtifactValidationError> {
    if value.len() != expected_len || !value.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')) {
        return Err(ArtifactValidationError::new(path, "ABI digest has an invalid width or hexadecimal encoding"));
    }
    Ok(())
}

fn validate_qualified_name(value: &str, path: &str) -> Result<(), ArtifactValidationError> {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 1024 {
        return Err(ArtifactValidationError::new(path, "qualified name must contain 1 through 1024 bytes"));
    }
    let mut expect_start = true;
    for byte in bytes {
        if expect_start {
            if !byte.is_ascii_alphabetic() && *byte != b'_' {
                return Err(ArtifactValidationError::new(path, "invalid qualified name"));
            }
            expect_start = false;
        } else if *byte == b'.' || *byte == b':' {
            expect_start = true;
        } else if !byte.is_ascii_alphanumeric() && *byte != b'_' && *byte != b'-' {
            return Err(ArtifactValidationError::new(path, "invalid qualified name"));
        }
    }
    if expect_start {
        return Err(ArtifactValidationError::new(path, "invalid qualified name"));
    }
    Ok(())
}

fn validate_version(value: &str, path: &str) -> Result<(), ArtifactValidationError> {
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(ArtifactValidationError::new(
            path,
            "version must contain 1 through 128 printable non-whitespace ASCII bytes",
        ));
    }
    Ok(())
}

#[cfg(test)]
fn artifact_interface_id(interface_id: NcInterfaceId) -> ArtifactInterfaceId {
    ArtifactInterfaceId {
        iid: interface_id.iid.to_string(),
        abi_signature: format_signature(interface_id.abi_signature),
    }
}

fn artifact_threading(threading: NcAbiThreadingMode) -> ArtifactThreading {
    match threading {
        NcAbiThreadingMode::ThreadLocal => ArtifactThreading::ThreadLocal,
        NcAbiThreadingMode::ThreadSafe => ArtifactThreading::ThreadSafe,
    }
}

#[cfg(test)]
pub(crate) fn format_signature(signature: NcAbiSignature) -> String {
    // SAFETY: Inspection receives validated descriptors from modules pinned
    // until process exit; built-in and generated local signatures are static.
    unsafe { format_abi_signature(signature) }.expect("validated ABI signature view")
}

#[cfg(test)]
pub(crate) fn sample_artifact() -> InspectionArtifact {
    let plugin_name = "example.plugin";
    let class_name = "example.Foo";
    let interface_name = "example.FooInterface:v1";
    InspectionArtifact {
        schema: SCHEMA.to_string(),
        schema_version: SCHEMA_VERSION,
        producer: ArtifactProducer {
            name: PRODUCER.to_string(),
            version: "0.1.0".to_string(),
        },
        target: ArtifactTarget {
            triple: "x86_64-unknown-linux-gnu".to_string(),
            pointer_width: 64,
            endianness: ArtifactEndianness::Little,
        },
        core_abi_version: neocogi_com_core::NC_COM_CORE_ABI_VERSION,
        com_validation: ArtifactValidationMode::None,
        plugin: ArtifactPlugin {
            id: guid_from_qualified_name(plugin_name).to_string(),
            name: plugin_name.to_string(),
            version: "1.0.0".to_string(),
            dependencies: Vec::new(),
        },
        activation: ArtifactActivation::LegacyClassTable {
            classes: vec![ArtifactClass {
                id: guid_from_qualified_name(class_name).to_string(),
                name: class_name.to_string(),
                version: "1.0.0".to_string(),
                constructor_abi_signature: format_signature(NcAbiSignature::EMPTY),
                threading: ArtifactThreading::ThreadLocal,
                interfaces: vec![
                    ArtifactInterface {
                        id: IID_IUNKNOWN.to_string(),
                        name: IUNKNOWN_QUALIFIED_NAME.to_string(),
                        abi_signature: "ncabi1;unchecked".to_string(),
                        base: artifact_interface_id(NcInterfaceId::NONE),
                        methods: Vec::new(),
                    },
                    ArtifactInterface {
                        id: guid_from_qualified_name(interface_name).to_string(),
                        name: interface_name.to_string(),
                        abi_signature: "ncabi1;unchecked".to_string(),
                        base: artifact_interface_id(NcInterfaceId::NONE),
                        methods: vec![ArtifactMethod {
                            index: 0,
                            name: "value".to_string(),
                            signature: "value() -> out<u32> status".to_string(),
                        }],
                    },
                ],
            }],
        },
    }
}

/// Builds one valid factory artifact with a single canonical leaf module.
#[cfg(test)]
pub(crate) fn sample_factory_artifact() -> InspectionArtifact {
    let module_name = "example.contract";
    let factory_name = "example.contract.RootFactory";
    let module_id = guid_from_qualified_name(module_name).to_string();
    let fingerprint = "11".repeat(32);
    let mut artifact = sample_artifact();
    artifact.activation = ArtifactActivation::Factory(ArtifactFactory {
        root_factory_id: guid_from_qualified_name(factory_name).to_string(),
        root_factory_threading: ArtifactThreading::ThreadLocal,
        direct_contracts: vec![ArtifactContractReference {
            module_id: module_id.clone(),
            abi_fingerprint: fingerprint.clone(),
        }],
        modules: vec![ArtifactContractModule {
            id: module_id,
            abi_fingerprint: fingerprint,
            name: module_name.to_string(),
            dependencies: Vec::new(),
            declarations: vec![ArtifactContractDeclaration {
                id: guid_from_qualified_name(factory_name).to_string(),
                version: 1,
                kind: ArtifactContractDeclarationKind::Factory,
                threading: Some(ArtifactThreading::ThreadLocal),
                name: factory_name.to_string(),
            }],
            source: "#[version(1)] factory RootFactory {}".to_string(),
        }],
    });
    artifact
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Borrows the legacy fixture classes for malformed-artifact tests.
    fn legacy_classes_mut(artifact: &mut InspectionArtifact) -> &mut Vec<ArtifactClass> {
        let ArtifactActivation::LegacyClassTable { classes } = &mut artifact.activation else {
            panic!("sample artifact must use legacy class-table activation");
        };
        classes
    }

    #[test]
    fn sample_artifact_is_valid_and_round_trips() {
        let artifact = sample_artifact();
        artifact.validate().unwrap();
        let json = serde_json::to_vec(&artifact).unwrap();
        let parsed: InspectionArtifact = serde_json::from_slice(&json).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed, artifact);
        assert!(String::from_utf8(json).unwrap().contains("\"com_validation\":\"none\""));
    }

    /// Confirms factory artifacts retain and validate the complete IDL graph.
    #[test]
    fn factory_artifact_is_valid_and_round_trips() {
        let artifact = sample_factory_artifact();
        artifact.validate().unwrap();
        let json = serde_json::to_vec(&artifact).unwrap();
        let parsed: InspectionArtifact = serde_json::from_slice(&json).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed, artifact);
    }

    /// Confirms the root capability is checked against its canonical declaration.
    #[test]
    fn rejects_factory_root_threading_disagreement() {
        let mut artifact = sample_factory_artifact();
        let ArtifactActivation::Factory(factory) = &mut artifact.activation else {
            unreachable!();
        };
        factory.root_factory_threading = ArtifactThreading::ThreadSafe;
        assert!(artifact.validate().unwrap_err().to_string().contains("matching threading"));
    }

    #[test]
    fn rejects_signatures_that_disagree_with_the_declared_mode() {
        let mut artifact = sample_artifact();
        artifact.com_validation = ArtifactValidationMode::Sha256;

        let error = artifact.validate().unwrap_err();

        assert!(error.to_string().contains("declared sha-256 mode"));
    }

    #[test]
    fn rejects_unknown_fields_and_schema_versions() {
        let mut json = serde_json::to_value(sample_artifact()).unwrap();
        json.as_object_mut().unwrap().insert("unexpected".to_string(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<InspectionArtifact>(json).is_err());

        let mut artifact = sample_artifact();
        artifact.schema_version += 1;
        assert!(artifact.validate().unwrap_err().to_string().contains("schema_version"));

        let mut artifact = sample_artifact();
        artifact.core_abi_version = 0;
        assert!(artifact.validate().unwrap_err().to_string().contains("core_abi_version"));
    }

    #[test]
    fn accepts_nonzero_historical_core_versions_for_comparison() {
        let mut artifact = sample_artifact();
        artifact.core_abi_version += 1;
        artifact.validate().unwrap();
    }

    #[test]
    fn rejects_duplicate_identities_before_diff_maps_are_built() {
        let mut artifact = sample_artifact();
        let duplicate = legacy_classes_mut(&mut artifact)[0].clone();
        legacy_classes_mut(&mut artifact).push(duplicate);
        assert!(artifact.validate().unwrap_err().to_string().contains("duplicate class ID"));

        let mut artifact = sample_artifact();
        let duplicate = legacy_classes_mut(&mut artifact)[0].interfaces[1].clone();
        legacy_classes_mut(&mut artifact)[0].interfaces.push(duplicate);
        assert!(artifact.validate().unwrap_err().to_string().contains("duplicate interface ID"));
    }

    #[test]
    fn rejects_noncontiguous_method_indices() {
        let mut artifact = sample_artifact();
        legacy_classes_mut(&mut artifact)[0].interfaces[1].methods[0].index = 1;
        assert!(artifact.validate().unwrap_err().to_string().contains("expected contiguous method index 0"));
    }

    #[test]
    fn diagnostic_method_text_is_not_signature_authority() {
        let mut artifact = sample_artifact();
        legacy_classes_mut(&mut artifact)[0].interfaces[1].methods[0].signature = "diagnostic format changed".to_string();
        artifact.validate().unwrap();
    }
}
