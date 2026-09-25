use std::path::Path;

use neocogi_idl::{ModuleSource, parse_module, parse_module_parts, resolve_modules};

/// Parses a fixture while preserving path-derived identity.
fn module(path: &str, source: &str) -> neocogi_idl::Module {
    parse_module(Path::new(path), source).unwrap()
}

/// Only marked data types can be connected; authors never import a lease to declare an output.
#[test]
fn outputs_require_concrete_messages() {
    let valid = module(
        "example/events.ncidl",
        "#[version(1)] #[message] struct Changed { value: u32 } #[version(1)] interface ISource { changed: Changed; }",
    );
    resolve_modules(vec![valid]).expect("message output without authored transport dependencies");
    for declaration in ["interface Changed {}", "struct Changed { value: u32 }"] {
        let source = module(
            "example/events.ncidl",
            &format!("#[version(1)] {declaration} #[version(1)] interface ISource {{ changed: Changed; }}"),
        );
        let error = resolve_modules(vec![source]).unwrap_err().to_string();
        assert!(error.contains("must name a concrete message"), "{error}");
    }
}

/// Resolves exact cross-module declarations and fingerprints.
#[test]
fn resolves_a_contract_dependency_graph() {
    let render = module(
        "neocogi/render.ncidl",
        r#"
            #[version(1)]
            struct Size { width: u32, height: u32 }

            #[version(1)]
            interface ITexture { fn size(&self) -> Result<Size>; }
        "#,
    );
    let ui = module(
        "neocogi/ui.ncidl",
        r#"
            extern neocogi.render;
            use neocogi.render::{ITexture, Size};

            #[version(1)]
            interface Button {
                fn set_image(&mut self, image: &ITexture) -> Result<()>;
            }

            #[version(1)]
            factory UiFactory {
                fn create_button(&self, size: Size) -> Result<Button>;
            }
        "#,
    );

    let modules = resolve_modules(vec![ui, render]).unwrap();
    let ui = modules.module("neocogi.ui").unwrap();
    let render = modules.module("neocogi.render").unwrap();
    assert_eq!(ui.dependencies.len(), 1);
    assert_eq!(ui.dependencies[0].module_id, render.module_id);
    assert_eq!(ui.dependencies[0].abi_fingerprint, render.abi_fingerprint);
    assert_eq!(ui.declaration("Button").unwrap().qualified_name, "neocogi.ui.Button");
    assert_eq!(ui.fingerprint_hex().len(), 64);
    assert_ne!(
        render.declaration("Size").unwrap().abi_signature,
        render.declaration("ITexture").unwrap().abi_signature,
        "declaration signatures must retain exact type identity inside one module"
    );
}

/// Confirms symbol imports cannot create hidden contract dependency edges.
#[test]
fn requires_extern_before_use() {
    let render = module("neocogi/render.ncidl", "#[version(1)] struct Size { width: u32 }");
    let ui = module("neocogi/ui.ncidl", "use neocogi.render::Size; #[version(1)] struct Widget { size: Size }");

    let error = resolve_modules(vec![ui, render]).unwrap_err().to_string();
    assert!(error.contains("error[EIDL035]"), "{error}");
    assert!(error.contains("requires `extern neocogi.render;`"), "{error}");
}

/// Confirms an external dependency permits qualified references without also
/// introducing an unqualified source name.
#[test]
fn resolves_qualified_types_through_extern() {
    let render = module("neocogi/render.ncidl", "#[version(1)] struct Size { width: u32 }");
    let ui = module(
        "neocogi/ui.ncidl",
        "extern neocogi.render; #[version(1)] struct Widget { size: neocogi::render::Size }",
    );

    let modules = resolve_modules(vec![ui, render]).unwrap();
    assert_eq!(modules.module("neocogi.ui").unwrap().dependencies.len(), 1);
}

/// Confirms comments and whitespace do not change an ABI fingerprint.
#[test]
fn ignores_documentation_and_formatting_in_fingerprints() {
    let compact = module("neocogi/ui.ncidl", "#[version(1)] struct Size { width: u32, height: u32 }");
    let documented = module(
        "neocogi/ui.ncidl",
        r#"
            /// A documented size.
            #[version(1)]
            struct Size {
                /// Horizontal extent.
                width: u32,
                height: u32,
            }
        "#,
    );
    let first = resolve_modules(vec![compact]).unwrap();
    let second = resolve_modules(vec![documented]).unwrap();
    assert_eq!(first.modules()[0].abi_fingerprint, second.modules()[0].abi_fingerprint);
}

