//! Parses only the IDL type grammar; no general Syn Type tree is constructed.
use super::*;

/// One admitted expression and its construction-time normalized height.
/// The height is temporary parser evidence, never a second retained type model.
struct Value {
    ty: IdlType,
    height: usize,
}
impl Value {
    /// Checks the prospective parent before installing any recursive owned node.
    fn new(ty: IdlType, height: usize, span: Span) -> syn::Result<Self> {
        check_height(height, span)?;
        debug_assert_eq!(crate::nesting::type_height(&ty), height);
        Ok(Self { ty, height })
    }
    /// Installs one unary constructor after its resulting height is admitted.
    fn wrap(self, span: Span, make: impl FnOnce(Box<IdlType>) -> IdlType) -> syn::Result<Self> {
        let height = self.height + 1;
        check_height(height, span)?;
        Ok(Self { ty: make(Box::new(self.ty)), height })
    }
}

/// Identifiers and punctuation preceding a path's optional final type arguments.
struct Head {
    segments: Vec<String>,
    span: Span,
}
/// Checks the shared normalized height contract at the offending constructor.
fn check_height(height: usize, span: Span) -> syn::Result<()> {
    if height > crate::nesting::TYPE_HEIGHT {
        Err(syn::Error::new(span, "[EIDL068] IDL type exceeds the compiler nesting limit"))
    } else {
        Ok(())
    }
}
/// Reads one bounded normalized type at a public grammar root.
pub(super) fn abi(input: ParseStream<'_>) -> syn::Result<IdlType> {
    Ok(value(input, 0)?.ty)
}

/// Reads path identifiers iteratively, retaining the exact namespace rejection span.
fn head(input: ParseStream<'_>) -> syn::Result<Head> {
    if input.peek(Token![::]) {
        return Err(syn::Error::new_spanned(
            input.parse::<Token![::]>()?,
            "IDL paths are module-qualified without a leading ::",
        ));
    }
    let first = input.parse::<Ident>()?;
    let mut last = first.clone();
    let mut segments = vec![first.to_string()];
    while input.peek(Token![::]) {
        let separator = input.parse::<Token![::]>()?;
        if input.peek(Token![<]) {
            break;
        }
        ModuleName::validate_segment(&last.to_string()).map_err(|error| syn::Error::new(last.span(), error))?;
        last = input
            .parse::<Ident>()
            .map_err(|_| syn::Error::new(separator.spans[1], "expected a declaration path"))?;
        segments.push(last.to_string());
    }
    Ok(Head {
        segments,
        span: first.span().join(last.span()).unwrap_or(first.span()),
    })
}

