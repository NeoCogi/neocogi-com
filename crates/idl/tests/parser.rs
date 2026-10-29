use std::path::Path;

use neocogi_idl::{
    AbiType, DeclarationKind, EnumNames, EnumRepr, LabelStyle, ModuleSource, ParseError, Receiver, SourcePosition, Threading, VariantFields, generate_rust,
    parse_module, parse_module_parts, resolve_modules,
};

/// Checks owned source coordinates after the parser's syntax worker has returned.
fn assert_receiver_lifetime_diagnostic(error: ParseError, module: &str, path: &Path, source: &str, lifetime: &str) {
    assert_syntax_diagnostic(
        error,
        module,
        path,
        source,
        lifetime,
        "IDL receivers cannot declare lifetimes; use `&self` or `&mut self`",
    );
}

/// All syntax failures own the authored range after their syntax worker returns.
fn assert_syntax_diagnostic(error: ParseError, module: &str, path: &Path, source: &str, syntax: &str, message: &str) {
    let ParseError::Source(diagnostic) = error else {
        panic!("expected an authored syntax error: {error}");
    };
    assert_eq!(diagnostic.code, "EIDL001");
    assert_eq!(diagnostic.message, message);
    assert_eq!(diagnostic.labels.len(), 1);
    let label = &diagnostic.labels[0];
    assert_eq!(label.style, LabelStyle::Primary);
    assert_eq!(label.span.module(), module);
    assert_eq!(label.span.path(), path.to_str().unwrap().replace('\\', "/"));

    let prefix = &source[..source.find(syntax).unwrap()];
    let line = prefix.bytes().filter(|&byte| byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
    assert_eq!(label.span.start(), SourcePosition { line, column });
    assert_eq!(
        label.span.end(),
        SourcePosition {
            line,
            column: column + syntax.chars().count(),
        }
    );
}

/// Owners and every authored module-reference form share one logical segment rule.
#[test]
fn rejects_invalid_logical_module_segments() {
    let declaration = "#[version(1)] struct Value { value: u32 }";
    for invalid in ["gen", "type", "async", "_", "self", "Self", "super", "crate", "r#type", "r#ordinary", "Café"] {
        for name in [format!("{invalid}.owner"), format!("proof.{invalid}"), format!("proof.{invalid}.owner")] {
            let directory = name.replace('.', "/");
            let file = format!("{directory}.ncidl");
            assert!(parse_module(Path::new(&file), declaration).is_err(), "accepted owner {name}");
            let part = format!("{directory}/part.ncidl");
            assert!(
                parse_module_parts(
                    Path::new(&directory),
                    &[ModuleSource {
                        path: Path::new(&part),
                        text: declaration
                    }]
                )
                .is_err(),
                "accepted directory owner {name}"
            );
            let qualified = name.replace('.', "::");
            for source in [
                format!("extern {name}; {declaration}"),
                format!("use {name}::Value; {declaration}"),
                format!("#[version(1)] struct Unused<T> {{ value: {qualified}::Value }}"),
                format!("#[version(1)] interface Unused<T>: {qualified}::Probe {{}}"),
            ] {
                assert!(parse_module(Path::new("proof/consumer.ncidl"), &source).is_err(), "accepted reference {source}");
            }
        }
    }
    for name in [
        "union",
        "safe",
        "raw",
        "macro_rules",
        "types",
        "_private",
        "CaseName",
        "case_name",
        "generation",
        "gen_value",
    ] {
        let owner = format!("proof.{name}");
        let file = format!("proof/{name}.ncidl");
        let module = parse_module(Path::new(&file), declaration).unwrap();
        assert_eq!(module.name.to_string(), owner);
        let source = format!("extern {owner}; use {owner}::Value; #[version(1)] struct Container {{ imported: Value, qualified: proof::{name}::Value }}");
        let consumer = parse_module(Path::new("proof/consumer.ncidl"), &source).unwrap();
        let graph = resolve_modules(vec![module, consumer]).unwrap();
        generate_rust(&graph, "proof.consumer").unwrap();
    }
    parse_module(Path::new("ordinary.ncidl"), declaration).unwrap();
    assert!(parse_module(Path::new("proof/consumer.ncidl"), "extern ordinary;").is_err());
    // Organizational part filenames do not define the enclosing module identity.
    parse_module_parts(
        Path::new("proof/ordinary"),
        &[ModuleSource {
            path: Path::new("proof/ordinary/type.ncidl"),
            text: declaration,
        }],
    )
    .unwrap();
}

/// Namespace diagnostics own the exact identifier span after worker exit/reordering.
#[test]
fn logical_module_segment_diagnostics_preserve_source_parts() {
    for invalid in ["gen", "r#ordinary"] {
        let message = format!("IDL module segment {invalid:?} must be a nonraw ASCII identifier other than `_` or a Rust 2024 keyword");
        for source in [
            format!("// café\nextern proof.{invalid}.owner;\n"),
            format!("// café\nuse proof.{invalid}::Value;\n"),
            format!("#[version(1)]\nstruct Unused<T> {{ /* café */ value: proof::{invalid}::Value }}\n"),
        ] {
            let path = Path::new("proof/consumer.ncidl");
            assert_syntax_diagnostic(parse_module(path, &source).unwrap_err(), "proof.consumer", path, &source, invalid, &message);
            let path = Path::new("proof/consumer/z_invalid.ncidl");
            let earlier = ModuleSource {
                path: Path::new("proof/consumer/a_valid.ncidl"),
                text: "#[version(1)] struct Earlier { value: u32 }",
            };
            let invalid_part = ModuleSource { path, text: &source };
            for parts in [[earlier, invalid_part], [invalid_part, earlier]] {
                assert_syntax_diagnostic(
                    parse_module_parts(Path::new("proof/consumer"), &parts).unwrap_err(),
                    "proof.consumer",
                    path,
                    &source,
                    invalid,
                    &message,
                );
            }
        }
    }
}

/// Rust-2024 keywords reject at member admission without restricting legal names.
#[test]
fn rejects_rust_2024_keyword_members() {
    let path = Path::new("proof/keywords.ncidl");
    for name in ["union", "safe", "raw", "macro_rules", "generation", "gen_value"] {
        let source = format!(
            "#[version(1)] struct Data {{ {name}: u32 }} \
             #[version(1)] enum Choice {{ Named {{ {name}: u32 }} }} \
             #[version(1)] interface Probe {{ fn {name}(&self, {name}: u32) -> Result<u32>; }}"
        );
        let graph = resolve_modules(vec![parse_module(path, &source).unwrap()]).unwrap();
        generate_rust(&graph, "proof.keywords").unwrap();
    }
    // A metadata path limit is not an authored member-name limit.
    let long_name = "a".repeat(neocogi_com_core::NC_DESCRIPTOR_RUST_PATH_MAX_LEN + 1);
    let source = format!("#[version(1)] struct Data {{ {long_name}: u32 }}");
    let parsed = parse_module(path, &source).unwrap();
    let DeclarationKind::Struct(data) = &parsed.declaration("Data").unwrap().kind else {
        panic!("expected record")
    };
    assert_eq!(data.fields[0].name, long_name);
    let source = "#[version(1)] struct Data<T> { value: T } #[version(1)] struct Closed { item: Data<u32> } \
                  #[version(1)] interface Probe<T> { fn read(&self, value: T) -> Result<T>; } \
                  #[version(1)] interface ClosedReader: Probe<u32> {}";
    resolve_modules(vec![parse_module(path, source).unwrap()]).unwrap();

    // Raw IDL names stay excluded; older strict keywords may reject inside Syn.
    for name in ["r#gen", "r#union", "type", "async", "self", "try"] {
        for source in [
            format!("#[version(1)] struct Data {{ {name}: u32 }}"),
            format!("#[version(1)] enum Choice {{ Named {{ {name}: u32 }} }}"),
            format!("#[version(1)] interface Probe {{ fn {name}(&self) -> Result<u32>; }}"),
            format!("#[version(1)] interface Probe {{ fn read(&self, {name}: u32) -> Result<u32>; }}"),
        ] {
            assert!(parse_module(path, &source).is_err(), "{source}");
        }
    }

    let mut accepted = Vec::new();
    for (source, role) in [
        ("#[version(1)] struct Data { gen: u32 }", "field"),
        ("#[version(1)] enum Choice { Named { gen: u32 } }", "field"),
        ("#[version(1)] interface Probe { fn gen(&self) -> Result<u32>; }", "method"),
        ("#[version(1)] interface Probe { fn read(&self, gen: u32) -> Result<u32>; }", "parameter"),
        ("#[version(1)] struct Unused<T> { gen: T }", "field"),
        ("#[version(1)] interface Unused<T> { fn gen(&self) -> Result<T>; }", "method"),
        (
            "#[version(1)] struct Used<T> { value: T } #[version(1)] struct Closed { item: Used<u32> } \
             #[version(1)] interface Unused<T> { fn read(&self, gen: T) -> Result<T>; }",
            "parameter",
        ),
    ] {
        match parse_module(path, source) {
            Ok(_) => accepted.push(source),
            Err(error) => assert_syntax_diagnostic(
                error,
                "proof.keywords",
                path,
                source,
                "gen",
                &format!("IDL {role} name \"gen\" is a reserved Rust 2024 keyword"),
            ),
        }
    }
    assert!(accepted.is_empty(), "reserved member names survived source admission: {accepted:?}");
}

/// Keyword diagnostics retain the exact part and Unicode scalar range after parsing.
#[test]
fn keyword_member_diagnostics_preserve_source_parts() {
    let directory = Path::new("proof/keywords");
    let path = directory.join("z_invalid.ncidl");
    let valid = ModuleSource {
        path: Path::new("proof/keywords/a_valid.ncidl"),
        text: "#[version(1)] struct Earlier { value: u32 }",
    };
    let mut accepted = Vec::new();
    for (declaration, role) in [
        ("struct Probe<T> { /* café */ gen: T }", "field"),
        ("enum Probe { Named { /* café */ gen: u32 } }", "field"),
        ("interface Probe<T> { /* café */ fn gen(&self) -> Result<T>; }", "method"),
        ("interface Probe { /* café */ fn read(&self, gen: u32) -> Result<u32>; }", "parameter"),
    ] {
        let source = format!("#[version(1)]\n{declaration}\n");
        let message = format!("IDL {role} name \"gen\" is a reserved Rust 2024 keyword");
        match parse_module(&path, &source) {
            Ok(_) => accepted.push(declaration),
            Err(error) => assert_syntax_diagnostic(error, "proof.keywords.z_invalid", &path, &source, "gen", &message),
        }
        let invalid = ModuleSource { path: &path, text: &source };
        for parts in [[valid, invalid], [invalid, valid]] {
            match parse_module_parts(directory, &parts) {
                Ok(_) => accepted.push(declaration),
                Err(error) => assert_syntax_diagnostic(error, "proof.keywords", &path, &source, "gen", &message),
            }
        }
    }
    assert!(accepted.is_empty(), "reserved member names survived part admission: {accepted:?}");
}

/// Unsupported annotations cannot disappear even when a template is never used.
#[test]
fn rejects_unsupported_argument_attributes() {
    let path = Path::new("proof/admission.ncidl");
    let mut accepted = Vec::new();
    for (index, attribute) in ["#[cfg(any())]", "#[unknown]", "#[doc = \"ignored\"]"].iter().enumerate() {
        let mut cases = Vec::new();
        for receiver in ["&self", "&mut self"] {
            cases.push((format!("fn read({attribute} {receiver}) -> Result<u32>;"), "", ""));
        }
        let (parameters, consumer) = match index {
            0 => ("", ""),
            1 => ("<T>", ""),
            _ => ("<T>", "#[version(1)] interface Consumer: Probe<u32> {}"),
        };
        cases.push((format!("fn read(&self, {attribute} value: u32) -> Result<u32>;"), parameters, consumer));
        for (method, parameters, consumer) in cases {
            let source = format!("#[version(1)] interface Probe{parameters} {{ {method} }} {consumer}");
            match parse_module(path, &source) {
                Ok(_) => accepted.push(method),
                Err(error) => assert_syntax_diagnostic(
                    error,
                    "proof.admission",
                    path,
                    &source,
                    attribute,
                    "attributes are not supported in this IDL position",
                ),
            }
        }
    }
    let source = "#[version(1)] interface Probe { fn read(#[cfg(any())] &self, #[unknown] value: u32) -> Result<u32>; }";
    match parse_module(path, source) {
        Ok(_) => accepted.push(source.to_owned()),
        Err(error) => assert_syntax_diagnostic(
            error,
            "proof.admission",
            path,
            source,
            "#[cfg(any())]",
            "attributes are not supported in this IDL position",
        ),
    }
    assert!(accepted.is_empty(), "unsupported argument syntax was erased: {accepted:?}");
    for attribute in ["#[cfg(any())]", "#[unknown]"] {
        let source = format!("#[version(1)] interface Probe {{ {attribute} fn read(&self) -> Result<u32>; }}");
        assert!(parse_module(path, &source).unwrap_err().to_string().contains("only documentation attributes"));
    }
}

/// Cover each shared lowering owner without multiplying source-coordinate cases.
#[test]
fn rejects_unrepresented_type_and_integer_syntax() {
    let path = Path::new("proof/admission.ncidl");
    let mut accepted = Vec::new();
    for (source, syntax, message) in [
        (
            "#[version(1)] struct Data { value: ::u32 }",
            "::",
            "IDL paths are module-qualified without a leading ::",
        ),
        (
            "#[version(1)] struct Data { value: ::String }",
            "::",
            "IDL paths are module-qualified without a leading ::",
        ),
        (
            "#[version(1)] struct Data { value: ::Vec<u8> }",
            "::",
            "IDL paths are module-qualified without a leading ::",
        ),
        (
            "#[version(1)] interface Probe { fn read(&self) -> ::Result<u32>; }",
            "::",
            "IDL paths are module-qualified without a leading ::",
        ),
        (
            "#[version(1)] struct Data { value: [u8; #[cfg(any())] 2] }",
            "#[cfg(any())]",
            "attributes are not supported in this IDL position",
        ),
        (
            "#[version(1)] enum Code: i8 { Item = #[unknown] 1 }",
            "#[unknown]",
            "attributes are not supported in this IDL position",
        ),
        (
            "#[version(1)] enum Code: i8 { Item = #[unknown] -1 }",
            "#[unknown]",
            "attributes are not supported in this IDL position",
        ),
        (
            "#[version(1)] enum Code: i8 { Item = -#[unknown] 1 }",
            "#[unknown]",
            "attributes are not supported in this IDL position",
        ),
        (
            "#[version(1)] bitenum Flags: u8 { ENABLED = #[doc = \"ignored\"] 1 }",
            "#[doc = \"ignored\"]",
            "attributes are not supported in this IDL position",
        ),
    ] {
        match parse_module(path, source) {
            Ok(_) => accepted.push(source),
            Err(error) => assert_syntax_diagnostic(error, "proof.admission", path, source, syntax, message),
        }
    }
    assert!(accepted.is_empty(), "unsupported type/integer syntax was erased: {accepted:?}");
    let source = "#[version(1)] struct Data<T> { value: T } #[version(1)] struct Holder { value: ::Data<u32> }";
    assert_syntax_diagnostic(
        parse_module(path, source).unwrap_err(),
        "proof.admission",
        path,
        source,
        "::",
        "IDL paths are module-qualified without a leading ::",
    );
}

/// Every newly checked owner reports the actual part with Unicode-aware columns.
#[test]
fn unsupported_syntax_diagnostics_preserve_source_parts() {
    let directory = Path::new("proof/admission");
    let path = directory.join("z_invalid.ncidl");
    let valid = ModuleSource {
        path: Path::new("proof/admission/a_valid.ncidl"),
        text: "#[version(1)] struct Earlier { value: u32 }",
    };
    for (declaration, syntax, message) in [
        (
            "interface Probe { /* café */ fn read(#[unknown] &self) -> Result<u32>; }",
            "#[unknown]",
            "attributes are not supported in this IDL position",
        ),
        (
            "interface Probe { /* café */ fn read(&self, #[doc = \"ignored\"] value: u32) -> Result<u32>; }",
            "#[doc = \"ignored\"]",
            "attributes are not supported in this IDL position",
        ),
        (
            "struct Probe { /* café */ value: [u8; #[cfg(any())] 2] }",
            "#[cfg(any())]",
            "attributes are not supported in this IDL position",
        ),
        (
            "struct Probe { /* café */ value: ::u32 }",
            "::",
            "IDL paths are module-qualified without a leading ::",
        ),
    ] {
        let source = format!("#[version(1)]\n{declaration}\n");
        let invalid = ModuleSource { path: &path, text: &source };
        assert_syntax_diagnostic(
            parse_module(&path, &source).unwrap_err(),
            "proof.admission.z_invalid",
            &path,
            &source,
            syntax,
            message,
        );
        for parts in [[valid, invalid], [invalid, valid]] {
            assert_syntax_diagnostic(
                parse_module_parts(directory, &parts).unwrap_err(),
                "proof.admission",
                &path,
                &source,
                syntax,
                message,
            );
        }
    }
}

/// Punctuation normalization and supported docs must preserve the same contract.
#[test]
fn syntax_normalization_preserves_valid_contracts() {
    let prefix = "#[version(1)] struct Data<T> { value: T } #[version(1)] interface Base { fn value(&self) -> Result<u32>; }";
    let plain = format!("{prefix} #[version(1)] interface Probe: Base {{ fn read(&self, value: Data<u32>) -> Result<u32>; }}");
    let punctuation = format!("{prefix} #[version(1)] interface Probe: (Base) {{ #[doc = \"kept\"] fn read<>(&self, value: Data::<u32>) -> Result::<u32>; }}");
    let graph = |source: &str| resolve_modules(vec![parse_module(Path::new("proof/admission.ncidl"), source).unwrap()]).unwrap();
    let plain = graph(&plain);
    let punctuation = graph(&punctuation);
    assert_eq!(
        plain.module("proof.admission").unwrap().abi_fingerprint,
        punctuation.module("proof.admission").unwrap().abi_fingerprint
    );
    let plain_rust = syn::parse_file(&generate_rust(&plain, "proof.admission").unwrap()).unwrap();
    let documented_rust = syn::parse_file(&generate_rust(&punctuation, "proof.admission").unwrap()).unwrap();
    for file in [&plain_rust, &documented_rust] {
        let provider = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Trait(item) if item.ident == "Probe" => Some(item),
                _ => None,
            })
            .unwrap();
        let syn::TraitItem::Fn(method) = &provider.items[0] else {
            panic!("expected method")
        };
        assert_eq!(method.sig.inputs.len(), 2);
        let syn::FnArg::Typed(argument) = &method.sig.inputs[1] else {
            panic!("expected argument")
        };
        assert!(argument.attrs.is_empty());
        assert!(method.sig.generics.params.is_empty());
    }
    let documented = punctuation.module("proof.admission").unwrap().source.declaration("Probe").unwrap();
    let DeclarationKind::Interface(interface) = &documented.kind else {
        panic!("expected interface")
    };
    assert_eq!(interface.methods[0].docs, ["kept"]);
}

