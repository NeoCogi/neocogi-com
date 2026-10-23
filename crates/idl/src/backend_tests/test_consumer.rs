//! Cargo-selected SDK locations for executable macro tests, including extracted archives.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use serde::Deserialize;

/// Minimal Cargo output needed to locate exact package sources, not a new graph.
#[derive(Deserialize)]
struct Metadata {
    /// Packages selected by Cargo for the requested consumer.
    packages: Vec<Package>,
}

/// One package location reported by Cargo; names alone never imply a directory.
#[derive(Deserialize)]
struct Package {
    /// Published package name, independent of its directory spelling.
    name: String,
    /// Exact SDK version selected by the resolver.
    version: String,
    /// Actual manifest in a checkout, registry extraction or patched archive.
    manifest_path: PathBuf,
}

/// Owned scratch consumer and its resolved SDK source locations.
/// No SDK source is copied or invented around a package under test.
pub(super) struct Consumer {
    /// Generated consumer source and lockfile, removed after all children exit.
    pub directory: tempfile::TempDir,
    /// Core package chosen by this consumer's dependency graph.
    pub core: PathBuf,
    /// Facade package chosen by this consumer's dependency graph.
    pub facade: PathBuf,
}

impl Consumer {
    /// Adds the selected implementation only to a final test owner that actually
    /// acquires plugins or exercises generated host startup. Pure codec/interface
    /// consumers keep their existing implementation-free dependency boundary.
    pub fn with_runtime_owner(self) -> Self {
        let workspace = metadata(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
        let implementation = workspace
            .packages
            .iter()
            .find(|package| package.name == "neocogi-com-runtime" && package.version == env!("CARGO_PKG_VERSION"));
        let dependency = match implementation {
            Some(package) => format!("{{path={:?}}}", package.manifest_path.parent().unwrap()),
            None => format!("\"={}\"", env!("CARGO_PKG_VERSION")),
        };
        let manifest = self.directory.path().join("Cargo.toml");
        let mut source = fs::read_to_string(&manifest).unwrap();
        source.push_str(&format!("neocogi-com-runtime={dependency}\n"));
        fs::write(manifest, source).unwrap();
        self
    }

    /// Creates an ordinary consumer, preferring Cargo's existing workspace facade.
    /// Outside a checkout it requests the matching published SDK version. The
    /// archive gate supplies that version through normal Cargo patch configuration.
    pub fn new(name: &str) -> Self {
        let workspace = metadata(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
        let facade = workspace
            .packages
            .iter()
            .find(|package| package.name == "neocogi-com" && package.version == env!("CARGO_PKG_VERSION"));
        let dependency = match facade {
            Some(package) => format!("{{path={:?}}}", package.manifest_path.parent().unwrap()),
            None => format!("\"={}\"", env!("CARGO_PKG_VERSION")),
        };
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("src")).unwrap();
        let manifest = directory.path().join("Cargo.toml");
        fs::write(
            &manifest,
            format!("[package]\nname={name:?}\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={dependency}\n"),
        )
        .unwrap();
        // Cargo metadata requires a target even before the test emits bindings.
        fs::write(directory.path().join("src/lib.rs"), "").unwrap();
        let selected = metadata(&manifest);
        let root = |name: &str| {
            selected
                .packages
                .iter()
                .find(|package| package.name == name && package.version == env!("CARGO_PKG_VERSION"))
                .expect("consumer resolves the matching SDK package")
                .manifest_path
                .parent()
                .unwrap()
                .to_owned()
        };
        Self {
            directory,
            core: root("neocogi-com-core"),
            facade: root("neocogi-com"),
        }
    }
}

/// Queries Cargo's actual selected package locations; failures retain Cargo's diagnostic.
/// Resolution may require target-only packages a host build never downloaded.
/// Cache-only drivers select `CARGO_NET_OFFLINE` explicitly for all their children.
fn metadata(manifest: &Path) -> Metadata {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--manifest-path"])
        .arg(manifest)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}
