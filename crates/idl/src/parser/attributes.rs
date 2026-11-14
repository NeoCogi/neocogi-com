//! Bounded attribute syntax: paths, token lists and literal strings, never Expr parsing.
use super::*;
use syn::{AttrStyle, Meta, MetaList, MetaNameValue, Path as RustPath, PathSegment, bracketed};

/// Reads outer attributes without parsing arbitrary name-value Rust expressions.
/// List tokens are already group-admitted; their context-specific leaf validators
/// below retain the established IDL policies and source spans.
pub(super) fn outer(input: ParseStream<'_>) -> syn::Result<Vec<Attribute>> {
    let mut attributes = Vec::new();
    while input.peek(Token![#]) {
        let pound_token = input.parse()?;
        let body;
        let bracket_token = bracketed!(body in input);
        let mut segments = Punctuated::new();
        segments.push_value(PathSegment::from(body.call(Ident::parse_any)?));
        while body.peek(Token![::]) {
            segments.push_punct(body.parse()?);
            segments.push_value(PathSegment::from(body.call(Ident::parse_any)?));
        }
        let path = RustPath { leading_colon: None, segments };
        let meta = if body.peek(Token![=]) {
            let eq_token = body.parse()?;
            if !body.peek(LitStr) {
                return Err(body.error("documentation text must be a string"));
            }
            let value = body.parse::<LitStr>()?;
            Meta::NameValue(MetaNameValue {
                path,
                eq_token,
                value: Expr::Lit(ExprLit { attrs: Vec::new(), lit: Lit::Str(value) }),
            })
        } else if body.peek(syn::token::Paren) {
            let arguments;
            let paren = parenthesized!(arguments in body);
            Meta::List(MetaList {
                path,
                delimiter: syn::MacroDelimiter::Paren(paren),
                tokens: arguments.parse()?,
            })
        } else {
            Meta::Path(path)
        };
        if !body.is_empty() {
            return Err(body.error("unsupported IDL attribute syntax"));
        }
        attributes.push(Attribute {
            pound_token,
            style: AttrStyle::Outer,
            bracket_token,
            meta,
        });
    }
    Ok(attributes)
}

/// Integer positions admit no attributes, including documentation.
pub(super) fn unsigned(input: ParseStream<'_>) -> syn::Result<u64> {
    reject_attributes(&outer(input)?)?;
    input.parse::<LitInt>()?.base10_parse()
}

/// Preserves the existing signed magnitude/radix/suffix policy without a Rust AST.
pub(super) fn signed(input: ParseStream<'_>) -> syn::Result<i64> {
    reject_attributes(&outer(input)?)?;
    if input.peek(Token![-]) {
        let sign = input.parse::<Token![-]>()?;
        let magnitude = unsigned(input)?;
        let magnitude = i64::try_from(magnitude).map_err(|_| syn::Error::new(sign.span, "negative discriminant magnitude exceeds i64"))?;
        Ok(-magnitude)
    } else {
        input.parse::<LitInt>()?.base10_parse()
    }
}