/// Eager syntax admission rejects lifetimes even on templates that are never used.
#[test]
fn rejects_explicit_receiver_lifetimes() {
    let path = Path::new("proof/receivers.ncidl");
    for lifetime in ["'static", "'not_declared", "'_"] {
        for mutable in ["", "mut "] {
            for (parameters, consumer) in [("", ""), ("<T>", ""), ("<T>", "#[version(1)] interface Consumer: Probe<u32> {}")] {
                let source = format!("#[version(1)]\ninterface Probe{parameters} {{\n    fn read(&{lifetime} {mutable}self) -> Result<u32>;\n}}\n{consumer}");
                let error = parse_module(path, &source).unwrap_err();
                assert_receiver_lifetime_diagnostic(error, "proof.receivers", path, &source, lifetime);
            }
        }
    }
}

/// Sorted multipart admission must attach the failing part, not an earlier valid one.
#[test]
fn receiver_lifetime_diagnostics_preserve_source_parts() {
    let directory = Path::new("proof/receivers");
    let path = directory.join("z_invalid.ncidl");
    let valid = ModuleSource {
        path: Path::new("proof/receivers/a_valid.ncidl"),
        text: "#[version(1)] interface Earlier { fn read(&self) -> Result<u32>; }",
    };
    for lifetime in ["'static", "'not_declared", "'_"] {
        for mutable in ["", "mut "] {
            // A non-ASCII comment also distinguishes Unicode columns from byte offsets.
            let source = format!("#[version(1)]\ninterface Probe {{\n    /* café */ fn read(&{lifetime} {mutable}self) -> Result<u32>;\n}}");
            let invalid = ModuleSource { path: &path, text: &source };
            assert_receiver_lifetime_diagnostic(parse_module(&path, &source).unwrap_err(), "proof.receivers.z_invalid", &path, &source, lifetime);
            for parts in [[valid, invalid], [invalid, valid]] {
                let error = parse_module_parts(directory, &parts).unwrap_err();
                assert_receiver_lifetime_diagnostic(error, "proof.receivers", &path, &source, lifetime);
            }
        }
    }
}

