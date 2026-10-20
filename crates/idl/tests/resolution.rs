use std::path::Path;

use neocogi_idl::{ModuleSource, parse_module, parse_module_parts, resolve_modules};

/// Parses a fixture while preserving path-derived identity.
fn module(path: &str, source: &str) -> neocogi_idl::Module {
    parse_module(Path::new(path), source).unwrap()
}

/// Any payload forbids assignments throughout the enum, including unused templates.
#[test]
fn payload_enum_tags_are_compiler_owned_for_every_variant_shape() {
    for body in [
        "Empty = 0, Value(T)",
        "Empty, Value(T) = 1",
        "Value { data: T } = 3",
        "Empty = 4, Zero(())",
        "Zero { value: () } = 0",
    ] {
        for parameters in ["", "<T>"] {
            let body = if parameters.is_empty() { body.replace("T", "u32") } else { body.into() };
            let source = format!("#[version(1)] enum Choice{parameters} {{ {body} }}");
            let error = resolve_modules(vec![module("example/tags.ncidl", &source)]).unwrap_err().to_string();
            assert!(error.contains("EIDL073"), "{source}: {error}");
            assert!(error.contains("declaration order"), "{error}");
        }
    }
}

/// Integer enums still support sparse/signed values; sum classification is independent of payload size.
#[test]
fn integer_assignments_and_automatic_payload_tags_remain_supported() {
    for declaration in [
        "enum State: i8 { Failed = -7, Ready = 3 }",
        "enum Choice<T> { Empty, Value(T) }",
        "enum Choice { Empty, Zero(()), Record { value: () } }",
    ] {
        resolve_modules(vec![module("example/tags.ncidl", &format!("#[version(1)] {declaration}"))]).unwrap();
    }
}

/// The library stream accepts declared messages, while general IDL remains event-agnostic.
#[test]
fn common_event_binding_rejects_type_arguments() {
    for declaration in ["interface Changed {}"] {
        let source = module(
            "example/events.ncidl",
            &format!(
                "extern neocogi.com.events; use neocogi.com.events::EventBinding; #[version(1)] {declaration} #[version(1)] interface SourceFactory {{ fn create(&self, events: &EventBinding<Source, Changed>) -> Result<ComPtr<Source>>; }}
        #[version(1)] interface Source {{}}"
            ),
        );
        let transport = module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl"));
        match resolve_modules(vec![source, transport]) {
            Err(_) => {}
            Ok(graph) => assert!(neocogi_idl::generate_rust(&graph, "example.events").is_err()),
        }
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
            interface UiFactory {
                fn create_button(&self, size: Size) -> Result<ComPtr<Button>>;
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
        fingerprint("#[threading(shared)] #[version(1)] interface Service {}"),
        fingerprint("#[threading(local)] #[version(1)] interface Service {}")
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
            interface UiFactory {
                fn adopt(&self, button: Button) -> Result<()>;
            }
        "#,
    );
    let error = resolve_modules(vec![source]).unwrap_err();
    assert!(error.to_string().contains("must borrow interface"));
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
            text: "#[version(1)] interface Asset { fn take(&self, texture: ITexture) -> Result<()>; }",
        },
    ];
    let source = parse_module_parts(Path::new("neocogi/ui"), &parts).unwrap();
    let error = resolve_modules(vec![source]).unwrap_err().to_string();

    assert!(error.contains("must borrow interface values"), "{error}");
    assert!(error.contains("neocogi/ui/assets.ncidl:1:"), "{error}");
    assert!(error.contains("neocogi/ui/rendering.ncidl:1:25"), "{error}");
    assert!(error.contains("interface values require an explicit borrow"), "{error}");
    assert!(error.contains("borrow the object type"), "{error}");
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
            #[version(1)] struct Beta { text: &str }
        "#,
    );
    let error = resolve_modules(vec![source]).unwrap_err();
    let rendered = error.to_string();

    assert_eq!(error.diagnostics.len(), 2);
    assert!(rendered.contains("unknown IDL type \"Missing\""), "{rendered}");
    assert!(rendered.contains("struct Beta field text requires an owned field"), "{rendered}");
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
        error.contains("error[EIDL063]: recursive copied-value layout: First -> Option<Second> -> Second -> First"),
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