/// Confirms the build tool fingerprints the complete typed contract surface.
#[test]
fn fingerprints_typed_layout_and_callable_changes() {
    /// Resolves one isolated module and returns its generator-owned digest.
    fn fingerprint(source: &str) -> [u8; 32] {
        resolve_modules(vec![module("example/contract.ncidl", source)]).unwrap().modules()[0].abi_fingerprint
    }

    assert_ne!(
        fingerprint("#[version(1)] struct Value { number: u32 }"),
        fingerprint("#[version(1)] struct Value { number: u64 }")
    );
    assert_ne!(
        fingerprint("#[version(1)] enum State: u8 { Ready = 1 }"),
        fingerprint("#[version(1)] enum State: u16 { Ready = 1 }")
    );
    assert_ne!(
        fingerprint("#[version(1)] interface IService { fn load(&self, key: u32) -> Result<u32>; }"),
        fingerprint("#[version(1)] interface IService { fn load(&mut self, key: u32) -> Result<u32>; }")
    );
    assert_ne!(
        fingerprint("#[version(1)] interface IService { fn load(&self, key: u32) -> Result<u32>; }"),
        fingerprint("#[version(1)] interface IService { fn load(&self, key: u64) -> Result<u32>; }")
    );
    assert_ne!(
        fingerprint("#[version(1)] interface IBase {} #[version(1)] interface IService: IBase {}"),
        fingerprint("#[version(1)] interface IBase {} #[version(1)] interface IService {}")
    );
    assert_ne!(
        fingerprint("#[threading(shared)] #[version(1)] factory Service {}"),
        fingerprint("#[threading(local)] #[version(1)] factory Service {}")
    );
}

/// Confirms canonical top-level sorting does not erase ABI-significant order
/// inside declarations.
#[test]
fn fingerprints_member_order() {
    /// Resolves one isolated source and returns its canonical ABI fingerprint.
    fn fingerprint(source: &str) -> [u8; 32] {
        resolve_modules(vec![module("example/order.ncidl", source)]).unwrap().modules()[0].abi_fingerprint
    }

    assert_ne!(
        fingerprint("#[version(1)] struct Pair { first: u32, second: u64 }"),
        fingerprint("#[version(1)] struct Pair { second: u64, first: u32 }")
    );
    assert_ne!(
        fingerprint("#[version(1)] enum State: u8 { First = 0, Second = 1 }"),
        fingerprint("#[version(1)] enum State: u8 { Second = 1, First = 0 }")
    );
    assert_ne!(
        fingerprint("#[version(1)] interface IService { fn first(&self) -> Result<()>; fn second(&self) -> Result<()>; }"),
        fingerprint("#[version(1)] interface IService { fn second(&self) -> Result<()>; fn first(&self) -> Result<()>; }")
    );
    assert_ne!(
        fingerprint("#[version(1)] interface IService { fn call(&self, first: u32, second: u64) -> Result<()>; }"),
        fingerprint("#[version(1)] interface IService { fn call(&self, second: u64, first: u32) -> Result<()>; }")
    );
    assert_ne!(
        fingerprint("#[version(1)] interface IFirst {} #[version(1)] interface ISecond {} #[version(1)] interface IService: IFirst + ISecond {}"),
        fingerprint("#[version(1)] interface IFirst {} #[version(1)] interface ISecond {} #[version(1)] interface IService: ISecond + IFirst {}")
    );
}

/// Confirms declaration signatures track the module revision without losing
/// the identity of individual types within that module.
#[test]
fn declaration_signatures_are_typed_and_revision_scoped() {
    let first = resolve_modules(vec![module(
        "example/contract.ncidl",
        "#[version(1)] struct Left { value: u32 } #[version(1)] struct Right { value: u32 }",
    )])
    .unwrap();
    let revised = resolve_modules(vec![module(
        "example/contract.ncidl",
        "#[version(1)] struct Left { value: u64 } #[version(1)] struct Right { value: u32 }",
    )])
    .unwrap();
    let first = first.module("example.contract").unwrap();
    let revised = revised.module("example.contract").unwrap();

    assert_ne!(
        first.declaration("Left").unwrap().abi_signature,
        first.declaration("Right").unwrap().abi_signature
    );
    assert_ne!(
        first.declaration("Left").unwrap().abi_signature,
        revised.declaration("Left").unwrap().abi_signature
    );
    assert_ne!(
        first.declaration("Right").unwrap().abi_signature,
        revised.declaration("Right").unwrap().abi_signature
    );
}

