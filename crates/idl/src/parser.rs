//! Parser for the intentionally small Rust-like NeoCogi IDL grammar.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    path::{Component, Path},
};

use neocogi_com_core::NcGuid;
use proc_macro2::Span;
use syn::{
    Attribute, Expr, ExprLit, ExprUnary, FnArg, GenericArgument, Ident, Lit, LitInt, LitStr, Pat, PathArguments, ReturnType, Token, TraitItemFn, Type,
    TypeParamBound, UnOp, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_str,
    punctuated::Punctuated,
    spanned::Spanned,
};

use crate::{
    PersistencePolicy, AbiType, BitEnum, BitFlag, Declaration, DeclarationId, DeclarationKind, Diagnostic, Enum, EnumNames, EnumRepr, EnumVariant,
    ExternalModule, Field, Import, Interface, Method, Module, ModuleName, ModulePathError, NamedType, Parameter, Receiver, SourceFile, SourceSpan, Struct,
    Threading, VariantFields,
};

mod keyword {
    //! Custom declaration keywords that intentionally are not Rust items.

    syn::custom_keyword!(interface);
    syn::custom_keyword!(bitenum);
}

/// Failure to derive a module name or parse and merge its IDL source parts.
#[derive(Debug)]
pub enum ParseError {
    /// The relative source path could not define a portable logical module.
    ModulePath(ModulePathError),
    /// The source violated the supported IDL grammar or structural rules.
    Source(
        /// Structured compiler diagnostic with exact source annotations.
        Diagnostic,
    ),
}

impl fmt::Display for ParseError {
    /// Writes the precise path or source parser diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModulePath(error) => write!(formatter, "{error}"),
            Self::Source(error) => write!(formatter, "{error}"),
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

/// One borrowed source part supplied to [`parse_module_parts`].
#[derive(Clone, Copy, Debug)]
pub struct ModuleSource<'a> {
    /// Portable source path relative to the configured IDL root.
    pub path: &'a Path,
    /// Exact UTF-8 contents of this part.
    pub text: &'a str,
}

/// Parses one source file whose relative path defines its logical module name.
pub fn parse_module(relative_path: &Path, source: &str) -> Result<Module, ParseError> {
    let name = ModuleName::from_relative_source(relative_path)?;
    assemble_module(name, [ModuleSource { path: relative_path, text: source }])
}

/// Parses and merges every source part belonging to one logical IDL module.
///
/// `relative_directory` alone defines module identity. Part filenames and
/// caller enumeration order are presentation details and never enter the ABI
/// fingerprint. Each part must be a `.ncidl` file below that directory.
pub fn parse_module_parts(relative_directory: &Path, parts: &[ModuleSource<'_>]) -> Result<Module, ParseError> {
    let name = ModuleName::from_relative_directory(relative_directory)?;
    for part in parts {
        let normalized = !part.path.is_absolute()
            && part
                .path
                .components()
                .all(|component| matches!(component, Component::Normal(value) if value.to_str().is_some()));
        if !normalized || part.path.extension().and_then(|extension| extension.to_str()) != Some("ncidl") || !part.path.starts_with(relative_directory) {
            return Err(ParseError::Source(Diagnostic::new(
                "EIDL002",
                format!(
                    "IDL part {} must be a .ncidl file below module directory {}",
                    part.path.display(),
                    relative_directory.display()
                ),
            )));
        }
    }
    assemble_module(name, parts.iter().copied())
}

/// Parses source parts in portable path order and builds one canonical module.
fn assemble_module<'a>(name: ModuleName, parts: impl IntoIterator<Item = ModuleSource<'a>>) -> Result<Module, ParseError> {
    let mut parts = parts.into_iter().collect::<Vec<_>>();
    parts.sort_by_key(|part| part.path.to_string_lossy().replace('\\', "/"));

    let mut source_paths = HashSet::new();
    let mut sources = Vec::with_capacity(parts.len());
    let mut externals = BTreeMap::<String, ExternalModule>::new();
    let mut imports = Vec::new();
    let mut declarations = BTreeMap::<String, Declaration>::new();
    for part in parts {
        let source_file = SourceFile::new(&name, part.path, part.text);
        if !source_paths.insert(source_file.path().to_owned()) {
            return Err(ParseError::Source(
                Diagnostic::new("EIDL003", format!("IDL module contains source path {:?} more than once", source_file.path())).with_source(&source_file),
            ));
        }
        let mut syntax = parse_str::<ModuleSyntax>(part.text).map_err(|error| ParseError::Source(Diagnostic::from_syn(&source_file, error)))?;
        syntax.attach_source(&source_file);
        // A dependency is a module-level fact, so repeating it in another
        // part is an error rather than an instruction to merge two edges.
        for external in syntax.externals {
            if let Some(previous) = externals.get(&external.module) {
                return Err(ParseError::Source(
                    Diagnostic::new("EIDL019", format!("duplicate external dependency {:?}", external.module))
                        .with_sources(&[sources.as_slice(), std::slice::from_ref(&source_file)].concat())
                        .primary(&external.span, "duplicate external dependency")
                        .secondary(&previous.span, "dependency first declared here"),
                ));
            }
            externals.insert(external.module.clone(), external);
        }
        imports.extend(syntax.imports);
        for declaration in syntax.declarations {
            if let Some(previous) = declarations.get(&declaration.name) {
                return Err(ParseError::Source(
                    Diagnostic::new("EIDL010", format!("duplicate declaration {:?}", declaration.name))
                        .with_sources(&[sources.as_slice(), std::slice::from_ref(&source_file)].concat())
                        .primary(&declaration.span, "duplicate declaration")
                        .secondary(&previous.span, "first declared here"),
                ));
            }
            declarations.insert(declaration.name.clone(), declaration);
        }
        sources.push(source_file);
    }

    if declarations.is_empty() {
        return Err(ParseError::Source(
            Diagnostic::new("EIDL004", "an IDL module must declare at least one ABI item").with_sources(&sources),
        ));
    }

    Ok(Module {
        name,
        sources,
        externals: externals.into_values().collect(),
        imports: canonical_imports(imports),
        declarations: declarations.into_values().collect(),
        templates: Vec::new(),
    })
}

