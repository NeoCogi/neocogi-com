//! Deterministic Rust binding generation from a resolved IDL graph.

use std::fmt;

use crate::{
    AbiType, Declaration, DeclarationKind, EnumNames, EnumRepr, InterfaceRole, Method, NamedType, Receiver, ResolvedDeclaration, ResolvedModule,
    ResolvedModules, Threading, VariantFields,
};
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

mod events;
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
    for name in paths.modules.keys() {
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

/// Emits one module after validating backend mappings against its semantic graph.
/// Generated support paths are absolute: local IDL declarations such as Option,
/// Some and None must remain application names, never capture Rust scaffolding.
fn generate_module(graph: &Generation<'_>, module_name: &str) -> Result<crate::GeneratedRust, GenerateError> {
    let module = graph
        .graph
        .module(module_name)
        .ok_or_else(|| GenerateError::new(format!("resolved graph has no module {module_name:?}")))?;
    let imports = generate_imports(graph, module)?;
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
    let declarations = module
        .source
        .declarations
        .iter()
        .enumerate()
        .map(|(index, declaration)| {
            generate_declaration(graph, module, declaration, index)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let module_id = guid_literal(module.module_id);
    let fingerprint = module.abi_fingerprint.iter().copied();
    let module_name = Literal::string(module_name);
    let tokens = quote! {
        // This file is generated by neocogi-idl. Edit the .ncidl source.
        #value_alloc
        #imports

        /// Stable logical identity of this generated contract module.
        pub const MODULE_ID: ::neocogi_com::NcGuid = #module_id;

        /// Transitive canonical ABI fingerprint of this generated module.
        pub const ABI_FINGERPRINT: [u8; 32] = [#(#fingerprint),*];

        /// Exact dotted module name derived from the normalized IDL path.
        pub const MODULE_NAME: &str = #module_name;


    };
    let mut source = String::from("// This file is generated by neocogi-idl. Edit the .ncidl source.\n\n");
    source.push_str(&format_rust(tokens)?);
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

/// Emits descriptions from the same resolved graph that generated the COM ABI.
/// Widget authoring reads these through its registered concrete interface type;
/// documents never need to repeat Rust paths or event message declarations.
fn generate_design_metadata(graph: &Generation<'_>, module: &ResolvedModule) -> Result<String, GenerateError> {
    let mut descriptions = Vec::new();
    for declaration in &module.source.declarations {
        let name = rust_ident(&declaration.name)?;
        if !matches!(declaration.kind, DeclarationKind::Interface(_)) {
            let descriptor = design_definition(graph, module, declaration)?;
            descriptions.push(quote! {
                impl #name {
                    /// Source-level declaration shape and public Rust payload name, generated by IDL.
                    pub const IDL_TYPE: ::neocogi_com::design::IdlType = #descriptor;
                }
            });
        }
        if let DeclarationKind::Interface(_) = &declaration.kind {
            let abi = rust_ident(&format!("{}Abi", declaration.name))?;
            let rust_abi = Literal::string(&public_rust_path(graph, module, &format!("{}Abi", declaration.name))?);
            let events = events::descriptions(graph, module, declaration)?;
            let values = interface_value_metadata(graph, module, declaration)?;
            descriptions.push(quote! {
                impl #abi {
                    /// COM product identity, constructor receiver and complete reachable value graph.
                    pub const IDL_INTERFACE: ::neocogi_com::design::IdlInterface = ::neocogi_com::design::IdlInterface {
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
            let AbiType::Named(name) = value.as_ref() else { return Err(GenerateError::new("weak target is not a contract")); };
            let qualified = match builtin_object_type(name) {
                Some(_) => name.segments.join("."),
                None => resolve(graph, module, name)?.identity.qualified_name.clone(),
            };
            Ok(match ty {
                AbiType::WeakComPtr(_) => quote! { ::neocogi_com::design::IdlValueType::WeakObject(#qualified) },
                _ => quote! { ::neocogi_com::design::IdlValueType::WeakBoxObject(#qualified) },
            })
        }
        AbiType::ComPtr(value) | AbiType::ComBox(value) => {
            design_field_type(graph, module, value)
        }
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
fn interface_value_metadata(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration) -> Result<Vec<TokenStream>, GenerateError> {
    use std::collections::BTreeSet;
    fn data(name: &str, graph: &Generation<'_>, names: &mut BTreeSet<String>) {
        if !names.insert(name.to_owned()) {
            return;
        }
        if let Some(definition) = graph.graph.values.definitions.get(name) {
            for field in &definition.fields {
                edge(field, graph, names);
            }
        }
    }
    fn edge(ty: &crate::ValueType, graph: &Generation<'_>, names: &mut BTreeSet<String>) {
        match ty {
            crate::ValueType::Named(name) => data(name, graph, names),
            crate::ValueType::Sequence(value) | crate::ValueType::Array(value, _) => edge(value, graph, names),
            _ => {}
        }
    }
    fn root(ty: &AbiType, graph: &Generation<'_>, module: &ResolvedModule, names: &mut BTreeSet<String>) -> Result<(), GenerateError> {
        match ty {
            AbiType::Named(name) if builtin_object_type(name).is_none() && name.segments.as_slice() != ["str"] => {
                let value = resolve(graph, module, name)?;
                if !matches!(value.declaration.kind, DeclarationKind::Interface(_)) {
                    data(&value.identity.qualified_name, graph, names);
                }
            }
            AbiType::ComPtr(value) | AbiType::WeakComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value)
            | AbiType::Borrowed(value)
            | AbiType::Vector(value)
            | AbiType::UnsizedSlice(value)
            | AbiType::Slice(value)
            | AbiType::OutputSlice(value)
            | AbiType::Array { element: value, .. } => root(value, graph, module, names)?,
            _ => {}
        }
        Ok(())
    }
    fn interface(
        graph: &Generation<'_>,
        module: &ResolvedModule,
        declaration: &Declaration,
        visited: &mut BTreeSet<String>,
        names: &mut BTreeSet<String>,
    ) -> Result<(), GenerateError> {
        if !visited.insert(module.source.name.qualify(&declaration.name)) {
            return Ok(());
        }
        let DeclarationKind::Interface(value) = &declaration.kind else {
            return Ok(());
        };
        if let Some(specialization) = &declaration.specialization {
            for argument in &specialization.arguments {
                root(argument, graph, module, names)?;
            }
        }
        for method in &value.methods {
            for parameter in &method.parameters {
                root(&parameter.ty, graph, module, names)?;
            }
            root(&method.output, graph, module, names)?;
        }
        for base in value.bases.iter().chain(&value.events) {
            let base = resolve(graph, module, base)?;
            interface(graph, base.module, base.declaration, visited, names)?;
        }
        Ok(())
    }
    let mut names = BTreeSet::new();
    interface(graph, module, declaration, &mut BTreeSet::new(), &mut names)?;
    names
        .iter()
        .map(|name| {
            let value = graph
                .graph
                .resolve_canonical(&module.source.name.to_string(), name)
                .expect("reachable definition exists");
            design_definition(graph, value.module, value.declaration)
        })
        .collect()
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

/// Generates explicit imports from conventionally named contract crates.
fn generate_imports(graph: &Generation<'_>, module: &ResolvedModule) -> Result<TokenStream, GenerateError> {
    let imports = module.source.imports.iter().map(|import| {
        let crate_ident = graph.paths.path(&import.module)?;
        let names = import.names.iter().map(|name| rust_ident(name)).collect::<Result<Vec<_>, _>>()?;
        let origin = origin_docs(&[], &import.span);
        Ok(quote! { #origin pub use #crate_ident::{#(#names),*}; })
    });
    let imports = imports.collect::<Result<Vec<_>, GenerateError>>()?;
    Ok(quote! { #(#imports)* })
}

/// Generates one public declaration from its resolved source-order identity.
fn generate_declaration(graph: &Generation<'_>, module: &ResolvedModule, declaration: &Declaration, source_index: usize) -> Result<String, GenerateError> {
    let declaration_docs = origin_docs(&declaration.docs, &declaration.span);
    let name = rust_ident(&declaration.name)?;
    let identity = module
        .declarations
        .get(source_index)
        .ok_or_else(|| GenerateError::new("resolved declaration order does not match its source module"))?;
    if graph.graph.values.definitions.get(&identity.qualified_name).is_some_and(|value| !value.copied) {
        return value::generate_owned(graph, module, declaration, identity);
    }
    // One contract module remains the compatibility unit, but the resolver
    // folds the declaration ID into each signature so typed metadata can still
    // distinguish two aggregates from the same exact module revision.
    let assigned_signature = signature_literal(identity.abi_signature);
    let id = Literal::string(&identity.id.to_string());
    let qualified_name = Literal::string(&identity.qualified_name);
    match &declaration.kind {
        DeclarationKind::Struct(record) => {
            let (deserialize, serialize) = value::persistence_attributes(graph, declaration, identity);
            let com_struct = quote! { #[::neocogi_com::com_struct(assigned_signature = #assigned_signature #deserialize)] };
            let fields = record
                .fields
                .iter()
                .map(|field| {
                    let docs = origin_docs(&field.docs, &field.span);
                    let field_name = rust_ident(&field.name)?;
                    let ty = value_type(graph, module, &field.ty)?;
                    Ok(quote! { #docs pub #field_name: #ty, })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            format_rust(quote! {
                #com_struct
                #serialize
                #[derive(Debug, PartialEq)]
                #declaration_docs
                pub struct #name { #(#fields)* }
            })
        }
        DeclarationKind::Enum(enumeration) => {
            let repr = enum_repr(enumeration.repr);
            let has_payload = enumeration.has_payload();
            let value_names = enumeration
                .variants
                .iter()
                .map(|variant| rust_ident(&variant.name))
                .collect::<Result<Vec<_>, _>>()?;
            let dense_members = if enumeration.dense {
                let count = enumeration.variants.len();
                let canonical_members = match enumeration.names {
                    Some(EnumNames::SnakeCase) => {
                        let names = enumeration
                            .variants
                            .iter()
                            .map(|variant| Literal::string(&snake_case(&variant.name)))
                            .collect::<Vec<_>>();
                        quote! {
                            /// Canonical names corresponding exactly to [`Self::ALL`].
                            pub const NAMES: [&'static str; Self::COUNT] = [#(#names),*];

                            /// Returns the canonical name, or `None` for an unknown ABI value.
                            pub const fn canonical_name(self) -> ::core::option::Option<&'static str> {
                                let ordinal = self.raw() as usize;
                                if ordinal < Self::COUNT { ::core::option::Option::Some(Self::NAMES[ordinal]) } else { ::core::option::Option::None }
                            }
                        }
                    }
                    None => quote! {},
                };
                quote! {
                    impl #name {
                        /// Number of declared values in this dense IDL domain.
                        pub const COUNT: usize = #count;

                        /// Every declared value in its zero-based discriminant order.
                        pub const ALL: [Self; Self::COUNT] = [#(Self::#value_names),*];

                        #canonical_members
                    }
                }
            } else {
                quote! {}
            };
            let mut previous = -1i64;
            let variants = enumeration
                .variants
                .iter()
                .map(|variant| {
                    let variant_docs = origin_docs(&variant.docs, &variant.span);
                    let variant_name = rust_ident(&variant.name)?;
                    let discriminant = variant.discriminant.unwrap_or(previous + 1);
                    previous = discriminant;
                    // Signed IDL representations may use negative values. Emit a
                    // unary minus and magnitude so both Rust enum syntax and the
                    // COM tag parser see the same integer expression.
                    let tag = if discriminant < 0 {
                        let magnitude = Literal::u64_unsuffixed(discriminant.unsigned_abs());
                        quote! { -#magnitude }
                    } else {
                        let value = Literal::u64_unsuffixed(discriminant as u64);
                        quote! { #value }
                    };
                    let fields = match &variant.fields {
                        VariantFields::Unit => quote! {},
                        VariantFields::Tuple(fields) => {
                            let fields = fields.iter().map(|ty| value_type(graph, module, ty)).collect::<Result<Vec<_>, _>>()?;
                            quote! { (#(#fields),*) }
                        }
                        VariantFields::Named(fields) => {
                            let fields = fields
                                .iter()
                                .map(|field| {
                                    let docs = origin_docs(&field.docs, &field.span);
                                    let name = rust_ident(&field.name)?;
                                    let ty = value_type(graph, module, &field.ty)?;
                                    Ok(quote! { #docs #name: #ty })
                                })
                                .collect::<Result<Vec<_>, GenerateError>>()?;
                            quote! { { #(#fields),* } }
                        }
                    };
                    if has_payload {
                        Ok(quote! { #variant_docs #variant_name #fields, })
                    } else {
                        Ok(quote! { #variant_docs #variant_name = #tag, })
                    }
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            if has_payload {
                let (deserialize, serialize) = value::persistence_attributes(graph, declaration, identity);
                format_rust(quote! {
                    #[::neocogi_com::com_enum(tag = #repr, assigned_signature = #assigned_signature #deserialize)]
                    #serialize
                    #[derive(Clone, Copy, Debug, PartialEq)]
                    #declaration_docs
                    pub enum #name { #(#variants)* }
                })
            } else {
                let mut source = format_macro_enum("com_int_enum", "enum", declaration_docs, &name, &repr, &variants)?;
                if graph.graph.values.definitions[&identity.qualified_name].persistence == crate::Persistence::Supported {
                    source.push_str(&format_rust(value::integer_persistence(&name, &repr))?);
                }
                source.push_str(&format_rust(quote! {
                    #dense_members
                    impl ::neocogi_com::ComValueType for #name { const VALUE_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(#assigned_signature); }
                })?);
                Ok(source)
            }
        }
        DeclarationKind::BitEnum(bit_enum) => {
            let repr = enum_repr(bit_enum.repr);
            let flags = bit_enum
                .flags
                .iter()
                .map(|flag| {
                    let flag_docs = origin_docs(&flag.docs, &flag.span);
                    let flag_name = rust_ident(&flag.name)?;
                    let value = Literal::u64_unsuffixed(flag.value);
                    Ok(quote! { #flag_docs #flag_name = #value, })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            let mut source = format_macro_enum("com_bit_enum", "bitenum", declaration_docs, &name, &repr, &flags)?;
            source.push_str(&format_rust(quote! {
                impl ::neocogi_com::ComValueType for #name { const VALUE_SIGNATURE: ::neocogi_com::NcAbiSignature = ::neocogi_com::NcAbiSignature::from_static(#assigned_signature); }
            })?);
            Ok(source)
        }
        DeclarationKind::Interface(interface) => {
            let bases = bases(graph, module, &interface.bases)?;
            let methods = interface_methods(graph, module, &interface.methods)?;
            let threading = threading_ident(interface.threading);
            let factory = matches!(interface.role, InterfaceRole::Factory).then(|| quote! { factory, });
            let class_abi = rust_ident(&format!("{}Abi", declaration.name))?;
            let class_marker = (interface.role == InterfaceRole::Class).then(|| {
                quote! {
                    // SAFETY: this projection was declared by an explicit IDL class contract.
                    unsafe impl ::neocogi_com::ComClassInterface for #class_abi {}
                }
            });
            format_rust(quote! {
                #[::neocogi_com::com_interface(
                    assigned_guid = #id,
                    name = #qualified_name,
                    threading = #threading,
                    #factory
                    assigned_signature = #assigned_signature
                )]
                #declaration_docs
                pub trait #name #bases { #(#methods)* }
                #class_marker
            })
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

/// Formats the Rust-like body accepted by one declarative COM enum macro.
fn format_macro_enum(
    macro_name: &str,
    keyword: &str,
    declaration_docs: TokenStream,
    name: &Ident,
    repr: &TokenStream,
    members: &[TokenStream],
) -> Result<String, GenerateError> {
    // Pretty-printers intentionally treat arbitrary macro bodies as opaque.
    // Format an equivalent, valid Rust enum first, then restore the small
    // `enum Name: Repr` / `bitenum Name: Repr` grammar owned by our macros.
    let formatted = format_rust(quote! {
        #declaration_docs
        #[repr(#repr)]
        pub enum #name { #(#members)* }
    })?;
    let repr_attribute = format!("#[repr({repr})]");
    let rust_declaration = format!("pub enum {name} {{");
    let macro_declaration = format!("pub {keyword} {name}: {repr} {{");
    let mut found_declaration = false;
    let mut source = format!("::neocogi_com::{macro_name}! {{\n");

    for line in formatted.lines() {
        // The representation is expressed after the name in the macro grammar,
        // so its temporary standard-Rust attribute must not remain in the body.
        if line == repr_attribute {
            continue;
        }
        source.push_str("    ");
        if line == rust_declaration {
            source.push_str(&macro_declaration);
            found_declaration = true;
        } else {
            source.push_str(line);
        }
        source.push('\n');
    }
    if !found_declaration {
        return Err(GenerateError::new(format!("formatted enum {name} lost its generated declaration")));
    }
    source.push_str("}\n");
    Ok(source)
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

/// Generates a Rust supertrait suffix from IDL required interfaces.
fn bases(graph: &Generation<'_>, module: &ResolvedModule, values: &[NamedType]) -> Result<TokenStream, GenerateError> {
    if values.is_empty() {
        return Ok(quote! {});
    }
    let values = values
        .iter()
        .map(|value| {
            let resolved = resolve(graph, module, value)?;
            named_rust_type(graph, module, resolved, false)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! { : #(#values)+* })
}

/// Emits ordinary methods in their normative vtable order.
fn interface_methods(graph: &Generation<'_>, module: &ResolvedModule, values: &[Method]) -> Result<Vec<TokenStream>, GenerateError> {
    values
        .iter()
        .map(|method| {
            let docs = origin_docs(&method.docs, &method.span);
            let name = rust_ident(&method.name)?;
            let receiver = match method.receiver {
                Receiver::Shared => quote! { &self },
                Receiver::Mutable => quote! { &mut self },
            };
            let parameters = method
                .parameters
                .iter()
                .map(|parameter| {
                    let name = rust_ident(&parameter.name)?;
                    let ty = parameter_type(graph, module, &parameter.ty)?;
                    Ok(quote! { #name: #ty })
                })
                .collect::<Result<Vec<_>, GenerateError>>()?;
            let output = output_type(graph, module, &method.output)?;
            // Owning arguments are transactional: failure returns the
            // original owners in declaration order, through ordinary Rust
            // values. Only the status and restored C slots cross COM.
            let owners = method.parameters.iter().filter(|parameter| matches!(parameter.ty, AbiType::ComBox(_)))
                .map(|parameter| parameter_type(graph, module, &parameter.ty))
                .collect::<Result<Vec<_>, _>>()?;
            let error = match owners.is_empty() {
                true => quote! { ::neocogi_com::NcStatus },
                false => quote! { (::neocogi_com::NcStatus, #(#owners),*) },
            };
            Ok(quote! {
                #docs fn #name(#receiver, #(#parameters),*) -> ::core::result::Result<#output, #error>;
            })
        })
        .collect()
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

/// Lowers one method input according to its explicit IDL ownership form.
fn parameter_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    match ty {
        AbiType::Scalar(_) | AbiType::Named(_) | AbiType::Array { .. } => {
            if let AbiType::Named(name) = ty {
                let resolved = resolve(graph, module, name)?;
                if !matches!(
                    resolved.declaration.kind,
                    DeclarationKind::Struct(_) | DeclarationKind::Enum(_) | DeclarationKind::BitEnum(_)
                ) {
                    return Err(GenerateError::new(format!("object parameter {} must be borrowed", name.rust_path())));
                }
            }
            value_type(graph, module, ty)
        }
        AbiType::Borrowed(referent) => match referent.as_ref() {
            AbiType::Named(name) if name.segments.as_slice() == ["str"] => Ok(quote! { ::neocogi_com::NcCopyFromBufferOffset<'_, u8> }),
            AbiType::Named(name) if builtin_object_type(name).is_none() => {
                let resolved = resolve(graph, module, name)?;
                match &resolved.declaration.kind {
                    DeclarationKind::Interface(_) => object_reference_type(graph, module, name, false),
                    _ => {
                        let value = named_rust_type(graph, module, resolved, false)?;
                        Ok(quote! { ::neocogi_com::NcValueRef<'_, #value> })
                    }
                }
            }
            AbiType::Named(name) => object_reference_type(graph, module, name, false),
            _ => Err(GenerateError::new("IDL borrows support only str and object declarations")),
        },
        AbiType::Slice(element) => {
            let element = value_type(graph, module, element)?;
            Ok(quote! { ::neocogi_com::NcCopyFromBufferOffset<'_, #element> })
        }
        AbiType::OutputSlice(element) => {
            let element = value_type(graph, module, element)?;
            Ok(quote! { ::neocogi_com::NcOutputWriter<'_, #element> })
        }
        AbiType::ComBox(_) => owned_object_type(graph, module, ty),
        AbiType::WeakComBox(_) | AbiType::WeakComPtr(_) | AbiType::ComPtr(_) | AbiType::Unit | AbiType::String | AbiType::Vector(_) | AbiType::UnsizedSlice(_) => Err(GenerateError::new(
            "unit, owned collections/text, and unborrowed slices are not valid method inputs",
        )),
    }
}

/// Lowers one successful method output and its ownership transfer.
fn output_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    match ty {
        AbiType::Unit => Ok(quote! { () }),
        AbiType::ComPtr(_) | AbiType::WeakComPtr(_) | AbiType::ComBox(_) | AbiType::WeakComBox(_) => owned_object_type(graph, module, ty),
        _ => value_type(graph, module, ty),
    }
}

/// Produces a `ComRef` or `ComPtr` for one interface-like declaration.
fn object_reference_type(graph: &Generation<'_>, module: &ResolvedModule, name: &NamedType, owned: bool) -> Result<TokenStream, GenerateError> {
    if let Some(abi) = builtin_object_type(name) {
        return Ok(if owned {
            quote! { ::neocogi_com::ComPtr<#abi> }
        } else {
            quote! { ::neocogi_com::ComRef<'_, #abi> }
        });
    }
    let resolved = resolve(graph, module, name)?;
    if matches!(
        resolved.declaration.kind,
        DeclarationKind::Struct(_) | DeclarationKind::Enum(_) | DeclarationKind::BitEnum(_)
    ) {
        return Err(GenerateError::new(format!("{} is a copied value, not an object reference", name.rust_path())));
    }
    let abi = named_rust_type(graph, module, resolved, true)?;
    let threading = match &resolved.declaration.kind {
        DeclarationKind::Interface(interface) => interface.threading,
        _ => unreachable!("value declarations were rejected above"),
    };
    let mode = matches!(threading, Threading::Shared).then(|| quote! { , ::neocogi_com::ThreadSafe });
    if owned {
        Ok(quote! { ::neocogi_com::ComPtr<#abi #mode> })
    } else {
        Ok(quote! { ::neocogi_com::ComRef<'_, #abi #mode> })
    }
}

/// Resolves a type through the semantic graph with an internal-invariant error.
fn resolve<'a>(graph: &Generation<'a>, module: &ResolvedModule, name: &NamedType) -> Result<ResolvedDeclaration<'a>, GenerateError> {
    graph
        .graph
        .resolve_declaration(&module.source.name.to_string(), name)
        .ok_or_else(|| GenerateError::new(format!("resolved module {} lost type mapping for {}", module.source.name, name.rust_path())))
}

/// Selects a native value type or an interface marker from local/imported bindings.
/// Only interfaces use the Abi suffix; copied values always use their native type.
fn named_rust_type(graph: &Generation<'_>, module: &ResolvedModule, resolved: ResolvedDeclaration<'_>, abi_suffix: bool) -> Result<TokenStream, GenerateError> {
    let mut name = resolved.declaration.name.clone();
    if abi_suffix && matches!(resolved.declaration.kind, DeclarationKind::Interface(_)) {
        name.push_str("Abi");
    }
    let name = rust_ident(&name)?;
    if resolved.module.source.name == module.source.name {
        Ok(quote! { #name })
    } else {
        let crate_ident = graph.paths.path(&resolved.module.source.name.to_string())?;
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

/// Converts an IDL threading capability into the macro's grammar identifier.
fn threading_ident(value: Threading) -> Ident {
    match value {
        Threading::Local => Ident::new("local", Span::call_site()),
        Threading::Shared => Ident::new("shared", Span::call_site()),
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
    match name.segments.as_slice() {
        [name] if name == "IUnknown" => Some(quote! { ::neocogi_com::IUnknown }),
        [name] if name == "INcBytes" => Some(quote! { ::neocogi_com::INcBytesAbi }),
        [name] if name == "INcString" => Some(quote! { ::neocogi_com::INcStringAbi }),
        _ => None,
    }
}

/// Adds readable provenance after authored docs, so it survives Rust macro forwarding.
fn origin_docs(lines: &[String], span: &crate::SourceSpan) -> TokenStream {
    let documentation = docs(lines);
    let origin = Literal::string(&format!(" {}", mapping::origin_label(span)));
    quote! { #documentation #[doc = #origin] }
}

/// Emits an ownership-explicit object wrapper from the same resolved interface
/// and threading policy as shared references. The wrapper itself selects lifetime.
fn owned_object_type(graph: &Generation<'_>, module: &ResolvedModule, ty: &AbiType) -> Result<TokenStream, GenerateError> {
    let (target, wrapper) = match ty {
        AbiType::ComPtr(target) => (target, "ComPtr"),
        AbiType::WeakComPtr(target) => (target, "WeakComPtr"),
        AbiType::ComBox(target) => (target, "ComBox"),
        AbiType::WeakComBox(target) => (target, "WeakComBox"),
        _ => return Err(GenerateError::new("resolved type is not an object ownership wrapper")),
    };
    let AbiType::Named(name) = target.as_ref() else { return Err(GenerateError::new("object target is not a contract")); };
    let shared = object_reference_type(graph, module, name, true)?;
    let mut ty: syn::TypePath = syn::parse2(shared).map_err(|error| GenerateError::new(error.to_string()))?;
    let segment = ty.path.segments.last_mut().expect("generated object wrapper");
    segment.ident = syn::Ident::new(wrapper, segment.ident.span());
    Ok(quote! { #ty })
}
