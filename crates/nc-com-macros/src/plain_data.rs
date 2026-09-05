//! Syntactic rejection of Rust-only types from plain ABI positions.
//!
//! Generated `AbiType` assertions remain the authoritative positive check. This
//! walker provides earlier, domain-specific diagnostics for references,
//! pointers, containers, and COM smart pointers even when they are nested in
//! arrays, tuples, qualified paths, or generic arguments.

use syn::{Fields, Type};

use crate::util::push_error;

#[derive(Clone, Copy)]
/// Location in which a syntactically plain ABI type is being validated.
pub(crate) enum PlainAbiTypeContext {
    // Context changes remediation text: a method can use a COM interface or a
    // modeled buffer, while a buffer element must remain a plain value.
    /// Field of a macro-generated `repr(C)` struct.
    StructField,
    /// Payload field of a generated tagged enum variant.
    EnumPayload,
    /// Direct by-value COM method argument.
    MethodArgument,
    /// Direct out-value COM method result.
    MethodReturn,
    /// Element copied through a bounded buffer parameter.
    BufferElement,
}

#[derive(Clone, Copy)]
/// Rust-only type category that cannot safely occupy a plain ABI position.
enum ProhibitedPlainAbiType {
    /// Borrowed Rust reference.
    Reference,
    /// Unmodeled raw pointer.
    RawPointer,
    /// Dynamically sized slice.
    Slice,
    /// Rust-owned UTF-8 string.
    String,
    /// Rust-owned growable vector.
    Vec,
    /// Owning COM smart pointer used outside its modeled return position.
    ComPtr,
    /// Borrowed COM smart pointer used outside its modeled argument position.
    ComRef,
}

/// A prohibited nested type together with the syntax span to diagnose.
struct ProhibitedPlainAbiTypeAt<'a> {
    /// Exact nested type node that triggered rejection.
    ty: &'a Type,
    /// Ownership-bearing category used to select remediation text.
    kind: ProhibitedPlainAbiType,
}

impl PlainAbiTypeContext {
    /// Returns the human-readable subject used in diagnostics.
    fn subject(self) -> &'static str {
        match self {
            PlainAbiTypeContext::StructField => "COM struct fields",
            PlainAbiTypeContext::EnumPayload => "COM enum payload fields",
            PlainAbiTypeContext::MethodArgument => "COM method arguments",
            PlainAbiTypeContext::MethodReturn => "COM method returns",
            PlainAbiTypeContext::BufferElement => "COM buffer elements",
        }
    }

    /// Builds context-specific remediation for a prohibited type category.
    fn prohibited_message(self, ty: ProhibitedPlainAbiType) -> String {
        if matches!(self, PlainAbiTypeContext::BufferElement) {
            let subject = self.subject();
            return match ty {
                ProhibitedPlainAbiType::Reference => {
                    format!(
                        "{subject} cannot be Rust references; buffer elements must be plain ABI values"
                    )
                }
                ProhibitedPlainAbiType::RawPointer => {
                    format!(
                        "{subject} cannot be raw pointers; buffer elements must be plain ABI values"
                    )
                }
                ProhibitedPlainAbiType::Slice => {
                    format!("{subject} cannot be slices; buffer elements must be plain ABI values")
                }
                ProhibitedPlainAbiType::String => {
                    format!("{subject} cannot be String; buffer elements must be plain ABI values")
                }
                ProhibitedPlainAbiType::Vec => {
                    format!("{subject} cannot be Vec; buffer elements must be plain ABI values")
                }
                ProhibitedPlainAbiType::ComPtr => {
                    format!("{subject} cannot be ComPtr; buffer elements must be plain ABI values")
                }
                ProhibitedPlainAbiType::ComRef => {
                    format!("{subject} cannot be ComRef; buffer elements must be plain ABI values")
                }
            };
        }

        let subject = self.subject();
        match ty {
            ProhibitedPlainAbiType::Reference => {
                format!(
                    "{subject} cannot be Rust references; model borrowed data as COM interfaces"
                )
            }
            ProhibitedPlainAbiType::RawPointer => {
                format!("{subject} cannot be raw pointers; model pointed-to data as COM interfaces")
            }
            ProhibitedPlainAbiType::Slice => {
                format!("{subject} cannot be slices; model variable-size data as COM interfaces")
            }
            ProhibitedPlainAbiType::String => {
                format!("{subject} cannot be String; model strings as COM interfaces")
            }
            ProhibitedPlainAbiType::Vec => {
                format!("{subject} cannot be Vec; model variable-size data as COM interfaces")
            }
            ProhibitedPlainAbiType::ComPtr => {
                format!(
                    "{subject} cannot be ComPtr; store owned interface references in COM objects instead"
                )
            }
            ProhibitedPlainAbiType::ComRef => {
                format!(
                    "{subject} cannot be ComRef; borrowed interface references are call-scoped method parameters"
                )
            }
        }
    }
}

