//! Expansion and validation for fieldless and payload-bearing COM enums.
//!
//! Fieldless enums become transparent integer newtypes that preserve unknown
//! values. Payload-bearing enums become a C-compatible tag plus union storage,
//! with safe constructors and checked conversion back to Rust variants. Layout
//! metadata includes every variant tag and payload ABI identity.

use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Attribute, Expr, Fields, Ident, ItemEnum, Lit, LitInt, LitStr, Result, UnOp};

use crate::args::EnumArgs;
use crate::deserialize::expand_struct as expand_struct_deserialize;
use crate::plain_data::{PlainAbiTypeContext, validate_plain_abi_fields};
use crate::util::{abi_signature_static_tokens, checked_u32_len, push_error, runtime_crate_path, validate_no_generics};

/// Validates an enum and emits its stable raw representation and conversions.
pub(crate) fn expand_com_enum(args: EnumArgs, mut input: ItemEnum) -> Result<proc_macro2::TokenStream> {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    validate_com_enum(&input)?;
    let enum_ident = &input.ident;
    checked_u32_len(input.variants.len(), enum_ident.span(), "COM enum variant count")?;
    for variant in &input.variants {
        let field_count = match &variant.fields {
            Fields::Unit => 0,
            Fields::Unnamed(fields) => fields.unnamed.len(),
            Fields::Named(fields) => fields.named.len(),
        };
        checked_u32_len(field_count, variant.ident.span(), "COM enum payload field count")?;
    }
    let vis = &input.vis;
    let tag_ty = &args.tag;
    let abi_ident = format_ident!("{enum_ident}Abi");
    let tag_ident = format_ident!("{enum_ident}Tag");
    let payload_ident = format_ident!("{enum_ident}Payload");

    let mut variants = Vec::new();
    for (index, variant) in input.variants.iter_mut().enumerate() {
        let tag_value = variant_tag(&variant.attrs)?.unwrap_or_else(|| {
            let literal = LitInt::new(&(index + 1).to_string(), variant.ident.span());
            syn::parse_quote! { #literal }
        });
        variant.attrs = strip_com_enum_attrs(&variant.attrs);
        variants.push((variant.ident.clone(), variant.fields.clone(), tag_value));
    }
    validate_unique_literal_tags(&variants)?;
    validate_com_enum_payloads(&variants)?;
    validate_unique_union_payload_fields(&variants)?;

    let (deserialize_payload_impls, deserialize_impl) = if args.deserialize {
        expand_enum_deserialize(enum_ident, &variants, &nc)?
    } else {
        (Vec::new(), quote! {})
    };

    let tag_count = variants.len();
    let tag_values_for_uniqueness = variants.iter().map(|(_, _, tag_value)| tag_value);
    let tag_uniqueness_assertion = quote! {
        const _: () = {
            let __nc_tags: [i128; #tag_count] = [
                #((#tag_values_for_uniqueness) as i128,)*
            ];
            let mut __nc_i = 0usize;
            while __nc_i < __nc_tags.len() {
                let mut __nc_j = __nc_i + 1;
                while __nc_j < __nc_tags.len() {
                    if __nc_tags[__nc_i] == __nc_tags[__nc_j] {
                        panic!("duplicate #[com_enum] tag value");
                    }
                    __nc_j += 1;
                }
                __nc_i += 1;
            }
        };
    };

    let tag_consts = variants.iter().map(|(variant_ident, _, tag_value)| {
        quote! {
            #[doc = concat!("Wire tag for [`", stringify!(#enum_ident), "::", stringify!(#variant_ident), "`].")]
            pub const #variant_ident: Self = Self(#tag_value);
        }
    });

    let payload_structs = variants.iter().filter_map(|(variant_ident, fields, _)| match fields {
        Fields::Unit => None,
        Fields::Unnamed(unnamed) => {
            let struct_ident = variant_payload_ident(enum_ident, variant_ident);
            let field_layout_parts = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let index = index as u32;
                    let field_ident = tuple_payload_field(index as usize);
                    let ty = &field.ty;
                    quote! {
                        #nc::NcAbiFieldLayout {
                            index: #index,
                            offset: core::mem::offset_of!(#struct_ident, #field_ident),
                            ty: #nc::__nc_abi_plain_type::<#ty>(),
                        }
                    }
                })
                .collect::<Vec<_>>();
            let mut signature_parts = vec![
                quote! { #nc::NcAbiSignaturePart::Aggregate { kind: #nc::NcAbiAggregateKind::Struct } },
                quote! {
                    #nc::NcAbiSignaturePart::Layout {
                        size: core::mem::size_of::<#struct_ident>(),
                        align: core::mem::align_of::<#struct_ident>(),
                    }
                },
            ];
            signature_parts.extend(unnamed.unnamed.iter().enumerate().map(|(index, field)| {
                let index = index as u32;
                let field_ident = tuple_payload_field(index as usize);
                let ty = &field.ty;
                quote! {
                    #nc::NcAbiSignaturePart::Field {
                        index: #index,
                        offset: core::mem::offset_of!(#struct_ident, #field_ident),
                        ty: #nc::__nc_abi_plain_type::<#ty>(),
                    }
                }
            }));
            let parts_ident = format_ident!("__nc_{struct_ident}_abi_signature_parts");
            let bytes_ident = format_ident!("__nc_{struct_ident}_abi_signature_bytes");
            let signature_static = abi_signature_static_tokens(&parts_ident, &bytes_ident, &signature_parts, &nc);
            let struct_fields = unnamed.unnamed.iter().enumerate().map(|(index, field)| {
                let field_ident = tuple_payload_field(index);
                let ty = &field.ty;
                // Preserve author semantics on the public wire field while
                // documenting its generated position even without source docs.
                let docs = field.attrs.iter().filter(|attr| attr.path().is_ident("doc"));
                quote! {
                    #[doc = concat!("Copied tuple payload field ", stringify!(#index), ".")]
                    #(#docs)*
                    pub #field_ident: #ty
                }
            });
            Some(quote! {
                #signature_static

                #[repr(C)]
                #[derive(Clone, Copy)]
                #[doc = concat!("ABI payload for [`", stringify!(#enum_ident), "::", stringify!(#variant_ident), "`].")]
                #vis struct #struct_ident {
                    #(#struct_fields,)*
                }

                // SAFETY: The generated payload struct is `repr(C)`, and
                // layout metadata is computed from compiler layout facts.
                unsafe impl #nc::ComStruct for #struct_ident {
                    const ABI_SIGNATURE: #nc::NcAbiSignature =
                        #nc::NcAbiSignature::from_static(&#bytes_ident);

                    const ABI_STRUCT_LAYOUT: #nc::NcAbiStructLayout = #nc::NcAbiStructLayout {
                        size: core::mem::size_of::<Self>(),
                        align: core::mem::align_of::<Self>(),
                        fields: &[
                            #(#field_layout_parts,)*
                        ],
                    };
                }

                // SAFETY: The generated ABI type signature requires every
                // payload field to be a plain ABI value and includes the
                // payload's physical layout.
                unsafe impl #nc::AbiType for #struct_ident {
                    const ABI_TYPE: #nc::NcAbiType = #nc::__nc_abi_com_struct_type::<Self>();
                }
                // SAFETY: The payload is a fixed-layout aggregate of plain
                // ABI values with no release protocol.
                unsafe impl #nc::AbiPlainValue for #struct_ident {}
            })
        }
        Fields::Named(named) => {
            let struct_ident = variant_payload_ident(enum_ident, variant_ident);
            let field_layout_parts = named
                .named
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let index = index as u32;
                    let field_ident = field.ident.as_ref().expect("named field has an ident");
                    let ty = &field.ty;
                    quote! {
                        #nc::NcAbiFieldLayout {
                            index: #index,
                            offset: core::mem::offset_of!(#struct_ident, #field_ident),
                            ty: #nc::__nc_abi_plain_type::<#ty>(),
                        }
                    }
                })
                .collect::<Vec<_>>();
            let mut signature_parts = vec![
                quote! { #nc::NcAbiSignaturePart::Aggregate { kind: #nc::NcAbiAggregateKind::Struct } },
                quote! {
                    #nc::NcAbiSignaturePart::Layout {
                        size: core::mem::size_of::<#struct_ident>(),
                        align: core::mem::align_of::<#struct_ident>(),
                    }
                },
            ];
            signature_parts.extend(named.named.iter().enumerate().map(|(index, field)| {
                let index = index as u32;
                let field_ident = field.ident.as_ref().expect("named field has an ident");
                let ty = &field.ty;
                quote! {
                    #nc::NcAbiSignaturePart::Field {
                        index: #index,
                        offset: core::mem::offset_of!(#struct_ident, #field_ident),
                        ty: #nc::__nc_abi_plain_type::<#ty>(),
                    }
                }
            }));
            let parts_ident = format_ident!("__nc_{struct_ident}_abi_signature_parts");
            let bytes_ident = format_ident!("__nc_{struct_ident}_abi_signature_bytes");
            let signature_static = abi_signature_static_tokens(&parts_ident, &bytes_ident, &signature_parts, &nc);
            let struct_fields = named.named.iter().map(|field| {
                let field_ident = field.ident.as_ref().expect("named field has an ident");
                let ty = &field.ty;
                // Only documentation is copied: conditional/layout attributes
                // must not silently change the separately validated ABI shape.
                let docs = field.attrs.iter().filter(|attr| attr.path().is_ident("doc"));
                quote! {
                    #[doc = concat!("Copied payload field `", stringify!(#field_ident), "`.")]
                    #(#docs)*
                    pub #field_ident: #ty
                }
            });
            Some(quote! {
                #signature_static

                #[repr(C)]
                #[derive(Clone, Copy)]
                #[doc = concat!("ABI payload for [`", stringify!(#enum_ident), "::", stringify!(#variant_ident), "`].")]
                #vis struct #struct_ident {
                    #(#struct_fields,)*
                }

                // SAFETY: The generated payload struct is `repr(C)`, and
                // layout metadata is computed from compiler layout facts.
                unsafe impl #nc::ComStruct for #struct_ident {
                    const ABI_SIGNATURE: #nc::NcAbiSignature =
                        #nc::NcAbiSignature::from_static(&#bytes_ident);

                    const ABI_STRUCT_LAYOUT: #nc::NcAbiStructLayout = #nc::NcAbiStructLayout {
                        size: core::mem::size_of::<Self>(),
                        align: core::mem::align_of::<Self>(),
                        fields: &[
                            #(#field_layout_parts,)*
                        ],
                    };
                }

                // SAFETY: The generated ABI type signature requires every
                // payload field to be a plain ABI value and includes the
                // payload's physical layout.
                unsafe impl #nc::AbiType for #struct_ident {
                    const ABI_TYPE: #nc::NcAbiType = #nc::__nc_abi_com_struct_type::<Self>();
                }
                // SAFETY: The payload is a fixed-layout aggregate of plain
                // ABI values with no release protocol.
                unsafe impl #nc::AbiPlainValue for #struct_ident {}
            })
        }
    });

    let payload_struct_idents = variants
        .iter()
        .filter(|(_, fields, _)| !matches!(fields, Fields::Unit))
        .map(|(variant_ident, _, _)| variant_payload_ident(enum_ident, variant_ident))
        .collect::<Vec<_>>();
    let mut payload_union_signature_parts = vec![
        quote! { #nc::NcAbiSignaturePart::Aggregate { kind: #nc::NcAbiAggregateKind::Union } },
        quote! {
            #nc::NcAbiSignaturePart::Layout {
                size: core::mem::size_of::<#payload_ident>(),
                align: core::mem::align_of::<#payload_ident>(),
            }
        },
    ];
    payload_union_signature_parts.extend(payload_struct_idents.iter().enumerate().map(|(index, struct_ident)| {
        let index = index as u32;
        quote! {
            #nc::NcAbiSignaturePart::Field {
                index: #index,
                offset: 0usize,
                ty: #nc::__nc_abi_plain_type::<#struct_ident>(),
            }
        }
    }));
    let payload_union_parts_ident = format_ident!("__nc_{payload_ident}_abi_signature_parts");
    let payload_union_bytes_ident = format_ident!("__nc_{payload_ident}_abi_signature_bytes");
    let payload_union_signature_static =
        abi_signature_static_tokens(&payload_union_parts_ident, &payload_union_bytes_ident, &payload_union_signature_parts, &nc);

    let variant_signature_parts = variants
        .iter()
        .enumerate()
        .map(|(index, (variant_ident, fields, tag_value))| {
            let index = index as u32;
            let ty = match fields {
                Fields::Unit => quote! { #nc::NcAbiType::Void },
                Fields::Unnamed(_) | Fields::Named(_) => {
                    let struct_ident = variant_payload_ident(enum_ident, variant_ident);
                    quote! { #nc::__nc_abi_plain_type::<#struct_ident>() }
                }
            };
            quote! {
                #nc::NcAbiSignaturePart::Variant {
                    index: #index,
                    tag: (#tag_value) as i128,
                    ty: #ty,
                }
            }
        })
        .collect::<Vec<_>>();
    let mut tagged_enum_signature_parts = vec![
        quote! { #nc::NcAbiSignaturePart::Aggregate { kind: #nc::NcAbiAggregateKind::TaggedEnum } },
        quote! {
            #nc::NcAbiSignaturePart::Layout {
                size: core::mem::size_of::<#abi_ident>(),
                align: core::mem::align_of::<#abi_ident>(),
            }
        },
        quote! {
            #nc::NcAbiSignaturePart::Field {
                index: 0,
                offset: #abi_ident::TAG_OFFSET,
                ty: #nc::__nc_abi_plain_type::<#tag_ident>(),
            }
        },
        quote! {
            #nc::NcAbiSignaturePart::Field {
                index: 1,
                offset: #abi_ident::PAYLOAD_OFFSET,
                ty: #nc::__nc_abi_plain_type::<#payload_ident>(),
            }
        },
    ];
    tagged_enum_signature_parts.extend(variant_signature_parts.iter().cloned());
    let tagged_enum_parts_ident = format_ident!("__nc_{abi_ident}_abi_signature_parts");
    let tagged_enum_bytes_ident = format_ident!("__nc_{abi_ident}_abi_signature_bytes");
    let tagged_enum_signature_static = abi_signature_static_tokens(&tagged_enum_parts_ident, &tagged_enum_bytes_ident, &tagged_enum_signature_parts, &nc);

    let union_fields = variants.iter().filter_map(|(variant_ident, fields, _)| {
        if matches!(fields, Fields::Unit) {
            return None;
        }
        let field_ident = union_payload_field(variant_ident);
        let struct_ident = variant_payload_ident(enum_ident, variant_ident);
        Some(quote! {
            #[doc = concat!("Wire payload selected by [`", stringify!(#enum_ident), "::", stringify!(#variant_ident), "`].")]
            pub #field_ident: core::mem::ManuallyDrop<#struct_ident>
        })
    });
    let empty_union_field = quote! {
        #[doc = "Unit payload for variants without data and for opaque unknown tags."]
        pub _empty: ()
    };

    let from_arms = variants.iter().map(|(variant_ident, fields, _)| {
        let tag_const = variant_ident;
        match fields {
            Fields::Unit => quote! {
                #enum_ident::#variant_ident => unsafe {
                    // SAFETY: The unit variant has no payload member to read.
                    Self::from_raw_parts_unchecked(
                        #tag_ident::#tag_const,
                        #payload_ident { _empty: () },
                    )
                }
            },
            Fields::Unnamed(fields) => {
                let bindings = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(index, _)| format_ident!("__field_{index}"))
                    .collect::<Vec<_>>();
                let struct_ident = variant_payload_ident(enum_ident, variant_ident);
                let union_field = union_payload_field(variant_ident);
                let payload_fields = bindings.iter().enumerate().map(|(index, binding)| {
                    let field_ident = tuple_payload_field(index);
                    quote! { #field_ident: #binding }
                });
                quote! {
                    #enum_ident::#variant_ident(#(#bindings),*) => unsafe {
                        // SAFETY: This arm initializes exactly the payload member
                        // selected by its generated tag from valid Rust fields.
                        Self::from_raw_parts_unchecked(
                            #tag_ident::#tag_const,
                            #payload_ident {
                            #union_field: core::mem::ManuallyDrop::new(#struct_ident {
                                #(#payload_fields,)*
                            }),
                            },
                        )
                    }
                }
            }
            Fields::Named(fields) => {
                let bindings = fields
                    .named
                    .iter()
                    .map(|field| field.ident.as_ref().expect("named field has an ident"))
                    .collect::<Vec<_>>();
                let struct_ident = variant_payload_ident(enum_ident, variant_ident);
                let union_field = union_payload_field(variant_ident);
                quote! {
                    #enum_ident::#variant_ident { #(#bindings),* } => unsafe {
                        // SAFETY: This arm initializes exactly the payload member
                        // selected by its generated tag from valid Rust fields.
                        Self::from_raw_parts_unchecked(
                            #tag_ident::#tag_const,
                            #payload_ident {
                                #union_field: core::mem::ManuallyDrop::new(#struct_ident {
                                    #(#bindings,)*
                                }),
                            },
                        )
                    }
                }
            }
        }
    });

    let try_from_arms = variants.iter().map(|(variant_ident, fields, _)| {
        let tag_const = variant_ident;
        match fields {
            Fields::Unit => quote! {
                tag if tag == #tag_ident::#tag_const => Ok(#enum_ident::#variant_ident)
            },
            Fields::Unnamed(fields) => {
                let union_field = union_payload_field(variant_ident);
                let field_accesses = fields.unnamed.iter().enumerate().map(|(index, _)| {
                    let field_ident = tuple_payload_field(index);
                    quote! { __payload.#field_ident }
                });
                quote! {
                    tag if tag == #tag_ident::#tag_const => {
                        let __payload = unsafe {
                            core::mem::ManuallyDrop::into_inner(payload.#union_field)
                        };
                        Ok(#enum_ident::#variant_ident(#(#field_accesses),*))
                    }
                }
            }
            Fields::Named(fields) => {
                let union_field = union_payload_field(variant_ident);
                let bindings = fields.named.iter().map(|field| {
                    let field_ident = field.ident.as_ref().expect("named field has an ident");
                    quote! { #field_ident: __payload.#field_ident }
                });
                quote! {
                    tag if tag == #tag_ident::#tag_const => {
                        let __payload = unsafe {
                            core::mem::ManuallyDrop::into_inner(payload.#union_field)
                        };
                        Ok(#enum_ident::#variant_ident { #(#bindings),* })
                    }
                }
            }
        }
    });
    let known_tag_arms = variants.iter().map(|(variant_ident, _, _)| {
        quote! {
            tag if tag == #tag_ident::#variant_ident => Err(#nc::NC_E_INVALID_ARG)
        }
    });

    Ok(quote! {
        #input

        #tag_uniqueness_assertion

        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[doc = concat!("ABI tag for [`", stringify!(#enum_ident), "`].")]
        #vis struct #tag_ident(
            #[doc = "Raw integer wire tag; unknown values remain representable."]
            pub #tag_ty
        );

        // SAFETY: The transparent tag has exactly the representation of
        // `#tag_ty`, and the where-clause requires that type to be ABI-safe.
        unsafe impl #nc::AbiType for #tag_ident
        where
            #tag_ty: #nc::AbiType,
        {
            const ABI_TYPE: #nc::NcAbiType = <#tag_ty as #nc::AbiType>::ABI_TYPE;
        }

        // SAFETY: The tag stores only a plain ABI scalar with no ownership or
        // release protocol.
        unsafe impl #nc::AbiPlainValue for #tag_ident
        where
            #tag_ty: #nc::AbiPlainValue,
        {}

        #[allow(non_upper_case_globals)]
        impl #tag_ident {
            #(#tag_consts)*

            #[doc = "Wraps an arbitrary raw tag value without interpreting it."]
            pub const fn from_raw(value: #tag_ty) -> Self {
                Self(value)
            }

            #[doc = "Returns the underlying integer tag value."]
            pub const fn raw(self) -> #tag_ty {
                self.0
            }
        }

        #(#payload_structs)*

        #(#deserialize_payload_impls)*

        #payload_union_signature_static
        #tagged_enum_signature_static

        #[repr(C)]
        #[derive(Clone, Copy)]
        #[doc = concat!("ABI payload union for [`", stringify!(#enum_ident), "`].")]
        #vis union #payload_ident {
            #(#union_fields,)*
            #empty_union_field
        }

        // SAFETY: The union contains only generated payload structs whose ABI
        // type signatures require plain ABI values.
        unsafe impl #nc::AbiType for #payload_ident
        where
            #(#payload_struct_idents: #nc::AbiType,)*
        {
            const ABI_TYPE: #nc::NcAbiType = #nc::NcAbiType::Aggregate(
                #nc::NcAbiSignature::from_static(&#payload_union_bytes_ident),
            );
        }

        // SAFETY: The union has no active drop glue; all possible payloads are
        // plain ABI values.
        unsafe impl #nc::AbiPlainValue for #payload_ident
        where
            #(#payload_struct_idents: #nc::AbiPlainValue,)*
        {}

        // The core-owned storage fields are private outside `neocogi-com-core`.
        // Wrapping that storage prevents safe code in the macro invocation's
        // own module from forging a tag/payload pair, while `repr(transparent)`
        // preserves the exact C ABI of the inner `repr(C)` pair.
        #[repr(transparent)]
        #[derive(Clone, Copy)]
        #[doc = concat!(
            "Opaque ABI representation for [`",
            stringify!(#enum_ident),
            "`]. Use `From` for known variants, `from_unknown_tag` for an unknown tag, or the unsafe raw-parts constructor for low-level FFI adapters."
        )]
        #vis struct #abi_ident {
            storage: #nc::__NcTaggedEnumAbiStorage<#tag_ident, #payload_ident>,
        }

        impl #abi_ident {
            #[doc = "Byte offset of the tag in the C-compatible ABI representation."]
            pub const TAG_OFFSET: usize =
                #nc::__NcTaggedEnumAbiStorage::<#tag_ident, #payload_ident>::TAG_OFFSET;

            #[doc = "Byte offset of the payload union in the C-compatible ABI representation."]
            pub const PAYLOAD_OFFSET: usize =
                #nc::__NcTaggedEnumAbiStorage::<#tag_ident, #payload_ident>::PAYLOAD_OFFSET;

            #[doc = "Returns the ABI tag without inspecting the payload union."]
            pub const fn tag(&self) -> #tag_ident {
                self.storage.tag()
            }

            #[doc = "Borrows the raw payload union without selecting a union member."]
            pub const fn payload(&self) -> &#payload_ident {
                self.storage.payload()
            }

            #[doc = "Consumes the value and returns its raw tag and payload union."]
            pub fn into_raw_parts(self) -> (#tag_ident, #payload_ident) {
                self.storage.into_raw_parts()
            }

            #[doc = "Creates an opaque ABI value for an unknown tag without an active payload."]
            #[doc = ""]
            #[doc = "Returns `NC_E_INVALID_ARG` when `tag` names a known variant, because known tags require their corresponding initialized payload."]
            pub fn from_unknown_tag(
                tag: #tag_ident,
            ) -> core::result::Result<Self, #nc::NcStatus> {
                match tag {
                    #(#known_tag_arms,)*
                    _ => {
                        // SAFETY: Unknown tags are rejected by `TryFrom` without
                        // selecting a payload member, so `_empty` is sufficient.
                        Ok(unsafe {
                            Self::from_raw_parts_unchecked(
                                tag,
                                #payload_ident { _empty: () },
                            )
                        })
                    }
                }
            }

            #[doc = "Creates an ABI enum from a raw tag and payload union."]
            #[doc = ""]
            #[doc = "# Safety"]
            #[doc = ""]
            #[doc = "For a known tag, `payload` must have the corresponding union member active, every byte read from that member must be initialized, and every field must contain a valid value of its Rust type. Nested tagged ABI values must satisfy the same invariant. For an unknown tag, callers must not read a payload member selected by that tag."]
            pub const unsafe fn from_raw_parts_unchecked(
                tag: #tag_ident,
                payload: #payload_ident,
            ) -> Self {
                Self {
                    // SAFETY: The caller accepts the same raw-parts validity
                    // contract required by the opaque core storage.
                    storage: unsafe {
                        #nc::__NcTaggedEnumAbiStorage::from_raw_parts_unchecked(tag, payload)
                    },
                }
            }
        }

        // SAFETY: The transparent wrapper has the ABI of the core-owned
        // `repr(C)` tag/payload pair. Safe construction preserves the active
        // member invariant; arbitrary construction requires an unsafe call.
        unsafe impl #nc::AbiType for #abi_ident
        where
            #tag_ident: #nc::AbiType,
            #payload_ident: #nc::AbiType,
        {
            const ABI_TYPE: #nc::NcAbiType = #nc::NcAbiType::Aggregate(
                #nc::NcAbiSignature::from_static(&#tagged_enum_bytes_ident),
            );
        }

        // SAFETY: The representation contains only plain ABI values and has no
        // hidden ownership or release protocol. This marker does not make
        // arbitrary tag/payload combinations valid; safe constructors and the
        // unsafe raw-parts contract preserve that separate invariant.
        unsafe impl #nc::AbiPlainValue for #abi_ident
        where
            #tag_ident: #nc::AbiPlainValue,
            #payload_ident: #nc::AbiPlainValue,
        {}

        impl From<#enum_ident> for #abi_ident {
            #[doc = "Encodes a valid Rust enum variant into its tagged ABI representation."]
            fn from(value: #enum_ident) -> Self {
                match value {
                    #(#from_arms,)*
                }
            }
        }

        impl core::convert::TryFrom<#abi_ident> for #enum_ident {
            type Error = #nc::NcStatus;

            #[doc = "Validates the tag and decodes the corresponding active payload member."]
            fn try_from(value: #abi_ident) -> core::result::Result<Self, Self::Error> {
                let (tag, payload) = value.into_raw_parts();
                match tag {
                    #(#try_from_arms,)*
                    _ => Err(#nc::NC_E_INVALID_TAG),
                }
            }
        }

        #deserialize_impl
    })
}

