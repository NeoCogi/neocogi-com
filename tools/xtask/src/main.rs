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
    // Locating the compiled-in checkout is purely lexical. Compatibility owns
    // filesystem validation after it invalidates the caller's old receipt.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("tools directory").parent().expect("checkout directory").to_path_buf();
    let runner = Runner::new(root);
    let mut args = std::env::args_os().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    let remaining: Vec<_> = args.collect();
    match command.to_str() {
        Some("compatibility") => compatibility::run(&runner, compatibility::Options::parse(&remaining)?),
        Some("quality" | "msrv" | "packages" | "markdown" | "workspace" | "abi-32") if remaining.is_empty() => match command.to_str().unwrap() {
            "msrv" => qualification::msrv(&runner),
            "markdown" => markdown::check(&runner.root),
            task => {
                if task == "quality" {
                    // Check links against the actual checkout, including its
                    // nightly Cargo configuration omitted from SDK snapshots.
                    markdown::check(&runner.root)?;
                }
                // SDK qualification must not inherit the development workspace's
                // nightly-only Cargo configuration. Build outputs stay on disk.
                let (owner, fixture) = process::sources(&runner)?;
                let result = match task {
                    "quality" => qualification::quality(&fixture),
                    "packages" => packages::run(&fixture),
                    "workspace" => qualification::workspace(&fixture),
                    "abi-32" => qualification::abi_32(&fixture),
                    _ => unreachable!(),
                };
                process::retire(owner, result)
            }
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
