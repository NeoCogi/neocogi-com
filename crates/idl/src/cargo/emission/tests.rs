//! Causal filesystem-finishing and unchanged-publication checks over real emissions.

use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};
use super::{ModuleEmission, generate_module_emission, write_emissions};
use crate::{
    ResolvedModules,
    generate::RustModulePaths,
    cargo::{
        BuildError, GraphModule, GraphRecord, GRAPH_SCHEMA, ModuleSources,
        test_support::{SnapshotEntry, seed_unrelated, snapshot},
    },
};

/// Writes independent source owners; only the first module changes between revisions.
fn sources(root: &Path, changed: bool) {
    fs::create_dir_all(root.join("proof")).unwrap();
    let (prefix, scalar) = if changed {
        ("\n/// Changed authored revision.\n", "u32")
    } else {
        ("", "u64")
    };
    fs::write(
        root.join("proof/a.ncidl"),
        format!("{prefix}#[version(1)] struct Ready {{ value: Option<{scalar}> }}"),
    )
    .unwrap();
    fs::write(root.join("proof/z.ncidl"), "#[version(1)] struct Later { value: u32 }").unwrap();
}

/// Prepares real source records, paired generator results and graph bytes without writing output.
fn prepare(root: &Path, output: &Path) -> (ResolvedModules, GraphRecord, Vec<ModuleEmission>, Vec<u8>) {
    let names = ["proof.a", "proof.z"];
    let mut records = BTreeMap::new();
    let mut paths = RustModulePaths::default();
    for name in names {
        let rust_path = name.replace('.', "::");
        paths.insert(name, &rust_path).unwrap();
        let sources = ModuleSources::load_file(root, &std::path::PathBuf::from(name.replace('.', "/")).with_extension("ncidl")).unwrap();
        records.insert(name.to_owned(), GraphModule { sources, rust_path: Some(rust_path) });
    }
    let modules = records.values().map(|record| record.sources.parse().unwrap()).collect();
    let resolved = crate::resolve_modules(modules).unwrap();
    let emissions = names
        .into_iter()
        .map(|name| generate_module_emission(&resolved, name, &paths, output, &name.replace('.', "_"), "// Test-owned completed emission.").unwrap())
        .collect();
    let graph = GraphRecord {
        schema: GRAPH_SCHEMA,
        modules: records,
        source_roots: names.into_iter().map(|name| (name.to_owned(), root.to_owned())).collect(),
    };
    let bytes = serde_json::to_vec_pretty(&graph).unwrap();
    (resolved, graph, emissions, bytes)
}

/// Compares main artifacts and unrelated files while allowing navigation materialization.
fn main_snapshot(output: &Path) -> BTreeMap<std::path::PathBuf, SnapshotEntry> {
    snapshot(output)
        .unwrap()
        .into_iter()
        .filter(|(path, _)| !path.starts_with("idl-sources"))
        .collect()
}

/// A second map's real canonicalization error cannot publish the first changed module.
#[test]
fn later_map_failure_preserves_all_main_artifacts() {
    for seeded in [false, true] {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("schemas");
        let output = fixture.path().join("output");
        seed_unrelated(&output);
        sources(&root, false);
        if seeded {
            let (resolved, graph, emissions, bytes) = prepare(&root, &output);
            write_emissions(emissions, &graph.source_roots, &resolved, &output, &bytes).unwrap();
        }
        let before = main_snapshot(&output);
        sources(&root, true);
        let (resolved, graph, emissions, bytes) = prepare(&root, &output);
        if seeded {
            assert_ne!(
                emissions[0].generated.source.as_bytes(),
                fs::read(&emissions[0].output.bindings).unwrap(),
                "the first prepared Rust must actually change"
            );
        }
        let missing = root.join("proof/z.ncidl");
        fs::remove_file(&missing).unwrap();
        let error = write_emissions(emissions, &graph.source_roots, &resolved, &output, &bytes).unwrap_err();
        let BuildError::Io { operation, path, source } = error else {
            panic!("wrong finishing phase: {error:?}")
        };
        assert_eq!(operation, "resolve source-map location");
        assert_eq!(path, missing);
        assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(main_snapshot(&output), before);
        // The first map reached bundled materialization before the later error.
        // Those exact navigation bytes are allowed, unlike main publication.
        let bundled = &resolved.module("neocogi.lang").unwrap().source.sources[0];
        assert_eq!(fs::read(output.join("idl-sources").join(bundled.path())).unwrap(), bundled.text().as_bytes());
    }
}

