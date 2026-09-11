//! Parser for the intentionally small Rust-like NeoCogi IDL grammar.

use std::{collections::HashSet, fmt, path::Path};

use neocogi_com_core::NcGuid;
use proc_macro2::Span;
use syn::{
    Attribute, Expr, ExprLit, ExprUnary, FnArg, GenericArgument, Ident, Lit, LitInt, LitStr, Pat, PathArguments, ReturnType, Token, TraitItemFn, Type,
    TypeParamBound, UnOp, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_str,
    punctuated::Punctuated,
};

use crate::{
    AbiType, Class, Declaration, DeclarationId, DeclarationKind, Enum, EnumRepr, EnumVariant, Factory, Field, Import, Interface, Method, Module, ModuleName,
    ModulePathError, NamedType, Parameter, Receiver, Struct, Threading, VariantFields,
};

mod keyword {
    //! Custom declaration keywords that intentionally are not Rust items.

    syn::custom_keyword!(class);
    syn::custom_keyword!(factory);
}

/// Failure to derive a module name or parse one IDL source file.
#[derive(Debug)]
pub enum ParseError {
    /// The relative source path could not define a portable logical module.
    ModulePath(ModulePathError),
    /// The source violated the supported IDL grammar or structural rules.
    Source(
        /// Human-readable parser diagnostic including span context when Syn
        /// can provide it.
        String,
    ),
}

impl fmt::Display for ParseError {
    /// Writes the precise path or source parser diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModulePath(error) => write!(formatter, "{error}"),
            Self::Source(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<ModulePathError> for ParseError {
    /// Preserves a module-path failure without erasing its category.
    fn from(error: ModulePathError) -> Self {
        Self::ModulePath(error)
    }
}

impl From<syn::Error> for ParseError {
    /// Converts Syn's spanned diagnostic into the crate's stable error surface.
    fn from(error: syn::Error) -> Self {
        Self::Source(error.to_string())
    }
}

/// Parses one source file whose relative path defines its logical module name.
pub fn parse_module(relative_path: &Path, source: &str) -> Result<Module, ParseError> {
    let name = ModuleName::from_relative_source(relative_path)?;
    let syntax = parse_str::<ModuleSyntax>(source)?;
    Ok(Module {
        name,
        imports: syntax.imports,
        declarations: syntax.declarations,
    })
}

/// Parsed top-level source before its path-derived module name is attached.
struct ModuleSyntax {
    /// Imports flattened into module/name groups.
    imports: Vec<Import>,
    /// Declarations retained in normative source order.
    declarations: Vec<Declaration>,
}

impl Parse for ModuleSyntax {
    /// Parses imports and declarations while rejecting every other Rust item.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut imports = Vec::new();
        let mut declarations = Vec::new();
        let mut declaration_names = HashSet::new();

        while !input.is_empty() {
            let attributes = Attribute::parse_outer(input)?;
            if input.peek(Token![pub]) {
                return Err(input.error("IDL declarations are public implicitly; remove `pub`"));
            }

            if input.peek(Token![use]) {
                if !attributes.is_empty() {
                    return Err(syn::Error::new_spanned(&attributes[0], "IDL imports do not accept attributes"));
                }
                imports.push(parse_import(input)?);
                continue;
            }

            let declaration = if input.peek(Token![struct]) {
                parse_struct(attributes, input)?
            } else if input.peek(Token![enum]) {
                parse_enum(attributes, input)?
            } else if input.peek(Token![trait]) {
                parse_interface(attributes, input)?
            } else if input.peek(keyword::class) {
                parse_class(attributes, input)?
            } else if input.peek(keyword::factory) {
                parse_factory(attributes, input)?
            } else {
                return Err(input.error("expected `use`, `struct`, `enum`, `trait`, `class`, or `factory`"));
            };

            if !declaration_names.insert(declaration.name.clone()) {
                return Err(syn::Error::new(Span::call_site(), format!("duplicate declaration {:?}", declaration.name)));
            }
            declarations.push(declaration);
        }

        if declarations.is_empty() {
            return Err(input.error("an IDL module must declare at least one ABI item"));
        }
        Ok(Self { imports, declarations })
    }
}

/// Parses one copied record declaration.
fn parse_struct(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::Plain)?;
    input.parse::<Token![struct]>()?;
    let name = input.parse::<Ident>()?;
    let body;
    braced!(body in input);
    let mut fields = Vec::new();
    let mut names = HashSet::new();
    while !body.is_empty() {
        let field = parse_named_field(&body)?;
        if !names.insert(field.name.clone()) {
            return Err(body.error(format!("duplicate field {:?}", field.name)));
        }
        fields.push(field);
    }
    if fields.is_empty() {
        return Err(syn::Error::new(name.span(), "IDL structs must contain at least one field"));
    }
    Ok(declaration(name, metadata, DeclarationKind::Struct(Struct { fields })))
}

