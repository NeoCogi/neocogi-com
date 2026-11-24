//! Real native imports, direct getters, and forwarding exports through host admission.

use std::{
    ffi::c_void,
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

use neocogi_com_host::{PluginLoadErrorKind, inspect_plugin_library};
use neocogi_com_sdk::{NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_TYPE_PLUGIN, NC_E_DROPPED, NC_E_LOAD_FAILED, NcDebugMessageV4, NcDebugMessengerCreateInfoV4, PluginOrigin};

/// Supplies an isolated native fixture or selects the ordinary parent setup.
const FIXTURE_PATH: &str = "NC_COM_TEST_GETTER_ORIGIN";
/// Distinguishes a returned mandatory load from the expected process abort.
const LIVE_LOAD_RETURNED: i32 = 97;
/// The provider named by the Mach-O reexport or PE forwarder.
const FORWARD_TARGET: &str = "nc_origin_forward_target";

/// Stages independently built files, then exercises inspection and mandatory loading
/// in subprocesses so the pinned files and the runtime catalog are scoped to each child.
#[test]
fn getters_must_be_exported_by_the_selected_native_module() {
    if let Some(path) = std::env::var_os(FIXTURE_PATH) {
        let path = Path::new(&path);
        if path.is_dir() {
            inspect_and_register(path);
        } else {
            reject_live_load(path);
        }
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    #[cfg(any(target_os = "macos", windows))]
    let provider = super::build_case_with_name(0, root, Some(FORWARD_TARGET));
    #[cfg(not(any(target_os = "macos", windows)))]
    let provider = super::build_case(0, root);
    // Export admission also accepts a caller-selected alias of a real provider.
    #[cfg(unix)]
    std::os::unix::fs::symlink(provider, native_path(root, "alias")).unwrap();
    #[cfg(windows)]
    fs::hard_link(provider, native_path(root, "alias")).unwrap();
    build_native(root, "nc_origin_dependency", Some("NC_ORIGIN_DEPENDENCY"), false);
    build_native(root, "local", Some("NC_ORIGIN_LOCAL_GETTER"), false);
    build_native(root, "import_only", None, false);
    #[cfg(any(target_os = "macos", windows))]
    build_native(root, "forwarded", Some("NC_ORIGIN_FORWARD_ONLY"), true);

    let child = |path: &Path| {
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native_getter_origin::getters_must_be_exported_by_the_selected_native_module",
                "--nocapture",
            ])
            .env(FIXTURE_PATH, path)
            .output()
            .unwrap()
    };
    let valid = child(root);
    assert!(valid.status.success(), "{}", String::from_utf8_lossy(&valid.stderr));
    assert!(String::from_utf8_lossy(&valid.stdout).contains("native getter cases completed"));

    let failure = child(&native_path(root, "import_only"));
    let stderr = String::from_utf8_lossy(&failure.stderr);
    assert!(!failure.status.success(), "{stderr}");
    assert_ne!(failure.status.code(), Some(LIVE_LOAD_RETURNED), "{stderr}");
    assert!(stderr.contains("fatal neocogi-com-sdk plugin load failure: FindSymbol"), "{stderr}");
    assert!(stderr.contains("catalog empty at rejection"), "{stderr}");
}

/// Runs the positive and negative callers against actual native mappings.
fn inspect_and_register(directory: &Path) {
    let runtime = neocogi_com_host::initialize_runtime();
    // SAFETY: each library below was built by the parent from trusted fixture
    // sources. Direct controls use the exact getter signature and valid storage.
    unsafe {
        let local = native_path(directory, "local");
        let error = inspect_plugin_library(local).unwrap_err();
        assert_eq!(error.kind(), PluginLoadErrorKind::Entrypoint);
        assert_eq!(error.status(), Some(NC_E_DROPPED));
        assert_missing_export(directory);
        assert_eq!(
            runtime.with_plugin_library_descriptor(native_path(directory, "import_only"), |_| -> Result<(), _> {
                panic!("an import-only library has no admitted descriptor")
            }),
            Err(NC_E_LOAD_FAILED)
        );
        assert!(runtime.plugins().unwrap().is_empty());

        #[cfg(any(target_os = "macos", windows))]
        admit_forwarded_export(directory, runtime);

        let before = runtime.plugins().unwrap();
        let alias = native_path(directory, "alias");
        let metadata = inspect_plugin_library(&alias).unwrap();
        let id = runtime.with_plugin_library_descriptor(&alias, |descriptor| Ok(descriptor.plugin_id)).unwrap();
        assert_eq!(id, metadata.plugin_id);
        assert_eq!(runtime.plugins().unwrap(), before);

        #[cfg(not(any(target_os = "macos", windows)))]
        {
            assert_eq!(runtime.load_plugin_library(alias.clone()), id);
            assert_eq!(runtime.plugin_info(id).unwrap().unwrap().origin, PluginOrigin::Dynamic { path: alias });
        }
    }
    println!("native getter cases completed");
}

