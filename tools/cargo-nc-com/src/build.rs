//! Ordered Cargo subprocesses and artifact discovery for local and registry sources.
//!
//! This module runs inside the installed command, not inside a Cargo build script.
//! The runtime is prepared first, then compatible plugins and the application share
//! one Cargo feature graph. Thin cdylib roots re-export component Rust libraries so
//! Cargo can apply LTO to the deployed output without recompiling their source.

use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use serde::{Deserialize, Serialize};
use crate::{Result, cli::Options, config::ComponentSpec, project::Project};

/// Parameters shared by every compilation in one assembly operation.
pub(crate) struct Builder<'a> {
    /// CLI choices, including toolchain and offline/lockfile policies.
    pub options: &'a Options,
    /// Resolved application, components, assets, and cache root.
    pub project: &'a Project,
    /// Explicit compilation target: host triple or bundled custom-target JSON path.
    target: String,
    /// Encoded rustc arguments, retaining caller arguments without shell evaluation.
    flags: String,
    /// Managed state shared by all builds of this assembly/variant combination.
    pub state: PathBuf,
    /// Final assembly directory, isolated by toolchain, target, and profile.
    pub destination: PathBuf,
}

/// Cargo-produced paths for exactly one requested package and target.
#[derive(Debug, Default)]
pub(crate) struct Artifact {
    /// Logical Rust library target name, before Cargo adds a dependency filename hash.
    pub target_name: String,
    /// All files reported for the selected target, including Windows import libraries.
    pub files: Vec<PathBuf>,
    /// Executable path supplied explicitly by Cargo for an application target.
    pub executable: Option<PathBuf>,
    /// Owns an isolated snapshot until deployment finishes; dropping it removes only
    /// the temporary copy, after all configured libraries have been installed.
    pub storage: Option<tempfile::TempDir>,
}

impl Artifact {
    /// Supplies conventional runtime linker names when registry artifacts are hashed.
    /// Original files are retained too because their platform loader identity may use
    /// the hashed name. Import-library suffixes are preserved for both Windows ABIs.
    pub fn aliases(&self) -> Result<Vec<(PathBuf, String)>> {
        let library = self.library()?;
        let stem = library.file_stem().and_then(|stem| stem.to_str()).ok_or("runtime filename must be UTF-8")?;
        let prefix = if library.extension().is_some_and(|extension| extension == "dll") {
            ""
        } else {
            "lib"
        };
        let logical = format!("{prefix}{}", self.target_name);
        let mut aliases = BTreeMap::new();
        for file in self.native_files().filter(|file| {
            matches!(
                file.extension().and_then(|extension| extension.to_str()),
                Some("so" | "dylib" | "dll" | "lib" | "a")
            )
        }) {
            let name = file.file_name().and_then(|name| name.to_str()).ok_or("native filename must be UTF-8")?;
            let alias = name.replace(stem, &logical);
            if alias != name {
                aliases.insert(alias, file.clone());
            }
            // MSVC produces foo.dll.lib while an ordinary #[link(name="foo")]
            // can request foo.lib. Provide both names, including for unhashed artifacts.
            if name.ends_with(".dll.lib") {
                aliases.insert(format!("{}.lib", self.target_name), file.clone());
            }
        }
        // Cargo may already report a conventional companion name. Avoid aliases that
        // would collide with an original file or duplicate another companion alias.
        for file in self.native_files() {
            if let Some(name) = file.file_name().and_then(|name| name.to_str()) {
                aliases.remove(name);
            }
        }
        Ok(aliases.into_iter().map(|(alias, source)| (source, alias)).collect())
    }

    /// Identifies the loadable shared library without assuming its Cargo directory.
    pub fn library(&self) -> Result<&Path> {
        let libraries: Vec<_> = self
            .files
            .iter()
            .filter(|path| matches!(path.extension().and_then(|extension| extension.to_str()), Some("so" | "dylib" | "dll")))
            .collect();
        match libraries.as_slice() {
            [path] => Ok(path.as_path()),
            _ => Err("Cargo did not report exactly one shared-library artifact".into()),
        }
    }

    /// Selects deployment/linker files, excluding Rust's internal rlib/rmeta artifacts.
    pub fn native_files(&self) -> impl Iterator<Item = &PathBuf> {
        self.files
            .iter()
            .filter(|path| !matches!(path.extension().and_then(|extension| extension.to_str()), Some("rlib" | "rmeta")))
    }
}

