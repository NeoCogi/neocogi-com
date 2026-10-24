//! One typed application/value plan for copied and owning IDL aggregates.
use super::*;
use crate::codegen::{ScalarDomain, ScalarMember, ScalarPlan, ValueCase};
use syn::parse::Parser;

/// Lowers checked integer values directly into the shared backend's scalar plan.
/// Dense names are compiler policy; the backend receives their resolved text/order.
pub(super) fn generate_scalar(graph: &Generation<'_>, declaration: &Declaration, identity: &crate::DeclarationIdentity) -> Result<String, GenerateError> {
    let (repr, domain, members) = match &declaration.kind {
        DeclarationKind::Enum(enumeration) => {
            let mut previous = -1i64;
            let members = enumeration
                .variants
                .iter()
                .map(|variant| {
                    let value = variant.discriminant.unwrap_or(previous + 1);
                    previous = value;
                    Ok(ScalarMember {
                        name: rust_ident(&variant.name)?,
                        value,
                        docs: origin_docs(&variant.docs, &variant.span),
                    })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            let domain = if enumeration.dense {
                let names = enumeration.names.map(|policy| match policy {
                    EnumNames::SnakeCase => enumeration
                        .variants
                        .iter()
                        .map(|variant| syn::LitStr::new(&snake_case(&variant.name), proc_macro2::Span::call_site()))
                        .collect(),
                });
                ScalarDomain::DenseEnumeration { names }
            } else {
                ScalarDomain::Enumeration
            };
            (enumeration.repr, domain, members)
        }
        DeclarationKind::BitEnum(flags) => {
            let members = flags
                .flags
                .iter()
                .map(|flag| {
                    Ok(ScalarMember {
                        name: rust_ident(&flag.name)?,
                        value: i64::try_from(flag.value).map_err(|_| GenerateError::new("resolved flag exceeds supported integer widths"))?,
                        docs: origin_docs(&flag.docs, &flag.span),
                    })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            (flags.repr, ScalarDomain::Flags, members)
        }
        _ => return Err(GenerateError::new("scalar emission requires a fieldless enum or flag set")),
    };
    let repr = enum_repr(repr);
    let signature = signature_literal(identity.abi_signature);
    let plan = ScalarPlan {
        name: rust_ident(&declaration.name)?,
        visibility: syn::parse_quote!(pub),
        docs: declaration_docs(graph, declaration),
        repr: syn::parse_quote!(#repr),
        members,
        domain,
        signature: quote! { ::neocogi_com::NcAbiSignature::from_static(#signature) },
        persistence: graph.graph.values.definitions[&identity.qualified_name].persistence == crate::Persistence::Supported,
    };
    format_rust(crate::codegen::emit_scalar(&plan, &syn::parse_quote!(::neocogi_com)))
}

/// Lowers a bound aggregate once into application fields and checked wire cases.
/// Native record layout is local; only the backend's explicit C wire crosses COM.
pub(super) fn generate_aggregate(
    graph: &mut Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
    identity: &crate::DeclarationIdentity,
) -> Result<String, GenerateError> {
    let name = rust_ident(&declaration.name)?;
    let docs = declaration_docs(graph, declaration);
    let signature = signature_literal(identity.abi_signature);
    let copied = graph.graph.values.definitions[&identity.qualified_name].copied;
    let mut cases = Vec::new();
    let (data, native) = match &declaration.kind {
        DeclarationKind::Struct(record) => {
            let fields = named_fields(graph, module, &record.fields, true)?;
            let native = quote! { pub struct #name #fields };
            cases.push(ValueCase {
                variant: None,
                tag: quote! { 0 },
                fields: syn::Fields::Named(fields.clone()),
            });
            (
                syn::Data::Struct(syn::DataStruct {
                    struct_token: Default::default(),
                    fields: syn::Fields::Named(fields),
                    semi_token: None,
                }),
                native,
            )
        }
        DeclarationKind::Enum(enumeration) => {
            let mut variants = syn::punctuated::Punctuated::new();
            for (ordinal, variant) in enumeration.variants.iter().enumerate() {
                let ident = rust_ident(&variant.name)?;
                let attributes = attributes(origin_docs(&variant.docs, &variant.span))?;
                let fields = match &variant.fields {
                    VariantFields::Unit => syn::Fields::Unit,
                    VariantFields::Named(fields) => syn::Fields::Named(named_fields(graph, module, fields, false)?),
                    VariantFields::Tuple(fields) => syn::Fields::Unnamed(syn::FieldsUnnamed {
                        paren_token: Default::default(),
                        unnamed: fields
                            .iter()
                            .map(|ty| field(graph, module, None, ty, Vec::new(), false))
                            .collect::<Result<_, _>>()?,
                    }),
                };
                let tag = Literal::u32_unsuffixed(u32::try_from(ordinal).map_err(|_| GenerateError::new("enum ordinal exceeds u32"))?);
                cases.push(ValueCase {
                    variant: Some(ident.clone()),
                    tag: quote! { #tag },
                    fields: fields.clone(),
                });
                variants.push(syn::Variant {
                    attrs: attributes,
                    ident,
                    fields,
                    discriminant: None,
                });
            }
            let native = quote! { pub enum #name { #variants } };
            (
                syn::Data::Enum(syn::DataEnum {
                    enum_token: Default::default(),
                    brace_token: Default::default(),
                    variants,
                }),
                native,
            )
        }
        _ => return Err(GenerateError::new("aggregate emission requires a record or payload enum")),
    };
    let nc = syn::parse_quote!(::neocogi_com);
    let codec = crate::codegen::emit_value(
        &name,
        &syn::parse_quote!(pub),
        &cases,
        quote! { ::neocogi_com::NcAbiSignature::from_static(#signature) },
        copied,
        copied && matches!(declaration.kind, DeclarationKind::Struct(_)),
        &nc,
    );
    let derives = if copied {
        quote! { #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug, ::core::cmp::PartialEq)] }
    } else {
        quote! { #[derive(::core::clone::Clone, ::core::fmt::Debug)] }
    };
    let nominal = copied.then(|| {
        quote! {
            // SAFETY: the bound IDL declaration supplies nominal identity; the
            // checked self-contained codec supplies the physical wire contract.
            unsafe impl ::neocogi_com::AbiType for #name {
                const ABI_TYPE: ::neocogi_com::NcAbiType = ::neocogi_com::NcAbiType::Aggregate(<Self as ::neocogi_com::ComValueType>::VALUE_SIGNATURE);
            }
            // SAFETY: every copied field is self-contained and has no release protocol.
            unsafe impl ::neocogi_com::AbiPlainValue for #name {}
        }
    });
    let persistence = if graph.graph.values.definitions[&identity.qualified_name].persistence == crate::Persistence::Supported {
        let decoder = match &data {
            syn::Data::Struct(record) => crate::codegen::emit_record_decoder(
                &name,
                &record.fields,
                &nc,
                matches!(&declaration.kind, DeclarationKind::Struct(record) if record.persistence == crate::PersistencePolicy::Default),
            ),
            syn::Data::Enum(enumeration) => crate::codegen::emit_enum_decoder(&name, enumeration, &nc),
            syn::Data::Union(_) => unreachable!("IDL has no native unions"),
        }
        .map_err(|error| GenerateError::new(error.to_string()))?;
        let serializer = crate::codegen::expand_com_serialize_derive(syn::DeriveInput {
            attrs: Vec::new(),
            vis: syn::parse_quote!(pub),
            ident: name.clone(),
            generics: Default::default(),
            data,
        })
        .map_err(|error| GenerateError::new(error.to_string()))?;
        quote! { #decoder #serializer }
    } else {
        quote! {}
    };
    format_rust(quote! { #derives #docs #native #codec #nominal #persistence })
}

/// Builds application fields directly from their bound IDL field uses.
fn named_fields(graph: &mut Generation<'_>, module: &ResolvedModule, fields: &[crate::Field], public: bool) -> Result<syn::FieldsNamed, GenerateError> {
    Ok(syn::FieldsNamed {
        brace_token: Default::default(),
        named: fields
            .iter()
            .map(|source| {
                field(
                    graph,
                    module,
                    Some(rust_ident(&source.name)?),
                    &source.ty,
                    attributes(origin_docs(&source.docs, &source.span))?,
                    public,
                )
            })
            .collect::<Result<_, _>>()?,
    })
}

/// Only Rust syntax is represented here; no field spelling is resolved again.
fn field(
    graph: &mut Generation<'_>,
    module: &ResolvedModule,
    ident: Option<Ident>,
    ty: &AbiType,
    attrs: Vec<syn::Attribute>,
    public: bool,
) -> Result<syn::Field, GenerateError> {
    Ok(syn::Field {
        attrs,
        vis: if public { syn::parse_quote!(pub) } else { syn::Visibility::Inherited },
        mutability: syn::FieldMutability::None,
        colon_token: ident.as_ref().map(|_| Default::default()),
        ident,
        ty: syn::parse2(owned_type(graph, module, ty)?).map_err(|error| GenerateError::new(error.to_string()))?,
    })
}

/// Keeps source annotations as syntax shared by native fields and source mapping.
fn attributes(tokens: TokenStream) -> Result<Vec<syn::Attribute>, GenerateError> {
    syn::Attribute::parse_outer
        .parse2(tokens)
        .map_err(|error| GenerateError::new(error.to_string()))
}

/// Maps a recursive field to application ownership; no native wire copying occurs here.
pub(super) fn owned_type(graph: &mut Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    Ok(match ty {
        AbiType::Unit => quote! { () },
        AbiType::Scalar(name) => scalar(name)?,
        AbiType::String => quote! { self::alloc::string::String },
        AbiType::ComPtr(_) | AbiType::WeakComPtr(_) | AbiType::WeakComBox(_) => owned_object_type(graph, module, ty)?,
        AbiType::Named(name) => named_rust_type(graph, module, resolve(graph, module, name)?, false)?,
        AbiType::Vector(value) => {
            let value = owned_type(graph, module, value)?;
            quote! { self::alloc::vec::Vec<#value> }
        }
        AbiType::Array { element, len } => {
            let value = owned_type(graph, module, element)?;
            let len = *len as usize;
            quote! { [#value; #len] }
        }
        _ => return Err(GenerateError::new("resolved field has no owned value lowering")),
    })
}