/// Deduplicates exact bindings while retaining each name's first authored origin.
/// Canonical module/name order is independent of source-part enumeration.
fn canonical_imports(imports: Vec<Import>) -> Vec<Import> {
    let mut bindings = BTreeMap::new();
    for import in imports {
        bindings.entry((import.module.clone(), import.name.clone())).or_insert(import);
    }
    bindings.into_values().collect()
}

/// Parsed top-level source before its path-derived module name is attached.
struct ModuleSyntax {
    /// External module dependencies retained for graph construction.
    externals: Vec<ExternalModule>,
    /// One source-bearing import per declaration name.
    imports: Vec<Import>,
    /// Declarations retained in normative source order.
    declarations: Vec<Declaration>,
}

impl ModuleSyntax {
    /// Attaches one source path recursively to every retained syntax range.
    fn attach_source(&mut self, source: &SourceFile) {
        for external in &mut self.externals {
            external.span.attach(source);
        }
        for import in &mut self.imports {
            import.span.attach(source);
        }
        for declaration in &mut self.declarations {
            attach_declaration_source(declaration, source);
        }
    }
}

impl Parse for ModuleSyntax {
    /// Parses dependencies, imports, and declarations while rejecting every other Rust item.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut externals = Vec::new();
        let mut imports = Vec::new();
        let mut declarations = Vec::new();
        let mut declaration_names = HashMap::new();

        while !input.is_empty() {
            let attributes = Attribute::parse_outer(input)?;
            if input.peek(Token![pub]) {
                return Err(input.error("IDL declarations are public implicitly; remove `pub`"));
            }

            if input.peek(Token![extern]) {
                if !attributes.is_empty() {
                    return Err(syn::Error::new_spanned(&attributes[0], "IDL external dependencies do not accept attributes"));
                }
                externals.push(parse_external(input)?);
                continue;
            }

            if input.peek(Token![use]) {
                if !attributes.is_empty() {
                    return Err(syn::Error::new_spanned(&attributes[0], "IDL imports do not accept attributes"));
                }
                imports.extend(parse_import(input)?);
                continue;
            }

            let declaration = if input.peek(Token![struct]) {
                parse_struct(attributes, input)?
            } else if input.peek(Token![enum]) {
                parse_enum(attributes, input)?
            } else if input.peek(keyword::bitenum) {
                parse_bit_enum(attributes, input)?
            } else if input.peek(keyword::interface) {
                parse_interface(attributes, input)?
            } else {
                return Err(input.error("expected `extern`, `use`, `struct`, `enum`, `bitenum`, or `interface`"));
            };

            if let Some(previous) = declaration_names.insert(declaration.name.clone(), declaration.span.clone()) {
                return Err(duplicate_error("EIDL010", "declaration", &declaration.name, &declaration.span, &previous));
            }
            declarations.push(declaration);
        }

