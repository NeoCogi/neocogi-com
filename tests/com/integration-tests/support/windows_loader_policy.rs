//! Test-owned native fixtures and bounded, isolated Windows policy cases.

use std::{
    ffi::{OsStr, OsString, c_void},
    fs,
    io::Write,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

use neocogi_com::{NC_E_LOAD_FAILED, PluginOrigin, guid_from_qualified_name};
use neocogi_com_host::{PluginLoadErrorKind, inspect_plugin_library};

/// Unique basename shared by both helper implementations and the provider import.
const HELPER: &str = "nc_policy_helper_83d97.dll";
/// Unique provider basename used to distinguish explicit paths from PATH lookup.
const PROVIDER: &str = "nc_policy_provider_83d97.dll";
/// An unexpectedly returned mandatory load is distinct from abort and observation failure.
const RETURNED: i32 = 97;
/// The C initializer terminates with this code when marker instrumentation fails.
const OBSERVER_FAILED: i32 = 98;

/// Exhaustive isolated operations; only the parent creates filesystem layouts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Case {
    /// Establish the current toolchain's actual process-abort status.
    AbortControl,
    /// Prove the genuine helper initializer is observable without a descriptor.
    ObserveGenuine,
    /// Prove the alternative helper initializer is observable without a descriptor.
    ObserveAlternative,
    /// Prove marker creation failure cannot be mistaken for open rejection.
    ObserverFailure,
    /// Absolute provider with adjacent genuine helper and CWD alternative.
    OwnedConflict,
    /// Relative provider, absent genuine helper and CWD alternative.
    OwnedMissingCwd,
    /// Absent genuine helper, alternatives beside the executable and in a user directory.
    OwnedMissingExtraDirectories,
    /// Absent genuine helper, alternative only in PATH.
    OwnedMissingPath,
    /// Bare provider absent from CWD, with a complete alternative deployment in PATH.
    OwnedBarePath,
    /// Relative provider under a Unicode directory, with an adjacent genuine helper.
    OwnedRelativeUnicode,
    /// Interior NUL must fail before mapping the otherwise valid provider.
    OwnedNul,
    /// A missing image must retain the caller's spelling in its recoverable error.
    OwnedMissingImage,
    /// An empty path is not permission to search for a module.
    OwnedEmpty,
    /// The callback adapter visits one admitted descriptor exactly once.
    CallbackSuccess,
    /// The callback adapter reports a loader error without calling the visitor.
    CallbackMissing,
    /// Required loading registers the ID and original relative provenance.
    LiveSuccess,
    /// Required loading diagnoses the open phase and aborts before the getter.
    LiveMissing,
}

impl Case {
    /// Finite dispatch inventory also rejects unknown child environment values.
    const ALL: [Self; 17] = [
        Self::AbortControl,
        Self::ObserveGenuine,
        Self::ObserveAlternative,
        Self::ObserverFailure,
        Self::OwnedConflict,
        Self::OwnedMissingCwd,
        Self::OwnedMissingExtraDirectories,
        Self::OwnedMissingPath,
        Self::OwnedBarePath,
        Self::OwnedRelativeUnicode,
        Self::OwnedNul,
        Self::OwnedMissingImage,
        Self::OwnedEmpty,
        Self::CallbackSuccess,
        Self::CallbackMissing,
        Self::LiveSuccess,
        Self::LiveMissing,
    ];

    /// Successful provider opens must select the genuine adjacent helper.
    fn opens_provider(self) -> bool {
        matches!(
            self,
            Self::OwnedConflict | Self::OwnedRelativeUnicode | Self::CallbackSuccess | Self::LiveSuccess
        )
    }
}