/// Returns the ordered field types for unit, tuple, or named aggregate syntax.
pub(crate) fn payload_field_types(fields: &Fields) -> Vec<&Type> {
    match fields {
        Fields::Unit => Vec::new(),
        Fields::Unnamed(fields) => fields.unnamed.iter().map(|field| &field.ty).collect(),
        Fields::Named(fields) => fields.named.iter().map(|field| &field.ty).collect(),
    }
}

/// Reports every field that contains syntactically prohibited Rust-only data.
pub(crate) fn validate_plain_abi_fields(
    fields: &Fields,
    context: PlainAbiTypeContext,
    errors: &mut Option<syn::Error>,
) {
    for ty in payload_field_types(fields) {
        if let Err(err) = validate_plain_abi_type(ty, context) {
            push_error(errors, err);
        }
    }
}

/// Rejects the first ownership-bearing type nested anywhere inside `ty`.
pub(crate) fn validate_plain_abi_type(ty: &Type, context: PlainAbiTypeContext) -> syn::Result<()> {
    if let Some(prohibited) = prohibited_plain_abi_type(ty) {
        Err(syn::Error::new_spanned(
            prohibited.ty,
            context.prohibited_message(prohibited.kind),
        ))
    } else {
        Ok(())
    }
}

/// Recursively locates the first Rust-only type hidden by supported syntax.
fn prohibited_plain_abi_type(ty: &Type) -> Option<ProhibitedPlainAbiTypeAt<'_>> {
    // Recurse through every type wrapper that can hide an ownership-bearing
    // type. Unknown syntax is left to generated trait bounds instead of being
    // rejected heuristically.
    match ty {
        Type::Reference(_) => Some(prohibited_at(ty, ProhibitedPlainAbiType::Reference)),
        Type::Ptr(_) => Some(prohibited_at(ty, ProhibitedPlainAbiType::RawPointer)),
        Type::Slice(_) => Some(prohibited_at(ty, ProhibitedPlainAbiType::Slice)),
        Type::Array(array) => prohibited_plain_abi_type(&array.elem),
        Type::Group(group) => prohibited_plain_abi_type(&group.elem),
        Type::Paren(paren) => prohibited_plain_abi_type(&paren.elem),
        Type::Path(path) => {
            if let Some(qself) = &path.qself {
                if let Some(prohibited) = prohibited_plain_abi_type(&qself.ty) {
                    return Some(prohibited);
                }
            }

            let ident = path.path.segments.last()?.ident.to_string();
            match ident.as_str() {
                "String" => return Some(prohibited_at(ty, ProhibitedPlainAbiType::String)),
                "Vec" => return Some(prohibited_at(ty, ProhibitedPlainAbiType::Vec)),
                "ComPtr" => return Some(prohibited_at(ty, ProhibitedPlainAbiType::ComPtr)),
                "ComRef" => return Some(prohibited_at(ty, ProhibitedPlainAbiType::ComRef)),
                _ => {}
            }

            path.path
                .segments
                .iter()
                .find_map(|segment| prohibited_in_path_arguments(&segment.arguments))
        }
        Type::Tuple(tuple) => tuple.elems.iter().find_map(prohibited_plain_abi_type),
        _ => None,
    }
}

/// Associates a prohibited category with the exact syntax node to highlight.
fn prohibited_at(ty: &Type, kind: ProhibitedPlainAbiType) -> ProhibitedPlainAbiTypeAt<'_> {
    ProhibitedPlainAbiTypeAt { ty, kind }
}

/// Searches angle-bracketed or function-style path arguments recursively.
fn prohibited_in_path_arguments(
    arguments: &syn::PathArguments,
) -> Option<ProhibitedPlainAbiTypeAt<'_>> {
    match arguments {
        syn::PathArguments::None => None,
        syn::PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .find_map(prohibited_in_generic_argument),
        syn::PathArguments::Parenthesized(arguments) => arguments
            .inputs
            .iter()
            .find_map(prohibited_plain_abi_type)
            .or_else(|| match &arguments.output {
                syn::ReturnType::Default => None,
                syn::ReturnType::Type(_, ty) => prohibited_plain_abi_type(ty),
            }),
    }
}

/// Searches one generic argument, including associated bindings and constraints.
fn prohibited_in_generic_argument(
    argument: &syn::GenericArgument,
) -> Option<ProhibitedPlainAbiTypeAt<'_>> {
    // Associated type bindings and constraints can themselves contain generic
    // arguments, so inspect both their bindings and nested argument lists.
    match argument {
        syn::GenericArgument::Type(ty) => prohibited_plain_abi_type(ty),
        syn::GenericArgument::AssocType(assoc) => assoc
            .generics
            .as_ref()
            .and_then(|generics| {
                generics
                    .args
                    .iter()
                    .find_map(prohibited_in_generic_argument)
            })
            .or_else(|| prohibited_plain_abi_type(&assoc.ty)),
        syn::GenericArgument::Constraint(constraint) => {
            constraint.generics.as_ref().and_then(|generics| {
                generics
                    .args
                    .iter()
                    .find_map(prohibited_in_generic_argument)
            })
        }
        syn::GenericArgument::Lifetime(_)
        | syn::GenericArgument::Const(_)
        | syn::GenericArgument::AssocConst(_) => None,
        _ => None,
    }
}
