//! Explicit subprocess policy and temporary source ownership shared by all tasks.
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tempfile::TempDir;

/// Task failures retain IO, Cargo and semantic-admission context for the CLI.
pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// A source root and its explicit inherited child environment; never mutates globals.
#[derive(Clone)]
pub(crate) struct Runner {
    /// Working directory for Cargo and relative fixture inputs; copied source
    /// roots are canonicalized during preparation, after receipt invalidation.
    pub root: PathBuf,
    /// Credentials/tool discovery survive; ambient compiler/loader policy does not.
    pub env: BTreeMap<OsString, OsString>,
}

impl Runner {
    /// Capture an isolated environment and enforce serial nested compilation/tests.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            env: clean_environment(std::env::vars_os()),
        }
    }

    /// Stream stdout into CI while retaining it for Cargo JSON and test admission.
    /// Stderr stays live on the caller's stderr. Exit status always precedes markers.
    pub fn run<I, S>(&self, program: impl AsRef<OsStr>, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(&self.root)
            .env_clear()
            .envs(&self.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        println!("+ {:?} {:?}", command.get_program(), command.get_args().collect::<Vec<_>>());
        let mut child = command.spawn()?;
        let mut output = String::new();
        let read = (|| -> std::io::Result<()> {
            let mut reader = BufReader::new(child.stdout.take().expect("piped stdout"));
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line)? == 0 {
                    break;
                }
                print!("{line}");
                output.push_str(&line);
            }
            Ok(())
        })();
        // Always reap on a decoding/read failure as well as on a nonzero exit.
        if read.is_err() {
            let _ = child.kill();
        }
        let status = child.wait()?;
        read?;
        if !status.success() {
            return Err(format!("command exited {status}: {:?}", command.get_program()).into());
        }
        Ok(output)
    }

    /// Run Cargo using an explicitly selected Rustup toolchain for all children.
    pub fn cargo(&self, toolchain: &str, args: &[&str]) -> Result<String> {
        let mut runner = self.clone();
        runner.env.insert("RUSTUP_TOOLCHAIN".into(), toolchain.into());
        runner.run("rustup", ["run", toolchain, "cargo"].into_iter().chain(args.iter().copied()))
    }

    /// Capture structured discovery output without dumping entire dependency graphs.
    /// Diagnostics remain live on stderr and a nonzero child always fails the task.
    pub fn read<I, S>(&self, program: impl AsRef<OsStr>, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(program);
        command.args(args).current_dir(&self.root).env_clear().envs(&self.env).stderr(Stdio::inherit());
        println!("+ {:?} {:?}", command.get_program(), command.get_args().collect::<Vec<_>>());
        let output = command.output()?;
        if !output.status.success() {
            return Err(format!("command exited {}: {:?}", output.status, command.get_program()).into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    /// Allocate disk-backed task scratch below this checkout, ignoring ambient TMPDIR.
    /// The checkout excludes target from workspace membership so temporary Cargo
    /// projects cannot join it when searching past a copied source workspace.
    pub fn scratch(&self, prefix: &str) -> Result<TempDir> {
        let directory = self.root.join("target/xtask-work");
        fs::create_dir_all(&directory)?;
        Ok(tempfile::Builder::new().prefix(prefix).tempdir_in(directory)?)
    }

    /// Keep nested test-owned temporary sources and targets on the same disk as
    /// their task owner. All three spellings cover Unix and Windows discovery.
    /// Enclosing workspaces must exclude this subtree (tmp directly, target for
    /// outer checkouts) so generated Cargo projects can own their own workspaces.
    pub fn temporary_files(&mut self, directory: PathBuf) -> Result<()> {
        fs::create_dir_all(&directory)?;
        for key in ["TMPDIR", "TMP", "TEMP"] {
            self.env.insert(key.into(), directory.as_os_str().to_owned());
        }
        Ok(())
    }
}

/// Remove inherited build/loader overrides while retaining OS discovery and auth.
/// Case-specific settings are applied to cloned runners after this boundary.
pub(crate) fn clean_environment(values: impl IntoIterator<Item = (OsString, OsString)>) -> BTreeMap<OsString, OsString> {
    let mut env: BTreeMap<_, _> = values
        .into_iter()
        .filter(|(key, _)| {
            let name = key.to_string_lossy().to_ascii_uppercase();
            !name.starts_with("CARGO_PROFILE_")
                && !name.starts_with("CARGO_TARGET_")
                && !name.starts_with("RUSTUP_TOOLCHAIN")
                && !matches!(
                    name.as_str(),
                    "CARGO"
                        | "RUSTC"
                        | "RUSTC_WRAPPER"
                        | "RUSTC_WORKSPACE_WRAPPER"
                        | "RUSTFLAGS"
                        | "CARGO_ENCODED_RUSTFLAGS"
                        | "RUSTDOCFLAGS"
                        | "CARGO_BUILD_TARGET"
                        | "CARGO_BUILD_RUSTFLAGS"
                        | "CARGO_BUILD_RUSTC"
                        | "CARGO_BUILD_RUSTC_WRAPPER"
                        | "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER"
                        | "MIRIFLAGS"
                        | "MIRI_SYSROOT"
                        | "ASAN_OPTIONS"
                        | "LSAN_OPTIONS"
                        | "TSAN_OPTIONS"
                        | "LD_LIBRARY_PATH"
                        | "DYLD_LIBRARY_PATH"
                        | "DYLD_FALLBACK_LIBRARY_PATH"
                        | "CARGO_BUILD_JOBS"
                        | "RUST_TEST_THREADS"
                )
        })
        .collect();
    env.insert("CARGO_BUILD_JOBS".into(), "1".into());
    env.insert("RUST_TEST_THREADS".into(), "1".into());
    env
}

/// Copy regular repository files recursively; targets and private evidence stay out.
/// Source fixtures are build inputs, not copies of the Git repository or reviews.
pub(crate) fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if matches!(entry.file_name().to_str(), Some("target" | ".git" | "__pycache__" | "plans" | "reviews")) {
            continue;
        }
        let target = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err(format!("unsupported source fixture entry: {}", entry.path().display()).into());
        }
    }
    Ok(())
}

