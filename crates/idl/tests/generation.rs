use std::path::Path;

use neocogi_idl::{ModuleSource, parse_module, parse_module_parts, resolve_modules};

/// Parses one inline fixture using its path-derived logical module name.
fn module(path: &str, source: &str) -> neocogi_idl::Module {
    parse_module(Path::new(path), source).unwrap()
}

/// Compare actual public dense-only emission with the shared admission inventory.
/// Subtracting ordinary enum members avoids maintaining a second list of common
/// reflection/codec members; adding a new dense helper must update the policy.
#[test]
fn dense_public_members_match_admission_inventory() {
    use std::collections::BTreeSet;
    /// Reads the actual inherent public members from the production emitter.
    fn members(attributes: &str) -> BTreeSet<String> {
        let source = format!("#[version(1)] {attributes} enum Choice: u8 {{ Ready = 0, Other = 1 }}");
        let graph = resolve_modules(vec![module("proof/public.ncidl", &source)]).unwrap();
        let source = neocogi_idl::generate_rust(&graph, "proof.public").unwrap();
        syn::parse_file(&source)
            .unwrap()
            .items
            .into_iter()
            .filter_map(|item| match item {
                syn::Item::Impl(item) if item.trait_.is_none() => Some(item.items),
                _ => None,
            })
            .flatten()
            .filter_map(|item| match item {
                syn::ImplItem::Const(item) if matches!(item.vis, syn::Visibility::Public(_)) => Some(item.ident.to_string()),
                syn::ImplItem::Fn(item) if matches!(item.vis, syn::Visibility::Public(_)) => Some(item.sig.ident.to_string()),
                _ => None,
            })
            .collect()
    }
    let ordinary = members("");
    for names in [false, true] {
        let dense = members(if names { "#[dense] #[names(snake_case)]" } else { "#[dense]" });
        let actual = dense.difference(&ordinary).cloned().collect::<BTreeSet<_>>();
        let expected = neocogi_com_core::NcComDenseEnumMember::ALL
            .into_iter()
            .filter(|member| member.is_emitted(names))
            .map(|member| member.name().to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
    }
}

/// Imported messages generate typed access without pulling transport into the baseline ABI crate.
#[test]
fn imported_event_getters_use_the_declaring_module() {
    let message = module(
        "example/messages.ncidl",
        "extern neocogi.com.events; use neocogi.com.events::IEvent; #[version(1)] struct Changed { value: u32 } #[version(1)] interface IChanged: IEvent { fn value(&self) -> Result<Changed>; }",
    );
    let source = module(
        "example/source.ncidl",
        r#"
        extern example.messages;
        use example.messages::IChanged;
        extern neocogi.com.events; use neocogi.com.events::EventBinding;
        #[version(1)] interface ISourceFactory { fn create(&self, events: &EventBinding) -> Result<ComPtr<ISource>>; }
        #[version(1)] #[events(IChanged)] interface ISource {}
    "#,
    );
    let transport = module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl"));
    let graph = resolve_modules(vec![message, source, transport]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.source").unwrap();
    syn::parse_file(&generated).expect("imported message bindings parse");
    assert!(generated.contains("IdlEvent"));
    assert!(generated.contains("example_messages_contracts::Changed"));
    assert!(generated.contains("IChangedAbi::INTERFACE_DESCRIPTOR"));
    assert!(!generated.contains("__map_"));
    let baseline = neocogi_idl::generate_rust(&graph, "neocogi.com.events").unwrap();
    assert!(!baseline.contains("EVENT_BINDING_API_VERSION"));
    assert!(!baseline.contains("__nc_get_contract_module_description"));
}

/// Confirms generated bindings preserve names and expose factory-only objects.
#[test]
fn generates_rust_bindings_for_a_module_graph() {
    let render = module(
        "neocogi/render.ncidl",
        r#"
            /// Pixel dimensions.

            #[version(1)]
            struct Size { width: u32, height: u32 }

            // Status supports both live transport and automatic persistence.
            #[version(1)]
            struct StatusValue { status: Status }

            #[version(2)]
            interface ITexture { fn size(&self) -> Result<Size>; }

        "#,
    );
    let ui_source = r#"
        extern neocogi.render;
        use neocogi.render::{ITexture, Size};

        /// Texture sampling filter.
        #[version(1)]
        #[dense]
        #[names(snake_case)]
        enum Filter: u8 {
            /// Uses the nearest texel.
            Nearest = 0,
            Linear = 1,
        }

        #[version(1)]
        bitenum WindowOptions: u16 {
            NONE = 0,
            RESIZABLE = 1,
            TITLED = 2,
        }

        #[version(1)]
        interface Button: ITexture {
            fn resize(&mut self, size: Size) -> Result<()>;
        }

        #[threading(shared)]
        #[version(3)]
        interface UiFactory {
            fn create(&self, size: Size, texture: &ITexture) -> Result<ComPtr<Button>>;
            fn read(&self, output: &mut [u8]) -> Result<u32>;
        }
    "#;
    let ui = module("neocogi/ui.ncidl", ui_source);
    let graph = resolve_modules(vec![ui, render]).unwrap();

    let generated = neocogi_idl::generate_rust(&graph, "neocogi.ui").unwrap();
    let generated_render = neocogi_idl::generate_rust(&graph, "neocogi.render").unwrap();
    syn::parse_file(&generated).expect("generated bindings must be valid Rust syntax");
    assert!(generated.lines().count() > 20, "generated bindings must use readable multiline formatting");
    assert!(generated.contains("unsafe impl ::neocogi_com::ComInterface for UiFactoryAbi"));
    assert!(!generated.contains("factory,"));
    assert!(generated.contains("com_int_enum"));
    assert!(generated.contains("com_bit_enum"));
    assert!(generated.contains("pub bitenum WindowOptions: u16"));
    assert!(generated.contains("pub enum Filter: u8"));
    assert!(generated.contains("/// Texture sampling filter."));
    assert!(generated.contains("/// Uses the nearest texel."));
    assert!(generated.contains("pub const COUNT: usize = 2"));
    assert!(generated.contains("pub const ALL: [Self; Self::COUNT]"));
    assert!(generated.contains("pub const NAMES: [&'static str; Self::COUNT]"));
    assert!(generated.contains("pub const fn canonical_name"));
    assert!(!generated.contains("fn index"));
    assert!(generated.contains("trait UiFactory"));
    assert!(generated.contains("type Threading = ::neocogi_com::ThreadSafe"));
    assert!(generated.contains("NcOutputWriter"));
    assert!(generated_render.contains("pub status: ::neocogi_com::NcStatus"));
    assert!(generated_render.contains("unsafe impl ::neocogi_com::AbiType for Size"));
    assert!(generated_render.contains("impl ::neocogi_com::ComSerialize for Size"));
    assert!(generated_render.contains("impl ::neocogi_com::ComDeserialize for Size"));
    // Neither copied declarations nor COM object methods require alloc syntax.
    assert!(!generated_render.contains("extern crate alloc"));
    assert!(!generated.contains("extern crate alloc"));
    for (module, source) in [("neocogi.ui", &generated), ("neocogi.render", &generated_render)] {
        for declaration in &graph.module(module).unwrap().declarations {
            assert!(source.contains(&proc_macro2::Literal::byte_string(&declaration.abi_signature).to_string()));
        }
        for retired in ["com_interface", "com_struct", "com_enum", "com_value"] {
            assert!(!source.contains(&format!("::neocogi_com::{retired}(")));
        }
    }
    assert!(!generated.contains("NcAbiSignaturePart"));
    assert!(!generated.contains("__encode"));
    assert!(!generated.contains("std::any::Any"));
    // Event helpers require an EventBinding parameter; ordinary factory methods
    // must not synthesize a receiver or an event-source implementation.
    assert!(!generated.contains("impl ::neocogi_com::events::EventSource"));
    assert!(!generated.contains("pub type ButtonEventBinding"));
}

/// Confirms docs and whitespace remain outside generated ABI constants.
#[test]
fn preserves_documentation_without_making_it_an_abi_input() {
    let source = module(
        "example/value.ncidl",
        r#"
            /// Public generated value.
            #[version(1)]
            struct Value {
                /// Copied numeric payload.
                number: u64,
            }
        "#,
    );
    let graph = resolve_modules(vec![source]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.value").unwrap();

    assert!(generated.contains("/// Public generated value."));
    assert!(generated.contains("/// Copied numeric payload."));
    assert!(!generated.contains("#[doc ="));
    assert!(generated.contains("pub number: u64"));
}

/// Confirms file names, declaration placement, and discovery order do not
/// affect the fingerprint or generated Rust behavior; provenance records the new locations.
#[test]
fn multipart_permutations_preserve_bindings_except_provenance() {
    let geometry = "/// Size docs.\n#[version(1)] struct Size { width: u32, height: u32 }";
    let widgets = "/// Widget docs.\n#[version(1)] interface IWidget { fn resize(&mut self, size: Size) -> Result<()>; }";
    let first_parts = [
        ModuleSource {
            path: Path::new("example/contract/z_geometry.ncidl"),
            text: geometry,
        },
        ModuleSource {
            path: Path::new("example/contract/a_widgets.ncidl"),
            text: widgets,
        },
    ];
    let renamed_parts = [
        ModuleSource {
            path: Path::new("example/contract/one.ncidl"),
            text: widgets,
        },
        ModuleSource {
            path: Path::new("example/contract/nested/two.ncidl"),
            text: geometry,
        },
    ];
    let combined = module("example/contract.ncidl", &format!("{widgets}\n{geometry}"));
    let first = parse_module_parts(Path::new("example/contract"), &first_parts).unwrap();
    let renamed = parse_module_parts(Path::new("example/contract"), &renamed_parts).unwrap();

    let graphs = [
        resolve_modules(vec![combined]).unwrap(),
        resolve_modules(vec![first]).unwrap(),
        resolve_modules(vec![renamed]).unwrap(),
    ];
    let fingerprints = graphs
        .iter()
        .map(|graph| graph.module("example.contract").unwrap().abi_fingerprint)
        .collect::<Vec<_>>();
    let bindings = graphs
        .iter()
        .map(|graph| neocogi_idl::generate_rust(graph, "example.contract").unwrap())
        .collect::<Vec<_>>();

    assert!(fingerprints.windows(2).all(|pair| pair[0] == pair[1]));
    assert_ne!(bindings[0], bindings[1], "source layout changes must update provenance");
    let without_provenance = bindings
        .iter()
        .map(|source| {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("/// IDL source:"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert!(without_provenance.windows(2).all(|pair| pair[0] == pair[1]));
}

/// Signed integer enums retain negative values; payload tags start at zero.
#[test]
fn generates_negative_signed_enum_discriminants() {
    let source = neocogi_idl::parse_module(
        std::path::Path::new("example/signed.ncidl"),
        "#[version(1)] enum State: i8 { Failed = -1, Ready = 0 } \
         #[version(1)] enum Response: i16 { Rejected, Accepted(i32) }",
    )
    .unwrap();
    let graph = neocogi_idl::resolve_modules(vec![source]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.signed").unwrap();
    assert!(generated.contains("Failed = -1"), "{generated}");
    assert!(!generated.contains("com_tag"), "{generated}");
    assert!(generated.contains("Accepted(i32)"), "{generated}");
}

/// Generated COM projections retain the message shape needed by designer tools.
#[test]
fn generated_interface_exposes_its_idl_event_and_nested_message_shape() {
    let source = module(
        "example/widget.ncidl",
        r#"
        #[version(1)] struct Bounds { x: i32, y: i32 }
        #[version(1)] enum State: u8 { Idle = 0, Busy = 1 }
        #[version(1)] bitenum Flags: u8 { NONE = 0, READY = 1 }
        #[version(1)] struct Changed { text: String }
        #[version(1)] enum WidgetEvent: u8 {
            Changed(Changed),
            Submitted,
        }
        extern neocogi.com.events; use neocogi.com.events::EventBinding;
        #[version(1)] interface WidgetFactory { fn create(&self, events: &EventBinding) -> Result<ComPtr<Widget>>; }
        #[version(1)] interface Widget {}
    "#,
    );
    let transport = module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl"));
    let graph = resolve_modules(vec![source, transport]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.widget").unwrap();
    syn::parse_file(&generated).unwrap();
    assert!(generated.contains("pub const IDL_TYPE"));
    assert!(generated.contains("pub const IDL_TYPE"));
    assert!(generated.contains("IdlShape::BitEnum"));
    assert!(generated.contains("pub const IDL_INTERFACE"));
    assert!(generated.contains("events: &[]"));
    assert!(generated.contains(r#"name: "Changed""#));
    assert!(generated.contains(r#"name: "Submitted""#));
}

/// Metadata generation resolves field and variant shapes directly without a zipper.
#[test]
fn direct_design_shape_resolution_preserves_variant_grouping_and_tags() {
    let source = module(
        "example/shapes.ncidl",
        r#"
        #[version(1)] struct Point { x: i32, y: i32 }
        #[version(1)] enum Action: u8 {
            Move { point: Point, speed: f32 },
            Say(String),
            Pause,
        }
        extern neocogi.com.events; use neocogi.com.events::EventBinding;
        #[version(1)] interface ShapeFactory { fn create(&self, events: &EventBinding) -> Result<ComPtr<Actor>>; }
        #[version(1)] interface Actor {}
    "#,
    );
    let transport = module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl"));
    let graph = resolve_modules(vec![source, transport]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.shapes").unwrap();
    syn::parse_file(&generated).unwrap();

    // Verify Point struct shape and fields
    assert!(generated.contains(r#"qualified_name: "example.shapes.Point""#));
    assert!(generated.contains(r#"name: "x""#));
    assert!(generated.contains(r#"name: "y""#));

    // Verify Action enum shape with tags 0, 1, 2 and distinct field groups
    assert!(generated.contains(r#"qualified_name: "example.shapes.Action""#));
    assert!(generated.contains(r#"name: "Move""#));
    assert!(generated.contains(r#"tag: 0i64"#));
    assert!(generated.contains(r#"name: "point""#));
    assert!(generated.contains(r#""example.shapes.Point""#));
    assert!(generated.contains(r#"name: "speed""#));
    assert!(generated.contains(r#"Scalar("f32")"#));

    assert!(generated.contains(r#"name: "Say""#));
    assert!(generated.contains(r#"tag: 1i64"#));
    assert!(generated.contains(r#"name: "0""#));
    assert!(generated.contains(r#"IdlValueType::Text"#));

    assert!(generated.contains(r#"name: "Pause""#));
    assert!(generated.contains(r#"tag: 2i64"#));
}