/// Parent-owned fixture artifacts outlive all children; no mapping is opened here.
struct Fixtures {
    /// Removes staged DLLs, compiler outputs, copied executable and logs after reaping.
    owner: tempfile::TempDir,
    /// Directory containing the one copied test executable; never a shared Cargo directory.
    host: PathBuf,
    /// Path to the copied test executable used for every case.
    executable: PathBuf,
    /// Rust provider with a real import of HELPER.
    provider: PathBuf,
    /// Genuine helper and its import library live here.
    genuine: PathBuf,
    /// Same-basename alternative helper lives here.
    alternative: PathBuf,
}

/// All per-child search directories and durable observations are disjoint.
struct Layout {
    /// Unique case directory, containing all observations and process output.
    root: PathBuf,
    /// CWD is never the provider, executable, PATH or user directory.
    cwd: PathBuf,
    /// Sole PATH entry given to the child.
    path_directory: PathBuf,
    /// Explicit user-added directory, enabled only by the corresponding child case.
    user_directory: PathBuf,
    /// Original filesystem spelling supplied to the actual host or SDK operation.
    library: PathBuf,
    /// Durable process-attach observation from the genuine helper.
    genuine_marker: PathBuf,
    /// Durable process-attach observation from the alternative helper.
    alternative_marker: PathBuf,
    /// Descriptor getter observation, containing the selected helper's origin number.
    getter_marker: PathBuf,
}

/// Completed child with file-backed output, so a full pipe cannot deadlock waiting.
struct ChildOutput {
    /// Exact platform exit status; mandatory failure compares it with AbortControl.
    status: ExitStatus,
    /// Harness and case admission evidence.
    stdout: String,
    /// Loader diagnostics and unexpected panic information.
    stderr: String,
}

/// Builds once, exercises the controls first, then checks each consumer sequentially.
pub(super) fn exercise() {
    let fixtures = Fixtures::build();
    let mut abort_status = None;
    for case in Case::ALL {
        let layout = fixtures.stage(case);
        let output = fixtures.run(case, &layout);
        let details = format!("{case:?}: {:?}\n{}\n{}", output.status, output.stdout, output.stderr);
        assert!(output.stdout.contains(&format!("NC_CASE_ENTERED:{case:?}")), "{details}");
        if case == Case::AbortControl {
            assert!(!output.status.success(), "{details}");
            assert_ne!(output.status.code(), Some(RETURNED), "{details}");
            assert_ne!(output.status.code(), Some(OBSERVER_FAILED), "{details}");
            abort_status = Some(output.status);
        } else if case == Case::ObserverFailure {
            assert_eq!(output.status.code(), Some(OBSERVER_FAILED), "{details}");
        } else if case == Case::LiveMissing {
            assert_eq!(Some(output.status), abort_status, "{details}");
            assert!(output.stdout.contains("NC_LOAD_ENTERED"), "{details}");
            assert!(!output.stdout.contains("NC_LOAD_RETURNED"), "{details}");
            assert!(output.stderr.contains("fatal neocogi-com plugin load failure:"), "{details}");
            assert!(output.stderr.contains("OpenLibrary"), "{details}");
        } else {
            assert!(output.status.success(), "{details}");
            assert!(output.stdout.contains(&format!("NC_CASE_COMPLETED:{case:?}")), "{details}");
            assert!(output.stdout.contains("test result: ok. 1 passed; 0 failed;"), "{details}");
        }
        let genuine = case.opens_provider() || case == Case::ObserveGenuine;
        let alternative = case == Case::ObserveAlternative;
        assert_eq!(layout.genuine_marker.exists(), genuine, "genuine initializer: {details}");
        assert_eq!(layout.alternative_marker.exists(), alternative, "alternative initializer: {details}");
        assert_eq!(layout.getter_marker.exists(), case.opens_provider(), "getter: {details}");
        if case.opens_provider() {
            assert_eq!(fs::read_to_string(&layout.getter_marker).unwrap(), "11", "{details}");
        }
        // The executable-directory copy is row-specific. Children have exited,
        // so Windows permits removal before the next row's preconditions apply.
        if case == Case::OwnedMissingExtraDirectories {
            fs::remove_file(fixtures.host.join(HELPER)).unwrap();
        }
    }
}

