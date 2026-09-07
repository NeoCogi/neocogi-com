//! Descriptor-validated, transactional deployment of executable-relative assemblies.
//!
//! Compilation and inspection happen before replacing a previous output directory.
//! The ownership marker and file lock apply only to this tool's managed outputs;
//! authored directories and foreign files are never used as replacement roots.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use crate::{
    Result,
    build::{Artifact, Builder, write_changed},
};

/// Marker filename reserved inside directories fully managed by this command.
const MARKER: &str = ".nc-com-assembly";

/// Builds libraries in dependency order, validates descriptors, and installs the bundle.
/// The returned executable path is suitable for launching from any working directory.
pub(crate) fn build(builder: &Builder<'_>) -> Result<PathBuf> {
    fs::create_dir_all(&builder.state)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(builder.state.join("assembly.lock"))?;
    // OS locks are released on drop or process death, unlike sentinel-file locks.
    // Serialize writers of this assembly's generated manifests and output directory.
    lock.lock()?;
    let snapshots = tempfile::Builder::new().prefix(".artifacts-").tempdir_in(&builder.state)?;
    let runtime = snapshot(
        builder.component(&builder.project.assembly.runtime.spec(), None)?,
        &snapshots.path().join("runtime"),
    )?;
    let link_dir = builder.destination.parent().unwrap().join(format!("link-{}", builder.options.profile));
    fs::create_dir_all(&link_dir)?;
    for path in runtime.native_files().filter(|path| path.is_file()) {
        let destination = link_dir.join(filename(path)?);
        write_changed(&destination, &fs::read(path)?)?;
    }
    for (source, alias) in runtime.aliases()? {
        write_changed(&link_dir.join(alias), &fs::read(source)?)?;
    }
    // Supplying the same runtime search path to every subsequent phase also handles
    // applications whose ordinary dependencies unify nc-com/runtime into plugins.
    let (plugins, application) = builder.application(&link_dir)?;
    install(builder, &runtime, &plugins, &application)
}

/// Retains a component before later Cargo invocations can replace shared output names
/// with another feature combination. Only native files enter the private snapshot.
pub(crate) fn snapshot(artifact: Artifact, directory: &Path) -> Result<Artifact> {
    fs::create_dir_all(directory)?;
    let mut files = Vec::new();
    for source in artifact.native_files() {
        let destination = directory.join(filename(source)?);
        copy_tree(source, &destination)?;
        files.push(destination);
    }
    Ok(Artifact {
        target_name: artifact.target_name,
        files,
        executable: None,
        storage: None,
    })
}

