//! Cargo subcommand for reproducible builds and executable-relative COM assemblies.
//!
//! Configuration describes the application and its independently built shared
//! components. Cargo still resolves and compiles Rust packages; this executable
//! owns the ordering, artifact discovery, descriptor validation, and packaging.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::missing_docs_in_private_items)]

mod assembly;
mod build;
mod cli;
mod config;
mod project;

use std::{error::Error, process::ExitCode};

/// Fallible tool operations retain their concrete error messages and source chains.
/// This error boundary uses the `Error` contract, never untyped value downcasts.
type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Reports a contextual failure once and returns a nonzero status to Cargo.
fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("nc-com: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Parses both Cargo's subcommand invocation and direct executable invocations.
fn execute() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1).peekable();
    // Inspection runs in a short-lived child because the runtime deliberately pins
    // loaded modules until process exit; Windows then permits staging renames too.
    if arguments.peek().is_some_and(|argument| argument == "__inspect") {
        arguments.next();
        return assembly::inspect(&arguments.collect::<Vec<_>>());
    }
    let Some(options) = cli::Options::parse(arguments)? else {
        return Ok(());
    };
    let project = project::Project::load(&options)?;
    // A plan resolves configuration without compiling or inspecting component code.
    match options.command {
        cli::Command::Plan => println!("{}", serde_json::to_string_pretty(&project)?),
        cli::Command::Build | cli::Command::Run => {
            let builder = build::Builder::new(&project, &options)?;
            let executable = assembly::build(&builder)?;
            if options.command == cli::Command::Run {
                let status = std::process::Command::new(executable).args(&options.arguments).status()?;
                // Preserve the application's exit code for scripts and CI callers.
                std::process::exit(status.code().unwrap_or(1));
            }
        }
    }
    Ok(())
}
