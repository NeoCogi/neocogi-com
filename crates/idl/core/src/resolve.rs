//! Cross-module name resolution, ABI validation, and canonical identities.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt,
};

use neocogi_com_core::{NcGuid, guid_from_name, guid_from_qualified_name};

use crate::{AbiType, Declaration, DeclarationKind, Enum, EnumRepr, Method, Module, NamedType, Threading, VariantFields, sha256::digest};

/// UUID namespace reserved for canonical NeoCogi IDL identities.
const NC_IDL_IDENTITY_NAMESPACE: NcGuid = guid_from_qualified_name("neocogi.idl.identity:v1");

/// Stable resolved identity for one declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationIdentity {
    /// Exact dotted qualified name used in descriptions and diagnostics.
    pub qualified_name: String,
    /// Positive source version from the version or ID attribute.
    pub version: u32,
    /// Derived or explicitly supplied 128-bit ABI identity.
    pub id: NcGuid,
}

/// One exact module dependency after its ABI fingerprint has been resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedDependency {
    /// Logical dotted dependency name.
    pub module_name: String,
    /// Stable identity derived from the dependency module name.
    pub module_id: NcGuid,
    /// Complete dependency ABI fingerprint.
    pub abi_fingerprint: [u8; 32],
}

/// One semantically valid IDL module with canonical ABI identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedModule {
    /// Parsed source model retained for generation and inspection.
    pub source: Module,
    /// Stable identity derived from the normalized module name.
    pub module_id: NcGuid,
    /// SHA-256 of the canonical ABI model and exact dependencies.
    pub abi_fingerprint: [u8; 32],
    /// Direct dependencies in canonical module-name order.
    pub dependencies: Vec<ResolvedDependency>,
    /// Declaration identities in source order.
    pub declarations: Vec<DeclarationIdentity>,
}

impl ResolvedModule {
    /// Renders the complete ABI fingerprint as lowercase hexadecimal text.
    pub fn fingerprint_hex(&self) -> String {
        // A fixed-size digest always produces exactly sixty-four hexadecimal
        // digits, so reserve its complete output once.
        let mut output = String::with_capacity(self.abi_fingerprint.len() * 2);
        for byte in self.abi_fingerprint {
            use fmt::Write as _;
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
        }
        output
    }

    /// Returns one resolved declaration by its exact source identifier.
    pub fn declaration(&self, name: &str) -> Option<&DeclarationIdentity> {
        let qualified_name = self.source.name.qualify(name);
        // Declaration names are unique after source parsing, so an exact
        // qualified-name match is unambiguous.
        self.declarations.iter().find(|declaration| declaration.qualified_name == qualified_name)
    }
}

/// A complete resolved module graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedModules {
    /// Modules retained in caller order after cross-reference validation.
    modules: Vec<ResolvedModule>,
}

/// Borrowed declaration selected through one already-validated type path.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedDeclaration<'a> {
    /// Module that owns the selected declaration.
    pub module: &'a ResolvedModule,
    /// Parsed declaration retained for kind-specific code generation.
    pub declaration: &'a Declaration,
    /// Stable identity paired with the parsed declaration.
    pub identity: &'a DeclarationIdentity,
}

impl ResolvedModules {
    /// Borrows every resolved module in caller order.
    pub fn modules(&self) -> &[ResolvedModule] {
        &self.modules
    }

    /// Returns one module by its exact dotted logical name.
    pub fn module(&self, name: &str) -> Option<&ResolvedModule> {
        // Logical module names are unique after graph construction.
        self.modules.iter().find(|module| module.source.name.to_string() == name)
    }

    /// Resolves one local, imported, or module-qualified declaration path.
    ///
    /// Semantic resolution has already rejected missing and ambiguous paths.
    /// This read-only view lets generators reuse the authoritative mapping
    /// without rebuilding a second name-resolution implementation.
    pub fn resolve_declaration<'a>(&'a self, owner: &str, name: &NamedType) -> Option<ResolvedDeclaration<'a>> {
        let owner = self.module(owner)?;
        let (module_name, declaration_name) = if name.segments.len() == 1 {
            let declaration_name = name.segments.first()?.as_str();
            if owner.source.declaration(declaration_name).is_some() {
                (owner.source.name.to_string(), declaration_name)
            } else {
                let import = owner
                    .source
                    .imports
                    .iter()
                    .find(|import| import.names.iter().any(|candidate| candidate == declaration_name))?;
                (import.module.clone(), declaration_name)
            }
        } else {
            (name.segments[..name.segments.len() - 1].join("."), name.segments.last()?.as_str())
        };
        let module = self.module(&module_name)?;
        let source_index = module.source.declarations.iter().position(|declaration| declaration.name == declaration_name)?;
        Some(ResolvedDeclaration {
            module,
            declaration: &module.source.declarations[source_index],
            identity: &module.declarations[source_index],
        })
    }
}

