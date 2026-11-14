//! Parser for the intentionally small Rust-like NeoCogi IDL grammar.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    path::{Component, Path},
};

use neocogi_com_core::NcGuid;
use proc_macro2::Span;
use syn::{
    Attribute, Expr, ExprLit, Ident, Lit, LitInt, LitStr, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
    ext::IdentExt,
    punctuated::Punctuated,
    spanned::Spanned,
};

use crate::{
    PersistencePolicy, AbiType, BitEnum, BitFlag, Declaration, DeclarationKind, Diagnostic, Enum, EnumNames, EnumRepr, EnumVariant, ExternalModule, Field,
    Import, Interface, Method, Module, ModuleName, ModulePathError, NamedType, Parameter, Receiver, SourceFile, SourceSpan, Struct, Threading, VariantFields,
};

mod attributes;
mod methods;
mod types;

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
        /// Owned source diagnostic, boxed to keep fallible compiler APIs small
        /// as structured diagnostics gain nonfatal severity and source metadata.
        Box<Diagnostic>,
    ),
    /// The compiler could not create its explicitly sized IDL syntax worker.
    Worker(
        /// Operating-system reason the syntax worker could not be started.
        std::io::Error,
    ),
}

impl fmt::Display for ParseError {
    /// Writes the precise path or source parser diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModulePath(error) => write!(formatter, "{error}"),
            Self::Source(error) => write!(formatter, "{error}"),
            Self::Worker(error) => write!(formatter, "cannot start IDL syntax worker: {error}"),
        }
    }
}

impl std::error::Error for ParseError {
    /// Exposes environmental/path causes without manufacturing errors from diagnostics.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModulePath(source) => Some(source),
            Self::Worker(source) => Some(source),
            Self::Source(_) => None,
        }
    }
}

impl From<ModulePathError> for ParseError {
    /// Preserves a module-path failure without erasing its category.
    fn from(error: ModulePathError) -> Self {
        Self::ModulePath(error)
    }
}

