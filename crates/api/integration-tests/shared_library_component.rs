use neocogi_com::{
    ComPtr, IUnknown, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK, NcAbiHash, NcAbiHashPart, NcAbiPassing, NcAbiType, NcClassDescriptor, NcGuid, NcPluginDescriptor,
    NcStatus, Runtime, com_interface, validate_plugin_descriptor_with_issues,
};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

mod support;
use support::Library;

const WIDGET_CLASS_ID: NcGuid =
    neocogi_com::guid_from_qualified_name("nc.tests.shared_component.TestWidget");
const SHARED_COMPONENT_PLUGIN_ID: NcGuid =
    neocogi_com::guid_from_qualified_name("nc.tests.shared_component");

const WRONG_CLASS_ID: NcGuid = NcGuid::from_fields(
    0x6c7d_b736,
    0x5af7,
    0x4f5f,
    [0xb1, 0xeb, 0xf0, 0x48, 0x2b, 0xa7, 0x19, 0x7f],
);

const RUN_FATAL_LOAD_HELPER_ENV: &str = "NC_COM_RUN_FATAL_LOAD_HELPER";
const RUN_DESCRIPTOR_INSPECTION_HELPER_ENV: &str = "NC_COM_RUN_DESCRIPTOR_INSPECTION_HELPER";
const MISSING_DEPENDENCY_ENV: &str = "NC_COM_TEST_MISSING_PLUGIN_DEPENDENCY";
const LOAD_RETURNED_EXIT_CODE: i32 = 97;

#[com_interface(auto_guid = "nc.tests.shared_component.Widget:v1")]
pub trait IWidget {
    fn value(&self) -> Result<u32, NcStatus>;
}

const WIDGET_CONSTRUCTOR_ABI_HASH: NcAbiHash = NcAbiHash::from_abi_parts(&[NcAbiHashPart::Arg {
    index: 0,
    passing: NcAbiPassing::Value,
    ty: NcAbiType::U32,
}]);

/// Concrete immutable frame matching the dynamically loaded constructor.
#[repr(C)]
struct WidgetArguments {
    /// Base value consumed synchronously by the plugin.
    base: u32,
}

#[test]
fn host_loads_shared_library_class_table_and_constructs_class() {
    let library_path = build_fixture();
    let library = Library::open(&library_path);
    let get_plugin_descriptor = library.get_plugin_descriptor();

    let mut plugin = std::ptr::null();
    let status = unsafe { get_plugin_descriptor(&mut plugin) };
    assert_eq!(status, NC_OK);
    assert!(!plugin.is_null());
    assert_eq!(
        unsafe { validate_plugin_descriptor_with_issues(plugin, |_| {}) },
        NC_OK
    );

    let plugin: &NcPluginDescriptor = unsafe { &*plugin };
    let table = unsafe { &*plugin.class_table };
    let classes = unsafe { table.classes() }.expect("valid class table");
    assert_eq!(classes.len(), 1);

    let class = &classes[0];
    assert_eq!(class.class_id, WIDGET_CLASS_ID);
    assert_eq!(class.constructor_abi_hash, WIDGET_CONSTRUCTOR_ABI_HASH);
    assert_eq!(unsafe { class.qualified_name_str() }.unwrap(), "nc.tests.shared_component.TestWidget");

    constructor_rejects_null_output(class);
    constructor_rejects_wrong_class_descriptor(class);
    constructor_rejects_missing_arguments(class);
    constructor_returns_owned_unknown(class);
}

#[test]
fn runtime_loads_shared_library_and_constructs_class() {
    let library_path = build_fixture();
    let runtime = Runtime::global();

    let loaded = unsafe { runtime.load_plugin_library(&library_path) };
    assert_eq!(loaded.path(), library_path.as_path());
    assert_eq!(loaded.plugin_id(), SHARED_COMPONENT_PLUGIN_ID);
    assert_eq!(loaded.qualified_name(), "nc.tests.shared_component");
    assert_eq!(loaded.plugin_version(), "1.0.0");

    let inspected_module = runtime
        .inspection()
        .loaded_module(loaded.module_id())
        .unwrap()
        .expect("loaded module inspection record");
    assert_eq!(inspected_module, loaded);

    let inspected_plugin = runtime
        .inspection()
        .plugin_info(loaded.plugin_id())
        .unwrap()
        .expect("registered plugin inspection record");
    assert_eq!(inspected_plugin.qualified_name, loaded.qualified_name());
    assert_eq!(inspected_plugin.version, loaded.plugin_version());

    let inspected_class = runtime
        .inspection()
        .class_info(WIDGET_CLASS_ID)
        .unwrap()
        .expect("registered class inspection record");
    assert_eq!(inspected_class.plugin_id, loaded.plugin_id());
    assert_eq!(inspected_class.module_id, Some(loaded.module_id()));

    let arguments = WidgetArguments { base: 32 };
    let unknown = unsafe {
        runtime
            .construct_class_raw(WIDGET_CLASS_ID, WIDGET_CONSTRUCTOR_ABI_HASH, core::ptr::from_ref(&arguments).cast())
            .unwrap()
    };
    let widget = unknown.query::<IWidgetAbi>().unwrap();
    assert_eq!(widget.value(), Ok(42));
}

