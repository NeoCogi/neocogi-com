//! Qualify actual Cargo archives and consumers using extracted sibling packages only.
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
};
use flate2::read::GzDecoder;
use crate::{
    compatibility::{IMPORTED_INTERFACE_PROVIDER_TEST, SHARED_SOURCE_CONSUMER_TEST, require_named_execution, words},
    process::{Result, Runner},
};

/// Name and manifest directory of the deployed codec shim.
const SERIALIZATION_PLUGIN: (&str, &str) = ("nc-serialization", "crates/serialization-dylib");

/// The complete distributable SDK; this tool and all fixture packages are unpublished.
pub(crate) const PUBLISHED: &[(&str, &str)] = &[
    ("neocogi-com", "crates/core"),
    ("neocogi-idl", "crates/idl"),
    ("neocogi-com-macros", "crates/macros"),
    ("neocogi-com-sdk", "crates/com"),
    ("neocogi-com-serialization", "crates/serialization"),
    SERIALIZATION_PLUGIN,
    ("neocogi-com-host", "crates/host"),
    ("neocogi-inspect", "tools/neocogi-inspect"),
];

/// Validates requirements independently of the graph Cargo happened to resolve.
pub(crate) fn require_exact_sdk_dependencies<'a>(packages: impl IntoIterator<Item = &'a serde_json::Value>, version: &str) -> Result<()> {
    let expected = format!("={version}");
    let published = |name: &str| PUBLISHED.iter().any(|(package, _)| *package == name);
    for package in packages {
        let owner = package["name"].as_str().ok_or("missing Cargo package name")?;
        if !published(owner) {
            continue;
        }
        let dependencies = package["dependencies"].as_array().ok_or("missing Cargo dependency declarations")?;
        for dependency in dependencies {
            // Cargo reports both the canonical package name and any local rename.
            let name = dependency["name"].as_str().ok_or("missing Cargo dependency name")?;
            if published(name) {
                let requirement = dependency["req"].as_str().ok_or("missing Cargo dependency requirement")?;
                if requirement != expected {
                    return Err(format!("{owner} requires {name} {requirement:?}, expected exact SDK requirement {expected:?}").into());
                }
            }
        }
    }
    Ok(())
}

/// Validate and extract this package's single archive prefix without an external tar.
/// Only regular source files/directories are inputs to Cargo; links are rejected.
pub(crate) fn extract(archive: &Path, destination: &Path, prefix: &str) -> Result<()> {
    let decoder = GzDecoder::new(fs::File::open(archive)?);
    let mut archive = tar::Archive::new(decoder);
    let mut names = BTreeSet::new();
    let mut source = false;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let parts: Vec<_> = path.components().collect();
        if parts.first() != Some(&Component::Normal(prefix.as_ref()))
            || parts.iter().any(|part| !matches!(part, Component::Normal(_)))
            || parts
                .iter()
                .any(|part| matches!(part.as_os_str().to_str(), Some("target" | ".git" | "plans" | "reviews" | "scripts")))
        {
            return Err(format!("unexpected archive path: {}", path.display()).into());
        }
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err(format!("unexpected archive entry type: {}", path.display()).into());
        }
        if !names.insert(path.clone()) {
            return Err(format!("duplicate archive path: {}", path.display()).into());
        }
        source |= parts.get(1).is_some_and(|part| part.as_os_str() == "src") && kind.is_file();
        if !entry.unpack_in(destination)? {
            return Err(format!("archive entry escaped extraction: {}", path.display()).into());
        }
    }
    for required in ["Cargo.toml", "Cargo.toml.orig", "README.md", "LICENSE-MIT"] {
        if !names.contains(&Path::new(prefix).join(required)) {
            return Err(format!("archive is missing {required}").into());
        }
    }
    if !source {
        return Err("archive contains no Rust sources".into());
    }
    Ok(())
}
/// Encode paths with TOML's own string formatter, including Windows separators.
fn toml_string(value: &Path) -> Result<String> {
    Ok(toml::Value::String(value.to_str().ok_or("non-UTF8 Cargo path")?.to_owned()).to_string())
}
/// Create consumers outside the workspace; only extracted siblings supply SDK code.
fn consumers(repository: &Path, root: &Path, version: &str) -> Result<()> {
    // Scratch lives below the checkout's target directory. This workspace
    // boundary owns only extracted packages and consumers, with no inherited
    // metadata or source-checkout dependencies to mask incomplete archives.
    // Temporary Cargo projects remain independent, just as in copied sources.
    let mut members = PUBLISHED.iter().map(|(name, _)| format!("{name}-{version}")).collect::<Vec<_>>();
    members.extend(["consumer".into(), "strings-consumer".into()]);
    let members = toml::Value::Array(members.into_iter().map(toml::Value::String).collect());
    fs::write(
        root.join("Cargo.toml"),
        format!("[workspace]\nresolver='3'\nexclude=['target','tmp','getting-started']\nmembers={members}\n"),
    )?;
    fs::create_dir(root.join(".cargo"))?;
    let mut patch = String::from("[patch.crates-io]\n");
    for (package, _) in PUBLISHED {
        patch.push_str(&format!(
            "{package} = {{ path = {} }}\n",
            toml_string(&root.join(format!("{package}-{version}")))?
        ));
    }
    fs::write(root.join(".cargo/config.toml"), patch)?;
    let strings = root.join("strings-consumer");
    fs::create_dir_all(strings.join("src"))?;
    fs::copy(repository.join("tests/com/examples/strings_and_bytes.rs"), strings.join("src/main.rs"))?;
    fs::copy(
        repository.join("tests/com/examples/schemas/nc/examples/strings_and_bytes.ncidl"),
        strings.join("contract.ncidl"),
    )?;
    fs::write(
        strings.join("Cargo.toml"),
        format!(
            "[package]\nname='archive-strings-consumer'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nneocogi-com-sdk='={version}'\n[build-dependencies]\nneocogi-idl='={version}'\n"
        ),
    )?;
    fs::write(strings.join("build.rs"), include_str!("../fixtures/strings-build.rs"))?;
    let consumer = root.join("consumer");
    fs::create_dir_all(consumer.join("src"))?;
    fs::create_dir_all(consumer.join("idl/proof"))?;
    fs::write(
        consumer.join("Cargo.toml"),
        format!(
            "[package]\nname='archive-consumer'\nversion='0.0.0'\nedition='2024'\nlinks='nc_contract_70726f6f662e61726368697665'\n[dependencies]\nneocogi-com-sdk={{version='={version}',features=['runtime']}}\nneocogi-com-host='={version}'\n[build-dependencies]\nneocogi-idl='={version}'\n"
        ),
    )?;
    fs::write(consumer.join("build.rs"), include_str!("../fixtures/archive-build.rs"))?;
    fs::write(consumer.join("src/main.rs"), include_str!("../fixtures/archive-main.rs"))?;
    fs::write(
        consumer.join("idl/proof/archive.ncidl"),
        "/// Minimal extracted SDK proof.\ninterface Counter { fn read(&self) -> Result<u32>; }\n",
    )?;
    Ok(())
}

