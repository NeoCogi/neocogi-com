//! Versioned non-reconstructive inspection artifact and semantic validation.
//!
//! The serialized model is deliberately separate from runtime inspection
//! structs. It records compatibility identities and derived diagnostics, but
//! does not expose structured method or aggregate definitions as a stable
//! reflection contract.

use std::collections::{BTreeMap, BTreeSet};

use neocogi_com_core::{NcAbiThreadingMode, NcGuid, guid_from_qualified_name};
use nc_com_shared_runtime::{ContractDeclarationKind, PluginInspection};
use serde::{Deserialize, Serialize};

pub(crate) const SCHEMA: &str = "neocogi-com.inspection";
pub(crate) const SCHEMA_VERSION: u32 = 8;
const PRODUCER: &str = "neocogi-inspect";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionArtifact {
    pub(crate) schema: String,
    pub(crate) schema_version: u32,
    pub(crate) producer: ArtifactProducer,
    pub(crate) target: ArtifactTarget,
    pub(crate) core_abi_version: u32,
    pub(crate) plugin: ArtifactPlugin,
    /// Typed root factory and complete canonical contract graph.
    pub(crate) factory: ArtifactFactory,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactPlugin {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) dependencies: Vec<String>,
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
    /// Transparent integer set of named bit flags.
    #[serde(rename = "bitenum")]
    BitEnum,
    /// COM interface trait.
    Trait,
    /// Factory-created object contract.
    Class,
    /// Plugin or service construction interface.
    Factory,
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
        let factory = ArtifactFactory {
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
                })
                .collect(),
        };

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
            plugin: ArtifactPlugin {
                id: value.plugin_id.to_string(),
                name: value.qualified_name.clone(),
                version: value.plugin_version.clone(),
                dependencies,
            },
            factory,
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
        ContractDeclarationKind::BitEnum => ArtifactContractDeclarationKind::BitEnum,
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

        validate_factory(&self.factory, &mut id_to_name, &mut name_to_id)?;
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

/// Validates a factory root against one complete canonical module graph.
fn validate_factory<'a>(
    factory: &'a ArtifactFactory,
    id_to_name: &mut BTreeMap<[u8; 16], &'a str>,
    name_to_id: &mut BTreeMap<&'a str, NcGuid>,
) -> Result<(), ArtifactValidationError> {
    let root_factory_id = parse_guid(&factory.root_factory_id, "factory.root_factory_id")?;
    if root_factory_id.is_nil() {
        return Err(ArtifactValidationError::new("factory.root_factory_id", "root factory ID must not be nil"));
    }
    if factory.direct_contracts.is_empty() {
        return Err(ArtifactValidationError::new(
            "factory.direct_contracts",
            "factory plugin must directly reference at least one contract module",
        ));
    }

    let mut modules_by_id = BTreeMap::<[u8; 16], (usize, &ArtifactContractModule)>::new();
    let mut module_names = BTreeSet::<&str>::new();
    let mut declaration_ids = BTreeSet::<[u8; 16]>::new();
    let mut matching_roots = 0usize;
    for (module_index, module) in factory.modules.iter().enumerate() {
        let path = format!("factory.modules[{module_index}]");
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
        let path = format!("factory.direct_contracts[{index}]");
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
            "factory.root_factory_id",
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

fn artifact_threading(threading: NcAbiThreadingMode) -> ArtifactThreading {
    match threading {
        NcAbiThreadingMode::ThreadLocal => ArtifactThreading::ThreadLocal,
        NcAbiThreadingMode::ThreadSafe => ArtifactThreading::ThreadSafe,
    }
}

#[cfg(test)]
pub(crate) fn sample_artifact() -> InspectionArtifact {
    let plugin_name = "example.plugin";
    let module_name = "example.contract";
    let factory_name = "example.contract.RootFactory";
    let module_id = guid_from_qualified_name(module_name).to_string();
    let fingerprint = "11".repeat(32);
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
        plugin: ArtifactPlugin {
            id: guid_from_qualified_name(plugin_name).to_string(),
            name: plugin_name.to_string(),
            version: "1.0.0".to_string(),
            dependencies: Vec::new(),
        },
        factory: ArtifactFactory {
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
            }],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_artifact_is_valid_and_round_trips() {
        let artifact = sample_artifact();
        artifact.validate().unwrap();
        let json = serde_json::to_vec(&artifact).unwrap();
        let parsed: InspectionArtifact = serde_json::from_slice(&json).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed, artifact);
    }

    /// Confirms the root capability is checked against its canonical declaration.
    #[test]
    fn rejects_factory_root_threading_disagreement() {
        let mut artifact = sample_artifact();
        artifact.factory.root_factory_threading = ArtifactThreading::ThreadSafe;
        assert!(artifact.validate().unwrap_err().to_string().contains("matching threading"));
    }

    #[test]
    fn rejects_contract_fingerprints_with_noncanonical_text() {
        let mut artifact = sample_artifact();
        artifact.factory.modules[0].abi_fingerprint = "AA".repeat(32);

        let error = artifact.validate().unwrap_err();

        assert!(error.to_string().contains("lowercase hexadecimal"));
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
}