/// Valid modes survive parsing, resolution and typed emission; other receiver syntax stays excluded.
#[test]
fn ordinary_receivers_keep_their_semantic_modes() {
    let path = Path::new("proof/receivers.ncidl");
    let module = parse_module(
        path,
        "#[version(1)] interface Probe { fn read(&self) -> Result<u32>; fn write(&mut self, value: u32) -> Result<()>; }",
    )
    .unwrap();
    let DeclarationKind::Interface(probe) = &module.declaration("Probe").unwrap().kind else {
        panic!("expected interface");
    };
    assert_eq!(probe.methods[0].receiver, Receiver::Shared);
    assert_eq!(probe.methods[1].receiver, Receiver::Mutable);
    let graph = resolve_modules(vec![module]).unwrap();
    let rust = generate_rust(&graph, "proof.receivers").unwrap();
    let generated = syn::parse_file(&rust).unwrap();
    let probe = generated
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Trait(item) if item.ident == "Probe" => Some(item),
            _ => None,
        })
        .expect("generated provider trait");
    assert_eq!(probe.items.len(), 2);
    for (item, mutable) in probe.items.iter().zip([false, true]) {
        let syn::TraitItem::Fn(method) = item else { panic!("expected method") };
        let Some(syn::FnArg::Receiver(receiver)) = method.sig.inputs.first() else {
            panic!("expected receiver")
        };
        assert!(matches!(receiver.reference, Some((_, None))));
        assert_eq!(receiver.mutability.is_some(), mutable);
    }

    for method in [
        "fn read(&self, value: &'static u32) -> Result<u32>;",
        "fn read(self) -> Result<u32>;",
        "fn read(mut self) -> Result<u32>;",
        "fn read(self: &Self) -> Result<u32>;",
        "fn read() -> Result<u32>;",
        "fn read<'a>(&'a self) -> Result<u32>;",
    ] {
        let source = format!("#[version(1)] interface Probe {{ {method} }}");
        let error = parse_module(path, &source).unwrap_err();
        assert!(matches!(error, ParseError::Source(_)), "{method}: {error}");
    }
    assert!(parse_module(path, "#[version(1)] interface Probe<'a> { fn read(&'a self) -> Result<u32>; }").is_err());
}

