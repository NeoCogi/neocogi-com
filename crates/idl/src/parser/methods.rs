//! Direct method grammar: no Rust patterns, bodies or generic signature ASTs.
use super::*;

/// Reads one IDL method, rejecting unsupported Rust syntax before parsing it.
pub(super) fn method(input: ParseStream<'_>, attributes: &[Attribute]) -> syn::Result<(Method, Span)> {
    let (docs, scoped) = method_attributes(attributes)?;
    if !input.peek(Token![fn]) {
        return Err(input.error("IDL methods cannot be const, async, unsafe, extern, variadic, or generic"));
    }
    input.parse::<Token![fn]>()?;
    let name = input.parse::<Ident>()?;
    require_snake_case(&name, "method")?;
    if input.peek(Token![<]) {
        input.parse::<Token![<]>()?;
        if !input.peek(Token![>]) {
            return Err(input.error("IDL methods cannot be const, async, unsafe, extern, variadic, or generic"));
        }
        input.parse::<Token![>]>()?;
    }
    let body;
    parenthesized!(body in input);
    reject_attributes(&attributes::outer(&body)?)?;
    if !body.peek(Token![&]) {
        return Err(body.error("IDL methods must begin with `&self` or `&mut self`"));
    }
    body.parse::<Token![&]>()?;
    if body.peek(syn::Lifetime) {
        return Err(syn::Error::new_spanned(
            body.parse::<syn::Lifetime>()?,
            "IDL receivers cannot declare lifetimes; use `&self` or `&mut self`",
        ));
    }
    let receiver = if body.peek(Token![mut]) {
        body.parse::<Token![mut]>()?;
        Receiver::Mutable
    } else {
        Receiver::Shared
    };
    body.parse::<Token![self]>()?;
    if body.peek(Token![:]) {
        return Err(body.error("IDL methods must begin with `&self` or `&mut self`"));
    }
    let mut parameters = Vec::new();
    let mut names = HashMap::new();
    if !body.is_empty() {
        body.parse::<Token![,]>()?;
    }
    while !body.is_empty() {
        reject_attributes(&attributes::outer(&body)?)?;
        let ident = body.parse::<Ident>()?;
        require_snake_case(&ident, "parameter")?;
        body.parse::<Token![:]>()?;
        let ty = types::abi(&body)?;
        let name = ident.to_string();
        if let Some(previous) = names.insert(name.clone(), ident.span()) {
            return Err(duplicate_error("EIDL017", "parameter", &name, ident.span(), previous));
        }
        parameters.push(Parameter {
            span: SourceSpan::from_parser(ident.span()),
            name,
            ty,
            passing: None,
        });
        if !body.is_empty() {
            body.parse::<Token![,]>()?;
        }
    }
    let (output, return_projection) = if input.peek(Token![->]) {
        input.parse::<Token![->]>()?;
        types::method_output(input)?
    } else {
        (IdlType::Unit, crate::ReturnProjection::Direct)
    };
    if input.peek(syn::token::Brace) {
        return Err(input.error("IDL methods have no bodies"));
    }
    if input.peek(Token![where]) {
        return Err(input.error("IDL methods cannot be const, async, unsafe, extern, variadic, or generic"));
    }
    input.parse::<Token![;]>()?;
    Ok((
        Method {
            span: SourceSpan::from_parser(name.span()),
            docs,
            name: name.to_string(),
            receiver,
            parameters,
            output,
            return_projection,
            scoped,
            return_passing: None,
        },
        name.span(),
    ))
}

/// Accepts a bare visitor annotation on a method and keeps it out of Rust docs.
fn method_attributes(attributes: &[Attribute]) -> syn::Result<(Vec<String>, bool)> {
    let mut docs = Vec::new();
    let mut scoped = false;
    for attribute in attributes {
        if attribute.path().is_ident("doc") {
            docs.push(parse_doc_attribute(attribute)?);
        } else if attribute.path().is_ident("scoped") {
            if !matches!(attribute.meta, syn::Meta::Path(_)) {
                return Err(syn::Error::new_spanned(attribute, "use the bare #[scoped] method attribute"));
            }
            if scoped {
                return Err(syn::Error::new_spanned(attribute, "duplicate #[scoped] method attribute"));
            }
            scoped = true;
        } else {
            return Err(syn::Error::new_spanned(
                attribute,
                "only documentation attributes and #[scoped] are valid on methods",
            ));
        }
    }
    Ok((docs, scoped))
}
