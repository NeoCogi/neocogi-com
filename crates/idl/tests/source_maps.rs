//! Source maps must resolve the actual formatted output, including derived helpers.

use std::path::Path;
use neocogi_idl::{generate_rust_with_source_map, parse_module_parts, resolve_modules, ModuleSource, RustSourceMap, SourceMapKind, SourceMapPosition};

/// One-based Unicode coordinates computed independently from the generator's parser spans.
fn position(source: &str, needle: &str) -> SourceMapPosition {
    let before = &source[..source.find(needle).unwrap_or_else(|| panic!("missing {needle:?}"))];
    SourceMapPosition {
        line: before.chars().filter(|character| *character == '\n').count() + 1,
        column: before.rsplit('\n').next().unwrap().chars().count() + 1,
    }
}

/// Inline source with non-ASCII text before a declaration exercises the column convention.
const TYPES: &str = r#"/* café 😀 */ #[version(1)] struct Size { width: u32, height: u32 }
#[version(1)] enum Alignment: u8 { Left = 0, Right = 1 }
#[version(1)] bitenum Flags: u16 { NONE = 0, ACTIVE = 1 }
#[version(1)] struct Changed { text: String }
#[version(1)] enum Completed { Accepted { text: String }, Cancelled }
"#;
/// Both authored and synthesized interfaces originate in this separate part.
const WIDGETS: &str = r#"#[version(1)] interface IStream<T> { fn post(&mut self, value: &T) -> Result<()>; }
#[version(1)] interface Textbox: IStream<Changed> {
    fn resize(&mut self, size: Size) -> Result<()>;
}
#[version(1)] interface Widgets {
    fn textbox(&self) -> Result<ComPtr<Textbox>>;
}
"#;

/// Builds identical semantic input in either filesystem discovery order.
fn graph(reverse: bool) -> neocogi_idl::ResolvedModules {
    let mut parts = [
        ModuleSource {
            path: Path::new("example/widgets/types.ncidl"),
            text: TYPES,
        },
        ModuleSource {
            path: Path::new("example/widgets/controls.ncidl"),
            text: WIDGETS,
        },
    ];
    if reverse {
        parts.reverse();
    }
    resolve_modules(vec![parse_module_parts(Path::new("example/widgets"), &parts).unwrap()]).unwrap()
}

/// Ordinary Rust definitions and members map through formatting and closed generic code.
#[test]
fn formatted_rust_and_derived_helpers_retain_exact_idl_origins() {
    let generated = generate_rust_with_source_map(&graph(false), "example.widgets").unwrap();
    let map = &generated.source_map;
    assert_eq!(map.generated_file, "example_widgets.rs");
    assert!(map.sources.iter().all(|file| file.location.is_none()));
    for (rust, idl, needle, path, kind) in [
        ("pub struct Size", TYPES, "Size", "types.ncidl", SourceMapKind::Struct),
        ("pub width:", TYPES, "width", "types.ncidl", SourceMapKind::Field),
        ("pub const Right:", TYPES, "Right", "types.ncidl", SourceMapKind::Constant),
        ("pub const ACTIVE:", TYPES, "ACTIVE", "types.ncidl", SourceMapKind::Constant),
        ("fn resize(", WIDGETS, "resize", "controls.ncidl", SourceMapKind::Method),
        ("pub trait Textbox", WIDGETS, "Textbox", "controls.ncidl", SourceMapKind::Trait),
        ("Cancelled", TYPES, "Cancelled", "types.ncidl", SourceMapKind::Variant),
    ] {
        let entry = map.lookup(position(&generated.source, rust)).unwrap_or_else(|| panic!("unmapped {rust}"));
        assert_eq!(entry.kind, kind, "{rust}");
        assert_eq!(map.sources[entry.original.source].path, format!("example/widgets/{path}"), "{rust}");
        assert_eq!(entry.original.range.start, position(idl, needle), "{rust}");
    }
    // Raw wire helpers come from the typed backend; source navigation maps
    // their authored value declaration rather than synthetic receiver methods.
    // The owned payload and borrowed wire field share the authored field location.
    let text_fields = map
        .mappings
        .iter()
        .filter(|entry| entry.kind == SourceMapKind::Field && entry.name.ends_with("::text") && entry.original.range.start == position(TYPES, "text: String"))
        .count();
    assert!(text_fields >= 1);
    // Module-only metadata has no declaration source; half-open ranges exclude EOF.
    assert!(map.lookup(position(&generated.source, "pub const MODULE_ID")).is_none());
    assert!(
        map.lookup(SourceMapPosition {
            line: generated.source.lines().count() + 1,
            column: 1
        })
        .is_none()
    );
    assert!(generated.source.contains("/// IDL source: `example/widgets/types.ncidl:1:"));
}