        Ok(Self { externals, imports, declarations })
    }
}

/// Parses one canonical dotted external module dependency.
fn parse_external(input: ParseStream<'_>) -> syn::Result<ExternalModule> {
    let extern_token = input.parse::<Token![extern]>()?;
    // Dots belong to the IDL module-name grammar; Rust `::` remains reserved
    // for paths to declarations inside an already-declared dependency.
    let mut module = vec![input.parse::<Ident>()?.to_string()];
    while input.peek(Token![.]) {
        input.parse::<Token![.]>()?;
        module.push(input.parse::<Ident>()?.to_string());
    }
    if module.len() < 2 {
        // Requiring a dotted absolute name prevents hidden parent-namespace
        // lookup from changing meaning when the importing module moves.
        return Err(input.error("IDL external dependencies require a fully qualified dotted module name"));
    }
    input.parse::<Token![;]>()?;
    Ok(ExternalModule {
        span: SourceSpan::from_parser(extern_token.span),
        module: module.join("."),
    })
}

/// Parses one fixed-width bit set with explicit named flag patterns.
fn parse_bit_enum(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::BitEnum)?;
    input.parse::<keyword::bitenum>()?;
    let name = input.parse::<Ident>()?;
    require_upper_camel_case(&name, "bit enum")?;
    if !input.peek(Token![:]) {
        return Err(syn::Error::new(name.span(), "IDL bit enums require an explicit unsigned representation"));
    }
    let repr = parse_optional_enum_repr(input)?;
    if !matches!(repr, EnumRepr::U8 | EnumRepr::U16 | EnumRepr::U32) {
        return Err(syn::Error::new(name.span(), "IDL bit enum representation must be u8, u16, or u32"));
    }

    let body;
    braced!(body in input);
    let mut flags = Vec::new();
    let mut names = HashMap::new();
    while !body.is_empty() {
        let flag = parse_bit_flag(&body)?;
        if let Some(previous) = names.insert(flag.name.clone(), flag.span.clone()) {
            return Err(duplicate_error("EIDL014", "bit enum flag", &flag.name, &flag.span, &previous));
        }
        flags.push(flag);
    }
    if flags.is_empty() {
        return Err(syn::Error::new(name.span(), "IDL bit enums must contain at least one flag"));
    }
    Ok(declaration(name, Vec::new(), metadata, DeclarationKind::BitEnum(BitEnum { repr, flags })))
}

/// Parses one copied record declaration.
fn parse_struct(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::Plain)?;
    input.parse::<Token![struct]>()?;
    let name = input.parse::<Ident>()?;
    require_upper_camel_case(&name, "struct")?;
    let parameters = parse_type_parameters(input)?;
    let body;
    braced!(body in input);
    let mut fields = Vec::new();
    let mut names = HashMap::new();
    while !body.is_empty() {
        let field = parse_named_field(&body)?;
        if let Some(previous) = names.insert(field.name.clone(), field.span.clone()) {
            return Err(duplicate_error("EIDL013", "field", &field.name, &field.span, &previous));
        }
        fields.push(field);
    }
    if fields.is_empty() {
        return Err(syn::Error::new(name.span(), "IDL structs must contain at least one field"));
    }
    Ok(declaration(
        name,
        parameters,
        metadata.clone(),
        DeclarationKind::Struct(Struct {
            fields,
            persistence: metadata.persistence,
        }),
    ))
}

/// Parses one enum with unit, tuple, or named payload variants.
fn parse_enum(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::Enum)?;
    input.parse::<Token![enum]>()?;
    let name = input.parse::<Ident>()?;
    require_upper_camel_case(&name, "enum")?;
    let parameters = parse_type_parameters(input)?;
    let repr = parse_optional_enum_repr(input)?;
    let body;
    braced!(body in input);
    let mut variants = Vec::new();
    let mut names = HashMap::new();
    while !body.is_empty() {
        let variant = parse_enum_variant(&body)?;
        if let Some(previous) = names.insert(variant.name.clone(), variant.span.clone()) {
            return Err(duplicate_error("EIDL015", "enum variant", &variant.name, &variant.span, &previous));
        }
        variants.push(variant);
    }
    if variants.is_empty() {
        return Err(syn::Error::new(name.span(), "IDL enums must contain at least one variant"));
    }
    Ok(declaration(
        name,
        parameters,
        metadata.clone(),
        DeclarationKind::Enum(Enum {
            repr,
            dense: metadata.dense,
            names: metadata.enum_names,
            variants,
        }),
    ))
}

