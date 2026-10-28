//! A selected Rust type and its resolved IDL identity are one emission decision.
//!
//! Module constants describe a namespace, not necessarily the declarations a
//! facade reexports. Obligations therefore use the selected type's own trait
//! identity. No source-only dependency walk or runtime admission is involved.

use std::collections::BTreeMap;
use super::*;

/// Deterministic obligations for actual external types selected in one module.
/// Keys include the accessible Rust path: a supplying route must not substitute
/// another unchecked facade merely because its logical identity was seen before.
#[derive(Default)]
pub(super) struct ExternalBindings<'a> {
    /// Qualified declaration and selected marker/value path to the exact proof.
    selected: BTreeMap<(&'a str, String), ExternalBinding<'a>>,
}

/// One immutable semantic declaration paired with its actual emitted Rust owner.
struct ExternalBinding<'a> {
    /// Complete identity from resolution, including specialization provenance.
    declaration: ResolvedDeclaration<'a>,
    /// Interface marker or value type, using the accessible dependency route.
    rust_type: syn::Path,
}

impl ExternalBindings<'_> {
    /// Emits anonymous, allocation-free compile-time obligations. All operations
    /// are qualified and no public proof name can collide with authored names.
    pub(super) fn assertions(&self) -> TokenStream {
        let assertions = self.selected.iter().map(|((qualified, path), binding)| {
            let ty = &binding.rust_type;
            let signature = signature_literal(binding.declaration.identity.abi_signature);
            let message = Literal::string(&format!("IDL binding mismatch: {qualified} at {path}"));
            let actual = match binding.declaration.declaration.kind {
                DeclarationKind::Interface(_) => quote! {
                    <#ty as ::neocogi_com::ComInterface>::INTERFACE_ID.abi_signature
                },
                _ => quote! { <#ty as ::neocogi_com::ComValueType>::VALUE_SIGNATURE },
            };
            let iid = matches!(binding.declaration.declaration.kind, DeclarationKind::Interface(_)).then(|| {
                // A literal array pattern is const-compatible on Rust 1.85;
                // it needs neither const PartialEq nor new helper/local names.
                let bytes = binding.declaration.identity.id.to_uuid_bytes();
                quote! {
                    ::core::assert!(::core::matches!(
                        <#ty as ::neocogi_com::ComInterface>::INTERFACE_ID.iid.to_uuid_bytes(),
                        [#(#bytes),*]
                    ), #message);
                }
            });
            quote! {
                const _: () = {
                    #iid
                    ::core::assert!(#actual.__const_eq(::neocogi_com::NcAbiSignature::from_static(#signature)), #message);
                };
            }
        });
        quote! { #(#assertions)* }
    }
}

/// Selects syntax and records its identity together, never in a second traversal.
/// Local references point into this generated module, whose definitions the
/// compiler owns. External references can pass through ordinary facade reexports
/// and must prove the actual value/marker, regardless of module constants.
pub(super) fn named_rust_type<'a>(
    graph: &mut Generation<'a>,
    module: &ResolvedModule,
    resolved: ResolvedDeclaration<'a>,
    abi_suffix: bool,
) -> Result<TokenStream, GenerateError> {
    let path = named_rust_path(graph, module, resolved, abi_suffix)?;
    Ok(quote! { #path })
}

/// Selects the typed path and records its external marker/value proof once.
/// Imports keep this syntax to derive the same supplying route's public siblings;
/// token consumers use the thin quoting adapter above rather than select again.
pub(super) fn named_rust_path<'a>(
    graph: &mut Generation<'a>,
    module: &ResolvedModule,
    resolved: ResolvedDeclaration<'a>,
    abi_suffix: bool,
) -> Result<syn::Path, GenerateError> {
    let interface = matches!(resolved.declaration.kind, DeclarationKind::Interface(_));
    let name = rust_ident(&resolved.declaration.name)?;
    let marker = if interface {
        rust_ident(&format!("{}{}", resolved.declaration.name, NcComInterfaceCompanion::Abi.suffix()))?
    } else {
        name.clone()
    };
    let emitted = if interface && abi_suffix { &marker } else { &name };
    if resolved.module.source.name == module.source.name {
        return Ok(syn::parse_quote! { self::#emitted });
    }
    let mut selected = graph.paths.dependency_path(&resolved.module.source.name.to_string())?;
    selected.segments.push(syn::PathSegment {
        ident: emitted.clone(),
        arguments: syn::PathArguments::None,
    });
    let mut rust_type = selected.clone();
    rust_type.segments.last_mut().expect("selected declaration has a final segment").ident = marker;
    // This is diagnostic/dedup text only. The bound declaration and syntax above
    // remain authoritative; the text is never parsed to recover semantic facts.
    let path = quote! { #rust_type }.to_string().replace(' ', "");
    graph
        .bindings
        .selected
        .entry((resolved.identity.qualified_name.as_str(), path))
        .or_insert(ExternalBinding { declaration: resolved, rust_type });
    Ok(selected)
}
