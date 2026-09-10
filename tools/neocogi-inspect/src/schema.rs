//! Versioned non-reconstructive inspection artifact and semantic validation.
//!
//! The serialized model is deliberately separate from runtime inspection
//! structs. It records compatibility identities and derived diagnostics, but
//! does not expose structured method or aggregate definitions as a stable
//! reflection contract.

use std::collections::{BTreeMap, BTreeSet};

use neocogi_com_core::{
    IID_IUNKNOWN, IUNKNOWN_INTERFACE_ID, IUNKNOWN_QUALIFIED_NAME, NcAbiSignature, NcAbiThreadingMode, NcGuid, format_abi_signature, guid_from_qualified_name,
};
#[cfg(test)]
use neocogi_com_core::NcInterfaceId;
use nc_com_shared_runtime::PluginInspection;
use serde::{Deserialize, Serialize};

pub(crate) const SCHEMA: &str = "neocogi-com.inspection";
pub(crate) const SCHEMA_VERSION: u32 = 4;
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
    pub(crate) classes: Vec<ArtifactClass>,
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

        let mut classes: Vec<_> = value
            .classes
            .iter()
            .map(|class| {
                let mut interfaces: Vec<_> = class
                    .interfaces
                    .iter()
                    .map(|class_interface| {
                        let interface = &class_interface.interface;
                        ArtifactInterface {
                            id: interface.interface_id.to_string(),
                            name: interface.qualified_name.clone(),
                            abi_signature: interface.abi_signature.clone(),
                            base: ArtifactInterfaceId {
                                iid: interface.base.iid.to_string(),
                                abi_signature: interface.base.abi_signature.clone(),
                            },
                            methods: interface
                                .methods
                                .iter()
                                .map(|method| ArtifactMethod {
                                    index: method.index,
                                    name: method.name.clone(),
                                    signature: method.signature.clone(),
                                })
                                .collect(),
                        }
                    })
                    .collect();
                interfaces.sort_by(|left, right| (&left.id, &left.abi_signature).cmp(&(&right.id, &right.abi_signature)));
                ArtifactClass {
                    id: class.class_id.to_string(),
                    name: class.qualified_name.clone(),
                    version: class.class_version.clone(),
                    constructor_abi_signature: class.constructor_abi_signature.clone(),
                    threading: artifact_threading(class.threading),
                    interfaces,
                }
            })
            .collect();
        classes.sort_by(|left, right| left.id.cmp(&right.id));

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
            classes,
        }
    }
}