/// Semantic failure found while resolving a complete IDL module graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionError {
    /// Module being checked when the failure was found, when available.
    pub module: Option<String>,
    /// Specific violated identity, dependency, or ABI type rule.
    pub message: String,
}

impl fmt::Display for ResolutionError {
    /// Adds module context to the underlying semantic diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(module) = &self.module {
            write!(formatter, "IDL module {module}: {}", self.message)
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl std::error::Error for ResolutionError {}

/// Resolves imports, validates ABI positions, and fingerprints a module DAG.
pub fn resolve_modules(modules: Vec<Module>) -> Result<ResolvedModules, ResolutionError> {
    if modules.is_empty() {
        return Err(ResolutionError {
            module: None,
            message: "at least one IDL module is required".into(),
        });
    }

    let mut entries = build_entries(modules)?;
    connect_imports(&mut entries)?;
    for index in 0..entries.len() {
        validate_module(index, &entries)?;
    }

    let mut states = vec![VisitState::Unvisited; entries.len()];
    let mut fingerprints = vec![None; entries.len()];
    let mut stack = Vec::new();
    for index in 0..entries.len() {
        fingerprint_module(index, &entries, &mut states, &mut fingerprints, &mut stack)?;
    }

    // Capture peer data before consuming graph entries. This avoids
    // self-referential output while retaining caller module order.
    let snapshots = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            (
                entry.name.clone(),
                entry.module_id,
                fingerprints[index].expect("all module fingerprints completed"),
            )
        })
        .collect::<Vec<_>>();
    let modules = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let dependencies = entry
                .dependencies
                .iter()
                .map(|dependency| {
                    let (name, module_id, fingerprint) = &snapshots[*dependency];
                    ResolvedDependency {
                        module_name: name.clone(),
                        module_id: *module_id,
                        abi_fingerprint: *fingerprint,
                    }
                })
                .collect();
            ResolvedModule {
                source: entry.module,
                module_id: entry.module_id,
                abi_fingerprint: fingerprints[index].expect("all module fingerprints completed"),
                dependencies,
                declarations: entry.declaration_identities,
            }
        })
        .collect();
    Ok(ResolvedModules { modules })
}

/// Internal graph node with precomputed declaration identities.
struct ModuleEntry {
    /// Parsed source owned by this graph node.
    module: Module,
    /// Cached dotted name used for lookup and diagnostics.
    name: String,
    /// Stable module identity.
    module_id: NcGuid,
    /// Local declaration names mapped to resolved symbols.
    local_symbols: HashMap<String, Symbol>,
    /// Unqualified imported names mapped to owning symbols.
    imported_symbols: HashMap<String, Symbol>,
    /// Direct dependency indexes in canonical module-name order.
    dependencies: Vec<usize>,
    /// Declaration identities in source order.
    declaration_identities: Vec<DeclarationIdentity>,
}

/// Resolved declaration reference used during validation and encoding.
#[derive(Clone, Copy)]
struct Symbol {
    /// Index of the module that owns this declaration.
    module: usize,
    /// Stable derived or explicit declaration ID.
    id: NcGuid,
    /// Declaration category relevant to ABI position rules.
    kind: SymbolKind,
}

/// Coarse declaration categories used by the type validator.
#[derive(Clone, Copy, Eq, PartialEq)]
enum SymbolKind {
    /// A copied struct or enum value.
    Value,
    /// A reusable COM interface.
    Interface,
    /// A factory-created class primary projection.
    Class,
    /// A typed provider interface.
    Factory,
}

/// Depth-first traversal state for dependency fingerprinting.
#[derive(Clone, Copy, Eq, PartialEq)]
enum VisitState {
    /// No dependency path has reached this module yet.
    Unvisited,
    /// The module is on the active recursion stack.
    Visiting,
    /// The module and all dependencies have final fingerprints.
    Complete,
}