/// Repeating successful publication preserves exact bytes and known old timestamps.
#[test]
fn unchanged_publication_preserves_bytes_and_timestamps() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("schemas");
    let output = fixture.path().join("output");
    sources(&root, false);
    seed_unrelated(&output);
    let (resolved, graph, emissions, bytes) = prepare(&root, &output);
    let first = write_emissions(emissions, &graph.source_roots, &resolved, &output, &bytes).unwrap();
    let old = UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    let tree = snapshot(&output).unwrap();
    let mut times = BTreeMap::new();
    for (relative, entry) in &tree {
        if matches!(entry, SnapshotEntry::File(_)) {
            let file = fs::OpenOptions::new().write(true).open(output.join(relative)).unwrap();
            file.set_modified(old).unwrap();
            let actual = file.metadata().unwrap().modified().unwrap();
            times.insert(relative.clone(), actual);
        }
    }
    let (resolved, graph, emissions, bytes) = prepare(&root, &output);
    let second = write_emissions(emissions, &graph.source_roots, &resolved, &output, &bytes).unwrap();
    assert_eq!(first, second);
    assert_eq!(snapshot(&output).unwrap(), tree);
    for (relative, before) in times {
        assert_eq!(
            fs::metadata(output.join(&relative)).unwrap().modified().unwrap(),
            before,
            "unchanged output was rewritten: {}",
            relative.display()
        );
    }
    let bundled = &resolved.module("neocogi.lang").unwrap().source.sources[0];
    assert_eq!(fs::read(output.join("idl-sources").join(bundled.path())).unwrap(), bundled.text().as_bytes());
    for generated in second {
        let map: crate::RustSourceMap = serde_json::from_slice(&fs::read(&generated.source_map).unwrap()).unwrap();
        assert_eq!(map.generated_file, generated.bindings.file_name().unwrap().to_str().unwrap());
        for source in map.sources {
            assert!(output.join(source.location.unwrap()).is_file());
        }
    }
}

/// Reads actual emitted Rust use trees, not the policy helper under test.
fn use_names(tree: &syn::UseTree) -> Vec<String> {
    match tree {
        syn::UseTree::Path(path) => use_names(&path.tree),
        syn::UseTree::Group(group) => group.items.iter().flat_map(use_names).collect(),
        syn::UseTree::Name(name) => vec![name.ident.to_string()],
        syn::UseTree::Rename(rename) => vec![rename.rename.to_string()],
        syn::UseTree::Glob(_) => vec!["*".into()],
    }
}

/// The normal module and explicit wrapper must expose exactly the same original
/// interface family; metadata and typed dependency namespace retain precedence.
#[test]
fn imported_interface_export_inventory_matches_both_actual_surfaces() {
    use std::collections::BTreeSet;
    let base = crate::parse_module(Path::new("proof/base.ncidl"), "#[version(1)] interface IBase { fn get(&self) -> Result<u32>; }").unwrap();
    let consumer = crate::parse_module(
        Path::new("proof/consumer.ncidl"),
        "extern proof.base; use proof.base::IBase; #[version(1)] struct Anchor { value: u32 }",
    )
    .unwrap();
    let graph = crate::resolve_modules(vec![base, consumer]).unwrap();
    let mut paths = RustModulePaths::default();
    paths.insert("proof.base", "base::r#type").unwrap();
    paths.insert("proof.consumer", "consumer::api").unwrap();
    paths.route_dependency(&graph, "proof.base", "proof.base").unwrap();
    let owner = tempfile::tempdir().unwrap();
    let emission = generate_module_emission(&graph, "proof.consumer", &paths, owner.path(), "consumer", "// test").unwrap();
    let family: BTreeSet<_> = std::iter::once("IBase".to_string())
        .chain(
            neocogi_com_core::NcComInterfaceCompanion::ALL
                .into_iter()
                .map(|kind| format!("IBase{}", kind.suffix())),
        )
        .collect();
    let generated: syn::File = syn::parse_str(&emission.generated.source).unwrap();
    let imports: Vec<_> = generated
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Use(item) => {
                let names: BTreeSet<_> = use_names(&item.tree).into_iter().collect();
                names.contains("IBase").then_some(names)
            }
            _ => None,
        })
        .collect();
    assert_eq!(imports.as_slice(), std::slice::from_ref(&family));
    let wrapper: syn::File = syn::parse_str(&emission.include_text).unwrap();
    let explicit: Vec<_> = wrapper
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Use(item) => {
                let names: BTreeSet<_> = use_names(&item.tree).into_iter().collect();
                names.contains("MODULE_ID").then_some(names)
            }
            _ => None,
        })
        .collect();
    let mut expected = family;
    expected.extend(["MODULE_ID", "ABI_FINGERPRINT", "MODULE_NAME"].map(str::to_owned));
    expected.insert(paths.dependency_namespace().unwrap().to_string());
    assert_eq!(explicit, [expected]);
}