/// Own a complete source fixture, keeping compile-time embedded paths alive.
/// Normal completion uses explicit close so cleanup failure prevents a green task.
pub(crate) fn sources(runner: &Runner) -> Result<(TempDir, Runner)> {
    let mut fixture = runner.clone();
    fixture.root = runner
        .root
        .canonicalize()
        .map_err(|error| format!("cannot prepare checkout {}: {error}", runner.root.display()))?;
    let owner = fixture.scratch("sources-")?;
    for relative in [
        "crates",
        "schemas",
        "tests",
        "tools/neocogi-inspect",
        "tools/xtask",
        "examples/com-plugin",
        "examples/com-host",
        "examples/getting-started",
        "examples/schemas",
        "docs",
    ] {
        copy_tree(&fixture.root.join(relative), &owner.path().join(relative))?;
    }
    for relative in ["README.md", "rustfmt.toml", "Cargo.lock", "Cargo.toml"] {
        fs::copy(fixture.root.join(relative), owner.path().join(relative))?;
    }
    fixture.root = owner.path().to_path_buf();
    fixture.env.insert("CARGO_TARGET_DIR".into(), fixture.root.join("target").into_os_string());
    fixture.temporary_files(fixture.root.join("tmp"))?;
    Ok((owner, fixture))
}

/// Require the requested artifact itself to exist and contain bytes.
pub(crate) fn require_file(path: &Path) -> Result<()> {
    if !path.is_file() || path.metadata()?.len() == 0 {
        return Err(format!("missing or empty artifact: {}", path.display()).into());
    }
    Ok(())
}

/// Explicit teardown precedes success, and secondary teardown errors retain context.
pub(crate) fn retire<T>(owner: TempDir, result: Result<T>) -> Result<T> {
    match (result, owner.close()) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(format!("task cleanup failed: {error}").into()),
        (Err(error), Err(cleanup)) => Err(format!("{error}; task cleanup also failed: {cleanup}").into()),
    }
}