/// Common interfaces preserve thread-safety promises in signatures.
#[test]
fn interface_threading_is_a_contract_identity_input() {
    // Keep the nominal name/version equal so only the threading capability differs.
    let fingerprint = |source: &str| resolve_modules(vec![module("example/contract.ncidl", source)]).unwrap().modules()[0].abi_fingerprint;
    let ordinary = fingerprint("#[version(1)] interface Root {}");
    let shared = fingerprint("#[version(1)] #[threading(shared)] interface Root {}");
    assert_ne!(ordinary, shared);
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

/// Interface companions remain reserved; native value enums emit no public companions.
#[test]
fn reserves_all_visible_macro_companions() {
    for suffix in ["Abi", "VTable", "VtblFor", "Client"] {
        let name = format!("IFoo{suffix}");
        let source = format!("#[version(1)] interface IFoo {{}} #[version(1)] struct {name} {{ value: u32 }}");
        let error = resolve_modules(vec![module("example/interfaces.ncidl", &source)]).unwrap_err().to_string();
        assert!(error.contains("error[EIDL021]") && error.contains(&name), "{error}");
    }

    for suffix in ["Abi", "VTable", "VtblFor", "Client"] {
        let name = format!("Editor{suffix}");
        let source = format!(
            r#"
                #[version(1)] enum EditorEvent {{ Changed }}
                #[version(1)] interface Editor {{ fn post(&mut self, event: &EditorEvent) -> Result<()>; }}
                #[version(1)] struct {name} {{ value: u32 }}
            "#
        );
        let error = resolve_modules(vec![module("example/observers.ncidl", &source)]).unwrap_err().to_string();
        assert!(error.contains("error[EIDL021]") && error.contains(&name), "{error}");
    }

    // Copied and owned enums both leave the obsolete public companion names
    // available to authored values; only internal checked wire helpers remain.
    for payload in ["u32", "String"] {
        let source = format!(
            "#[version(1)] enum Choice {{ Selected({payload}) }}
             #[version(1)] struct ChoiceAbi {{ value: u32 }}
             #[version(1)] struct ChoiceTag {{ value: u32 }}
             #[version(1)] struct ChoicePayload {{ value: u32 }}
             #[version(1)] struct ChoiceSelectedPayload {{ value: u32 }}"
        );
        resolve_modules(vec![module("example/values.ncidl", &source)]).unwrap();
    }

    // An owned message does not invoke `com_enum`; its private wire helpers
    // cannot shadow a normal UpperCamel declaration.
    let owned_message = module(
        "example/owned_messages.ncidl",
        r#"
            #[version(1)] enum Choice { Selected(String) }
            #[version(1)] struct ChoiceAbi { value: u32 }
        "#,
    );
    let graph = resolve_modules(vec![owned_message]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.owned_messages").unwrap();
    assert!(generated.contains("pub struct ChoiceAbi"));
    assert!(!generated.contains("pub struct ChoiceTag"));
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

/// Only actually emitted public dense members reserve authored variant spellings.
#[test]
fn dense_member_conflicts_are_source_aware_and_policy_specific() {
    for generic in ["", "<T>"] {
        for name in ["COUNT", "ALL", "NAMES"] {
            for named in [false, true] {
                let policy = if named { "#[names(snake_case)]" } else { "" };
                let source = format!("#[version(1)] #[dense] {policy} enum Choice{generic}: u8 {{ {name} = 0, Ready = 1 }}");
                let result = resolve_modules(vec![module("proof/dense.ncidl", &source)]);
                if name == "NAMES" && !named {
                    result.unwrap();
                } else {
                    let error = result.unwrap_err().to_string();
                    assert!(error.contains("error[EIDL021]"), "{source}: {error}");
                    assert!(error.contains("proof/dense.ncidl") && error.contains(name), "{error}");
                    assert!(error.contains("public dense-enum API"), "{error}");
                }
            }
        }
    }
    // An ordinary integer enum has no dense helpers and keeps all three names.
    resolve_modules(vec![module(
        "proof/plain.ncidl",
        "#[version(1)] enum Choice: u8 { COUNT = 0, ALL = 1, NAMES = 2 }",
    )])
    .unwrap();
}