impl Fixtures {
    /// Compiles two minimal C helpers and one independent ABI-core-only provider.
    fn build() -> Self {
        let owner = tempfile::tempdir().unwrap();
        let root = owner.path();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("integration-tests/fixtures/windows-loader-policy");
        let genuine = root.join("genuine");
        let alternative = root.join("alternative");
        for (directory, origin, marker) in [(&genuine, "11", "NC_COM_POLICY_GENUINE"), (&alternative, "22", "NC_COM_POLICY_ALTERNATIVE")] {
            fs::create_dir(directory).unwrap();
            let compiler = cc::Build::new()
                .target(env!("NC_COM_TEST_TARGET"))
                .host(env!("NC_COM_TEST_HOST"))
                .opt_level(0)
                .debug(false)
                .out_dir(directory)
                .cargo_metadata(false)
                .get_compiler();
            let mut command = compiler.to_command();
            if compiler.is_like_msvc() {
                command
                    .arg("/LD")
                    .arg(format!("/DNC_POLICY_ORIGIN={origin}"))
                    .arg(format!("/DNC_POLICY_MARKER=L\"{marker}\""))
                    .arg(source.join("helper.c"))
                    .arg("/link")
                    .arg(format!("/OUT:{}", directory.join(HELPER).display()))
                    .arg(format!("/IMPLIB:{}", directory.join("nc_policy_helper_83d97.lib").display()))
                    .arg("kernel32.lib");
            } else {
                command
                    .arg("-shared")
                    .arg(format!("-DNC_POLICY_ORIGIN={origin}"))
                    .arg(format!("-DNC_POLICY_MARKER=L\"{marker}\""))
                    .arg(source.join("helper.c"))
                    .arg("-o")
                    .arg(directory.join(HELPER))
                    .arg(format!("-Wl,--out-implib,{}", directory.join("libnc_policy_helper_83d97.dll.a").display()))
                    .arg("-lkernel32");
            }
            command.current_dir(directory);
            checked(&mut command);
        }
        let target = root.join("provider-target");
        checked(
            Command::new(env!("CARGO"))
                .args(["build", "--offline", "--locked", "--manifest-path"])
                .arg(source.join("Cargo.toml"))
                .arg("--target-dir")
                .arg(&target)
                .arg("--target")
                .arg(env!("NC_COM_TEST_TARGET"))
                .env("NC_COM_POLICY_IMPORT_DIR", &genuine),
        );
        let provider = target.join(env!("NC_COM_TEST_TARGET")).join("debug/nc_windows_loader_policy_fixture.dll");
        assert!(provider.is_file(), "provider fixture was not built: {}", provider.display());
        let host = root.join("host");
        fs::create_dir(&host).unwrap();
        let executable = host.join("policy-tests.exe");
        fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
        Self {
            owner,
            host,
            executable,
            provider,
            genuine,
            alternative,
        }
    }