impl From<Diagnostic> for ParseError {
    /// Transfers a source failure without copying its source graph or large record.
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Source(Box::new(diagnostic))
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
/// Recursive syntax admission uses a compiler-owned stack. Returned models and
/// diagnostics contain owned source coordinates, never thread-local token spans.
pub fn parse_module(relative_path: &Path, source: &str) -> Result<Module, ParseError> {
    let name = ModuleName::from_relative_source(relative_path)?;
    crate::syntax::with_syntax_stack(|| assemble_module(name, [ModuleSource { path: relative_path, text: source }])).map_err(ParseError::Worker)?
}

/// Parses and merges every source part belonging to one logical IDL module.
///
/// `relative_directory` alone defines module identity. Part filenames and
/// caller enumeration order are presentation details and never enter the ABI
/// fingerprint. Each part must be a `.ncidl` file below that directory.
/// All parts share one syntax worker and return the same owned model boundary
/// as [`parse_module`]. Worker startup failure is [`ParseError::Worker`].
pub fn parse_module_parts(relative_directory: &Path, parts: &[ModuleSource<'_>]) -> Result<Module, ParseError> {
    let name = ModuleName::from_relative_directory(relative_directory)?;
    for part in parts {
        let normalized = !part.path.is_absolute()
            && part
                .path
                .components()
                .all(|component| matches!(component, Component::Normal(value) if value.to_str().is_some()));
        if !normalized || part.path.extension().and_then(|extension| extension.to_str()) != Some("ncidl") || !part.path.starts_with(relative_directory) {
            return Err(ParseError::from(Diagnostic::new(
                "EIDL002",
                format!(
                    "IDL part {} must be a .ncidl file below module directory {}",
                    part.path.display(),
                    relative_directory.display()
                ),
            )));
        }
    }
    crate::syntax::with_syntax_stack(|| assemble_module(name, parts.iter().copied())).map_err(ParseError::Worker)?
}

/// A lowered item and its name's parser-local span during duplicate admission.
/// Only `value` enters the stable module model; `span` never leaves this worker.
struct Parsed<T> {
    /// Language-neutral item with stable, owned source coordinates.
    value: T,
    /// Original identifier span used to compose an immediate Syn diagnostic.
    span: Span,
}

/// Parses source parts in portable path order and builds one canonical module.
fn assemble_module<'a>(name: ModuleName, parts: impl IntoIterator<Item = ModuleSource<'a>>) -> Result<Module, ParseError> {
    let mut parts = parts.into_iter().collect::<Vec<_>>();
    parts.sort_by_key(|part| part.path.to_string_lossy().replace('\\', "/"));

    let mut source_paths = HashSet::new();
    let mut sources = Vec::with_capacity(parts.len());
    let mut warnings = Vec::new();
    let mut externals = BTreeMap::<String, ExternalModule>::new();
    let mut imports = Vec::new();
    let mut declarations = BTreeMap::<String, Declaration>::new();
    for part in parts {
        let source_file = SourceFile::new(&name, part.path, part.text);
        if !source_paths.insert(source_file.path().to_owned()) {
            return Err(ParseError::from(
                Diagnostic::new("EIDL003", format!("IDL module contains source path {:?} more than once", source_file.path())).with_source(&source_file),
            ));
        }
        // Fallback tokenization and cleanup are iterative. Admit groups before Syn's
        // recursively constructed TokenBuffer, including unsupported syntax.
        let tokens = part
            .text
            .parse::<proc_macro2::TokenStream>()
            .map_err(|error| ParseError::from(Diagnostic::from_syn(&source_file, syn::Error::new(error.span(), error))))?;
        crate::nesting::admit_tokens(&tokens).map_err(|error| ParseError::from(Diagnostic::from_syn(&source_file, error)))?;
        let mut syntax = syn::parse2::<ModuleSyntax>(tokens).map_err(|error| ParseError::from(Diagnostic::from_syn(&source_file, error)))?;
        syntax.attach_source(&source_file);
        warnings.extend(syntax.warnings);
        // A dependency is a module-level fact, so repeating it in another
        // part is an error rather than an instruction to merge two edges.
        for external in syntax.externals {
            if let Some(previous) = externals.get(&external.module) {
                return Err(ParseError::from(
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
                return Err(ParseError::from(
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
        return Err(ParseError::from(
            Diagnostic::new("EIDL004", "an IDL module must declare at least one ABI item").with_sources(&sources),
        ));
    }

    warnings.sort_by(Diagnostic::stable_cmp);
    Ok(Module {
        name,
        sources,
        warnings,
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
    /// Nonfatal attribute diagnostics before source paths are attached.
    warnings: Vec<Diagnostic>,
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
        for warning in &mut self.warnings {
            warning.attach_source(source);
        }
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
        let mut warnings = Vec::new();
        let mut declaration_names = HashMap::new();

        while !input.is_empty() {
            let attributes = attributes::outer(input)?;
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

            let context = if input.peek(Token![struct]) {
                AttributeContext::Plain
            } else if input.peek(Token![enum]) {
                AttributeContext::Enum
            } else if input.peek(keyword::bitenum) {
                AttributeContext::BitEnum
            } else if input.peek(keyword::interface) {
                AttributeContext::Interface
            } else {
                return Err(input.error("expected `extern`, `use`, `struct`, `enum`, `bitenum`, or `interface`"));
            };
            let mut metadata = parse_attributes(attributes, context)?;
            warnings.append(&mut metadata.warnings);
            let Parsed { value: declaration, span } = if input.peek(Token![struct]) {
                parse_struct(metadata, input)?
            } else if input.peek(Token![enum]) {
                parse_enum(metadata, input)?
            } else if input.peek(keyword::bitenum) {
                parse_bit_enum(metadata, input)?
            } else {
                parse_interface(metadata, input)?
            };

            if let Some(previous) = declaration_names.insert(declaration.name.clone(), span) {
                return Err(duplicate_error("EIDL010", "declaration", &declaration.name, span, previous));
            }
            declarations.push(declaration);
        }

        Ok(Self {
            warnings,
            externals,
            imports,
            declarations,
        })
    }
}

/// Parses one canonical dotted external module dependency.
fn parse_external(input: ParseStream<'_>) -> syn::Result<ExternalModule> {
    let extern_token = input.parse::<Token![extern]>()?;
    let module = parse_module_reference(input)?;
    input.parse::<Token![;]>()?;
    Ok(ExternalModule {
        span: SourceSpan::from_parser(extern_token.span),
        module,
    })
}

/// Read a dotted logical reference, keeping token spans at the shared admission.
/// Tokenization admits keywords so this language's rule owns their rejection.
fn parse_module_reference(input: ParseStream<'_>) -> syn::Result<String> {
    // Dots belong to the IDL module-name grammar; Rust `::` remains reserved
    // for paths to declarations inside an already-declared dependency.
    let mut module = Vec::new();
    loop {
        let identifier = input.call(Ident::parse_any)?;
        let segment = identifier.to_string();
        ModuleName::validate_segment(&segment).map_err(|error| syn::Error::new(identifier.span(), error))?;
        module.push(segment);
        if !input.peek(Token![.]) {
            break;
        }
        input.parse::<Token![.]>()?;
    }
    if module.len() < 2 {
        // Requiring a dotted absolute name prevents hidden parent-namespace
        // lookup from changing meaning when the importing module moves.
        return Err(input.error("IDL module references require a fully qualified dotted module name"));
    }
    Ok(module.join("."))
}

/// Parses one fixed-width bit set with explicit named flag patterns.
fn parse_bit_enum(metadata: Attributes, input: ParseStream<'_>) -> syn::Result<Parsed<Declaration>> {
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
        let Parsed { value: flag, span } = parse_bit_flag(&body)?;
        if let Some(previous) = names.insert(flag.name.clone(), span) {
            return Err(duplicate_error("EIDL014", "bit enum flag", &flag.name, span, previous));
        }
        flags.push(flag);
    }
    if flags.is_empty() {
        return Err(syn::Error::new(name.span(), "IDL bit enums must contain at least one flag"));
    }
    Ok(declaration(name, Vec::new(), metadata, DeclarationKind::BitEnum(BitEnum { repr, flags })))
}

/// Parses one copied record declaration.
fn parse_struct(metadata: Attributes, input: ParseStream<'_>) -> syn::Result<Parsed<Declaration>> {
    input.parse::<Token![struct]>()?;
    let name = input.parse::<Ident>()?;
    require_upper_camel_case(&name, "struct")?;
    let parameters = parse_type_parameters(input)?;
    let body;
    braced!(body in input);
    let mut fields = Vec::new();
    let mut names = HashMap::new();
    while !body.is_empty() {
        let Parsed { value: field, span } = parse_named_field(&body)?;
        if let Some(previous) = names.insert(field.name.clone(), span) {
            return Err(duplicate_error("EIDL013", "field", &field.name, span, previous));
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
fn parse_enum(metadata: Attributes, input: ParseStream<'_>) -> syn::Result<Parsed<Declaration>> {
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
        let Parsed { value: variant, span } = parse_enum_variant(&body)?;
        if let Some(previous) = names.insert(variant.name.clone(), span) {
            return Err(duplicate_error("EIDL015", "enum variant", &variant.name, span, previous));
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
fn parse_interface(metadata: Attributes, input: ParseStream<'_>) -> syn::Result<Parsed<Declaration>> {
    let threading = metadata_threading(&metadata);
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
        DeclarationKind::Interface(Interface { bases, methods, threading }),
    ))
}

/// Combines common metadata with one declaration-specific payload.
fn declaration(name: Ident, parameters: Vec<String>, metadata: Attributes, kind: DeclarationKind) -> Parsed<Declaration> {
    Parsed {
        span: name.span(),
        value: Declaration {
            span: SourceSpan::from_parser(name.span()),
            docs: metadata.docs,
            name: name.to_string(),
            explicit_id: metadata.explicit_id,
            parameters,
            specialization: None,
            kind,
        },
    }
}

/// Parses the optional `: Interface + Interface` suffix used by callable types.
fn parse_bases(input: ParseStream<'_>) -> syn::Result<Vec<NamedType>> {
    if !input.peek(Token![:]) {
        return Ok(Vec::new());
    }
    input.parse::<Token![:]>()?;
    let mut bases = vec![types::base(input)?];
    while input.peek(Token![+]) {
        input.parse::<Token![+]>()?;
        bases.push(types::base(input)?);
    }
    Ok(bases)
}

/// Parses methods without accepting general Rust bodies, patterns or signatures.
fn parse_methods(input: ParseStream<'_>) -> syn::Result<Vec<Method>> {
    let body;
    braced!(body in input);
    let mut methods = Vec::new();
    let mut names = HashMap::new();
    while !body.is_empty() {
        let attributes = attributes::outer(&body)?;
        let (method, span) = methods::method(&body, &attributes)?;
        // Duplicate diagnostics use the original method identifier, not its body.
        if let Some(previous) = names.insert(method.name.clone(), span) {
            return Err(duplicate_error("EIDL016", "method or port", &method.name, span, previous));
        }
        methods.push(method);
    }
    Ok(methods)
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
fn parse_named_field(input: ParseStream<'_>) -> syn::Result<Parsed<Field>> {
    let attributes = attributes::outer(input)?;
    let docs = parse_doc_only_attributes(&attributes)?;
    if input.peek(Token![pub]) {
        return Err(input.error("IDL fields are public implicitly; remove `pub`"));
    }
    let name = input.parse::<Ident>()?;
    require_snake_case(&name, "field")?;
    input.parse::<Token![:]>()?;
    let ty = types::abi(input)?;
    if !input.is_empty() {
        input.parse::<Token![,]>()?;
    }
    Ok(Parsed {
        span: name.span(),
        value: Field {
            span: SourceSpan::from_parser(name.span()),
            docs,
            name: name.to_string(),
            ty,
        },
    })
}

/// Parses one enum variant including an optional explicit discriminant.
fn parse_enum_variant(input: ParseStream<'_>) -> syn::Result<Parsed<EnumVariant>> {
    let attributes = attributes::outer(input)?;
    let docs = parse_doc_only_attributes(&attributes)?;
    let name = input.parse::<Ident>()?;
    require_upper_camel_case(&name, "enum variant")?;
    let fields = if input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);
        let mut values = Vec::new();
        while !content.is_empty() {
            values.push(types::abi(&content)?);
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }
        if values.is_empty() {
            return Err(content.error("an empty tuple variant should be written without parentheses"));
        }
        VariantFields::Tuple(values)
    } else if input.peek(syn::token::Brace) {
        let content;
        braced!(content in input);
        let mut values = Vec::new();
        let mut names = HashMap::new();
        while !content.is_empty() {
            let Parsed { value: field, span } = parse_named_field(&content)?;
            if let Some(previous) = names.insert(field.name.clone(), span) {
                return Err(duplicate_error("EIDL013", "variant field", &field.name, span, previous));
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
        Some(attributes::signed(input)?)
    } else {
        None
    };
    if !input.is_empty() {
        input.parse::<Token![,]>()?;
    }
    Ok(Parsed {
        span: name.span(),
        value: EnumVariant {
            span: SourceSpan::from_parser(name.span()),
            docs,
            name: name.to_string(),
            discriminant,
            fields,
        },
    })
}

/// Parses one explicitly valued bit flag and consumes its trailing comma.
fn parse_bit_flag(input: ParseStream<'_>) -> syn::Result<Parsed<BitFlag>> {
    let attributes = attributes::outer(input)?;
    let docs = parse_doc_only_attributes(&attributes)?;
    let name = input.parse::<Ident>()?;
    require_screaming_snake_case(&name, "bit enum flag")?;
    input.parse::<Token![=]>()?;
    let value = attributes::unsigned(input)?;
    if !input.is_empty() {
        input.parse::<Token![,]>()?;
    }
    Ok(Parsed {
        span: name.span(),
        value: BitFlag {
            span: SourceSpan::from_parser(name.span()),
            docs,
            name: name.to_string(),
            value,
        },
    })
}

/// Parses one dotted module import with one name or a braced name list.
fn parse_import(input: ParseStream<'_>) -> syn::Result<Vec<Import>> {
    input.parse::<Token![use]>()?;
    let module = parse_module_reference(input)?;
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
        if let Some(previous) = unique_names.insert(normalized.clone(), name.span()) {
            return Err(duplicate_error("EIDL018", "import name", &normalized, name.span(), previous));
        }
        normalized_names.push(Import {
            span,
            module: module.clone(),
            name: normalized,
        });
    }
    Ok(normalized_names)
}

/// Produces one syntax error with both duplicate and original declarations.
fn duplicate_error(code: &str, role: &str, name: &str, current: Span, previous: Span) -> syn::Error {
    let mut error = syn::Error::new(current, format!("[{code}] duplicate {role} {name:?}"));
    error.combine(syn::Error::new(previous, "first declared here"));
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

/// Attaches one ordinary callable signature and every nested input span.
fn attach_method_source(method: &mut Method, source: &SourceFile) {
    method.span.attach(source);
    for parameter in &mut method.parameters {
        parameter.span.attach(source);
        attach_type_source(&mut parameter.ty, source);
    }
    attach_type_source(&mut method.output, source);
}

/// Attaches the authored source to a named reference and every nested argument.
/// Direct base substitution can move an argument into the reference slot;
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

/// Admits unchanged Rust-2024 member names without imposing metadata path bounds.
fn require_snake_case(identifier: &Ident, role: &str) -> syn::Result<()> {
    let value = identifier.to_string();
    let bytes = value.as_bytes();
    let valid = bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes.last().is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes.iter().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
        && !bytes.windows(2).any(|pair| pair == b"__");
    if !valid {
        return Err(syn::Error::new_spanned(identifier, format!("IDL {role} name {value:?} must use snake_case")));
    }
    // Casing already proves one nonraw ASCII identifier. Share only the keyword
    // rejection: metadata's length bound does not constrain member names, and
    // every reserved word is shorter than that bound.
    if matches!(
        neocogi_com_core::validate_rust_path(&value),
        Err(neocogi_com_core::NcRustPathError::ReservedIdentifier { .. })
    ) {
        Err(syn::Error::new_spanned(
            identifier,
            format!("IDL {role} name {value:?} is a reserved Rust 2024 keyword"),
        ))
    } else {
        Ok(())
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
    /// Accepted deprecated syntax, drained before constructing semantic declarations.
    warnings: Vec<Diagnostic>,
    /// Documentation lines retained for generated APIs and tools.
    docs: Vec<String>,
    /// Optional UUID override; ordinary declarations need no identity attribute.
    explicit_id: Option<NcGuid>,
    /// Interface threading capability.
    threading: Option<Threading>,
    /// Typed construction policy selected independently of live transport.
    persistence: PersistencePolicy,
    /// Whether an enum is a zero-based contiguous unit-value domain.
    dense: bool,
    /// Optional canonical naming policy for dense enum values.
    enum_names: Option<EnumNames>,
}

/// Parses common attributes and rejects ambiguous or inapplicable combinations.
fn parse_attributes(attributes: Vec<Attribute>, context: AttributeContext) -> syn::Result<Attributes> {
    let mut warnings = Vec::new();
    let mut docs = Vec::new();
    let mut id = None;
    let mut threading = None;
    let mut persistence = PersistencePolicy::Automatic;
    let mut dense = false;
    let mut enum_names = None;

    for attribute in attributes {
        if attribute.path().is_ident("doc") {
            docs.push(parse_doc_attribute(&attribute)?);
        } else if attribute.path().is_ident("version") {
            // Parse the former integer grammar without storing or interpreting it.
            // Even zero and large literals are inert; identity uses names alone.
            attribute.parse_args::<LitInt>()?;
            warnings.push(
                Diagnostic::warning("WIDL001", "declaration versions are deprecated and ignored")
                    .primary(&SourceSpan::from_parser(attribute.span()), "remove this version attribute")
                    .note("use distinct named types for different semantic contracts"),
            );
        } else if attribute.path().is_ident("id") {
            if id.is_some() {
                return Err(syn::Error::new_spanned(attribute, "duplicate explicit ID"));
            }
            let uuid = match attribute.parse_args::<IdArguments>()? {
                IdArguments::Current(uuid) => uuid,
                IdArguments::Deprecated(uuid) => {
                    warnings.push(
                        Diagnostic::warning("WIDL001", "the version argument of `id` is deprecated and ignored")
                            .primary(&SourceSpan::from_parser(attribute.span()), "keep only the UUID argument")
                            .note("write #[id(\"UUID\")]"),
                    );
                    uuid
                }
            };
            id = Some(NcGuid::from_uuid_bytes(parse_uuid(&uuid)?));
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

    Ok(Attributes {
        warnings,
        docs,
        explicit_id: id,
        threading,
        persistence,
        dense,
        enum_names,
    })
}

/// Explicit identity spelling; a former integer argument has no semantic storage.
enum IdArguments {
    /// Current UUID-only syntax.
    Current(LitStr),
    /// Accepted numbered syntax requiring a migration warning.
    Deprecated(LitStr),
}

impl Parse for IdArguments {
    /// Consumes a deprecated integer when present, retaining only its UUID.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let deprecated = input.peek(LitInt);
        if deprecated {
            input.parse::<LitInt>()?;
            input.parse::<Token![,]>()?;
        }
        let uuid = input.parse::<LitStr>()?;
        if !input.is_empty() {
            return Err(input.error("`id` accepts one UUID string"));
        }
        Ok(if deprecated { Self::Deprecated(uuid) } else { Self::Current(uuid) })
    }
}

/// Parses one mutually exclusive construction policy without ambiguous boolean states.
fn parse_persistence(attribute: &Attribute) -> syn::Result<PersistencePolicy> {
    let policy = attribute.parse_args::<Ident>()?;
    match policy.to_string().as_str() {
        "default" => Ok(PersistencePolicy::Default),
        _ => Err(syn::Error::new_spanned(
            policy,
            "expected default; field names follow declarations and ordinary value persistence is automatic",
        )),
    }
}

/// Returns a callable object's declared capability or the conservative local default.
fn metadata_threading(metadata: &Attributes) -> Threading {
    metadata.threading.unwrap_or(Threading::Local)
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

/// Rejects annotations at syntax positions whose compact model has no such meaning.
///
/// Receivers, parameters and admitted integer expressions support no attributes,
/// including docs. Check before narrowing the syntax so cfg/unknown meaning can
/// never silently disappear. Supported method/field docs use their own admission.
fn reject_attributes(attributes: &[Attribute]) -> syn::Result<()> {
    if let Some(attribute) = attributes.first() {
        return Err(syn::Error::new_spanned(attribute, "attributes are not supported in this IDL position"));
    }
    Ok(())
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
