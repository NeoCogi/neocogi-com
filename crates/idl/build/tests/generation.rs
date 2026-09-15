use std::path::Path;

use neocogi_idl::{ModuleSource, parse_module, parse_module_parts, resolve_modules};

/// Parses one inline fixture using its path-derived logical module name.
fn module(path: &str, source: &str) -> neocogi_idl::Module {
    parse_module(Path::new(path), source).unwrap()
}

/// One NCIDL event generates both native producer replay and typed application wiring.
#[test]
fn generates_event_payload_sink_adapter_and_subscription_port() {
    let transport = module(
        "neocogi/com/events.ncidl",
        include_str!("../../../event-contracts/idl/neocogi/com/events/events.ncidl"),
    );
    let events = module(
        "example/events.ncidl",
        r#"
        extern neocogi.com.events;
        use neocogi.com.events::IEventSubscription;
        #[version(1)]
        event IChangedSink {
            fn changed(&self, text: &str, cursor: u64) -> Result<()>;
        }
        #[version(1)]
        trait IEditor {
            fn revision(&self) -> Result<u64>;
            event subscribe_changed(&mut self): IChangedSink;
        }
    "#,
    );
    let graph = resolve_modules(vec![events, transport]).unwrap();
    let generated = neocogi_idl_build::generate_rust(&graph, "example.events").unwrap();
    syn::parse_file(&generated).expect("event bindings must be valid Rust syntax");
    assert!(generated.contains("pub struct IChangedSinkEvent"));
    assert!(generated.contains("pub text: ::alloc::string::String"));
    assert!(generated.contains("impl ::neocogi_com_events::EventPayload for IChangedSinkEvent"));
    assert!(generated.contains("::neocogi_com_events::copy_text(text)?"));
    assert!(generated.contains("create_i_changed_sink_event_sink"));
    assert!(generated.contains("create_i_changed_sink_checked_event_sink"));
    assert!(generated.contains("if let Some(validate) = self.validate"));
    assert!(generated.contains("EVENT_BINDING_API_VERSION == 1"));
    assert!(generated.contains("fn subscribe_changed"));
    assert!(generated.contains("pub fn connect_i_editor_subscribe_changed"));
    assert!(generated.contains(".register(|| {"));
    assert!(generated.contains("source.subscribe_changed(sink.as_com_ref())"));
    assert!(generated.contains("IEventSubscriptionAbi"));
    assert!(!generated.contains("std::any::Any"));
    let transport_bindings = neocogi_idl_build::generate_rust(&graph, "neocogi.com.events").unwrap();
    assert!(
        !transport_bindings.contains("EVENT_BINDING_API_VERSION"),
        "the baseline ABI crate must not depend on its own transport implementation"
    );
}

/// A source-only module still checks the COM event support API and calls its
/// imported sink adapter; it does not need to redeclare the sink event.
#[test]
fn source_only_event_port_checks_support_api() {
    let transport = module(
        "neocogi/com/events.ncidl",
        include_str!("../../../event-contracts/idl/neocogi/com/events/events.ncidl"),
    );
    let sink = module(
        "example/sink.ncidl",
        "#[version(1)] event IChangedSink { fn changed(&self, value: u32) -> Result<()>; }",
    );
    let source = module(
        "example/source.ncidl",
        r#"
        extern neocogi.com.events;
        extern example.sink;
        use neocogi.com.events::IEventSubscription;
        use example.sink::IChangedSink;
        #[version(1)] trait ISource { event subscribe(&self): IChangedSink; }
    "#,
    );
    let graph = resolve_modules(vec![source, sink, transport]).unwrap();
    let generated = neocogi_idl_build::generate_rust(&graph, "example.source").unwrap();
    syn::parse_file(&generated).expect("source-only event bindings must parse");
    assert!(generated.contains("EVENT_BINDING_API_VERSION == 1"));
    assert!(generated.contains("pub fn connect_i_source_subscribe"));
    assert!(generated.contains("example_sink_contracts::create_i_changed_sink_event_sink"));
}