/// Requires the exact lookup phase and original path for an ordinary import.
///
/// # Safety
/// The parent owns the trusted fixtures in `directory`.
unsafe fn assert_missing_export(directory: &Path) {
    let path = native_path(directory, "import_only");
    // SAFETY: the parent built this independently linked native control.
    let error = unsafe { inspect_plugin_library(&path) }.unwrap_err();
    assert_eq!(error.kind(), PluginLoadErrorKind::FindSymbol, "{error}");
    assert_eq!(error.path(), path);
    #[cfg(target_os = "linux")]
    assert!(error.message().contains("different native module"), "{error}");
}

/// Admits a native export facade backed by a different trusted image. Only
/// required loading publishes the facade path as the descriptor's origin.
///
/// # Safety
/// The parent owns the trusted fixtures in `directory`.
#[cfg(any(target_os = "macos", windows))]
unsafe fn admit_forwarded_export(directory: &Path, runtime: neocogi_com_sdk::RuntimeHandle) {
    #[cfg(windows)]
    let donor = {
        use std::os::windows::ffi::OsStrExt as _;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn LoadLibraryExW(path: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
        }

        // The PE forward-only facade has no ordinary import of its donor.
        // Preload the exact donor for native name resolution without changing
        // Windows search directories or adding the fixture to ambient search.
        let path = native_path(directory, FORWARD_TARGET);
        let wide = path.as_os_str().encode_wide().chain([0]).collect::<Vec<_>>();
        // SAFETY: `wide` is the terminated path of the trusted donor fixture.
        let donor = unsafe { LoadLibraryExW(wide.as_ptr(), core::ptr::null_mut(), 0x0000_0900) };
        assert!(!donor.is_null(), "cannot preload the forward target: {}", std::io::Error::last_os_error());
        donor
    };

    let path = native_path(directory, "forwarded");
    // SAFETY: the trusted facade explicitly exports the real versioned getter.
    let metadata = unsafe { inspect_plugin_library(&path) }.unwrap();
    // SAFETY: inspecting the same trusted facade admits the static descriptor.
    let (id, acquire) = unsafe { runtime.with_plugin_library_descriptor(&path, |descriptor| Ok((descriptor.plugin_id, descriptor.acquire.unwrap()))) }.unwrap();
    assert_eq!(id, metadata.plugin_id);
    assert!(runtime.plugins().unwrap().is_empty());
    // SAFETY: loading the trusted facade publishes the valid descriptor.
    assert_eq!(unsafe { runtime.load_plugin_library(&path) }, id);
    assert_eq!(runtime.plugin_info(id).unwrap().unwrap().origin, PluginOrigin::Dynamic { path });

    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn FreeLibrary(module: *mut c_void) -> i32;
        }

        // SAFETY: release exactly the matching reference obtained by the test.
        assert_ne!(unsafe { FreeLibrary(donor) }, 0, "cannot release the test's donor reference");
    }
    let mut out = neocogi_com_sdk::ComRaw::null();
    // SAFETY: admitted provider code remains pinned after the caller's preload
    // reference is released. `out` is a writable, initially empty COM output.
    assert_eq!(
        unsafe { acquire(neocogi_com_host::initialize_process(), &mut out) },
        neocogi_com_sdk::NC_E_CLOSED
    );
    assert!(out.is_null());
}

