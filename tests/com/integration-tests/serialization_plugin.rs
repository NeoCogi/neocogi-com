//! The production thin shim exposes the same factories and diagnostics as static linking.
use std::{path::Path, process::Command};
use neocogi_com::{ComSerialize, ComDeserialize, NcCopyFromBufferOffset};
use neocogi_com::serialization::{ISerializationPluginAbi, Format, Limits};

#[path = "../../../crates/serialization/tests/support/json_numbers.rs"]
mod json_numbers;

/// Application-owned state never crosses the native boundary as Rust storage.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct State {
    count: u32,
    label: String,
    active: bool,
    /// Functional float serialization must survive target Debug stripping.
    weight: f64,
}

/// Loads an ordinary independently built cdylib, acquires both built-in formats,
/// performs real vtable calls and copies a detailed failure back to the caller.
#[test]
fn native_shim_supplies_formats_and_failure_details() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let build = Command::new("rustup")
        .args([
            "run",
            "nightly",
            "cargo",
            "build",
            "--profile",
            "min-size",
            "-p",
            "neocogi-com-serialization-dylib",
            "--locked",
            "--message-format=json",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let artifact = String::from_utf8(build.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| record["reason"] == "compiler-artifact" && record["target"]["name"] == "neocogi_com_serialization_dylib")
        .flat_map(|record| record["filenames"].as_array().cloned().unwrap_or_default())
        .filter_map(|path| path.as_str().map(std::path::PathBuf::from))
        .find(|path| path.extension().is_some_and(|extension| extension == std::env::consts::DLL_EXTENSION))
        .unwrap();
    let runtime = neocogi_com_host::initialize_runtime();
    // SAFETY: the fixture is the trusted production shim just compiled above.
    let id = unsafe { runtime.load_plugin_library(&artifact) };
    let root = runtime.acquire_plugin_root::<ISerializationPluginAbi>(id).unwrap();
    let value = State {
        count: 42,
        label: "across the ABI".into(),
        active: true,
        weight: 0.125,
    };
    for name in ["ncb", "json"] {
        let format = Format::new(root.format(NcCopyFromBufferOffset::new(0, name.as_bytes())).unwrap(), Limits::default());
        let bytes = format.serialize(&value).unwrap();
        assert_eq!(format.deserialize::<State>(&bytes).unwrap(), value);
        if name == "json" {
            let error = format.deserialize::<State>(br#"{"count":true,"label":"bad","active":true}"#).unwrap_err();
            assert_eq!(error.path, "$.count");
            assert!(error.message.contains("u32"));
            let error = format.deserialize::<State>("{\n \"name\": \"é\",\n \"bad\": ]\n}".as_bytes()).unwrap_err();
            assert_eq!(error.byte_offset, Some(25));
            assert_eq!((error.line, error.column), (Some(3), Some(9)));
            json_numbers::assert_json_numbers(&format);
            compare_json_grammar(&format);
        }
    }
    let default = Format::new(root.default_format().unwrap(), Limits::default());
    assert!(default.serialize(&value).unwrap().starts_with(b"NCB\x01"));
    persistence_round_trip(&runtime);
}

/// Exercises host persistence with the actual independently built deployment DLL.
/// This process never registers a static serialization descriptor.
fn persistence_round_trip(runtime: &neocogi_com::RuntimeHandle) {
    use neocogi_com::persistence::{FactoryRegistry, IPersistableAbi, ObjectRecord, snapshot};
    let value = Persistent::from_value(Persistent);
    let bytes = snapshot(runtime, value.as_com_ref()).unwrap();
    let expected = neocogi_com_serialization::ncb::to_vec(&ObjectRecord {
        type_id: PERSISTENT_KIND.to_uuid_bytes(),
        schema_version: 1,
        state: vec![1, 2, 3],
        children: vec![],
    })
    .unwrap();
    assert_eq!(bytes, expected, "the loaded provider preserves NCB-v1 bytes");
    let mut registry = FactoryRegistry::new(runtime).unwrap();
    registry
        .register(PERSISTENT_KIND, PersistenceFactory::from_value(PersistenceFactory).query().unwrap())
        .unwrap();
    let restored = registry.restore(&bytes).unwrap();
    assert_eq!(restored.query::<IPersistableAbi>().unwrap().schema_version().unwrap(), 1);
    assert_eq!(snapshot(runtime, restored.as_com_ref()).unwrap(), bytes);
    for end in 0..bytes.len() {
        assert_eq!(registry.restore(&bytes[..end]).unwrap_err(), neocogi_com::NC_E_INVALID_ARG);
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(registry.restore(&trailing).unwrap_err(), neocogi_com::NC_E_INVALID_ARG);
}

/// Explicit recipe identity for an application object crossing the loaded codec.
const PERSISTENT_KIND: neocogi_com::NcGuid = neocogi_com::guid_from_qualified_name("nc.tests.loaded-serialization.persistence");

/// Opaque application state; the host and codec own only its persistence envelope.
#[neocogi_com::com_object(neocogi_com::persistence::IPersistable)]
struct Persistent;
impl neocogi_com::persistence::IPersistable for Persistent {
    fn type_id(&self) -> Result<neocogi_com::NcGuid, neocogi_com::NcStatus> {
        Ok(PERSISTENT_KIND)
    }
    fn schema_version(&self) -> Result<u32, neocogi_com::NcStatus> {
        Ok(1)
    }
    fn save_state(&self) -> Result<neocogi_com::ComPtr<neocogi_com::INcBytesAbi>, neocogi_com::NcStatus> {
        Ok(neocogi_com::NcBytes::from_vec(vec![1, 2, 3]))
    }
    fn child_count(&self) -> Result<u32, neocogi_com::NcStatus> {
        Ok(0)
    }
    fn child(&self, _: u32) -> Result<neocogi_com::ComPtr<neocogi_com::IUnknown>, neocogi_com::NcStatus> {
        Err(neocogi_com::NC_E_OUT_OF_RANGE)
    }
}

/// Reconstruction validates the initialization received through real COM calls.
#[neocogi_com::com_object(neocogi_com::persistence::IPersistenceFactory)]
struct PersistenceFactory;
impl neocogi_com::persistence::IPersistenceFactory for PersistenceFactory {
    fn validate(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, count: u32) -> Result<(), neocogi_com::NcStatus> {
        if version == 1 && state.offset() == 0 && state.as_slice() == [1, 2, 3] && count == 0 {
            Ok(())
        } else {
            Err(neocogi_com::NC_E_INVALID_ARG)
        }
    }
    fn restore(
        &self,
        version: u32,
        state: NcCopyFromBufferOffset<'_, u8>,
        children: neocogi_com::ComRef<'_, neocogi_com::persistence::IObjectChildrenAbi>,
    ) -> Result<neocogi_com::ComPtr<neocogi_com::IUnknown>, neocogi_com::NcStatus> {
        self.validate(version, state, children.count()?)?;
        Persistent::from_value(Persistent).query()
    }
}

/// The unpublished test owner already uses serde_json for Cargo messages. Use it
/// as an independent syntax oracle over byte mutations, without linking it into
/// either production crate. This corpus avoids intentional range/duplicate-key
/// policy differences and exercises the actual parser inside the loaded library.
fn compare_json_grammar(format: &Format) {
    use neocogi_com::serialization::ISerializationDiagnosticsAbi;
    let original = "[true,false,null,17,-2,0.5,\"é\\n\",{\"nested\":[0,1]}]".as_bytes();
    let mut bytes = original.to_vec();
    for index in 0..bytes.len() {
        for replacement in 0..=127 {
            bytes[index] = replacement;
            let expected = serde_json::from_slice::<serde_json::Value>(&bytes).is_ok();
            let source = format.deserializer(&bytes).unwrap();
            let diagnostic = source.query_ref::<ISerializationDiagnosticsAbi>().unwrap();
            let accepted = diagnostic.failure_status().is_err();
            assert_eq!(accepted, expected, "position {index}, byte {replacement}: {:?}", bytes);
        }
        bytes[index] = original[index];
    }
}