/// Confirms object inputs cannot imply an ownership transfer.
#[test]
fn rejects_ambiguous_object_parameter_ownership() {
    let source = module(
        "neocogi/ui.ncidl",
        r#"
            #[version(1)]
            interface Button {}

            #[version(1)]
            factory UiFactory {
                fn adopt(&self, button: Button) -> Result<()>;
            }
        "#,
    );
    let error = resolve_modules(vec![source]).unwrap_err();
    assert!(error.to_string().contains("must borrow interface, class, and factory"));
}

/// Confirms module imports form an acyclic native dependency graph.
#[test]
fn rejects_contract_dependency_cycles() {
    let first = module(
        "example/first.ncidl",
        r#"
            extern example.second;
            use example.second::Second;
            #[version(1)]
            struct First { second: Second }
        "#,
    );
    let second = module(
        "example/second.ncidl",
        r#"
            extern example.first;
            use example.first::First;
            #[version(1)]
            struct Second { first: First }
        "#,
    );
    let error = resolve_modules(vec![first, second]).unwrap_err();
    assert!(error.to_string().contains("dependency cycle"));
}

/// Confirms a dependency ABI change propagates into dependents.
#[test]
fn fingerprints_exact_dependency_revisions() {
    let ui = module(
        "neocogi/ui.ncidl",
        r#"
            extern neocogi.render;
            use neocogi.render::Size;
            #[version(1)]
            struct WidgetOptions { size: Size }
        "#,
    );
    let render_v1 = module("neocogi/render.ncidl", "#[version(1)] struct Size { width: u32 }");
    let render_v2 = module("neocogi/render.ncidl", "#[version(1)] struct Size { width: u32, height: u32 }");

    let first = resolve_modules(vec![ui.clone(), render_v1]).unwrap();
    let second = resolve_modules(vec![ui, render_v2]).unwrap();
    assert_ne!(
        first.module("neocogi.ui").unwrap().abi_fingerprint,
        second.module("neocogi.ui").unwrap().abi_fingerprint
    );
}

/// Confirms caller file discovery order is not an ABI input.
#[test]
fn ignores_module_input_order_in_fingerprints() {
    let geometry = module("example/geometry.ncidl", "#[version(1)] struct Point { x: f32, y: f32 }");
    let paint = module("example/paint.ncidl", "#[version(1)] struct Color { rgba: u32 }");
    let canvas = module(
        "example/canvas.ncidl",
        r#"
            extern example.geometry;
            extern example.paint;
            use example.paint::Color;
            use example.geometry::Point;
            #[version(1)] struct Vertex { point: Point, color: Color }
        "#,
    );

    let first = resolve_modules(vec![canvas.clone(), geometry.clone(), paint.clone()]).unwrap();
    let second = resolve_modules(vec![paint, canvas, geometry]).unwrap();
    assert_eq!(
        first.module("example.canvas").unwrap().abi_fingerprint,
        second.module("example.canvas").unwrap().abi_fingerprint
    );
}

/// Confirms explicit declaration IDs remain unique across a complete graph.
#[test]
fn rejects_cross_module_identity_collisions() {
    let first = module(
        "example/first.ncidl",
        r#"#[id(1, "9be73186-f123-47bc-8140-4a57825eeb21")] struct First { value: u32 }"#,
    );
    let second = module(
        "example/second.ncidl",
        r#"#[id(1, "9be73186-f123-47bc-8140-4a57825eeb21")] struct Second { value: u32 }"#,
    );

    let error = resolve_modules(vec![first, second]).unwrap_err();
    assert!(error.to_string().contains("reuses the ID assigned"));
}

/// Confirms enum value collisions identify both cases instead of only naming
/// the later value.
#[test]
fn reports_both_locations_for_duplicate_enum_discriminants() {
    let source = module("neocogi/ui.ncidl", "#[version(1)]\nenum State: u8 {\n    Ready = 1,\n    Running = 1,\n}");
    let error = resolve_modules(vec![source]).unwrap_err().to_string();

    assert!(error.contains("error[EIDL046]: enum State variant Running repeats discriminant 1"), "{error}");
    assert!(error.contains("neocogi/ui.ncidl:4:5"), "{error}");
    assert!(error.contains("neocogi/ui.ncidl:3:5"), "{error}");
    assert!(error.contains("discriminant first assigned here"), "{error}");
}