/// Cargo's JSON stream has a finite set of messages relevant to this tool.
#[derive(Deserialize)]
#[serde(tag = "reason")]
enum Message {
    /// A compilation result, emitted for both fresh and newly compiled targets.
    #[serde(rename = "compiler-artifact")]
    Artifact {
        /// Exact package identity used to exclude dependency artifacts.
        package_id: String,
        /// Target kind and name distinguish a binary from its build script/library.
        target: ArtifactTarget,
        /// Cargo-authoritative paths to the generated files.
        filenames: Vec<PathBuf>,
        /// Binary path, if this message describes an executable.
        executable: Option<PathBuf>,
        /// Cargo's cache result; phase progress must not imply actual compilation.
        fresh: bool,
    },
    /// Rustc diagnostic whose rendered form should remain visible to the user.
    #[serde(rename = "compiler-message")]
    Diagnostic {
        /// Displayable compiler output nested inside Cargo's message envelope.
        message: Diagnostic,
    },
    /// Build-script/status messages are not needed to locate selected artifacts.
    #[serde(other)]
    Other,
}

/// Stable target fields used when filtering the artifact event stream.
#[derive(Deserialize)]
struct ArtifactTarget {
    /// Binary/library target name, which need not equal the Cargo package name.
    name: String,
    /// Emitted library forms used to require a cdylib for component builds.
    crate_types: Vec<String>,
}

/// Rendered rustc output; other diagnostic fields remain owned by Cargo/rustc.
#[derive(Deserialize)]
struct Diagnostic {
    /// Human-readable warning/error text, including source spans when available.
    rendered: Option<String>,
}

/// Source locations from Cargo's resolved application dependency graph.
#[derive(Deserialize)]
struct ResolvedSources {
    /// All resolved packages, including dependencies outside the local workspace.
    packages: Vec<ResolvedPackage>,
    /// Resolved edges identify the generated project's direct component precisely.
    resolve: Option<ResolvedGraph>,
}

/// Cargo's dependency graph, used without interpreting its opaque package IDs.
#[derive(Deserialize)]
struct ResolvedGraph {
    /// Generated package root; virtual application workspaces may have no root.
    root: Option<String>,
    /// Nodes with renamed dependency keys and their exact package identities.
    nodes: Vec<ResolvedNode>,
}

impl ResolvedGraph {
    /// Finds a direct dependency by Cargo's renamed edge key, never by parsing IDs.
    fn edge(&self, root: &str, name: &str) -> Result<String> {
        self.nodes
            .iter()
            .find(|node| node.id == root)
            .and_then(|node| node.deps.iter().find(|edge| edge.name == name))
            .map(|edge| edge.pkg.clone())
            .ok_or_else(|| format!("resolver has no dependency named {name}").into())
    }
}

/// Only authored feature names are needed to request an application's default root.
#[derive(Deserialize)]
struct ApplicationFeatures {
    /// Feature definitions remain Cargo-owned; an absent table means no default feature.
    #[serde(default)]
    features: BTreeMap<String, Vec<String>>,
}

/// One resolved package and its direct dependency relationships.
#[derive(Deserialize)]
struct ResolvedNode {
    /// Exact Cargo package identifier for this node.
    id: String,
    /// Direct edges; the generated project's `component` edge is authoritative.
    deps: Vec<ResolvedEdge>,
}

/// A Cargo dependency key mapped to one resolved package version/source.
#[derive(Deserialize)]
struct ResolvedEdge {
    /// Dependency name as seen by the containing crate, including renaming.
    name: String,
    /// Exact target package ID, even when multiple versions share a package name.
    pkg: String,
}

/// Minimal source identity needed to reuse an already-declared Cargo dependency.
#[derive(Deserialize)]
struct ResolvedPackage {
    /// Cargo's opaque identity used to locate a resolved component's library target.
    id: String,
    /// Actual package name, independent of dependency renaming in the application.
    name: String,
    /// Cargo-fetched manifest; its parent is a read-only source input to our build.
    manifest_path: PathBuf,
    /// Declared outputs distinguish importable Rust libraries from binary-only crates.
    targets: Vec<ArtifactTarget>,
}

/// Cargo profiles and source overrides copied into generated workspaces.
#[derive(Default, Deserialize, Serialize)]
struct Profiles {
    /// Cargo owns the schema inside each profile; TOML values are preserved verbatim.
    #[serde(default)]
    profile: BTreeMap<String, toml::Table>,
    /// Workspace source overrides must also apply when the application is selected
    /// through a generated workspace. Cargo owns each override's remaining schema.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    patch: BTreeMap<String, BTreeMap<String, toml::Table>>,
    /// Preserve legacy exact-version replacements for existing consumer workspaces.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    replace: BTreeMap<String, toml::Table>,
}