/// Exercises every initial declaration kind and ownership spelling together.
#[test]
fn parses_a_complete_contract_module() {
    let module = parse_module(
        Path::new("neocogi/ui.ncidl"),
        r#"
            extern neocogi.render;
            use neocogi.render::{ITexture, TextureFactory};

            /// Copied widget construction state.
            #[persistence(default)]
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
            interface UiFactory {
                fn create_button(&self, label: &str, options: WidgetOptions) -> Result<ComPtr<Button>>;
                fn upload(&self, bytes: &[u8]) -> Result<ComPtr<ITexture>>;
                fn read(&self, output: &mut [u8]) -> Result<u32>;
            }
        "#,
    )
    .unwrap();

    assert_eq!(module.name.to_string(), "neocogi.ui");
    assert_eq!(module.externals[0].module, "neocogi.render");
    assert_eq!(module.imports[0].module, "neocogi.render");
    assert_eq!(
        module.imports.iter().map(|import| import.name.as_str()).collect::<Vec<_>>(),
        ["ITexture", "TextureFactory"]
    );
    assert_eq!(module.declarations.len(), 7);
    assert_eq!(module.declaration("IWidget").unwrap().name, "IWidget");

    let DeclarationKind::Struct(options) = &module.declaration("WidgetOptions").unwrap().kind else {
        panic!("WidgetOptions should be a struct");
    };
    assert_eq!(options.persistence, neocogi_idl::PersistencePolicy::Default);
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
    let method = &button.methods[0];
    assert_eq!(method.receiver, Receiver::Mutable);

    let DeclarationKind::Interface(factory) = &module.declaration("UiFactory").unwrap().kind else {
        panic!("UiFactory should be a factory");
    };
    assert_eq!(factory.threading, Threading::Shared);
    let first = &factory.methods[0];
    let second = &factory.methods[1];
    let third = &factory.methods[2];
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
        #[version(1)] enum Changed { Edited(String), Submitted }
        #[version(1)] interface IStream<T> {}
        #[version(1)] interface Editor: IStream<Changed> { fn revision(&self) -> Result<u64>; }
    "#,
    )
    .unwrap();
    assert!(matches!(module.declaration("Changed").unwrap().kind, DeclarationKind::Enum(_)));
    let DeclarationKind::Interface(editor) = &module.declaration("Editor").unwrap().kind else {
        panic!("class")
    };
    assert_eq!(editor.bases[0].arguments.len(), 1);
    assert_eq!(editor.methods.len(), 1);
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
        "#[version(1)] #[message] struct Wrong { value: u32 }",
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
        ("#[version(1)] interface ui_factory {}", "UpperCamelCase"),
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
        "#[version(1)] interface IWidget {} #[version(1)] interface Factory { fn edit(&self, value: &mut IWidget) -> Result<()>; }",
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
                fn new(&self) -> Result<ComPtr<Button>>;
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

/// Construction and implementation roles belong to Rust, outside the contract grammar.
#[test]
fn rejects_removed_class_and_factory_keywords() {
    for keyword in ["class", "factory"] {
        let source = format!("#[version(1)] {keyword} Service {{}}");
        assert!(parse_module(Path::new("example/contracts.ncidl"), &source).is_err());
    }
}