/// Parses one enum with unit, tuple, or named payload variants.
fn parse_enum(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::Enum)?;
    let repr = metadata_repr(&metadata);
    input.parse::<Token![enum]>()?;
    let name = input.parse::<Ident>()?;
    let body;
    braced!(body in input);
    let mut variants = Vec::new();
    let mut names = HashSet::new();
    while !body.is_empty() {
        let variant = parse_enum_variant(&body)?;
        if !names.insert(variant.name.clone()) {
            return Err(body.error(format!("duplicate enum variant {:?}", variant.name)));
        }
        variants.push(variant);
    }
    if variants.is_empty() {
        return Err(syn::Error::new(name.span(), "IDL enums must contain at least one variant"));
    }
    Ok(declaration(name, metadata, DeclarationKind::Enum(Enum { repr, variants })))
}

/// Parses one reusable interface declared with the Rust-like `trait` keyword.
fn parse_interface(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::Interface)?;
    input.parse::<Token![trait]>()?;
    let name = input.parse::<Ident>()?;
    let bases = parse_bases(input)?;
    let methods = parse_methods(input)?;
    Ok(declaration(name, metadata, DeclarationKind::Interface(Interface { bases, methods })))
}

/// Parses one public object contract without admitting constructor syntax.
fn parse_class(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::Object)?;
    let threading = metadata_threading(&metadata);
    input.parse::<keyword::class>()?;
    let name = input.parse::<Ident>()?;
    let required_interfaces = parse_bases(input)?;
    let methods = parse_methods(input)?;
    Ok(declaration(
        name,
        metadata,
        DeclarationKind::Class(Class { required_interfaces, methods, threading }),
    ))
}

/// Parses one typed construction or provider interface.
fn parse_factory(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::Object)?;
    let threading = metadata_threading(&metadata);
    input.parse::<keyword::factory>()?;
    let name = input.parse::<Ident>()?;
    let required_interfaces = parse_bases(input)?;
    let methods = parse_methods(input)?;
    Ok(declaration(
        name,
        metadata,
        DeclarationKind::Factory(Factory { required_interfaces, methods, threading }),
    ))
}

/// Combines common metadata with one declaration-specific payload.
fn declaration(name: Ident, metadata: Attributes, kind: DeclarationKind) -> Declaration {
    Declaration {
        docs: metadata.docs,
        name: name.to_string(),
        id: metadata.id,
        kind,
    }
}

/// Parses the optional `: Interface + Interface` suffix used by callable types.
fn parse_bases(input: ParseStream<'_>) -> syn::Result<Vec<NamedType>> {
    if !input.peek(Token![:]) {
        return Ok(Vec::new());
    }
    input.parse::<Token![:]>()?;
    let bounds = Punctuated::<TypeParamBound, Token![+]>::parse_separated_nonempty(input)?;
    bounds
        .into_iter()
        .map(|bound| match bound {
            TypeParamBound::Trait(bound) if bound.lifetimes.is_none() && bound.modifier == syn::TraitBoundModifier::None => named_path(&bound.path),
            _ => Err(syn::Error::new_spanned(bound, "IDL bases must be plain interface paths")),
        })
        .collect()
}

