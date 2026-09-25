use std::path::Path;

use neocogi_idl::{ModuleSource, parse_module, parse_module_parts, resolve_modules};

/// Parses one inline fixture using its path-derived logical module name.
fn module(path: &str, source: &str) -> neocogi_idl::Module {
    parse_module(Path::new(path), source).unwrap()
}

/// Imported messages generate typed access without pulling transport into the baseline ABI crate.
#[test]
fn imported_outputs_need_allocation_but_no_queue_runtime() {
    let message = module("example/messages.ncidl", "#[version(1)] #[message] struct Changed { value: u32 }");
    let source = module(
        "example/source.ncidl",
        r#"
        extern example.messages;
        use example.messages::Changed;
        #[version(1)] interface ISource { first: Changed; second: Changed; }
    "#,
    );
    let transport = module("neocogi/com/events.ncidl", include_str!("../../api/idl/neocogi/com/events/events.ncidl"));
    let graph = resolve_modules(vec![message, source, transport]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.source").unwrap();
    syn::parse_file(&generated).expect("imported message bindings parse");
    assert!(generated.contains("extern crate alloc"));
    assert!(!generated.contains("neocogi_com::events"));
    assert!(generated.contains("fn first("));
    assert!(generated.contains("fn second("));
    assert!(generated.contains("example_messages_contracts::__ChangedReceiverAbi"));
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
            #[deserialize]
            #[version(1)]
            struct Size { width: u32, height: u32, status: Status }

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
        factory UiFactory {
            fn create(&self, size: Size, texture: &ITexture) -> Result<Button>;
            fn read(&self, output: &mut [u8]) -> Result<u32>;
        }
    "#;
    let ui = module("neocogi/ui.ncidl", ui_source);
    let graph = resolve_modules(vec![ui, render]).unwrap();

    let generated = neocogi_idl::generate_rust(&graph, "neocogi.ui").unwrap();
    let generated_render = neocogi_idl::generate_rust(&graph, "neocogi.render").unwrap();
    syn::parse_file(&generated).expect("generated bindings must be valid Rust syntax");
    assert!(generated.lines().count() > 20, "generated bindings must use readable multiline formatting");
    assert!(generated.contains("com_interface"));
    assert!(generated.contains("factory,"));
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

/// Signed unit and tagged enums must retain negative ABI values during lowering.
#[test]
fn generates_negative_signed_enum_discriminants() {
    let source = neocogi_idl::parse_module(
        std::path::Path::new("example/signed.ncidl"),
        "#[version(1)] enum State: i8 { Failed = -1, Ready = 0 } \
         #[version(1)] enum Response: i16 { Rejected = -2, Accepted(i32) }",
    )
    .unwrap();
    let graph = neocogi_idl::resolve_modules(vec![source]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.signed").unwrap();
    assert!(generated.contains("Failed = -1"), "{generated}");
    assert!(generated.contains("com_tag(-2)"), "{generated}");
}
