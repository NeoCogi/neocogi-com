//! Application storage selection for the single recursively checked value model.
use super::*;

/// Emits an owning record or enum; its fields use the same codec as copied values.
pub(super) fn generate_owned(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
    identity: &crate::DeclarationIdentity,
) -> Result<String, GenerateError> {
    let name = rust_ident(&declaration.name)?;
    let docs = origin_docs(&declaration.docs, &declaration.span);
    let signature = signature_literal(identity.abi_signature);
    let data = match &declaration.kind {
        DeclarationKind::Struct(record) => {
            let fields = record
                .fields
                .iter()
                .map(|field| {
                    let name = rust_ident(&field.name)?;
                    let docs = origin_docs(&field.docs, &field.span);
                    let ty = owned_type(graph, module, &field.ty)?;
                    Ok(quote! { #docs pub #name: #ty })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            quote! { pub struct #name { #(#fields),* } }
        }
        DeclarationKind::Enum(enumeration) => {
            let mut previous = -1;
            let variants = enumeration
                .variants
                .iter()
                .map(|variant| {
                    let name = rust_ident(&variant.name)?;
                    let docs = origin_docs(&variant.docs, &variant.span);
                    let tag = variant.discriminant.unwrap_or(previous + 1);
                    previous = tag;
                    let fields = match &variant.fields {
                        VariantFields::Unit => quote! {},
                        VariantFields::Tuple(fields) => {
                            let types = fields.iter().map(|ty| owned_type(graph, module, ty)).collect::<Result<Vec<_>, _>>()?;
                            quote! { (#(#types),*) }
                        }
                        VariantFields::Named(fields) => {
                            let fields = fields
                                .iter()
                                .map(|field| {
                                    let name = rust_ident(&field.name)?;
                                    let ty = owned_type(graph, module, &field.ty)?;
                                    let docs = origin_docs(&field.docs, &field.span);
                                    Ok(quote! { #docs #name: #ty })
                                })
                                .collect::<Result<Vec<_>, GenerateError>>()?;
                            quote! { { #(#fields),* } }
                        }
                    };
                    Ok(quote! { #docs #[com_tag(#tag)] #name #fields })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            quote! { pub enum #name { #(#variants),* } }
        }
        _ => return Err(GenerateError::new("owned values require a record or enum")),
    };
    let deserialize = match &declaration.kind {
        DeclarationKind::Struct(record) if record.deserialize => {
            let default = record.deserialize_default.then(|| quote! { , default });
            let sequence = record.deserialize_sequence.then(|| quote! { , sequence });
            quote! { , deserialize #default #sequence }
        }
        _ => quote! {},
    };
    format_rust(quote! {
        #[::neocogi_com::com_value(assigned_signature = #signature #deserialize)]
        #[derive(Clone, Debug)]
        #docs #data
    })
}

/// Maps a recursive field to application ownership; no native wire copying occurs here.
pub(super) fn owned_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    Ok(match ty {
        AbiType::Unit => quote! { () },
        AbiType::Scalar(name) => scalar(name)?,
        AbiType::String => quote! { self::alloc::string::String },
        AbiType::ComPtr(value) => match value.as_ref() {
            AbiType::Named(name) => object_reference_type(graph, module, name, true)?,
            _ => return Err(GenerateError::new("resolved ComPtr target is not a contract")),
        },
        AbiType::Named(name) => named_rust_type(graph, module, resolve(graph, module, name)?, false)?,
        AbiType::Slice(value) => {
            let value = owned_type(graph, module, value)?;
            quote! { self::alloc::vec::Vec<#value> }
        }
        AbiType::Option(value) => {
            let value = owned_type(graph, module, value)?;
            quote! { Option<#value> }
        }
        AbiType::Array { element, len } => {
            let value = owned_type(graph, module, element)?;
            let len = *len as usize;
            quote! { [#value; #len] }
        }
        _ => return Err(GenerateError::new("resolved field has no owned value lowering")),
    })
}
