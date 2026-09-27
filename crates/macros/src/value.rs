//! One recursive field converter shared by copied and owned declaration macros.
//!
//! Application fields never double as unchecked foreign wire fields. This module
//! emits raw C records, a tag-selected union for enums, and concrete backing
//! guards. Field traversal always delegates to the same ComValue contract.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, Ident, Visibility};

/// A record or enum alternative with its declared tag and ordered field shape.
pub(crate) struct ValueCase {
    /// None identifies a record; Some names an enum alternative.
    pub variant: Option<Ident>,
    /// Raw discriminant used only by tagged enums.
    pub tag: TokenStream,
    /// Source-level fields whose individual codecs define recursive lowering.
    pub fields: Fields,
}

/// Generates a complete checked codec without altering the application type.
pub(crate) fn expand(name: &Ident, visibility: &Visibility, cases: &[ValueCase], signature: TokenStream, inline: bool, nc: &syn::Path) -> TokenStream {
    let wire_name = format_ident!("__{name}ValueWire");
    let union_name = format_ident!("__{name}ValuePayload");
    let storage_name = format_ident!("__{name}ValueStorage");
    let is_enum = cases.first().is_some_and(|case| case.variant.is_some());
    let mut wires = Vec::new();
    let mut members = Vec::new();
    let mut storages = Vec::new();
    let mut encode = Vec::new();
    let mut decode = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let case_wire = if is_enum {
            format_ident!("__{name}ValueCase{index}")
        } else {
            wire_name.clone()
        };
        let case_storage = format_ident!("Case{index}");
        let member = format_ident!("case_{index}");
        let mut raw_fields = Vec::new();
        let mut stored_fields = Vec::new();
        let mut bindings = Vec::new();
        let mut encoding = Vec::new();
        let mut decoding = Vec::new();
        let mut raw_values = Vec::new();
        let mut stored_values = Vec::new();
        for (position, field) in case.fields.iter().enumerate() {
            let binding = field.ident.clone().unwrap_or_else(|| format_ident!("field_{position}"));
            let raw = format_ident!("raw_{position}");
            let stored = format_ident!("storage_{position}");
            let ty = &field.ty;
            raw_fields.push(quote! { /// Recursively checked raw field; never an application reference.
            pub #binding: <#ty as #nc::ComValue>::Wire });
            stored_fields.push(quote! { /// Retains this field's converted indirect storage through the call.
            #binding: <#ty as #nc::ComValue>::Storage });
            // A local copy allows packed application fields to be borrowed safely.
            let field_value = if inline {
                quote! { &#binding }
            } else {
                quote! { #binding }
            };
            encoding.push(quote! { let (#raw, #stored) = <#ty as #nc::ComValue>::encode_value(#field_value, budget)?; });
            decoding.push(quote! {
                // SAFETY: the selected raw field and its active backing are live.
                unsafe { <#ty as #nc::ComValue>::decode_value(raw.#binding, budget) }?
            });
            raw_values.push(quote! { #binding: #raw });
            stored_values.push(quote! { #binding: #stored });
            bindings.push(binding);
        }
        let constructor = match &case.variant {
            Some(variant) => quote! { Self::#variant },
            None => quote! { Self },
        };
        let (pattern, decoded) = match &case.fields {
            Fields::Unit => (constructor.clone(), constructor),
            Fields::Named(_) => (
                quote! { #constructor { #(#bindings),* } },
                quote! { #constructor { #(#bindings: #decoding),* } },
            ),
            Fields::Unnamed(_) => (quote! { #constructor(#(#bindings),*) }, quote! { #constructor(#(#decoding),*) }),
        };
        if raw_fields.is_empty() {
            raw_fields.push(quote! { /// Initialized placeholder for an empty raw record.
            pub empty: u8 });
            raw_values.push(quote! { empty: 0 });
        }
        wires.push(quote! {
            #[doc(hidden)] #[repr(C)] #[derive(Clone, Copy)]
            /// Raw fields for one structural value alternative.
            #visibility struct #case_wire { #(#raw_fields),* }
        });
        members.push(quote! { /// Read only when the corresponding tag has been validated.
        pub #member: #case_wire });
        storages.push(quote! { /// Typed backing storage for one selected alternative.
        #case_storage { #(#stored_fields),* } });
        let tag = &case.tag;
        let raw_value = quote! { #case_wire { #(#raw_values),* } };
        let wire = if is_enum {
            quote! { #wire_name { tag: #tag, payload: #union_name { #member: #raw_value } } }
        } else {
            raw_value
        };
        let stored = if inline {
            quote! { () }
        } else {
            quote! { #storage_name::#case_storage { #(#stored_values),* } }
        };
        encode.push(quote! { #pattern => { #(#encoding)* Ok((#wire, #stored)) } });
        let read = (!bindings.is_empty()).then(|| {
            if is_enum {
                quote! { // SAFETY: this match arm validated the tag before selecting a payload.
                let raw = unsafe { wire.payload.#member }; }
            } else {
                quote! { let raw = wire; }
            }
        });
        decode.push(if is_enum {
            quote! { #tag => { #read Ok(#decoded) } }
        } else {
            quote! { #read Ok(#decoded) }
        });
    }
    let tagged_wire = is_enum.then(|| {
        quote! {
            #[doc(hidden)] #[repr(C)] #[derive(Clone, Copy)]
            /// Raw alternatives, inspected only after validating the active tag.
            #visibility union #union_name { #(#members),* }
            #[doc(hidden)] #[repr(C)] #[derive(Clone, Copy)]
            /// Explicit integer tag and recursively lowered active payload.
            #visibility struct #wire_name {
                /// Declared variant discriminant; unknown values fail conversion.
                pub tag: i64,
                /// Storage for the alternative selected by tag.
                pub payload: #union_name,
            }
        }
    });
    let storage_type = if inline {
        quote! { () }
    } else {
        quote! { #storage_name }
    };
    let storage = (!inline).then(|| {
        quote! {
            #[doc(hidden)]
            /// Concrete ownership of converted indirect fields; never erased.
            #visibility enum #storage_name { #(#storages),* }
        }
    });
    let decode_body = if is_enum {
        quote! { match wire.tag { #(#decode),*, _ => Err(#nc::NC_E_INVALID_TAG) } }
    } else {
        quote! { #(#decode)* }
    };
    let encode_source = if inline {
        quote! { *self }
    } else {
        quote! { self }
    };
    let inline_impl = inline.then(|| {
        quote! {
            // SAFETY: every field is statically required to have self-contained wire storage.
            unsafe impl #nc::ComInlineValue for #name {}
        }
    });
    let inline_checks = inline.then(|| {
        let types = cases.iter().flat_map(|case| case.fields.iter().map(|field| &field.ty));
        quote! { const _: () = { fn check<T: #nc::ComInlineValue>() {} fn fields() { #(check::<#types>();)* } }; }
    });
    quote! {
        #(#wires)* #tagged_wire #storage #inline_checks
        // SAFETY: only raw field representations cross COM; recursive conversion
        // validates tags and values before constructing application data.
        unsafe impl #nc::ComValue for #name {
            type Wire = #wire_name;
            type Storage = #storage_type;
            /// Retains the storage required by every recursively converted child.
            fn encode_value(&self, budget: &mut #nc::ValueBudget) -> Result<(Self::Wire, Self::Storage), #nc::NcStatus> {
                budget.value(|budget| match #encode_source { #(#encode),* })
            }
            /// Selects the valid alternative before decoding any application field.
            unsafe fn decode_value(wire: Self::Wire, budget: &mut #nc::ValueBudget) -> Result<Self, #nc::NcStatus> {
                budget.value(|budget| { #decode_body })
            }
        }
        impl #nc::ComValueType for #name {
            const VALUE_SIGNATURE: #nc::NcAbiSignature = #signature;
        }
        #inline_impl
    }
}
