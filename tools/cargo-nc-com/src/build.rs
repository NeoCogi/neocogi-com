//! Ordered Cargo subprocesses and artifact discovery for local and registry sources.
//!
//! This module runs inside the installed command, not inside a Cargo build script.
//! Each selected library is a separate Cargo invocation so its features remain
//! independent of the application's features. A common target directory retains
//! Cargo's cache, including compatible rebuilt standard-library artifacts.

use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
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
            _ => Err("Cargo did not report exactly one cdylib; declare crate-type = [\"rlib\", \"cdylib\"]".into()),
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
    /// Actual package name, independent of dependency renaming in the application.
    name: String,
    /// Cargo-fetched manifest; its parent is a read-only source input to our build.
    manifest_path: PathBuf,
}

/// Standard Cargo profile tables copied into generated component workspaces.
#[derive(Default, Deserialize, Serialize)]
struct Profiles {
    /// Cargo owns the schema inside each profile; TOML values are preserved verbatim.
    #[serde(default)]
    profile: BTreeMap<String, toml::Table>,
}

/// A self-contained resolver project used for an explicitly versioned/path component.
#[derive(Serialize)]
struct GeneratedManifest {
    /// Private resolver package; only its selected dependency supplies staged artifacts.
    package: GeneratedPackage,
    /// An empty workspace prevents accidental inheritance from the consumer workspace.
    workspace: BTreeMap<String, String>,
    /// One component per resolver project preserves independent feature selection.
    dependencies: BTreeMap<String, Dependency>,
    /// The consumer's profiles keep local and registry component settings aligned.
    #[serde(flatten)]
    profiles: Profiles,
}

/// Identity of the generated resolver package, which is never published or staged.
#[derive(Serialize)]
struct GeneratedPackage {
    /// A name distinct from the selected component.
    name: String,
    /// Fixed local version; component versions are locked separately by Cargo.
    version: &'static str,
    /// Explicit edition avoids warnings and keeps resolver behavior predictable.
    edition: &'static str,
    /// Publishing generated resolver packages is always disabled.
    publish: bool,
    /// Disable library autodiscovery, including stale lib.rs from older tool versions.
    autolib: bool,
}

