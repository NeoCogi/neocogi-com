//! Cross-module name resolution, ABI validation, and canonical identities.


mod events;
use std::{
    collections::{BTreeSet, HashMap},
    fmt,
};

use neocogi_com_core::{NcGuid, guid_from_name, guid_from_qualified_name};

use crate::{
    AbiType, BitEnum, Declaration, DeclarationKind, Diagnostic, Enum, EnumRepr, Method, Module, NamedType, SourceFile, SourceSpan, Threading,
    VariantFields, sha256::digest,
};

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
    /// Tool-computed identity of this declaration in the exact module revision.
    ///
    /// The module fingerprint makes every transitive contract change visible,
    /// while the declaration ID keeps distinct value types distinguishable in
    /// generated method and field descriptors.
    pub abi_signature: [u8; 32],
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
    /// Declaration identities in canonical top-level name order.
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
        self.source
            .declarations
            .iter()
            .position(|declaration| declaration.name == name)
            .map(|index| &self.declarations[index])
    }
}

/// A complete resolved module graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedModules {
    /// Modules retained in canonical logical-name and source-content order.
    modules: Vec<ResolvedModule>,
    /// Canonical recursive data graph shared with generated conversion and inspection.
    pub values: crate::ValueGraph,
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
    /// Borrows every resolved module in canonical order.
    pub fn modules(&self) -> &[ResolvedModule] {
        &self.modules
    }

    /// Returns one module by its exact dotted logical name.
    pub fn module(&self, name: &str) -> Option<&ResolvedModule> {
        // Logical module names are unique after graph construction.
        self.modules.iter().find(|module| module.source.name.to_string() == name)
    }

    /// Selects a local projection of a canonical identity, falling back to its owner.
    /// Closed generic values share identity across consumers but have distinct
    /// generated Rust paths; metadata must use the path available to its caller.
    pub(crate) fn resolve_canonical<'a>(&'a self, owner: &str, canonical: &str) -> Option<ResolvedDeclaration<'a>> {
        let preferred = self.module(owner)?;
        std::iter::once(preferred)
            .chain(self.modules.iter().filter(|module| module.source.name.to_string() != owner))
            .find_map(|module| {
                let index = module.declarations.iter().position(|identity| identity.qualified_name == canonical)?;
                Some(ResolvedDeclaration {
                    module,
                    declaration: &module.source.declarations[index],
                    identity: &module.declarations[index],
                })
            })
    }

    /// Resolves one local, imported, or module-qualified declaration path.
    ///
    /// Semantic resolution has already rejected missing and ambiguous paths.
    /// This read-only view lets generators reuse the authoritative mapping
    /// without rebuilding a second name-resolution implementation.
    pub fn resolve_declaration<'a>(&'a self, owner: &str, name: &NamedType) -> Option<ResolvedDeclaration<'a>> {
        let owner = self.module(owner)?;
        if !name.arguments.is_empty() {
            // Specialization provenance keeps fully qualified source arguments.
            // Compare semantic spellings rather than source spans, then select
            // the using module's concrete Rust projection.
            let spelling = name.rust_path();
            let index = owner.source.declarations.iter().position(|declaration| {
                declaration.specialization.as_ref().is_some_and(|instance| {
                    let mut applied = instance.template.clone();
                    applied.arguments = instance.arguments.clone();
                    applied.rust_path() == spelling
                })
            })?;
            return Some(ResolvedDeclaration {
                module: owner,
                declaration: &owner.source.declarations[index],
                identity: &owner.declarations[index],
            });
        }
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
    /// Independent diagnostics sorted by source location and error code when rendered.
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for ResolutionError {
    /// Renders the structured semantic diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut diagnostics = self.diagnostics.iter().collect::<Vec<_>>();
        diagnostics.sort_by(|left, right| left.stable_cmp(right));
        for (index, diagnostic) in diagnostics.into_iter().enumerate() {
            if index != 0 {
                writeln!(formatter)?;
            }
            write!(formatter, "{diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ResolutionError {}

/// Resolves imports, validates ABI positions, and fingerprints a module DAG.
pub fn resolve_modules(modules: Vec<Module>) -> Result<ResolvedModules, ResolutionError> {
    if modules.is_empty() {
        return Err(error(None, "at least one IDL module is required"));
    }

    let mut modules = crate::standard::attach(modules)?;
    modules.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.sources.iter().map(SourceFile::path).cmp(right.sources.iter().map(SourceFile::path)))
            .then_with(|| left.sources.iter().map(SourceFile::text).cmp(right.sources.iter().map(SourceFile::text)))
    });
    // Validate authored names and dependencies before templates are removed;
    // unused templates must not hide duplicate identities or ambiguous imports.
    let mut authored = build_entries(modules.clone())?;
    connect_externals_and_imports(&mut authored)?;
    // Enum shape/tag and public method-name rules are independent of type arguments.
    // Validate before specialization so unused templates cannot conceal a bad contract.
    for entry in &authored {
        for declaration in &entry.module.declarations {
            match &declaration.kind {
                DeclarationKind::Enum(enumeration) => validate_enum(entry, declaration, enumeration)?,
                DeclarationKind::Interface(interface) => validate_method_names(&entry.module, &interface.methods)?,
                _ => {}
            }
        }
    }
    let modules = crate::specialize::expand(modules)?;
    let mut entries = build_entries(modules)?;
    connect_externals_and_imports(&mut entries)?;
    let values = resolve_value_graph(&entries)?;
    let mut diagnostics = Vec::new();
    for index in 0..entries.len() {
        if let Err(error) = validate_module(index, &entries, &values) {
            diagnostics.extend(error.diagnostics);
        }
    }
    if !diagnostics.is_empty() {
        // A specialized field retains its template's location while the closed
        // use belongs to a consumer. Include both source sets in diagnostics.
        let sources = entries.iter().flat_map(|entry| entry.module.sources.iter().cloned()).collect::<Vec<_>>();
        return Err(ResolutionError {
            diagnostics: diagnostics.into_iter().map(|diagnostic| diagnostic.with_sources(&sources)).collect(),
        });
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
            let module_fingerprint = fingerprints[index].expect("all module fingerprints completed");
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
            let declarations = entry
                .declaration_identities
                .into_iter()
                .map(|mut declaration| {
                    declaration.abi_signature = derive_declaration_signature(module_fingerprint, declaration.id);
                    declaration
                })
                .collect();
            ResolvedModule {
                source: entry.module,
                module_id: entry.module_id,
                abi_fingerprint: module_fingerprint,
                dependencies,
                declarations,
            }
        })
        .collect();
    let mut graph = ResolvedModules { modules, values };
    let mut signatures = Vec::new();
    for (module_index, module) in graph.modules.iter().enumerate() {
        for (declaration_index, declaration) in module.source.declarations.iter().enumerate() {
            if let Some(specialization) = &declaration.specialization {
                signatures.push((module_index, declaration_index, crate::specialize::signature(&graph, specialization)?));
            }
        }
    }
    for (module, declaration, signature) in signatures {
        graph.modules[module].declarations[declaration].abi_signature = signature;
    }
    // Library adapters add constraints to their ordinary closed interfaces.
    // Report unsupported instantiations at the IDL type argument, not generation.
    for module in &graph.modules {
        for declaration in &module.source.declarations {
            if let Some(specialization) = &declaration.specialization {
                for argument in &specialization.arguments {
                    validate_closed_argument(&graph, module, argument, &declaration.span)?;
                }
            }
        }
    }
    // Validate authored and generated interface companion names together.
    let mut diagnostics = Vec::new();
    for module in &graph.modules {
        collect_diagnostics(&mut diagnostics, validate_generated_names(module));
    }
    if !diagnostics.is_empty() {
        let sources = graph.modules.iter().flat_map(|module| module.source.sources.iter().cloned()).collect::<Vec<_>>();
        return Err(ResolutionError {
            diagnostics: diagnostics.into_iter().map(|diagnostic| diagnostic.with_sources(&sources)).collect(),
        });
    }
    Ok(graph)
}