/// Parses a braced sequence of declaration-only COM methods.
fn parse_methods(input: ParseStream<'_>) -> syn::Result<Vec<Method>> {
    let body;
    braced!(body in input);
    let mut methods = Vec::new();
    let mut names = HashSet::new();
    while !body.is_empty() {
        let source = body.parse::<TraitItemFn>()?;
        let method = lower_method(source)?;
        if !names.insert(method.name.clone()) {
            return Err(body.error(format!("duplicate method {:?}", method.name)));
        }
        methods.push(method);
    }
    Ok(methods)
}

/// Validates and lowers one Syn trait method into the language-neutral model.
fn lower_method(source: TraitItemFn) -> syn::Result<Method> {
    if source.default.is_some() {
        return Err(syn::Error::new_spanned(source, "IDL methods have no bodies"));
    }
    let docs = parse_doc_only_attributes(&source.attrs)?;
    let signature = source.sig;
    if signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
    {
        return Err(syn::Error::new_spanned(
            signature,
            "IDL methods cannot be const, async, unsafe, extern, variadic, or generic",
        ));
    }

    let mut inputs = signature.inputs.into_iter();
    let receiver = match inputs.next() {
        Some(FnArg::Receiver(receiver)) if receiver.reference.is_some() && receiver.colon_token.is_none() => {
            if receiver.mutability.is_some() {
                Receiver::Mutable
            } else {
                Receiver::Shared
            }
        }
        other => return Err(syn::Error::new_spanned(other, "IDL methods must begin with `&self` or `&mut self`")),
    };

    let mut parameters = Vec::new();
    let mut names = HashSet::new();
    for input in inputs {
        let FnArg::Typed(argument) = input else {
            return Err(syn::Error::new_spanned(input, "a method can have only one receiver"));
        };
        let Pat::Ident(pattern) = argument.pat.as_ref() else {
            return Err(syn::Error::new_spanned(argument.pat, "IDL parameters require simple identifier patterns"));
        };
        if pattern.by_ref.is_some() || pattern.mutability.is_some() || pattern.subpat.is_some() {
            return Err(syn::Error::new_spanned(
                pattern,
                "IDL parameter patterns cannot use `ref`, `mut`, or subpatterns",
            ));
        }
        let name = pattern.ident.to_string();
        if !names.insert(name.clone()) {
            return Err(syn::Error::new_spanned(pattern, format!("duplicate parameter {name:?}")));
        }
        parameters.push(Parameter { name, ty: lower_type(&argument.ty)? });
    }

    let ReturnType::Type(_, output) = signature.output else {
        return Err(syn::Error::new_spanned(signature.ident, "IDL methods must return `Result<T>`"));
    };
    let output = lower_result_output(&output)?;
    Ok(Method {
        docs,
        name: signature.ident.to_string(),
        receiver,
        parameters,
        output,
    })
}

/// Extracts the successful value from the required single-argument `Result<T>`.
fn lower_result_output(output: &Type) -> syn::Result<AbiType> {
    let Type::Path(path) = output else {
        return Err(syn::Error::new_spanned(output, "IDL methods must return `Result<T>`"));
    };
    if path.qself.is_some() || path.path.segments.len() != 1 || path.path.segments[0].ident != "Result" {
        return Err(syn::Error::new_spanned(output, "IDL methods must return `Result<T>`"));
    }
    let PathArguments::AngleBracketed(arguments) = &path.path.segments[0].arguments else {
        return Err(syn::Error::new_spanned(output, "`Result` requires one successful output type"));
    };
    if arguments.args.len() != 1 {
        return Err(syn::Error::new_spanned(arguments, "IDL `Result` accepts exactly one output type"));
    }
    let Some(GenericArgument::Type(output)) = arguments.args.first() else {
        return Err(syn::Error::new_spanned(arguments, "IDL `Result` output must be a type"));
    };
    lower_type(output)
}