/// Creates graph nodes and rejects duplicate module or declaration IDs.
fn build_entries(modules: Vec<Module>) -> Result<Vec<ModuleEntry>, ResolutionError> {
    let mut module_names = HashSet::new();
    let mut global_declaration_ids = HashMap::new();
    let mut entries = Vec::with_capacity(modules.len());
    for module in modules {
        let name = module.name.to_string();
        if !module_names.insert(name.clone()) {
            return Err(error(None, format!("duplicate logical IDL module {name:?}")));
        }
        let module_id = derive_module_id(&name);
        let mut local_symbols = HashMap::new();
        let mut declaration_ids = HashSet::new();
        let mut declaration_identities = Vec::with_capacity(module.declarations.len());
        for declaration in &module.declarations {
            if is_reserved_name(&declaration.name) {
                return Err(error(
                    Some(&name),
                    format!("declaration name {:?} is reserved by the IDL type system", declaration.name),
                ));
            }
            let qualified_name = module.name.qualify(&declaration.name);
            let id = declaration
                .id
                .explicit
                .unwrap_or_else(|| derive_declaration_id(&qualified_name, declaration.id.version));
            if !declaration_ids.insert(id) {
                return Err(error(Some(&name), format!("declaration {:?} reuses another declaration ID", declaration.name)));
            }
            if let Some(previous_name) = global_declaration_ids.insert(id, qualified_name.clone()) {
                return Err(error(
                    Some(&name),
                    format!("declaration {qualified_name:?} reuses the ID assigned to {previous_name:?}"),
                ));
            }
            let symbol = Symbol {
                module: entries.len(),
                id,
                kind: symbol_kind(&declaration.kind),
            };
            local_symbols.insert(declaration.name.clone(), symbol);
            declaration_identities.push(DeclarationIdentity {
                qualified_name,
                version: declaration.id.version,
                id,
            });
        }
        entries.push(ModuleEntry {
            module,
            name,
            module_id,
            local_symbols,
            imported_symbols: HashMap::new(),
            dependencies: Vec::new(),
            declaration_identities,
        });
    }
    Ok(entries)
}

/// Resolves module imports and makes unqualified names unambiguous.
fn connect_imports(entries: &mut [ModuleEntry]) -> Result<(), ResolutionError> {
    let indexes = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.name.clone(), index))
        .collect::<HashMap<_, _>>();

    for index in 0..entries.len() {
        let module_name = entries[index].name.clone();
        let imports = entries[index].module.imports.clone();
        let mut dependencies = BTreeSet::new();
        let mut imported_symbols = HashMap::new();
        for import in imports {
            let Some(&dependency) = indexes.get(&import.module) else {
                return Err(error(Some(&module_name), format!("imports missing module {:?}", import.module)));
            };
            if dependency == index {
                return Err(error(Some(&module_name), "a module cannot import itself"));
            }
            dependencies.insert((entries[dependency].name.clone(), dependency));
            for name in import.names {
                if entries[index].local_symbols.contains_key(&name) {
                    return Err(error(Some(&module_name), format!("import {name:?} conflicts with a local declaration")));
                }
                let Some(&symbol) = entries[dependency].local_symbols.get(&name) else {
                    return Err(error(
                        Some(&module_name),
                        format!("module {:?} does not declare imported name {name:?}", entries[dependency].name),
                    ));
                };
                if imported_symbols.insert(name.clone(), symbol).is_some() {
                    return Err(error(Some(&module_name), format!("imported name {name:?} is ambiguous")));
                }
            }
        }
        entries[index].dependencies = dependencies.into_iter().map(|(_, dependency)| dependency).collect();
        entries[index].imported_symbols = imported_symbols;
    }
    Ok(())
}