/// Cargo-compatible dependency declaration translated from our typed source config.
#[derive(Serialize)]
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

    /// Builds a configured component as a root package and requires a shared artifact.
    pub fn component(&self, spec: &ComponentSpec, runtime_dir: Option<&Path>) -> Result<Artifact> {
        let manifest = if spec.version.is_some() || spec.path.is_some() {
            self.generated_manifest(spec)?
        } else {
            if self.project.packages.iter().any(|package| package.name == spec.package) {
                self.project.manifest.clone()
            } else {
                // Reuse Cargo's fetched sources for a named dependency, without guessing
                // a registry cache layout or writing inside that immutable source tree.
                let mut resolved = spec.clone();
                resolved.path = Some(self.dependency_source(&spec.package)?);
                self.generated_manifest(&resolved)?
            }
        };
        let artifact = self.compile(&manifest, &spec.package, None, &spec.features, spec.default_features, runtime_dir)?;
        artifact.library()?;
        Ok(artifact)
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

    /// Links the application only after a completed runtime has an established directory.
    pub fn application(&self, runtime_dir: &Path) -> Result<Artifact> {
        let app = &self.project.assembly.application;
        self.compile(
            &self.project.manifest,
            app.package.as_deref().unwrap(),
            app.bin.as_deref(),
            &app.features,
            app.default_features,
            Some(runtime_dir),
        )
    }

    /// Writes a stable, Cargo-resolved source project and retains its lockfile between runs.
    fn generated_manifest(&self, spec: &ComponentSpec) -> Result<PathBuf> {
        let directory = self.state.join("sources").join(&spec.package);
        let profiles: Profiles = toml::from_str(&fs::read_to_string(&self.project.manifest)?)?;
        let dependency = Dependency {
            package: spec.package.clone(),
            version: spec.version.clone(),
            path: spec.path.as_ref().map(|path| self.project.base.join(path).canonicalize()).transpose()?,
            registry: spec.registry.clone(),
            features: spec.features.clone(),
            default_features: spec.default_features,
        };
        let manifest = GeneratedManifest {
            package: GeneratedPackage {
                name: format!("nc-com-resolver-{}", spec.package),
                version: "0.0.0",
                edition: "2024",
                publish: false,
                autolib: false,
            },
            workspace: BTreeMap::from([("resolver".into(), "3".into())]),
            dependencies: BTreeMap::from([("component".into(), dependency)]),
            profiles,
        };
        write_changed(
            &directory.join("src/main.rs"),
            b"//! Generated dependency resolver; this executable is never staged.\n/// Keeps the resolver root compatible with application LTO builds.\nfn main() {}\n",
        )?;
        let path = directory.join("Cargo.toml");
        write_changed(&path, toml::to_string(&manifest)?.as_bytes())?;
        Ok(path)
    }

    /// Resolves an exact package ID, streams diagnostics, and selects its final artifact.
    fn compile(
        &self,
        manifest: &Path,
        package: &str,
        binary: Option<&str>,
        features: &[String],
        defaults: bool,
        runtime_dir: Option<&Path>,
    ) -> Result<Artifact> {
        eprintln!("Building {package} ({})", self.options.profile);
        // Cargo's resolved edge supplies the exact component identity, even when a
        // transitive dependency has another version of the same package name.
        let generated = manifest != self.project.manifest;
        let mut resolve = self.options.cargo();
        resolve.args(["metadata", "--format-version", "1", "--manifest-path"]).arg(manifest);
        self.options.policy(&mut resolve);
        let resolved = resolve.output()?;
        if !resolved.status.success() {
            return Err(format!("resolving {package}: {}", String::from_utf8_lossy(&resolved.stderr)).into());
        }
        let id = if generated {
            let sources: ResolvedSources = serde_json::from_slice(&resolved.stdout)?;
            let graph = sources.resolve.ok_or("Cargo did not resolve the generated component graph")?;
            let root = graph.root.ok_or("generated component graph has no root")?;
            graph
                .nodes
                .into_iter()
                .find(|node| node.id == root)
                .and_then(|node| node.deps.into_iter().find(|edge| edge.name == "component"))
                .ok_or("generated project has no resolved component dependency")?
                .pkg
        } else {
            self.project
                .packages
                .iter()
                .find(|candidate| candidate.name == package)
                .ok_or("selected application/component is not a workspace package")?
                .id
                .clone()
        };
        let mut command = self.options.cargo();
        command
            .args(["build", "--manifest-path"])
            .arg(manifest)
            .args(["--profile", &self.options.profile, "--target", &self.target]);
        // Generated projects must build the resolver root: the dependency declaration
        // then owns features, and Cargo emits the dependency's cdylib artifact too.
        if !generated {
            command.args(["-p", &id]);
        }
        command
            .arg("--target-dir")
            .arg(&self.project.target_dir)
            .arg("--message-format=json-render-diagnostics");
        command.env("CARGO_ENCODED_RUSTFLAGS", &self.flags);
        if let Some(directory) = runtime_dir {
            command.env("NC_COM_RUNTIME_LINK_DIR", directory);
        }
        if generated {
            command.args(["--bin", &format!("nc-com-resolver-{package}")]);
        } else if let Some(binary) = binary {
            command.args(["--bin", binary]);
        } else {
            command.arg("--lib");
        }
        if !generated && !defaults {
            command.arg("--no-default-features");
        }
        if !generated && !features.is_empty() {
            command.args(["--features", &features.join(",")]);
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
        let mut child = command.stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;
        let mut artifact = None;
        // Drain stdout before waiting to avoid pipe deadlocks. A malformed compiler
        // message is reported after waiting, so no failed subprocess is left behind.
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
                    } if package_id == id => {
                        let selected = binary.map_or_else(
                            || target.crate_types.iter().any(|kind| kind == "cdylib"),
                            |name| target.name == name && executable.is_some(),
                        );
                        if selected {
                            artifact = Some(Artifact {
                                target_name: target.name,
                                files: filenames,
                                executable,
                            });
                        }
                    }
                    Message::Diagnostic {
                        message: Diagnostic { rendered: Some(text) },
                    } => eprint!("{text}"),
                    _ => {}
                }
            }
            Ok(())
        })();
        if read_result.is_err() {
            let _ = child.kill();
        }
        let status = child.wait()?;
        read_result?;
        if !status.success() {
            return Err(format!("{package} build failed ({status})").into());
        }
        artifact.ok_or_else(|| format!("Cargo reported no requested artifact for {package}; components must export a cdylib").into())
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
                target_name: "nc_com_runtime".into(),
                files: vec![format!("nc_com_runtime{suffix}.dll").into(), format!("nc_com_runtime{suffix}.dll.lib").into()],
                executable: None,
            };
            let aliases = artifact.aliases().unwrap();
            assert!(aliases.iter().any(|(_, name)| name == "nc_com_runtime.lib"));
            assert_eq!(
                aliases.iter().filter(|(_, name)| name == "nc_com_runtime.dll").count(),
                usize::from(!suffix.is_empty())
            );
        }
    }
}