/// Lowers one supported Rust-like type spelling into explicit ABI semantics.
fn lower_type(source: &Type) -> syn::Result<AbiType> {
    match source {
        Type::Tuple(tuple) if tuple.elems.is_empty() => Ok(AbiType::Unit),
        Type::Path(path) if path.qself.is_none() => lower_path_type(&path.path),
        Type::Reference(reference) if reference.lifetime.is_none() && reference.mutability.is_none() => match reference.elem.as_ref() {
            Type::Slice(slice) => Ok(AbiType::Slice(Box::new(lower_type(&slice.elem)?))),
            referent => Ok(AbiType::Borrowed(Box::new(lower_type(referent)?))),
        },
        Type::Array(array) => {
            let len = parse_unsigned_integer(&array.len)?;
            let len = u32::try_from(len).map_err(|_| syn::Error::new_spanned(&array.len, "IDL array length exceeds u32"))?;
            Ok(AbiType::Array {
                element: Box::new(lower_type(&array.elem)?),
                len,
            })
        }
        Type::Slice(slice) => Err(syn::Error::new_spanned(slice, "IDL slices must be borrowed as `&[T]`")),
        _ => Err(syn::Error::new_spanned(source, "unsupported IDL ABI type")),
    }
}

/// Lowers a built-in, optional, or named path type.
fn lower_path_type(path: &syn::Path) -> syn::Result<AbiType> {
    if path.segments.len() == 1 {
        let segment = &path.segments[0];
        let name = segment.ident.to_string();
        if matches!(
            name.as_str(),
            "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64"
        ) {
            if segment.arguments != PathArguments::None {
                return Err(syn::Error::new_spanned(segment, "scalar IDL types do not accept generic arguments"));
            }
            return Ok(AbiType::Scalar(name));
        }
        if name == "String" {
            if segment.arguments != PathArguments::None {
                return Err(syn::Error::new_spanned(segment, "IDL `String` does not accept generic arguments"));
            }
            return Ok(AbiType::String);
        }
        if name == "Option" {
            let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return Err(syn::Error::new_spanned(segment, "IDL `Option` requires one value type"));
            };
            if arguments.args.len() != 1 {
                return Err(syn::Error::new_spanned(arguments, "IDL `Option` accepts exactly one value type"));
            }
            let Some(GenericArgument::Type(value)) = arguments.args.first() else {
                return Err(syn::Error::new_spanned(arguments, "IDL `Option` value must be a type"));
            };
            return Ok(AbiType::Option(Box::new(lower_type(value)?)));
        }
        if name == "Result" {
            return Err(syn::Error::new_spanned(segment, "IDL `Result` is valid only as a method return"));
        }
    }

    Ok(AbiType::Named(named_path(path)?))
}

/// Converts a nongeneric source path into a stable segment vector.
fn named_path(path: &syn::Path) -> syn::Result<NamedType> {
    let mut segments = Vec::with_capacity(path.segments.len());
    for segment in &path.segments {
        if segment.arguments != PathArguments::None {
            return Err(syn::Error::new_spanned(segment, "named IDL types cannot have generic arguments"));
        }
        segments.push(segment.ident.to_string());
    }
    if segments.is_empty() {
        return Err(syn::Error::new_spanned(path, "IDL type path cannot be empty"));
    }
    Ok(NamedType { segments })
}

/// Parses one named field and consumes its required comma when another follows.
fn parse_named_field(input: ParseStream<'_>) -> syn::Result<Field> {
    let attributes = Attribute::parse_outer(input)?;
    let docs = parse_doc_only_attributes(&attributes)?;
    if input.peek(Token![pub]) {
        return Err(input.error("IDL fields are public implicitly; remove `pub`"));
    }
    let name = input.parse::<Ident>()?;
    input.parse::<Token![:]>()?;
    let ty = lower_type(&input.parse::<Type>()?)?;
    if !input.is_empty() {
        input.parse::<Token![,]>()?;
    }
    Ok(Field { docs, name: name.to_string(), ty })
}