/// Validates declaration-specific ABI positions and referenced kinds.
fn validate_module(index: usize, entries: &[ModuleEntry]) -> Result<(), ResolutionError> {
    let entry = &entries[index];
    for declaration in &entry.module.declarations {
        match &declaration.kind {
            DeclarationKind::Struct(record) => {
                for field in &record.fields {
                    validate_value_type(index, &field.ty, entries, &format!("struct {} field {}", declaration.name, field.name))?;
                }
            }
            DeclarationKind::Enum(enumeration) => {
                validate_enum(entry, declaration, enumeration)?;
                for variant in &enumeration.variants {
                    match &variant.fields {
                        VariantFields::Unit => {}
                        VariantFields::Tuple(fields) => {
                            for (field_index, field) in fields.iter().enumerate() {
                                validate_value_type(
                                    index,
                                    field,
                                    entries,
                                    &format!("enum {} variant {} field {field_index}", declaration.name, variant.name),
                                )?;
                            }
                        }
                        VariantFields::Named(fields) => {
                            for field in fields {
                                validate_value_type(
                                    index,
                                    &field.ty,
                                    entries,
                                    &format!("enum {} variant {} field {}", declaration.name, variant.name, field.name),
                                )?;
                            }
                        }
                    }
                }
            }
            DeclarationKind::Interface(interface) => {
                validate_interface_bases(index, &interface.bases, entries, &declaration.name)?;
                validate_methods(index, &interface.methods, entries, &declaration.name)?;
            }
            DeclarationKind::Class(class) => {
                validate_interface_bases(index, &class.required_interfaces, entries, &declaration.name)?;
                validate_methods(index, &class.methods, entries, &declaration.name)?;
            }
            DeclarationKind::Factory(factory) => {
                validate_interface_bases(index, &factory.required_interfaces, entries, &declaration.name)?;
                validate_methods(index, &factory.methods, entries, &declaration.name)?;
            }
        }
    }
    Ok(())
}

/// Checks enum discriminants against representation range and uniqueness.
fn validate_enum(entry: &ModuleEntry, declaration: &Declaration, enumeration: &Enum) -> Result<(), ResolutionError> {
    let (minimum, maximum) = repr_range(enumeration.repr);
    let mut previous = -1i128;
    let mut values = HashSet::new();
    for variant in &enumeration.variants {
        let value = variant.discriminant.map_or_else(
            || {
                previous
                    .checked_add(1)
                    .ok_or_else(|| error(Some(&entry.name), format!("enum {} discriminant overflow", declaration.name)))
            },
            |value| Ok(i128::from(value)),
        )?;
        if value < minimum || value > maximum {
            return Err(error(
                Some(&entry.name),
                format!(
                    "enum {} variant {} discriminant {value} does not fit its representation",
                    declaration.name, variant.name
                ),
            ));
        }
        if !values.insert(value) {
            return Err(error(
                Some(&entry.name),
                format!("enum {} variant {} repeats discriminant {value}", declaration.name, variant.name),
            ));
        }
        previous = value;
    }
    Ok(())
}

/// Returns the inclusive integer range for one enum representation.
fn repr_range(repr: EnumRepr) -> (i128, i128) {
    match repr {
        EnumRepr::U8 => (0, i128::from(u8::MAX)),
        EnumRepr::U16 => (0, i128::from(u16::MAX)),
        EnumRepr::U32 => (0, i128::from(u32::MAX)),
        EnumRepr::I8 => (i128::from(i8::MIN), i128::from(i8::MAX)),
        EnumRepr::I16 => (i128::from(i16::MIN), i128::from(i16::MAX)),
        EnumRepr::I32 => (i128::from(i32::MIN), i128::from(i32::MAX)),
    }
}

/// Requires each base or required projection to name an interface.
fn validate_interface_bases(index: usize, bases: &[NamedType], entries: &[ModuleEntry], owner: &str) -> Result<(), ResolutionError> {
    let mut identities = HashSet::new();
    for base in bases {
        let symbol = resolve_named(index, base, entries)?;
        if symbol.kind != SymbolKind::Interface {
            return Err(error(
                Some(&entries[index].name),
                format!("{owner} base {} is not an interface", base.rust_path()),
            ));
        }
        if !identities.insert((symbol.module, symbol.id)) {
            return Err(error(Some(&entries[index].name), format!("{owner} repeats interface {}", base.rust_path())));
        }
    }
    Ok(())
}

/// Validates parameter and result ownership for one callable surface.
fn validate_methods(index: usize, methods: &[Method], entries: &[ModuleEntry], owner: &str) -> Result<(), ResolutionError> {
    for method in methods {
        for parameter in &method.parameters {
            validate_parameter_type(index, &parameter.ty, entries, &format!("{owner}::{} parameter {}", method.name, parameter.name))?;
        }
        validate_output_type(index, &method.output, entries, &format!("{owner}::{} result", method.name))?;
    }
    Ok(())
}