/// Parses one callable contract with threading, projections and ordinary methods.
fn parse_interface(attributes: Vec<Attribute>, input: ParseStream<'_>) -> syn::Result<Declaration> {
    let metadata = parse_attributes(attributes, AttributeContext::Interface)?;
    let threading = metadata_threading(&metadata);
    let events = metadata.events.clone();
    // Dispatch already identified the keyword. Consuming it here keeps one
    // grammar for identity, projections, threading, and callable ordering.
    input.parse::<Ident>()?;
    let name = input.parse::<Ident>()?;
    require_upper_camel_case(&name, "interface")?;
    let parameters = parse_type_parameters(input)?;
    let bases = parse_bases(input)?;
    let methods = parse_methods(input)?;
    Ok(declaration(
        name,
        parameters,
        metadata,
        DeclarationKind::Interface(Interface { bases, events, methods, threading }),
    ))
}

/// Combines common metadata with one declaration-specific payload.
fn declaration(name: Ident, parameters: Vec<String>, metadata: Attributes, kind: DeclarationKind) -> Declaration {
    Declaration {
        span: SourceSpan::from_parser(name.span()),
        docs: metadata.docs,
        name: name.to_string(),
        id: metadata.id,
        parameters,
        specialization: None,
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

/// Parses ordered methods, reserving method names.
fn parse_methods(input: ParseStream<'_>) -> syn::Result<Vec<Method>> {
    let body;
    braced!(body in input);
    let mut methods = Vec::new();
    let mut names = HashMap::new();
    while !body.is_empty() {
        let attributes = Attribute::parse_outer(&body)?;
        let mut source = body.parse::<TraitItemFn>()?;
        source.attrs.splice(0..0, attributes);
        let method = lower_method(source)?;
        if let Some(previous) = names.insert(method.name.clone(), method.span.clone()) {
            return Err(duplicate_error("EIDL016", "method or port", &method.name, &method.span, &previous));
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
    require_snake_case(&signature.ident, "method")?;
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
    let mut names = HashMap::new();
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
        require_snake_case(&pattern.ident, "parameter")?;
        let span = SourceSpan::from_parser(pattern.ident.span());
        if let Some(previous) = names.insert(name.clone(), span.clone()) {
            return Err(duplicate_error("EIDL017", "parameter", &name, &span, &previous));
        }
        parameters.push(Parameter {
            span,
            name,
            ty: lower_type(&argument.ty)?,
        });
    }

    let ReturnType::Type(_, output) = signature.output else {
        return Err(syn::Error::new_spanned(signature.ident, "IDL methods must return `Result<T>`"));
    };
    let output = lower_result_output(&output)?;
    Ok(Method {
        span: SourceSpan::from_parser(signature.ident.span()),
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
        Type::Reference(reference) if reference.lifetime.is_none() => {
            // Bracket and intrinsic spellings normalize before ownership validation.
            let referent = lower_type(&reference.elem)?;
            match (reference.mutability.is_some(), referent) {
                (false, AbiType::UnsizedSlice(element)) => Ok(AbiType::Slice(element)),
                (true, AbiType::UnsizedSlice(element)) => Ok(AbiType::OutputSlice(element)),
                (false, referent) => Ok(AbiType::Borrowed(Box::new(referent))),
                (true, _) => Err(syn::Error::new_spanned(reference, "IDL mutable borrows must use `&mut [T]` or `&mut Slice<T>`")),
            }
        }
        Type::Array(array) => {
            let len = parse_unsigned_integer(&array.len)?;
            let len = u32::try_from(len).map_err(|_| syn::Error::new_spanned(&array.len, "IDL array length exceeds u32"))?;
            Ok(AbiType::Array {
                element: Box::new(lower_type(&array.elem)?),
                len,
            })
        }
        Type::Slice(slice) => Ok(AbiType::UnsizedSlice(Box::new(lower_type(&slice.elem)?))),
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
            "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64" | "Guid" | "Status"
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
        if name == "Array" {
            let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return Err(syn::Error::new_spanned(segment, "Array requires an element type and integer length"));
            };
            let arguments = arguments.args.iter().collect::<Vec<_>>();
            let [GenericArgument::Type(element), GenericArgument::Const(length)] = arguments.as_slice() else {
                return Err(syn::Error::new_spanned(
                    segment,
                    "Array requires Array<T, N> with a literal nonnegative u32 length",
                ));
            };
            let len = u32::try_from(parse_unsigned_integer(length)?).map_err(|_| syn::Error::new_spanned(length, "IDL array length exceeds u32"))?;
            return Ok(AbiType::Array {
                element: Box::new(lower_type(element)?),
                len,
            });
        }
        if matches!(name.as_str(), "ComPtr" | "WeakComPtr" | "ComBox" | "WeakComBox" | "Vec" | "Slice") {
            let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return Err(syn::Error::new_spanned(segment, format!("IDL `{name}` requires one type argument")));
            };
            if arguments.args.len() != 1 {
                return Err(syn::Error::new_spanned(arguments, format!("IDL `{name}` accepts exactly one type argument")));
            }
            let Some(GenericArgument::Type(value)) = arguments.args.first() else {
                return Err(syn::Error::new_spanned(arguments, format!("IDL `{name}` argument must be a type")));
            };
            return Ok(match name.as_str() {
                "ComPtr" => AbiType::ComPtr(Box::new(lower_type(value)?)),
                "WeakComPtr" => AbiType::WeakComPtr(Box::new(lower_type(value)?)),
                "ComBox" => AbiType::ComBox(Box::new(lower_type(value)?)),
                "WeakComBox" => AbiType::WeakComBox(Box::new(lower_type(value)?)),
                "Vec" => AbiType::Vector(Box::new(lower_type(value)?)),
                "Slice" => AbiType::UnsizedSlice(Box::new(lower_type(value)?)),
                _ => unreachable!("intrinsic constructor was checked above"),
            });
        }
        if name == "Result" {
            return Err(syn::Error::new_spanned(segment, "IDL `Result` is valid only as a method return"));
        }
    }

    Ok(AbiType::Named(named_path(path)?))
}

/// Parses named type arguments without accepting lifetime, const, or associated-type syntax.
fn named_path(path: &syn::Path) -> syn::Result<NamedType> {
    if path.leading_colon.is_some() {
        return Err(syn::Error::new_spanned(path, "IDL paths are module-qualified without a leading ::"));
    }
    let mut segments = Vec::new();
    let mut arguments = Vec::new();
    for (index, segment) in path.segments.iter().enumerate() {
        segments.push(segment.ident.to_string());
        match &segment.arguments {
            PathArguments::None => {}
            PathArguments::AngleBracketed(values) if index + 1 == path.segments.len() && !values.args.is_empty() => {
                for value in &values.args {
                    match value {
                        GenericArgument::Type(value) => arguments.push(lower_type(value)?),
                        _ => return Err(syn::Error::new_spanned(value, "generic arguments must be types")),
                    }
                }
            }
            _ => return Err(syn::Error::new_spanned(segment, "type arguments belong on the final interface name")),
        }
    }
    if segments.is_empty() {
        return Err(syn::Error::new_spanned(path, "IDL type path cannot be empty"));
    }
    Ok(NamedType {
        binding: crate::binding::Binding::Unbound,
        span: SourceSpan::from_parser(path.span()),
        segments,
        arguments,
    })
}

/// Accepts a small ordered set of plain type parameters; bounds are checked at each closed use.
fn parse_type_parameters(input: ParseStream<'_>) -> syn::Result<Vec<String>> {
    if !input.peek(Token![<]) {
        return Ok(Vec::new());
    }
    input.parse::<Token![<]>()?;
    let mut parameters = Vec::new();
    loop {
        let parameter = input.parse::<Ident>()?;
        require_upper_camel_case(&parameter, "type parameter")?;
        let name = parameter.to_string();
        if crate::resolve::is_reserved_name(&name) {
            return Err(syn::Error::new_spanned(parameter, "a type parameter cannot shadow an IDL built-in"));
        }
        if parameters.contains(&name) {
            return Err(syn::Error::new_spanned(parameter, "duplicate type parameter"));
        }
        parameters.push(name);
        if !input.peek(Token![,]) {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    input.parse::<Token![>]>()?;
    Ok(parameters)
}

/// Parses one named field and consumes its required comma when another follows.
fn parse_named_field(input: ParseStream<'_>) -> syn::Result<Field> {
    let attributes = Attribute::parse_outer(input)?;
    let docs = parse_doc_only_attributes(&attributes)?;
    if input.peek(Token![pub]) {
        return Err(input.error("IDL fields are public implicitly; remove `pub`"));
    }
    let name = input.parse::<Ident>()?;
    require_snake_case(&name, "field")?;
    input.parse::<Token![:]>()?;
    let ty = lower_type(&input.parse::<Type>()?)?;
    if !input.is_empty() {
        input.parse::<Token![,]>()?;
    }
    Ok(Field {
        span: SourceSpan::from_parser(name.span()),
        docs,
        name: name.to_string(),
        ty,
    })
}

/// Parses one enum variant including an optional explicit discriminant.
fn parse_enum_variant(input: ParseStream<'_>) -> syn::Result<EnumVariant> {
    let attributes = Attribute::parse_outer(input)?;
    let docs = parse_doc_only_attributes(&attributes)?;
    let name = input.parse::<Ident>()?;
    require_upper_camel_case(&name, "enum variant")?;
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
        let mut names = HashMap::new();
        while !content.is_empty() {
            let field = parse_named_field(&content)?;
            if let Some(previous) = names.insert(field.name.clone(), field.span.clone()) {
                return Err(duplicate_error("EIDL013", "variant field", &field.name, &field.span, &previous));
            }
            values.push(field);
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
        span: SourceSpan::from_parser(name.span()),
        docs,
        name: name.to_string(),
        discriminant,
        fields,
    })
}

/// Parses one explicitly valued bit flag and consumes its trailing comma.
fn parse_bit_flag(input: ParseStream<'_>) -> syn::Result<BitFlag> {
    let attributes = Attribute::parse_outer(input)?;
    let docs = parse_doc_only_attributes(&attributes)?;
    let name = input.parse::<Ident>()?;
    require_screaming_snake_case(&name, "bit enum flag")?;
    input.parse::<Token![=]>()?;
    let value = parse_unsigned_integer(&input.parse::<Expr>()?)?;
    if !input.is_empty() {
        input.parse::<Token![,]>()?;
    }
    Ok(BitFlag {
        span: SourceSpan::from_parser(name.span()),
        docs,
        name: name.to_string(),
        value,
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
fn parse_import(input: ParseStream<'_>) -> syn::Result<Vec<Import>> {
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
        names.into_iter().collect::<Vec<_>>()
    } else {
        vec![input.parse::<Ident>()?]
    };
    input.parse::<Token![;]>()?;

    let mut unique_names = HashMap::new();
    let mut normalized_names = Vec::with_capacity(names.len());
    for name in names {
        require_upper_camel_case(&name, "imported declaration")?;
        let normalized = name.to_string();
        let span = SourceSpan::from_parser(name.span());
        if let Some(previous) = unique_names.insert(normalized.clone(), span.clone()) {
            return Err(duplicate_error("EIDL018", "import name", &normalized, &span, &previous));
        }
        normalized_names.push(Import {
            span,
            module: module.join("."),
            name: normalized,
        });
    }
    Ok(normalized_names)
}

/// Produces one syntax error with both duplicate and original declarations.
fn duplicate_error(code: &str, role: &str, name: &str, current: &SourceSpan, previous: &SourceSpan) -> syn::Error {
    let mut error = syn::Error::new(current.parser_span(), format!("[{code}] duplicate {role} {name:?}"));
    error.combine(syn::Error::new(previous.parser_span(), "first declared here"));
    error
}

/// Attaches a source file to all spans nested under one declaration.
fn attach_declaration_source(declaration: &mut Declaration, source: &SourceFile) {
    declaration.span.attach(source);
    match &mut declaration.kind {
        DeclarationKind::Struct(record) => {
            for field in &mut record.fields {
                attach_field_source(field, source);
            }
        }
        DeclarationKind::Enum(enumeration) => {
            for variant in &mut enumeration.variants {
                variant.span.attach(source);
                match &mut variant.fields {
                    VariantFields::Unit => {}
                    VariantFields::Tuple(fields) => {
                        for field in fields {
                            attach_type_source(field, source);
                        }
                    }
                    VariantFields::Named(fields) => {
                        for field in fields {
                            attach_field_source(field, source);
                        }
                    }
                }
            }
        }
        DeclarationKind::BitEnum(bit_enum) => {
            for flag in &mut bit_enum.flags {
                flag.span.attach(source);
            }
        }
        DeclarationKind::Interface(interface) => {
            attach_interface_source(&mut interface.bases, &mut interface.methods, source);
            for event in &mut interface.events {
                attach_named_source(event, source);
            }
        }
    }
}

/// Attaches a source file to one field and all types nested inside it.
fn attach_field_source(field: &mut Field, source: &SourceFile) {
    field.span.attach(source);
    attach_type_source(&mut field.ty, source);
}

/// Attaches a source file to bases, methods, parameters, and result types.
fn attach_interface_source(bases: &mut [NamedType], methods: &mut [Method], source: &SourceFile) {
    for base in bases {
        attach_named_source(base, source);
    }
    for method in methods {
        attach_method_source(method, source);
    }
}

/// Attaches one ordinary or event-delivery signature and every nested input span.
fn attach_method_source(method: &mut Method, source: &SourceFile) {
    method.span.attach(source);
    for parameter in &mut method.parameters {
        parameter.span.attach(source);
        attach_type_source(&mut parameter.ty, source);
    }
    attach_type_source(&mut method.output, source);
}

/// Attaches the authored source to a named reference and every nested argument.
/// Direct base/event substitution can move an argument into the reference slot;
/// its original source owner must survive that move just as in ordinary types.
fn attach_named_source(name: &mut NamedType, source: &SourceFile) {
    name.span.attach(source);
    for argument in &mut name.arguments {
        attach_type_source(argument, source);
    }
}

/// Attaches a source file to every named path nested in one ABI type.
fn attach_type_source(ty: &mut AbiType, source: &SourceFile) {
    match ty {
        AbiType::Named(name) => attach_named_source(name, source),
        AbiType::ComPtr(value)
        | AbiType::WeakComPtr(value)
        | AbiType::ComBox(value)
        | AbiType::WeakComBox(value)
        | AbiType::Borrowed(value)
        | AbiType::Vector(value)
        | AbiType::UnsizedSlice(value)
        | AbiType::Slice(value)
        | AbiType::OutputSlice(value)
        | AbiType::Array { element: value, .. } => attach_type_source(value, source),
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
    }
}

/// Requires the Rust convention used by every generated type declaration and enum variant.
fn require_upper_camel_case(identifier: &Ident, role: &str) -> syn::Result<()> {
    let value = identifier.to_string();
    let mut characters = value.chars();
    let valid = characters.next().is_some_and(|first| first.is_ascii_uppercase()) && characters.all(|character| character.is_ascii_alphanumeric());
    if valid {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            identifier,
            format!("IDL {role} name {value:?} must use UpperCamelCase"),
        ))
    }
}

/// Requires the Rust convention used by generated fields, methods, and parameters.
fn require_snake_case(identifier: &Ident, role: &str) -> syn::Result<()> {
    let value = identifier.to_string();
    let bytes = value.as_bytes();
    let valid = bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes.last().is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes.iter().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
        && !bytes.windows(2).any(|pair| pair == b"__");
    if valid {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(identifier, format!("IDL {role} name {value:?} must use snake_case")))
    }
}

/// Requires the Rust constant convention reserved for combinable bit flags.
fn require_screaming_snake_case(identifier: &Ident, role: &str) -> syn::Result<()> {
    let value = identifier.to_string();
    let bytes = value.as_bytes();
    let valid = bytes.first().is_some_and(u8::is_ascii_uppercase)
        && bytes.last().is_some_and(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        && bytes.iter().all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b'_')
        && !bytes.windows(2).any(|pair| pair == b"__");
    if valid {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            identifier,
            format!("IDL {role} name {value:?} must use SCREAMING_SNAKE_CASE"),
        ))
    }
}