/// A Cargo-managed workspace for a component root or the complete shared assembly.
#[derive(Serialize)]
struct GeneratedManifest {
    /// Private resolver package; only its selected dependency supplies staged artifacts.
    package: GeneratedPackage,
    /// An empty workspace prevents accidental inheritance from the consumer workspace.
    workspace: BTreeMap<String, String>,
    /// Component roots or application dependencies with their requested features.
    dependencies: BTreeMap<String, Dependency>,
    /// A component wrapper has one pure cdylib target, allowing Cargo's normal LTO.
    #[serde(skip_serializing_if = "Option::is_none")]
    lib: Option<GeneratedLibrary>,
    /// An inert target makes a resolver package valid; only its dependency targets
    /// are selected for compilation when it participates in an assembly build.
    #[serde(skip_serializing_if = "Vec::is_empty", rename = "bin")]
    binaries: Vec<GeneratedBinary>,
    /// The consumer's profiles keep local and registry component settings aligned.
    #[serde(flatten)]
    profiles: Profiles,
}

/// Identity of the generated resolver package, which is never published or staged.
#[derive(Serialize)]
struct GeneratedPackage {
    /// A name distinct from the selected component.
    name: String,
    /// Fixed local version; Cargo locks the actual component dependency versions.
    version: &'static str,
    /// Explicit edition avoids warnings and keeps resolver behavior predictable.
    edition: &'static str,
    /// Publishing generated resolver packages is always disabled.
    publish: bool,
    /// Disable library autodiscovery, including stale lib.rs from older tool versions.
    autolib: bool,
    /// Disable stale/generated binary autodiscovery when a resolver becomes a library.
    autobins: bool,
}

/// Pure shared-library root importing the original package's Rust API and C exports.
#[derive(Serialize)]
struct GeneratedLibrary {
    /// Plugins use a private name; the runtime uses its native linker name directly.
    name: String,
    /// Always cdylib alone, because rustc rejects LTO with a simultaneous rlib output.
    #[serde(rename = "crate-type")]
    crate_types: Vec<&'static str>,
    /// Documented, generated re-export source relative to its private manifest.
    path: &'static str,
}

/// Inert root selected alongside the real application to unify their dependencies.
#[derive(Serialize)]
struct GeneratedBinary {
    /// Distinct from the application's binary so Cargo outputs cannot overwrite it.
    name: String,
    /// Private source file; this executable is never deployed or run.
    path: &'static str,
}

/// Resolved wrapper inputs retained while preparing the assembly's shared graph.
struct PreparedComponent {
    /// Private wrapper manifest whose dependency selects the original source/features.
    manifest: PathBuf,
    /// Exact selected library identity, either the wrapper or a native-only component.
    id: String,
    /// Private package name used in the assembly resolver's dependency declaration.
    package: String,
    /// Original library name, including custom Cargo lib names, for runtime aliases.
    logical_name: String,
    /// Explicit request to preserve this plugin's independent feature graph.
    isolated: bool,
    /// Dependency selected by the assembly: a wrapper or an existing pure cdylib.
    dependency: Dependency,
    /// Pure cdylib implementations need an inert binary resolver, not a Rust import.
    binary: bool,
}

/// Requested artifact in a Cargo stream containing many packages and target kinds.
enum Selection {
    /// A shared library with the original component's logical linker name.
    Library(String),
    /// The configured application executable; other binaries remain build artifacts.
    Binary(String),
}

/// Cargo-compatible dependency declaration translated from our typed source config.
#[derive(Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Dependency {
    /// Actual package name even when the generated dependency key is renamed.
    package: String,
    /// Optional registry requirement; Cargo controls resolution and locking.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    /// Canonical local source directory when the user specified a path.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    /// Optional named Cargo registry from the caller's Cargo configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    registry: Option<String>,
    /// Component-specific feature list.
    features: Vec<String>,
    /// Whether this resolver enables the component's default features.
    default_features: bool,
}

