//! Admission of inline syntax, separate from graph traversal and runtime budgets.
use proc_macro2::{TokenStream, TokenTree};
use crate::AbiType;

/// Maximum normalized inline type height, counting a leaf as one node.
/// Named declaration edges stop at their reference; siblings never add height.
pub(crate) const TYPE_HEIGHT: usize = 96;
/// Maximum actual token-group nesting before Syn constructs its recursive buffer.
pub(crate) const TOKEN_GROUP_DEPTH: usize = 128;
/// Reference/slice sugar may require two parser frames per normalized constructor.
/// This independent implementation bound also stops malformed prefix chains.
pub(crate) const PARSER_DESCENT: usize = TYPE_HEIGHT * 2;

/// Checks groups iteratively before any Syn parser/buffer consumes this stream.
/// Shallow token handles share the existing fallback allocation. Rejected streams
/// are neither formatted nor recursively copied; their fallback Drop is iterative.
pub(crate) fn admit_tokens(tokens: &TokenStream) -> syn::Result<()> {
    let mut frames = vec![tokens.clone().into_iter()];
    while let Some(frame) = frames.last_mut() {
        match frame.next() {
            Some(TokenTree::Group(group)) => {
                if frames.len() > TOKEN_GROUP_DEPTH {
                    return Err(syn::Error::new(
                        group.span_open(),
                        "[EIDL068] IDL token groups exceed the compiler nesting limit",
                    ));
                }
                frames.push(group.stream().into_iter());
            }
            Some(_) => {}
            None => {
                frames.pop();
            }
        }
    }
    Ok(())
}

/// Measures an already admitted inline tree without following nominal definitions.
/// This shared definition also admits prospective substitution destinations.
pub(crate) fn type_height(ty: &AbiType) -> usize {
    match ty {
        AbiType::Named(name) => 1 + name.arguments.iter().map(type_height).max().unwrap_or(0),
        AbiType::ComPtr(child)
        | AbiType::WeakComPtr(child)
        | AbiType::ComBox(child)
        | AbiType::WeakComBox(child)
        | AbiType::Borrowed(child)
        | AbiType::Vector(child)
        | AbiType::UnsizedSlice(child)
        | AbiType::Slice(child)
        | AbiType::OutputSlice(child)
        | AbiType::Array { element: child, .. } => 1 + type_height(child),
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => 1,
    }
}

#[cfg(test)]
mod tests;