/// Parses one enum variant including an optional explicit discriminant.
fn parse_enum_variant(input: ParseStream<'_>) -> syn::Result<EnumVariant> {
    let attributes = Attribute::parse_outer(input)?;
    let docs = parse_doc_only_attributes(&attributes)?;
    let name = input.parse::<Ident>()?;
    let fields = if input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);
        let values = Punctuated::<Type, Token![,]>::parse_terminated(&content)?;
        if values.is_empty() {
            return Err(content.error("an empty tuple variant should be written without parentheses"));
        }
        VariantFields::Tuple(values.iter().map(lower_type).collect::<syn::Result<Vec<_>>>()?)
    } else if input.peek(syn::token::Brace) {
        let content;
        braced!(content in input);
        let mut values = Vec::new();
        while !content.is_empty() {
            values.push(parse_named_field(&content)?);
        }
        if values.is_empty() {
            return Err(content.error("an empty named variant should be written without braces"));
        }
        VariantFields::Named(values)
    } else {
        VariantFields::Unit
    };

    let discriminant = if input.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        Some(parse_signed_integer(&input.parse::<Expr>()?)?)
    } else {
        None
    };
    if !input.is_empty() {
        input.parse::<Token![,]>()?;
    }
    Ok(EnumVariant {
        docs,
        name: name.to_string(),
        discriminant,
        fields,
    })
}

/// Parses a nonnegative integer expression used as a fixed array length.
fn parse_unsigned_integer(expression: &Expr) -> syn::Result<u64> {
    let Expr::Lit(ExprLit { lit: Lit::Int(value), .. }) = expression else {
        return Err(syn::Error::new_spanned(expression, "expected an integer literal"));
    };
    value.base10_parse::<u64>()
}

/// Parses a signed integer literal without accepting computed expressions.
fn parse_signed_integer(expression: &Expr) -> syn::Result<i64> {
    match expression {
        Expr::Lit(ExprLit { lit: Lit::Int(value), .. }) => value.base10_parse::<i64>(),
        Expr::Unary(ExprUnary { op: UnOp::Neg(_), expr, .. }) => {
            let magnitude = parse_unsigned_integer(expr)?;
            let magnitude = i64::try_from(magnitude).map_err(|_| syn::Error::new_spanned(expression, "negative discriminant magnitude exceeds i64"))?;
            Ok(-magnitude)
        }
        _ => Err(syn::Error::new_spanned(expression, "enum discriminants must be integer literals")),
    }
}

/// Parses one dotted module import with one name or a braced name list.
fn parse_import(input: ParseStream<'_>) -> syn::Result<Import> {
    input.parse::<Token![use]>()?;
    let mut module = vec![input.parse::<Ident>()?.to_string()];
    while input.peek(Token![.]) {
        input.parse::<Token![.]>()?;
        module.push(input.parse::<Ident>()?.to_string());
    }
    if module.len() < 2 {
        return Err(input.error("IDL imports require a dotted module name"));
    }
    input.parse::<Token![::]>()?;

    let names = if input.peek(syn::token::Brace) {
        let body;
        braced!(body in input);
        let names = Punctuated::<Ident, Token![,]>::parse_terminated(&body)?;
        if names.is_empty() {
            return Err(body.error("IDL import groups cannot be empty"));
        }
        names.into_iter().map(|name| name.to_string()).collect::<Vec<_>>()
    } else {
        vec![input.parse::<Ident>()?.to_string()]
    };
    input.parse::<Token![;]>()?;

    let mut unique_names = HashSet::new();
    for name in &names {
        if !unique_names.insert(name) {
            return Err(input.error(format!("duplicate import name {name:?}")));
        }
    }
    Ok(Import { module: module.join("."), names })
}

