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
#[version(1)] #[message] struct Changed { text: String }
#[version(1)] #[message] enum Completed { Accepted { text: String }, Cancelled }
"#;
/// Both authored and synthesized interfaces originate in this separate part.
const WIDGETS: &str = r#"#[version(1)] #[observer] interface Textbox {
    fn resize(&mut self, size: Size) -> Result<()>;
    changed: Changed;
    completed: Completed;
}
#[version(1)] factory Widgets {
    fn textbox(&self, observer: Option<&TextboxObserver>) -> Result<Textbox>;
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

/// Definitions and members map through formatting, ordinary macros and derived observer code.
#[test]
fn formatted_rust_and_derived_helpers_retain_exact_idl_origins() {
    let generated = generate_rust_with_source_map(&graph(false), "example.widgets").unwrap();
    let map = &generated.source_map;
    assert_eq!(map.generated_file, "example_widgets.rs");
    assert_eq!(map.source_root, None);
    for (rust, idl, needle, path, kind) in [
        ("pub struct Size", TYPES, "Size", "types.ncidl", SourceMapKind::Struct),
        ("pub width:", TYPES, "width", "types.ncidl", SourceMapKind::Field),
        ("Right = 1", TYPES, "Right", "types.ncidl", SourceMapKind::Variant),
        ("ACTIVE = 1", TYPES, "ACTIVE", "types.ncidl", SourceMapKind::Variant),
        ("fn resize(", WIDGETS, "resize", "controls.ncidl", SourceMapKind::Method),
        ("pub trait TextboxObserver", WIDGETS, "Textbox", "controls.ncidl", SourceMapKind::Trait),
        ("Changed(Changed)", WIDGETS, "changed:", "controls.ncidl", SourceMapKind::Variant),
        ("pub struct __ChangedWire", TYPES, "Changed", "types.ncidl", SourceMapKind::Struct),
        ("pub struct __CompletedCase0", TYPES, "Accepted", "types.ncidl", SourceMapKind::Struct),
    ] {
        let entry = map.lookup(position(&generated.source, rust)).unwrap_or_else(|| panic!("unmapped {rust}"));
        assert_eq!(entry.kind, kind, "{rust}");
        assert_eq!(entry.original.path, format!("example/widgets/{path}"), "{rust}");
        assert_eq!(entry.original.range.start, position(idl, needle), "{rust}");
    }
    // A location inside the generated callback body resolves to the output declaration.
    let callback = map
        .mappings
        .iter()
        .find(|entry| entry.kind == SourceMapKind::Method && entry.name.contains("__TextboxObserver") && entry.name.ends_with("::changed"))
        .unwrap();
    let inside_body = SourceMapPosition {
        line: callback.generated.end.line,
        column: callback.generated.end.column - 1,
    };
    let mapped_body = map.lookup(inside_body).unwrap();
    assert_eq!(mapped_body.kind, SourceMapKind::Method);
    assert_eq!(mapped_body.original.range.start, position(WIDGETS, "changed:"));
    // The owned payload and borrowed wire field share the authored field location.
    let text_fields = map
        .mappings
        .iter()
        .filter(|entry| entry.kind == SourceMapKind::Field && entry.name.ends_with("::text") && entry.original.range.start == position(TYPES, "text: String"))
        .count();
    assert!(text_fields >= 2);
    // Metadata has no declaration source, and a half-open range does not swallow EOF.
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
    assert_eq!(imported.original.range.start, position(text, "use example"));
    assert_eq!(imported.original.path, "example/consumer.ncidl");
}