/// Confirms type-category errors point to both the invalid use and the
/// declaration that determines the actual category.
#[test]
fn reports_definition_for_mismatched_field_type() {
    let parts = [
        ModuleSource {
            path: Path::new("neocogi/ui/rendering.ncidl"),
            text: "#[version(1)] interface ITexture {}",
        },
        ModuleSource {
            path: Path::new("neocogi/ui/assets.ncidl"),
            text: "#[version(1)] struct Asset { texture: ITexture }",
        },
    ];
    let source = parse_module_parts(Path::new("neocogi/ui"), &parts).unwrap();
    let error = resolve_modules(vec![source]).unwrap_err().to_string();

    assert!(
        error.contains("error[EIDL060]: struct Asset field texture must use a copied value type"),
        "{error}"
    );
    assert!(error.contains("neocogi/ui/assets.ncidl:1:39"), "{error}");
    assert!(error.contains("neocogi/ui/rendering.ncidl:1:25"), "{error}");
    assert!(error.contains("found interface"), "{error}");
    assert!(error.contains("copied struct, enum, or bit enum"), "{error}");
}

/// Confirms source and module enumeration permutations produce exactly the
/// same semantic diagnostic text.
#[test]
fn semantic_diagnostics_are_independent_of_input_order() {
    let first = module("example/first.ncidl", "#[version(1)] struct Value { missing: Missing }");
    let second = module("example/second.ncidl", "#[version(1)] struct Other { value: u32 }");

    let forward = resolve_modules(vec![first.clone(), second.clone()]).unwrap_err().to_string();
    let reverse = resolve_modules(vec![second, first]).unwrap_err().to_string();

    assert_eq!(forward, reverse);
    assert!(forward.contains("error[EIDL070]: unknown IDL type \"Missing\""), "{forward}");
}

/// Confirms unrelated semantic failures are reported in one compiler run and
/// rendered in source order rather than validation traversal order.
#[test]
fn collects_independent_type_errors() {
    let source = module(
        "example/errors.ncidl",
        r#"
            #[version(1)] struct Alpha { missing: Missing }
            #[version(1)] struct Beta { text: String }
        "#,
    );
    let error = resolve_modules(vec![source]).unwrap_err();
    let rendered = error.to_string();

    assert_eq!(error.diagnostics.len(), 2);
    assert!(rendered.contains("unknown IDL type \"Missing\""), "{rendered}");
    assert!(rendered.contains("struct Beta field text must use a copied value type"), "{rendered}");
    assert!(rendered.find("unknown IDL type").unwrap() < rendered.find("struct Beta field").unwrap());
}

/// Confirms recursive copied records fail in the IDL compiler with their
/// complete layout cycle instead of failing later in generated Rust.
#[test]
fn rejects_recursive_value_layouts() {
    let source = module(
        "example/recursive.ncidl",
        r#"
            #[version(1)] struct First { second: Option<Second> }
            #[version(1)] struct Second { first: First }
        "#,
    );
    let error = resolve_modules(vec![source]).unwrap_err().to_string();

    assert!(
        error.contains("error[EIDL063]: recursive copied-value layout: First -> Second -> First"),
        "{error}"
    );
    assert!(error.contains("this field closes the recursive layout"), "{error}");
    assert!(error.contains("finite inline size"), "{error}");
}

/// Confirms explicit IDs preserve exact UUID bytes and declaration names.
#[test]
fn preserves_explicit_identity_without_rewriting_names() {
    let source = module(
        "neocogi/ui.ncidl",
        r#"
            #[id(7, "9be73186-f123-47bc-8140-4a57825eeb21")]
            interface IWidget {}
        "#,
    );
    let modules = resolve_modules(vec![source]).unwrap();
    let identity = modules.module("neocogi.ui").unwrap().declaration("IWidget").unwrap();
    assert_eq!(identity.qualified_name, "neocogi.ui.IWidget");
    assert_eq!(identity.version, 7);
    assert_eq!(identity.id.to_string(), "9be73186-f123-47bc-8140-4a57825eeb21");
}

/// Confirms dense domains reject gaps before generated ordinal tables can be used.
#[test]
fn rejects_sparse_dense_enums() {
    let source = module(
        "neocogi/ui.ncidl",
        r#"
            #[version(1)]
            #[dense]
            enum Direction: u8 { Horizontal = 0, Vertical = 2 }
        "#,
    );
    let error = resolve_modules(vec![source]).unwrap_err();
    assert!(error.to_string().contains("must use discriminant 1"));
}