/// Checks the public mandatory-load boundary without terminating the parent.
fn reject_live_load(path: &Path) {
    let runtime = neocogi_com_host::initialize_runtime();
    let info = NcDebugMessengerCreateInfoV4 {
        size: size_of::<NcDebugMessengerCreateInfoV4>(),
        severity_mask: NC_DEBUG_SEVERITY_ERROR,
        value_type_mask: NC_DEBUG_TYPE_PLUGIN,
        callback: Some(observe_rejected_load),
        user_data: core::ptr::null_mut(),
    };
    // SAFETY: the static observer ignores user data and is reentrant. The child
    // opens an independently compiled trusted fixture and expects a fatal status.
    let _observer = unsafe { runtime.debug().create_messenger(&info) }.unwrap();
    // SAFETY: this child opens the parent's trusted import-only fixture.
    unsafe { runtime.load_plugin_library(path) };
    std::process::exit(LIVE_LOAD_RETURNED);
}

/// Observes the committed catalog at the error callback before mandatory abort.
unsafe extern "C" fn observe_rejected_load(_: u32, _: u32, message: *const NcDebugMessageV4, _: *mut c_void) {
    // SAFETY: the debug callback receives one readable message for this call.
    if unsafe { (*message).status } == NC_E_LOAD_FAILED && neocogi_com_host::initialize_runtime().plugins().is_ok_and(|plugins| plugins.is_empty()) {
        let _ = writeln!(std::io::stderr().lock(), "catalog empty at rejection");
    }
}

/// Derives the platform filename for a test-owned native module.
fn native_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{}{name}{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX))
}

/// Builds a companion against a real native import. Apple and Windows also
/// supply an explicit loader-level export facade with no local getter.
fn build_native(root: &Path, name: &str, define: Option<&str>, forward: bool) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("integration-tests/fixtures/plugin-inspection/native_origin.c");
    let library = native_path(root, name);
    let dependency = native_path(root, if forward { FORWARD_TARGET } else { "nc_origin_dependency" });
    let is_dependency = define == Some("NC_ORIGIN_DEPENDENCY");
    let compiler = cc::Build::new()
        .target(env!("NC_COM_TEST_TARGET"))
        .host(env!("NC_COM_TEST_HOST"))
        .opt_level(0)
        .debug(false)
        .out_dir(root)
        .cargo_metadata(false)
        .get_compiler();
    let mut command = compiler.to_command();
    command.current_dir(root);
    let export = root.join("forwarded.def");
    if forward && cfg!(windows) {
        fs::write(
            &export,
            format!("EXPORTS\n    nc_get_plugin_descriptor_v4={FORWARD_TARGET}.nc_get_plugin_descriptor_v4\n"),
        )
        .unwrap();
    }
    if compiler.is_like_msvc() {
        command.arg("/LD");
        if let Some(define) = define {
            command.arg(format!("/D{define}"));
        }
        command.arg(source).arg("/link").arg(format!("/OUT:{}", library.display()));
        let import = root.join("nc_origin_dependency.lib");
        if is_dependency {
            command.arg(format!("/IMPLIB:{}", import.display()));
        } else if !forward {
            command.arg(import);
        }
        if forward {
            command.arg(format!("/DEF:{}", export.display()));
        }
    } else {
        command.args(if cfg!(target_os = "macos") {
            ["-dynamiclib", "-fPIC"]
        } else {
            ["-shared", "-fPIC"]
        });
        if let Some(define) = define {
            command.arg(format!("-D{define}"));
        }
        command.arg(source).arg("-o").arg(&library);
        if cfg!(windows) {
            let import = root.join("libnc_origin_dependency.dll.a");
            if is_dependency {
                command.arg(format!("-Wl,--out-implib,{}", import.display()));
            } else if !forward {
                command.arg(import);
            }
            if forward {
                command.arg(export);
            }
        } else if forward {
            command.args(["-Xlinker", "-reexport_library", "-Xlinker"]).arg(dependency);
        } else if !is_dependency {
            command.arg(dependency);
        }
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{name}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