/// Generates typed source decoding for a COM enum and its payload records.
fn expand_enum_deserialize(
    enum_ident: &Ident,
    variants: &[(Ident, Fields, Expr)],
    nc: &syn::Path,
) -> Result<(Vec<proc_macro2::TokenStream>, proc_macro2::TokenStream)> {
    let mut payload_impls = Vec::new();
    let mut arms = Vec::new();
    let variant_names = variants
        .iter()
        .map(|(variant, _, _)| LitStr::new(&variant.to_string(), variant.span()))
        .collect::<Vec<_>>();

    for (index, (variant_ident, fields, _)) in variants.iter().enumerate() {
        let index = index as u32;
        match fields {
            Fields::Unit => arms.push(quote! {
                #index => {
                    value.unit()?;
                    Ok(Self::#variant_ident)
                }
            }),
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let ty = &fields.unnamed.first().expect("newtype variant has one field").ty;
                arms.push(quote! {
                    #index => Ok(Self::#variant_ident(value.newtype::<#ty>()?))
                });
            }
            Fields::Unnamed(fields) => {
                let payload_ident = variant_payload_ident(enum_ident, variant_ident);
                let mut named_fields = syn::punctuated::Punctuated::new();
                for (field_index, field) in fields.unnamed.iter().enumerate() {
                    let mut field = field.clone();
                    field.ident = Some(tuple_payload_field(field_index));
                    named_fields.push(field);
                }
                let generated_fields = Fields::Named(syn::FieldsNamed {
                    brace_token: Default::default(),
                    named: named_fields,
                });
                payload_impls.push(expand_struct_deserialize(&payload_ident, &generated_fields, nc, false, true)?);
                let payload_fields = fields.unnamed.iter().enumerate().map(|(field_index, _)| {
                    let field = tuple_payload_field(field_index);
                    quote! { payload.#field }
                });
                let length = fields.unnamed.len();
                arms.push(quote! {
                    #index => {
                        let payload: #payload_ident = value.tuple(#length)?;
                        Ok(Self::#variant_ident(#(#payload_fields),*))
                    }
                });
            }
            Fields::Named(fields) => {
                let payload_ident = variant_payload_ident(enum_ident, variant_ident);
                let generated_fields = Fields::Named(fields.clone());
                payload_impls.push(expand_struct_deserialize(&payload_ident, &generated_fields, nc, false, false)?);
                let payload_fields = fields.named.iter().map(|field| {
                    let field = field.ident.as_ref().expect("named field has an identifier");
                    quote! { #field: payload.#field }
                });
                arms.push(quote! {
                    #index => {
                        let payload: #payload_ident = value.record()?;
                        Ok(Self::#variant_ident { #(#payload_fields),* })
                    }
                });
            }
        }
    }

    let implementation = quote! {
        impl #nc::ComDeserialize for #enum_ident {
            fn com_deserialize<'__nc_de, __NcDecoder>(decoder: __NcDecoder) -> Result<Self, __NcDecoder::Error>
            where
                __NcDecoder: #nc::ComDeserializer<'__nc_de>,
            {
                decoder.deserialize_enum::<Self>()
            }
        }

        impl #nc::ComEnum for #enum_ident {
            const NAME: &'static str = stringify!(#enum_ident);
            const VARIANTS: &'static [&'static str] = &[#(#variant_names),*];

            fn read_variant<'__nc_de, __NcVariant>(variant: u32, value: __NcVariant) -> Result<Self, __NcVariant::Error>
            where
                __NcVariant: #nc::ComVariantAccess<'__nc_de>,
            {
                match variant {
                    #(#arms,)*
                    _ => Err(value.invalid_variant(variant, Self::VARIANTS)),
                }
            }
        }
    };
    Ok((payload_impls, implementation))
}