/// Confirms dense domains remain simple unsigned unit-value axes.
#[test]
fn rejects_payload_dense_enums() {
    let source = module(
        "neocogi/ui.ncidl",
        r#"
            #[version(1)]
            #[dense]
            enum Event: u8 { Idle, Pointer(i32, i32) }
        "#,
    );
    let error = resolve_modules(vec![source]).unwrap_err();
    assert!(error.to_string().contains("must use an unsigned representation and unit variants"));
}

/// Confirms canonical enum names cannot silently imply a dense numeric domain.
#[test]
fn rejects_named_non_dense_enums() {
    let source = module(
        "neocogi/ui.ncidl",
        r#"
            #[version(1)]
            #[names(snake_case)]
            enum Direction: u8 { Horizontal, Vertical }
        "#,
    );
    let error = resolve_modules(vec![source]).unwrap_err();
    assert!(error.to_string().contains("without `#[dense]`"));
}

/// Confirms bit patterns fit their selected storage and remain unambiguous.
#[test]
fn rejects_invalid_bit_enum_values() {
    let overflow = module("neocogi/ui.ncidl", "#[version(1)] bitenum WindowOptions: u8 { NONE = 0, OVERFLOW = 256 }");
    let error = resolve_modules(vec![overflow]).unwrap_err();
    assert!(error.to_string().contains("value 256 does not fit"));

    let duplicate = module("neocogi/ui.ncidl", "#[version(1)] bitenum WindowOptions: u16 { FIRST = 1, SECOND = 1 }");
    let error = resolve_modules(vec![duplicate]).unwrap_err();
    assert!(error.to_string().contains("repeats value 1"));
}

/// Common interfaces preserve construction roles and thread-safety promises in signatures.
#[test]
fn interface_roles_and_threading_are_contract_identity_inputs() {
    // Keep the nominal name/version equal so only the role/capability differs.
    let fingerprint = |source: &str| resolve_modules(vec![module("example/contract.ncidl", source)]).unwrap().modules()[0].abi_fingerprint;
    let ordinary = fingerprint("#[version(1)] interface Root {}");
    let factory = fingerprint("#[version(1)] factory Root {}");
    let shared = fingerprint("#[version(1)] #[threading(shared)] interface Root {}");
    assert_ne!(ordinary, factory);
    assert_ne!(ordinary, shared);
    // Identical data layouts acquire different identities when posting becomes part of the contract.
    let copied = fingerprint("#[version(1)] struct Root { value: u32 }");
    let message = fingerprint("#[version(1)] #[message] struct Root { value: u32 }");
    assert_ne!(copied, message);
}

/// Required projections must have a finite layout and preserve every threading promise.
#[test]
fn rejects_recursive_and_weakened_required_interfaces() {
    for (source, diagnostic) in [
        ("#[version(1)] interface Root: Root {}", "recursive interface projections"),
        (
            "#[version(1)] interface Root: Child {} #[version(1)] interface Child: Root {}",
            "recursive interface projections",
        ),
        (
            "#[version(1)] #[threading(shared)] interface Shared {} #[version(1)] interface Local: Shared {}",
            "cannot weaken the shared threading promise",
        ),
    ] {
        let parsed = module("example/contracts.ncidl", source);
        assert!(resolve_modules(vec![parsed]).unwrap_err().to_string().contains(diagnostic));
    }
}

/// Generated ABI and output companions share a Rust namespace with authored and imported names.
#[test]
fn rejects_generated_rust_name_collisions_before_emission() {
    let local = module(
        "example/local.ncidl",
        "#[version(1)] interface IFoo {} #[version(1)] struct IFooAbi { value: u32 }",
    );
    let error = resolve_modules(vec![local]).unwrap_err().to_string();
    assert!(error.contains("error[EIDL021]"), "{error}");
    assert!(error.contains("IFooAbi"), "{error}");

    let origin = module("example/origin.ncidl", "#[version(1)] struct IFooAbi { value: u32 }");
    let consumer = module(
        "example/consumer.ncidl",
        "extern example.origin; use example.origin::IFooAbi; #[version(1)] interface IFoo {}",
    );
    let error = resolve_modules(vec![origin, consumer]).unwrap_err().to_string();
    assert!(error.contains("error[EIDL021]"), "{error}");
    assert!(error.contains("IFooAbi"), "{error}");
}
