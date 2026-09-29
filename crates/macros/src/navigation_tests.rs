//! Source-span regressions for names produced from IDL declarations.

use proc_macro2::{Span, TokenStream, TokenTree};

/// Every occurrence of a derived name must retain the declaration's location.
/// Searching tokens covers both its definition and generated companion uses.
fn assert_origin(tokens: TokenStream, name: &str, expected: Span) {
    /// Descends nested syntax while requiring every matching token to retain its origin.
    fn visit(tokens: TokenStream, name: &str, expected: Span) -> usize {
        tokens
            .into_iter()
            .map(|token| match token {
                TokenTree::Group(group) => visit(group.stream(), name, expected),
                TokenTree::Ident(ident) if ident == name => {
                    assert_eq!(ident.span().start(), expected.start(), "{name} lost its source start");
                    assert_eq!(ident.span().end(), expected.end(), "{name} lost its source end");
                    1
                }
                _ => 0,
            })
            .sum()
    }
    assert!(visit(tokens, name, expected) > 0, "missing generated identifier {name}");
}

/// The actual IDL pipeline supplies the attributes and signatures used in builds.
/// Deliberately spaced declarations distinguish source spans from call-site spans.
#[test]
fn generated_interface_and_enum_names_keep_their_authored_identifier_spans() {
    let module = neocogi_idl::parse_module(
        std::path::Path::new("example/navigation.ncidl"),
        "\n#[version(1)] interface Counter { fn value(&self) -> Result<u32>; }\n\n#[version(1)] enum Track: u8 { Fixed { extent: i32 }, Auto }",
    )
    .unwrap();
    let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
    let generated = neocogi_idl::generate_rust(&graph, "example.navigation").unwrap();
    let mut verified = 0;
    for item in syn::parse_file(&generated).unwrap().items {
        match item {
            syn::Item::Trait(mut item) if item.ident == "Counter" => {
                let expected = item.ident.span();
                let index = item
                    .attrs
                    .iter()
                    .position(|attr| attr.path().segments.last().unwrap().ident == "com_interface")
                    .unwrap();
                let args = item.attrs.remove(index).parse_args().unwrap();
                let expanded = crate::interface::expand_com_interface(args, item).unwrap();
                for name in ["CounterAbi", "CounterVTable", "CounterVtblFor", "CounterClient"] {
                    assert_origin(expanded.clone(), name, expected);
                }
                verified += 1;
            }
            syn::Item::Enum(mut item) if item.ident == "Track" => {
                let expected = item.ident.span();
                let index = item
                    .attrs
                    .iter()
                    .position(|attr| attr.path().segments.last().unwrap().ident == "com_enum")
                    .unwrap();
                let args = item.attrs.remove(index).parse_args().unwrap();
                let expanded = crate::enums::expand_com_enum(args, item).unwrap();
                for name in ["Track", "__TrackValueWire", "__TrackValuePayload"] {
                    assert_origin(expanded.clone(), name, expected);
                }
                verified += 1;
            }
            _ => {}
        }
    }
    assert_eq!(verified, 2, "both source declarations must exercise their macro expansion");
}