/// Repeated generation, part enumeration and JSON transport preserve the same mapping.
#[test]
fn source_maps_are_deterministic_and_round_trip() {
    let first = generate_rust_with_source_map(&graph(false), "example.widgets").unwrap();
    let second = generate_rust_with_source_map(&graph(true), "example.widgets").unwrap();
    assert_eq!(first, second);
    assert_eq!(neocogi_idl::generate_rust(&graph(false), "example.widgets").unwrap(), first.source);
    let json = serde_json::to_string_pretty(&first.source_map).unwrap();
    let decoded: RustSourceMap = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, first.source_map);
}

/// Imports point to the local authored use statement rather than a guessed dependency path.
#[test]
fn imported_bindings_keep_their_local_import_origin() {
    let dependency = neocogi_idl::parse_module(Path::new("example/value.ncidl"), "#[version(1)] struct Value { value: u32 }").unwrap();
    let text = "extern example.value;\nuse example.value::Value;\n#[version(1)] interface Reader { fn read(&self) -> Result<Value>; }";
    let consumer = neocogi_idl::parse_module(Path::new("example/consumer.ncidl"), text).unwrap();
    let graph = resolve_modules(vec![dependency, consumer]).unwrap();
    let generated = generate_rust_with_source_map(&graph, "example.consumer").unwrap();
    let imported = generated.source_map.lookup(position(&generated.source, "pub use")).unwrap();
    assert_eq!(imported.kind, SourceMapKind::Import);
    assert_eq!(imported.original.range.start, position(text, "Value;"));
    assert_eq!(generated.source_map.sources[imported.original.source].path, "example/consumer.ncidl");
}

/// Merging imports must preserve the origin of each identifier, including two
/// bindings to the same dependency authored in different source parts.
#[test]
fn split_imports_keep_each_identifier_and_do_not_change_the_abi() {
    let dependency = neocogi_idl::parse_module(
        Path::new("example/value.ncidl"),
        "#[version(1)] struct First { value: u32 } #[version(1)] struct Second { value: u32 }",
    )
    .unwrap();
    let a = "extern example.value; use example.value::First; #[version(1)] struct Pair { first: First, second: Second }";
    let z = "use example.value::Second;";
    let parts = [
        ModuleSource {
            path: Path::new("example/pair/a.ncidl"),
            text: a,
        },
        ModuleSource {
            path: Path::new("example/pair/z.ncidl"),
            text: z,
        },
    ];
    let consumer = parse_module_parts(Path::new("example/pair"), &parts).unwrap();
    let graph = resolve_modules(vec![dependency.clone(), consumer]).unwrap();
    let generated = generate_rust_with_source_map(&graph, "example.pair").unwrap();
    for (name, text, path) in [("First", a, "a.ncidl"), ("Second", z, "z.ncidl")] {
        let entry = generated
            .source_map
            .mappings
            .iter()
            .find(|entry| entry.kind == SourceMapKind::Import && entry.name.ends_with(name))
            .unwrap();
        let file = &generated.source_map.sources[entry.original.source];
        assert_eq!(file.module, "example.pair");
        assert_eq!(file.path, format!("example/pair/{path}"));
        assert_eq!(entry.original.range.start, position(text, name));
    }
    // Grouping both names in one statement changes provenance, not the contract.
    let grouped = neocogi_idl::parse_module(
        Path::new("example/pair.ncidl"),
        "extern example.value; use example.value::{First, Second}; #[version(1)] struct Pair { first: First, second: Second }",
    )
    .unwrap();
    let grouped = resolve_modules(vec![dependency, grouped]).unwrap();
    assert_eq!(
        graph.module("example.pair").unwrap().abi_fingerprint,
        grouped.module("example.pair").unwrap().abi_fingerprint
    );
}

