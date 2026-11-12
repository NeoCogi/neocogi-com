//! The production thin shim exposes the same factories and diagnostics as static linking.
use std::{path::Path, process::Command};
use neocogi_com::{ComSerialize, ComDeserialize, NcCopyFromBufferOffset};
use neocogi_com::serialization::{ISerializationPluginAbi, Format, Limits};

/// Application-owned state never crosses the native boundary as Rust storage.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct State {
    count: u32,
    label: String,
    active: bool,
    /// Functional float serialization must survive target Debug stripping.
    weight: f64,
}

/// Loads an actual independently linked cdylib, acquires all enabled formats,
/// performs real vtable calls and copies a detailed failure back to the caller.
#[test]
fn native_shim_supplies_formats_and_failure_details() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let build = Command::new("cargo")
        .args(["xtask", "min-size", "--features", "json", "--json-output"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let artifact = String::from_utf8(build.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|record| record["artifact"].as_str().map(std::path::PathBuf::from))
        .unwrap();
    // SAFETY: this final test executable retains its selected host implementation.
    let runtime = unsafe { neocogi_com::Runtime::__from_host_api(neocogi_com_host::initialize_process()) }.unwrap();
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
            compare_json_grammar(&format);
        }
    }
    let default = Format::new(root.default_format().unwrap(), Limits::default());
    assert!(default.serialize(&value).unwrap().starts_with(b"NCB\x01"));
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
