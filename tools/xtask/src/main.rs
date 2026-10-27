//! Unpublished, cross-platform owner of repository qualification policy.
//! Children run sequentially with one Cargo job and one test thread. A task
//! owns its temporary sources and targets until every child has exited.
mod compatibility;
mod markdown;
mod packages;
mod process;
mod qualification;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use process::{Result, Runner};

/// Print a contextual error and make every failed task fail its calling CI step.
fn main() {
    if let Err(error) = dispatch() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

/// Parse a small fixed command surface before creating sources or starting builds.
fn dispatch() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    let runner = Runner::new(root);
    let mut args = std::env::args_os().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    let remaining: Vec<_> = args.collect();
    match command.to_str() {
        Some("compatibility") => compatibility::run(&runner, compatibility::Options::parse(&remaining)?),
        Some("quality" | "msrv" | "packages" | "markdown" | "workspace" | "abi-32") if remaining.is_empty() => match command.to_str().unwrap() {
            "quality" => qualification::quality(&runner),
            "msrv" => qualification::msrv(&runner),
            "packages" => packages::run(&runner),
            "markdown" => markdown::check(&runner.root),
            "workspace" => qualification::workspace(&runner),
            "abi-32" => qualification::abi_32(&runner),
            _ => unreachable!(),
        },
        Some("help" | "--help" | "-h") if remaining.is_empty() => {
            println!(
                "cargo xtask <quality|msrv|packages|markdown|workspace|abi-32>\ncargo xtask compatibility [--suite linked|cross-version|build-flags|safety|address|thread|sanitizers] [--report PATH]"
            );
            Ok(())
        }
        _ => Err(format!("unknown task or arguments: {command:?} {remaining:?}; run cargo xtask help").into()),
    }
}
