//! One recursive field converter shared by copied and owned declaration macros.
//!
//! Application fields never double as unchecked foreign wire fields. This module
//! emits raw C records, a tag-selected union for enums, and concrete backing
//! guards. Field traversal always delegates to the same ComValue contract.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, Ident, Visibility};

use crate::util::private_ident;

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
    let wire_name = format_ident!("__{name}ValueWire", span = name.span());
    let union_name = format_ident!("__{name}ValuePayload", span = name.span());
    let storage_name = format_ident!("__{name}ValueStorage", span = name.span());
    // These are implementation locals, never authored field bindings. Positional
    // renaming remains correct when generated tokens are serialized as Rust text.
    let budget = private_ident("__nc_budget");
    let wire_input = private_ident("__nc_wire");
    let raw_record = private_ident("__nc_raw_record");
    let cost = private_ident("__nc_cost");
    let is_enum = cases.first().is_some_and(|case| case.variant.is_some());
    let mut wires = Vec::new();
    let mut members = Vec::new();
    let mut storages = Vec::new();
    let mut encode = Vec::new();
    let mut decode = Vec::new();
    let mut encode_prechecked = Vec::new();
    let mut decode_prechecked = Vec::new();
    let mut fixed_costs = Vec::new();
    let mut maximum_costs = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let case_wire = if is_enum {
            format_ident!("__{name}ValueCase{index}", span = name.span())
        } else {
            wire_name.clone()
        };
        let case_storage = format_ident!("Case{index}");
        let member = format_ident!("case_{index}");
        let mut raw_fields = Vec::new();
        let mut stored_fields = Vec::new();
        let mut bindings = Vec::new();
        let mut field_members = Vec::new();
        let mut encoding = Vec::new();
        let mut decoding = Vec::new();
        let mut encoding_fixed = Vec::new();
        let mut decoding_fixed = Vec::new();
        let mut raw_values = Vec::new();
        let mut stored_values = Vec::new();
        for (position, field) in case.fields.iter().enumerate() {
            // Preserve the public member, but never introduce it as a local.
            // Prefixes alone cannot prevent an author from choosing the same name.
            let member = field.ident.clone().unwrap_or_else(|| format_ident!("field_{position}"));
            let binding = private_ident(&format!("__nc_field_{position}"));
            let raw = private_ident(&format!("__nc_wire_field_{position}"));
            let stored = private_ident(&format!("__nc_storage_field_{position}"));
            let ty = &field.ty;
            raw_fields.push(quote! { /// Recursively checked raw field; never an application reference.
            pub #member: <#ty as #nc::ComValue>::Wire });
            stored_fields.push(quote! { /// Retains this field's converted indirect storage through the call.
            #member: <#ty as #nc::ComValue>::Storage });
            // A local copy allows packed application fields to be borrowed safely.
            let field_value = if inline {
                quote! { &#binding }
            } else {
                quote! { #binding }
            };
            encoding.push(quote! {
                let (#raw, #stored) = match <#ty as #nc::ComValue>::FIXED_COST {
                    // SAFETY: this alternative's aggregate preflight includes every fixed child.
                    ::core::option::Option::Some(_) => unsafe { <#ty as #nc::ComValue>::encode_prechecked(#field_value) },
                    ::core::option::Option::None => <#ty as #nc::ComValue>::encode_value(#field_value, #budget),
                }?;
            });
            decoding.push(quote! {
                match <#ty as #nc::ComValue>::FIXED_COST {
                    // SAFETY: the selected raw field is live and its cost was included
                    // in the enclosing preflight. Content validation still happens here.
                    ::core::option::Option::Some(_) => unsafe { <#ty as #nc::ComValue>::decode_prechecked(#raw_record.#member) },
                    // SAFETY: this field's active backing remains live for conversion.
                    ::core::option::Option::None => unsafe { <#ty as #nc::ComValue>::decode_value(#raw_record.#member, #budget) },
                }?
            });
            encoding_fixed.push(quote! {
                // SAFETY: the enclosing resource preflight includes this active child.
                let (#raw, #stored) = unsafe { <#ty as #nc::ComValue>::encode_prechecked(#field_value) }?;
            });
            decoding_fixed.push(quote! {
                // SAFETY: the selected field is initialized, its resource requirements were checked,
                // and this converter still validates every restricted raw value.
                unsafe { <#ty as #nc::ComValue>::decode_prechecked(#raw_record.#member) }?
            });
            raw_values.push(quote! { #member: #raw });
            stored_values.push(quote! { #member: #stored });
            field_members.push(member);
            bindings.push(binding);
        }
        let constructor = match &case.variant {
            Some(variant) => quote! { Self::#variant },
            None => quote! { Self },
        };
        let (pattern, decoded) = match &case.fields {
            Fields::Unit => (constructor.clone(), constructor.clone()),
            Fields::Named(_) => (
                quote! { #constructor { #(#field_members: #bindings),* } },
                quote! { #constructor { #(#field_members: #decoding),* } },
            ),
            Fields::Unnamed(_) => (quote! { #constructor(#(#bindings),*) }, quote! { #constructor(#(#decoding),*) }),
        };
        let decoded_fixed = match &case.fields {
            Fields::Unit => constructor,
            Fields::Named(_) => quote! { #constructor { #(#field_members: #decoding_fixed),* } },
            Fields::Unnamed(_) => quote! { #constructor(#(#decoding_fixed),*) },
        };
        let field_types: Vec<_> = case.fields.iter().map(|field| &field.ty).collect();
        // Only the active alternative contributes values. Fixed children are
        // counted here; dynamic children receive the shared budget individually.
        let known_cost = quote! { #nc::ValueCost::known_fields(&[#(<#field_types as #nc::ComValue>::FIXED_COST),*]) };
        fixed_costs.push(quote! { #nc::ValueCost::aggregate(&[#(<#field_types as #nc::ComValue>::FIXED_COST),*]) });
        maximum_costs.push(quote! { #nc::ValueCost::aggregate(&[#(<#field_types as #nc::ComValue>::MAX_COST),*]) });
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
        let constructed_wire = if is_enum {
            quote! { #wire_name { tag: #tag, payload: #union_name { #member: #raw_value } } }
        } else {
            raw_value
        };
        let stored = if inline {
            quote! { () }
        } else {
            quote! { #storage_name::#case_storage { #(#stored_values),* } }
        };
        encode.push(quote! { #pattern => {
            const COST: ::core::option::Option<#nc::ValueCost> = #known_cost;
            #budget.aggregate(COST.ok_or(#nc::NC_E_OUT_OF_RANGE)?, |#budget| {
                #(#encoding)* ::core::result::Result::Ok((#constructed_wire, #stored))
            })
        } });
        encode_prechecked.push(quote! { #pattern => { #(#encoding_fixed)* ::core::result::Result::Ok((#constructed_wire, #stored)) } });
        let read = (!bindings.is_empty()).then(|| {
            if is_enum {
                quote! { // SAFETY: this match arm validated the tag before selecting a payload.
                let #raw_record = unsafe { #wire_input.payload.#member }; }
            } else {
                quote! { let #raw_record = #wire_input; }
            }
        });
        decode_prechecked.push(if is_enum {
            quote! { #tag => { #read ::core::result::Result::Ok(#decoded_fixed) } }
        } else {
            quote! { #read ::core::result::Result::Ok(#decoded_fixed) }
        });
        let decode_case = quote! {
            const COST: ::core::option::Option<#nc::ValueCost> = #known_cost;
            #budget.aggregate(COST.ok_or(#nc::NC_E_OUT_OF_RANGE)?, |#budget| {
                #read ::core::result::Result::Ok(#decoded)
            })
        };
        decode.push(if is_enum {
            quote! { #tag => { #decode_case } }
        } else {
            decode_case
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
        quote! { match #wire_input.tag {
            #(#decode),*,
            // An unknown tag still consumes the enum root before being rejected.
            _ => #budget.value(|_| ::core::result::Result::Err(#nc::NC_E_INVALID_TAG)),
        } }
    } else {
        quote! { #(#decode)* }
    };
    let decode_prechecked_body = if is_enum {
        quote! { match #wire_input.tag { #(#decode_prechecked),*, _ => ::core::result::Result::Err(#nc::NC_E_INVALID_TAG) } }
    } else {
        quote! { #(#decode_prechecked)* }
    };
    let fixed_cost = if is_enum {
        quote! { #nc::ValueCost::alternatives(&[#(#fixed_costs),*]) }
    } else {
        fixed_costs[0].clone()
    };
    let maximum_cost = if is_enum {
        quote! { #nc::ValueCost::maximum(&[#(#maximum_costs),*]) }
    } else {
        maximum_costs[0].clone()
    };
    let borrow_wire = if !is_enum && inline {
        let fields = cases.iter().flat_map(|case| case.fields.iter()).map(|field| &field.ty);
        let offsets = cases.iter().flat_map(|case| case.fields.iter().enumerate()).map(|(index, field)| {
            let application = field.ident.as_ref().map(|name| quote! { #name }).unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote! { #index }
            });
            let raw = field.ident.clone().unwrap_or_else(|| format_ident!("field_{index}"));
            quote! { ::core::mem::offset_of!(Self, #application) == ::core::mem::offset_of!(Self::Wire, #raw) }
        });
        quote! { const BORROW_WIRE: bool = true #(&& <#fields as #nc::ComValue>::BORROW_WIRE)*
        && ::core::mem::size_of::<Self>() == ::core::mem::size_of::<Self::Wire>()
        && ::core::mem::align_of::<Self>() == ::core::mem::align_of::<Self::Wire>()
        #(&& #offsets)*; }
    } else {
        quote! {}
    };
    let direct_encode = inline.then(|| {
        quote! {
            if Self::BORROW_WIRE {
                // SAFETY: BORROW_WIRE proves identical field offsets/size/alignment,
                // unrestricted initialized bits, and no ownership or conversion work.
                let #wire_input = unsafe { ::core::ptr::from_ref(self).cast::<Self::Wire>().read() };
                return ::core::result::Result::Ok((#wire_input, ()));
            }
        }
    });
    let direct_decode = inline.then(|| {
        quote! {
            if Self::BORROW_WIRE {
                // SAFETY: the recursive proof excludes bools/tags and equates every
                // active field layout. Padding stays padding in the copied value.
                return ::core::result::Result::Ok(unsafe { ::core::ptr::from_ref(&#wire_input).cast::<Self>().read() });
            }
        }
    });
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
            #borrow_wire
            const FIXED_COST: ::core::option::Option<#nc::ValueCost> = #fixed_cost;
            const MAX_COST: ::core::option::Option<#nc::ValueCost> = #maximum_cost;
            type Wire = #wire_name;
            type Storage = #storage_type;
            /// Retains the storage required by every recursively converted child.
            /// Keep the budgeted traversal shared; isolated small copies use prechecked conversion.
            #[inline(never)]
            fn encode_value(&self, #budget: &mut #nc::ValueBudget) -> ::core::result::Result<(Self::Wire, Self::Storage), #nc::NcStatus> {
                match Self::FIXED_COST {
                    ::core::option::Option::Some(#cost) => {
                        #budget.fixed(#cost)?;
                        // SAFETY: this exact allocation-free subtree has been charged.
                        unsafe { Self::encode_prechecked(self) }
                    }
                    ::core::option::Option::None => match #encode_source { #(#encode),* },
                }
            }
            /// Selects the valid alternative before decoding any application field.
            /// A shared traversal avoids duplicating its budget handling into every buffer adapter.
            #[inline(never)]
            unsafe fn decode_value(#wire_input: Self::Wire, #budget: &mut #nc::ValueBudget) -> ::core::result::Result<Self, #nc::NcStatus> {
                match Self::FIXED_COST {
                    ::core::option::Option::Some(#cost) => {
                        #budget.fixed(#cost)?;
                        // SAFETY: raw backing satisfies this codec and cost was charged.
                        unsafe { Self::decode_prechecked(#wire_input) }
                    }
                    ::core::option::Option::None => { #decode_body },
                }
            }
            /// Converts fields through their same validators after the resource preflight.
            #[inline]
            unsafe fn encode_prechecked(&self) -> ::core::result::Result<(Self::Wire, Self::Storage), #nc::NcStatus> {
                #direct_encode
                match #encode_source { #(#encode_prechecked),* }
            }
            /// Checks tags and every field after the whole conversion was proved to fit.
            #[inline]
            unsafe fn decode_prechecked(#wire_input: Self::Wire) -> ::core::result::Result<Self, #nc::NcStatus> {
                #direct_decode
                #decode_prechecked_body
            }
        }
        impl #nc::ComValueType for #name {
            const VALUE_SIGNATURE: #nc::NcAbiSignature = #signature;
        }
        #inline_impl
    }
}

/// Parses owned declaration policy before sharing the ordinary field converter.
pub(crate) fn expand_owned(args: crate::args::StructArgs, input: syn::DeriveInput) -> syn::Result<TokenStream> {
    crate::util::validate_no_generics(&input.generics, "IDL values")?;
    let nc = crate::util::runtime_crate_path(args.runtime_crate.as_ref());
    let signature = args
        .assigned_signature
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(&input, "com_value requires an IDL assigned_signature"))?;
    let deserialize = if args.deserialize {
        match &input.data {
            syn::Data::Struct(record) => {
                crate::deserialize::expand_struct(&input.ident, &record.fields, &nc, args.deserialize_default, args.deserialize_sequence)?
            }
            syn::Data::Enum(enumeration) if !args.deserialize_default && !args.deserialize_sequence => {
                crate::deserialize::expand_enum(&input.ident, enumeration, &nc, None)?
            }
            _ => return Err(syn::Error::new_spanned(&input, "default/sequence deserialization requires a struct")),
        }
    } else {
        quote! {}
    };
    let cases = match &input.data {
        syn::Data::Struct(record) => vec![ValueCase {
            variant: None,
            tag: quote! { 0 },
            fields: record.fields.clone(),
        }],
        syn::Data::Enum(enumeration) => enumeration
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                let tag = ordinal_tag(index, variant)?;
                Ok(ValueCase {
                    variant: Some(variant.ident.clone()),
                    tag,
                    fields: variant.fields.clone(),
                })
            })
            .collect::<syn::Result<Vec<_>>>()?,
        syn::Data::Union(_) => return Err(syn::Error::new_spanned(&input, "unions require an explicit enum tag")),
    };
    let codec = expand(
        &input.ident,
        &input.vis,
        &cases,
        quote! { #nc::NcAbiSignature::from_static(#signature) },
        false,
        &nc,
    );
    Ok(quote! { #input #codec #deserialize })
}

/// Assigns one wire/persistence ordinal without a second author-controlled tag path.
/// Both copied and owning enum macros call this helper; IDL enforces the same
/// restriction before specialization, and never emits override attributes.
pub(crate) fn ordinal_tag(index: usize, variant: &syn::Variant) -> syn::Result<TokenStream> {
    if let Some(attribute) = variant.attrs.iter().find(|attribute| attribute.path().is_ident("com_tag")) {
        return Err(syn::Error::new_spanned(
            attribute,
            "explicit payload tags are not supported; tags follow zero-based declaration order",
        ));
    }
    if let Some((_, value)) = &variant.discriminant {
        return Err(syn::Error::new_spanned(
            value,
            "explicit payload discriminants are not supported; tags follow zero-based declaration order",
        ));
    }
    let ordinal = crate::util::checked_u32_len(index, variant.ident.span(), "enum variant ordinal")?;
    let ordinal = proc_macro2::Literal::u64_unsuffixed(u64::from(ordinal));
    Ok(quote! { #ordinal })
}
