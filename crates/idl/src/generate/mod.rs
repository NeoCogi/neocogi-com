//! Deterministic Rust binding generation from a resolved IDL graph.

use std::fmt;

use crate::{
    AbiType, Declaration, DeclarationKind, EnumNames, EnumRepr, Method, NamedType, Receiver, ResolvedDeclaration, ResolvedModule, ResolvedModules, Threading,
    VariantFields,
};
use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use neocogi_com_core::NcComInterfaceCompanion;

mod events;
mod lower;
mod mapping;
mod paths;
mod value;
pub use paths::RustModulePaths;

/// Failure to select a module or lower one validated IDL construct to Rust.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerateError {
    /// Concrete unsupported or internally inconsistent generation condition.
    message: String,
}

impl GenerateError {
    /// Creates one generator diagnostic without exposing mutable error state.
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for GenerateError {
    /// Writes the generator's actionable diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GenerateError {}

/// Generates complete Rust bindings for one module in a resolved graph.
///
/// The output contains ABI layouts, typed calls, description marker constants,
/// and already-computed declaration signatures as fixed byte literals. Runtime
/// module descriptions are unnecessary: exact signatures travel with interface
/// identities, and implementations remain private to their owning component.
pub fn generate_rust(graph: &ResolvedModules, module_name: &str) -> Result<String, GenerateError> {
    generate_rust_with_paths(graph, module_name, &RustModulePaths::default())
}

/// Generates bindings using explicit Rust locations independently of ABI identity.
///
/// Paths only affect emitted Rust imports. The already-resolved module graph and
/// all declaration signatures remain unchanged. Unmapped modules retain the
/// conventional `<dotted_module_with_underscores>_contracts` crate location.
pub fn generate_rust_with_paths(graph: &ResolvedModules, module_name: &str, paths: &RustModulePaths) -> Result<String, GenerateError> {
    generate_rust_with_paths_and_source_map(graph, module_name, paths).map(|generated| generated.source)
}

/// Generates Rust and exact formatted-source ranges with conventional module paths.
pub fn generate_rust_with_source_map(graph: &ResolvedModules, module_name: &str) -> Result<crate::GeneratedRust, GenerateError> {
    generate_rust_with_paths_and_source_map(graph, module_name, &RustModulePaths::default())
}

/// Generates mapped Rust with explicit backend paths; provenance never changes ABI identity.
pub fn generate_rust_with_paths_and_source_map(
    graph: &ResolvedModules,
    module_name: &str,
    paths: &RustModulePaths,
) -> Result<crate::GeneratedRust, GenerateError> {
    for name in paths.modules.keys().chain(paths.dependencies.keys()) {
        if graph.module(name).is_none() {
            return Err(GenerateError::new(format!("Rust location names unknown IDL module {name:?}")));
        }
    }
    generate_module(&Generation { graph, paths }, module_name)
}

/// Immutable resolved graph and Rust locations shared by every declaration backend.
struct Generation<'a> {
    /// Canonical semantic ownership, identity, and type resolution.
    graph: &'a ResolvedModules,
    /// Backend-only Rust locations; never part of an ABI fingerprint.
    paths: &'a RustModulePaths,
}

/// Generated allocation-free interface contracts and their facade-owned providers.
/// This explicit bootstrap keeps core independent of its own compiler/backend.
pub struct BootstrapBindings {
    /// Module constants, provider traits, raw ABI, clients and physical metadata.
    /// Core can resolve the canonical facade path to itself when checking in this half.
    pub abi: String,
    /// Concrete-object thunks and provider projection support, owned by the facade.
    pub provider: String,
}

/// Emits built-in interface-only bootstrap halves from the ordinary typed backend.
/// Authoring design metadata is facade-only and is deliberately omitted. Values
/// and open templates are outside this allocation-free core bootstrap contract.
pub fn generate_bootstrap(graph: &ResolvedModules, module_name: &str) -> Result<BootstrapBindings, GenerateError> {
    let paths = RustModulePaths::default();
    let generation = Generation { graph, paths: &paths };
    let module = graph.module(module_name).ok_or_else(|| GenerateError::new("bootstrap module is absent"))?;
    if !module.source.templates.is_empty() {
        return Err(GenerateError::new("bootstrap cannot contain open templates"));
    }
    let mut abi = module_header(&generation, module)?;
    let mut provider = String::new();
    for declaration in &module.source.declarations {
        let parts = interface_parts(&generation, module, declaration)?;
        abi.push_str(&format_rust(parts.abi)?);
        provider.push_str(&format_rust(parts.provider)?);
    }
    Ok(BootstrapBindings { abi, provider })
}

/// Emits one module after validating backend mappings against its semantic graph.
/// Generated support paths are absolute: local IDL declarations such as Option,
/// Some and None must remain application names, never capture Rust scaffolding.
fn generate_module(graph: &Generation<'_>, module_name: &str) -> Result<crate::GeneratedRust, GenerateError> {
    let module = graph
        .graph
        .module(module_name)
        .ok_or_else(|| GenerateError::new(format!("resolved graph has no module {module_name:?}")))?;
    let declarations = module
        .source
        .declarations
        .iter()
        .enumerate()
        .map(|(index, declaration)| generate_declaration(graph, module, declaration, index))
        .collect::<Result<Vec<_>, _>>()?;
    let mut source = module_header(graph, module)?;
    let mut source_map = mapping::new_map(module);
    mapping::map_header(&source, module, &mut source_map)?;
    let mut line_offset = source.lines().count();
    for (declaration, origin) in declarations.into_iter().zip(&module.source.declarations) {
        // Keep declarations visibly separated so generated files remain useful
        // during diagnostics, reviews, and editor navigation.
        source.push('\n');
        line_offset += 1;
        mapping::map_declaration(&declaration, line_offset, origin, &mut source_map)?;
        line_offset += declaration.lines().count();
        source.push_str(&declaration);
    }
    source.push_str(&generate_design_metadata(graph, module)?);
    Ok(crate::GeneratedRust { source, source_map })
}

/// Formats the contract header once for ordinary emission and core bootstrap.
fn module_header(graph: &Generation<'_>, module: &ResolvedModule) -> Result<String, GenerateError> {
    let imports = generate_imports(graph, module)?;
    let dependency_exports = graph.paths.dependency_exports(graph.graph)?;
    let value_alloc = module
        .source
        .declarations
        .iter()
        .filter_map(|declaration| {
            graph
                .graph
                .values
                .definitions
                .get(&module.declaration(&declaration.name).expect("resolved declaration").qualified_name)
        })
        .flat_map(|definition| &definition.fields)
        .any(needs_value_allocation)
        .then(|| quote! { extern crate alloc; });
    let module_id = guid_literal(module.module_id);
    let fingerprint = module.abi_fingerprint.iter().copied();
    let module_name = Literal::string(&module.source.name.to_string());
    let tokens = quote! {
        // This file is generated by neocogi-idl. Edit the .ncidl source.
        #value_alloc
        #imports
        #dependency_exports

        /// Stable logical identity of this generated contract module.
        pub const MODULE_ID: ::neocogi_com::NcGuid = #module_id;

        /// Transitive canonical ABI fingerprint of this generated module.
        pub const ABI_FINGERPRINT: [u8; 32] = [#(#fingerprint),*];

        /// Exact dotted module name derived from the normalized IDL path.
        pub const MODULE_NAME: &str = #module_name;


    };
    let mut source = String::from("// This file is generated by neocogi-idl. Edit the .ncidl source.\n\n");
    source.push_str(&format_rust(tokens)?);
    Ok(source)
}

/// Emits descriptions from the same resolved graph that generated the COM ABI.
/// Widget authoring reads these through its registered concrete interface type;
/// documents never need to repeat Rust paths or event message declarations.
fn generate_design_metadata(graph: &Generation<'_>, module: &ResolvedModule) -> Result<String, GenerateError> {
    let value_member = rust_ident(crate::metadata_names::VALUE_METADATA_MEMBER)?;
    let interface_member = rust_ident(crate::metadata_names::INTERFACE_METADATA_MEMBER)?;
    let mut descriptions = Vec::new();
    for declaration in &module.source.declarations {
        let name = rust_ident(&declaration.name)?;
        if !matches!(declaration.kind, DeclarationKind::Interface(_)) {
            let descriptor = design_definition(graph, module, declaration)?;
            descriptions.push(quote! {
                impl #name {
                    /// Source-level declaration shape and public Rust payload name, generated by IDL.
                    pub const #value_member: ::neocogi_com::design::IdlType = #descriptor;
                }
            });
        }
        if let DeclarationKind::Interface(_) = &declaration.kind {
            let abi_name = format!("{}{}", declaration.name, NcComInterfaceCompanion::Abi.suffix());
            let abi = rust_ident(&abi_name)?;
            let rust_abi = Literal::string(&public_rust_path(graph, module, &abi_name)?);
            let events = events::descriptions(graph, module, declaration)?;
            let values = interface_value_metadata(graph, module, declaration)?;
            descriptions.push(quote! {
                impl #abi {
                    /// COM product identity, constructor receiver and complete reachable value graph.
                    pub const #interface_member: ::neocogi_com::design::IdlInterface = ::neocogi_com::design::IdlInterface {
                        abi: &Self::INTERFACE_DESCRIPTOR,
                        chain: &Self::INTERFACE_CHAIN,
                        rust_abi: #rust_abi,
                        events: &[#(#events),*],
                        values: &[#(#values),*],
                    };
                }
            });
        }
    }
    format_rust(quote! { #(#descriptions)* })
}

/// Materializes a resolved definition as data, without requiring a Rust dependency
/// on the package of a transitively referenced value.
fn design_definition(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<TokenStream, GenerateError> {
    let identity = module.declaration(&declaration.name).expect("resolved declaration");
    let qualified_name = Literal::string(&identity.qualified_name);
    let rust_type = Literal::string(&public_rust_path(graph, module, &declaration.name)?);
    let shape = match &declaration.kind {
        DeclarationKind::Struct(record) => {
            let fields = design_fields(graph, module, &record.fields)?;
            quote! { ::neocogi_com::design::IdlShape::Struct(&[#(#fields),*]) }
        }
        DeclarationKind::Enum(enumeration) => {
            let mut variants = Vec::new();
            let mut previous = -1;
            for variant in &enumeration.variants {
                let tag = variant.discriminant.unwrap_or(previous + 1);
                previous = tag;
                let variant_name = Literal::string(&variant.name);
                let fields = match &variant.fields {
                    VariantFields::Unit => Vec::new(),
                    VariantFields::Named(fields) => design_fields(graph, module, fields)?,
                    VariantFields::Tuple(types) => types
                        .iter()
                        .enumerate()
                        .map(|(index, ty)| {
                            let field = Literal::string(&index.to_string());
                            let ty = design_field_type(graph, module, ty)?;
                            Ok(quote! { ::neocogi_com::design::IdlField { name: #field, ty: #ty } })
                        })
                        .collect::<Result<Vec<_>, GenerateError>>()?,
                };
                variants.push(quote! { ::neocogi_com::design::IdlVariant { name: #variant_name, tag: #tag, fields: &[#(#fields),*] } });
            }
            quote! { ::neocogi_com::design::IdlShape::Enum(&[#(#variants),*]) }
        }
        DeclarationKind::BitEnum(bit_enum) => {
            let flags = bit_enum
                .flags
                .iter()
                .map(|flag| {
                    let name = Literal::string(&flag.name);
                    let value = flag.value;
                    quote! { ::neocogi_com::design::IdlFlag { name: #name, value: #value } }
                })
                .collect::<Vec<_>>();
            quote! { ::neocogi_com::design::IdlShape::BitEnum(&[#(#flags),*]) }
        }
        DeclarationKind::Interface(_) => unreachable!("interfaces are handled separately"),
    };
    Ok(quote! { ::neocogi_com::design::IdlType { qualified_name: #qualified_name, rust_type: #rust_type, shape: #shape } })
}

/// Keeps authoring metadata on the same public Rust import path as generated contracts.
fn public_rust_path(graph: &Generation<'_>, module: &ResolvedModule, name: &str) -> Result<String, GenerateError> {
    let path = graph.paths.path(&module.source.name.to_string())?;
    let mut parts = path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>();
    parts.push(name.to_owned());
    let result = parts.join("::");
    // The declaration suffix is part of the transported path and its byte limit.
    neocogi_com_core::validate_rust_path(&result).map_err(|error| GenerateError::new(format!("invalid public Rust path {result:?}: {error}")))?;
    Ok(result)
}

/// Emits the resolved design type directly from an authored field type.
fn design_field_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    match ty {
        AbiType::Unit => Ok(quote! { ::neocogi_com::design::IdlValueType::Unit }),
        AbiType::String => Ok(quote! { ::neocogi_com::design::IdlValueType::Text }),
        AbiType::Scalar(name) => {
            let name = Literal::string(name);
            Ok(quote! { ::neocogi_com::design::IdlValueType::Scalar(#name) })
        }
        AbiType::Named(name) if builtin_object_type(name).is_some() => {
            let name = Literal::string(&name.segments.join("."));
            Ok(quote! { ::neocogi_com::design::IdlValueType::Object(#name) })
        }
        AbiType::Named(name) => {
            let resolved = resolve(graph, module, name)?;
            let qualified = Literal::string(&resolved.identity.qualified_name);
            Ok(match resolved.declaration.kind {
                DeclarationKind::Interface(_) => quote! { ::neocogi_com::design::IdlValueType::Object(#qualified) },
                _ => quote! { ::neocogi_com::design::IdlValueType::Named(#qualified) },
            })
        }
        AbiType::WeakComPtr(value) | AbiType::WeakComBox(value) => {
            let AbiType::Named(name) = value.as_ref() else {
                return Err(GenerateError::new("weak target is not a contract"));
            };
            let qualified = match builtin_object_type(name) {
                Some(_) => name.segments.join("."),
                None => resolve(graph, module, name)?.identity.qualified_name.clone(),
            };
            Ok(match ty {
                AbiType::WeakComPtr(_) => quote! { ::neocogi_com::design::IdlValueType::WeakObject(#qualified) },
                _ => quote! { ::neocogi_com::design::IdlValueType::WeakBoxObject(#qualified) },
            })
        }
        AbiType::ComPtr(value) | AbiType::ComBox(value) => design_field_type(graph, module, value),
        AbiType::Vector(value) => {
            let value = design_field_type(graph, module, value)?;
            Ok(quote! { ::neocogi_com::design::IdlValueType::Sequence(&#value) })
        }
        AbiType::Array { element, len } => {
            let element = design_field_type(graph, module, element)?;
            let len = Literal::u32_unsuffixed(*len);
            Ok(quote! { ::neocogi_com::design::IdlValueType::Array(&#element, #len) })
        }
        _ => Ok(quote! { ::neocogi_com::design::IdlValueType::Unit }),
    }
}

/// Pairs authored field names with their canonical resolved structural types.
fn design_fields(graph: &Generation<'_>, module: &ResolvedModule, fields: &[crate::Field]) -> Result<Vec<TokenStream>, GenerateError> {
    fields
        .iter()
        .map(|field| {
            let name = Literal::string(&field.name);
            let ty = design_field_type(graph, module, &field.ty)?;
            Ok(quote! { ::neocogi_com::design::IdlField { name: #name, ty: #ty } })
        })
        .collect()
}

/// Collects reachable definitions once, retaining named edges across recursive sequences.
fn interface_value_metadata<'a>(graph: &Generation<'a>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<TokenStream>, GenerateError> {
    use std::collections::{BTreeMap, BTreeSet};
    fn data<'a>(
        value: ResolvedDeclaration<'a>,
        graph: &Generation<'a>,
        preferred: &ResolvedModule,
        names: &mut BTreeMap<String, ResolvedDeclaration<'a>>,
    ) -> Result<(), GenerateError> {
        let value = graph.graph.local_projection(preferred, &value.identity.qualified_name).unwrap_or(value);
        if names.contains_key(&value.identity.qualified_name) {
            return Ok(());
        }
        names.insert(value.identity.qualified_name.clone(), value);
        // Follow the actual supplying declaration's bound fields. The global
        // canonical capability graph deliberately contains no Rust projection.
        for field in crate::resolve::value_fields(value.declaration) {
            root(field, graph, value.module, preferred, names)?;
        }
        Ok(())
    }
    fn root<'a>(
        ty: &AbiType,
        graph: &Generation<'a>,
        module: &ResolvedModule,
        preferred: &ResolvedModule,
        names: &mut BTreeMap<String, ResolvedDeclaration<'a>>,
    ) -> Result<(), GenerateError> {
        match ty {
            AbiType::Named(name) if builtin_object_type(name).is_none() && !name.is_str() => {
                let value = resolve(graph, module, name)?;
                if !matches!(value.declaration.kind, DeclarationKind::Interface(_)) {
                    data(value, graph, preferred, names)?;
                }
            }
            AbiType::ComPtr(value)
            | AbiType::WeakComPtr(value)
            | AbiType::ComBox(value)
            | AbiType::WeakComBox(value)
            | AbiType::Borrowed(value)
            | AbiType::Vector(value)
            | AbiType::UnsizedSlice(value)
            | AbiType::Slice(value)
            | AbiType::OutputSlice(value)
            | AbiType::Array { element: value, .. } => root(value, graph, module, preferred, names)?,
            _ => {}
        }
        Ok(())
    }
    fn interface<'a>(
        graph: &Generation<'a>,
        module: &ResolvedModule,
        declaration: &Declaration,
        preferred: &ResolvedModule,
        visited: &mut BTreeSet<String>,
        names: &mut BTreeMap<String, ResolvedDeclaration<'a>>,
    ) -> Result<(), GenerateError> {
        if !visited.insert(module.source.name.qualify(&declaration.name)) {
            return Ok(());
        }
        let DeclarationKind::Interface(value) = &declaration.kind else {
            return Ok(());
        };
        if let Some(specialization) = &declaration.specialization {
            for argument in &specialization.arguments {
                root(argument, graph, module, preferred, names)?;
            }
        }
        for method in &value.methods {
            for parameter in &method.parameters {
                root(&parameter.ty, graph, module, preferred, names)?;
            }
            root(&method.output, graph, module, preferred, names)?;
        }
        for base in value.bases.iter().chain(&value.events) {
            let base = resolve(graph, module, base)?;
            interface(graph, base.module, base.declaration, preferred, visited, names)?;
        }
        Ok(())
    }
    let mut names = BTreeMap::new();
    interface(graph, module, declaration, module, &mut BTreeSet::new(), &mut names)?;
    names.values().map(|value| design_definition(graph, value.module, value.declaration)).collect()
}

/// Detects explicit String/Vec spellings in this module's normalized field graph.
/// COM object methods and named imported fields do not themselves require alloc.
fn needs_value_allocation(value: &crate::ValueType) -> bool {
    // Named fields use the referenced declaration's public Rust type. Only
    // storage spelled inside this module requires its own alloc import.
    match value {
        crate::ValueType::Text | crate::ValueType::Sequence(_) => true,
        crate::ValueType::Array(value, _) => needs_value_allocation(value),
        _ => false,
    }
}

/// Generates authored imports through accessible typed dependency paths.
fn generate_imports(graph: &Generation<'_>, module: &ResolvedModule) -> Result<TokenStream, GenerateError> {
    let imports = module.source.imports.iter().map(|import| {
        let crate_ident = graph.paths.dependency_path(&import.module)?;
        let name = rust_ident(&import.name)?;
        let origin = origin_docs(&[], &import.span);
        Ok(quote! { #origin pub use #crate_ident::#name; })
    });
    let imports = imports.collect::<Result<Vec<_>, GenerateError>>()?;
    Ok(quote! { #(#imports)* })
}

/// Generates one public declaration from its resolved source-order identity.
fn generate_declaration(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration, source_index: usize) -> Result<String, GenerateError> {
    let identity = module
        .declarations
        .get(source_index)
        .ok_or_else(|| GenerateError::new("resolved declaration order does not match its source module"))?;
    match &declaration.kind {
        DeclarationKind::Struct(_) => value::generate_aggregate(graph, module, declaration, identity),
        DeclarationKind::Enum(enumeration) if enumeration.has_payload() => value::generate_aggregate(graph, module, declaration, identity),
        DeclarationKind::Enum(_) | DeclarationKind::BitEnum(_) => value::generate_scalar(graph, declaration, identity),
        DeclarationKind::Interface(_) => {
            let parts = interface_parts(graph, module, declaration)?;
            let abi = parts.abi;
            let provider = parts.provider;
            format_rust(quote! { #abi #provider })
        }
    }
}

/// Formats ordinary generated Rust syntax with stable, human-readable layout.
fn format_rust(tokens: TokenStream) -> Result<String, GenerateError> {
    // `TokenStream::to_string` is intended for diagnostics rather than source
    // generation: it collapses layout and exposes doc comments as `#[doc]`.
    // Parsing gives the source printer enough syntax to choose conventional
    // whitespace and documentation-comment notation.
    let syntax = syn::parse2::<syn::File>(tokens).map_err(|error| GenerateError::new(format!("generated invalid Rust syntax: {error}")))?;
    Ok(prettyplease::unparse(&syntax))
}

/// Emits both interface halves from one resolved ownership plan. The backend
/// does not parse provider Rust wrappers to recover the contract it just received.
fn interface_parts(
    graph: &Generation<'_>,
    module: &ResolvedModule,
    declaration: &Declaration,
) -> Result<neocogi_com_codegen::InterfaceExpansion, GenerateError> {
    use neocogi_com_codegen::{model::InterfacePlan, InterfaceThreading};
    let DeclarationKind::Interface(interface) = &declaration.kind else {
        return Err(GenerateError::new("interface emission requires a callable declaration"));
    };
    let identity = module
        .declaration(&declaration.name)
        .ok_or_else(|| GenerateError::new("bound interface lost its identity"))?;
    let methods = lower::methods(graph, module, &interface.methods)?;
    let name = rust_ident(&declaration.name)?;
    use syn::parse::Parser;
    let docs = syn::Attribute::parse_outer
        .parse2(declaration_docs(graph, declaration))
        .map_err(|error| GenerateError::new(error.to_string()))?;
    let bases = interface
        .bases
        .iter()
        .map(|base| {
            let bound = resolve(graph, module, base)?;
            let tokens = named_rust_type(graph, module, bound, false)?;
            syn::parse2(tokens).map_err(|error| GenerateError::new(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let plan = InterfacePlan {
        name,
        visibility: syn::parse_quote!(pub),
        docs,
        id: identity.id,
        signature: identity.abi_signature,
        qualified_name: identity.qualified_name.clone(),
        threading: match interface.threading {
            Threading::Local => InterfaceThreading::Local,
            Threading::Shared => InterfaceThreading::Shared,
        },
        bases,
        methods,
    };
    neocogi_com_codegen::expand_interface_parts(plan).map_err(|error| GenerateError::new(error.to_string()))
}

/// Converts one Rust-like variant identifier into a stable snake-case name.
fn snake_case(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut result = String::new();
    for (index, character) in characters.iter().copied().enumerate() {
        if character.is_uppercase() {
            let previous = index.checked_sub(1).and_then(|previous| characters.get(previous)).copied();
            let next = characters.get(index + 1).copied();
            if !result.is_empty()
                && !result.ends_with('_')
                && (previous.is_some_and(|value| value.is_lowercase() || value.is_numeric()) || next.is_some_and(char::is_lowercase))
            {
                result.push('_');
            }
            result.extend(character.to_lowercase());
        } else {
            result.push(character);
        }
    }
    result
}

/// Lowers one copied aggregate field or payload type.
fn value_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    match ty {
        AbiType::Scalar(name) => scalar(name),
        AbiType::Named(name) => {
            let resolved = resolve(graph, module, name)?;
            match resolved.declaration.kind {
                DeclarationKind::Struct(_) | DeclarationKind::BitEnum(_) | DeclarationKind::Enum(_) => named_rust_type(graph, module, resolved, false),
                _ => Err(GenerateError::new(format!("{} is not a copied value declaration", name.rust_path()))),
            }
        }
        AbiType::Array { element, len } => {
            let element = value_type(graph, module, element)?;
            let len = Literal::u32_unsuffixed(*len);
            Ok(quote! { [#element; #len] })
        }
        _ => Err(GenerateError::new("borrowed, string, and unit types cannot occupy copied aggregate storage")),
    }
}

/// Resolves a type through the semantic graph with an internal-invariant error.
fn resolve<'a>(graph: &Generation<'a>, module: &ResolvedModule, name: &NamedType) -> Result<ResolvedDeclaration<'a>, GenerateError> {
    graph
        .graph
        .resolve_declaration(name)
        .ok_or_else(|| GenerateError::new(format!("resolved module {} lost type mapping for {}", module.source.name, name.rust_path())))
}

/// Selects a native value type or an interface marker from local/imported bindings.
/// Only interfaces use the Abi suffix; copied values always use their native type.
fn named_rust_type(graph: &Generation<'_>, module: &ResolvedModule, resolved: ResolvedDeclaration<'_>, abi_suffix: bool) -> Result<TokenStream, GenerateError> {
    let mut name = resolved.declaration.name.clone();
    if abi_suffix && matches!(resolved.declaration.kind, DeclarationKind::Interface(_)) {
        name.push_str(NcComInterfaceCompanion::Abi.suffix());
    }
    let name = rust_ident(&name)?;
    if resolved.module.source.name == module.source.name {
        // Generation owns this module location; local references cannot become
        // private generic binders introduced by a subsequent macro expansion.
        Ok(quote! { self::#name })
    } else {
        let crate_ident = graph.paths.dependency_path(&resolved.module.source.name.to_string())?;
        Ok(quote! { #crate_ident::#name })
    }
}

/// Generates doc attributes while preserving authored text exactly.
fn docs(lines: &[String]) -> TokenStream {
    // Rust's canonical doc-comment spelling contains one space after `///`.
    // The leading space in the attribute value lets the syntax-aware printer
    // recover that spelling instead of producing a visually cramped comment.
    let lines = lines.iter().map(|line| Literal::string(&format!(" {line}")));
    quote! { #(#[doc = #lines])* }
}

/// Converts one validated IDL identifier into a Rust token identifier.
fn rust_ident(value: &str) -> Result<Ident, GenerateError> {
    syn::parse_str::<Ident>(value).map_err(|_| GenerateError::new(format!("IDL identifier {value:?} is not a Rust identifier")))
}

/// Emits a field-wise GUID expression independent of host byte order.
fn guid_literal(value: neocogi_com_core::NcGuid) -> TokenStream {
    let (data1, data2, data3, data4) = value.fields();
    quote! { ::neocogi_com::NcGuid::from_fields(#data1, #data2, #data3, [#(#data4),*]) }
}

/// Emits one complete SHA-256 digest as a fixed Rust byte array.
fn signature_literal(value: [u8; 32]) -> Literal {
    Literal::byte_string(&value)
}

/// Converts one enum representation into its Rust fixed-width integer type.
fn enum_repr(value: EnumRepr) -> TokenStream {
    match value {
        EnumRepr::U8 => quote! { u8 },
        EnumRepr::U16 => quote! { u16 },
        EnumRepr::U32 => quote! { u32 },
        EnumRepr::I8 => quote! { i8 },
        EnumRepr::I16 => quote! { i16 },
        EnumRepr::I32 => quote! { i32 },
    }
}

/// Converts one supported scalar spelling into a Rust type token.
fn scalar(value: &str) -> Result<TokenStream, GenerateError> {
    match value {
        "Guid" => Ok(quote! { ::neocogi_com::NcGuid }),
        "Status" => Ok(quote! { ::neocogi_com::NcStatus }),
        "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64" => {
            let ident = rust_ident(value)?;
            Ok(quote! { #ident })
        }
        _ => Err(GenerateError::new(format!("unsupported scalar type {value:?}"))),
    }
}

/// Maps the two explicit core-capability spellings to existing COM interface markers.
fn builtin_object_type(name: &NamedType) -> Option<TokenStream> {
    use crate::binding::{Binding, Intrinsic};
    match name.binding {
        Binding::Intrinsic(Intrinsic::Unknown) => Some(quote! { ::neocogi_com::IUnknown }),
        Binding::Intrinsic(Intrinsic::Bytes) => Some(quote! { ::neocogi_com::INcBytesAbi }),
        Binding::Intrinsic(Intrinsic::String) => Some(quote! { ::neocogi_com::INcStringAbi }),
        _ => None,
    }
}

/// Documents a concrete declaration and explains generated generic name mangling.
/// Cargo module-name encoding is reversible; specialization UUIDs are identities,
/// not an encoding of source text. Keep the closed spelling beside the Rust name.
fn declaration_docs(graph: &Generation<'_>, declaration: &Declaration) -> TokenStream {
    let origin = origin_docs(&declaration.docs, &declaration.span);
    let Some(instance) = &declaration.specialization else { return origin };
    let mut applied = instance.template.clone();
    applied.arguments = instance.arguments.clone();
    // This is a synthesized display view, not a rewritten semantic AST. Canonical
    // owner keys explain imported templates without losing authored source spans.
    let mut display = AbiType::Named(applied);
    qualify_display(graph, &mut display);
    let explanation = Literal::string(&format!(
        " Closed IDL type: `{}`. Rust name `{}` appends `Instance` and the closed type's nominal UUID as hexadecimal digits. \
         The UUID derives from the template and argument identities; it is not reversibly mangled source text. \
         Use this closed spelling to interpret the generated name; decoding the UUID cannot recover the type arguments.",
        display.source_name(),
        declaration.name,
    ));
    quote! { #origin #[doc = #explanation] }
}

/// Qualifies a disposable documentation view using established declaration keys.
/// It neither looks up source spelling nor changes canonical/supplying ownership.
fn qualify_display(graph: &Generation<'_>, ty: &mut AbiType) {
    match ty {
        AbiType::Named(name) => {
            if let Some(key) = name.binding.declaration() {
                let mut segments = graph.graph.modules()[key.module]
                    .source
                    .name
                    .to_string()
                    .split('.')
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                segments.push(key.name.clone());
                name.segments = segments;
            }
            for argument in &mut name.arguments {
                qualify_display(graph, argument);
            }
        }
        AbiType::ComPtr(value)
        | AbiType::WeakComPtr(value)
        | AbiType::ComBox(value)
        | AbiType::WeakComBox(value)
        | AbiType::Borrowed(value)
        | AbiType::Vector(value)
        | AbiType::UnsizedSlice(value)
        | AbiType::Slice(value)
        | AbiType::OutputSlice(value)
        | AbiType::Array { element: value, .. } => qualify_display(graph, value),
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
    }
}

/// Adds readable provenance after authored docs, so it survives Rust macro forwarding.
fn origin_docs(lines: &[String], span: &crate::SourceSpan) -> TokenStream {
    let documentation = docs(lines);
    let origin = Literal::string(&format!(" {}", mapping::origin_label(span)));
    quote! { #documentation #[doc = #origin] }
}

/// Emits explicit object ownership using the bound target's exact threading proof.
fn owned_object_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    let (target, wrapper) = match ty {
        AbiType::ComPtr(target) => (target, "ComPtr"),
        AbiType::WeakComPtr(target) => (target, "WeakComPtr"),
        AbiType::ComBox(target) => (target, "ComBox"),
        AbiType::WeakComBox(target) => (target, "WeakComBox"),
        _ => return Err(GenerateError::new("resolved type is not an object ownership wrapper")),
    };
    let (interface, mode) = lower::object(graph, module, target)?;
    let mode = mode.as_ref().map(|mode| quote! { , #mode });
    let wrapper = rust_ident(wrapper)?;
    Ok(quote! { ::neocogi_com::#wrapper<#interface #mode> })
}