/// Requires a recursively copyable type in aggregate payload storage.
fn validate_value_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], context: &str) -> Result<(), ResolutionError> {
    match ty {
        AbiType::Scalar(_) => Ok(()),
        AbiType::Named(name) => match resolve_named_or_str(index, name, entries)? {
            ResolvedName::Symbol(symbol) if symbol.kind == SymbolKind::Value => Ok(()),
            _ => Err(error(Some(&entries[index].name), format!("{context} must use a copied value type"))),
        },
        AbiType::Array { element, .. } | AbiType::Option(element) => validate_value_type(index, element, entries, context),
        AbiType::Unit | AbiType::String | AbiType::Borrowed(_) | AbiType::Slice(_) | AbiType::OutputSlice(_) => {
            Err(error(Some(&entries[index].name), format!("{context} must use a copied value type")))
        }
    }
}

/// Validates an input without allowing implicit COM ownership transfer.
fn validate_parameter_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], context: &str) -> Result<(), ResolutionError> {
    match ty {
        AbiType::Unit | AbiType::String => Err(error(Some(&entries[index].name), format!("{context} has no valid input ownership"))),
        AbiType::Scalar(_) => Ok(()),
        AbiType::Named(name) => match resolve_named_or_str(index, name, entries)? {
            ResolvedName::Symbol(symbol) if symbol.kind == SymbolKind::Value => Ok(()),
            ResolvedName::Symbol(_) => Err(error(
                Some(&entries[index].name),
                format!("{context} must borrow interface, class, and factory values with &"),
            )),
            ResolvedName::Str => Err(error(Some(&entries[index].name), format!("{context} must borrow str as &str"))),
        },
        AbiType::Borrowed(referent) => match referent.as_ref() {
            AbiType::Named(name) => match resolve_named_or_str(index, name, entries)? {
                ResolvedName::Str => Ok(()),
                ResolvedName::Symbol(symbol) if symbol.kind != SymbolKind::Value => Ok(()),
                ResolvedName::Symbol(_) => Err(error(Some(&entries[index].name), format!("{context} cannot borrow a copied value type"))),
            },
            _ => Err(error(Some(&entries[index].name), format!("{context} has an unsupported borrowed type"))),
        },
        AbiType::Slice(element) | AbiType::OutputSlice(element) => validate_value_type(index, element, entries, context),
        AbiType::Array { element, .. } => validate_value_type(index, element, entries, context),
        AbiType::Option(value) => validate_parameter_type(index, value, entries, context),
    }
}

/// Validates an output where bare COM names mean owned references.
fn validate_output_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], context: &str) -> Result<(), ResolutionError> {
    match ty {
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => Ok(()),
        AbiType::Named(name) => match resolve_named_or_str(index, name, entries)? {
            ResolvedName::Symbol(_) => Ok(()),
            ResolvedName::Str => Err(error(Some(&entries[index].name), format!("{context} cannot return unsized str; return String"))),
        },
        AbiType::Array { element, .. } => validate_value_type(index, element, entries, context),
        AbiType::Option(value) => validate_output_type(index, value, entries, context),
        AbiType::Borrowed(_) | AbiType::Slice(_) | AbiType::OutputSlice(_) => {
            Err(error(Some(&entries[index].name), format!("{context} cannot return a borrowed value")))
        }
    }
}

/// Named resolution result including the borrowed-only str built-in.
enum ResolvedName {
    /// The built-in unsized UTF-8 string view.
    Str,
    /// One local or imported declaration.
    Symbol(Symbol),
}

/// Resolves a path while recognizing the unsized str built-in.
fn resolve_named_or_str(index: usize, name: &NamedType, entries: &[ModuleEntry]) -> Result<ResolvedName, ResolutionError> {
    if name.segments.as_slice() == ["str"] {
        Ok(ResolvedName::Str)
    } else {
        resolve_named(index, name, entries).map(ResolvedName::Symbol)
    }
}

