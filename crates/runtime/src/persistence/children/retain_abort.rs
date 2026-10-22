//! Package-local subprocess checks for strong-only providers; never linked in production.
//! Kept local so this package can test its own C boundaries after extraction.

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// Selects a single operation in a fresh test process without modifying parent state.
const CASE_ENV: &str = "NC_COM_RETAIN_ABORT_CASE";
/// Distinguishes an incorrectly returned owner from an abort and a harness failure.
const RETURNED_EXIT_CODE: i32 = 97;

/// Runs a real provider operation and requires exhaustion to terminate at its C boundary.
///
/// The child uses initialized provider storage with an injected full counter.
/// It must abort with the counter diagnostic; an ordinary test panic, missing test,
/// or successful return cannot satisfy this assertion. Separate processes also
/// keep intentionally unbalanced synthetic counts away from ordinary Drop tests.
pub fn assert_aborts(test_name: &str, case: &str, operation: impl FnOnce()) {
    let selection = format!("{test_name}/{case}");
    if let Ok(selected) = std::env::var(CASE_ENV) {
        if selected == selection {
            operation();
            std::process::exit(RETURNED_EXIT_CODE);
        }
        // A child executes just its selected case, never more subprocesses.
        return;
    }

    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command.args(["--exact", test_name, "--nocapture", "--test-threads=1"]);
    if cfg!(panic = "abort") {
        // Nightly abort-mode libtest normally wraps each test in another
        // process and reports its abort as exit 101. This child already owns
        // process isolation: observe the provider's signal directly instead.
        command.args(["-Z", "unstable-options", "--force-run-in-process"]);
    }
    let output = command
        .env(CASE_ENV, selection)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("run retain exhaustion subprocess");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.signal().is_some(), "{case} returned or unwound: {:?}\n{stderr}", output.status);
    assert!(
        stderr.contains("COM reference count exhausted or unowned"),
        "{case} terminated for an unrelated reason:\n{stderr}"
    );
}
