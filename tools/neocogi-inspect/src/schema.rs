//! Versioned non-reconstructive inspection artifact and semantic validation.
//!
//! The serialized model is deliberately separate from runtime inspection
//! structs. It records compatibility identities and derived diagnostics, but
//! does not expose structured method or aggregate definitions as a stable
//! reflection contract.

use std::collections::BTreeSet;

use neocogi_com_core::{NcAbiThreadingMode, NcGuid, guid_from_qualified_name};
use nc_com::PluginInspection;
use serde::{Deserialize, Serialize};

/// Stable document-family identifier, separate from the native descriptor ABI.
pub(crate) const SCHEMA: &str = "neocogi-com.inspection";
/// Current strict JSON shape; removing runtime contract graphs changed version 8.
pub(crate) const SCHEMA_VERSION: u32 = 9;
/// Tool identity recorded in every serialized inspection.
const PRODUCER: &str = "neocogi-inspect";

/// Owned, versioned snapshot of one plugin and its root factory.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionArtifact {
    /// Document family used to reject unrelated JSON.
    pub(crate) schema: String,
    /// Exact serialized shape accepted by this tool.
    pub(crate) schema_version: u32,
    /// Tool and version that copied the native metadata.
    pub(crate) producer: ArtifactProducer,
    /// Native platform assumptions under which inspection succeeded.
    pub(crate) target: ArtifactTarget,
    /// Version of the plugin descriptor layout.
    pub(crate) description_abi_version: u32,
    /// Version of the object ABI used by this plugin.
    pub(crate) core_abi_version: u32,
    /// Deployable provider identity and its hard dependencies.
    pub(crate) plugin: ArtifactPlugin,
    /// Exact root factory identity, signature, and threading promise.
    pub(crate) factory: ArtifactFactory,
}

/// Provenance of the tool that produced an inspection document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactProducer {
    /// Normative identifier used to validate this producer or plugin.
    pub(crate) name: String,
    /// Printable release text; diagnostic rather than a compatibility digest.
    pub(crate) version: String,
}

/// Native platform facts required when comparing two plugins.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactTarget {
    /// Compiler target triple used by the inspector.
    pub(crate) triple: String,
    /// Number of bits in a native pointer on the inspected target.
    pub(crate) pointer_width: u8,
    /// Byte order of copied native ABI values.
    pub(crate) endianness: ArtifactEndianness,
}

/// Portable JSON spelling of native byte order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactEndianness {
    /// Least-significant byte first.
    Little,
    /// Most-significant byte first.
    Big,
}

/// Plugin identity and deployment dependencies copied from its descriptor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactPlugin {
    /// Canonical UUID derived from the normative plugin name.
    pub(crate) id: String,
    /// Normative identifier used to validate this producer or plugin.
    pub(crate) name: String,
    /// Printable release text; diagnostic rather than a compatibility digest.
    pub(crate) version: String,
    /// Canonical IDs of plugins required before this one is registered.
    pub(crate) dependencies: Vec<String>,
}

/// Serialized factory compatibility information without runtime module metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactFactory {
    /// Stable IDL identity returned by plugin bootstrap.
    pub(crate) root_factory_id: String,
    /// Threading capability promised by the root factory.
    pub(crate) root_factory_threading: ArtifactThreading,
    /// Lowercase hexadecimal SHA-256 signature of the exact root contract.
    pub(crate) root_factory_signature: String,
}

/// Minimum capability promised by a root factory.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactThreading {
    /// References and calls stay on their owning thread.
    ThreadLocal,
    /// Implementations permit concurrent calls and cross-thread ownership.
    ThreadSafe,
}

/// A field-specific failure while validating an owned inspection artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArtifactValidationError {
    /// JSON field path identifying the invalid input.
    path: String,
    /// Human-readable explanation without retaining the parsed document.
    message: String,
}

