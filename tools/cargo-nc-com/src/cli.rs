//! Strict command-line parsing shared by planning, building, and running assemblies.

use std::{ffi::OsString, path::PathBuf};

use crate::Result;

/// Operation requested by the user; build is the default when omitted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Command {
    /// Resolve configuration and print a machine-readable plan without compilation.
    Plan,
    /// Compile and stage a complete native application.
    Build,
    /// Build the assembly and execute its staged application.
    Run,
}

/// Explicit command-line overrides; omitted values preserve project configuration.
#[derive(Debug)]
pub(crate) struct Options {
    /// Operation to perform after configuration has been validated.
    pub command: Command,
    /// Application/workspace manifest, resolved by Cargo when omitted.
    pub manifest: Option<PathBuf>,
    /// Optional standalone TOML configuration; paths are relative to this file.
    pub config: Option<PathBuf>,
    /// Workspace application package to select or override.
    pub package: Option<String>,
    /// Named assembly in a configuration containing multiple applications.
    pub assembly: Option<String>,
    /// Ordered variant names; later variants may replace earlier feature choices.
    pub variants: Vec<String>,
    /// Rustup toolchain used for every Cargo/rustc subprocess.
    pub toolchain: Option<String>,
    /// Cargo profile; `min-size` enables the bundled Linux nightly preset.
    pub profile: String,
    /// Explicit native target triple; foreign targets cannot be inspected locally.
    pub target: Option<String>,
    /// Cargo compilation cache root, separate from the managed assembly directory.
    pub target_dir: Option<PathBuf>,
    /// Disallow network access in all Cargo subprocesses.
    pub offline: bool,
    /// Require existing, up-to-date application and generated component lockfiles.
    pub locked: bool,
    /// Application arguments following `run --`, preserved as operating-system strings.
    pub arguments: Vec<OsString>,
}

impl Options {
    /// Rejects unknown flags and ambiguous profile choices before invoking Cargo.
    pub fn parse(args: impl Iterator<Item = OsString>) -> Result<Option<Self>> {
        let mut args = args.peekable();
        // Cargo supplies its subcommand name as argv[1]; direct execution does not.
        if args.peek().is_some_and(|arg| arg == "nc-com") {
            args.next();
        }
        let mut options = Self {
            command: Command::Build,
            manifest: None,
            config: None,
            package: None,
            assembly: None,
            variants: Vec::new(),
            toolchain: None,
            profile: "dev".into(),
            target: None,
            target_dir: None,
            offline: false,
            locked: false,
            arguments: Vec::new(),
        };
        let mut command_seen = false;
        let mut profile_seen = false;
        while let Some(arg) = args.next() {
            let flag = arg.to_str().ok_or("option names must be UTF-8")?;
            match flag {
                "-h" | "--help" => {
                    println!("{}", include_str!("help.txt"));
                    return Ok(None);
                }
                "-V" | "--version" => {
                    println!("cargo-nc-com {}", env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                "plan" | "build" | "run" if !command_seen => {
                    command_seen = true;
                    options.command = match flag {
                        "plan" => Command::Plan,
                        "run" => Command::Run,
                        _ => Command::Build,
                    };
                }
                "--manifest-path" => options.manifest = Some(next(&mut args, flag)?.into()),
                "--config" => options.config = Some(next(&mut args, flag)?.into()),
                "--target-dir" => options.target_dir = Some(next(&mut args, flag)?.into()),
                "-p" | "--package" => options.package = Some(next_text(&mut args, flag)?),
                "--assembly" => options.assembly = Some(next_text(&mut args, flag)?),
                "--variant" => options.variants.push(next_text(&mut args, flag)?),
                "--toolchain" => options.toolchain = Some(next_text(&mut args, flag)?),
                "--target" => options.target = Some(next_text(&mut args, flag)?),
                "--release" | "--profile" => {
                    if profile_seen {
                        return Err("specify only one of --release or --profile".into());
                    }
                    profile_seen = true;
                    options.profile = if flag == "--release" { "release".into() } else { next_text(&mut args, flag)? };
                }
                "--offline" => options.offline = true,
                "--locked" => options.locked = true,
                "--frozen" => {
                    options.offline = true;
                    options.locked = true;
                }
                "--" if options.command == Command::Run => {
                    options.arguments.extend(args);
                    break;
                }
                _ => return Err(format!("unknown argument {flag:?}; see cargo nc-com --help").into()),
            }
        }
        crate::config::validate_name(&options.profile)?;
        if options.profile == "min-size" && options.toolchain.is_none() {
            options.toolchain = Some("nightly".into());
        }
        Ok(Some(options))
    }

    /// Appends resolution policies to every Cargo invocation, including metadata.
    pub fn policy(&self, command: &mut std::process::Command) {
        if self.offline {
            command.arg("--offline");
        }
        if self.locked {
            command.arg("--locked");
        }
    }

    /// Uses rustup explicitly when requested, even when the tool itself was installed
    /// with stable Cargo. Otherwise preserves Cargo's selected executable/toolchain.
    pub fn cargo(&self) -> std::process::Command {
        if let Some(toolchain) = &self.toolchain {
            let mut command = std::process::Command::new("rustup");
            command.args(["run", toolchain, "cargo"]);
            command
        } else {
            std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        }
    }
}

/// Takes one required flag value while retaining non-Unicode filesystem paths.
fn next(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<OsString> {
    args.next().ok_or_else(|| format!("{flag} requires a value").into())
}

/// Converts identifiers and toolchain names to text with a specific diagnostic.
fn next_text(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<String> {
    next(args, flag)?.into_string().map_err(|_| format!("{flag} requires UTF-8 text").into())
}