/// Attribute capabilities allowed for one declaration category.
#[derive(Clone, Copy)]
enum AttributeContext {
    /// Structs additionally accept format-neutral generated decoding policy.
    Plain,
    /// Enums accept identity and documentation; their representation is syntax.
    Enum,
    /// Bit enums accept only identity and documentation attributes.
    BitEnum,
    /// Interfaces additionally accept threading capability.
    Interface,
}

/// Fully parsed common declaration attributes.
#[derive(Clone)]
struct Attributes {
    /// Documentation lines retained for generated APIs and tools.
    docs: Vec<String>,
    /// Required version and optional explicit UUID.
    id: DeclarationId,
    /// Interface threading capability.
    threading: Option<Threading>,
    /// Authored emitted event interfaces, independent of constructor signatures.
    events: Vec<NamedType>,
    /// Typed construction policy selected independently of live transport.
    persistence: PersistencePolicy,
    /// Whether an enum is a zero-based contiguous unit-value domain.
    dense: bool,
    /// Optional canonical naming policy for dense enum values.
    enum_names: Option<EnumNames>,
}

/// Parses common attributes and rejects ambiguous or inapplicable combinations.
fn parse_attributes(attributes: Vec<Attribute>, context: AttributeContext) -> syn::Result<Attributes> {
    let mut docs = Vec::new();
    let mut id = None;
    let mut threading = None;
    let mut events = None;
    let mut persistence = PersistencePolicy::Automatic;
    let mut dense = false;
    let mut enum_names = None;

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
        } else if attribute.path().is_ident("events") && matches!(context, AttributeContext::Interface) {
            if events.is_some() {
                return Err(syn::Error::new_spanned(attribute, "duplicate event declaration"));
            }
            let names = attribute.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)?;
            if names.is_empty() {
                return Err(syn::Error::new_spanned(attribute, "events requires at least one event interface"));
            }
            events = Some(names.iter().map(named_path).collect::<syn::Result<Vec<_>>>()?);
        } else if attribute.path().is_ident("threading") && matches!(context, AttributeContext::Interface) {
            if threading.is_some() {
                return Err(syn::Error::new_spanned(attribute, "duplicate threading capability"));
            }
            threading = Some(parse_threading(&attribute)?);
        } else if attribute.path().is_ident("persistence") && matches!(context, AttributeContext::Plain) {
            if persistence.required() {
                return Err(syn::Error::new_spanned(attribute, "duplicate persistence policy"));
            }
            persistence = parse_persistence(&attribute)?;
        } else if attribute.path().is_ident("dense") && matches!(context, AttributeContext::Enum) {
            if dense {
                return Err(syn::Error::new_spanned(attribute, "duplicate dense enum policy"));
            }
            if !matches!(&attribute.meta, syn::Meta::Path(_)) {
                return Err(syn::Error::new_spanned(attribute, "`dense` does not accept arguments"));
            }
            dense = true;
        } else if attribute.path().is_ident("names") && matches!(context, AttributeContext::Enum) {
            if enum_names.is_some() {
                return Err(syn::Error::new_spanned(attribute, "duplicate enum names policy"));
            }
            let policy = attribute.parse_args::<Ident>()?;
            if policy != "snake_case" {
                return Err(syn::Error::new_spanned(policy, "enum names policy must be `snake_case`"));
            }
            enum_names = Some(EnumNames::SnakeCase);
        } else {
            return Err(syn::Error::new_spanned(attribute, "unsupported attribute for this IDL declaration"));
        }
    }

    let id = id.ok_or_else(|| syn::Error::new(Span::call_site(), "every IDL declaration requires `#[version(N)]` or `#[id(N, \"UUID\")]`"))?;
    Ok(Attributes {
        docs,
        id,
        threading,
        events: events.unwrap_or_default(),
        persistence,
        dense,
        enum_names,
    })
}

