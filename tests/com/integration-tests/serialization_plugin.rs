//! The production thin shim exposes the same factories and diagnostics as static linking.
use std::{path::Path, process::Command};
use neocogi_com_sdk::{ComSerialize, ComDeserialize, NcCopyFromBufferOffset};
use neocogi_com_sdk::serialization::{ISerializationPluginAbi, Format, Limits};

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

/// Tells the child harness to load the exact deployed library from the parent.
const STAGED_PLUGIN: &str = "NC_TEST_STAGED_SERIALIZATION_PLUGIN";

/// Stages the real deployment artifact and loads it from a fresh host process.
#[test]
fn native_shim_supplies_formats_and_failure_details() {
    if let Some(path) = std::env::var_os(STAGED_PLUGIN) {
        exercise_staged_plugin(Path::new(&path));
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let stage = tempfile::tempdir().unwrap();
    let build = Command::new("rustup")
        .args(["run", "nightly", "cargo", "xtask", "stage-serialization"])
        .arg(stage.path())
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));

    let filename = format!("{}nc-serialization{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    let artifact = stage.path().join(filename);
    assert!(artifact.is_file(), "expected deployed plugin at {}", artifact.display());
    // The child owns the loaded library until it exits, so Windows can also
    // delete the staging directory after the native host has finished.
    let host = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "native_shim_supplies_formats_and_failure_details", "--nocapture"])
        .env(STAGED_PLUGIN, &artifact)
        .output()
        .unwrap();
    assert!(
        host.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&host.stdout),
        String::from_utf8_lossy(&host.stderr)
    );
    assert!(String::from_utf8_lossy(&host.stdout).contains("test result: ok. 1 passed"));
    stage.close().unwrap();
}

/// Acquires both built-in formats, exercises calls, and copies back diagnostics.
fn exercise_staged_plugin(artifact: &Path) {
    let runtime = neocogi_com_host::initialize_runtime();
    // SAFETY: The parent stages a trusted local build of the production plugin.
    let id = unsafe { runtime.load_plugin_library(artifact) };
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
fn persistence_round_trip(runtime: &neocogi_com_sdk::RuntimeHandle) {
    use neocogi_com_sdk::persistence::{FactoryRegistry, IPersistableAbi, SavedObject, save_object_tree};
    let value = Persistent::from_value(Persistent);
    let bytes = save_object_tree(runtime, &value).unwrap();
    let expected = neocogi_com_serialization::ncb::to_vec(&SavedObject {
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
    let restored = registry.restore_object_tree(&bytes).unwrap();
    assert_eq!(restored.query::<IPersistableAbi>().unwrap().schema_version().unwrap(), 1);
    assert_eq!(save_object_tree(runtime, &restored).unwrap(), bytes);
    for end in 0..bytes.len() {
        assert_eq!(registry.restore_object_tree(&bytes[..end]).unwrap_err(), neocogi_com_sdk::NC_E_INVALID_ARG);
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(registry.restore_object_tree(&trailing).unwrap_err(), neocogi_com_sdk::NC_E_INVALID_ARG);
}

/// Explicit recipe identity for an application object crossing the loaded codec.
const PERSISTENT_KIND: neocogi_com_sdk::NcGuid = neocogi_com_sdk::guid_from_qualified_name("nc.tests.loaded-serialization.persistence");

/// Opaque application state; the host and codec own only its persistence envelope.
#[neocogi_com_sdk::com_object(neocogi_com_sdk::persistence::IPersistable)]
struct Persistent;
impl neocogi_com_sdk::persistence::IPersistable for Persistent {
    fn type_id(&self) -> Result<neocogi_com_sdk::NcGuid, neocogi_com_sdk::NcStatus> {
        Ok(PERSISTENT_KIND)
    }
    fn schema_version(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {
        Ok(1)
    }
    fn save_state(&self) -> Result<neocogi_com_sdk::ComPtr<neocogi_com_sdk::INcBytesAbi>, neocogi_com_sdk::NcStatus> {
        Ok(neocogi_com_sdk::NcBytes::from_vec(vec![1, 2, 3]))
    }
    fn child_count(&self) -> Result<u32, neocogi_com_sdk::NcStatus> {
        Ok(0)
    }
    fn child(&self, _: u32) -> Result<neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown>, neocogi_com_sdk::NcStatus> {
        Err(neocogi_com_sdk::NC_E_OUT_OF_RANGE)
    }
}

/// Reconstruction validates the initialization received through real COM calls.
#[neocogi_com_sdk::com_object(neocogi_com_sdk::persistence::IPersistenceFactory)]
struct PersistenceFactory;
impl neocogi_com_sdk::persistence::IPersistenceFactory for PersistenceFactory {
    fn validate(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, count: u32) -> Result<(), neocogi_com_sdk::NcStatus> {
        if version == 1 && state.offset() == 0 && state.as_slice() == [1, 2, 3] && count == 0 {
            Ok(())
        } else {
            Err(neocogi_com_sdk::NC_E_INVALID_ARG)
        }
    }
    fn restore(
        &self,
        version: u32,
        state: NcCopyFromBufferOffset<'_, u8>,
        children: neocogi_com_sdk::ComRef<'_, neocogi_com_sdk::persistence::IObjectChildrenAbi>,
    ) -> Result<neocogi_com_sdk::ComPtr<neocogi_com_sdk::IUnknown>, neocogi_com_sdk::NcStatus> {
        self.validate(version, state, children.count()?)?;
        Persistent::from_value(Persistent).query()
    }
}

/// The unpublished test owner already uses serde_json for Cargo messages. Use it
/// as an independent syntax oracle over byte mutations, without linking it into
/// either production crate. This corpus avoids intentional range/duplicate-key
/// policy differences and exercises the actual parser inside the loaded library.
fn compare_json_grammar(format: &Format) {
    use neocogi_com_sdk::serialization::ISerializationDiagnosticsAbi;
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
