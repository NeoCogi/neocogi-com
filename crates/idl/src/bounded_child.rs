//! Small native-Linux regression guard. Only the selected operation child gets
//! rlimits; Cargo, rustc and the parent test process retain their normal resources.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

/// Owns kill/reap even if an assertion or IO operation unwinds before completion.
struct Running(
    /// The only process started by the bounded operation launcher.
    Child,
);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Installs the regression ceiling, allowing joined 8 MiB syntax workers and
/// normal allocator arenas. Installation failure is a test failure, never a skip.
pub(crate) fn cap() {
    let address = libc::rlimit {
        rlim_cur: 512 * 1024 * 1024,
        rlim_max: 512 * 1024 * 1024,
    };
    let core = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: both constants/types are supplied by libc for this Linux target;
    // these initialized records live for the calls and affect only this child.
    unsafe {
        assert_eq!(
            libc::setrlimit(libc::RLIMIT_CORE, &core),
            0,
            "disable core dumps: {}",
            std::io::Error::last_os_error()
        );
        assert_eq!(
            libc::setrlimit(libc::RLIMIT_AS, &address),
            0,
            "cap operation child: {}",
            std::io::Error::last_os_error()
        );
    }
}

/// Executes an exact-selected operation with a wall watchdog. File-backed output
/// prevents a full pipe from blocking the child while the parent polls/waits.
pub(crate) fn run(mut command: Command) -> Output {
    let mut stdout = tempfile::tempfile().unwrap();
    let mut stderr = tempfile::tempfile().unwrap();
    command
        .stdout(Stdio::from(stdout.try_clone().unwrap()))
        .stderr(Stdio::from(stderr.try_clone().unwrap()));
    let mut child = Running(command.spawn().expect("start bounded operation child"));
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "bounded compiler operation timed out");
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: read(&mut stdout),
        stderr: read(&mut stderr),
    }
}

/// Rewinds an already-reaped child's small diagnostic log.
fn read(file: &mut File) -> Vec<u8> {
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    bytes
}
