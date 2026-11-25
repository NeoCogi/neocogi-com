//! Command-line extraction and comparison of owned plugin inspection artifacts.
//!
//! `dump` and `check` copy descriptor metadata from trusted libraries whose
//! mappings remain pinned for the process lifetime. `diff` never loads code; it strictly validates versioned serialized
//! artifacts and compares their identities, ABI signatures, and derived diagnostics for
//! deterministic CI output.

#![deny(missing_docs)]

use std::{
    fs,
    io::{self, Write as _},
    path::Path,
    process::ExitCode,
};

use neocogi_com_host::{inspect_plugin_library, plan_plugin_load_order};

use crate::cli::{Command, DiffMode, OutputFormat, USAGE, parse_args};
use crate::compat::check_compatibility;
use crate::diff::diff_inspections;
use crate::plan::{render_plan_json, render_plan_text};
use crate::render::{render_compatibility, render_diff, render_json, render_text};
use crate::schema::{ArtifactValidationError, InspectionArtifact};

mod cli;
mod compat;
mod diff;
mod plan;
mod render;
mod schema;

fn main() -> ExitCode {
    let error = match run() {
        Ok(status) => return status,
        Err(error) => error,
    };
    // A refused diagnostic leaves the selected usage or operational exit intact.
    let mut stderr = io::stderr().lock();
    let _ = match error {
        CliError::Usage(message) => {
            let _ = write!(stderr, "error: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
        CliError::Inspect(error) => writeln!(stderr, "error: {error}"),
        CliError::Io(error) => writeln!(stderr, "error: {error}"),
        CliError::Json(error) => writeln!(stderr, "error: {error}"),
        CliError::Artifact(error) => writeln!(stderr, "error: invalid inspection artifact: {error}"),
        CliError::PlanInspection(errors) => {
            for error in errors {
                let _ = writeln!(stderr, "error: {error}");
            }
            return ExitCode::FAILURE;
        }
        CliError::Plan(error) => writeln!(stderr, "error: {error}"),
    };
    ExitCode::FAILURE
}

/// Validates, renders and publishes each command before returning its exit status.
fn run() -> Result<ExitCode, CliError> {
    let command = parse_args(std::env::args_os()).map_err(CliError::Usage)?;
    match command {
        Command::Help => publish(None, USAGE)?,
        Command::Dump { library, format, output } => {
            // SAFETY: The user explicitly asks the inspector to load this
            // shared library. The runtime reads only neocogi-com-sdk descriptor metadata.
            let inspection = unsafe { inspect_plugin_library(&library) }.map_err(CliError::Inspect)?;
            let rendered = match format {
                OutputFormat::Json => render_json(&inspection).map_err(CliError::Json)?,
                OutputFormat::Text => render_text(&inspection),
            };
            publish(output.as_deref(), &rendered)?;
        }
        Command::Check { library } => {
            // SAFETY: The user explicitly asks the inspector to load this
            // shared library. The runtime reads only neocogi-com-sdk descriptor metadata.
            let inspection = unsafe { inspect_plugin_library(&library) }.map_err(CliError::Inspect)?;
            let rendered = format!(
                "{}: ok ({}, factory {})\n",
                library.display(),
                inspection.qualified_name,
                inspection.factory.root_factory_id
            );
            publish(None, &rendered)?;
        }
        Command::Plan { libraries, format, output } => {
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
            let inspections: Vec<_> = artifacts.iter().map(|(_, inspection)| inspection.clone()).collect();
            let order = plan_plugin_load_order(&inspections).map_err(CliError::Plan)?;
            let rendered = match format {
                OutputFormat::Json => render_plan_json(&artifacts, &order).map_err(CliError::Json)?,
                OutputFormat::Text => render_plan_text(&artifacts, &order),
            };
            publish(output.as_deref(), &rendered)?;
        }
        Command::Diff { old, new, mode } => {
            let old = read_json(&old)?;
            let new = read_json(&new)?;
            match mode {
                DiffMode::Exact => {
                    let differences = diff_inspections(&old, &new);
                    if differences.is_empty() {
                        publish(None, "no differences\n")?;
                    } else {
                        publish(None, &render_diff(&differences))?;
                        return Ok(ExitCode::FAILURE);
                    }
                }
                DiffMode::Compatible => {
                    let report = check_compatibility(&old, &new);
                    publish(None, &render_compatibility(&report))?;
                    if !report.is_compatible() {
                        return Ok(ExitCode::FAILURE);
                    }
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Writes a complete rendering to the selected destination and checks stdout flush.
fn publish(output: Option<&Path>, rendered: &str) -> Result<(), CliError> {
    match output {
        Some(path) => fs::write(path, rendered),
        None => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(rendered.as_bytes()).and_then(|()| stdout.flush())
        }
    }
    .map_err(CliError::Io)
}

/// Loads and validates one saved inspection before rendering comparisons.
fn read_json(path: &Path) -> Result<InspectionArtifact, CliError> {
    let data = fs::read(path).map_err(CliError::Io)?;
    let artifact: InspectionArtifact = serde_json::from_slice(&data).map_err(CliError::Json)?;
    artifact.validate().map_err(CliError::Artifact)?;
    Ok(artifact)
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Inspect(neocogi_com_host::PluginLoadError),
    Io(io::Error),
    Json(serde_json::Error),
    Artifact(ArtifactValidationError),
    PlanInspection(Vec<String>),
    Plan(neocogi_com_host::PluginDependencyPlanError),
}