/// Prepares a private staging tree, then swaps only an output owned by this assembly.
fn install(builder: &Builder<'_>, runtime: &Artifact, plugins: &[Artifact], application: &Artifact) -> Result<PathBuf> {
    let parent = builder.destination.parent().ok_or("assembly output has no parent")?;
    fs::create_dir_all(parent)?;
    let pending = tempfile::Builder::new().prefix(".nc-com-pending-").tempdir_in(parent)?;
    let owner = format!("cargo-nc-com assembly v1\n{}\n{}\n", builder.project.manifest.display(), builder.project.name);
    write_new(&pending.path().join(MARKER), owner.as_bytes())?;
    for artifact in std::iter::once(runtime).chain(plugins) {
        for path in artifact.native_files() {
            copy_tree(path, &pending.path().join(filename(path)?))?;
        }
    }
    for (source, alias) in runtime.aliases()? {
        let original = pending.path().join(filename(&source)?);
        let alias = pending.path().join(alias);
        // Hard links avoid duplicating runtime storage; filesystems without hard-link
        // support fall back to an exclusive copy with the same collision checks.
        if fs::hard_link(&original, &alias).is_err() {
            copy_file(&original, &alias)?;
        }
    }
    let executable = application.executable.as_ref().ok_or("Cargo did not report an application executable")?;
    let executable_name = filename(executable)?;
    copy_file(executable, &pending.path().join(executable_name))?;
    // Cargo may report separate PDB/dSYM/DWP debug data. Preserve reported symbol
    // files and bundles without guessing filenames in Cargo's output directory.
    for symbols in application
        .files
        .iter()
        .filter(|path| matches!(path.extension().and_then(|extension| extension.to_str()), Some("pdb" | "dSYM" | "dwp")))
    {
        copy_tree(symbols, &pending.path().join(filename(symbols)?))?;
    }
    // Inspect staged libraries in a child process. Its exit releases the runtime's
    // pinned modules before replacement, which is essential for Windows file locks.
    let mut inspector = std::process::Command::new(std::env::current_exe()?);
    inspector.arg("__inspect").arg(pending.path().join("plugins.txt"));
    for plugin in plugins {
        inspector.arg(pending.path().join(filename(plugin.library()?)?));
    }
    if !inspector.status()?.success() {
        return Err("plugin descriptor inspection failed; previous assembly preserved".into());
    }
    for asset in &builder.project.assembly.assets {
        copy_tree(&builder.project.base.join(&asset.from), &pending.path().join(&asset.to))?;
    }
    // A resource collision, failed descriptor check, or missing file above leaves
    // the previous output untouched. Retain its directory until the rename succeeds.
    let backup = tempfile::Builder::new().prefix(".nc-com-backup-").tempdir_in(parent)?;
    let previous = backup.path().join("previous");
    if builder.destination.try_exists()? {
        if fs::symlink_metadata(&builder.destination)?.file_type().is_symlink()
            || !fs::read(builder.destination.join(MARKER)).is_ok_and(|marker| marker == owner.as_bytes())
        {
            return Err(format!("refusing to replace unowned output {}", builder.destination.display()).into());
        }
        fs::rename(&builder.destination, &previous)?;
    }
    if let Err(error) = fs::rename(pending.path(), &builder.destination) {
        if previous.exists()
            && let Err(restore) = fs::rename(&previous, &builder.destination)
        {
            // Preserve the only previous copy even if an external filesystem change
            // makes rollback impossible; TempDir must not delete this backup.
            let retained = backup.keep();
            return Err(format!(
                "assembly rename failed: {error}; rollback failed: {restore}; previous output retained in {}",
                retained.display()
            )
            .into());
        }
        return Err(error.into());
    }
    // TempDir drops remove the backup after a successful replacement; the renamed
    // pending directory is now owned by the tool and survives its former guard.
    let path = builder.destination.join(executable_name);
    println!("Staged {}", builder.destination.display());
    Ok(path)
}

/// Private child-process entry point that writes a dependency-ordered manifest.
/// The first argument is an exclusive output file; remaining arguments are trusted
/// shared libraries selected by the parent. Process exit releases all pinned code.
pub(crate) fn inspect(arguments: &[std::ffi::OsString]) -> Result<()> {
    let (output, libraries) = arguments.split_first().ok_or("inspector requires an output path")?;
    let mut inspections = Vec::new();
    for library in libraries {
        // SAFETY: The assembly command explicitly selected this native component.
        // Descriptor validation and owned metadata copying are provided by the runtime.
        inspections.push(unsafe { nc_com_shared_runtime::inspect_plugin_library(Path::new(library))? });
    }
    let mut manifest = String::new();
    for id in nc_com_shared_runtime::plan_plugin_load_order(&inspections)? {
        let index = inspections
            .iter()
            .position(|plugin| plugin.plugin_id == id)
            .expect("planner returns an inspected identity");
        let name = filename(Path::new(&libraries[index]))?
            .to_str()
            .ok_or("plugin filename must be UTF-8 for plugins.txt")?;
        manifest.push_str(name);
        manifest.push('\n');
    }
    write_new(Path::new(output), manifest.as_bytes())
}

/// Extracts one filename for staging without accepting a path traversal component.
fn filename(path: &Path) -> Result<&std::ffi::OsStr> {
    path.file_name().ok_or_else(|| format!("artifact has no filename: {}", path.display()).into())
}

/// Writes tool metadata exclusively, detecting collisions with configured resources.
fn write_new(destination: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(destination)?;
    file.write_all(bytes)?;
    Ok(())
}

/// Copies one regular file without overwriting another artifact or following links.
fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.is_file() {
        return Err(format!("only regular asset/artifact files are supported: {}", source.display()).into());
    }
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| format!("cannot stage {}: {error}", destination.display()))?;
    std::io::copy(&mut input, &mut output)?;
    // Executable permissions are essential when launching a copied binary on Unix.
    fs::set_permissions(destination, metadata.permissions())?;
    Ok(())
}

/// Recursively copies authored resources, rejecting symlinks and filename collisions.
fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source).map_err(|error| format!("asset {}: {error}", source.display()))?;
    if metadata.is_dir() {
        fs::create_dir_all(destination)?;
        let mut entries = fs::read_dir(source)?.collect::<std::io::Result<Vec<_>>>()?;
        // Deterministic traversal makes collisions and diagnostics reproducible.
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        copy_file(source, destination)?;
    }
    Ok(())
}