    /// Creates the real preconditions for one row, without changing parent globals.
    fn stage(&self, case: Case) -> Layout {
        let root = self.owner.path().join(format!("{case:?}"));
        let cwd = root.join("cwd");
        let provider_directory = root.join("可信插件");
        let path_directory = root.join("path");
        let user_directory = root.join("user");
        for directory in [&cwd, &provider_directory, &path_directory, &user_directory] {
            fs::create_dir_all(directory).unwrap();
        }
        let provider = provider_directory.join(PROVIDER);
        fs::copy(&self.provider, &provider).unwrap();
        if case.opens_provider() || case == Case::OwnedNul {
            fs::copy(self.genuine.join(HELPER), provider_directory.join(HELPER)).unwrap();
        }
        match case {
            Case::OwnedConflict | Case::OwnedMissingCwd | Case::CallbackMissing | Case::LiveMissing => {
                fs::copy(self.alternative.join(HELPER), cwd.join(HELPER)).unwrap();
            }
            Case::OwnedMissingExtraDirectories => {
                fs::copy(self.alternative.join(HELPER), self.host.join(HELPER)).unwrap();
                fs::copy(self.alternative.join(HELPER), user_directory.join(HELPER)).unwrap();
            }
            Case::OwnedMissingPath => {
                fs::copy(self.alternative.join(HELPER), path_directory.join(HELPER)).unwrap();
            }
            Case::OwnedBarePath => {
                fs::copy(&self.provider, path_directory.join(PROVIDER)).unwrap();
                fs::copy(self.alternative.join(HELPER), path_directory.join(HELPER)).unwrap();
            }
            _ => {}
        }
        let library = match case {
            Case::ObserveGenuine | Case::ObserverFailure => self.genuine.join(HELPER),
            Case::ObserveAlternative => self.alternative.join(HELPER),
            Case::OwnedBarePath => PathBuf::from(PROVIDER),
            Case::OwnedMissingCwd | Case::OwnedRelativeUnicode | Case::LiveSuccess => Path::new("..").join("可信插件").join(PROVIDER),
            Case::OwnedMissingImage => PathBuf::from("missing.dll"),
            // Interior NUL is created in the child; environment strings cannot contain it.
            Case::OwnedEmpty => PathBuf::new(),
            _ => provider,
        };
        Layout {
            cwd,
            path_directory,
            user_directory,
            library,
            genuine_marker: if case == Case::ObserverFailure {
                root.join("absent/marker")
            } else {
                root.join("genuine.marker")
            },
            alternative_marker: root.join("alternative.marker"),
            getter_marker: root.join("getter.marker"),
            root,
        }
    }

    /// Waits with a deadline; a timeout is reaped and fails rather than passing a negative row.
    fn run(&self, case: Case, layout: &Layout) -> ChildOutput {
        let stdout = layout.root.join("stdout");
        let stderr = layout.root.join("stderr");
        let mut child = Command::new(&self.executable)
            .args(["--exact", "windows_loader_child", "--nocapture"])
            .current_dir(&layout.cwd)
            .env("NC_COM_POLICY_CASE", format!("{case:?}"))
            .env("NC_COM_POLICY_LIBRARY", &layout.library)
            .env("NC_COM_POLICY_USER", &layout.user_directory)
            .env("NC_COM_POLICY_GENUINE", &layout.genuine_marker)
            .env("NC_COM_POLICY_ALTERNATIVE", &layout.alternative_marker)
            .env("NC_COM_POLICY_GETTER", &layout.getter_marker)
            .env("PATH", &layout.path_directory)
            .stdout(Stdio::from(fs::File::create(&stdout).unwrap()))
            .stderr(Stdio::from(fs::File::create(&stderr).unwrap()))
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "{case:?} timed out\n{}\n{}",
                    fs::read_to_string(stdout).unwrap(),
                    fs::read_to_string(stderr).unwrap()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        ChildOutput {
            status,
            stdout: fs::read_to_string(stdout).unwrap(),
            stderr: fs::read_to_string(stderr).unwrap(),
        }
    }
}