impl<'a> Builder<'a> {
    /// Selects a native toolchain/target and computes contained state/output paths.
    /// Foreign targets fail before compilation because descriptor inspection loads code.
    pub fn new(project: &'a Project, options: &'a Options) -> Result<Self> {
        let mut rustc = if let Some(compiler) = std::env::var_os("RUSTC") {
            std::process::Command::new(compiler)
        } else if let Some(toolchain) = &options.toolchain {
            let mut command = std::process::Command::new("rustup");
            command.args(["run", toolchain, "rustc"]);
            command
        } else {
            std::process::Command::new("rustc")
        };
        let output = rustc.arg("-vV").output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
        }
        let version = String::from_utf8(output.stdout)?;
        let host = version
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .ok_or("rustc did not report its host triple")?;
        let release = version
            .lines()
            .find_map(|line| line.strip_prefix("release: "))
            .ok_or("rustc did not report its release")?;
        // Include compiler identity in deployment paths: stable and nightly output must
        // not replace each other's assemblies even when both use the release profile.
        let compiler_key = format!(
            "{}-{}",
            release,
            version.lines().find_map(|line| line.strip_prefix("commit-hash: ")).unwrap_or("unknown")
        );
        crate::config::validate_name(&compiler_key.replace('.', "_"))?;
        let variant_key = if options.variants.is_empty() {
            "default".into()
        } else {
            options.variants.join("+")
        };
        let state = project
            .target_dir
            .join("nc-com")
            .join(project.assembly.application.package.as_ref().unwrap())
            .join(&project.name)
            .join(variant_key);
        let min_size = options.profile == "min-size";
        let target = if min_size {
            if host != "x86_64-unknown-linux-gnu" || options.target.as_deref().is_some_and(|target| target != host) {
                return Err("min-size currently supports native x86_64-unknown-linux-gnu only".into());
            }
            if !release.contains("nightly") && !release.contains("dev") {
                return Err("min-size requires a nightly toolchain with rust-src installed".into());
            }
            let path = state.join("targets/x86_64-unknown-linux-min-size.json");
            write_changed(&path, include_bytes!("../targets/x86_64-unknown-linux-min-size.json"))?;
            path.to_str().ok_or("custom target path must be UTF-8")?.to_owned()
        } else {
            let target = options.target.as_deref().unwrap_or(host);
            if target != host {
                return Err(format!("target {target:?} cannot be inspected by this native {host} tool").into());
            }
            target.to_owned()
        };
        let target_key = if min_size { "x86_64-unknown-linux-min-size" } else { host };
        let profile_key = if options.profile == "dev" { "debug" } else { &options.profile };
        let destination = state.join("dist").join(compiler_key).join(target_key).join(profile_key);
        let mut flags = match std::env::var("CARGO_ENCODED_RUSTFLAGS") {
            Ok(encoded) => encoded.split('\x1f').filter(|flag| !flag.is_empty()).map(str::to_owned).collect::<Vec<_>>(),
            Err(_) => std::env::var("RUSTFLAGS").unwrap_or_default().split_whitespace().map(str::to_owned).collect(),
        };
        // Loader paths apply to every shipped artifact with identical settings, retaining
        // Cargo cache compatibility between the runtime, plugin, and application phases.
        if host.contains("linux") {
            flags.extend(["-C".into(), "link-arg=-Wl,-rpath,$ORIGIN".into()]);
        }
        if host.contains("apple") {
            // Rust emits @rpath dylib identities only with -Crpath. The explicit
            // @loader_path search then finds the staged library after relocation.
            flags.extend(["-Crpath".into(), "-C".into(), "link-arg=-Wl,-rpath,@loader_path".into()]);
        }
        if min_size {
            flags.extend(
                [
                    "-Cforce-unwind-tables=no",
                    "-Cstrip=symbols",
                    "-Clink-arg=-s",
                    "-Clink-arg=-Wl,--gc-sections",
                    "-Clink-arg=-Wl,--no-eh-frame-hdr",
                    "-Clink-arg=-Wl,--build-id=none",
                    "-Zlocation-detail=none",
                    "-Zfmt-debug=none",
                ]
                .map(str::to_owned),
            );
        }
        Ok(Self {
            options,
            project,
            target,
            flags: flags.join("\x1f"),
            state,
            destination,
        })
    }

    /// Resolves a named dependency from the application graph, rejecting multiple
    /// versions as ambiguous. Explicit path/version configuration disambiguates them.
    fn dependency_source(&self, name: &str) -> Result<PathBuf> {
        let mut command = self.options.cargo();
        command
            .args(["metadata", "--format-version", "1", "--manifest-path"])
            .arg(&self.project.manifest);
        // Optional component dependencies must be visible under the same application
        // feature selection that will be used for the eventual executable build.
        let application = &self.project.assembly.application;
        if !application.default_features {
            command.arg("--no-default-features");
        }
        if !application.features.is_empty() {
            let package = application.package.as_ref().unwrap();
            let features = application
                .features
                .iter()
                .map(|feature| format!("{package}/{feature}"))
                .collect::<Vec<_>>()
                .join(",");
            command.args(["--features", &features]);
        }
        self.options.policy(&mut command);
        let output = command.output()?;
        if !output.status.success() {
            return Err(format!("resolving component {name}: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        let sources: ResolvedSources = serde_json::from_slice(&output.stdout)?;
        let candidates: Vec<_> = sources.packages.into_iter().filter(|package| package.name == name).collect();
        match candidates.as_slice() {
            [package] => Ok(package.manifest_path.parent().ok_or("component manifest has no directory")?.to_owned()),
            [] => Err(format!("component {name:?} is not in the application graph; specify path or version").into()),
            _ => Err(format!("component {name:?} has multiple resolved versions; specify path or version").into()),
        }
    }

    /// Builds the runtime through a pure cdylib root with its final native name.
    pub fn component(&self, spec: &ComponentSpec, runtime_dir: Option<&Path>) -> Result<Artifact> {
        let prepared = self.prepare(spec, true)?;
        self.build_component(&prepared, runtime_dir)
    }

    /// Resolves a source declaration without guessing registry cache locations.
    fn source(&self, spec: &ComponentSpec) -> Result<Dependency> {
        let path = if let Some(path) = &spec.path {
            Some(self.project.base.join(path).canonicalize()?)
        } else if spec.version.is_none() {
            Some(
                if let Some(package) = self.project.packages.iter().find(|package| package.name == spec.package) {
                    package.manifest_path.parent().ok_or("package manifest has no parent")?.to_owned()
                } else {
                    self.dependency_source(&spec.package)?
                },
            )
        } else {
            None
        };
        Ok(Dependency {
            package: spec.package.clone(),
            version: spec.version.clone(),
            path,
            registry: spec.registry.clone(),
            features: spec.features.clone(),
            default_features: spec.default_features,
        })
    }

    /// Creates a documented private manifest with the consumer's Cargo profiles.
    fn manifest(&self, package: String, dependencies: BTreeMap<String, Dependency>) -> Result<GeneratedManifest> {
        let mut profiles: Profiles = toml::from_str(&fs::read_to_string(&self.project.manifest)?)?;
        let base = self.project.manifest.parent().unwrap();
        // Relative override paths originally belong to the consumer workspace, not
        // the generated source directory. Preserve that meaning when copying them.
        for replacement in profiles
            .patch
            .values_mut()
            .flat_map(|registry| registry.values_mut())
            .chain(profiles.replace.values_mut())
        {
            if let Some(toml::Value::String(path)) = replacement.get_mut("path") {
                *path = base.join(&*path).canonicalize()?.to_str().ok_or("override path must be UTF-8")?.into();
            }
        }
        Ok(GeneratedManifest {
            package: GeneratedPackage {
                name: package,
                version: "0.0.0",
                edition: "2024",
                publish: false,
                autolib: false,
                autobins: false,
            },
            workspace: BTreeMap::from([("resolver".into(), "3".into())]),
            dependencies,
            lib: None,
            binaries: Vec::new(),
            profiles,
        })
    }

    /// Seeds resolver locks from the consumer's dependency selections and refreshes
    /// them only when that lock changes. Locked builds never silently replace a lock.
    fn write_manifest(&self, path: &Path, manifest: &GeneratedManifest) -> Result<()> {
        write_changed(path, toml::to_string(manifest)?.as_bytes())?;
        let directory = path.parent().unwrap();
        let application = self.project.manifest.with_file_name("Cargo.lock");
        let seed = fs::read(&application)?;
        let previous = directory.join("application.lock");
        if fs::read(&previous).is_ok_and(|bytes| bytes == seed) {
            return Ok(());
        }
        if self.options.locked {
            return Err(format!(
                "{} needs its initial lock or an updated application lock; build once without --locked",
                path.display()
            )
            .into());
        }
        // Cargo adds private resolver packages and keeps compatible locked versions.
        // A separate seed records the consumer lock without mistaking Cargo's updates
        // to the generated graph for a change to the application itself.
        write_changed(&directory.join("Cargo.lock"), &seed)?;
        write_changed(&previous, &seed)
    }

    /// Resolves metadata under the same offline/lock policies as compilation.
    fn resolve(&self, manifest: &Path) -> Result<ResolvedSources> {
        let mut command = self.options.cargo();
        command.args(["metadata", "--format-version", "1", "--manifest-path"]).arg(manifest);
        self.options.policy(&mut command);
        let output = command.output()?;
        if !output.status.success() {
            return Err(format!("resolving {}: {}", manifest.display(), String::from_utf8_lossy(&output.stderr)).into());
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// Generates a pure cdylib re-export root; Cargo owns its compilation and cache.
    /// Re-exporting retains the component's public C symbols and its original crate
    /// identity, including descriptor names and package-relative compile-time inputs.
    /// `runtime` selects the original library name so its loader and linker agree
    /// without deploying a second filename for the same shared library.
    fn prepare(&self, spec: &ComponentSpec, runtime: bool) -> Result<PreparedComponent> {
        // Resolve the consumer before seeding any generated lock, including on the
        // first run of a binary-only application that has no Cargo.lock yet.
        self.resolve(&self.project.manifest)?;
        let directory = self.state.join("sources").join(&spec.package);
        let path = directory.join("Cargo.toml");
        let package = format!("nc-com-resolver-{}", spec.package);
        let mut manifest = self.manifest(package.clone(), BTreeMap::from([("component".into(), self.source(spec)?)]))?;
        // Keep a stable initial name across runs. Custom logical names are discovered
        // from the resolved dependency rather than inferred from its package spelling.
        manifest.lib = Some(GeneratedLibrary {
            name: format!("nc_com_shared_{}", spec.package.replace('-', "_")),
            crate_types: vec!["cdylib"],
            path: "src/lib.rs",
        });
        write_changed(
            &directory.join("src/lib.rs"),
            b"//! Generated shared-library root; implementation remains in its original crate.\n\
              /// Preserve the component's public C exports while allowing cdylib LTO.\n\
              pub use component::*;\n",
        )?;
        self.write_manifest(&path, &manifest)?;
        let sources = self.resolve(&path)?;
        let graph = sources.resolve.as_ref().ok_or("component resolver has no graph")?;
        let root = graph.root.as_ref().ok_or("component resolver has no root")?;
        let component = graph.edge(root, "component")?;
        let original = sources
            .packages
            .iter()
            .find(|package| package.id == component)
            .ok_or("resolved component package is missing")?;
        let library = original
            .targets
            .iter()
            .find(|target| target.crate_types.iter().any(|kind| matches!(kind.as_str(), "rlib" | "lib" | "cdylib")))
            .ok_or("components must expose a Rust library or cdylib")?;
        let binary = !library.crate_types.iter().any(|kind| kind == "rlib" || kind == "lib");
        if runtime && !binary {
            // The dependency's hashed artifact stays in deps; the selected cdylib
            // root owns the conventional top-level name and platform loader identity.
            manifest.lib.as_mut().unwrap().name = library.name.clone();
            self.write_manifest(&path, &manifest)?;
        }
        let dependency = if binary {
            // A pure cdylib already receives Cargo's LTO and cannot be imported as
            // Rust. Keep its original artifact rather than generating a wrapper.
            manifest.lib = None;
            manifest.binaries.push(GeneratedBinary {
                name: package.clone(),
                path: "src/main.rs",
            });
            write_changed(
                &directory.join("src/main.rs"),
                b"//! Resolves a native-only dependency without importing it as Rust.\n/// This executable is never deployed.\nfn main() {}\n",
            )?;
            self.write_manifest(&path, &manifest)?;
            manifest.dependencies["component"].clone()
        } else {
            Dependency {
                package: package.clone(),
                version: None,
                path: Some(directory),
                registry: None,
                features: Vec::new(),
                default_features: false,
            }
        };
        Ok(PreparedComponent {
            manifest: path,
            id: if binary { component } else { root.clone() },
            package,
            logical_name: library.name.clone(),
            isolated: spec.isolated,
            dependency,
            binary,
        })
    }

    /// Compiles one independent root, used before the application can link its runtime.
    fn build_component(&self, component: &PreparedComponent, runtime_dir: Option<&Path>) -> Result<Artifact> {
        let mut command = self.cargo_build(&component.manifest, runtime_dir);
        if component.binary {
            // Select the resolver for its feature requests, but compile only the
            // native dependency. No dummy binary needs to link an unimportable cdylib.
            command.args(["-p", &component.package, "-p", &component.id, "--lib", "--no-default-features"]);
        } else {
            command.arg("--lib");
        }
        let mut artifacts = self.collect(
            command,
            BTreeMap::from([(component.id.clone(), Selection::Library(component.logical_name.clone()))]),
            &component.package,
        )?;
        artifacts.remove(&component.id).ok_or("Cargo reported no component cdylib".into())
    }

    /// Builds compatible plugins and the executable in one dependency/feature graph.
    /// Explicitly isolated plugins retain separate graphs, while the normal Cargo
    /// dependency declarations remain responsible for feature unification and locking.
    pub fn application(&self, runtime_dir: &Path) -> Result<(Vec<Artifact>, Artifact)> {
        let components = self
            .project
            .assembly
            .plugins
            .iter()
            .map(|component| self.prepare(&component.spec(), false))
            .collect::<Result<Vec<_>>>()?;
        let app = &self.project.assembly.application;
        let application = self
            .project
            .packages
            .iter()
            .find(|package| Some(&package.name) == app.package.as_ref())
            .ok_or("application is not a workspace package")?;
        let directory = self.state.join("sources/assembly");
        let path = directory.join("Cargo.toml");
        let root_name = format!("nc-com-assembly-{}", application.name);
        let mut dependencies = BTreeMap::from([(
            "application".into(),
            Dependency {
                package: application.name.clone(),
                version: None,
                path: Some(application.manifest_path.parent().ok_or("application manifest has no parent")?.into()),
                registry: None,
                features: app.features.clone(),
                default_features: app.default_features,
            },
        )]);
        let mut isolated = BTreeMap::new();
        for (index, component) in components.iter().enumerate() {
            if component.isolated {
                // Native-only components can share Cargo output names with another
                // feature graph. Retain their files before any later Cargo invocation.
                let built = self.build_component(component, Some(runtime_dir))?;
                let directory = tempfile::Builder::new().prefix(".isolated-").tempdir_in(&self.state)?;
                let mut retained = crate::assembly::snapshot(built, directory.path())?;
                retained.storage = Some(directory);
                isolated.insert(index, retained);
            } else {
                dependencies.insert(format!("plugin_{index}"), component.dependency.clone());
            }
        }
        let mut manifest = self.manifest(root_name.clone(), dependencies)?;
        manifest.binaries.push(GeneratedBinary {
            name: root_name.clone(),
            path: "src/main.rs",
        });
        write_changed(
            &directory.join("src/main.rs"),
            b"//! Generated assembly resolver; this executable is never deployed.\n\
              /// Selects all component dependencies in the application's feature graph.\n\
              fn main() {}\n",
        )?;
        self.write_manifest(&path, &manifest)?;
        let sources = self.resolve(&path)?;
        let graph = sources.resolve.ok_or("assembly resolver has no graph")?;
        let root = graph.root.as_ref().ok_or("assembly resolver has no root")?;
        // Binary-only dependencies have no library edge in Cargo's resolve nodes,
        // but remain selectable packages in the resolver's package collection.
        let app_id = application.id.clone();
        let mut selections = BTreeMap::from([(app_id.clone(), Selection::Binary(app.bin.clone().unwrap()))]);
        let mut plugin_ids = BTreeMap::new();
        for (index, component) in components.iter().enumerate().filter(|(_, component)| !component.isolated) {
            let id = graph.edge(root, &format!("plugin_{index}"))?;
            selections.insert(id.clone(), Selection::Library(component.logical_name.clone()));
            plugin_ids.insert(index, id);
        }
        let mut command = self.cargo_build(&path, Some(runtime_dir));
        // The resolver participates in feature selection but has no selected target.
        // Explicit library roots avoid compiling an inert binary with cdylib externs.
        command.args([
            "-p",
            root,
            "-p",
            &app_id,
            "--lib",
            "--bin",
            app.bin.as_deref().unwrap(),
            "--no-default-features",
        ]);
        for id in plugin_ids.values() {
            command.args(["-p", id]);
        }
        // The selected application may be binary-only, in which case Cargo ignores
        // its library dependency but still needs its configured root-package features.
        let mut features = app.features.clone();
        if app.default_features {
            // Metadata describes whether the package actually declares a default feature.
            // Enabling an absent feature explicitly would differ from Cargo's defaults.
            let authored: ApplicationFeatures = toml::from_str(&fs::read_to_string(&application.manifest_path)?)?;
            if authored.features.contains_key("default") {
                features.push("default".into());
            }
        }
        if !features.is_empty() {
            command.args([
                "--features",
                &features.iter().map(|feature| format!("application/{feature}")).collect::<Vec<_>>().join(","),
            ]);
        }
        let mut artifacts = self.collect(command, selections, "application and shared plugins")?;
        let executable = artifacts.remove(&app_id).ok_or("Cargo reported no application executable")?;
        let mut plugins = Vec::new();
        for index in 0..components.len() {
            plugins.push(if let Some(artifact) = isolated.remove(&index) {
                artifact
            } else {
                artifacts.remove(&plugin_ids[&index]).ok_or("Cargo reported no plugin cdylib")?
            });
        }
        Ok((plugins, executable))
    }

    /// Supplies identical target/profile/compiler settings to every generated Cargo root.
    fn cargo_build(&self, manifest: &Path, runtime_dir: Option<&Path>) -> Command {
        let mut command = self.options.cargo();
        command
            .args(["build", "--manifest-path"])
            .arg(manifest)
            .args(["--profile", &self.options.profile, "--target", &self.target])
            .arg("--target-dir")
            .arg(&self.project.target_dir)
            .arg("--message-format=json-render-diagnostics")
            .env("CARGO_ENCODED_RUSTFLAGS", &self.flags);
        if let Some(directory) = runtime_dir {
            command.env("NC_COM_RUNTIME_LINK_DIR", directory);
        }
        if self.options.profile == "min-size" {
            command.args([
                "-Zjson-target-spec",
                "-Zpanic-immediate-abort",
                "-Zbuild-std=std,panic_abort",
                "-Zbuild-std-features=optimize_for_size",
            ]);
            for setting in [
                "profile.min-size.inherits=\"release\"",
                "profile.min-size.opt-level=\"z\"",
                "profile.min-size.lto=\"fat\"",
                "profile.min-size.codegen-units=1",
                "profile.min-size.debug=\"none\"",
                "profile.min-size.strip=\"symbols\"",
                "profile.min-size.incremental=false",
                "profile.min-size.panic=\"immediate-abort\"",
                "profile.min-size.overflow-checks=false",
            ] {
                command.args(["--config", setting]);
            }
        }
        self.options.policy(&mut command);
        command
    }

    /// Drains Cargo's stream, preserving diagnostics and reporting real cache results.
    fn collect(&self, mut command: Command, selections: BTreeMap<String, Selection>, label: &str) -> Result<BTreeMap<String, Artifact>> {
        eprintln!("Checking {label} ({})", self.options.profile);
        let mut child = command.stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;
        let mut artifacts = BTreeMap::new();
        let mut compiled = 0;
        let mut reused = 0;
        let read_result = (|| -> Result<()> {
            for line in BufReader::new(child.stdout.take().unwrap()).lines() {
                let line = line?;
                let message = match serde_json::from_str::<Message>(&line) {
                    Ok(message) => message,
                    Err(_) => {
                        eprintln!("{line}");
                        continue;
                    }
                };
                match message {
                    Message::Artifact {
                        package_id,
                        target,
                        filenames,
                        executable,
                        fresh,
                    } => {
                        if fresh {
                            reused += 1;
                        } else {
                            compiled += 1;
                        }
                        let logical_name = match selections.get(&package_id) {
                            Some(Selection::Library(name)) if target.crate_types.iter().any(|kind| kind == "cdylib") => name,
                            Some(Selection::Binary(name)) if target.name == *name && executable.is_some() => name,
                            _ => continue,
                        };
                        artifacts.insert(
                            package_id,
                            Artifact {
                                target_name: logical_name.clone(),
                                files: filenames,
                                executable,
                                storage: None,
                            },
                        );
                    }
                    Message::Diagnostic {
                        message: Diagnostic { rendered: Some(text) },
                    } => eprint!("{text}"),
                    _ => {}
                }
            }
            Ok(())
        })();
        // A parse/I/O failure must not leave a compiler process running or a full pipe.
        if read_result.is_err() {
            let _ = child.kill();
        }
        let status = child.wait()?;
        read_result?;
        if !status.success() {
            return Err(format!("{label} build failed ({status})").into());
        }
        if artifacts.len() != selections.len() {
            return Err(format!("Cargo did not report every requested artifact for {label}").into());
        }
        eprintln!("{label}: {compiled} compiled, {reused} cached Cargo targets");
        Ok(artifacts)
    }
}

/// Preserves unchanged generated files so Cargo can reuse fingerprints and lockfiles.
pub(crate) fn write_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    fs::create_dir_all(path.parent().ok_or("generated file has no parent")?)?;
    fs::write(path, bytes)?;
    Ok(())
}

/// Platform filename contracts can be checked independently of the native test host.
#[cfg(test)]
mod tests {
    use super::Artifact;

    /// MSVC import libraries need a conventional link-name alias, with or without a hash.
    #[test]
    fn windows_import_library_aliases() {
        for suffix in ["", "-123abc"] {
            let artifact = Artifact {
                target_name: "nc_com_shared_runtime".into(),
                files: vec![
                    format!("nc_com_shared_runtime{suffix}.dll").into(),
                    format!("nc_com_shared_runtime{suffix}.dll.lib").into(),
                ],
                executable: None,
                storage: None,
            };
            let aliases = artifact.aliases().unwrap();
            assert!(aliases.iter().any(|(_, name)| name == "nc_com_shared_runtime.lib"));
            assert_eq!(
                aliases.iter().filter(|(_, name)| name == "nc_com_shared_runtime.dll").count(),
                usize::from(!suffix.is_empty())
            );
        }
    }
}