impl ArtifactValidationError {
    /// Attaches a validation failure to its precise serialized field.
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ArtifactValidationError {
    /// Renders one actionable field-path diagnostic.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for ArtifactValidationError {}

impl From<&PluginInspection> for InspectionArtifact {
    /// Copies native owned inspection data into stable JSON vocabulary.
    fn from(value: &PluginInspection) -> Self {
        let mut dependencies: Vec<_> = value.dependencies.iter().map(ToString::to_string).collect();
        dependencies.sort();
        let factory = &value.factory;
        let factory = ArtifactFactory {
            root_factory_id: factory.root_factory_id.to_string(),
            root_factory_threading: artifact_threading(factory.root_factory_threading),
            root_factory_signature: format_signature(factory.root_factory_signature),
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
            description_abi_version: value.description_abi_version,
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

/// Formats a complete factory signature as fixed-width lowercase hexadecimal text.
fn format_signature(bytes: [u8; 32]) -> String {
    use std::fmt::Write as _;

    let mut text = String::with_capacity(64);
    for byte in bytes {
        write!(text, "{byte:02x}").expect("writing into String cannot fail");
    }
    text
}

impl InspectionArtifact {
    /// Checks envelope, plugin dependencies, and complete root identity.
    pub(crate) fn validate(&self) -> Result<(), ArtifactValidationError> {
        self.validate_envelope()?;

        let plugin_id = parse_guid(&self.plugin.id, "plugin.id")?;
        validate_named_guid(plugin_id, &self.plugin.name, "plugin")?;
        validate_version(&self.plugin.version, "plugin.version")?;
        validate_dependencies(plugin_id, &self.plugin.dependencies)?;

        validate_factory(&self.factory)?;
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
        if self.description_abi_version == 0 {
            return Err(ArtifactValidationError::new(
                "description_abi_version",
                "descriptor ABI version must not be zero",
            ));
        }
        if self.core_abi_version == 0 {
            return Err(ArtifactValidationError::new("core_abi_version", "core ABI version must not be zero"));
        }
        Ok(())
    }
}

/// Validates the exact root identity without requiring a declaration inventory.
fn validate_factory(factory: &ArtifactFactory) -> Result<(), ArtifactValidationError> {
    let root = parse_guid(&factory.root_factory_id, "factory.root_factory_id")?;
    if root.is_nil() {
        return Err(ArtifactValidationError::new("factory.root_factory_id", "root factory ID must not be nil"));
    }
    validate_signature(&factory.root_factory_signature, "factory.root_factory_signature")
}

/// Requires one complete lowercase 32-byte hexadecimal digest.
fn validate_signature(value: &str, path: &str) -> Result<(), ArtifactValidationError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(ArtifactValidationError::new(path, "signature must be 64 lowercase hexadecimal characters"));
    }
    Ok(())
}

/// Rejects nil, repeated, and self-referential provider requirements.
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

/// Confirms that a plugin name and its derived UUID identify the same provider.
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

/// Parses only the canonical lowercase UUID spelling used in artifacts.
fn parse_guid(value: &str, path: &str) -> Result<NcGuid, ArtifactValidationError> {
    let guid = value
        .parse::<NcGuid>()
        .map_err(|error| ArtifactValidationError::new(path, format!("invalid GUID: {error}")))?;
    if guid.to_string() != value {
        return Err(ArtifactValidationError::new(path, "GUID must use lowercase canonical UUID text"));
    }
    Ok(guid)
}

/// Uses the same normative plugin-name grammar as runtime validation.
fn validate_qualified_name(value: &str, path: &str) -> Result<(), ArtifactValidationError> {
    neocogi_com_core::validate_qualified_name(value).map_err(|error| ArtifactValidationError::new(path, format!("invalid qualified name: {error}")))
}

/// Adds the JSON field location to COM's authoritative descriptor-version error.
fn validate_version(value: &str, path: &str) -> Result<(), ArtifactValidationError> {
    neocogi_com_core::validate_version_text(value).map_err(|error| ArtifactValidationError::new(path, format!("invalid version: {error}")))
}

/// Converts validated native capabilities without exposing ABI integers in JSON.
fn artifact_threading(threading: NcAbiThreadingMode) -> ArtifactThreading {
    match threading {
        NcAbiThreadingMode::ThreadLocal => ArtifactThreading::ThreadLocal,
        NcAbiThreadingMode::ThreadSafe => ArtifactThreading::ThreadSafe,
    }
}

/// Supplies a complete valid baseline for comparison and validation tests.
#[cfg(test)]
pub(crate) fn sample_artifact() -> InspectionArtifact {
    let plugin_name = "example.plugin";
    let factory_name = "example.contract.RootFactory";
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
        description_abi_version: neocogi_com_core::NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
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
            root_factory_signature: fingerprint,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saved inspection artifacts use the same version policy as native descriptors.
    #[test]
    fn version_validation_delegates_to_com_with_field_context() {
        for version in ["", "1.0 beta", "1.β", &"v".repeat(129)] {
            let error = validate_version(version, "plugin.version").unwrap_err().to_string();
            assert!(error.contains("plugin.version"), "{error}");
            assert!(error.contains(&neocogi_com_core::validate_version_text(version).unwrap_err().to_string()), "{error}");
        }
        validate_version(&"v".repeat(128), "plugin.version").unwrap();
    }

    #[test]
    fn sample_artifact_is_valid_and_round_trips() {
        let artifact = sample_artifact();
        artifact.validate().unwrap();
        let json = serde_json::to_vec(&artifact).unwrap();
        let parsed: InspectionArtifact = serde_json::from_slice(&json).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed, artifact);
    }

    /// A missing root identity cannot be used for factory compatibility checks.
    #[test]
    fn rejects_nil_factory_root() {
        let mut artifact = sample_artifact();
        artifact.factory.root_factory_id = NcGuid::nil().to_string();
        assert!(artifact.validate().unwrap_err().to_string().contains("must not be nil"));
    }

    #[test]
    fn rejects_contract_fingerprints_with_noncanonical_text() {
        let mut artifact = sample_artifact();
        artifact.factory.root_factory_signature = "AA".repeat(32);

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