/// Resolves an unqualified import or explicitly module-qualified path.
fn resolve_named(index: usize, name: &NamedType, entries: &[ModuleEntry]) -> Result<Symbol, ResolutionError> {
    let entry = &entries[index];
    if name.segments.len() == 1 {
        let source_name = &name.segments[0];
        return entry
            .local_symbols
            .get(source_name)
            .or_else(|| entry.imported_symbols.get(source_name))
            .copied()
            .ok_or_else(|| error(Some(&entry.name), format!("unknown IDL type {source_name:?}")));
    }

    let module_name = name.segments[..name.segments.len() - 1].join(".");
    let declaration_name = &name.segments[name.segments.len() - 1];
    let Some(module_index) = entries.iter().position(|candidate| candidate.name == module_name) else {
        return Err(error(Some(&entry.name), format!("unknown IDL module {module_name:?}")));
    };
    if !entry.dependencies.contains(&module_index) {
        return Err(error(
            Some(&entry.name),
            format!("type {} is used without importing its module", name.rust_path()),
        ));
    }
    entries[module_index]
        .local_symbols
        .get(declaration_name)
        .copied()
        .ok_or_else(|| error(Some(&entry.name), format!("unknown IDL type {}", name.rust_path())))
}

/// Computes a fingerprint after recursively completing dependencies.
fn fingerprint_module(
    index: usize,
    entries: &[ModuleEntry],
    states: &mut [VisitState],
    fingerprints: &mut [Option<[u8; 32]>],
    stack: &mut Vec<usize>,
) -> Result<[u8; 32], ResolutionError> {
    match states[index] {
        VisitState::Complete => {
            return Ok(fingerprints[index].expect("complete module has a fingerprint"));
        }
        VisitState::Visiting => {
            let cycle_start = stack.iter().position(|candidate| *candidate == index).unwrap_or(0);
            let mut cycle = stack[cycle_start..]
                .iter()
                .map(|candidate| entries[*candidate].name.as_str())
                .collect::<Vec<_>>();
            cycle.push(entries[index].name.as_str());
            return Err(error(Some(&entries[index].name), format!("contract dependency cycle: {}", cycle.join(" -> "))));
        }
        VisitState::Unvisited => {}
    }

    states[index] = VisitState::Visiting;
    stack.push(index);
    let mut dependencies = Vec::with_capacity(entries[index].dependencies.len());
    for &dependency in &entries[index].dependencies {
        let fingerprint = fingerprint_module(dependency, entries, states, fingerprints, stack)?;
        // `connect_imports` stores dependencies in module-name order. Retain
        // that order so caller source order cannot affect the digest.
        dependencies.push((dependency, fingerprint));
    }
    stack.pop();

    let fingerprint = encode_module(index, entries, &dependencies)?;
    fingerprints[index] = Some(fingerprint);
    states[index] = VisitState::Complete;
    Ok(fingerprint)
}

/// Encodes one module into canonical bytes and hashes the result.
fn encode_module(index: usize, entries: &[ModuleEntry], dependencies: &[(usize, [u8; 32])]) -> Result<[u8; 32], ResolutionError> {
    let entry = &entries[index];
    let mut encoder = CanonicalEncoder::new();
    encoder.bytes(b"neocogi-idl:module-abi:v1\0");
    encoder.guid(entry.module_id);

    encoder.u32(u32_len(dependencies.len(), &entry.name, "dependency count")?);
    for (dependency, fingerprint) in dependencies {
        encoder.guid(entries[*dependency].module_id);
        encoder.bytes(fingerprint);
    }

    // Top-level declaration order is presentation-only. Sorting by exact name
    // makes harmless source reordering preserve the module ABI fingerprint.
    let mut declarations = entry.module.declarations.iter().enumerate().collect::<Vec<_>>();
    declarations.sort_by_key(|(_, declaration)| declaration.name.as_str());
    encoder.u32(u32_len(declarations.len(), &entry.name, "declaration count")?);
    for (source_index, declaration) in declarations {
        encode_declaration(index, declaration, entry.declaration_identities[source_index].id, entries, &mut encoder)?;
    }
    Ok(digest(&encoder.bytes))
}

