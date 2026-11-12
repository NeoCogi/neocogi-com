//! Complete platform-style deployment builds without changing stable SDK checks.
use std::{ffi::OsString, path::PathBuf};
use serde_json::Value;
use crate::process::{Result, Runner};
use crate::compatibility::words;

/// The same pinned nightly as the platform's immediate-abort/size workflow.
pub(crate) const TOOLCHAIN: &str = "nightly-2026-09-29";

/// Small explicit deployment surface; arbitrary Cargo policy is not inherited.
#[derive(Default)]
struct Options {
    /// Enable the optional custom JSON implementation; NCB is unconditional.
    json: bool,
    /// Build the static throughput example instead of the dynamic shim.
    example: bool,
    /// Machine-readable final artifact selection for tests and measurement tools.
    json_output: bool,
    /// Optional controlled comparison of LLVM's two size optimization policies.
    optimization: Option<String>,
}
impl Options {
    /// Accepts only the supported format, artifact and size-policy choices.
    fn parse(args: &[OsString]) -> Result<Self> {
        let mut result = Self::default();
        let mut args = args.iter();
        while let Some(argument) = args.next() {
            match argument.to_str() {
                Some("--features") if args.next().and_then(|value| value.to_str()) == Some("json") => result.json = true,
                Some("--features=json") => result.json = true,
                Some("--example") => result.example = true,
                Some("--json-output") => result.json_output = true,
                Some("--opt-level") => {
                    let level = args.next().and_then(|value| value.to_str()).ok_or("--opt-level requires s or z")?;
                    if !matches!(level, "s" | "z") {
                        return Err("--opt-level requires s or z".into());
                    }
                    result.optimization = Some(level.to_owned());
                }
                _ => return Err("min-size accepts [--features json] [--example] [--json-output] [--opt-level s|z]".into()),
            }
        }
        Ok(result)
    }
}
/// Rebuilds target std with immediate abort while host macros/build scripts keep
/// ordinary panic/Debug behavior. Only Cargo's reported native artifact is used.
pub(crate) fn run(runner: &Runner, args: &[OsString]) -> Result<()> {
    let options = Options::parse(args)?;
    let mut runner = runner.clone();
    runner.env.insert("RUSTUP_TOOLCHAIN".into(), TOOLCHAIN.into());
    if let Some(level) = options.optimization {
        runner.env.insert("CARGO_PROFILE_MIN_SIZE_OPT_LEVEL".into(), level.into());
    }
    let version = runner.read("rustup", ["run", TOOLCHAIN, "rustc", "-vV"])?;
    let native = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or("nightly compiler did not report its host target")?;
    let package = if options.example {
        "neocogi-com-serialization"
    } else {
        "neocogi-com-serialization-dylib"
    };
    let name = if options.example { "format_costs" } else { "neocogi_com_serialization_dylib" };
    let mut args = words(&["run", TOOLCHAIN, "cargo", "--config"]);
    args.push(runner.root.join(".cargo/min-size.toml").into_os_string());
    args.extend(words(&[
        "build",
        "--locked",
        "--profile",
        "min-size",
        "--target",
        native,
        "-p",
        package,
        "--message-format=json",
    ]));
    if options.json {
        args.extend(words(&["--features", "json"]));
    }
    if options.example {
        args.extend(words(&["--example", "format_costs"]));
    }
    let output = runner.read("rustup", args)?;
    let mut artifacts = Vec::new();
    for line in output.lines() {
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if record["reason"] != "compiler-artifact" || record["target"]["name"] != name {
            continue;
        }
        if options.example {
            if let Some(path) = record["executable"].as_str() {
                artifacts.push(PathBuf::from(path));
            }
        } else if let Some(paths) = record["filenames"].as_array() {
            artifacts.extend(
                paths
                    .iter()
                    .filter_map(Value::as_str)
                    .map(PathBuf::from)
                    .filter(|path| path.extension().is_some_and(|ext| ext == std::env::consts::DLL_EXTENSION)),
            );
        }
    }
    artifacts.sort();
    artifacts.dedup();
    let [artifact] = artifacts.as_slice() else {
        return Err("min-size build did not report exactly one deployment artifact".into());
    };
    let bytes = std::fs::metadata(artifact)?.len();
    if options.json_output {
        println!("{}", serde_json::json!({"artifact":artifact,"bytes":bytes,"toolchain":TOOLCHAIN}));
    } else {
        println!("Built {} ({bytes} bytes)", artifact.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Unsupported codecs and optimization values cannot silently select a different build.
    #[test]
    fn deployment_options_are_explicit() {
        assert!(!Options::parse(&[]).unwrap().json);
        let options = Options::parse(&words(&["--features", "json", "--example", "--opt-level", "z"])).unwrap();
        assert!(options.json && options.example);
        assert_eq!(options.optimization.as_deref(), Some("z"));
        assert!(Options::parse(&words(&["--features", "yaml"])).is_err());
        assert!(Options::parse(&words(&["--opt-level", "3"])).is_err());
    }
}