/// Parses one mutually exclusive construction policy without ambiguous boolean states.
fn parse_persistence(attribute: &Attribute) -> syn::Result<PersistencePolicy> {
    let policy = attribute.parse_args::<Ident>()?;
    match policy.to_string().as_str() {
        "default" => Ok(PersistencePolicy::Default),
        _ => Err(syn::Error::new_spanned(
            policy,
            "expected default; NCB fields are always positional and ordinary value persistence is automatic",
        )),
    }
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

/// Parses an optional `: integer` enum suffix, defaulting to `u32`.
fn parse_optional_enum_repr(input: ParseStream<'_>) -> syn::Result<EnumRepr> {
    if !input.peek(Token![:]) {
        return Ok(EnumRepr::U32);
    }
    input.parse::<Token![:]>()?;
    let repr = input.parse::<Ident>()?;
    match repr.to_string().as_str() {
        "u8" => Ok(EnumRepr::U8),
        "u16" => Ok(EnumRepr::U16),
        "u32" => Ok(EnumRepr::U32),
        "i8" => Ok(EnumRepr::I8),
        "i16" => Ok(EnumRepr::I16),
        "i32" => Ok(EnumRepr::I32),
        _ => Err(syn::Error::new_spanned(repr, "IDL enum representation must be u8, u16, u32, i8, i16, or i32")),
    }
}

/// Parses one explicit interface threading capability.
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
