use std::path::Path;

use neocogi_idl::{AbiType, DeclarationKind, EnumRepr, Receiver, Threading, VariantFields, parse_module};

/// Exercises every initial declaration kind and ownership spelling together.
#[test]
fn parses_a_complete_contract_module() {
    let module = parse_module(
        Path::new("neocogi/ui.ncidl"),
        r#"
            use neocogi.render::{ITexture, TextureFactory};

            /// Copied widget construction state.
            #[deserialize(default)]
            #[version(1)]
            struct WidgetOptions {
                enabled: bool,
                dimensions: [i32; 2],
                teardown: Status,
            }

            #[repr(u8)]
            #[version(2)]
            enum Direction {
                Horizontal = 0,
                Vertical = 1,
            }

            #[version(1)]
            enum Event {
                Idle,
                Pointer(i32, i32),
                Resize { width: u32, height: u32 },
            }

            #[version(1)]
            trait IWidget {
                fn enabled(&self) -> Result<bool>;
                fn set_enabled(&mut self, enabled: bool) -> Result<()>;
            }

            #[id(1, "9be73186-f123-47bc-8140-4a57825eeb21")]
            class Button: IWidget {
                fn set_image(&mut self, image: &ITexture) -> Result<()>;
                fn texture(&self) -> Result<Option<ITexture>>;
            }

            #[threading(shared)]
            #[version(1)]
            factory UiFactory {
                fn create_button(&self, label: &str, options: WidgetOptions) -> Result<Button>;
                fn upload(&self, bytes: &[u8]) -> Result<ITexture>;
                fn read(&self, output: &mut [u8]) -> Result<u32>;
            }
        "#,
    )
    .unwrap();

    assert_eq!(module.name.to_string(), "neocogi.ui");
    assert_eq!(module.imports[0].module, "neocogi.render");
    assert_eq!(module.imports[0].names, ["ITexture", "TextureFactory"]);
    assert_eq!(module.declarations.len(), 6);
    assert_eq!(module.declaration("IWidget").unwrap().name, "IWidget");

    let DeclarationKind::Struct(options) = &module.declaration("WidgetOptions").unwrap().kind else {
        panic!("WidgetOptions should be a struct");
    };
    assert!(options.deserialize);
    assert!(options.deserialize_default);
    assert!(!options.deserialize_sequence);
    assert_eq!(
        options.fields[1].ty,
        AbiType::Array {
            element: Box::new(AbiType::Scalar("i32".into())),
            len: 2,
        }
    );
    assert_eq!(options.fields[2].ty, AbiType::Scalar("Status".into()));

    let DeclarationKind::Enum(direction) = &module.declaration("Direction").unwrap().kind else {
        panic!("Direction should be an enum");
    };
    assert_eq!(direction.repr, EnumRepr::U8);
    assert_eq!(direction.variants[1].discriminant, Some(1));

    let DeclarationKind::Enum(event) = &module.declaration("Event").unwrap().kind else {
        panic!("Event should be an enum");
    };
    assert!(matches!(event.variants[0].fields, VariantFields::Unit));
    assert!(matches!(event.variants[1].fields, VariantFields::Tuple(_)));
    assert!(matches!(event.variants[2].fields, VariantFields::Named(_)));

    let button = module.declaration("Button").unwrap();
    assert_eq!(button.id.version, 1);
    assert!(button.id.explicit.is_some());
    let DeclarationKind::Class(button) = &button.kind else {
        panic!("Button should be a class");
    };
    assert_eq!(button.required_interfaces[0].rust_path(), "IWidget");
    assert_eq!(button.methods[0].receiver, Receiver::Mutable);

    let DeclarationKind::Factory(factory) = &module.declaration("UiFactory").unwrap().kind else {
        panic!("UiFactory should be a factory");
    };
    assert_eq!(factory.threading, Threading::Shared);
    assert!(matches!(factory.methods[0].parameters[0].ty, AbiType::Borrowed(_)));
    assert!(matches!(factory.methods[1].parameters[0].ty, AbiType::Slice(_)));
    assert!(matches!(factory.methods[2].parameters[0].ty, AbiType::OutputSlice(_)));
}

/// Confirms mutable borrows cannot imply mutable access to foreign objects.
#[test]
fn rejects_mutable_object_borrows() {
    let error = parse_module(
        Path::new("neocogi/ui.ncidl"),
        "#[version(1)] trait IWidget {} #[version(1)] factory Factory { fn edit(&self, value: &mut IWidget) -> Result<()>; }",
    )
    .unwrap_err();
    assert!(error.to_string().contains("must use `&mut [T]`"));
}

/// Confirms visibility syntax cannot enter a contract accidentally.
#[test]
fn rejects_public_keyword() {
    let error = parse_module(Path::new("neocogi/ui.ncidl"), "#[version(1)] pub struct Size { width: u32 }").unwrap_err();
    assert!(error.to_string().contains("public implicitly"));
}

/// Confirms every declaration forces an intentional version or identity.
#[test]
fn rejects_missing_identity_attribute() {
    let error = parse_module(Path::new("neocogi/ui.ncidl"), "struct Size { width: u32 }").unwrap_err();
    assert!(error.to_string().contains("requires `#[version(N)]`"));
}

/// Confirms constructors cannot return through the class grammar unnoticed.
#[test]
fn rejects_constructor_attributes() {
    let error = parse_module(
        Path::new("neocogi/ui.ncidl"),
        r#"
            #[version(1)]
            class Button {
                #[constructor]
                fn new(&self) -> Result<Button>;
            }
        "#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("only documentation attributes"));
}

/// Confirms the explicit UUID form carries its version without a second attribute.
#[test]
fn rejects_duplicate_version_sources() {
    let error = parse_module(
        Path::new("neocogi/ui.ncidl"),
        r#"
            #[version(1)]
            #[id(1, "9be73186-f123-47bc-8140-4a57825eeb21")]
            trait IWidget {}
        "#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("exactly one"));
}