/// Attribute capabilities allowed for one declaration category.
#[derive(Clone, Copy)]
enum AttributeContext {
    /// Structs accept identity and documentation only.
    Plain,
    /// Enums additionally accept a fixed integer representation.
    Enum,
    /// Interfaces accept identity and documentation only.
    Interface,
    /// Classes and factories additionally accept threading capability.
    Object,
}

/// Fully parsed common declaration attributes.
#[derive(Clone)]
struct Attributes {
    /// Documentation lines retained for generated APIs and tools.
    docs: Vec<String>,
    /// Required version and optional explicit UUID.
    id: DeclarationId,
    /// Enum representation when the declaration is an enum.
    repr: Option<EnumRepr>,
    /// Class or factory threading capability.
    threading: Option<Threading>,
}

/// Parses common attributes and rejects ambiguous or inapplicable combinations.
fn parse_attributes(attributes: Vec<Attribute>, context: AttributeContext) -> syn::Result<Attributes> {
    let mut docs = Vec::new();
    let mut id = None;
    let mut repr = None;
    let mut threading = None;

    for attribute in attributes {
        if attribute.path().is_ident("doc") {
            docs.push(parse_doc_attribute(&attribute)?);
        } else if attribute.path().is_ident("version") {
            if id.is_some() {
                return Err(syn::Error::new_spanned(attribute, "specify exactly one of `version` or `id`"));
            }
            let version = attribute.parse_args::<LitInt>()?.base10_parse::<u32>()?;
            id = Some(DeclarationId {
                version: positive_version(version, &attribute)?,
                explicit: None,
            });
        } else if attribute.path().is_ident("id") {
            if id.is_some() {
                return Err(syn::Error::new_spanned(attribute, "specify exactly one of `version` or `id`"));
            }
            let arguments = attribute.parse_args::<IdArguments>()?;
            id = Some(DeclarationId {
                version: positive_version(arguments.version, &attribute)?,
                explicit: Some(NcGuid::from_uuid_bytes(parse_uuid(&arguments.uuid)?)),
            });
        } else if attribute.path().is_ident("repr") && matches!(context, AttributeContext::Enum) {
            if repr.is_some() {
                return Err(syn::Error::new_spanned(attribute, "duplicate enum representation"));
            }
            repr = Some(parse_enum_repr(&attribute)?);
        } else if attribute.path().is_ident("threading") && matches!(context, AttributeContext::Object) {
            if threading.is_some() {
                return Err(syn::Error::new_spanned(attribute, "duplicate threading capability"));
            }
            threading = Some(parse_threading(&attribute)?);
        } else {
            return Err(syn::Error::new_spanned(attribute, "unsupported attribute for this IDL declaration"));
        }
    }

    let id = id.ok_or_else(|| syn::Error::new(Span::call_site(), "every IDL declaration requires `#[version(N)]` or `#[id(N, \"UUID\")]`"))?;
    Ok(Attributes { docs, id, repr, threading })
}

/// Returns an enum's declared representation or the stable `u32` default.
fn metadata_repr(metadata: &Attributes) -> EnumRepr {
    metadata.repr.unwrap_or(EnumRepr::U32)
}

/// Returns a callable object's declared capability or the conservative local default.
fn metadata_threading(metadata: &Attributes) -> Threading {
    metadata.threading.unwrap_or(Threading::Local)
}

/// Arguments accepted by `#[id(version, "UUID")]`.
struct IdArguments {
    /// Positive declaration version.
    version: u32,
    /// Canonical hyphenated UUID text.
    uuid: LitStr,
}

impl Parse for IdArguments {
    /// Parses exactly two comma-separated identity arguments.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let version = input.parse::<LitInt>()?.base10_parse::<u32>()?;
        input.parse::<Token![,]>()?;
        let uuid = input.parse::<LitStr>()?;
        if !input.is_empty() {
            return Err(input.error("`id` accepts exactly a version and UUID"));
        }
        Ok(Self { version, uuid })
    }
}