/// Rejects generics, empty enums, and unsupported enum-level syntax.
fn validate_com_enum(input: &ItemEnum) -> Result<()> {
    let mut errors = None;

    if let Err(err) = validate_no_generics(&input.generics, "COM enums") {
        push_error(&mut errors, err);
    }

    if let Some(errors) = errors { Err(errors) } else { Ok(()) }
}

/// Validates every payload field as a durable plain ABI value.
fn validate_com_enum_payloads(variants: &[(Ident, Fields, Expr)]) -> Result<()> {
    let mut errors = None;

    for (_, fields, _) in variants {
        validate_plain_abi_fields(fields, PlainAbiTypeContext::EnumPayload, &mut errors);
    }

    if let Some(errors) = errors { Err(errors) } else { Ok(()) }
}

/// Diagnoses duplicate tags when both expressions can be evaluated syntactically.
fn validate_unique_literal_tags(variants: &[(Ident, Fields, Expr)]) -> Result<()> {
    let mut seen = Vec::new();

    for (variant_ident, _, tag_value) in variants {
        let Some(tag) = literal_tag_value(tag_value) else {
            continue;
        };
        if let Some((_, previous_variant)) = seen.iter().find(|(seen_tag, _)| *seen_tag == tag) {
            return Err(syn::Error::new(
                tag_value.span(),
                format!("duplicate COM enum tag value; `{previous_variant}` already uses tag {tag}"),
            ));
        }
        seen.push((tag, variant_ident));
    }

    Ok(())
}