/// Encodes one declaration without documentation or source formatting.
fn encode_declaration(
    index: usize,
    declaration: &Declaration,
    id: NcGuid,
    entries: &[ModuleEntry],
    encoder: &mut CanonicalEncoder,
) -> Result<(), ResolutionError> {
    encoder.text(&declaration.name, &entries[index].name)?;
    encoder.u32(declaration.id.version);
    encoder.guid(id);
    match &declaration.kind {
        DeclarationKind::Struct(record) => {
            encoder.tag(1);
            encoder.u32(u32_len(record.fields.len(), &entries[index].name, "field count")?);
            for field in &record.fields {
                encoder.text(&field.name, &entries[index].name)?;
                encode_type(index, &field.ty, entries, encoder)?;
            }
        }
        DeclarationKind::Enum(enumeration) => {
            encoder.tag(2);
            encoder.tag(enum_repr_tag(enumeration.repr));
            encoder.u32(u32_len(enumeration.variants.len(), &entries[index].name, "variant count")?);
            let mut previous = -1i64;
            for variant in &enumeration.variants {
                encoder.text(&variant.name, &entries[index].name)?;
                let discriminant = variant.discriminant.unwrap_or(previous + 1);
                encoder.i64(discriminant);
                previous = discriminant;
                match &variant.fields {
                    VariantFields::Unit => encoder.tag(0),
                    VariantFields::Tuple(fields) => {
                        encoder.tag(1);
                        encoder.u32(u32_len(fields.len(), &entries[index].name, "tuple field count")?);
                        for field in fields {
                            encode_type(index, field, entries, encoder)?;
                        }
                    }
                    VariantFields::Named(fields) => {
                        encoder.tag(2);
                        encoder.u32(u32_len(fields.len(), &entries[index].name, "named field count")?);
                        for field in fields {
                            encoder.text(&field.name, &entries[index].name)?;
                            encode_type(index, &field.ty, entries, encoder)?;
                        }
                    }
                }
            }
        }
        DeclarationKind::Interface(interface) => {
            encoder.tag(3);
            encode_callable(index, &interface.bases, &interface.methods, Threading::Local, entries, encoder)?;
        }
        DeclarationKind::Class(class) => {
            encoder.tag(4);
            encode_callable(index, &class.required_interfaces, &class.methods, class.threading, entries, encoder)?;
        }
        DeclarationKind::Factory(factory) => {
            encoder.tag(5);
            encode_callable(index, &factory.required_interfaces, &factory.methods, factory.threading, entries, encoder)?;
        }
    }
    Ok(())
}

/// Encodes bases, threading, and ordered methods for callable types.
fn encode_callable(
    index: usize,
    bases: &[NamedType],
    methods: &[Method],
    threading: Threading,
    entries: &[ModuleEntry],
    encoder: &mut CanonicalEncoder,
) -> Result<(), ResolutionError> {
    encoder.tag(match threading {
        Threading::Local => 0,
        Threading::Shared => 1,
    });
    encoder.u32(u32_len(bases.len(), &entries[index].name, "base count")?);
    for base in bases {
        encode_named(index, base, entries, encoder)?;
    }
    encoder.u32(u32_len(methods.len(), &entries[index].name, "method count")?);
    for method in methods {
        encoder.text(&method.name, &entries[index].name)?;
        encoder.tag(match method.receiver {
            crate::Receiver::Shared => 0,
            crate::Receiver::Mutable => 1,
        });
        encoder.u32(u32_len(method.parameters.len(), &entries[index].name, "parameter count")?);
        for parameter in &method.parameters {
            encoder.text(&parameter.name, &entries[index].name)?;
            encode_type(index, &parameter.ty, entries, encoder)?;
        }
        encode_type(index, &method.output, entries, encoder)?;
    }
    Ok(())
}

/// Encodes one recursively structured ABI type.
fn encode_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], encoder: &mut CanonicalEncoder) -> Result<(), ResolutionError> {
    match ty {
        AbiType::Unit => encoder.tag(0),
        AbiType::Scalar(name) => {
            encoder.tag(1);
            encoder.text(name, &entries[index].name)?;
        }
        AbiType::String => encoder.tag(2),
        AbiType::Named(name) => {
            encoder.tag(3);
            if name.segments.as_slice() == ["str"] {
                encoder.tag(0);
            } else {
                encoder.tag(1);
                encode_named(index, name, entries, encoder)?;
            }
        }
        AbiType::Borrowed(referent) => {
            encoder.tag(4);
            encode_type(index, referent, entries, encoder)?;
        }
        AbiType::Slice(element) => {
            encoder.tag(5);
            encode_type(index, element, entries, encoder)?;
        }
        AbiType::Array { element, len } => {
            encoder.tag(6);
            encoder.u32(*len);
            encode_type(index, element, entries, encoder)?;
        }
        AbiType::Option(value) => {
            encoder.tag(7);
            encode_type(index, value, entries, encoder)?;
        }
        AbiType::OutputSlice(element) => {
            encoder.tag(8);
            encode_type(index, element, entries, encoder)?;
        }
    }
    Ok(())
}