/// Native output requested from Cargo; library and executable records differ.
#[derive(Clone, Copy)]
enum NativeOutput {
    /// One cdylib filename selected for the running platform.
    Library,
    /// One ordinary, non-test executable reported by Cargo.
    Executable,
}

/// Build the native codec shim and publish its hyphenated deployment filename.
pub(crate) fn stage_serialization(runner: &Runner, directory: &Path) -> Result<()> {
    let (package, manifest_dir) = SERIALIZATION_PLUGIN;
    let manifest = runner.root.join(manifest_dir).join("Cargo.toml");
    // Preserve a selected Cargo target directory for this ordinary build.
    let mut build = runner.clone();
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        build.env.insert("CARGO_TARGET_DIR".into(), std::path::absolute(target)?.into_os_string());
    }
    let output = build.read(
        "rustup",
        [
            "run",
            "nightly",
            "cargo",
            "build",
            "--profile",
            "min-size",
            "--locked",
            "-p",
            package,
            "--message-format=json",
        ],
    )?;
    let target = package.replace('-', "_");
    let artifact = native_artifact(&output, &manifest, &target, NativeOutput::Library)?;
    let filename = format!("{}{package}{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    fs::create_dir_all(directory)?;
    let deployed = directory.join(filename);
    // Publish the fully copied library only after a successful build and copy.
    let mut source = fs::File::open(artifact)?;
    let mut pending = tempfile::NamedTempFile::new_in(directory)?;
    std::io::copy(&mut source, pending.as_file_mut())?;
    pending.as_file().set_permissions(source.metadata()?.permissions())?;
    pending.persist(&deployed)?;
    println!("Staged {}", deployed.display());
    Ok(())
}

