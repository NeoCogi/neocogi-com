//! Command-line extraction and comparison of owned plugin inspection artifacts.
//!
//! `dump` and `check` load trusted libraries only long enough to copy descriptor
//! metadata. `diff` never loads code; it strictly validates versioned serialized
//! artifacts and compares their identities, hashes, and derived diagnostics for
//! deterministic CI output.

#![deny(missing_docs)]

use std::path::Path;

use nc_com_runtime::{inspect_plugin_library, plan_plugin_load_order};

use crate::cli::{Command, DiffMode, OutputFormat, USAGE, parse_args};
use crate::compat::check_compatibility;
use crate::diff::diff_inspections;
use crate::plan::{render_plan_json, render_plan_text};
use crate::render::{render_compatibility, render_diff, render_json, render_text};
use crate::schema::{ArtifactValidationError, InspectionArtifactV1};

mod cli;
mod compat;
mod diff;
mod plan;
mod render;
mod schema;

fn main() {
    // Usage errors are command-line mistakes (exit 2). Inspection, I/O, JSON,
    // exact artifact differences, and incompatibilities are operational
    // failures (exit 1).
    match run() {
        Ok(()) => {}
        Err(CliError::Usage(message)) => {
            eprintln!("error: {message}\n\n{USAGE}");
            std::process::exit(2);
        }
        Err(CliError::Inspect(error)) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        Err(CliError::Io(error)) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        Err(CliError::Json(error)) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        Err(CliError::Artifact(error)) => {
            eprintln!("error: invalid inspection artifact: {error}");
            std::process::exit(1);
        }
        Err(CliError::Different(report)) => {
            print!("{report}");
            std::process::exit(1);
        }
        Err(CliError::Incompatible(report)) => {
            print!("{report}");
            std::process::exit(1);
        }
        Err(CliError::PlanInspection(errors)) => {
            for error in errors {
                eprintln!("error: {error}");
            }
            std::process::exit(1);
        }
        Err(CliError::Plan(error)) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), CliError> {
    let command = parse_args(std::env::args_os()).map_err(CliError::Usage)?;
    match command {
        Command::Dump {
            library,
            format,
            output,
        } => {
            // SAFETY: The user explicitly asks the inspector to load this
            // shared library. The runtime reads only nc-com descriptor metadata.
            let inspection =
                unsafe { inspect_plugin_library(&library) }.map_err(CliError::Inspect)?;
            let rendered = match format {
                OutputFormat::Json => render_json(&inspection).map_err(CliError::Json)?,
                OutputFormat::Text => render_text(&inspection),
            };
            if let Some(output) = output {
                std::fs::write(output, rendered).map_err(CliError::Io)?;
            } else {
                print!("{rendered}");
            }
        }
        Command::Check { library } => {
            // SAFETY: The user explicitly asks the inspector to load this
            // shared library. The runtime reads only nc-com descriptor metadata.
            let inspection =
                unsafe { inspect_plugin_library(&library) }.map_err(CliError::Inspect)?;
            println!(
                "{}: ok ({}, {} class{})",
                library.display(),
                inspection.qualified_name,
                inspection.classes.len(),
                if inspection.classes.len() == 1 {
                    ""
                } else {
                    "es"
                }
            );
        }
        Command::Plan {
            libraries,
            format,
            output,
        } => {
            let mut artifacts = Vec::with_capacity(libraries.len());
            let mut errors = Vec::new();
            for library in libraries {
                // SAFETY: Each user-selected trusted artifact is inspected
                // independently and never registered or recursively loaded.
                match unsafe { inspect_plugin_library(&library) } {
                    Ok(inspection) => artifacts.push((library, inspection)),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            if !errors.is_empty() {
                return Err(CliError::PlanInspection(errors));
            }
            let inspections: Vec<_> = artifacts
                .iter()
                .map(|(_, inspection)| inspection.clone())
                .collect();
            let order = plan_plugin_load_order(&inspections).map_err(CliError::Plan)?;
            let rendered = match format {
                OutputFormat::Json => {
                    render_plan_json(&artifacts, &order).map_err(CliError::Json)?
                }
                OutputFormat::Text => render_plan_text(&artifacts, &order),
            };
            if let Some(output) = output {
                std::fs::write(output, rendered).map_err(CliError::Io)?;
            } else {
                print!("{rendered}");
            }
        }
        Command::Diff { old, new, mode } => {
            let old = read_json(&old)?;
            let new = read_json(&new)?;
            match mode {
                DiffMode::Exact => {
                    let differences = diff_inspections(&old, &new);
                    if differences.is_empty() {
                        println!("no differences");
                    } else {
                        return Err(CliError::Different(render_diff(&differences)));
                    }
                }
                DiffMode::Compatible => {
                    let report = check_compatibility(&old, &new);
                    let compatible = report.is_compatible();
                    let rendered = render_compatibility(&report);
                    if compatible {
                        print!("{rendered}");
                    } else {
                        return Err(CliError::Incompatible(rendered));
                    }
                }
            }
        }
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<InspectionArtifactV1, CliError> {
    let data = std::fs::read(path).map_err(CliError::Io)?;
    let artifact: InspectionArtifactV1 = serde_json::from_slice(&data).map_err(CliError::Json)?;
    artifact.validate().map_err(CliError::Artifact)?;
    Ok(artifact)
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Inspect(nc_com_runtime::PluginLoadError),
    Io(std::io::Error),
    Json(serde_json::Error),
    Artifact(ArtifactValidationError),
    Different(String),
    Incompatible(String),
    PlanInspection(Vec<String>),
    Plan(nc_com_runtime::PluginDependencyPlanError),
}