#[test]
fn runtime_exposes_validated_descriptor_without_registration() {
    if std::env::var_os(RUN_DESCRIPTOR_INSPECTION_HELPER_ENV).is_some() {
        let library_path = build_fixture();
        let runtime = Runtime::global();
        let before = runtime.plugin_registry().plugin_ids().unwrap();

        let visitor = |plugin: &NcPluginDescriptor| {
            // SAFETY: Runtime inspection validated the complete descriptor
            // graph before invoking this callback.
            let qualified_name = unsafe { plugin.qualified_name.as_str()? }.to_owned();
            // SAFETY: The class-table pointer was included in that validation.
            let class_table = unsafe { &*plugin.class_table };
            // SAFETY: The class table was included in that validation.
            let classes = unsafe { class_table.classes()? };
            Ok::<_, NcStatus>((
                plugin.plugin_id,
                qualified_name,
                classes.len(),
                classes[0].class_id,
            ))
        };
        // SAFETY: The shared-component fixture is trusted test code. Every
        // descriptor borrow is consumed within the synchronous callback.
        let observed = unsafe {
            runtime
                .inspection()
                .with_plugin_library_descriptor(&library_path, visitor)
        }
        .unwrap();

        assert_eq!(observed.0, SHARED_COMPONENT_PLUGIN_ID);
        assert_eq!(observed.1, "nc.tests.shared_component");
        assert_eq!(observed.2, 1);
        assert_eq!(observed.3, WIDGET_CLASS_ID);
        assert_eq!(runtime.plugin_registry().plugin_ids().unwrap(), before);
        assert!(!before.contains(&SHARED_COMPONENT_PLUGIN_ID));
        return;
    }

    let output = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("runtime_exposes_validated_descriptor_without_registration")
        .arg("--exact")
        .arg("--nocapture")
        .env(RUN_DESCRIPTOR_INSPECTION_HELPER_ENV, "1")
        .output()
        .expect("failed to run descriptor inspection helper");

    assert!(
        output.status.success(),
        "descriptor inspection helper failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn runtime_aborts_when_mapped_plugin_registration_fails() {
    if std::env::var_os(RUN_FATAL_LOAD_HELPER_ENV).is_some() {
        let library_path = build_fixture();
        // SAFETY: The fixture is trusted test code. Its selected descriptor has
        // a deliberately missing hard dependency, so live activation must
        // abort rather than return.
        let _ = unsafe { Runtime::global().load_plugin_library(&library_path) };
        std::process::exit(LOAD_RETURNED_EXIT_CODE);
    }

    let output = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("runtime_aborts_when_mapped_plugin_registration_fails")
        .arg("--exact")
        .arg("--nocapture")
        .env(RUN_FATAL_LOAD_HELPER_ENV, "1")
        .env(MISSING_DEPENDENCY_ENV, "1")
        .output()
        .expect("failed to run fatal plugin load helper");

    assert_ne!(output.status.code(), Some(LOAD_RETURNED_EXIT_CODE));
    assert!(
        !output.status.success(),
        "plugin load helper exited successfully\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    #[cfg(unix)]
    assert!(
        output.status.signal().is_some(),
        "plugin load helper did not abort\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn constructor_rejects_null_output(class: &NcClassDescriptor) {
    let constructor = class.construct.unwrap();
    let arguments = WidgetArguments { base: 5 };
    let status = unsafe { constructor(class, core::ptr::from_ref(&arguments).cast(), std::ptr::null_mut()) };
    assert_eq!(status, NC_E_POINTER);
}

fn constructor_rejects_wrong_class_descriptor(class: &NcClassDescriptor) {
    let constructor = class.construct.unwrap();
    let arguments = WidgetArguments { base: 5 };
    let mut wrong_class = *class;
    wrong_class.class_id = WRONG_CLASS_ID;
    let mut out = std::ptr::dangling_mut();

    let status = unsafe { constructor(&wrong_class, core::ptr::from_ref(&arguments).cast(), &mut out) };

    assert_eq!(status, NC_E_INVALID_ARG);
    assert!(out.is_null());
}

fn constructor_rejects_missing_arguments(class: &NcClassDescriptor) {
    let constructor = class.construct.unwrap();
    let mut out = std::ptr::dangling_mut();

    let status = unsafe { constructor(class, std::ptr::null(), &mut out) };

    assert_eq!(status, NC_E_INVALID_ARG);
    assert!(out.is_null());
}

fn constructor_returns_owned_unknown(class: &NcClassDescriptor) {
    let constructor = class.construct.unwrap();
    let arguments = WidgetArguments { base: 32 };
    let mut out = std::ptr::null_mut();

    let status = unsafe { constructor(class, core::ptr::from_ref(&arguments).cast(), &mut out) };

    assert_eq!(status, NC_OK);
    assert!(!out.is_null());

    let unknown = unsafe { ComPtr::<IUnknown>::from_raw_owned(out) }.unwrap();
    let widget = unknown.query::<IWidgetAbi>().unwrap();
    assert_eq!(widget.value(), Ok(42));
}

fn build_fixture() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("integration-tests/fixtures/shared-library-component/Cargo.toml");
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/shared-library-component-fixture");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let output = Command::new(cargo)
        .arg("build")
        .arg("--offline")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target_dir)
        .output()
        .expect("failed to build shared-library component fixture");

    assert!(
        output.status.success(),
        "shared-library component fixture failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    target_dir.join("debug").join(format!(
        "{}nc_com_shared_component{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}
