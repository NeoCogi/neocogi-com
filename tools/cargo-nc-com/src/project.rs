//! Discovers configuration through Cargo's stable metadata protocol.

use std::{
    fs,
    path::{Path, PathBuf},
};
use serde::{Deserialize, Serialize};
use crate::{
    Result,
    cli::Options,
    config::{Assembly, Configuration},
};

/// Resolved application and paths printed by `plan` and consumed by the builder.
#[derive(Debug, Serialize)]
pub(crate) struct Project {
    /// Selected assembly's validated output identifier.
    pub name: String,
    /// Manifest controlling the application's dependency graph and profiles.
    pub manifest: PathBuf,
    /// Directory against which authored resource and component paths are resolved.
    pub base: PathBuf,
    /// Cargo's effective target directory, including CLI/config/environment overrides.
    pub target_dir: PathBuf,
    /// Workspace package metadata needed to select local components accurately.
    pub packages: Vec<Package>,
    /// Fully selected assembly after applying the requested variants.
    pub assembly: Assembly,
}

/// Stable subset of Cargo metadata needed to locate workspace-owned configuration.
#[derive(Deserialize)]
struct Metadata {
    /// Workspace member packages; dependency downloading is unnecessary for planning.
    packages: Vec<Package>,
    /// Canonical workspace root, independent of the caller's current directory.
    workspace_root: PathBuf,
    /// Cargo's configured cache directory before a tool CLI override.
    target_directory: PathBuf,
    /// Configuration owned by a virtual or ordinary workspace root.
    metadata: Option<MetadataSection>,
}

/// Typed metadata namespace that ignores unrelated tools' metadata tables.
#[derive(Debug, Default, Deserialize, Serialize)]
struct MetadataSection {
    /// Our namespace; other manifest metadata remains Cargo's concern.
    #[serde(rename = "nc-com", skip_serializing)]
    nc_com: Option<Configuration>,
}

/// Workspace package identity and targets supplied by Cargo, not reconstructed paths.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Package {
    /// Human-readable Cargo package name used in configuration.
    pub name: String,
    /// Exact Cargo package ID used for build selection and artifact matching.
    pub id: String,
    /// Canonical package manifest path.
    pub manifest_path: PathBuf,
    /// Binary/library targets declared or inferred by Cargo.
    pub targets: Vec<Target>,
    /// Optional package-owned assembly definition.
    #[serde(default, skip_serializing)]
    metadata: Option<MetadataSection>,
}

/// Minimal target information required to infer or validate an executable name.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Target {
    /// Cargo target name, potentially different from its package name.
    pub name: String,
    /// Cargo target kinds, such as `bin`, `rlib`, or `cdylib`.
    pub kind: Vec<String>,
}

impl Project {
    /// Reads standalone config or package/workspace metadata with deterministic precedence.
    pub fn load(options: &Options) -> Result<Self> {
        let mut command = options.cargo();
        command.args(["metadata", "--format-version", "1", "--no-deps"]);
        if let Some(manifest) = &options.manifest {
            command.arg("--manifest-path").arg(manifest);
        }
        options.policy(&mut command);
        let output = command.output()?;
        if !output.status.success() {
            return Err(format!("cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        let mut metadata: Metadata = serde_json::from_slice(&output.stdout)?;
        // Locate the manifest Cargo would select, preserving package-local configuration
        // even when invoked from a source subdirectory rather than the package root.
        let mut locate = options.cargo();
        locate.args(["locate-project", "--message-format", "plain"]);
        if let Some(manifest) = &options.manifest {
            locate.arg("--manifest-path").arg(manifest);
        }
        let located = locate.output()?;
        if !located.status.success() {
            return Err(String::from_utf8_lossy(&located.stderr).into_owned().into());
        }
        let selected_manifest = PathBuf::from(String::from_utf8(located.stdout)?.trim()).canonicalize()?;
        let owner = metadata.packages.iter().position(|package| {
            options
                .package
                .as_ref()
                .map_or(package.manifest_path == selected_manifest, |name| package.name == *name)
        });
        if options.package.is_some() && owner.is_none() {
            return Err("--package does not name a workspace package".into());
        }
        let (configuration, base) = if let Some(path) = &options.config {
            let path = path.canonicalize()?;
            let text = fs::read_to_string(&path)?;
            (
                toml::from_str::<Configuration>(&text).map_err(|error| format!("{}: {error}", path.display()))?,
                parent(&path)?.to_owned(),
            )
        } else if let Some(configuration) = owner.and_then(|index| metadata.packages[index].metadata.as_mut()?.nc_com.take()) {
            (configuration, parent(&metadata.packages[owner.unwrap()].manifest_path)?.to_owned())
        } else {
            (
                metadata
                    .metadata
                    .and_then(|section| section.nc_com)
                    .ok_or("no nc-com configuration: add [package.metadata.nc-com], [workspace.metadata.nc-com], or --config PATH")?,
                metadata.workspace_root.clone(),
            )
        };
        let (name, mut assembly) = configuration.select(options.assembly.as_deref())?;
        assembly.configure(&options.variants)?;
        if let Some(package) = &options.package {
            assembly.application.package = Some(package.clone());
        }
        if assembly.application.package.is_none() {
            assembly.application.package = owner.map(|index| metadata.packages[index].name.clone());
        }
        let app = metadata
            .packages
            .iter()
            .find(|package| Some(&package.name) == assembly.application.package.as_ref())
            .ok_or("application.package must select a workspace package (or use --package)")?;
        let binaries: Vec<_> = app.targets.iter().filter(|target| target.kind.iter().any(|kind| kind == "bin")).collect();
        if assembly.application.bin.is_none() && binaries.len() == 1 {
            assembly.application.bin = Some(binaries[0].name.clone());
        }
        if !binaries.iter().any(|target| Some(&target.name) == assembly.application.bin.as_ref()) {
            return Err(format!("application.bin must select one binary in {}", app.name).into());
        }
        let target_dir = match &options.target_dir {
            Some(path) if path.is_absolute() => path.clone(),
            Some(path) => std::env::current_dir()?.join(path),
            None => metadata.target_directory,
        };
        Ok(Self {
            name,
            manifest: metadata.workspace_root.join("Cargo.toml"),
            base,
            target_dir,
            packages: metadata.packages,
            assembly,
        })
    }
}

/// Returns a file's containing directory or a contextual error for malformed paths.
fn parent(path: &Path) -> Result<&Path> {
    path.parent().ok_or_else(|| format!("{} has no parent directory", path.display()).into())
}