/// One active parser frame. The independent bound protects even invalid prefixes;
/// the returned value's normalized height preserves equivalent slice spellings.
fn value(input: ParseStream<'_>, descent: usize) -> syn::Result<Value> {
    if descent > crate::nesting::PARSER_DESCENT {
        return Err(input.error("[EIDL068] IDL type syntax exceeds the compiler nesting limit"));
    }
    let span = input.span();
    if input.peek(Token![&]) {
        input.parse::<Token![&]>()?;
        let mutable = if input.peek(Token![mut]) {
            input.parse::<Token![mut]>()?;
            true
        } else {
            false
        };
        let child = value(input, descent + 1)?;
        let height = child.height;
        return match (mutable, child.ty) {
            (false, IdlType::UnsizedSlice(element)) => Value::new(IdlType::Slice(element), height, span),
            (true, IdlType::UnsizedSlice(element)) => Value::new(IdlType::OutputSlice(element), height, span),
            (false, ty) => Value { ty, height }.wrap(span, IdlType::Borrowed),
            (true, ty) => Value { ty, height }.wrap(span, IdlType::BorrowedMut),
        };
    }
    if input.peek(syn::token::Paren) {
        let body;
        parenthesized!(body in input);
        if !body.is_empty() {
            return Err(body.error("unsupported IDL ABI type"));
        }
        return Value::new(IdlType::Unit, 1, span);
    }
    if input.peek(syn::token::Bracket) {
        let body;
        syn::bracketed!(body in input);
        let child = value(&body, descent + 1)?;
        let result = if body.peek(Token![;]) {
            body.parse::<Token![;]>()?;
            let len = length(&body)?;
            child.wrap(span, |element| IdlType::Array { element, len })?
        } else {
            child.wrap(span, IdlType::UnsizedSlice)?
        };
        if !body.is_empty() {
            return Err(body.error("unsupported IDL array syntax"));
        }
        return Ok(result);
    }
    let path = head(input)?;
    let name = path.segments[0].as_str();
    if path.segments.len() == 1 {
        if matches!(
            name,
            "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64" | "Guid" | "Status" | "String"
        ) {
            if input.peek(Token![<]) {
                return Err(input.error("scalar IDL types do not accept generic arguments"));
            }
            return Value::new(if name == "String" { IdlType::String } else { IdlType::Scalar(name.into()) }, 1, path.span);
        }
        if name == "Result" {
            return Err(syn::Error::new(path.span, "IDL `Result` is valid only as a method return"));
        }
        if name == "Array" {
            input.parse::<Token![<]>()?;
            let child = value(input, descent + 1)?;
            input.parse::<Token![,]>()?;
            let len = length(input)?;
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
            input.parse::<Token![>]>()?;
            return child.wrap(path.span, |element| IdlType::Array { element, len });
        }
        if matches!(name, "ComPtr" | "WeakComPtr" | "ComBox" | "Vec" | "Slice") {
            input.parse::<Token![<]>()?;
            let child = value(input, descent + 1)?;
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
            input.parse::<Token![>]>()?;
            return child.wrap(path.span, |child| match name {
                "ComPtr" => IdlType::ComPtr(child),
                "WeakComPtr" => IdlType::WeakComPtr(child),
                "ComBox" => IdlType::ComBox(child),
                "Vec" => IdlType::Vector(child),
                "Slice" => IdlType::UnsizedSlice(child),
                _ => unreachable!("checked intrinsic"),
            });
        }
    }
    named(input, path, descent)
}
/// Admits the literal unsigned32 array domain without general expression parsing.
fn length(input: ParseStream<'_>) -> syn::Result<u32> {
    let span = input.span();
    u32::try_from(attributes::unsigned(input)?).map_err(|_| syn::Error::new(span, "IDL array length exceeds u32"))
}
/// Installs a named application only after all argument heights fit its parent.
fn named(input: ParseStream<'_>, path: Head, descent: usize) -> syn::Result<Value> {
    let mut arguments = Vec::new();
    let mut height = 1;
    let mut span = path.span;
    if input.peek(Token![<]) {
        input.parse::<Token![<]>()?;
        if input.peek(Token![>]) {
            return Err(input.error("type arguments must not be empty"));
        }
        loop {
            let argument = value(input, descent + 1)?;
            height = height.max(argument.height + 1);
            check_height(height, path.span)?;
            arguments.push(argument.ty);
            if !input.peek(Token![,]) {
                break;
            }
            input.parse::<Token![,]>()?;
            if input.peek(Token![>]) {
                break;
            }
        }
        let end = input.parse::<Token![>]>()?;
        span = span.join(end.span()).unwrap_or(span);
    }
    Value::new(
        IdlType::Named(NamedType {
            binding: crate::binding::Binding::Unbound,
            span: SourceSpan::from_parser(span),
            segments: path.segments,
            arguments,
        }),
        height,
        path.span,
    )
}
/// Bases admit only named paths; redundant parentheses are bounded by the group gate.
pub(super) fn base(input: ParseStream<'_>) -> syn::Result<NamedType> {
    if input.peek(syn::token::Paren) {
        let body;
        parenthesized!(body in input);
        let value = base(&body)?;
        if !body.is_empty() {
            return Err(body.error("IDL bases must be plain interface paths"));
        }
        return Ok(value);
    }
    let path = head(input)?;
    let Value { ty: IdlType::Named(value), .. } = named(input, path, 0)? else {
        unreachable!()
    };
    Ok(value)
}
/// Separates method-only Result framing from the bounded successful type.
pub(super) fn method_output(input: ParseStream<'_>) -> syn::Result<(IdlType, crate::ReturnProjection)> {
    if !input.peek(Ident) || input.fork().parse::<Ident>()? != "Result" {
        return Ok((abi(input)?, crate::ReturnProjection::Direct));
    }
    let path = head(input)?;
    if path.segments != ["Result"] {
        return Err(syn::Error::new(path.span, "IDL method Result is unqualified"));
    }
    input.parse::<Token![<]>()?;
    let output = abi(input)?;
    if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
    }
    input.parse::<Token![>]>()?;
    Ok((output, crate::ReturnProjection::Fallible))
}
