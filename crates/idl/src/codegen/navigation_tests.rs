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

/// Typed plans preserve the Rust identifier supplied by the compiler. Final IDL
/// source provenance is checked separately through the generated source map.
#[test]
fn typed_interface_and_value_names_keep_their_supplied_identifier_spans() {
    use quote::quote;
    use crate::codegen::model::{AbiMethod, AbiReceiver, AbiReturn, AbiReturnKind};
    let definition: syn::ItemTrait = syn::parse_str("\n\n pub trait Counter { fn value(&self) -> Result<u32, ::neocogi_com::NcStatus>; }").unwrap();
    let expected = definition.ident.span();
    let method = AbiMethod {
        ident: syn::parse_quote!(value),
        doc_attrs: Vec::new(),
        receiver: AbiReceiver::Shared,
        args: Vec::new(),
        ok: AbiReturn {
            rust_ty: syn::parse_quote!(u32),
            abi_ty: syn::parse_quote!(<u32 as ::neocogi_com::ComValue>::Wire),
            kind: AbiReturnKind::Plain,
        },
    };
    let plan = crate::codegen::model::InterfacePlan {
        name: definition.ident,
        visibility: definition.vis,
        docs: definition.attrs,
        id: "fa4298eb-5301-4ccb-b132-67bcf98c9102".parse().unwrap(),
        qualified_name: "example.navigation.Counter".into(),
        signature: [3; 32],
        threading: crate::codegen::InterfaceThreading::Local,
        bases: Vec::new(),
        methods: vec![method],
    };
    let parts = crate::codegen::expand_interface_parts(plan).unwrap();
    let abi = parts.abi;
    let provider = parts.provider;
    let expanded = quote! { #abi #provider };
    let actual = syn::parse2::<syn::File>(expanded.clone())
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some(item.ident.to_string()),
            syn::Item::Enum(item) => Some(item.ident.to_string()),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let public = neocogi_com_core::NcComInterfaceCompanion::ALL
        .into_iter()
        .map(|companion| format!("Counter{}", companion.suffix()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual, public, "public emission and collision admission share one inventory");
    for name in public {
        assert_origin(expanded.clone(), &name, expected);
    }

    let enumeration: syn::ItemEnum = syn::parse_str("\n\n\n pub enum Track { Fixed { extent:i32 }, Auto }").unwrap();
    let cases = enumeration
        .variants
        .iter()
        .enumerate()
        .map(|(ordinal, variant)| {
            let tag = proc_macro2::Literal::usize_unsuffixed(ordinal);
            crate::codegen::ValueCase {
                variant: Some(variant.ident.clone()),
                fields: variant.fields.clone(),
                tag: quote! { #tag },
            }
        })
        .collect::<Vec<_>>();
    let expanded = crate::codegen::emit_value(
        &enumeration.ident,
        &enumeration.vis,
        &cases,
        quote! { ::neocogi_com::NcAbiSignature::from_static(&[3;32]) },
        true,
        false,
        &syn::parse_quote!(::neocogi_com),
    );
    assert_origin(expanded, "Track", enumeration.ident.span());
}