/// Checks constructor well-formedness even for an unused template parameter.
/// Field and method constraints are checked on substituted declarations, while
/// an object-reference wrapper must always target a closed contract.
fn validate_closed_argument(graph: &ResolvedModules, module: &ResolvedModule, ty: &AbiType, span: &SourceSpan) -> Result<(), ResolutionError> {
    match ty {
        AbiType::ComPtr(value) | AbiType::WeakComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value) => {
            let valid = match value.as_ref() {
                AbiType::Named(name) => {
                    builtin_object(name).is_some()
                        || graph
                            .resolve_declaration(&module.source.name.to_string(), name)
                            .is_some_and(|target| matches!(target.declaration.kind, DeclarationKind::Interface(_)))
                }
                _ => false,
            };
            if !valid {
                return Err(source_error(
                    &module.source,
                    "EIDL060",
                    span,
                    "COM reference argument must name a closed object contract",
                    "invalid owned-reference target",
                ));
            }
            validate_closed_argument(graph, module, value, span)
        }
        AbiType::Named(name) => {
            for argument in &name.arguments {
                validate_closed_argument(graph, module, argument, span)?;
            }
            Ok(())
        }
        AbiType::Borrowed(value)
        | AbiType::Vector(value)
        | AbiType::UnsizedSlice(value)
        | AbiType::Slice(value)
        | AbiType::OutputSlice(value)
        | AbiType::Array { element: value, .. } => validate_closed_argument(graph, module, value, span),
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => Ok(()),
    }
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
    /// Declaration identities in canonical top-level name order.
    declaration_identities: Vec<DeclarationIdentity>,
}

/// Resolved declaration reference used during validation and encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Symbol {
    /// Index of the module that owns this declaration.
    module: usize,
    /// Index of the declaration inside its owning module.
    declaration: usize,
    /// Stable derived or explicit declaration ID.
    id: NcGuid,
    /// Declaration category relevant to ABI position rules.
    kind: SymbolKind,
}

/// Coarse declaration categories used by the type validator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SymbolKind {
    /// A copied struct or enum value.
    Value,
    /// A reusable COM interface.
    Interface,
}