/// Encodes owning module and declaration IDs for a named reference.
fn encode_named(index: usize, name: &NamedType, entries: &[ModuleEntry], encoder: &mut CanonicalEncoder) -> Result<(), ResolutionError> {
    let symbol = resolve_named(index, name, entries)?;
    encoder.guid(entries[symbol.module].module_id);
    encoder.guid(symbol.id);
    Ok(())
}

/// Returns a compact stable tag for an enum representation.
fn enum_repr_tag(repr: EnumRepr) -> u8 {
    match repr {
        EnumRepr::U8 => 1,
        EnumRepr::U16 => 2,
        EnumRepr::U32 => 3,
        EnumRepr::I8 => 4,
        EnumRepr::I16 => 5,
        EnumRepr::I32 => 6,
    }
}

/// Owned canonical byte encoder with explicit length framing.
struct CanonicalEncoder {
    /// Accumulated canonical bytes in write order.
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    /// Starts an empty canonical byte sequence.
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Appends one single-byte grammar tag.
    fn tag(&mut self, value: u8) {
        self.bytes.push(value);
    }

    /// Appends one big-endian unsigned integer.
    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends one big-endian signed integer.
    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Appends canonical UUID bytes.
    fn guid(&mut self, value: NcGuid) {
        self.bytes.extend_from_slice(&value.to_uuid_bytes());
    }

    /// Appends raw bytes framed by the surrounding grammar.
    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    /// Appends one u32-length-prefixed UTF-8 field.
    fn text(&mut self, value: &str, module: &str) -> Result<(), ResolutionError> {
        self.u32(u32_len(value.len(), module, "UTF-8 field length")?);
        self.bytes(value.as_bytes());
        Ok(())
    }
}

/// Derives one module ID from its canonical dotted name.
fn derive_module_id(module_name: &str) -> NcGuid {
    let mut preimage = Vec::with_capacity(module_name.len() + 32);
    preimage.extend_from_slice(b"neocogi-idl:module:v1\0");
    preimage.extend_from_slice(&(module_name.len() as u64).to_be_bytes());
    preimage.extend_from_slice(module_name.as_bytes());
    guid_from_name(NC_IDL_IDENTITY_NAMESPACE, &preimage)
}

/// Derives one declaration ID from exact name and binary version fields.
fn derive_declaration_id(qualified_name: &str, version: u32) -> NcGuid {
    let mut preimage = Vec::with_capacity(qualified_name.len() + 40);
    preimage.extend_from_slice(b"neocogi-idl:declaration:v1\0");
    preimage.extend_from_slice(&(qualified_name.len() as u64).to_be_bytes());
    preimage.extend_from_slice(qualified_name.as_bytes());
    preimage.extend_from_slice(&version.to_be_bytes());
    guid_from_name(NC_IDL_IDENTITY_NAMESPACE, &preimage)
}

/// Maps a declaration payload to its coarse reference category.
fn symbol_kind(kind: &DeclarationKind) -> SymbolKind {
    match kind {
        DeclarationKind::Struct(_) | DeclarationKind::Enum(_) => SymbolKind::Value,
        DeclarationKind::Interface(_) => SymbolKind::Interface,
        DeclarationKind::Class(_) => SymbolKind::Class,
        DeclarationKind::Factory(_) => SymbolKind::Factory,
    }
}

/// Returns whether an identifier collides with an IDL built-in.
fn is_reserved_name(name: &str) -> bool {
    matches!(
        name,
        "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64" | "str" | "String" | "Option" | "Result"
    )
}

/// Converts a host length to the fixed ABI count type.
fn u32_len(value: usize, module: &str, field: &str) -> Result<u32, ResolutionError> {
    u32::try_from(value).map_err(|_| error(Some(module), format!("{field} exceeds u32")))
}

/// Constructs one consistently contextualized semantic error.
fn error(module: Option<&str>, message: impl Into<String>) -> ResolutionError {
    ResolutionError {
        module: module.map(str::to_owned),
        message: message.into(),
    }
}