impl InspectionArtifact {
    pub(crate) fn validate(&self) -> Result<(), ArtifactValidationError> {
        self.validate_envelope()?;

        let plugin_id = parse_guid(&self.plugin.id, "plugin.id")?;
        validate_named_guid(plugin_id, &self.plugin.name, "plugin")?;
        validate_version(&self.plugin.version, "plugin.version")?;
        validate_dependencies(plugin_id, &self.plugin.dependencies)?;

        let mut class_ids = BTreeSet::new();
        let mut id_to_name = BTreeMap::<[u8; 16], &str>::new();
        let mut name_to_id = BTreeMap::<&str, NcGuid>::new();
        insert_identity(&mut id_to_name, &mut name_to_id, plugin_id, &self.plugin.name, "plugin")?;

        for (class_index, class) in self.classes.iter().enumerate() {
            let class_path = format!("classes[{class_index}]");
            let class_id = parse_guid(&class.id, &format!("{class_path}.id"))?;
            validate_named_guid(class_id, &class.name, &class_path)?;
            validate_version(&class.version, &format!("{class_path}.version"))?;
            if !class_ids.insert(class_id.to_uuid_bytes()) {
                return Err(ArtifactValidationError::new(format!("{class_path}.id"), "duplicate class ID"));
            }
            insert_identity(&mut id_to_name, &mut name_to_id, class_id, &class.name, &class_path)?;
            validate_signature(&class.constructor_abi_signature, &format!("{class_path}.constructor_abi_signature"))?;

            let mut interface_ids = BTreeSet::new();
            let mut iunknown_count = 0;
            for (interface_index, interface) in class.interfaces.iter().enumerate() {
                let interface_path = format!("{class_path}.interfaces[{interface_index}]");
                let iid = validate_interface(interface, &interface_path)?;
                if !interface_ids.insert(iid.to_uuid_bytes()) {
                    return Err(ArtifactValidationError::new(format!("{interface_path}.id"), "duplicate interface ID in class"));
                }
                if iid == IID_IUNKNOWN {
                    iunknown_count += 1;
                }
                insert_identity(&mut id_to_name, &mut name_to_id, iid, &interface.name, &interface_path)?;
            }
            // Every inherited identity must resolve in this same class. Bound
            // traversal by table length so malformed artifacts cannot loop.
            for interface in &class.interfaces {
                let mut current = interface;
                let mut hops = 0;
                while !validate_interface_id(&current.base, &class_path, true)?.is_none() {
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

fn validate_interface(interface: &ArtifactInterface, path: &str) -> Result<NcGuid, ArtifactValidationError> {
    let iid = parse_guid(&interface.id, &format!("{path}.id"))?;
    let advertised_is_empty = validate_signature(&interface.abi_signature, &format!("{path}.abi_signature"))?;
    let base = validate_interface_id(&interface.base, &format!("{path}.base"), true)?;
    if base.iid == IID_IUNKNOWN {
        return Err(ArtifactValidationError::new(path, "IUnknown is an implicit prefix, not an explicit base"));
    }
    if interface.name == IUNKNOWN_QUALIFIED_NAME {
        if !base.is_none() {
            return Err(ArtifactValidationError::new(path, "IUnknown cannot have a base"));
        }

        if iid != IUNKNOWN_INTERFACE_ID.iid || interface.abi_signature != format_signature(IUNKNOWN_INTERFACE_ID.abi_signature) {
            return Err(ArtifactValidationError::new(path, "IUnknown must use its reserved IID and ABI signature"));
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

fn validate_interface_id(value: &ArtifactInterfaceId, path: &str, allow_none: bool) -> Result<ValidatedInterfaceId, ArtifactValidationError> {
    let iid = parse_guid(&value.iid, &format!("{path}.iid"))?;
    let signature_is_empty = validate_signature(&value.abi_signature, &format!("{path}.abi_signature"))?;
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

fn validate_signature(value: &str, path: &str) -> Result<bool, ArtifactValidationError> {
    if value.is_empty() {
        return Ok(true);
    }
    if let Some(digest) = value.strip_prefix("sha-256:") {
        return validate_hex_signature(digest, 64, path).map(|()| false);
    }
    if let Some(fingerprint) = value.strip_prefix("fnv-1a-128:") {
        return validate_hex_signature(fingerprint, 32, path).map(|()| false);
    }
    if value == "ncabi1;" || !value.starts_with("ncabi1;") {
        return Err(ArtifactValidationError::new(path, "ABI signature must use a known canonical representation"));
    }
    if value.chars().any(char::is_control) {
        return Err(ArtifactValidationError::new(path, "ABI signature must not contain control characters"));
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
        plugin: ArtifactPlugin {
            id: guid_from_qualified_name(plugin_name).to_string(),
            name: plugin_name.to_string(),
            version: "1.0.0".to_string(),
            dependencies: Vec::new(),
        },
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
                    abi_signature: format_signature(IUNKNOWN_INTERFACE_ID.abi_signature),
                    base: artifact_interface_id(NcInterfaceId::NONE),
                    methods: Vec::new(),
                },
                ArtifactInterface {
                    id: guid_from_qualified_name(interface_name).to_string(),
                    name: interface_name.to_string(),
                    abi_signature: "ncabi1;sample-interface".to_string(),
                    base: artifact_interface_id(NcInterfaceId::NONE),
                    methods: vec![ArtifactMethod {
                        index: 0,
                        name: "value".to_string(),
                        signature: "value() -> out<u32> status".to_string(),
                    }],
                },
            ],
        }],
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
        artifact.classes.push(artifact.classes[0].clone());
        assert!(artifact.validate().unwrap_err().to_string().contains("duplicate class ID"));

        let mut artifact = sample_artifact();
        let duplicate = artifact.classes[0].interfaces[1].clone();
        artifact.classes[0].interfaces.push(duplicate);
        assert!(artifact.validate().unwrap_err().to_string().contains("duplicate interface ID"));
    }

    #[test]
    fn rejects_noncontiguous_method_indices() {
        let mut artifact = sample_artifact();
        artifact.classes[0].interfaces[1].methods[0].index = 1;
        assert!(artifact.validate().unwrap_err().to_string().contains("expected contiguous method index 0"));
    }

    #[test]
    fn diagnostic_method_text_is_not_signature_authority() {
        let mut artifact = sample_artifact();
        artifact.classes[0].interfaces[1].methods[0].signature = "diagnostic format changed".to_string();
        artifact.validate().unwrap();
    }
}
