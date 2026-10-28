//! Qualify actual Cargo archives and consumers using extracted sibling packages only.
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
};
use flate2::read::GzDecoder;
use crate::{
    compatibility::{IMPORTED_INTERFACE_PROVIDER_TEST, require_named_execution, words},
    process::{Result, Runner},
};

/// The complete distributable SDK; this tool and all fixture packages are unpublished.
pub(crate) const PUBLISHED: &[(&str, &str)] = &[
    ("neocogi-com-core", "crates/core"),
    ("neocogi-idl", "crates/idl"),
    ("neocogi-com-macros", "crates/macros"),
    ("neocogi-com", "crates/com"),
    ("neocogi-com-host", "crates/host"),
    ("neocogi-inspect", "tools/neocogi-inspect"),
];
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
    let mut members = PUBLISHED.iter().map(|(name, _)| format!("{name}-{version}")).collect::<Vec<_>>();
    members.extend(["consumer".into(), "strings-consumer".into()]);
    let members = toml::Value::Array(members.into_iter().map(toml::Value::String).collect());
    fs::write(root.join("Cargo.toml"), format!("[workspace]\nresolver='3'\nmembers={members}\n"))?;
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
            "[package]\nname='archive-strings-consumer'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nneocogi-com='={version}'\n[build-dependencies]\nneocogi-idl='={version}'\n"
        ),
    )?;
    fs::write(strings.join("build.rs"), include_str!("../fixtures/strings-build.rs"))?;
    let consumer = root.join("consumer");
    fs::create_dir_all(consumer.join("src"))?;
    fs::create_dir_all(consumer.join("idl/proof"))?;
    fs::write(
        consumer.join("Cargo.toml"),
        format!(
            "[package]\nname='archive-consumer'\nversion='0.0.0'\nedition='2024'\nlinks='nc_contract_70726f6f662e61726368697665'\n[dependencies]\nneocogi-com={{version='={version}',features=['runtime']}}\nneocogi-com-host='={version}'\n[build-dependencies]\nneocogi-idl='={version}'\n"
        ),
    )?;
    fs::write(consumer.join("build.rs"), include_str!("../fixtures/archive-build.rs"))?;
    fs::write(consumer.join("src/main.rs"), include_str!("../fixtures/archive-main.rs"))?;
    fs::write(
        consumer.join("idl/proof/archive.ncidl"),
        "/// Minimal extracted SDK proof.\n#[version(1)] interface Counter { fn read(&self) -> Result<u32>; }\n",
    )?;
    Ok(())
}
/// Check shipped tests/docs and execute actual generated COM against only archives.
pub(crate) fn run(runner: &Runner) -> Result<()> {
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
    args.push(runner.root.join("target").into_os_string());
    runner.run("rustup", words(&["run", "stable", "cargo"]).into_iter().chain(args))?;
    let owner = runner.scratch("archives-")?;
    let result = (|| {
        let root = owner.path();
        for (package, _) in PUBLISHED {
            let prefix = format!("{package}-{version}");
            extract(&runner.root.join("target/package").join(format!("{prefix}.crate")), root, &prefix)?;
            if fs::read(root.join(&prefix).join("LICENSE-MIT"))? != license {
                return Err(format!("archive license mismatch: {package}").into());
            }
        }
        consumers(&runner.root, root, version)?;
        let mut verification = runner.clone();
        verification.root = root.to_path_buf();
        verification.env.insert("RUSTUP_TOOLCHAIN".into(), "1.85.0".into());
        verification.env.insert("CARGO_TARGET_DIR".into(), root.join("target").into_os_string());
        verification.temporary_files(root.join("tmp"))?;
        for (package, _) in PUBLISHED {
            let manifest = root.join(format!("{package}-{version}/Cargo.toml"));
            for options in [
                vec!["check", "--all-targets", "--all-features"],
                vec!["test", "--tests", "--all-features"],
                vec!["test", "--doc", "--all-features"],
            ] {
                if package == &"neocogi-inspect" && options.contains(&"--doc") {
                    continue;
                }
                let mut args = words(&["run", "1.85.0", "cargo"]);
                args.extend(words(&options));
                args.push("--manifest-path".into());
                args.push(manifest.as_os_str().to_owned());
                let output = verification.run("rustup", args)?;
                if package == &"neocogi-idl" && options.contains(&"--tests") {
                    require_named_execution(&output, IMPORTED_INTERFACE_PROVIDER_TEST)?;
                }
            }
        }
        let output = verification.cargo("1.85.0", &["run", "--manifest-path", "strings-consumer/Cargo.toml"])?;
        let expected = "display name: neocogi-com\nchunk len=8: \"chunked \"\nchunk len=8: \"stream o\"\nchunk len=7: \"ver COM\"";
        if output.trim_end_matches(['\r', '\n']).replace("\r\n", "\n") != expected {
            return Err(format!("unexpected strings_and_bytes output: {output}").into());
        }
        let runtime = format!("{}/Cargo.toml", root.join(format!("neocogi-com-host-{version}")).display());
        verification.cargo("1.85.0", &["build", "--manifest-path", &runtime])?;
        crate::process::require_file(&root.join("target/debug/libneocogi_com_host.rlib"))?;
        verification.cargo("1.85.0", &["run", "--manifest-path", "consumer/Cargo.toml"])?;
        let facade = format!("neocogi-com-{version}/Cargo.toml");
        for options in [
            vec!["check", "--manifest-path", &facade, "--no-default-features"],
            vec!["check", "--manifest-path", &facade, "--no-default-features", "--features", "alloc"],
        ] {
            verification.cargo("1.85.0", &options)?;
        }
        Ok(())
    })();
    crate::process::retire(owner, result)?;
    println!("package archive targets, shipped tests and downstream COM call passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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

    /// Cargo owns the extracted assembly even when an unrelated workspace is above it.
    #[test]
    fn extracted_workspace_cannot_join_the_source_checkout() -> Result<()> {
        let owner = tempfile::tempdir()?;
        fs::write(owner.path().join("Cargo.toml"), "[workspace]\nmembers=['source-only']\n")?;
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
        let runner = Runner::new(root.canonicalize()?);
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
                "neocogi-com-core-0.0.0/Cargo.toml",
            ],
        )?)?;
        assert_eq!(Path::new(graph["workspace_root"].as_str().unwrap()).canonicalize()?, root.canonicalize()?);
        assert_eq!(graph["workspace_members"].as_array().unwrap().len(), 8);
        for package in graph["packages"].as_array().unwrap() {
            assert!(
                Path::new(package["manifest_path"].as_str().unwrap())
                    .canonicalize()?
                    .starts_with(root.canonicalize()?)
            );
        }
        Ok(())
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