/// Confirms generated bindings preserve names and expose factory-only objects.
#[test]
fn generates_rust_bindings_for_a_module_graph() {
    let render = module(
        "neocogi/render.ncidl",
        r#"
            /// Pixel dimensions.
            #[deserialize]
            #[version(1)]
            struct Size { width: u32, height: u32, status: Status }

            #[version(2)]
            trait ITexture { fn size(&self) -> Result<Size>; }

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
        class Button: ITexture {
            fn resize(&mut self, size: Size) -> Result<()>;
        }

        #[threading(shared)]
        #[version(3)]
        factory UiFactory {
            fn create(&self, size: Size, texture: &ITexture) -> Result<Button>;
            fn read(&self, output: &mut [u8]) -> Result<u32>;
        }
    "#;
    let ui = module("neocogi/ui.ncidl", ui_source);
    let graph = resolve_modules(vec![ui, render]).unwrap();

    let generated = neocogi_idl_build::generate_rust(&graph, "neocogi.ui").unwrap();
    let generated_render = neocogi_idl_build::generate_rust(&graph, "neocogi.render").unwrap();
    syn::parse_file(&generated).expect("generated bindings must be valid Rust syntax");
    assert!(generated.lines().count() > 20, "generated bindings must use readable multiline formatting");
    assert!(generated.contains("com_class_description"));
    assert!(generated.contains("com_factory_description"));
    assert!(generated.contains("com_int_enum"));
    assert!(generated.contains("com_bit_enum"));
    assert!(generated.contains("pub bitenum WindowOptions: u16"));
    assert!(generated.contains("pub enum Filter: u8"));
    assert!(generated.contains("/// Texture sampling filter."));
    assert!(generated.contains("/// Uses the nearest texel."));
    assert!(!generated.contains("#[doc ="));
    assert!(generated.contains("pub const COUNT: usize = 2"));
    assert!(generated.contains("pub const ALL: [Self; Self::COUNT]"));
    assert!(generated.contains("pub const NAMES: [&'static str; Self::COUNT]"));
    assert!(generated.contains("pub const fn canonical_name"));
    assert!(!generated.contains("fn index"));
    assert!(generated.contains("trait UiFactory"));
    assert!(generated.contains("threading = shared"));
    assert!(generated.contains("NcCopyToBufferOffset"));
    assert!(generated_render.contains("pub status: ::neocogi_com::NcStatus"));
    assert!(generated_render.contains("com_struct("));
    assert!(generated_render.contains("deserialize,"));
    assert_eq!(generated.matches("assigned_signature = b").count(), 2);
    assert_eq!(generated_render.matches("assigned_signature = b").count(), 2);
    assert!(!generated.contains("NcAbiSignaturePart"));
    assert!(!generated.contains("__encode"));
    assert!(!generated.contains("std::any::Any"));
    assert!(!generated.contains("constructor"));
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
    let generated = neocogi_idl_build::generate_rust(&graph, "example.value").unwrap();

    assert!(generated.contains("/// Public generated value."));
    assert!(generated.contains("/// Copied numeric payload."));
    assert!(!generated.contains("#[doc ="));
    assert!(generated.contains("pub number: u64"));
}

/// Confirms file names, declaration placement, and discovery order do not
/// affect the fingerprint or generated Rust for one logical module.
#[test]
fn multipart_permutations_generate_identical_bindings() {
    let geometry = "/// Size docs.\n#[version(1)] struct Size { width: u32, height: u32 }";
    let widgets = "/// Widget docs.\n#[version(1)] trait IWidget { fn resize(&mut self, size: Size) -> Result<()>; }";
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
        .map(|graph| neocogi_idl_build::generate_rust(graph, "example.contract").unwrap())
        .collect::<Vec<_>>();

    assert!(fingerprints.windows(2).all(|pair| pair[0] == pair[1]));
    assert!(bindings.windows(2).all(|pair| pair[0] == pair[1]));
}