/// Executes a compiler with explicit diagnostics on failure, never silently skipping fixtures.
fn checked(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// Isolates test-only process policy changes from the parent and production code.
#[link(name = "kernel32")]
unsafe extern "system" {
    /// Registers a task-owned test directory; the cookie must be non-null.
    fn AddDllDirectory(directory: *const u16) -> *mut c_void;
    /// Suppresses interactive OS fault dialogs so the bounded parent can reap children.
    fn SetErrorMode(mode: u32) -> u32;
}

/// Emits and flushes admission evidence before any operation that may terminate.
fn sentinel(value: &str) {
    println!("{value}");
    std::io::stdout().flush().unwrap();
}

/// Runs exactly one finite case through the real owned, callback or live-load boundary.
pub(super) fn child() {
    let Some(name) = std::env::var_os("NC_COM_POLICY_CASE") else {
        return;
    };
    let case = Case::ALL
        .into_iter()
        .find(|case| OsStr::new(&format!("{case:?}")) == name)
        .expect("unknown loader case");
    // SAFETY: changes only this isolated child, before loading any fixture.
    unsafe { SetErrorMode(0x0001 | 0x0002 | 0x8000) };
    sentinel(&format!("NC_CASE_ENTERED:{case:?}"));
    if case == Case::AbortControl {
        std::process::abort();
    }
    if case == Case::OwnedMissingExtraDirectories {
        let mut directory: Vec<_> = std::env::var_os("NC_COM_POLICY_USER").unwrap().encode_wide().collect();
        directory.push(0);
        // SAFETY: terminated absolute native string is live for this call.
        assert!(
            !unsafe { AddDllDirectory(directory.as_ptr()) }.is_null(),
            "test user directory was not registered"
        );
    }
    let mut path = PathBuf::from(std::env::var_os("NC_COM_POLICY_LIBRARY").unwrap());
    if case == Case::OwnedNul {
        let mut native: OsString = path.into_os_string();
        native.push("\0suffix");
        path = native.into();
    }
    match case {
        Case::AbortControl => unreachable!(),
        Case::CallbackSuccess | Case::CallbackMissing => {
            let runtime = runtime();
            let mut calls = 0;
            // SAFETY: independently built trusted provider. The visitor does not panic.
            let result = unsafe {
                runtime.with_plugin_library_descriptor(&path, |description| {
                    calls += 1;
                    Ok(description.plugin_id)
                })
            };
            if case == Case::CallbackSuccess {
                assert_eq!(result.unwrap(), guid_from_qualified_name("nc.tests.windows_policy"));
                assert_eq!(calls, 1);
            } else {
                assert_eq!(result, Err(NC_E_LOAD_FAILED));
                assert_eq!(calls, 0);
            }
        }
        Case::LiveSuccess | Case::LiveMissing => {
            sentinel("NC_LOAD_ENTERED");
            // SAFETY: the parent built the complete trusted provider and helper fixture.
            let id = unsafe { runtime().load_plugin_library(&path) };
            sentinel("NC_LOAD_RETURNED");
            if case == Case::LiveMissing {
                std::process::exit(RETURNED);
            }
            assert_eq!(id, guid_from_qualified_name("nc.tests.windows_policy"));
            assert_eq!(runtime().plugin_info(id).unwrap().unwrap().origin, PluginOrigin::Dynamic { path });
        }
        _ => {
            // SAFETY: the parent built these trusted DLLs. Malformed spellings must
            // be rejected before opening; successful helper opens intentionally lack a descriptor.
            let result = unsafe { inspect_plugin_library(&path) };
            if case.opens_provider() {
                assert_eq!(result.unwrap().plugin_id, guid_from_qualified_name("nc.tests.windows_policy"));
            } else {
                let error = result.unwrap_err();
                let expected = if matches!(case, Case::ObserveGenuine | Case::ObserveAlternative) {
                    PluginLoadErrorKind::FindSymbol
                } else {
                    PluginLoadErrorKind::OpenLibrary
                };
                assert_eq!(error.kind(), expected, "{error}");
                assert_eq!(error.path(), path);
                if case == Case::OwnedNul {
                    assert!(error.message().contains("interior NUL"));
                }
            }
        }
    }
    sentinel(&format!("NC_CASE_COMPLETED:{case:?}"));
}

/// Admits only this final executable's process-lived host tables.
fn runtime() -> neocogi_com::Runtime {
    // SAFETY: this explicitly linked implementation owns the complete static graph.
    unsafe { neocogi_com::Runtime::__from_host_api(neocogi_com_host::initialize_process()) }.unwrap()
}