impl SymbolKind {
    /// Returns the IDL declaration category used in type-mismatch messages.
    fn description(self) -> &'static str {
        match self {
            Self::Value => "copied value",
            Self::Interface => "interface",
        }
    }
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
    let mut module_names = HashMap::<String, (SourceSpan, Vec<SourceFile>)>::new();
    let mut global_declaration_ids = HashMap::<NcGuid, (String, SourceSpan, Vec<SourceFile>)>::new();
    let mut entries = Vec::with_capacity(modules.len());
    for module in modules {
        let name = module.name.to_string();
        let module_span = module
            .declarations
            .first()
            .or_else(|| module.templates.first())
            .expect("parsed module contains declarations or templates")
            .span
            .clone();
        if let Some((previous_span, previous_sources)) = module_names.insert(name.clone(), (module_span.clone(), module.sources.clone())) {
            let sources = combined_sources(previous_sources, &module.sources);
            return Err(ResolutionError {
                diagnostics: vec![
                    Diagnostic::new("EIDL011", format!("duplicate logical IDL module {name:?}"))
                        .with_sources(&sources)
                        .primary(&module_span, "second module supplied here")
                        .secondary(&previous_span, "first module supplied here"),
                ],
            });
        }
        let module_id = derive_module_id(&name);
        let mut local_symbols = HashMap::new();
        let mut declaration_ids = HashMap::<NcGuid, SourceSpan>::new();
        let mut declaration_identities = Vec::with_capacity(module.declarations.len());
        for (declaration_index, declaration) in module.declarations.iter().enumerate() {
            if is_reserved_name(&declaration.name) {
                return Err(source_error(
                    &module,
                    "EIDL020",
                    &declaration.span,
                    format!("declaration name {:?} is reserved by the IDL type system", declaration.name),
                    "reserved type-system name",
                ));
            }
            let qualified_name = match &declaration.specialization {
                Some(specialization) => {
                    let segments = &specialization.template.segments;
                    format!("{}.{}", segments[..segments.len() - 1].join("."), declaration.name)
                }
                None => module.name.qualify(&declaration.name),
            };
            let id = declaration
                .id
                .explicit
                .unwrap_or_else(|| derive_declaration_id(&qualified_name, declaration.id.version));
            if let Some(previous) = declaration_ids.insert(id, declaration.span.clone()) {
                return Err(ResolutionError {
                    diagnostics: vec![
                        Diagnostic::new("EIDL012", format!("declaration {:?} reuses another declaration ID", declaration.name))
                            .with_sources(&module.sources)
                            .primary(&declaration.span, "duplicate declaration ID")
                            .secondary(&previous, "ID first assigned here"),
                    ],
                });
            }
            if let Some((previous_name, previous_span, previous_sources)) =
                global_declaration_ids.insert(id, (qualified_name.clone(), declaration.span.clone(), module.sources.clone()))
            {
                if declaration.specialization.is_none() || previous_name != qualified_name {
                    let sources = combined_sources(previous_sources, &module.sources);
                    return Err(ResolutionError {
                        diagnostics: vec![
                            Diagnostic::new("EIDL012", format!("declaration {qualified_name:?} reuses the ID assigned to {previous_name:?}"))
                                .with_sources(&sources)
                                .primary(&declaration.span, "duplicate declaration ID")
                                .secondary(&previous_span, format!("ID belongs to {previous_name}")),
                        ],
                    });
                }
            }
            let symbol = Symbol {
                module: entries.len(),
                declaration: declaration_index,
                id,
                kind: symbol_kind(&declaration.kind),
            };
            local_symbols.insert(declaration.name.clone(), symbol);
            declaration_identities.push(DeclarationIdentity {
                qualified_name,
                version: declaration.id.version,
                id,
                // The final value depends on the transitive module fingerprint,
                // which is available only after the complete graph is resolved.
                abi_signature: [0; 32],
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

/// Resolves explicit dependency edges and makes imported names unambiguous.
fn connect_externals_and_imports(entries: &mut [ModuleEntry]) -> Result<(), ResolutionError> {
    let indexes = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.name.clone(), index))
        .collect::<HashMap<_, _>>();

    for index in 0..entries.len() {
        let externals = entries[index].module.externals.clone();
        let imports = entries[index].module.imports.clone();
        let mut dependencies = BTreeSet::new();
        let mut imported_symbols = HashMap::new();
        let mut imported_origins = HashMap::<String, SourceSpan>::new();
        // Resolve the graph first. This makes an unused but intentional
        // external declaration a real fingerprinted native dependency.
        for external in externals {
            let Some(&dependency) = indexes.get(&external.module) else {
                return Err(source_error(
                    &entries[index].module,
                    "EIDL030",
                    &external.span,
                    format!("external dependency module {:?} is unavailable", external.module),
                    "dependency module is not present",
                ));
            };
            if dependency == index {
                return Err(source_error(
                    &entries[index].module,
                    "EIDL031",
                    &external.span,
                    "a module cannot depend on itself",
                    "self-dependency",
                ));
            }
            dependencies.insert((entries[dependency].name.clone(), dependency));
        }

        // Imports affect only unqualified name lookup. They may select symbols
        // from an established edge but cannot create a hidden dependency.
        for import in imports {
            let Some(&dependency) = indexes.get(&import.module) else {
                return Err(source_error(
                    &entries[index].module,
                    "EIDL035",
                    &import.span,
                    format!("import from {:?} requires `extern {};`", import.module, import.module),
                    "external dependency is not declared",
                ));
            };
            if !dependencies.contains(&(entries[dependency].name.clone(), dependency)) {
                return Err(source_error(
                    &entries[index].module,
                    "EIDL035",
                    &import.span,
                    format!("import from {:?} requires `extern {};`", import.module, import.module),
                    "external dependency is not declared",
                ));
            }
            for name in import.names {
                if let Some(local) = entries[index].local_symbols.get(&name) {
                    let local_span = &entries[index].module.declarations[local.declaration].span;
                    return Err(ResolutionError {
                        diagnostics: vec![
                            Diagnostic::new("EIDL032", format!("import {name:?} conflicts with a local declaration"))
                                .with_sources(&entries[index].module.sources)
                                .primary(&import.span, "conflicting import")
                                .secondary(local_span, "local declaration is here"),
                        ],
                    });
                }
                let Some(&symbol) = entries[dependency].local_symbols.get(&name) else {
                    return Err(source_error(
                        &entries[index].module,
                        "EIDL033",
                        &import.span,
                        format!("module {:?} does not declare imported name {name:?}", entries[dependency].name),
                        "name is not exported by this module",
                    ));
                };
                if imported_symbols.insert(name.clone(), symbol).is_some() {
                    let previous = imported_origins.get(&name).expect("an imported symbol always records its origin");
                    return Err(ResolutionError {
                        diagnostics: vec![
                            Diagnostic::new("EIDL034", format!("imported name {name:?} is ambiguous"))
                                .with_sources(&entries[index].module.sources)
                                .primary(&import.span, "second unqualified import")
                                .secondary(previous, "first unqualified import")
                                .note("remove one import or use a module-qualified type path"),
                        ],
                    });
                }
                imported_origins.insert(name, import.span.clone());
            }
        }
        entries[index].dependencies = dependencies.into_iter().map(|(_, dependency)| dependency).collect();
        entries[index].imported_symbols = imported_symbols;
    }
    Ok(())
}

/// Validates declaration-specific ABI positions and referenced kinds.
fn validate_module(index: usize, entries: &[ModuleEntry], values: &crate::ValueGraph) -> Result<(), ResolutionError> {
    let entry = &entries[index];
    let mut diagnostics = Vec::new();
    for declaration in &entry.module.declarations {
        match &declaration.kind {
            DeclarationKind::Struct(record) => {
                if record.persistence.required() {
                    let symbol = entry.local_symbols[&declaration.name];
                    let value = &values.definitions[&entry.declaration_identities[symbol.declaration].qualified_name];
                    for (field, ty) in record.fields.iter().zip(&value.fields) {
                        if let Some(reason) = values.persistence_failure(ty) {
                            let authored = match &declaration.specialization {
                                Some(instance) => {
                                    let mut name = instance.template.clone();
                                    name.arguments = instance.arguments.clone();
                                    name.rust_path()
                                }
                                None => declaration.name.clone(),
                            };
                            let mut diagnostic = Diagnostic::new(
                                "EIDL064",
                                format!("struct {authored} field {} cannot implement requested deserialization", field.name),
                            )
                            .with_sources(&entry.module.sources)
                            .primary(&field.span, "field has no supported decoder")
                            .note(reason);
                            if declaration.specialization.is_some() {
                                diagnostic = diagnostic.secondary(&declaration.span, "decoder required for this closed specialization");
                            }
                            diagnostics.push(diagnostic);
                        }
                    }
                }
                for field in &record.fields {
                    collect_diagnostics(
                        &mut diagnostics,
                        validate_field_type(
                            index,
                            &field.ty,
                            entries,
                            &format!("struct {} field {}", declaration.name, field.name),
                            &field.span,
                        ),
                    );
                }
            }
            DeclarationKind::Enum(enumeration) => {
                for variant in &enumeration.variants {
                    match &variant.fields {
                        VariantFields::Unit => {}
                        VariantFields::Tuple(fields) => {
                            for (field_index, field) in fields.iter().enumerate() {
                                collect_diagnostics(
                                    &mut diagnostics,
                                    validate_field_type(
                                        index,
                                        field,
                                        entries,
                                        &format!("enum {} variant {} field {field_index}", declaration.name, variant.name),
                                        &variant.span,
                                    ),
                                );
                            }
                        }
                        VariantFields::Named(fields) => {
                            for field in fields {
                                collect_diagnostics(
                                    &mut diagnostics,
                                    validate_field_type(
                                        index,
                                        &field.ty,
                                        entries,
                                        &format!("enum {} variant {} field {}", declaration.name, variant.name, field.name),
                                        &field.span,
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            DeclarationKind::BitEnum(bit_enum) => collect_diagnostics(&mut diagnostics, validate_bit_enum(entry, declaration, bit_enum)),
            DeclarationKind::Interface(interface) => {
                collect_diagnostics(&mut diagnostics, validate_interface_bases(index, interface, entries, &declaration.name));
                collect_diagnostics(&mut diagnostics, events::validate(index, interface, entries, &declaration.name));
                collect_diagnostics(&mut diagnostics, validate_methods(index, &interface.methods, entries, values, &declaration.name),
                );
            }
        }
    }
    collect_diagnostics(&mut diagnostics, validate_layouts(index, entries, values));
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(ResolutionError { diagnostics })
    }
}

/// Adds every top-level type emitted by `com_interface` for one trait name.
///
/// Keep this inventory aligned with `crates/com/macros/src/interface.rs`. Both
/// authored interfaces and closed generic specializations use the same
/// macro, so a single inventory covers both namespaces.
fn interface_companions(name: &str, companions: &mut Vec<String>) {
    for suffix in ["Abi", "VTable", "VtblFor", "Client"] {
        companions.push(format!("{name}{suffix}"));
    }
}

/// Reserves visible Rust companion types before generation, including imported names.
///
/// Authored declarations and explicit imports already have unambiguous IDL lookup,
/// but an interface or tagged enum also emits Rust types that share this module's
/// namespace. Reject collisions at the source location rather than leaving a
/// duplicate-item compiler error in an unrelated generated file.
fn validate_generated_names(module: &ResolvedModule) -> Result<(), ResolutionError> {
    let source = &module.source;
    let mut occupied = HashMap::<String, (SourceSpan, String)>::new();
    for declaration in &source.declarations {
        occupied
            .entry(declaration.name.clone())
            .or_insert_with(|| (declaration.span.clone(), format!("declaration {}", declaration.name)));
    }
    for import in &source.imports {
        for name in &import.names {
            occupied
                .entry(name.clone())
                .or_insert_with(|| (import.span.clone(), format!("imported name {name}")));
        }
    }

    let mut diagnostics = Vec::new();
    for declaration in &source.declarations {
        let mut companions = Vec::new();
        match &declaration.kind {
            DeclarationKind::Interface(_) => {
                interface_companions(&declaration.name, &mut companions);
            }
            DeclarationKind::Struct(_) | DeclarationKind::Enum(_) | DeclarationKind::BitEnum(_) => {}
        }
        for name in &companions {
            let origin = &declaration.span;
            if let Some((previous_span, previous_name)) = occupied.get(name.as_str()) {
                let diagnostic = Diagnostic::new(
                        "EIDL021",
                        format!("generated Rust name {name:?} from {} conflicts with {previous_name}", declaration.name),
                    )
                    .with_sources(&source.sources)
                    .primary(origin, "this declaration generates the conflicting name")
                    .secondary(previous_span, "name is already present here");
                diagnostics.push(diagnostic);
            } else {
                occupied.insert(name.to_owned(), (origin.clone(), format!("generated companion of {}", declaration.name)));
            }
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(ResolutionError { diagnostics })
    }
}

/// Rejects inline-value and required-interface cycles before generated Rust reaches layout checking.
fn validate_layouts(index: usize, entries: &[ModuleEntry], values: &crate::ValueGraph) -> Result<(), ResolutionError> {
    let mut states = vec![VisitState::Unvisited; entries[index].module.declarations.len()];
    let mut stack = Vec::new();
    for declaration in 0..states.len() {
        visit_layout(index, declaration, entries, values, &mut states, &mut stack)?;
    }
    Ok(())
}

/// Shows source template arguments instead of leaking a generated Rust hash name.
fn authored_declaration_name(declaration: &Declaration) -> String {
    match &declaration.specialization {
        Some(instance) => {
            let mut name = instance.template.clone();
            name.arguments = instance.arguments.clone();
            name.rust_path()
        }
        None => declaration.name.clone(),
    }
}

/// Walks one local declaration and reports the edge closing a cycle.
fn visit_layout(
    module: usize,
    declaration: usize,
    entries: &[ModuleEntry],
    values: &crate::ValueGraph,
    states: &mut [VisitState],
    stack: &mut Vec<usize>,
) -> Result<(), ResolutionError> {
    match states[declaration] {
        VisitState::Complete => return Ok(()),
        VisitState::Visiting => return Ok(()),
        VisitState::Unvisited => {}
    }
    states[declaration] = VisitState::Visiting;
    stack.push(declaration);

    let owner = &entries[module].module.declarations[declaration];
    let mut source_references = Vec::new();
    collect_layout_references(owner, &mut source_references);
    let references = match values.definitions.get(&entries[module].declaration_identities[declaration].qualified_name) {
        Some(definition) => definition
            .inline_references()
            .into_iter()
            .filter_map(|name| {
                // Layout edges use canonical identities. Recover the actual
                // local/imported projection and diagnostic span from its source
                // reference, including values instantiated in this consumer.
                source_references.iter().find_map(|reference| {
                    let target = resolve_named(module, reference, entries).ok()?;
                    let identity = &entries[target.module].declaration_identities[target.declaration];
                    (identity.qualified_name == name).then_some((target, &reference.span))
                })
            })
            .collect::<Vec<_>>(),
        None => source_references
            .iter()
            .filter_map(|reference| resolve_named(module, reference, entries).ok().map(|symbol| (symbol, &reference.span)))
            .collect(),
    };
    for (symbol, span) in references {
        if symbol.module != module || symbol.kind != symbol_kind(&owner.kind) {
            continue;
        }
        match states[symbol.declaration] {
            VisitState::Visiting => {
                let (layout, label, note) = match symbol.kind {
                    SymbolKind::Value => (
                        "copied-value layout",
                        "field",
                        "values require finite inline size; use an explicit sequence or object reference, since optional and array fields remain inline",
                    ),
                    SymbolKind::Interface => (
                        "interface projections",
                        "required interface",
                        "required interfaces must form an acyclic graph; use a method result for recursive object relationships",
                    ),
                };
                let cycle_start = stack.iter().position(|candidate| *candidate == symbol.declaration).unwrap_or(0);
                let mut names = stack[cycle_start..]
                    .iter()
                    .map(|index| authored_declaration_name(&entries[module].module.declarations[*index]))
                    .collect::<Vec<_>>();
                names.push(authored_declaration_name(&entries[module].module.declarations[symbol.declaration]));
                let definition = &entries[module].module.declarations[symbol.declaration];
                return Err(ResolutionError {
                    diagnostics: vec![
                        Diagnostic::new("EIDL063", format!("recursive {layout}: {}", names.join(" -> ")))
                            .with_sources(&entries[module].module.sources)
                            .primary(span, format!("this {label} closes the recursive layout"))
                            .secondary(&definition.span, "recursive declaration is defined here")
                            .note(note),
                    ],
                });
            }
            VisitState::Unvisited => visit_layout(module, symbol.declaration, entries, values, states, stack)?,
            VisitState::Complete => {}
        }
    }

    stack.pop();
    states[declaration] = VisitState::Complete;
    Ok(())
}

/// Collects inline value dependencies and required interface relationships for
/// cycle validation. Method arguments and owned COM handles do not inline their
/// referents; interface requirements determine a canonical set of interface contracts.
fn collect_layout_references<'a>(declaration: &'a Declaration, output: &mut Vec<&'a NamedType>) {
    match &declaration.kind {
        DeclarationKind::Struct(record) => {
            for field in &record.fields {
                collect_type_references(&field.ty, output);
            }
        }
        DeclarationKind::Enum(enumeration) => {
            for variant in &enumeration.variants {
                match &variant.fields {
                    VariantFields::Unit => {}
                    VariantFields::Tuple(fields) => {
                        for field in fields {
                            collect_type_references(field, output);
                        }
                    }
                    VariantFields::Named(fields) => {
                        for field in fields {
                            collect_type_references(&field.ty, output);
                        }
                    }
                }
            }
        }
        DeclarationKind::Interface(interface) => output.extend(&interface.bases),
        DeclarationKind::BitEnum(_) => {}
    }
}

/// Collects named paths nested through inline array and optional wrappers.
fn collect_type_references<'a>(ty: &'a AbiType, output: &mut Vec<&'a NamedType>) {
    match ty {
        AbiType::Named(name) => output.push(name),
        AbiType::Array { element, .. } => collect_type_references(element, output),
        AbiType::ComPtr(_) | AbiType::WeakComPtr(_) | AbiType::ComBox(_) | AbiType::WeakComBox(_)
        | AbiType::Borrowed(_)
        | AbiType::Vector(_)
        | AbiType::UnsizedSlice(_)
        | AbiType::Slice(_)
        | AbiType::OutputSlice(_)
        | AbiType::Unit
        | AbiType::Scalar(_)
        | AbiType::String => {}
    }
}

/// Checks every bit pattern against the selected unsigned storage width.
fn validate_bit_enum(entry: &ModuleEntry, declaration: &Declaration, bit_enum: &BitEnum) -> Result<(), ResolutionError> {
    let (_, maximum) = repr_range(bit_enum.repr);
    let mut values = HashMap::<u64, SourceSpan>::new();
    for flag in &bit_enum.flags {
        let value = i128::from(flag.value);
        if value > maximum {
            return Err(source_error(
                &entry.module,
                "EIDL040",
                &flag.span,
                format!("bit enum {} flag {} value {value} does not fit its representation", declaration.name, flag.name),
                "value is outside the selected representation",
            ));
        }
        if let Some(previous) = values.insert(flag.value, flag.span.clone()) {
            return Err(ResolutionError {
                diagnostics: vec![
                    Diagnostic::new("EIDL041", format!("bit enum {} flag {} repeats value {value}", declaration.name, flag.name))
                        .with_sources(&entry.module.sources)
                        .primary(&flag.span, "duplicate bit pattern")
                        .secondary(&previous, "value first assigned here"),
                ],
            });
        }
    }
    Ok(())
}

/// Checks enum discriminants against representation range and uniqueness.
fn validate_enum(entry: &ModuleEntry, declaration: &Declaration, enumeration: &Enum) -> Result<(), ResolutionError> {
    if enumeration.has_payload() {
        if let Some(variant) = enumeration.variants.iter().find(|variant| variant.discriminant.is_some()) {
            return Err(source_error(
                &entry.module,
                "EIDL073",
                &variant.span,
                format!("payload enum {} cannot assign an explicit value to {}", declaration.name, variant.name),
                "remove every explicit assignment; payload enum tags follow declaration order",
            ));
        }
    }
    if enumeration.names.is_some() && !enumeration.dense {
        return Err(source_error(
            &entry.module,
            "EIDL042",
            &declaration.span,
            format!("enum {} uses canonical names without `#[dense]`", declaration.name),
            "canonical names require a dense enum",
        ));
    }
    if enumeration.dense
        && (!matches!(enumeration.repr, EnumRepr::U8 | EnumRepr::U16 | EnumRepr::U32)
            || enumeration.variants.iter().any(|variant| !matches!(variant.fields, VariantFields::Unit)))
    {
        return Err(source_error(
            &entry.module,
            "EIDL043",
            &declaration.span,
            format!("dense enum {} must use an unsigned representation and unit variants", declaration.name),
            "invalid dense-enum shape",
        ));
    }
    let (minimum, maximum) = repr_range(enumeration.repr);
    let mut previous = -1i128;
    let mut values = HashMap::<i128, SourceSpan>::new();
    for (ordinal, variant) in enumeration.variants.iter().enumerate() {
        let value = variant.discriminant.map_or_else(
            || {
                previous.checked_add(1).ok_or_else(|| {
                    source_error(
                        &entry.module,
                        "EIDL044",
                        &variant.span,
                        format!("enum {} discriminant overflow", declaration.name),
                        "implicit discriminant overflows",
                    )
                })
            },
            |value| Ok(i128::from(value)),
        )?;
        if value < minimum || value > maximum {
            return Err(source_error(
                &entry.module,
                "EIDL045",
                &variant.span,
                format!(
                    "enum {} variant {} discriminant {value} does not fit its representation",
                    declaration.name, variant.name
                ),
                "value is outside the selected representation",
            ));
        }
        if let Some(previous) = values.insert(value, variant.span.clone()) {
            return Err(ResolutionError {
                diagnostics: vec![
                    Diagnostic::new(
                        "EIDL046",
                        format!("enum {} variant {} repeats discriminant {value}", declaration.name, variant.name),
                    )
                    .with_sources(&entry.module.sources)
                    .primary(&variant.span, "duplicate discriminant")
                    .secondary(&previous, "discriminant first assigned here"),
                ],
            });
        }
        if enumeration.dense && value != ordinal as i128 {
            return Err(source_error(
                &entry.module,
                "EIDL047",
                &variant.span,
                format!(
                    "dense enum {} variant {} must use discriminant {ordinal}; found {value}",
                    declaration.name, variant.name
                ),
                "non-contiguous dense discriminant",
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
fn validate_interface_bases(index: usize, interface: &crate::Interface, entries: &[ModuleEntry], owner: &str) -> Result<(), ResolutionError> {
    let mut identities = HashMap::<(usize, NcGuid), SourceSpan>::new();
    for base in &interface.bases {
        let symbol = resolve_named(index, base, entries)?;
        if symbol.kind != SymbolKind::Interface {
            return Err(kind_mismatch_error(
                index,
                symbol,
                entries,
                &base.span,
                format!("{owner} base {} is not an interface", base.rust_path()),
                "expected an interface base",
                "interface",
            ));
        }
        if let DeclarationKind::Interface(required) = &entries[symbol.module].module.declarations[symbol.declaration].kind {
            if matches!((interface.threading, required.threading), (Threading::Local, Threading::Shared)) {
                return Err(source_error(
                    &entries[index].module,
                    "EIDL052",
                    &base.span,
                    format!("{owner} cannot weaken the shared threading promise of {}", base.rust_path()),
                    "declare this interface with #[threading(shared)]",
                ));
            }
        }
        if let Some(previous) = identities.insert((symbol.module, symbol.id), base.span.clone()) {
            return Err(ResolutionError {
                diagnostics: vec![
                    Diagnostic::new("EIDL051", format!("{owner} repeats interface {}", base.rust_path()))
                        .with_sources(&entries[index].module.sources)
                        .primary(&base.span, "repeated base interface")
                        .secondary(&previous, "base first listed here"),
                ],
            });
        }
    }
    Ok(())
}

/// Rejects handle lookup conflicts at authored spans, even for unused templates.
/// Generated/specialized methods reuse the same policy during final validation.
fn validate_method_names(module: &Module, methods: &[Method]) -> Result<(), ResolutionError> {
    let diagnostics = methods.iter().filter(|method| neocogi_com_core::is_reserved_com_method(&method.name)).map(|method| {
        Diagnostic::new("EIDL100", format!("COM method `{}` conflicts with a COM handle operation", method.name))
            .with_sources(&module.sources)
            .primary(&method.span, "choose a distinct interface method name")
    }).collect::<Vec<_>>();
    if diagnostics.is_empty() { Ok(()) } else { Err(ResolutionError { diagnostics }) }
}

/// Validates ordinary signatures and generated output getters in vtable order.
fn validate_methods(index: usize, methods: &[Method], entries: &[ModuleEntry], values: &crate::ValueGraph, owner: &str) -> Result<(), ResolutionError> {
    let mut diagnostics = Vec::new();
    collect_diagnostics(&mut diagnostics, validate_method_names(&entries[index].module, methods));
    for method in methods {
        for parameter in &method.parameters {
            collect_diagnostics(
                &mut diagnostics,
                validate_parameter_type(
                    index,
                    &parameter.ty,
                    entries,
                    values,
                    &format!("{owner}::{} parameter {}", method.name, parameter.name),
                    &parameter.span,
                ),
            );
        }
        collect_diagnostics(
            &mut diagnostics,
            validate_output_type(index, &method.output, entries, values, &format!("{owner}::{} result", method.name), &method.span,
            ),
        );
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(ResolutionError { diagnostics })
    }
}

/// Visits message storage without synthesizing callback methods or delivery parameters.
fn value_fields(declaration: &Declaration) -> Vec<&AbiType> {
    match &declaration.kind {
        DeclarationKind::Struct(record) => record.fields.iter().map(|field| &field.ty).collect(),
        DeclarationKind::Enum(enumeration) => enumeration
            .variants
            .iter()
            .flat_map(|variant| match &variant.fields {
                VariantFields::Unit => Vec::new(),
                VariantFields::Tuple(fields) => fields.iter().collect(),
                VariantFields::Named(fields) => fields.iter().map(|field| &field.ty).collect(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Validates recursive field syntax independently from method ownership positions.
fn validate_field_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], context: &str, span: &SourceSpan) -> Result<(), ResolutionError> {
    if matches!(ty, AbiType::ComBox(_)) {
        return Err(source_error(&entries[index].module, "EIDL060", span,
            format!("{context}: exclusive owners are method transfers, not repeatable fields"),
            "use an explicit owning object contract instead of a repeatable value codec",
        ));
    }
    match ty {
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => Ok(()),
        AbiType::ComPtr(value) | AbiType::WeakComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value) => {
            validate_object_target(index, value, entries, context, span)}
        AbiType::Named(name) => {
            if builtin_object(name).is_some() {
                return Err(source_error(
                    &entries[index].module,
                    "EIDL060",
                    span,
                    format!("{context} must spell owned references as ComPtr<{}>", name.rust_path()),
                    "an object contract is not inline value storage",
                ));
            }
            let symbol = resolve_named(index, name, entries)?;
            match symbol.kind {
                SymbolKind::Value => Ok(()),
                SymbolKind::Interface => Err(source_error(
                    &entries[index].module,
                    "EIDL060",
                    span,
                    format!("{context} must spell owned references as ComPtr<{}>", name.rust_path()),
                    "an object contract is not inline value storage",
                )),
            }
        }
        AbiType::Array { element, .. } | AbiType::Vector(element) => validate_field_type(index, element, entries, context, span),
        AbiType::Borrowed(_) | AbiType::Slice(_) | AbiType::UnsizedSlice(_) | AbiType::OutputSlice(_) => Err(source_error(
            &entries[index].module,
            "EIDL060",
            span,
            format!("{context} requires an owned field"),
            "use Vec<T> for indirect ownership; borrowed fields cannot escape a call",
        )),
    }
}

/// Checks the referent kind of an explicit owner without treating it as stored data.
fn validate_object_target(index: usize, ty: &AbiType, entries: &[ModuleEntry], context: &str, span: &SourceSpan) -> Result<(), ResolutionError> {
    if let AbiType::Named(name) = ty {
        if builtin_object(name).is_some() {
            return Ok(());
        }
        if resolve_named(index, name, entries)?.kind == SymbolKind::Interface {
            return Ok(());
        }
    }
    Err(source_error(
        &entries[index].module,
        "EIDL060",
        span,
        format!("{context}: COM references require a closed interface contract"),
        "invalid owned-reference target",
    ))
}

/// Reads copied use-site eligibility from the resolved structural graph.
/// Layout validation separately reports inline cycles with source spans.
fn copied_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], values: &crate::ValueGraph) -> bool {
    match ty {
        AbiType::Scalar(_) => true,
        AbiType::Array { element, .. } => copied_type(index, element, entries, values),
        AbiType::Named(name) => resolve_named(index, name, entries).is_ok_and(|symbol| {
            let qualified = &entries[symbol.module].declaration_identities[symbol.declaration].qualified_name;
            values.definitions.get(qualified).is_some_and(|value| value.copied)
        }),
        _ => false,
    }
}

/// Resolves field paths once into finite named graph edges, including indirect recursion.
fn resolve_value_graph(entries: &[ModuleEntry]) -> Result<crate::ValueGraph, ResolutionError> {
    fn field(index: usize, ty: &AbiType, entries: &[ModuleEntry]) -> Result<crate::ValueType, ResolutionError> {
        use crate::ValueType as V;
        Ok(match ty {
            AbiType::WeakComPtr(value) => match field(index, value, entries)? {
                V::Object(name) => V::WeakObject(name),
                other => other,
            },
            AbiType::WeakComBox(value) => match field(index, value, entries)? {
                V::Object(name) => V::WeakBoxObject(name),
                other => other,
            },
            AbiType::ComPtr(value) | AbiType::ComBox(value) => match field(index, value, entries)? {
                V::Object(name) => V::Object(name),
                other => other,
            },
            AbiType::Unit => V::Unit,
            AbiType::Scalar(name) => V::Scalar(name.clone()),
            AbiType::String => V::Text,
            AbiType::Named(name) if builtin_object(name).is_some() => V::Object(name.segments.join(".")),
            AbiType::Named(name) => {
                let Ok(symbol) = resolve_named(index, name, entries) else {
                    // Keep diagnostic collection in validate_module; unresolved
                    // edges are never eligible for copied lowering.
                    return Ok(V::Named(name.segments.join(".")));
                };
                let name = entries[symbol.module].declaration_identities[symbol.declaration].qualified_name.clone();
                match symbol.kind {
                    SymbolKind::Value => V::Named(name),
                    _ => V::Object(name),
                }
            }
            AbiType::Vector(value) => V::Sequence(Box::new(field(index, value, entries)?)),
            AbiType::Array { element, len } => V::Array(Box::new(field(index, element, entries)?), *len),
            AbiType::Borrowed(_) | AbiType::Slice(_) | AbiType::OutputSlice(_) | AbiType::UnsizedSlice(_) => V::Unit,
        })
    }
    let mut graph = crate::ValueGraph::default();
    for (index, entry) in entries.iter().enumerate() {
        for (declaration_index, declaration) in entry.module.declarations.iter().enumerate() {
            if matches!(declaration.kind, DeclarationKind::Interface(_)) {
                continue;
            }
            let name = entry.declaration_identities[declaration_index].qualified_name.clone();
            let fields = value_fields(declaration)
                .into_iter()
                .map(|ty| field(index, ty, entries))
                .collect::<Result<Vec<_>, _>>()?;
            graph.definitions.insert(
                name.clone(),
                crate::ValueDefinition {
                    name,
                    fields,
                    copied: true,
                    persistence: crate::Persistence::Supported,
                },
            );
        }
    }
    graph.resolve_capabilities();
    Ok(graph)
}

/// Requires an implemented self-contained copied lowering at this method position.
fn validate_value_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], values: &crate::ValueGraph, context: &str, span: &SourceSpan,
) -> Result<(), ResolutionError> {
    validate_field_type(index, ty, entries, context, span)?;
    if copied_type(index, ty, entries, values) {
        Ok(())
    } else {
        Err(source_error(
            &entries[index].module,
            "EIDL060",
            span,
            format!("{context} has no copied ABI lowering"),
            "borrow a named value with &; strings, sequences and object fields use the checked borrowed value representation",
        ))
    }
}

/// Validates an input without allowing implicit COM ownership transfer.
fn validate_parameter_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], values: &crate::ValueGraph, context: &str, span: &SourceSpan,
) -> Result<(), ResolutionError> {
    let span = type_span(ty).unwrap_or(span);
    if let AbiType::Borrowed(referent) = ty {
        if let AbiType::Named(name) = referent.as_ref() {
            if builtin_object(name).is_some() {
                return Ok(());
            }
        }
    }
    match ty {
        AbiType::ComBox(value) => validate_object_target(index, value, entries, context, span),
        AbiType::WeakComBox(_) => Err(source_error(&entries[index].module, "EIDL061", span,
            format!("{context}: weak box inputs must use a live interface borrow"), "unsupported ownership transfer",
        )),
        AbiType::WeakComPtr(_) | AbiType::ComPtr(_) | AbiType::Unit | AbiType::String | AbiType::Vector(_) | AbiType::UnsizedSlice(_) => Err(source_error(
            &entries[index].module,
            "EIDL061",
            span,
            format!("{context} has no valid input ownership"),
            "invalid parameter type",
        )),
        AbiType::Scalar(_) => Ok(()),
        AbiType::Named(name) => match resolve_named_or_str(index, name, entries)? {
            ResolvedName::Symbol(symbol) if symbol.kind == SymbolKind::Value => validate_value_type(index, ty, entries, values, context, span),
            ResolvedName::Symbol(symbol) => Err(kind_mismatch_error(
                index,
                symbol,
                entries,
                span,
                format!("{context} must borrow interface values with &"),
                format!("{} values require an explicit borrow", symbol.kind.description()),
                "borrow the object type with `&`",
            )),
            ResolvedName::Str => Err(source_error(
                &entries[index].module,
                "EIDL061",
                span,
                format!("{context} must borrow str as &str"),
                "unsized string parameter",
            )),
        },
        AbiType::Borrowed(referent) => match referent.as_ref() {
            AbiType::Named(name) => match resolve_named_or_str(index, name, entries)? {
                ResolvedName::Str => Ok(()),
                ResolvedName::Symbol(symbol) if symbol.kind != SymbolKind::Value => Ok(()),
                ResolvedName::Symbol(_) => Ok(()),
            },
            _ => Err(source_error(
                &entries[index].module,
                "EIDL061",
                span,
                format!("{context} has an unsupported borrowed type"),
                "unsupported borrow",
            )),
        },
        AbiType::Slice(element) | AbiType::OutputSlice(element) => validate_value_type(index, element, entries, values, context, span),
        AbiType::Array { .. } => validate_value_type(index, ty, entries, values, context, span),
    }
}

/// Validates an output where bare COM names mean owned references.
fn validate_output_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], values: &crate::ValueGraph, context: &str, span: &SourceSpan,
) -> Result<(), ResolutionError> {
    let span = type_span(ty).unwrap_or(span);
    match ty {
        AbiType::Unit | AbiType::Scalar(_) => Ok(()),
        AbiType::ComPtr(value) | AbiType::WeakComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value) => {
            validate_object_target(index, value, entries, context, span)}
        AbiType::Named(_) | AbiType::Array { .. } => validate_value_type(index, ty, entries, values, context, span),
        AbiType::String => Err(source_error(
            &entries[index].module,
            "EIDL062",
            span,
            format!("{context}: return ComPtr<INcString> for an owned COM string"),
            "owned String values cannot escape a call-scoped wire guard",
        )),
        AbiType::Vector(_) => Err(source_error(
            &entries[index].module,
            "EIDL062",
            span,
            format!("{context} cannot transfer an owned collection through a call-scoped wire guard"),
            "use an explicit COM collection contract or a caller-owned output buffer",
        )),
        AbiType::Borrowed(_) | AbiType::Slice(_) | AbiType::OutputSlice(_) | AbiType::UnsizedSlice(_) => Err(source_error(
            &entries[index].module,
            "EIDL062",
            span,
            format!("{context} cannot return a borrowed value"),
            "borrowed return type",
        )),
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
            .ok_or_else(|| {
                source_error(
                    &entry.module,
                    "EIDL070",
                    &name.span,
                    format!("unknown IDL type {source_name:?}"),
                    "type is not declared or imported",
                )
            });
    }

    let module_name = name.segments[..name.segments.len() - 1].join(".");
    let declaration_name = &name.segments[name.segments.len() - 1];
    let Some(module_index) = entries.iter().position(|candidate| candidate.name == module_name) else {
        return Err(source_error(
            &entry.module,
            "EIDL071",
            &name.span,
            format!("unknown IDL module {module_name:?}"),
            "module is not present in the contract graph",
        ));
    };
    if !entry.dependencies.contains(&module_index) {
        return Err(source_error(
            &entry.module,
            "EIDL072",
            &name.span,
            format!("type {} is used without declaring its external module", name.rust_path()),
            "missing external dependency",
        ));
    }
    entries[module_index].local_symbols.get(declaration_name).copied().ok_or_else(|| {
        source_error(
            &entry.module,
            "EIDL070",
            &name.span,
            format!("unknown IDL type {}", name.rust_path()),
            "module does not declare this type",
        )
    })
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
        // Dependency connection stores modules in name order. Retain
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
    // Version 9 records advertised event interfaces as semantic dependencies.
    // This extends the canonical declaration grammar; every consumer regenerates.
    encoder.bytes(b"neocogi-idl:module-abi:v10\0");
    encoder.guid(entry.module_id);
    // Templates and concrete declarations use exactly the same grammar. Bound
    // parameters require a scope, not a second serializer or diagnostic strings.
    encoder.u32(u32_len(entry.module.templates.len(), &entry.name, "template count")?);
    for template in &entry.module.templates {
        let id = template.id.explicit.unwrap_or_else(|| derive_declaration_id(&entry.module.name.qualify(&template.name), template.id.version));
        encode_declaration(index, template, id, entries, &mut encoder)?;
    }

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

/// Lexical context shared by concrete declarations and open template encoding.
#[derive(Clone, Copy)]
struct EncodingScope<'a> {
    /// Index of the declaration's defining module for free-name resolution.
    module: usize,
    /// Authored binders; only their count and reference positions enter the bytes.
    parameters: &'a [String],
}

/// Encodes authored persistence policy uniformly; diagnostic formatting is never ABI grammar.
/// Surviving policy tags retain their meaning. The retired sequence tag is no
/// longer emitted; removing that authored policy requires a declaration revision.
fn encode_persistence(policy: crate::PersistencePolicy, encoder: &mut CanonicalEncoder) {
    encoder.tag(match policy {
        crate::PersistencePolicy::Automatic => 0,
        crate::PersistencePolicy::Default => 1,
    });
}

/// Encodes one declaration without documentation or source formatting.
fn encode_declaration(
    index: usize,
    declaration: &Declaration,
    id: NcGuid,
    entries: &[ModuleEntry],
    encoder: &mut CanonicalEncoder,
) -> Result<(), ResolutionError> {
    let scope = EncodingScope { module: index, parameters: &declaration.parameters ,
    };
    encoder.u32(u32_len(scope.parameters.len(), &entries[index].name, "type parameter count")?);
    encoder.text(&declaration.name, &entries[index].name)?;
    encoder.u32(declaration.id.version);
    encoder.guid(id);
    match &declaration.kind {
        DeclarationKind::Struct(record) => {
            encoder.tag(1);
            encode_persistence(record.persistence, encoder);
            encoder.u32(u32_len(record.fields.len(), &entries[index].name, "field count")?);
            for field in &record.fields {
                encoder.text(&field.name, &entries[index].name)?;
                encode_type(scope, &field.ty, entries, encoder)?;
            }
        }
        DeclarationKind::Enum(enumeration) => {
            encoder.tag(2);
            encoder.tag(enum_repr_tag(enumeration.repr));
            encoder.tag(u8::from(enumeration.has_payload()));
            encoder.tag(u8::from(enumeration.dense));
            encoder.tag(match enumeration.names { None => 0, Some(crate::EnumNames::SnakeCase) => 1 ,
            });
            encoder.u32(u32_len(enumeration.variants.len(), &entries[index].name, "variant count")?);
            let mut previous = -1i64;
            for variant in &enumeration.variants {
                encoder.text(&variant.name, &entries[index].name)?;
                // Presence is authored contract data even when the effective
                // integer matches an implicit assignment. Sum tags are always implicit.
                encoder.tag(u8::from(variant.discriminant.is_some()));
                let discriminant = variant.discriminant.unwrap_or(previous + 1);
                encoder.i64(discriminant);
                previous = discriminant;
                match &variant.fields {
                    VariantFields::Unit => encoder.tag(0),
                    VariantFields::Tuple(fields) => {
                        encoder.tag(1);
                        encoder.u32(u32_len(fields.len(), &entries[index].name, "tuple field count")?);
                        for field in fields {
                            encode_type(scope, field, entries, encoder)?;
                        }
                    }
                    VariantFields::Named(fields) => {
                        encoder.tag(2);
                        encoder.u32(u32_len(fields.len(), &entries[index].name, "named field count")?);
                        for field in fields {
                            encoder.text(&field.name, &entries[index].name)?;
                            encode_type(scope, &field.ty, entries, encoder)?;
                        }
                    }
                }
            }
        }
        DeclarationKind::BitEnum(bit_enum) => {
            // A distinct kind tag prevents an ordinary integer enum from being
            // ABI-equivalent to a combinable set with the same constants.
            encoder.tag(6);
            encoder.tag(enum_repr_tag(bit_enum.repr));
            encoder.u32(u32_len(bit_enum.flags.len(), &entries[index].name, "bit flag count")?);
            for flag in &bit_enum.flags {
                encoder.text(&flag.name, &entries[index].name)?;
                encoder.u64(flag.value);
            }
        }
        DeclarationKind::Interface(interface) => {
            encoder.tag(3);
            encode_methods(scope, &interface.bases, &interface.methods, interface.threading, entries, encoder)?;
            // Emitted interfaces are semantic dependencies, independent of vtable ancestry.
            encoder.u32(u32_len(interface.events.len(), &entries[index].name, "event count")?);
            for event in &interface.events { encode_named(scope, event, entries, encoder)?; }
        }
    }
    Ok(())
}

/// Encodes bases, threading, and ordered methods for callable types.
fn encode_methods(
    scope: EncodingScope<'_>,
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
    encoder.u32(u32_len(bases.len(), &entries[scope.module].name, "base count")?);
    for base in bases {
        encode_named(scope, base, entries, encoder)?;
    }
    encoder.u32(u32_len(methods.len(), &entries[scope.module].name, "method count")?);
    for method in methods {
        encoder.tag(0);
        encoder.text(&method.name, &entries[scope.module].name)?;
        encoder.tag(match method.receiver {
            crate::Receiver::Shared => 0,
            crate::Receiver::Mutable => 1,
        });
        encoder.u32(u32_len(method.parameters.len(), &entries[scope.module].name, "parameter count")?);
        for parameter in &method.parameters {
            encoder.text(&parameter.name, &entries[scope.module].name)?;
            encode_type(scope, &parameter.ty, entries, encoder)?;
        }
        encode_type(scope, &method.output, entries, encoder)?;
    }
    Ok(())
}

/// Encodes one recursively structured ABI type.
fn encode_type(scope: EncodingScope<'_>, ty: &AbiType, entries: &[ModuleEntry], encoder: &mut CanonicalEncoder) -> Result<(), ResolutionError> {
    match ty {
        AbiType::Unit => encoder.tag(0),
        AbiType::Scalar(name) => {
            encoder.tag(1);
            encoder.text(name, &entries[scope.module].name)?;
        }
        AbiType::String => encoder.tag(2),
        AbiType::Named(name) => {
            if let [local] = name.segments.as_slice() {
                if let Some(position) = scope.parameters.iter().position(|parameter| parameter == local) {
                    encoder.tag(14);
                    encoder.u32(u32_len(position, &entries[scope.module].name, "parameter position")?);
                    return Ok(());
                }
            }
            encoder.tag(3);
            if name.segments.as_slice() == ["str"] {
                encoder.tag(0);
            } else if let Some(identity) = builtin_object(name) {
                // Fixed core capabilities contribute their exact ABI identity.
                encoder.tag(2);
                encoder.guid(identity.iid);
                // SAFETY: built-in signatures point to immutable core constants.
                encoder.bytes(unsafe { identity.abi_signature.as_digest().expect("static core signature") });
            } else {
                encoder.tag(1);
                encode_named(scope, name, entries, encoder)?;
            }
        }
        AbiType::Borrowed(referent) => {
            encoder.tag(4);
            encode_type(scope, referent, entries, encoder)?;
        }
        AbiType::Slice(element) => {
            encoder.tag(5);
            encode_type(scope, element, entries, encoder)?;
        }
        AbiType::Array { element, len } => {
            encoder.tag(6);
            encoder.u32(*len);
            encode_type(scope, element, entries, encoder)?;
        }
        AbiType::ComBox(target) | AbiType::WeakComBox(target) => {
            encoder.tag(match ty { AbiType::ComBox(_) => 12, AbiType::WeakComBox(_) => 13, _ => unreachable!() ,
            });
            encode_type(scope, target, entries, encoder)?;
        }
        AbiType::ComPtr(target) => {
            encoder.tag(9);
            encode_type(scope, target, entries, encoder)?;
        }
        AbiType::WeakComPtr(target) => {
            // New ownership spelling cannot alias strong or exclusive weak transfers.
            encoder.tag(15);
            encode_type(scope, target, entries, encoder)?;
        }
        AbiType::Vector(element) => {
            encoder.tag(10);
            encode_type(scope, element, entries, encoder)?;
        }
        AbiType::UnsizedSlice(element) => {
            encoder.tag(11);
            encode_type(scope, element, entries, encoder)?;
        }
        AbiType::OutputSlice(element) => {
            // Output-only slots include an initialized-prefix count in ABI 17.
            // Their revision must differ from the retired initialized in/out ABI.
            encoder.tag(14);
            encode_type(scope, element, entries, encoder)?;
        }
    }
    Ok(())
}

/// Encodes owning module and declaration IDs for a named reference.
fn encode_named(scope: EncodingScope<'_>, name: &NamedType, entries: &[ModuleEntry], encoder: &mut CanonicalEncoder) -> Result<(), ResolutionError> {
    let (module, id) = if scope.parameters.is_empty() {
        let symbol = resolve_named(scope.module, name, entries)?;
        (symbol.module, symbol.id)
    } else {
        // Specialization qualifies every free template name. It can refer to
        // another template that has no concrete symbol-table entry of its own.
        let owner = name.segments[..name.segments.len() - 1].join(".");
        let local = name.segments.last().expect("normalized named type");
        let module = entries.iter().position(|entry| entry.name == owner)
            .ok_or_else(|| error(Some(&entries[scope.module].name), format!("unresolved template module {owner}")))?;
        let declaration = entries[module].module.declarations.iter().chain(&entries[module].module.templates)
            .find(|declaration| &declaration.name == local)
            .ok_or_else(|| error(Some(&owner), format!("unresolved template declaration {local}")))?;
        let id = declaration.id.explicit.unwrap_or_else(|| derive_declaration_id(&entries[module].module.name.qualify(local), declaration.id.version));
        (module, id)
    };
    encoder.guid(entries[module].module_id);
    encoder.guid(id);
    encoder.u32(u32_len(name.arguments.len(), &entries[scope.module].name, "type argument count")?);
    for argument in &name.arguments {
        encode_type(scope, argument, entries, encoder)?;
    }
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

    /// Appends one big-endian unsigned wide integer.
    fn u64(&mut self, value: u64) {
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
pub(crate) fn derive_declaration_id(qualified_name: &str, version: u32) -> NcGuid {
    let mut preimage = Vec::with_capacity(qualified_name.len() + 40);
    preimage.extend_from_slice(b"neocogi-idl:declaration:v1\0");
    preimage.extend_from_slice(&(qualified_name.len() as u64).to_be_bytes());
    preimage.extend_from_slice(qualified_name.as_bytes());
    preimage.extend_from_slice(&version.to_be_bytes());
    guid_from_name(NC_IDL_IDENTITY_NAMESPACE, &preimage)
}

/// Derives one declaration-scoped signature from an exact module revision.
///
/// A module fingerprint is deliberately part of the preimage: loading is
/// exact at module granularity, so every declaration signature must reject a
/// peer compiled against any other transitive revision. The declaration ID
/// prevents two aggregates in that same revision from collapsing to one type
/// identity in runtime descriptors.
fn derive_declaration_signature(module_fingerprint: [u8; 32], declaration_id: NcGuid) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(80);
    preimage.extend_from_slice(b"neocogi-idl:declaration-abi:v1\0");
    preimage.extend_from_slice(&module_fingerprint);
    preimage.extend_from_slice(&declaration_id.to_uuid_bytes());
    digest(&preimage)
}

/// Maps a declaration payload to its coarse reference category.
fn symbol_kind(kind: &DeclarationKind) -> SymbolKind {
    match kind {
        DeclarationKind::Struct(_) | DeclarationKind::Enum(_) | DeclarationKind::BitEnum(_) => SymbolKind::Value,
        DeclarationKind::Interface(_) => SymbolKind::Interface,
    }
}

/// Returns whether an identifier collides with an IDL built-in.
pub(crate) fn is_reserved_name(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "f32"
            | "f64"
            | "Guid"
            | "Status"
            | "IUnknown"
            | "INcBytes"
            | "INcString"
            | "ComPtr" | "WeakComPtr" | "ComBox" | "WeakComBox"
            | "Vec"
            | "Slice"
            | "Array"
            | "str"
            | "String"
            | "Result"
    )
}

/// Converts a host length to the fixed ABI count type.
fn u32_len(value: usize, module: &str, field: &str) -> Result<u32, ResolutionError> {
    u32::try_from(value).map_err(|_| error(Some(module), format!("{field} exceeds u32")))
}

/// Returns the most precise named-type range nested inside an ABI type.
fn type_span(ty: &AbiType) -> Option<&SourceSpan> {
    match ty {
        AbiType::Named(name) => Some(&name.span),
        AbiType::ComPtr(value) | AbiType::WeakComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value)
        | AbiType::Borrowed(value)
        | AbiType::Vector(value)
        | AbiType::UnsizedSlice(value)
        | AbiType::Slice(value)
        | AbiType::OutputSlice(value)
        | AbiType::Array { element: value, .. } => type_span(value),
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => None,
    }
}

/// Constructs one source-local semantic error.
fn source_error(module: &Module, code: &str, span: &SourceSpan, message: impl Into<String>, label: impl Into<String>) -> ResolutionError {
    ResolutionError {
        diagnostics: vec![Diagnostic::new(code, message).with_sources(&module.sources).primary(span, label)],
    }
}

/// Constructs a type-kind error that points to both use and definition.
fn kind_mismatch_error(
    owner: usize,
    symbol: Symbol,
    entries: &[ModuleEntry],
    span: &SourceSpan,
    message: impl Into<String>,
    label: impl Into<String>,
    help: impl Into<String>,
) -> ResolutionError {
    let definition = &entries[symbol.module].module.declarations[symbol.declaration];
    let sources = combined_sources(entries[owner].module.sources.clone(), &entries[symbol.module].module.sources);
    ResolutionError {
        diagnostics: vec![
            Diagnostic::new("EIDL060", message)
                .with_sources(&sources)
                .primary(span, label)
                .secondary(
                    &definition.span,
                    format!("{} {:?} is declared here", symbol.kind.description(), definition.name),
                )
                .note(help),
        ],
    }
}

/// Unions diagnostic sources in normalized path order without duplicates.
fn combined_sources(first: Vec<SourceFile>, second: &[SourceFile]) -> Vec<SourceFile> {
    let mut sources = first
        .into_iter()
        .map(|source| (source.path().to_owned(), source))
        .collect::<std::collections::BTreeMap<_, _>>();
    for source in second {
        sources.entry(source.path().to_owned()).or_insert_with(|| source.clone());
    }
    sources.into_values().collect()
}

/// Appends every diagnostic from a failed independent validation check.
fn collect_diagnostics(target: &mut Vec<Diagnostic>, result: Result<(), ResolutionError>) {
    if let Err(error) = result {
        target.extend(error.diagnostics);
    }
}

/// Constructs one consistently contextualized internal semantic error.
fn error(module: Option<&str>, message: impl Into<String>) -> ResolutionError {
    let message = message.into();
    ResolutionError {
        diagnostics: vec![Diagnostic::new(
            "EIDL099",
            match module {
                Some(module) => format!("IDL module {module}: {message}"),
                None => message,
            },
        )],
    }
}

/// Recognizes core object capabilities without manufacturing IDL declarations.
pub(crate) fn builtin_object(name: &NamedType) -> Option<neocogi_com_core::NcInterfaceId> {
    use neocogi_com_core::ComInterface;
    match name.segments.as_slice() {
        [name] if name == "IUnknown" => Some(neocogi_com_core::IUnknown::INTERFACE_ID),
        [name] if name == "INcBytes" => Some(neocogi_com_core::INcBytesAbi::INTERFACE_ID),
        [name] if name == "INcString" => Some(neocogi_com_core::INcStringAbi::INTERFACE_ID),
        _ => None,
    }
}