/// Evaluates a supported signed or unsigned integer literal tag expression.
fn literal_tag_value(expr: &Expr) -> Option<i128> {
    match expr {
        Expr::Lit(expr) => literal_int_value(&expr.lit, false),
        Expr::Unary(expr) if matches!(expr.op, UnOp::Neg(_)) => {
            let Expr::Lit(lit) = expr.expr.as_ref() else {
                return None;
            };
            literal_int_value(&lit.lit, true)
        }
        _ => None,
    }
}

/// Converts one integer literal into the normalized signed comparison domain.
fn literal_int_value(lit: &Lit, negative: bool) -> Option<i128> {
    let Lit::Int(value) = lit else {
        return None;
    };
    let value = value.base10_parse::<i128>().ok()?;
    Some(if negative { -value } else { value })
}

/// Extracts at most one `#[com_enum(tag = ...)]` variant attribute.
fn variant_tag(attrs: &[Attribute]) -> Result<Option<Expr>> {
    let mut tag = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("com_tag")) {
        if tag.is_some() {
            return Err(syn::Error::new_spanned(attr, "duplicate #[com_tag(...)] attribute"));
        }
        tag = Some(
            attr.parse_args::<Expr>()
                .map_err(|err| syn::Error::new(attr.span(), format!("expected #[com_tag(value)]: {err}")))?,
        );
    }
    Ok(tag)
}

