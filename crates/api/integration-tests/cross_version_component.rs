use neocogi_com::{
    ComInterface, ComPtr, ComRef, IUnknown, NC_ABI_THREADING_THREAD_SAFE, NC_OK, NcAbiSignature, NcAbiSignaturePart, NcAbiPassing, NcAbiType,
    NcClassDescriptor, NcGuid, NcPluginDescriptor, NcStatus, Runtime, ThreadSafe, com_object, com_interface, validate_plugin_descriptor_with_issues,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod support;
use support::Library;

const CROSS_VERSION_CLASS_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.tests.cross_version.Service");
const RUN_ISOLATED_HELPER_ENV: &str = "NC_COM_RUN_CROSS_VERSION_HELPER";
const ISOLATED_COMPLETION_PREFIX: &str = "cross-version paths completed: ";

#[com_interface(auto_guid = "nc.tests.cross_version.Number:v1")]
pub trait ICrossVersionNumber {
    fn value(&self) -> Result<u32, NcStatus>;
}

const SERVICE_CONSTRUCTOR_ABI_SIGNATURE: NcAbiSignature = neocogi_com::abi_signature!(&[NcAbiSignaturePart::Arg {
    index: 0,
    passing: NcAbiPassing::Value,
    ty: NcAbiType::U32,
}]);

/// Concrete immutable frame matching the cross-version service constructor.
#[repr(C)]
struct ServiceArguments {
    /// Seed consumed synchronously by the plugin.
    seed: u32,
}

#[com_interface(auto_guid = "nc.tests.cross_version.Service:v1")]
pub trait ICrossVersionService {
    fn add(&self, left: u32, right: u32) -> Result<u32, NcStatus>;
    fn reset(&self) -> Result<(), NcStatus>;
    fn read_number(&self, number: ComRef<'_, ICrossVersionNumberAbi>) -> Result<u32, NcStatus>;
    fn make_number(&self, value: u32) -> Result<ComPtr<ICrossVersionNumberAbi>, NcStatus>;
}

#[com_object(interfaces(ICrossVersionNumber))]
pub struct HostNumber {
    value: u32,
}

impl ICrossVersionNumber for HostNumber {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

#[derive(Clone, Copy)]
struct FixtureBuild {
    toolchain: &'static str,
    profile: FixtureProfile,
}

#[derive(Clone, Copy)]
enum FixtureProfile {
    Debug,
    Release { panic: &'static str, lto: &'static str },
}

impl FixtureBuild {
    const fn debug(toolchain: &'static str) -> Self {
        Self {
            toolchain,
            profile: FixtureProfile::Debug,
        }
    }

    const fn release(toolchain: &'static str, panic: &'static str, lto: &'static str) -> Self {
        Self {
            toolchain,
            profile: FixtureProfile::Release { panic, lto },
        }
    }

    fn label(self) -> String {
        match self.profile {
            FixtureProfile::Debug => format!("{} debug", self.toolchain),
            FixtureProfile::Release { panic, lto } => {
                format!("{} release panic={panic} lto={lto}", self.toolchain)
            }
        }
    }

    fn target_dir_name(self) -> String {
        match self.profile {
            FixtureProfile::Debug => format!("{}-debug", self.toolchain),
            FixtureProfile::Release { panic, lto } => {
                format!("{}-release-panic-{panic}-lto-{lto}", self.toolchain)
            }
        }
    }

    const fn artifact_profile_dir(self) -> &'static str {
        match self.profile {
            FixtureProfile::Debug => "debug",
            FixtureProfile::Release { .. } => "release",
        }
    }
}

#[test]
fn stable_debug_producer_component_is_abi_compatible() {
    exercise_cross_version_component_isolated(FixtureBuild::debug("stable"), "stable_debug_producer_component_is_abi_compatible");
}

#[test]
fn nightly_debug_producer_component_is_abi_compatible() {
    exercise_cross_version_component_isolated(FixtureBuild::debug("nightly"), "nightly_debug_producer_component_is_abi_compatible");
}

#[test]
fn stable_release_abort_thin_lto_producer_component_is_abi_compatible() {
    exercise_cross_version_component_isolated(
        FixtureBuild::release("stable", "abort", "thin"),
        "stable_release_abort_thin_lto_producer_component_is_abi_compatible",
    );
}

#[test]
fn nightly_release_unwind_fat_lto_producer_component_is_abi_compatible() {
    exercise_cross_version_component_isolated(
        FixtureBuild::release("nightly", "unwind", "fat"),
        "nightly_release_unwind_fat_lto_producer_component_is_abi_compatible",
    );
}

fn exercise_cross_version_component_isolated(build: FixtureBuild, test_name: &str) {
    let label = build.label();
    if std::env::var(RUN_ISOLATED_HELPER_ENV).as_deref() == Ok(label.as_str()) {
        exercise_cross_version_component(build);
        println!("{ISOLATED_COMPLETION_PREFIX}{label}");
        return;
    }

    let output = Command::new(std::env::current_exe().expect("cross-version test executable"))
        .arg(test_name)
        .arg("--exact")
        .arg("--nocapture")
        .env(RUN_ISOLATED_HELPER_ENV, &label)
        .output()
        .unwrap_or_else(|error| panic!("failed to run isolated cross-version helper `{label}`: {error}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "isolated cross-version helper failed for `{label}`\nstatus: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status,
    );
    let completion = format!("{ISOLATED_COMPLETION_PREFIX}{label}");
    assert!(
        stdout.contains(&completion),
        "isolated cross-version helper skipped a required path for `{label}`\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );
}

fn exercise_cross_version_component(build: FixtureBuild) {
    let library_path = build_fixture(build);
    {
        let library = Library::open(&library_path);
        exercise_descriptor_entrypoint(&library);
    }
    exercise_runtime_loader(&library_path);
}

fn exercise_descriptor_entrypoint(library: &Library) {
    let get_plugin_descriptor = library.get_plugin_descriptor();

    let mut plugin = std::ptr::null();
    let status = unsafe { get_plugin_descriptor(&mut plugin) };
    assert_eq!(status, NC_OK);
    assert!(!plugin.is_null());
    assert_eq!(unsafe { validate_plugin_descriptor_with_issues(plugin, |_| {}) }, NC_OK);

    let plugin: &NcPluginDescriptor = unsafe { &*plugin };
    let table = unsafe { &*plugin.class_table };
    let classes = unsafe { table.classes() }.expect("valid class table");
    assert_eq!(classes.len(), 1);

    let class = &classes[0];
    assert_eq!(class.class_id, CROSS_VERSION_CLASS_ID);
    assert_eq!(class.constructor_abi_signature, SERVICE_CONSTRUCTOR_ABI_SIGNATURE);
    assert_eq!(class.threading, NC_ABI_THREADING_THREAD_SAFE);
    require_class_interface::<ICrossVersionServiceAbi>(class);

    let service = construct_service_from_descriptor(class);
    exercise_service(service);
}

fn exercise_runtime_loader(library_path: &Path) {
    let runtime = Runtime::global();
    let loaded = unsafe { runtime.load_plugin_library(library_path) };
    assert_eq!(loaded.qualified_name(), "nc.tests.cross_version");
    assert_eq!(loaded.plugin_version(), "1.0.0");

    runtime.require_interface_type::<ICrossVersionServiceAbi>(CROSS_VERSION_CLASS_ID).unwrap();

    let arguments = ServiceArguments { seed: 11 };
    let arguments_pointer = core::ptr::from_ref(&arguments).cast();
    let unknown = unsafe {
        runtime
            .construct_class_raw(CROSS_VERSION_CLASS_ID, SERVICE_CONSTRUCTOR_ABI_SIGNATURE, arguments_pointer)
            .expect("registry output")
    };
    let service = unknown.query::<ICrossVersionServiceAbi>().expect("registry-constructed cross-version service");
    exercise_service(service);

    let thread_safe_unknown = unsafe {
        runtime
            .construct_class_with_model_raw::<ThreadSafe>(CROSS_VERSION_CLASS_ID, SERVICE_CONSTRUCTOR_ABI_SIGNATURE, arguments_pointer)
            .expect("thread-safe registry output")
    };
    let thread_safe_service = thread_safe_unknown.query::<ICrossVersionServiceAbi>().expect("thread-safe service projection");
    assert_eq!(std::thread::spawn(move || thread_safe_service.add(10, 21)).join().unwrap(), Ok(42));
}

fn exercise_service(service: ComPtr<ICrossVersionServiceAbi>) {
    assert_eq!(service.add(2, 5), Ok(18));
    assert_eq!(service.reset(), Ok(()));

    let host_number = HostNumber::from_value(HostNumber { value: 7 }).query::<ICrossVersionNumberAbi>().unwrap();
    assert_eq!(service.read_number(host_number.as_com_ref()), Ok(18));

    let returned_number = service.make_number(31).unwrap();
    assert_eq!(returned_number.value(), Ok(42));
}

fn require_class_interface<T: ComInterface>(class: &NcClassDescriptor) {
    let interfaces = unsafe { std::slice::from_raw_parts(class.interfaces, class.interface_count as usize) };
    let descriptor = interfaces
        .iter()
        .map(|interface| unsafe { &*interface.interface_descriptor })
        .find(|descriptor| descriptor.interface_id == T::IID)
        .expect("class exposes expected interface");
    assert_eq!(descriptor.interface_id, T::IID);
    assert_eq!(descriptor.abi_signature, T::ABI_SIGNATURE);
}

fn construct_service_from_descriptor(class: &NcClassDescriptor) -> ComPtr<ICrossVersionServiceAbi> {
    let arguments = ServiceArguments { seed: 11 };
    let constructor = class.construct.expect("class has constructor");
    let mut out = std::ptr::null_mut();
    let status = unsafe { constructor(class, core::ptr::from_ref(&arguments).cast(), &mut out) };
    assert_eq!(status, NC_OK);
    assert!(!out.is_null());
    let unknown = unsafe { ComPtr::<IUnknown>::from_raw_owned(out) }.expect("constructor output");
    unknown.query::<ICrossVersionServiceAbi>().expect("constructed cross-version service")
}

fn build_fixture(build: FixtureBuild) -> PathBuf {
    require_toolchain(build.toolchain);

    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("integration-tests/fixtures/cross-version-component");
    let manifest = fixture_dir.join("Cargo.toml");
    let lockfile = fixture_dir.join("Cargo.lock");
    let lockfile_before = fs::read(&lockfile).unwrap_or_else(|error| panic!("failed to read {}: {error}", lockfile.display()));
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/cross-version-component-fixture")
        .join(build.target_dir_name());
    let mut command = Command::new("rustup");
    command
        .arg("run")
        .arg(build.toolchain)
        .arg("cargo")
        .arg("build")
        .arg("--offline")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target_dir);
    if let FixtureProfile::Release { panic, lto } = build.profile {
        command
            .arg("--release")
            .env("CARGO_PROFILE_RELEASE_PANIC", panic)
            .env("CARGO_PROFILE_RELEASE_LTO", lto);
    }

    let label = build.label();
    let output = command.output().unwrap_or_else(|error| {
        panic!(
            "failed to invoke rustup run {} cargo build for cross-version fixture ({label}): {error}",
            build.toolchain,
        )
    });

    let lockfile_after = fs::read(&lockfile).unwrap_or_else(|error| panic!("failed to reread {}: {error}", lockfile.display()));
    assert_eq!(
        lockfile_after,
        lockfile_before,
        "cross-version fixture build modified {} for producer `{label}`",
        lockfile.display(),
    );

    assert!(
        output.status.success(),
        "cross-version fixture failed for producer build `{label}`\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    target_dir.join(build.artifact_profile_dir()).join(format!(
        "{}nc_com_cross_version_component{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

fn require_toolchain(toolchain: &str) {
    let output = Command::new("rustup")
        .arg("run")
        .arg(toolchain)
        .arg("cargo")
        .arg("--version")
        .output()
        .unwrap_or_else(|error| {
            panic!("failed to invoke rustup; install rustup and the `{toolchain}` Rust toolchain with `rustup toolchain install {toolchain}`: {error}")
        });

    assert!(
        output.status.success(),
        "required Rust toolchain `{toolchain}` is not available; install it with `rustup toolchain install {toolchain}`\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
