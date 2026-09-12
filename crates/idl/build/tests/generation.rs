use std::path::Path;

use neocogi_idl::{parse_module, resolve_modules};

/// Parses one inline fixture using its path-derived logical module name.
fn module(path: &str, source: &str) -> neocogi_idl::Module {
    parse_module(Path::new(path), source).unwrap()
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
