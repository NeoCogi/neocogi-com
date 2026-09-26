use std::path::Path;

use neocogi_idl::{AbiType, Callable, DeclarationKind, EnumNames, EnumRepr, ModuleSource, Receiver, Threading, VariantFields, parse_module, parse_module_parts};

/// Exercises every initial declaration kind and ownership spelling together.
#[test]
fn parses_a_complete_contract_module() {
    let module = parse_module(
        Path::new("neocogi/ui.ncidl"),
        r#"
            extern neocogi.render;
            use neocogi.render::{ITexture, TextureFactory};

            /// Copied widget construction state.
            #[deserialize(default)]
            #[version(1)]
            struct WidgetOptions {
                enabled: bool,
                dimensions: [i32; 2],
                teardown: Status,
            }

            #[version(2)]
            #[dense]
            #[names(snake_case)]
            enum Direction: u8 {
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
            bitenum WindowOptions: u16 {
                NONE = 0,
                RESIZABLE = 1,
                TITLED = 2,
            }

            #[version(1)]
            interface IWidget {
                fn enabled(&self) -> Result<bool>;
                fn set_enabled(&mut self, enabled: bool) -> Result<()>;
            }

            #[id(1, "9be73186-f123-47bc-8140-4a57825eeb21")]
            interface Button: IWidget {
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
    assert_eq!(module.externals[0].module, "neocogi.render");
    assert_eq!(module.imports[0].module, "neocogi.render");
    assert_eq!(module.imports[0].names, ["ITexture", "TextureFactory"]);
    assert_eq!(module.declarations.len(), 7);
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
    assert!(direction.dense);
    assert_eq!(direction.names, Some(EnumNames::SnakeCase));
    assert_eq!(direction.variants[1].discriminant, Some(1));

    let DeclarationKind::Enum(event) = &module.declaration("Event").unwrap().kind else {
        panic!("Event should be an enum");
    };
    assert_eq!(event.repr, EnumRepr::U32);
    assert!(matches!(event.variants[0].fields, VariantFields::Unit));
    assert!(matches!(event.variants[1].fields, VariantFields::Tuple(_)));
    assert!(matches!(event.variants[2].fields, VariantFields::Named(_)));

    let DeclarationKind::BitEnum(options) = &module.declaration("WindowOptions").unwrap().kind else {
        panic!("WindowOptions should be a bit enum");
    };
    assert_eq!(options.repr, EnumRepr::U16);
    assert_eq!(options.flags[2].name, "TITLED");
    assert_eq!(options.flags[2].value, 2);

    let button = module.declaration("Button").unwrap();
    assert_eq!(button.id.version, 1);
    assert!(button.id.explicit.is_some());
    let DeclarationKind::Interface(button) = &button.kind else {
        panic!("Button should be a class");
    };
    assert_eq!(button.bases[0].rust_path(), "IWidget");
    let Callable::Method(method) = &button.callables[0];
    assert_eq!(method.receiver, Receiver::Mutable);

    let DeclarationKind::Interface(factory) = &module.declaration("UiFactory").unwrap().kind else {
        panic!("UiFactory should be a factory");
    };
    assert_eq!(factory.threading, Threading::Shared);
    let Callable::Method(first) = &factory.callables[0];
    let Callable::Method(second) = &factory.callables[1];
    let Callable::Method(third) = &factory.callables[2];
    assert!(matches!(first.parameters[0].ty, AbiType::Borrowed(_)));
    assert!(matches!(second.parameters[0].ty, AbiType::Slice(_)));
    assert!(matches!(third.parameters[0].ty, AbiType::OutputSlice(_)));
}

/// A typed stream is an ordinary generic base; IDL contains no event-specific members.
#[test]
fn parses_message_stream_as_an_ordinary_class_base() {
    let module = parse_module(
        Path::new("example/events.ncidl"),
        r#"
        #[version(1)] #[message] enum Changed { Edited(String), Submitted }
        #[version(1)] interface IStream<T> {}
        #[version(1)] class Editor: IStream<Changed> { fn revision(&self) -> Result<u64>; }
    "#,
    )
    .unwrap();
    assert!(module.declaration("Changed").unwrap().message);
    let DeclarationKind::Interface(editor) = &module.declaration("Editor").unwrap().kind else {
        panic!("class")
    };
    assert_eq!(editor.bases[0].arguments.len(), 1);
    assert_eq!(editor.callables.len(), 1);
    for legacy in [
        "#[version(1)] interface Old { changed: Changed; }",
        "#[version(1)] #[observer] interface Old {}",
    ] {
        assert!(parse_module(Path::new("example/old.ncidl"), legacy).is_err());
    }
}

/// Obsolete protocol syntax and accessor/getter collisions receive parser diagnostics.
#[test]
fn rejects_obsolete_protocols_and_ambiguous_outputs() {
    for source in [
        "#[version(1)] port Changes { fn changed(&mut self) -> Result<()>; }",
        "#[version(1)] interface ISource { changed: Changed; changed: Changed; }",
        "#[version(1)] interface ISource { fn changed(&self) -> Result<()>; changed: Changed; }",
        "#[version(1)] interface ISource { changed: Changed; fn __map_changed(&self) -> Result<()>; }",
        "#[version(1)] interface ISource { port changed: Changed; }",
        "#[version(1)] interface ISource { event subscribe(&self): Changed; }",
        "#[version(1)] #[message] interface Wrong {}",
        "#[version(1)] #[message] #[message] struct Wrong { value: u32 }",
    ] {
        assert!(parse_module(Path::new("example/events.ncidl"), source).is_err(), "{source}");
    }
}

/// Confirms bit enums require intentional unsigned storage and explicit values.
#[test]
fn rejects_ambiguous_bit_enum_layouts() {
    let missing_repr = parse_module(Path::new("neocogi/ui.ncidl"), "#[version(1)] bitenum WindowOptions { NONE = 0 }").unwrap_err();
    assert!(missing_repr.to_string().contains("explicit unsigned representation"));

    let signed_repr = parse_module(Path::new("neocogi/ui.ncidl"), "#[version(1)] bitenum WindowOptions: i16 { NONE = 0 }").unwrap_err();
    assert!(signed_repr.to_string().contains("must be u8, u16, or u32"));

    let implicit_value = parse_module(Path::new("neocogi/ui.ncidl"), "#[version(1)] bitenum WindowOptions: u16 { NONE }").unwrap_err();
    assert!(implicit_value.to_string().contains("expected `=`"));
}

/// Confirms source identifiers match the Rust API shape emitted by generation.
#[test]
fn enforces_rust_identifier_conventions() {
    let invalid = [
        ("#[version(1)] struct widget_options { value: u8 }", "UpperCamelCase"),
        ("#[version(1)] enum flow_direction: u8 { Horizontal = 0 }", "UpperCamelCase"),
        ("#[version(1)] interface widget {}", "UpperCamelCase"),
        ("#[version(1)] interface button {}", "UpperCamelCase"),
        ("#[version(1)] factory ui_factory {}", "UpperCamelCase"),
        ("#[version(1)] enum Direction: u8 { LEFT_TO_RIGHT = 0 }", "UpperCamelCase"),
        ("#[version(1)] struct Size { Width: u32 }", "snake_case"),
        ("#[version(1)] enum Event { Resize { NewWidth: u32 } }", "snake_case"),
        ("#[version(1)] interface IWidget { fn SetWidth(&self) -> Result<()>; }", "snake_case"),
        (
            "#[version(1)] interface IWidget { fn set_width(&self, NewWidth: u32) -> Result<()>; }",
            "snake_case",
        ),
        ("#[version(1)] bitenum WindowOptions: u16 { Resizable = 1 }", "SCREAMING_SNAKE_CASE"),
        (
            "extern neocogi.render; use neocogi.render::texture; #[version(1)] struct Size { width: u32 }",
            "UpperCamelCase",
        ),
    ];

    for (source, expected) in invalid {
        let error = parse_module(Path::new("neocogi/naming.ncidl"), source).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?} diagnostic for {source:?}; found {error}"
        );
    }
}

/// Confirms mutable borrows cannot imply mutable access to foreign objects.
#[test]
fn rejects_mutable_object_borrows() {
    let error = parse_module(
        Path::new("neocogi/ui.ncidl"),
        "#[version(1)] interface IWidget {} #[version(1)] factory Factory { fn edit(&self, value: &mut IWidget) -> Result<()>; }",
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

/// Confirms representation is IDL syntax rather than a Rust layout attribute.
#[test]
fn rejects_rust_enum_repr_attributes() {
    let error = parse_module(
        Path::new("neocogi/ui.ncidl"),
        "#[version(1)] #[repr(u8)] enum Direction { Horizontal, Vertical }",
    )
    .unwrap_err();
    assert!(error.to_string().contains("unsupported attribute"));
}

/// Confirms only fixed-width integer discriminants can follow an enum colon.
#[test]
fn rejects_invalid_enum_representations() {
    let error = parse_module(Path::new("neocogi/ui.ncidl"), "#[version(1)] enum Direction: usize { Horizontal, Vertical }").unwrap_err();
    assert!(error.to_string().contains("must be u8, u16, u32, i8, i16, or i32"));
}

/// Confirms every documented fixed-width suffix maps to its semantic model.
#[test]
fn parses_every_enum_representation_suffix() {
    let cases = [
        ("u8", EnumRepr::U8),
        ("u16", EnumRepr::U16),
        ("u32", EnumRepr::U32),
        ("i8", EnumRepr::I8),
        ("i16", EnumRepr::I16),
        ("i32", EnumRepr::I32),
    ];

    for (suffix, expected) in cases {
        let source = format!("#[version(1)] enum Value: {suffix} {{ Zero = 0 }}");
        let module = parse_module(Path::new("neocogi/value.ncidl"), &source).unwrap();
        let DeclarationKind::Enum(value) = &module.declaration("Value").unwrap().kind else {
            panic!("Value should be an enum");
        };
        assert_eq!(value.repr, expected);
    }
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
            interface Button {
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
            interface IWidget {}
        "#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("exactly one"));
}

/// Confirms independent source parts compose one canonically ordered module.
#[test]
fn merges_module_parts_without_file_order_semantics() {
    let geometry = ModuleSource {
        path: Path::new("neocogi/ui/geometry.ncidl"),
        text: "#[version(1)] struct Size { width: u32, height: u32 }",
    };
    let widgets = ModuleSource {
        path: Path::new("neocogi/ui/widgets.ncidl"),
        text: "#[version(1)] interface IWidget { fn resize(&mut self, size: Size) -> Result<()>; }",
    };

    let forward = parse_module_parts(Path::new("neocogi/ui"), &[geometry, widgets]).unwrap();
    let reverse = parse_module_parts(Path::new("neocogi/ui"), &[widgets, geometry]).unwrap();

    assert_eq!(forward.name.to_string(), "neocogi.ui");
    assert_eq!(
        forward.declarations.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(),
        ["IWidget", "Size"]
    );
    assert_eq!(forward.declarations, reverse.declarations);
}

/// Confirms external dependencies are canonical module-level declarations
/// independent of source-part and declaration order.
#[test]
fn canonicalizes_external_dependencies_across_parts() {
    let first = ModuleSource {
        path: Path::new("neocogi/ui/first.ncidl"),
        text: "extern neocogi.render; #[version(1)] struct First { value: u32 }",
    };
    let second = ModuleSource {
        path: Path::new("neocogi/ui/second.ncidl"),
        text: "extern neocogi.assets; #[version(1)] struct Second { value: u32 }",
    };

    let module = parse_module_parts(Path::new("neocogi/ui"), &[second, first]).unwrap();
    assert_eq!(
        module.externals.iter().map(|external| external.module.as_str()).collect::<Vec<_>>(),
        ["neocogi.assets", "neocogi.render"]
    );
}

/// Confirms repeated dependency declarations identify both source parts.
#[test]
fn reports_both_parts_for_duplicate_external_dependencies() {
    let first = ModuleSource {
        path: Path::new("neocogi/ui/first.ncidl"),
        text: "extern neocogi.render; #[version(1)] struct First { value: u32 }",
    };
    let second = ModuleSource {
        path: Path::new("neocogi/ui/second.ncidl"),
        text: "extern neocogi.render; #[version(1)] struct Second { value: u32 }",
    };

    let error = parse_module_parts(Path::new("neocogi/ui"), &[second, first]).unwrap_err().to_string();
    assert!(error.contains("error[EIDL019]: duplicate external dependency \"neocogi.render\""), "{error}");
    assert!(error.contains("neocogi/ui/first.ncidl:1:1"), "{error}");
    assert!(error.contains("neocogi/ui/second.ncidl:1:1"), "{error}");
}

/// Confirms duplicate members identify both declarations with stable snippets.
#[test]
fn reports_both_locations_for_duplicate_fields() {
    let error = parse_module(
        Path::new("neocogi/ui.ncidl"),
        "#[version(1)]\nstruct Size {\n    width: u32,\n    width: f32,\n}",
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("error[EIDL013]: duplicate field \"width\""), "{error}");
    assert!(error.contains("neocogi/ui.ncidl:4:5"), "{error}");
    assert!(error.contains("neocogi/ui.ncidl:3:5"), "{error}");
    assert!(error.contains("first declared here"), "{error}");
}

/// Confirms every repeated named member reports its original declaration.
#[test]
fn reports_original_locations_for_all_duplicate_member_kinds() {
    let cases = [
        ("#[version(1)] enum State { Ready, Ready }", "duplicate enum variant \"Ready\""),
        (
            "#[version(1)] bitenum Flags: u8 { ENABLED = 1, ENABLED = 2 }",
            "duplicate bit enum flag \"ENABLED\"",
        ),
        (
            "#[version(1)] interface IService { fn load(&self) -> Result<()>; fn load(&self) -> Result<()>; }",
            "duplicate method or port \"load\"",
        ),
        (
            "#[version(1)] interface IService { fn load(&self, key: u32, key: u64) -> Result<()>; }",
            "duplicate parameter \"key\"",
        ),
        (
            "#[version(1)] enum Event { Resize { width: u32, width: u64 } }",
            "duplicate variant field \"width\"",
        ),
        (
            "use example.types::{Value, Value}; #[version(1)] struct Local { value: u32 }",
            "duplicate import name \"Value\"",
        ),
    ];

    for (source, expected) in cases {
        let error = parse_module(Path::new("example/duplicates.ncidl"), source).unwrap_err().to_string();
        assert!(error.contains(expected), "expected {expected:?} in:\n{error}");
        assert!(error.contains("first declared here"), "{error}");
        assert_eq!(error.matches("example/duplicates.ncidl:").count(), 2, "{error}");
    }
}

/// Confirms duplicate declarations across parts identify both source files.
#[test]
fn reports_both_parts_for_duplicate_declarations() {
    let first = ModuleSource {
        path: Path::new("neocogi/ui/geometry.ncidl"),
        text: "#[version(1)] struct Size { width: u32 }",
    };
    let second = ModuleSource {
        path: Path::new("neocogi/ui/layout.ncidl"),
        text: "#[version(1)] struct Size { width: f32 }",
    };
    let error = parse_module_parts(Path::new("neocogi/ui"), &[second, first]).unwrap_err().to_string();

    assert!(error.contains("error[EIDL010]: duplicate declaration \"Size\""), "{error}");
    assert!(error.contains("neocogi/ui/layout.ncidl:1:22"), "{error}");
    assert!(error.contains("neocogi/ui/geometry.ncidl:1:22"), "{error}");
}

/// Confirms diagnostic-only part paths are still portable and normalized.
#[test]
fn rejects_noncanonical_module_part_paths() {
    let part = ModuleSource {
        path: Path::new("neocogi/ui/../other.ncidl"),
        text: "#[version(1)] struct Value { raw: u32 }",
    };
    let error = parse_module_parts(Path::new("neocogi/ui"), &[part]).unwrap_err().to_string();

    assert!(error.contains("must be a .ncidl file below module directory"), "{error}");
}