/// Physical provider roots survive transitive graph exchange. All mapped files
/// must exist after an ordinary Cargo build, including methods specialized from
/// a grandparent template and the bundled Option alternatives.
#[test]
fn cargo_maps_resolve_transitive_templates_and_bundled_sources() {
    use std::{fs, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        "[workspace]\nresolver='2'\nmembers=['maps-provider','maps-bridge','maps-host']\n",
    )
    .unwrap();
    let first = "extern origin.bridge; use origin.bridge::{Derived, First};\n#[version(1)] interface Counter: Derived<u32> {}\n#[version(1)] struct Item { value: Option<u32> }\n#[version(1)] interface Plain { fn read(&self) -> Result<u32>; }\n";
    let second = "use origin.bridge::Second;\n#[version(1)] struct Both { first: First, second: Second }\n";
    for (package, module, input, source, dependency) in [
        (
            "maps-provider",
            "origin.generic",
            "origin/generic.ncidl",
            "#[version(1)] interface Value<T> { fn get(&self) -> Result<T>; }",
            None,
        ),
        (
            "maps-bridge",
            "origin.bridge",
            "origin/bridge.ncidl",
            "extern origin.generic; use origin.generic::Value; #[version(1)] interface Derived<T>: Value<T> {} #[version(1)] struct First { value: u32 } #[version(1)] struct Second { value: u32 }",
            Some("maps-provider"),
        ),
        ("maps-host", "proof.maps", "proof/maps", first, Some("maps-bridge")),
    ] {
        let directory = project.path().join(package);
        fs::create_dir_all(directory.join("src")).unwrap();
        let mut manifest = format!(
            "[package]\nname={package:?}\nversion='0.0.0'\nedition='2024'\nlinks={:?}\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            neocogi_idl::cargo::contract_links_name(module),
            root.join("crates/com/api")
        );
        if let Some(dependency) = dependency {
            manifest.push_str(&format!("{dependency}={{path='../{dependency}'}}\n"));
        }
        manifest.push_str(&format!("[build-dependencies]\nneocogi-idl={{path={:?}}}\n", root.join("crates/com/idl")));
        fs::write(directory.join("Cargo.toml"), manifest).unwrap();
        let constructor = if package == "maps-host" { "directory" } else { "new" };
        fs::write(
            directory.join("build.rs"),
            format!(
                "fn main() {{ neocogi_idl::cargo::Module::{constructor}(\"idl\", {input:?}).rust_path({:?}).build().unwrap(); }}",
                package.replace('-', "_")
            ),
        )
        .unwrap();
        fs::write(
            directory.join("src/lib.rs"),
            format!("include!(concat!(env!(\"OUT_DIR\"), \"/{}.include.rs\"));", module.replace('.', "_")),
        )
        .unwrap();
        let path = directory.join("idl").join(input);
        if package == "maps-host" {
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("a.ncidl"), source).unwrap();
            fs::write(path.join("z.ncidl"), second).unwrap();
        } else {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
    }
    // The real generated consumer reads value/interface metadata and dispatches
    // through COM; successful source-map/text assertions alone are insufficient.
    let consumer_rust = project.path().join("maps-host/src/lib.rs");
    let mut consumer_source = fs::read_to_string(&consumer_rust).unwrap();
    consumer_source.push_str(
        r#"
#[cfg(test)]
mod metadata_calls {
    use super::*;
    #[neocogi_com::com_object(Plain)]
    struct Provider;
    impl Plain for Provider {
        fn read(&self) -> Result<u32, neocogi_com::NcStatus> { Ok(37) }
    }
    #[test]
    fn metadata_and_actual_com_call_agree() {
        let value: neocogi_com::idl_metadata::TypeMetadata = Item::IDL_TYPE;
        assert_eq!(value.qualified_name, "proof.maps.Item");
        let neocogi_com::idl_metadata::TypeShape::Struct(fields) = value.shape else { panic!("record shape") };
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "value");
        let neocogi_com::idl_metadata::ValueType::Named(name) = fields[0].ty else { panic!("Option metadata must retain its named enum") };
        assert!(name.starts_with("neocogi.lang.OptionInstance"));
        let interface: neocogi_com::idl_metadata::InterfaceMetadata = PlainAbi::IDL_INTERFACE;
        assert!(interface.rust_abi.ends_with("::PlainAbi"));
        assert_eq!(interface.chain.len(), PlainAbi::INTERFACE_CHAIN.len());
        let method: &neocogi_com::idl_metadata::MethodMetadata = &interface.methods[0];
        assert_eq!(method.name, "read");
        assert!(method.parameters.is_empty());
        assert_eq!(method.returns, neocogi_com::idl_metadata::MethodType::Value(neocogi_com::idl_metadata::ValueType::Scalar("u32")));
        let owner = Provider::from_value(Provider);
        let projection = owner.query::<PlainAbi>().unwrap();
        assert_eq!(projection.read(), Ok(37));
    }
}
"#,
    );
    fs::write(consumer_rust, consumer_source).unwrap();
    let output = Command::new("cargo")
        .args(["test", "--lib", "--offline", "--message-format=json", "-p", "maps-host", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| root.join("target").into_os_string()),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("test metadata_calls::metadata_and_actual_com_call_agree ... ok"),
        "the generated consumer must execute its real metadata/COM test"
    );
    let out_dir = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|message| message["reason"] == "build-script-executed" && message["package_id"].as_str().is_some_and(|id| id.contains("/maps-host#")))
        .unwrap()["out_dir"]
        .as_str()
        .map(std::path::PathBuf::from)
        .unwrap();
    let map: RustSourceMap = serde_json::from_str(&fs::read_to_string(out_dir.join("proof_maps.rs.map")).unwrap()).unwrap();
    assert_eq!(map.schema, 2);
    for file in &map.sources {
        assert!(out_dir.join(file.location.as_ref().unwrap()).is_file(), "{}:{}", file.module, file.path);
    }
    let method = map
        .mappings
        .iter()
        .find(|entry| entry.kind == SourceMapKind::Method && entry.name.ends_with("::get"))
        .unwrap();
    let file = &map.sources[method.original.source];
    assert_eq!(file.module, "origin.generic");
    assert_eq!(
        out_dir.join(file.location.as_ref().unwrap()).canonicalize().unwrap(),
        project.path().join("maps-provider/idl/origin/generic.ncidl").canonicalize().unwrap()
    );
    let variant = map
        .mappings
        .iter()
        .find(|entry| entry.kind == SourceMapKind::Variant && entry.name.ends_with("::Some"))
        .unwrap();
    let file = &map.sources[variant.original.source];
    assert_eq!(file.module, "neocogi.lang");
    assert!(out_dir.join(file.location.as_ref().unwrap()).starts_with(&out_dir));
    assert!(
        fs::read_to_string(out_dir.join(file.location.as_ref().unwrap()))
            .unwrap()
            .contains("enum Option<T>")
    );
    for (name, text, part) in [("First", first, "a.ncidl"), ("Second", second, "z.ncidl")] {
        let entry = map
            .mappings
            .iter()
            .find(|entry| entry.kind == SourceMapKind::Import && entry.name.ends_with(name))
            .unwrap();
        assert_eq!(map.sources[entry.original.source].path, format!("proof/maps/{part}"));
        assert_eq!(entry.original.range.start, position(text, name));
    }
    let rust = fs::read_to_string(out_dir.join("proof_maps.rs")).unwrap();
    assert!(rust.contains("Closed IDL type: `origin::generic::Value<u32>`"));
    assert!(rust.contains("not reversibly mangled source text"));
}