/// Removes macro-only tag attributes before re-emitting the user's enum.
fn strip_com_enum_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs.iter().filter(|attr| !attr.path().is_ident("com_tag")).cloned().collect()
}

/// Builds the private payload struct name for one variant.
fn variant_payload_ident(enum_ident: &Ident, variant_ident: &Ident) -> Ident {
    format_ident!("{enum_ident}{variant_ident}Payload")
}

/// Builds the union field name used to store one variant payload.
fn union_payload_field(variant_ident: &Ident) -> Ident {
    let mut name = variant_ident.to_string();
    name.make_ascii_lowercase();
    // A private prefix keeps Move/Type/Crate and other keyword-shaped variants
    // valid without special-case keyword lists or changing their ABI layout.
    format_ident!("__nc_{name}")
}

/// Rejects case-folded variant names that would collide as generated union fields.
fn validate_unique_union_payload_fields(variants: &[(Ident, Fields, Expr)]) -> Result<()> {
    let mut errors = None;
    let mut fields: Vec<(String, &Ident)> = Vec::new();

    for (variant_ident, variant_fields, _) in variants {
        if matches!(variant_fields, Fields::Unit) {
            continue;
        }

        let field_name = union_payload_field(variant_ident).to_string();
        if let Some((_, previous_variant)) = fields.iter().find(|(previous_name, _)| previous_name == &field_name) {
            push_error(
                &mut errors,
                syn::Error::new(
                    variant_ident.span(),
                    format!(
                        "COM enum payload variants `{previous_variant}` and `{variant_ident}` both generate union field `{field_name}` after ASCII case folding"
                    ),
                ),
            );
        } else {
            fields.push((field_name, variant_ident));
        }
    }

    match errors {
        Some(errors) => Err(errors),
        None => Ok(()),
    }
}

/// Builds a stable named field for one tuple-variant payload position.
fn tuple_payload_field(index: usize) -> Ident {
    format_ident!("_{index}")
}