/// Rejects zero because it is reserved for missing or malformed metadata.
fn positive_version(version: u32, source: &impl quote::ToTokens) -> syn::Result<u32> {
    if version == 0 {
        Err(syn::Error::new_spanned(source, "IDL declaration versions must be greater than zero"))
    } else {
        Ok(version)
    }
}

/// Parses the six fixed enum representation spellings.
fn parse_enum_repr(attribute: &Attribute) -> syn::Result<EnumRepr> {
    let repr = attribute.parse_args::<Ident>()?;
    match repr.to_string().as_str() {
        "u8" => Ok(EnumRepr::U8),
        "u16" => Ok(EnumRepr::U16),
        "u32" => Ok(EnumRepr::U32),
        "i8" => Ok(EnumRepr::I8),
        "i16" => Ok(EnumRepr::I16),
        "i32" => Ok(EnumRepr::I32),
        _ => Err(syn::Error::new_spanned(repr, "IDL enum repr must be u8, u16, u32, i8, i16, or i32")),
    }
}

/// Parses one explicit class or factory threading capability.
fn parse_threading(attribute: &Attribute) -> syn::Result<Threading> {
    let capability = attribute.parse_args::<Ident>()?;
    match capability.to_string().as_str() {
        "local" => Ok(Threading::Local),
        "shared" => Ok(Threading::Shared),
        _ => Err(syn::Error::new_spanned(capability, "threading capability must be `local` or `shared`")),
    }
}

/// Extracts documentation while rejecting all other method or field attributes.
fn parse_doc_only_attributes(attributes: &[Attribute]) -> syn::Result<Vec<String>> {
    attributes
        .iter()
        .map(|attribute| {
            if attribute.path().is_ident("doc") {
                parse_doc_attribute(attribute)
            } else {
                Err(syn::Error::new_spanned(attribute, "only documentation attributes are valid here"))
            }
        })
        .collect()
}

/// Decodes one `///` comment represented by Syn as a name-value attribute.
fn parse_doc_attribute(attribute: &Attribute) -> syn::Result<String> {
    let syn::Meta::NameValue(value) = &attribute.meta else {
        return Err(syn::Error::new_spanned(attribute, "malformed documentation attribute"));
    };
    let Expr::Lit(ExprLit { lit: Lit::Str(text), .. }) = &value.value else {
        return Err(syn::Error::new_spanned(attribute, "documentation text must be a string"));
    };
    // Rust doc comments normally include one leading space. Removing only that
    // byte preserves intentional indentation in longer documentation blocks.
    let text = text.value();
    Ok(text.strip_prefix(' ').unwrap_or(&text).to_owned())
}

/// Parses a canonical lowercase or uppercase hyphenated UUID into network bytes.
fn parse_uuid(source: &LitStr) -> syn::Result<[u8; 16]> {
    let text = source.value();
    if text.len() != 36 {
        return Err(syn::Error::new_spanned(source, "explicit ID must be a 36-byte hyphenated UUID"));
    }
    let bytes = text.as_bytes();
    for index in [8, 13, 18, 23] {
        if bytes[index] != b'-' {
            return Err(syn::Error::new_spanned(source, "explicit ID must use canonical UUID hyphens"));
        }
    }

    let mut output = [0; 16];
    let mut source_index = 0;
    let mut output_index = 0;
    while output_index < output.len() {
        if matches!(source_index, 8 | 13 | 18 | 23) {
            source_index += 1;
        }
        let high = hex_value(bytes[source_index]).ok_or_else(|| syn::Error::new_spanned(source, "explicit ID contains a non-hexadecimal digit"))?;
        let low = hex_value(bytes[source_index + 1]).ok_or_else(|| syn::Error::new_spanned(source, "explicit ID contains a non-hexadecimal digit"))?;
        output[output_index] = (high << 4) | low;
        source_index += 2;
        output_index += 1;
    }
    Ok(output)
}

/// Converts one ASCII hexadecimal digit without locale or Unicode behavior.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