/// Select one actual fresh/cached artifact by manifest, target and output kind.
/// Reject ambiguous or missing outputs instead of guessing target directories.
fn native_artifact(output: &str, manifest: &Path, name: &str, kind: NativeOutput) -> Result<std::path::PathBuf> {
    let manifest = manifest.canonicalize()?;
    let mut selected = BTreeSet::new();
    for line in output.lines().filter(|line| line.starts_with('{')) {
        let record: serde_json::Value = serde_json::from_str(line)?;
        if record["reason"] != "compiler-artifact" || record["target"]["name"] != name || record["profile"]["test"] != false {
            continue;
        }
        let Some(owner) = record["manifest_path"].as_str() else { continue };
        if Path::new(owner).canonicalize()? != manifest {
            continue;
        }
        match kind {
            NativeOutput::Executable if record["target"]["kind"] == serde_json::json!(["bin"]) => {
                if let Some(path) = record["executable"].as_str() {
                    selected.insert(std::path::PathBuf::from(path));
                }
            }
            NativeOutput::Library
                if record["target"]["kind"] == serde_json::json!(["cdylib"]) && record["target"]["crate_types"] == serde_json::json!(["cdylib"]) =>
            {
                let filename = format!("{}{name}{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
                for path in record["filenames"].as_array().ok_or("missing Cargo artifact filenames")? {
                    let path = Path::new(path.as_str().ok_or("invalid Cargo artifact filename")?);
                    if path.file_name().is_some_and(|actual| actual == filename.as_str()) {
                        selected.insert(path.to_path_buf());
                    }
                }
            }
            NativeOutput::Executable | NativeOutput::Library => {}
        }
    }
    if selected.len() != 1 {
        return Err(format!("expected one native artifact for {name}, found {}", selected.len()).into());
    }
    let path = selected.into_iter().next().unwrap();
    crate::process::require_file(&path)?;
    Ok(path.canonicalize()?)
}

/// Verify native normal/build dependencies by their resolved package IDs.
/// Versions alone cannot distinguish an extracted candidate from the registry.
fn check_walkthrough_graph(graph: &serde_json::Value, extracted: &Path, manifest: &Path, version: &str, host: bool) -> Result<()> {
    use serde_json::Value;
    use std::collections::BTreeMap;

    fn indexed(rows: &Value) -> Result<BTreeMap<&str, &Value>> {
        rows.as_array()
            .ok_or("missing tutorial Cargo metadata entries")?
            .iter()
            .map(|row| Ok((row["id"].as_str().ok_or("missing tutorial package ID")?, row)))
            .collect()
    }
    let resolve = &graph["resolve"];
    let root = resolve["root"].as_str().ok_or("missing tutorial root package")?;
    let packages = indexed(&graph["packages"])?;
    let nodes = indexed(&resolve["nodes"])?;
    let root_package = packages.get(root).ok_or("missing tutorial root metadata")?;
    let root_manifest = Path::new(root_package["manifest_path"].as_str().ok_or("missing tutorial root manifest")?);
    if root_manifest.canonicalize()? != manifest.canonicalize()? {
        return Err("Cargo metadata selected the wrong tutorial manifest".into());
    }

    let mut direct = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let package = packages.get(id).ok_or("missing reachable tutorial package")?;
        let name = package["name"].as_str().ok_or("missing reachable tutorial package name")?;
        let forbidden = if host { name == "getting-started-plugin" } else { name == "neocogi-com-host" };
        if forbidden {
            return Err(format!("tutorial dependency direction includes {name}").into());
        }
        if PUBLISHED.iter().any(|&(published, _)| published == name) {
            let actual = Path::new(package["manifest_path"].as_str().ok_or("missing tutorial SDK manifest")?);
            let expected = extracted.join(format!("{name}-{version}/Cargo.toml"));
            if package["version"] != version || package.get("source") != Some(&Value::Null) || actual.canonicalize()? != expected.canonicalize()? {
                return Err(format!(
                    "tutorial dependency {name} must resolve to {} at version {version}; Cargo selected {} from {}",
                    expected.display(),
                    package["version"],
                    actual.display()
                )
                .into());
            }
        }
        let node = nodes.get(id).ok_or("missing reachable tutorial resolve node")?;
        for dep in node["deps"].as_array().ok_or("missing tutorial dependency edges")? {
            let child = dep["pkg"].as_str().ok_or("missing tutorial dependency ID")?;
            for kind in dep["dep_kinds"].as_array().ok_or("missing tutorial dependency kind")? {
                let build = match kind.get("kind") {
                    Some(Value::Null) => false,
                    Some(Value::String(kind)) if kind == "build" => true,
                    _ => continue,
                };
                pending.push(child);
                if id == root {
                    let name = packages
                        .get(child)
                        .and_then(|package| package["name"].as_str())
                        .ok_or("missing direct tutorial dependency")?;
                    direct.insert((name, build));
                }
            }
        }
    }
    for (name, build) in [("neocogi-com-sdk", false), ("neocogi-idl", true)]
        .into_iter()
        .chain(host.then_some(("neocogi-com-host", false)))
    {
        if !direct.contains(&(name, build)) {
            return Err(format!("tutorial root lacks its direct dependency on {name}").into());
        }
    }
    Ok(())
}

