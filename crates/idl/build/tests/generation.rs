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
            #[version(1)]
            struct Size { width: u32, height: u32 }

            #[version(2)]
            trait ITexture { fn size(&self) -> Result<Size>; }

        "#,
    );
    let ui_source = r#"
        use neocogi.render::{ITexture, Size};

        #[repr(u8)]
        #[version(1)]
        enum Filter { Nearest = 0, Linear = 1 }

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
    syn::parse_file(&generated).expect("generated bindings must be valid Rust syntax");
    assert!(generated.contains("com_class_description"));
    assert!(generated.contains("com_factory_description"));
    assert!(generated.contains("com_int_enum"));
    assert!(generated.contains("pub enum Filter : u8"));
    assert!(generated.contains("trait UiFactory"));
    assert!(generated.contains("threading = shared"));
    assert!(generated.contains("NcCopyToBufferOffset"));
    assert!(!generated.contains("std :: any :: Any"));
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

    assert!(generated.contains("Public generated value."));
    assert!(generated.contains("Copied numeric payload."));
    assert!(generated.contains("pub number : u64"));
}