/// Looks up a concrete impl's metadata through parsed Rust, then checks its entire
/// initializer token tree against an independently specified authored identifier.
fn assert_metadata_origin(generated: &neocogi_idl::GeneratedRust, owner: &str, module: &str, path: &str, text: &str, identifier: &str) {
    use quote::ToTokens;
    let syntax = syn::parse_file(&generated.source).unwrap();
    let constants = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item) if matches!(&*item.self_ty, syn::Type::Path(ty) if ty.path.is_ident(owner)) => Some(item),
            _ => None,
        })
        .flat_map(|item| &item.items)
        .filter_map(|item| match item {
            syn::ImplItem::Const(item) if item.ident == "IDL_TYPE" || item.ident == "IDL_INTERFACE" => Some(item),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(constants.len(), 1, "one metadata constant for {owner}");
    let constant = constants[0];
    let start = position(text, identifier);
    let end = SourceMapPosition {
        line: start.line,
        column: start.column + identifier.chars().count(),
    };
    // Every nested token, including array/struct initializer bodies, inherits the
    // declaration constant's cause. This also covers punctuation at closing braces.
    let mut pending = vec![constant.expr.to_token_stream()];
    let mut points = vec![constant.ident.span().start()];
    while let Some(tokens) = pending.pop() {
        for token in tokens {
            points.push(token.span().start());
            if let proc_macro2::TokenTree::Group(group) = token {
                points.push(group.span_close().start());
                pending.push(group.stream());
            }
        }
    }
    assert!(points.len() > 10, "exercise real initializer syntax");
    for point in points {
        let at = SourceMapPosition {
            line: point.line,
            column: point.column + 1,
        };
        let entry = generated.source_map.lookup(at).unwrap_or_else(|| panic!("unmapped {owner} metadata at {at:?}"));
        let file = &generated.source_map.sources[entry.original.source];
        assert_eq!((file.module.as_str(), file.path.as_str()), (module, path), "{owner} at {at:?}");
        assert_eq!(entry.original.range, neocogi_idl::SourceMapRange { start, end }, "{owner} at {at:?}");
        assert_eq!(entry.kind, SourceMapKind::Constant, "{owner} at {at:?}");
    }
}

/// Ordinary structs, scalar/native enums, flags and interface metadata are part
/// of their own fragment, even when many declarations share a module/source part.
#[test]
fn declaration_metadata_and_nested_initializers_have_exact_identifier_origins() {
    let graph = graph(false);
    let generated = generate_rust_with_source_map(&graph, "example.widgets").unwrap();
    for (owner, text, identifier, path) in [
        ("Size", TYPES, "Size", "types.ncidl"),
        ("Alignment", TYPES, "Alignment", "types.ncidl"),
        ("Flags", TYPES, "Flags", "types.ncidl"),
        ("Changed", TYPES, "Changed", "types.ncidl"),
        ("Completed", TYPES, "Completed", "types.ncidl"),
        ("TextboxAbi", WIDGETS, "Textbox", "controls.ncidl"),
        ("WidgetsAbi", WIDGETS, "Widgets", "controls.ncidl"),
    ] {
        assert_metadata_origin(&generated, owner, "example.widgets", &format!("example/widgets/{path}"), text, identifier);
    }
    // Adding metadata coverage must not steal authored field/variant/method
    // origins or map synthetic header constants and the half-open final boundary.
    for constant in ["MODULE_ID", "ABI_FINGERPRINT", "MODULE_NAME"] {
        assert!(
            generated
                .source_map
                .lookup(position(&generated.source, &format!("pub const {constant}")))
                .is_none()
        );
    }
    assert!(
        generated
            .source_map
            .lookup(SourceMapPosition {
                line: generated.source.lines().count() + 1,
                column: 1
            })
            .is_none()
    );
}

/// Templates without members still have a bound defining identifier. Definition
/// origins may move independently of complete application spans used by diagnostics.
#[test]
fn imported_and_phantom_specializations_separate_definitions_from_use_sites() {
    let templates = "/* café 😀 */ #[version(1)] struct Stored<T> { item: T }\n#[version(1)] interface Phantom<T> {}\n#[version(1)] interface Getter<T> { fn get(&self) -> Result<T>; }";
    let consumer = "extern origins.templates; use origins.templates::{Stored, Phantom, Getter};\n#[version(1)] struct Size { width: u32 }\n#[version(1)] struct Bundle { stored: Stored<Size> }\n#[version(1)] interface Reader: Getter<Size> + Phantom<Size> {}";
    let mut first_fingerprint = None;
    // Shifts in both authored files and the template's physical path must change
    // provenance, while preserving semantic identity and deterministic transport.
    for (prefix, template_path) in [("", "origins/templates/original.ncidl"), ("\n\n/* moved */\n", "origins/templates/part.ncidl")] {
        let template_text = format!("{prefix}{templates}");
        let consumer_text = format!("{prefix}{consumer}");
        let template = parse_module_parts(
            Path::new("origins/templates"),
            &[ModuleSource {
                path: Path::new(template_path),
                text: &template_text,
            }],
        )
        .unwrap();
        let source = neocogi_idl::parse_module(Path::new("origins/consumer.ncidl"), &consumer_text).unwrap();
        let graph = resolve_modules(vec![source, template]).unwrap();
        let module = graph.module("origins.consumer").unwrap();
        let generated = generate_rust_with_source_map(&graph, "origins.consumer").unwrap();
        let again = generate_rust_with_source_map(&graph, "origins.consumer").unwrap();
        assert_eq!(generated, again);
        let decoded: RustSourceMap = serde_json::from_str(&serde_json::to_string(&generated.source_map).unwrap()).unwrap();
        assert_eq!(decoded, generated.source_map);
        if let Some(fingerprint) = first_fingerprint {
            assert_eq!(fingerprint, module.abi_fingerprint);
        } else {
            first_fingerprint = Some(module.abi_fingerprint);
        }
        let mut closed = 0;
        for declaration in &module.source.declarations {
            let Some(instance) = &declaration.specialization else { continue };
            let name = instance.template.segments.last().unwrap();
            let owner = if matches!(declaration.kind, neocogi_idl::DeclarationKind::Interface(_)) {
                format!("{}Abi", declaration.name)
            } else {
                declaration.name.clone()
            };
            assert_metadata_origin(&generated, &owner, "origins.templates", template_path, &template_text, name);
            let application = format!("{name}<Size>");
            let start = position(&consumer_text, &application);
            assert_eq!(declaration.span.module(), "origins.consumer");
            assert_eq!(declaration.span.path(), "origins/consumer.ncidl");
            assert_eq!((declaration.span.start().line, declaration.span.start().column), (start.line, start.column));
            assert_eq!(
                (declaration.span.end().line, declaration.span.end().column),
                (start.line, start.column + application.len())
            );
            // Root/client/wire definitions use the template cause as well; none
            // may fall back to the consumer's application expression.
            let fragment = generated
                .source_map
                .mappings
                .iter()
                .find(|entry| entry.kind == SourceMapKind::Declaration && entry.name == declaration.name)
                .unwrap();
            assert_eq!(generated.source_map.sources[fragment.original.source].path, template_path);
            assert_eq!(fragment.original.range.start, position(&template_text, name));
            closed += 1;
        }
        assert_eq!(closed, 3, "include memberless phantom, stored value and interface");
        for (needle, identifier) in [("pub item:", "item"), ("fn get(", "get")] {
            let entry = generated.source_map.lookup(position(&generated.source, needle)).unwrap();
            assert_eq!(entry.original.range.start, position(&template_text, identifier));
        }
    }
    // A real malformed application diagnostic still blames the complete use,
    // rather than the template identifier chosen only for generated definitions.
    let bad = consumer.replace("Stored<Size>", "Stored<Size, Size>");
    let error = resolve_modules(vec![
        neocogi_idl::parse_module(Path::new("origins/templates.ncidl"), templates).unwrap(),
        neocogi_idl::parse_module(Path::new("origins/consumer.ncidl"), &bad).unwrap(),
    ])
    .unwrap_err();
    let neocogi_idl::ModuleGraphError::Resolution(error) = error else {
        panic!("expected semantic diagnostics")
    };
    let diagnostic = &error.diagnostics[0];
    let primary = diagnostic.labels.iter().find(|label| label.style == neocogi_idl::LabelStyle::Primary).unwrap();
    let start = position(&bad, "Stored<Size, Size>");
    assert_eq!((primary.span.start().line, primary.span.start().column), (start.line, start.column));
    assert_eq!(
        (primary.span.end().line, primary.span.end().column),
        (start.line, start.column + "Stored<Size, Size>".len())
    );
}