/// Execute the documented independent host/provider using extracted SDKs only.
fn walkthrough(runner: &Runner, version: &str) -> Result<()> {
    let plugin = runner.root.join("getting-started/plugin/Cargo.toml");
    let host = runner.root.join("getting-started/host/Cargo.toml");
    let compiler = runner.read("rustup", ["run", "1.85.0", "rustc", "-vV"])?;
    let triple = compiler
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or("missing Rust 1.85 host triple")?;
    for (manifest, host) in [(&plugin, false), (&host, true)] {
        let graph = runner.read(
            "rustup",
            [
                "run",
                "1.85.0",
                "cargo",
                "metadata",
                "--format-version",
                "1",
                "--filter-platform",
                triple,
                "--manifest-path",
                manifest.to_str().ok_or("non-UTF8 tutorial manifest")?,
            ],
        )?;
        check_walkthrough_graph(&serde_json::from_str(&graph)?, &runner.root, manifest, version, host)?;
    }
    let build = |manifest: &Path| -> Result<String> {
        runner.cargo(
            "1.85.0",
            &[
                "build",
                "--manifest-path",
                manifest.to_str().ok_or("non-UTF8 tutorial manifest")?,
                "--message-format=json",
                "--locked",
            ],
        )
    };
    let library = native_artifact(&build(&plugin)?, &plugin, "getting_started_plugin", NativeOutput::Library)?;
    let executable = native_artifact(&build(&host)?, &host, "getting-started-host", NativeOutput::Executable)?;
    let output = runner.run(executable, [library.as_os_str()])?;
    if output.trim() != "2 + 3 = 5" {
        return Err(format!("unexpected tutorial output: {output}").into());
    }
    let inspector = runner.root.join(format!("neocogi-inspect-{version}/Cargo.toml"));
    let executable = native_artifact(&build(&inspector)?, &inspector, "neocogi-inspect", NativeOutput::Executable)?;
    let output = runner.run(&executable, ["dump".as_ref(), library.as_os_str()])?;
    let metadata: serde_json::Value = serde_json::from_str(&output)?;
    let signature = metadata["factory"]["root_factory_signature"]
        .as_str()
        .ok_or("missing tutorial factory signature")?;
    if metadata["plugin"]["name"] != "nc.tutorial.calculator" || signature.len() != 64 || !signature.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("inspector did not identify the tutorial factory".into());
    }
    runner.run(&executable, ["check".as_ref(), library.as_os_str()])?;
    Ok(())
}
/// Check every shipped target, then let Cargo select runnable tests and doctests.
/// Explicit `--doc` rejects cdylib-only packages; ordinary test owns that target
/// policy, including `doctest=false`, without duplicating it in this driver.
/// Return the complete test output so required semantic names remain enforceable.
fn qualify_package(verification: &Runner, manifest: &Path) -> Result<String> {
    let mut output = String::new();
    for options in [vec!["check", "--all-targets", "--all-features"], vec!["test", "--all-features"]] {
        let mut args = words(&["run", "1.85.0", "cargo"]);
        args.extend(words(&options));
        args.push("--manifest-path".into());
        args.push(manifest.as_os_str().to_owned());
        output = verification.run("rustup", args)?;
    }
    Ok(output)
}

/// Qualifies feature boundaries that the general all-features lane cannot see.
fn qualify_reduced_serialization(verification: &Runner, manifest: &Path) -> Result<()> {
    for (options, required) in [
        (&["check", "--lib", "--no-default-features"][..], None),
        (
            &["test", "--no-default-features", "--test", "binary"][..],
            Some("fixed_output_roundtrips_with_exact_limits"),
        ),
        (&["test", "--test", "formats"][..], Some("default_static_plugin_omits_json")),
    ] {
        let mut args = words(&["run", "1.85.0", "cargo"]);
        args.extend(words(options));
        args.push("--manifest-path".into());
        args.push(manifest.as_os_str().to_owned());
        let output = verification.run("rustup", args)?;
        if let Some(test) = required {
            require_named_execution(&output, test)?;
        }
    }
    Ok(())
}

