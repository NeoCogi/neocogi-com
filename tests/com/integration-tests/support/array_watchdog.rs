//! Native stack regressions run separately so an overflow cannot kill the suite.
use std::{
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

/// Owns kill/reap even when the watchdog assertion unwinds.
struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Re-executes the exact test with a bounded runtime and inherited diagnostics.
pub(super) fn run(test: &str, mode: &str) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", test, "--nocapture"]).env("NC_ARRAY_STACK_CHILD", mode);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: before exec, only the async-signal-safe setrlimit call runs.
        unsafe {
            command.pre_exec(|| {
                let limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
                if libc::setrlimit(libc::RLIMIT_CORE, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    let mut child = Running(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "array stack child {mode} failed: {status}");
            break;
        }
        assert!(Instant::now() < deadline, "array stack child {mode} timed out");
        thread::sleep(Duration::from_millis(10));
    }
}
