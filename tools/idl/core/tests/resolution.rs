use std::path::Path;

use neocogi_idl::{parse_module, resolve_modules};

/// Parses a fixture while preserving path-derived identity.
fn module(path: &str, source: &str) -> neocogi_idl::Module {
    parse_module(Path::new(path), source).unwrap()
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
            trait ITexture { fn size(&self) -> Result<Size>; }
        "#,
    );
    let ui = module(
        "neocogi/ui.ncidl",
        r#"
            use neocogi.render::{ITexture, Size};

            #[version(1)]
            class Button {
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
        fingerprint("#[version(1)] trait IService { fn load(&self, key: u32) -> Result<u32>; }"),
        fingerprint("#[version(1)] trait IService { fn load(&mut self, key: u32) -> Result<u32>; }")
    );
    assert_ne!(
        fingerprint("#[version(1)] trait IService { fn load(&self, key: u32) -> Result<u32>; }"),
        fingerprint("#[version(1)] trait IService { fn load(&self, key: u64) -> Result<u32>; }")
    );
    assert_ne!(
        fingerprint("#[version(1)] trait IBase {} #[version(1)] trait IService: IBase {}"),
        fingerprint("#[version(1)] trait IBase {} #[version(1)] trait IService {}")
    );
    assert_ne!(
        fingerprint("#[threading(shared)] #[version(1)] factory Service {}"),
        fingerprint("#[threading(local)] #[version(1)] factory Service {}")
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
            class Button {}

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
            use example.second::Second;
            #[version(1)]
            struct First { second: Second }
        "#,
    );
    let second = module(
        "example/second.ncidl",
        r#"
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

/// Confirms explicit IDs preserve exact UUID bytes and declaration names.
#[test]
fn preserves_explicit_identity_without_rewriting_names() {
    let source = module(
        "neocogi/ui.ncidl",
        r#"
            #[id(7, "9be73186-f123-47bc-8140-4a57825eeb21")]
            trait IWidget {}
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