/// Check shipped tests/docs and execute actual generated COM against only archives.
/// Extracted sources use the same admitted temporary parent as the packaging sources.
pub(crate) fn run(runner: &Runner, source_parent: &Path) -> Result<()> {
    let mut runner = runner.clone();
    runner.env.insert("CARGO_NET_OFFLINE".into(), "true".into());
    let manifest: toml::Value = toml::from_str(&fs::read_to_string(runner.root.join("Cargo.toml"))?)?;
    let version = manifest["workspace"]["package"]["version"].as_str().ok_or("missing workspace version")?;
    let license = fs::read(runner.root.join("LICENSE-MIT"))?;
    for (_, relative) in PUBLISHED {
        if fs::read(runner.root.join(relative).join("LICENSE-MIT"))? != license {
            return Err(format!("{relative}/LICENSE-MIT differs from root license").into());
        }
    }
    let mut args = words(&["package"]);
    for (package, _) in PUBLISHED {
        args.extend(words(&["-p", package]));
    }
    // Archives reflect the current working tree; source cleanup never depends on Git.
    args.extend(words(&["--no-verify", "--allow-dirty", "--locked", "--target-dir"]));
    args.push(runner.target_dir().into_os_string());
    runner.run("rustup", words(&["run", "stable", "cargo"]).into_iter().chain(args))?;
    let (owner, mut verification) = runner.isolated_in("archives-", source_parent)?;
    let result = (|| {
        let root = owner.path();
        for (package, _) in PUBLISHED {
            let prefix = format!("{package}-{version}");
            extract(&runner.target_dir().join("package").join(format!("{prefix}.crate")), root, &prefix)?;
            if fs::read(root.join(&prefix).join("LICENSE-MIT"))? != license {
                return Err(format!("archive license mismatch: {package}").into());
            }
        }
        consumers(&runner.root, root, version)?;
        // These manifests are Cargo's normalized archive inputs, before any
        // build resolves their dependencies through the extracted patches.
        let graph: serde_json::Value = serde_json::from_str(&verification.read(
            "rustup",
            words(&["run", "stable", "cargo", "metadata", "--offline", "--no-deps", "--format-version", "1"]),
        )?)?;
        let packages = graph["packages"].as_array().ok_or("missing extracted Cargo packages")?;
        require_exact_sdk_dependencies(packages, version)?;
        crate::process::copy_tree(&runner.root.join("examples/getting-started"), &root.join("getting-started"))?;
        verification.env.insert("RUSTUP_TOOLCHAIN".into(), "1.85.0".into());
        for (package, _) in PUBLISHED {
            let manifest = root.join(format!("{package}-{version}/Cargo.toml"));
            let output = qualify_package(&verification, &manifest)?;
            if *package == "neocogi-idl" {
                require_named_execution(&output, IMPORTED_INTERFACE_PROVIDER_TEST)?;
                require_named_execution(&output, SHARED_SOURCE_CONSUMER_TEST)?;
            }
            if *package == "neocogi-com-serialization" {
                qualify_reduced_serialization(&verification, &manifest)?;
            }
        }
        let output = verification.cargo("1.85.0", &["run", "--manifest-path", "strings-consumer/Cargo.toml"])?;
        let expected = "display name: neocogi-com\nchunk len=8: \"chunked \"\nchunk len=8: \"stream o\"\nchunk len=7: \"ver COM\"";
        if output.trim_end_matches(['\r', '\n']).replace("\r\n", "\n") != expected {
            return Err(format!("unexpected strings_and_bytes output: {output}").into());
        }
        let runtime = format!("{}/Cargo.toml", root.join(format!("neocogi-com-host-{version}")).display());
        verification.cargo("1.85.0", &["build", "--manifest-path", &runtime])?;
        crate::process::require_file(&verification.target_dir().join("debug/libneocogi_com_host.rlib"))?;
        let output = verification.cargo("1.85.0", &["run", "--manifest-path", "consumer/Cargo.toml"])?;
        if output.trim() != "archive host: value=42; plugins=1" {
            return Err(format!("linked archive host did not complete its C calls: {output}").into());
        }
        let sdk = format!("neocogi-com-sdk-{version}/Cargo.toml");
        for options in [
            vec!["check", "--manifest-path", &sdk, "--no-default-features"],
            vec!["check", "--manifest-path", &sdk, "--no-default-features", "--features", "alloc"],
        ] {
            verification.cargo("1.85.0", &options)?;
        }
        walkthrough(&verification, version)?;
        Ok(())
    })();
    crate::process::retire(owner, result)?;
    println!("package archive targets, shipped tests, downstream COM and independent tutorial passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Cargo-normalized declarations retain version policy across aliases and targets.
    #[test]
    fn sdk_dependency_requirements_are_checked_without_resolution() -> Result<()> {
        let root = tempfile::tempdir()?;
        fs::create_dir(root.path().join("src"))?;
        fs::write(root.path().join("src/lib.rs"), "")?;
        let runner = Runner::new(root.path().into());
        for (table, dependency) in [
            ("dependencies", "neocogi-com"),
            ("build-dependencies", "neocogi-com"),
            ("dev-dependencies", "neocogi-com"),
            ("target.'cfg(windows)'.dependencies", "renamed = { package = 'neocogi-com', optional = true,"),
        ] {
            for requirement in ["=0.5.0-wip-alpha.1", "0.5.0-wip-alpha.1", "^0.5.0-wip-alpha.1", "*", "=0.4.0"] {
                let entry = if dependency == "neocogi-com" {
                    format!("{dependency} = '{requirement}'")
                } else {
                    format!("{dependency} version = '{requirement}' }}")
                };
                fs::write(
                    root.path().join("Cargo.toml"),
                    format!("[package]\nname = 'neocogi-idl'\nversion = '0.5.0-wip-alpha.1'\n[{table}]\n{entry}\nserde = '*'\n"),
                )?;
                let graph: serde_json::Value = serde_json::from_str(&runner.read(
                    "rustup",
                    words(&["run", "stable", "cargo", "metadata", "--offline", "--no-deps", "--format-version", "1"]),
                )?)?;
                let result = require_exact_sdk_dependencies(graph["packages"].as_array().unwrap(), "0.5.0-wip-alpha.1");
                if requirement == "=0.5.0-wip-alpha.1" {
                    result?;
                } else {
                    assert!(result.unwrap_err().to_string().contains("neocogi-idl requires neocogi-com"));
                }
            }
        }
        Ok(())
    }

    /// Cached Cargo artifacts still require exact ownership and a non-test role.
    #[test]
    fn native_artifact_admission_rejects_wrong_owner_test_and_ambiguity() -> Result<()> {
        let owner = tempfile::tempdir()?;
        let manifest = owner.path().join("Cargo.toml");
        let other_manifest = owner.path().join("other.toml");
        fs::write(&manifest, "[package]")?;
        fs::write(&other_manifest, "[package]")?;
        let executable = owner.path().join("probe");
        let other_executable = owner.path().join("other");
        fs::write(&executable, b"artifact")?;
        fs::write(&other_executable, b"artifact")?;
        let record = serde_json::json!({
            "reason": "compiler-artifact", "manifest_path": manifest,
            "target": {"name": "probe", "kind": ["bin"]},
            "profile": {"test": false}, "executable": executable, "fresh": true,
        });
        assert_eq!(
            native_artifact(&record.to_string(), &manifest, "probe", NativeOutput::Executable)?,
            executable.canonicalize()?
        );
        let mut wrong_owner = record.clone();
        wrong_owner["manifest_path"] = serde_json::json!(other_manifest);
        assert!(native_artifact(&wrong_owner.to_string(), &manifest, "probe", NativeOutput::Executable).is_err());
        let mut test = record.clone();
        test["profile"]["test"] = serde_json::json!(true);
        assert!(native_artifact(&test.to_string(), &manifest, "probe", NativeOutput::Executable).is_err());
        let mut different = record.clone();
        different["executable"] = serde_json::json!(other_executable);
        assert!(native_artifact(&format!("{record}\n{different}"), &manifest, "probe", NativeOutput::Executable).is_err());
        assert!(native_artifact(&record.to_string(), &manifest, "probe", NativeOutput::Library).is_err());
        Ok(())
    }

    /// Construct real gzip/tar bytes using the same Rust libraries as extraction.
    fn archive(path: &Path, entries: &[(&str, &[u8])], link: bool) -> Result<()> {
        let gzip = flate2::write::GzEncoder::new(fs::File::create(path)?, flate2::Compression::default());
        let mut tar = tar::Builder::new(gzip);
        for (name, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_size(bytes.len() as u64);
            header.set_cksum();
            tar.append_data(&mut header, name, *bytes)?;
        }
        if link {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o777);
            header.set_size(0);
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_link_name("Cargo.toml")?;
            header.set_cksum();
            tar.append_data(&mut header, "proof-0.0.0/linked", std::io::empty())?;
        }
        tar.into_inner()?.finish()?.flush()?;
        Ok(())
    }

    /// Real archives require metadata/source, a single prefix and ordinary files.
    #[test]
    fn archive_contents_and_bytes_are_verified() -> Result<()> {
        let owner = tempfile::tempdir()?;
        let archive_path = owner.path().join("proof.crate");
        let entries: Vec<(&str, &[u8])> = vec![
            ("proof-0.0.0/Cargo.toml", b"[package]"),
            ("proof-0.0.0/Cargo.toml.orig", b"original"),
            ("proof-0.0.0/README.md", b"readme"),
            ("proof-0.0.0/LICENSE-MIT", b"license"),
            ("proof-0.0.0/src/lib.rs", b"pub fn proof() {}"),
        ];
        archive(&archive_path, &entries, false)?;
        let good = owner.path().join("good");
        fs::create_dir(&good)?;
        extract(&archive_path, &good, "proof-0.0.0")?;
        assert_eq!(fs::read(good.join("proof-0.0.0/src/lib.rs"))?, b"pub fn proof() {}");
        let missing_source = &entries[..4];
        archive(&archive_path, missing_source, false)?;
        assert!(extract(&archive_path, owner.path(), "proof-0.0.0").is_err());
        archive(&archive_path, &entries[1..], false)?;
        assert!(extract(&archive_path, owner.path(), "proof-0.0.0").is_err());
        archive(&archive_path, &entries, true)?;
        assert!(extract(&archive_path, owner.path(), "proof-0.0.0").is_err());
        archive(&archive_path, &entries, false)?;
        assert!(extract(&archive_path, owner.path(), "wrong-prefix").is_err());
        let mut duplicate = entries.clone();
        duplicate.push(entries[0]);
        archive(&archive_path, &duplicate, false)?;
        assert!(extract(&archive_path, owner.path(), "proof-0.0.0").is_err());
        Ok(())
    }

    /// Same-version registry and checkout sources cannot pass archive qualification.
    /// A dev-only host must not be mistaken for a dependency of the built plugin.
    #[test]
    fn walkthrough_checks_reachable_origins_and_dependency_roles() -> Result<()> {
        use serde_json::json;

        let owner = tempfile::tempdir()?;
        let extracted = owner.path();
        let make_manifest = |directory: &str| -> Result<std::path::PathBuf> {
            let directory = extracted.join(directory);
            fs::create_dir_all(&directory)?;
            let manifest = directory.join("Cargo.toml");
            fs::write(&manifest, "")?;
            Ok(manifest)
        };
        let plugin = make_manifest("getting-started/plugin")?;
        let facade = make_manifest("neocogi-com-sdk-0.3.0")?;
        let compiler = make_manifest("neocogi-idl-0.3.0")?;
        let wrong = make_manifest("different-checkout/crates/com")?;
        let mut graph = json!({
            "packages": [
                {"id":"root", "name":"getting-started-plugin", "manifest_path":plugin},
                {"id":"facade", "name":"neocogi-com-sdk", "version":"0.3.0", "source":null, "manifest_path":facade},
                {"id":"compiler", "name":"neocogi-idl", "version":"0.3.0", "source":null, "manifest_path":compiler},
                {"id":"host", "name":"neocogi-com-host"}
            ],
            "resolve": {"root":"root", "nodes":[
                {"id":"root", "deps":[
                    {"pkg":"facade", "dep_kinds":[{"kind":null}]},
                    {"pkg":"compiler", "dep_kinds":[{"kind":"build"}]}
                ]},
                {"id":"facade", "deps":[{"pkg":"host", "dep_kinds":[{"kind":"dev"}]}]},
                {"id":"compiler", "deps":[]}
            ]}
        });
        check_walkthrough_graph(&graph, extracted, &plugin, "0.3.0", false)?;

        graph["packages"][1]["manifest_path"] = json!(wrong);
        graph["packages"][1]["source"] = json!("registry+https://github.com/rust-lang/crates.io-index");
        let error = check_walkthrough_graph(&graph, extracted, &plugin, "0.3.0", false).unwrap_err();
        assert!(error.to_string().contains("neocogi-com-sdk must resolve to"), "{error}");
        graph["packages"][1]["source"] = json!(null);
        let error = check_walkthrough_graph(&graph, extracted, &plugin, "0.3.0", false).unwrap_err();
        assert!(error.to_string().contains("different-checkout"), "{error}");
        graph["packages"][1]["manifest_path"] = json!(facade);

        graph["resolve"]["nodes"][1]["deps"][0]["dep_kinds"][0]["kind"] = json!(null);
        let error = check_walkthrough_graph(&graph, extracted, &plugin, "0.3.0", false).unwrap_err();
        assert!(error.to_string().contains("direction includes neocogi-com-host"), "{error}");
        graph["resolve"]["nodes"][1]["deps"][0]["dep_kinds"][0]["kind"] = json!("dev");
        graph["resolve"]["nodes"][0]["deps"][1]["dep_kinds"][0]["kind"] = json!(null);
        let error = check_walkthrough_graph(&graph, extracted, &plugin, "0.3.0", false).unwrap_err();
        assert!(error.to_string().contains("lacks its direct dependency on neocogi-idl"), "{error}");
        Ok(())
    }

    /// Cargo owns the extracted assembly below an unrelated source workspace.
    /// As in the checkout, its target subtree excludes nested task-owned projects.
    #[test]
    fn extracted_workspace_cannot_join_the_source_checkout() -> Result<()> {
        let owner = tempfile::tempdir()?;
        fs::write(owner.path().join("Cargo.toml"), "[workspace]\nmembers=['source-only']\nexclude=['target']\n")?;
        let root = owner.path().join("target/extracted");
        fs::create_dir_all(&root)?;
        for (package, _) in PUBLISHED {
            let member = root.join(format!("{package}-0.0.0"));
            fs::create_dir_all(member.join("src"))?;
            fs::write(
                member.join("Cargo.toml"),
                format!("[package]\nname='{package}'\nversion='0.0.0'\nedition='2024'\n[features]\nruntime=[]\nalloc=[]\n"),
            )?;
            fs::write(member.join("src/lib.rs"), "")?;
        }
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        consumers(&repository, &root, "0.0.0")?;
        let mut runner = Runner::new(root.canonicalize()?);
        runner.temporary_files(runner.root.join("tmp"))?;
        crate::tests::require_independent_temporary_package(&runner)?;
        let graph: serde_json::Value = serde_json::from_str(&runner.read(
            "rustup",
            [
                "run",
                "stable",
                "cargo",
                "metadata",
                "--format-version",
                "1",
                "--offline",
                "--no-deps",
                "--manifest-path",
                "neocogi-com-0.0.0/Cargo.toml",
            ],
        )?)?;
        assert_eq!(Path::new(graph["workspace_root"].as_str().unwrap()).canonicalize()?, root.canonicalize()?);
        // The extracted SDK inventory and its two executable consumers are owned
        // here; adding a published implementation must not imply source membership.
        assert_eq!(graph["workspace_members"].as_array().unwrap().len(), PUBLISHED.len() + 2);
        for package in graph["packages"].as_array().unwrap() {
            assert!(
                Path::new(package["manifest_path"].as_str().unwrap())
                    .canonicalize()?
                    .starts_with(root.canonicalize()?)
            );
        }
        Ok(())
    }

    /// The actual archive operation admits mixed target kinds, retains their
    /// tests, and propagates a supported doctest failure on the compiler floor.
    #[test]
    fn archive_tests_follow_cargo_target_policy_and_keep_failures_fatal() -> Result<()> {
        let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
        let (owner, runner) = Runner::new(checkout).isolated("archive-targets-")?;
        let result = (|| {
            let names = ["ordinary", "macros", "binary", "shim", "disabled"];
            fs::write(
                runner.root.join("Cargo.toml"),
                "[workspace]\nresolver='3'\nexclude=['tmp']\nmembers=['ordinary','macros','binary','shim','disabled']\n",
            )?;
            for name in names {
                let root = runner.root.join(name);
                fs::create_dir_all(root.join("src"))?;
                let library = match name {
                    "macros" => "[lib]\nproc-macro=true\n",
                    "shim" => "[lib]\ncrate-type=['cdylib']\n",
                    "disabled" => "[lib]\ndoctest=false\n",
                    _ => "",
                };
                fs::write(
                    root.join("Cargo.toml"),
                    format!("[package]\nname='archive-{name}'\nversion='0.0.0'\nedition='2024'\n{library}"),
                )?;
                let supported = matches!(name, "ordinary" | "macros");
                let assertion = if supported {
                    "assert_eq!(2+2,4);"
                } else {
                    "compile_error!(\"unsupported doctest selected\");"
                };
                let body = match name {
                    "macros" => {
                        "extern crate proc_macro;\n#[proc_macro] pub fn passthrough(value: proc_macro::TokenStream) -> proc_macro::TokenStream { value }"
                    }
                    "binary" => "fn main() {}",
                    _ => "pub fn value() -> u8 { 4 }",
                };
                fs::write(
                    root.join("src").join(if name == "binary" { "main.rs" } else { "lib.rs" }),
                    format!("//! ```\n//! {assertion}\n//! ```\n{body}\n#[cfg(test)] mod tests {{ #[test] fn unit_runs() {{ assert_eq!(2+2,4); }} }}\n"),
                )?;
                if name == "ordinary" {
                    fs::create_dir(root.join("tests"))?;
                    fs::write(root.join("tests/integration.rs"), "#[test] fn integration_runs() {}")?;
                }
            }
            // Cargo loads every workspace member before selecting one package.
            for name in names {
                let root = runner.root.join(name);
                let supported = matches!(name, "ordinary" | "macros");
                let output = qualify_package(&runner, &root.join("Cargo.toml"))?;
                require_named_execution(&output, "tests::unit_runs")?;
                if name == "ordinary" {
                    require_named_execution(&output, "integration_runs")?;
                }
                // Rustdoc uses native path separators in its test names.
                let doctest = format!("{} - (line 1) ... ok", Path::new("src").join("lib.rs").display());
                assert_eq!(output.contains(&doctest), supported, "wrong doc selection for {name}");
            }
            let source = runner.root.join("ordinary/src/lib.rs");
            fs::write(&source, fs::read_to_string(&source)?.replacen("assert_eq!(2+2,4);", "assert_eq!(2+2,5);", 1))?;
            let error = qualify_package(&runner, &runner.root.join("ordinary/Cargo.toml")).unwrap_err();
            assert!(error.to_string().contains("101"), "wrong supported-test failure: {error}");
            Ok(())
        })();
        crate::process::retire(owner, result)
    }

    /// Path serialization preserves Windows separators, spaces and quotes verbatim.
    #[test]
    fn configuration_paths_round_trip_through_toml() -> Result<()> {
        let path = Path::new(r#"C:\some "quoted" place\folder"#);
        let parsed: toml::Value = toml::from_str(&format!("path={}", toml_string(path)?))?;
        assert_eq!(parsed["path"].as_str().unwrap(), path.to_str().unwrap());
        Ok(())
    }
}
