//! Cross-module name resolution, ABI validation, and canonical identities.

mod bind;
use crate::ParseError;
use crate::binding::{Binding, DeclarationKey, Intrinsic};
use std::{
    collections::{BTreeSet, HashMap},
    fmt,
};

use neocogi_com_core::{NcComDenseEnumMember, NcComInterfaceCompanion, NcGuid, guid_from_name, guid_from_qualified_name};

use crate::{
    AbiType, BitEnum, Declaration, DeclarationKind, Diagnostic, Enum, EnumRepr, Method, Module, NamedType, SourceFile, SourceSpan, Threading, VariantFields,
    sha256::digest,
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

    /// Selects this graph's exact bound Rust projection. Source spelling and
    /// imports are never reconstructed here; the name must belong to this graph.
    pub(crate) fn resolve_declaration<'a>(&'a self, name: &NamedType) -> Option<ResolvedDeclaration<'a>> {
        self.project(name.binding.projection()?)
    }

    /// Borrows one concrete declaration and its already-computed canonical identity.
    fn project(&self, key: &DeclarationKey) -> Option<ResolvedDeclaration<'_>> {
        let module = self.modules.get(key.module)?;
        let index = module.source.declarations.iter().position(|declaration| declaration.name == key.name)?;
        Some(ResolvedDeclaration {
            module,
            declaration: &module.source.declarations[index],
            identity: &module.declarations[index],
        })
    }

    /// A consumer may expose its own closed projection of a supplied canonical
    /// type. This explicit local preference never searches unrelated modules.
    pub(crate) fn local_projection<'a>(&'a self, owner: &ResolvedModule, canonical: &str) -> Option<ResolvedDeclaration<'a>> {
        let module = self.module(&owner.source.name.to_string())?;
        let index = module.declarations.iter().position(|identity| identity.qualified_name == canonical)?;
        Some(ResolvedDeclaration {
            module,
            declaration: &module.source.declarations[index],
            identity: &module.declarations[index],
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

/// Failure to admit bundled source or resolve the authored module graph.
///
/// Semantic collectors retain [`ResolutionError`]; an environmental parser
/// failure must never be represented by an empty set of authored diagnostics.
#[derive(Debug)]
pub enum ModuleGraphError {
    /// The compiler-owned `neocogi.lang` dependency could not be parsed.
    BundledSource(ParseError),
    /// Admitted modules contain source-level semantic errors.
    Resolution(ResolutionError),
}

impl fmt::Display for ModuleGraphError {
    /// Adds bundled-source context while preserving existing semantic rendering.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BundledSource(source) => write!(formatter, "cannot admit bundled neocogi.lang source: {source}"),
            Self::Resolution(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for ModuleGraphError {
    /// Retains the parser/OS cause or complete semantic diagnostic collection.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BundledSource(source) => Some(source),
            Self::Resolution(source) => Some(source),
        }
    }
}

impl From<ResolutionError> for ModuleGraphError {
    /// Keeps semantic-only helpers independent of dependency admission errors.
    fn from(source: ResolutionError) -> Self {
        Self::Resolution(source)
    }
}

/// Admits the bundled dependency, then resolves and fingerprints a module DAG.
/// Worker startup failure is returned before generation or artifact publication;
/// genuine internal worker panics retain the syntax owner's panic behavior.
pub fn resolve_modules(modules: Vec<Module>) -> Result<ResolvedModules, ModuleGraphError> {
    if modules.is_empty() {
        return Err(error(None, "at least one IDL module is required").into());
    }

    let modules = crate::standard::attach(modules)?;
    resolve_semantics(modules).map_err(ModuleGraphError::Resolution)
}

/// Resolves already admitted owned models; only semantic diagnostics enter collectors.
fn resolve_semantics(mut modules: Vec<Module>) -> Result<ResolvedModules, ResolutionError> {
    modules.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.sources.iter().map(SourceFile::path).cmp(right.sources.iter().map(SourceFile::path)))
            .then_with(|| left.sources.iter().map(SourceFile::text).cmp(right.sources.iter().map(SourceFile::text)))
    });
    // Validate authored names and dependencies before templates are removed;
    // unused templates must not hide duplicate identities or ambiguous imports.
    let mut authored = build_entries(modules)?;
    connect_externals_and_imports(&mut authored)?;
    // Enum shape/tag and public method-name rules are independent of type arguments.
    // Validate before specialization so unused templates cannot conceal a bad contract.
    for entry in &authored {
        for declaration in &entry.module.declarations {
            match &declaration.kind {
                DeclarationKind::Enum(enumeration) => validate_enum(entry, declaration, enumeration)?,
                DeclarationKind::BitEnum(bit_enum) => validate_metadata_names(entry, bit_enum)?,
                DeclarationKind::Interface(interface) => validate_method_names(&entry.module, interface)?,
                _ => {}
            }
        }
    }
    let mut diagnostics = bind::bind(&mut authored)?;
    // Stored ownership form is independent of resolving another field's name.
    // Check the same policy before substitution and after closing arguments so
    // a missing declaration never hides an unrelated borrowed-field diagnostic.
    for (index, entry) in authored.iter().enumerate() {
        for declaration in &entry.module.declarations {
            for field in stored_fields(declaration) {
                collect_diagnostics(&mut diagnostics, validate_field_form(index, field.ty, &authored, &field.context, field.span));
            }
        }
    }
    if !diagnostics.is_empty() {
        let sources = authored.iter().flat_map(|entry| entry.module.sources.iter().cloned()).collect::<Vec<_>>();
        return Err(ResolutionError {
            diagnostics: diagnostics.into_iter().map(|diagnostic| diagnostic.with_sources(&sources)).collect(),
        });
    }
    let modules = crate::specialize::expand(authored.into_iter().map(|entry| entry.module).collect())?;
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
        let sources = graph
            .modules
            .iter()
            .flat_map(|module| module.source.sources.iter().cloned())
            .collect::<Vec<_>>();
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
                            .resolve_declaration(name)
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
    /// Authored arity used only by the lexical binding phase.
    parameters: usize,
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
    let owners = modules.iter().map(|module| module.name.to_string()).collect::<Vec<_>>();
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
                    let owner = specialization.template.binding.declaration().expect("bound specialization template").module;
                    format!("{}.{}", owners[owner], declaration.name)
                }
                None => module.name.qualify(&declaration.name),
            };
            let id = declaration
                .id
                .explicit
                .unwrap_or_else(|| derive_declaration_id(&qualified_name, declaration.id.version));
            if matches!(declaration.kind, DeclarationKind::Interface(_)) && id.is_nil() {
                return Err(source_error(
                    &module,
                    "EIDL074",
                    &declaration.span,
                    "a callable interface ID must not be nil",
                    "choose a nonnil explicit ID or a derived versioned identity",
                ));
            }
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
                parameters: declaration.parameters.len(),
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
            let name = import.name;
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
        for field in stored_fields(declaration) {
            collect_diagnostics(&mut diagnostics, validate_field_type(index, field.ty, entries, &field.context, field.span));
        }
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
            }
            DeclarationKind::Enum(_) => {}
            DeclarationKind::BitEnum(bit_enum) => collect_diagnostics(&mut diagnostics, validate_bit_enum(entry, declaration, bit_enum)),
            DeclarationKind::Interface(interface) => {
                collect_diagnostics(&mut diagnostics, validate_interface_bases(index, interface, entries, &declaration.name));
                collect_diagnostics(&mut diagnostics, validate_methods(index, interface, entries, values, &declaration.name));
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

/// Adds every public type emitted by the typed interface backend for one name.
///
/// Resolver and backend consume the core companion inventory directly. Authored
/// interfaces and closed specializations therefore reserve the same namespace
/// without maintaining a second list of generated support names.
fn interface_companions(name: &str, companions: &mut Vec<String>) {
    for companion in NcComInterfaceCompanion::ALL {
        companions.push(format!("{name}{}", companion.suffix()));
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
        occupied
            .entry(import.name.clone())
            .or_insert_with(|| (import.span.clone(), format!("imported name {}", import.name)));
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
        AbiType::ComPtr(_)
        | AbiType::WeakComPtr(_)
        | AbiType::ComBox(_)
        | AbiType::WeakComBox(_)
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

/// Flags share their value's inherent namespace with per-value IDL metadata.
/// Run at authored and closed stages so unused templates cannot hide conflicts.
fn validate_metadata_names(entry: &ModuleEntry, bit_enum: &BitEnum) -> Result<(), ResolutionError> {
    for flag in &bit_enum.flags {
        if flag.name == crate::metadata_names::VALUE_METADATA_MEMBER {
            return Err(source_error(
                &entry.module,
                "EIDL021",
                &flag.span,
                format!("flag {} conflicts with generated per-value metadata", flag.name),
                "choose a distinct flag name",
            ));
        }
    }
    Ok(())
}

/// Checks every bit pattern against the selected unsigned storage width.
fn validate_bit_enum(entry: &ModuleEntry, declaration: &Declaration, bit_enum: &BitEnum) -> Result<(), ResolutionError> {
    validate_metadata_names(entry, bit_enum)?;
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
    if enumeration.dense {
        for variant in &enumeration.variants {
            if let Some(member) = NcComDenseEnumMember::ALL
                .into_iter()
                .find(|member| member.is_emitted(enumeration.names.is_some()) && member.name() == variant.name)
            {
                return Err(source_error(
                    &entry.module,
                    "EIDL021",
                    &variant.span,
                    format!(
                        "enum {} variant {} conflicts with generated dense-enum member {}",
                        declaration.name,
                        variant.name,
                        member.name()
                    ),
                    "this spelling is part of the generated public dense-enum API",
                ));
            }
        }
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
fn validate_method_names(module: &Module, interface: &crate::Interface) -> Result<(), ResolutionError> {
    let prefix = neocogi_com_codegen::InterfaceVTablePrefix::select(!interface.bases.is_empty());
    let diagnostics = interface
        .methods
        .iter()
        .filter(|method| neocogi_com_core::is_reserved_com_method(&method.name) || method.name == prefix.name())
        .map(|method| {
            Diagnostic::new(
                "EIDL100",
                if method.name == prefix.name() {
                    format!("COM method `{}` conflicts with the generated `{}` vtable prefix", method.name, prefix.name())
                } else {
                    format!("COM method `{}` conflicts with a COM handle operation", method.name)
                },
            )
            .with_sources(&module.sources)
            .primary(&method.span, "choose a distinct interface method name")
        })
        .collect::<Vec<_>>();
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(ResolutionError { diagnostics })
    }
}

/// Validates ordinary signatures and generated output getters in vtable order.
fn validate_methods(
    index: usize,
    interface: &crate::Interface,
    entries: &[ModuleEntry],
    values: &crate::ValueGraph,
    owner: &str,
) -> Result<(), ResolutionError> {
    let mut diagnostics = Vec::new();
    collect_diagnostics(&mut diagnostics, validate_method_names(&entries[index].module, interface));
    for method in &interface.methods {
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
            validate_output_type(
                index,
                &method.output,
                entries,
                values,
                &format!("{owner}::{} result", method.name),
                &method.span,
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
pub(crate) fn value_fields(declaration: &Declaration) -> Vec<&AbiType> {
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

/// One stored field use retains source context independently of its type binding.
struct StoredField<'a> {
    /// Authored or substituted value type under this field's storage rules.
    ty: &'a AbiType,
    /// Original field/tuple location, including template provenance.
    span: &'a SourceSpan,
    /// Concrete declaration/variant/field name used in an author diagnostic.
    context: String,
}

/// Enumerates stored fields once for both authored and substituted admission.
fn stored_fields(declaration: &Declaration) -> Vec<StoredField<'_>> {
    match &declaration.kind {
        DeclarationKind::Struct(record) => record
            .fields
            .iter()
            .map(|field| StoredField {
                ty: &field.ty,
                span: &field.span,
                context: format!("struct {} field {}", declaration.name, field.name),
            })
            .collect(),
        DeclarationKind::Enum(enumeration) => enumeration
            .variants
            .iter()
            .flat_map(|variant| {
                let prefix = format!("enum {} variant {} field", declaration.name, variant.name);
                match &variant.fields {
                    VariantFields::Unit => Vec::new(),
                    VariantFields::Tuple(fields) => fields
                        .iter()
                        .enumerate()
                        .map(|(index, ty)| StoredField {
                            ty,
                            span: &variant.span,
                            context: format!("{prefix} {index}"),
                        })
                        .collect(),
                    VariantFields::Named(fields) => fields
                        .iter()
                        .map(|field| StoredField {
                            ty: &field.ty,
                            span: &field.span,
                            context: format!("{prefix} {}", field.name),
                        })
                        .collect(),
                }
            })
            .collect(),
        DeclarationKind::BitEnum(_) | DeclarationKind::Interface(_) => Vec::new(),
    }
}

/// Storage ownership form is independent of declaration lookup and specialization.
fn validate_field_form(index: usize, ty: &AbiType, entries: &[ModuleEntry], context: &str, span: &SourceSpan) -> Result<(), ResolutionError> {
    match ty {
        AbiType::ComBox(_) => Err(source_error(
            &entries[index].module,
            "EIDL060",
            span,
            format!("{context}: exclusive owners are method transfers, not repeatable fields"),
            "use an explicit owning object contract instead of a repeatable value codec",
        )),
        AbiType::Borrowed(_) | AbiType::Slice(_) | AbiType::UnsizedSlice(_) | AbiType::OutputSlice(_) => Err(source_error(
            &entries[index].module,
            "EIDL060",
            span,
            format!("{context} requires an owned field"),
            "use Vec<T> for indirect ownership; borrowed fields cannot escape a call",
        )),
        AbiType::Array { element, .. } | AbiType::Vector(element) => validate_field_form(index, element, entries, context, span),
        _ => Ok(()),
    }
}

/// Validates recursive field syntax independently from method ownership positions.
fn validate_field_type(index: usize, ty: &AbiType, entries: &[ModuleEntry], context: &str, span: &SourceSpan) -> Result<(), ResolutionError> {
    validate_field_form(index, ty, entries, context, span)?;
    match ty {
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => Ok(()),
        AbiType::ComPtr(value) | AbiType::WeakComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value) => {
            validate_object_target(index, value, entries, context, span)
        }
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
        AbiType::Borrowed(_) | AbiType::Slice(_) | AbiType::UnsizedSlice(_) | AbiType::OutputSlice(_) => unreachable!("field form already rejected"),
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
fn validate_value_type(
    index: usize,
    ty: &AbiType,
    entries: &[ModuleEntry],
    values: &crate::ValueGraph,
    context: &str,
    span: &SourceSpan,
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
fn validate_parameter_type(
    index: usize,
    ty: &AbiType,
    entries: &[ModuleEntry],
    values: &crate::ValueGraph,
    context: &str,
    span: &SourceSpan,
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
        AbiType::WeakComBox(_) => Err(source_error(
            &entries[index].module,
            "EIDL061",
            span,
            format!("{context}: weak box inputs must use a live interface borrow"),
            "unsupported ownership transfer",
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

/// Validates outputs with explicit object ownership and copied-value semantics.
fn validate_output_type(
    index: usize,
    ty: &AbiType,
    entries: &[ModuleEntry],
    values: &crate::ValueGraph,
    context: &str,
    span: &SourceSpan,
) -> Result<(), ResolutionError> {
    let span = type_span(ty).unwrap_or(span);
    match ty {
        AbiType::Unit | AbiType::Scalar(_) => Ok(()),
        AbiType::ComPtr(value) | AbiType::WeakComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value) => {
            validate_object_target(index, value, entries, context, span)
        }
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
    if name.is_str() {
        Ok(ResolvedName::Str)
    } else {
        resolve_named(index, name, entries).map(ResolvedName::Symbol)
    }
}

/// Reads a concrete bound projection from the completed symbol table.
/// Lexical lookup happens only in bind::bind, before specialization.
fn resolve_named(index: usize, name: &NamedType, entries: &[ModuleEntry]) -> Result<Symbol, ResolutionError> {
    let key = name.binding.projection().ok_or_else(|| {
        source_error(
            &entries[index].module,
            "EIDL099",
            &name.span,
            "type has no bound concrete projection",
            "compiler binding invariant",
        )
    })?;
    entries[key.module].local_symbols.get(&key.name).copied().ok_or_else(|| {
        source_error(
            &entries[index].module,
            "EIDL099",
            &name.span,
            "bound declaration is absent",
            "compiler projection invariant",
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

/// Exact domain prefix for canonical module ABI fingerprint bytes.
///
/// This compiler/build-time encoding revision is independent of runtime and
/// plugin descriptor discovery ABI v1. Documentation checks use this same prefix;
/// changing source provenance or Cargo name mangling must not change its bytes.
/// Revision 13 removes UI event publication edges from ordinary callable contracts.
pub const MODULE_ABI_ENCODING_DOMAIN: &[u8] = b"neocogi-idl:module-abi:v13\0";

/// Encodes one module into canonical bytes and hashes the result.
fn encode_module(index: usize, entries: &[ModuleEntry], dependencies: &[(usize, [u8; 32])]) -> Result<[u8; 32], ResolutionError> {
    let entry = &entries[index];
    let mut encoder = CanonicalEncoder::new();
    // Templates and concrete declarations share this v13 encoding, including
    // ordinary type dependencies and authored policies. Build/debug locations are absent.
    encoder.bytes(MODULE_ABI_ENCODING_DOMAIN);
    encoder.guid(entry.module_id);
    // Templates and concrete declarations use exactly the same grammar. Bound
    // parameters require a scope, not a second serializer or diagnostic strings.
    encoder.u32(u32_len(entry.module.templates.len(), &entry.name, "template count")?);
    for template in &entry.module.templates {
        let id = template
            .id
            .explicit
            .unwrap_or_else(|| derive_declaration_id(&entry.module.name.qualify(&template.name), template.id.version));
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

/// Lexical meaning of one name before choosing a type or reference encoding.
///
/// Qualified paths are always free. Only a bare identifier can bind to a template
/// parameter, and a bound parameter cannot itself accept generic arguments.
enum NameBinding<'a> {
    /// Zero-based binder position, independent of its authored spelling.
    Parameter {
        /// Checked zero-based position in the enclosing declaration's binder list.
        position: u32,
    },
    /// A declaration reference proven not to denote a lexical parameter.
    Free(FreeName<'a>),
}

/// Borrowed declaration name admitted only by lexical bound/free classification.
///
/// The free encoder takes this proof instead of silently assuming every named
/// reference is qualified. The original AST and source spans remain untouched.
struct FreeName<'a>(
    /// Authored reference that passed the common lexical classifier.
    &'a NamedType,
);

/// Classifies a name identically in ordinary type, base positions.
fn classify_name<'a>(scope: EncodingScope<'_>, name: &'a NamedType, entries: &[ModuleEntry]) -> Result<NameBinding<'a>, ResolutionError> {
    if let Binding::Parameter { position } = name.binding {
        // The binding phase already rejects arguments on parameters. Keep this
        // invariant check for direct internal encoder callers as well.
        if !name.arguments.is_empty() {
            return Err(source_error(
                &entries[scope.module].module,
                "EIDL060",
                &name.span,
                "a type parameter cannot accept arguments",
                "remove these type arguments",
            ));
        }
        return Ok(NameBinding::Parameter { position });
    }
    Ok(NameBinding::Free(FreeName(name)))
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
    let scope = EncodingScope {
        module: index,
        parameters: &declaration.parameters,
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
            encoder.tag(match enumeration.names {
                None => 0,
                Some(crate::EnumNames::SnakeCase) => 1,
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

/// Disjoint alternatives of the recursive type-expression grammar.
/// Explicit values are contract bytes. Rust rejects duplicate discriminants;
/// changing this enum also requires a module encoding revision and regeneration.
/// Values 7 and 8 are reserved for retired ownership forms.
#[repr(u8)]
#[derive(Clone, Copy)]
enum TypeTag {
    /// Successful unit value.
    Unit = 0,
    /// Length-framed built-in scalar name.
    Scalar = 1,
    /// Owned UTF-8 value.
    String = 2,
    /// Named declaration or fixed language capability, with its own discriminator.
    Named = 3,
    /// Call-scoped value borrow.
    Borrowed = 4,
    /// Readable element buffer.
    Slice = 5,
    /// Fixed-length recursively encoded array.
    Array = 6,
    /// Shared strong object capability.
    ComPtr = 9,
    /// Owned dynamic sequence.
    Vector = 10,
    /// Unsized sequence referent.
    UnsizedSlice = 11,
    /// Exclusive object transfer.
    ComBox = 12,
    /// Exclusive weak control token.
    WeakComBox = 13,
    /// Fresh copied output buffer, including initialized-prefix completion.
    OutputSlice = 14,
    /// Shared weak control token.
    WeakComPtr = 15,
    /// Bound template parameter followed by its u32 ordinal; never an output type.
    Parameter = 16,
}

/// Encodes one recursively structured ABI type.
fn encode_type(scope: EncodingScope<'_>, ty: &AbiType, entries: &[ModuleEntry], encoder: &mut CanonicalEncoder) -> Result<(), ResolutionError> {
    match ty {
        AbiType::Unit => encoder.type_tag(TypeTag::Unit),
        AbiType::Scalar(name) => {
            encoder.type_tag(TypeTag::Scalar);
            encoder.text(name, &entries[scope.module].name)?;
        }
        AbiType::String => encoder.type_tag(TypeTag::String),
        AbiType::Named(name) => match classify_name(scope, name, entries)? {
            NameBinding::Parameter { position } => {
                encoder.type_tag(TypeTag::Parameter);
                encoder.u32(position);
            }
            NameBinding::Free(free) => {
                encoder.type_tag(TypeTag::Named);
                if name.is_str() {
                    encoder.tag(0);
                } else if let Some(identity) = builtin_object(name) {
                    // Fixed core capabilities contribute their exact ABI identity.
                    encoder.tag(2);
                    encoder.guid(identity.iid);
                    // SAFETY: built-in signatures point to immutable core constants.
                    encoder.bytes(unsafe { identity.abi_signature.as_digest().expect("static core signature") });
                } else {
                    encoder.tag(1);
                    encode_free_declaration(scope, free, entries, encoder)?;
                }
            }
        },
        AbiType::Borrowed(referent) => {
            encoder.type_tag(TypeTag::Borrowed);
            encode_type(scope, referent, entries, encoder)?;
        }
        AbiType::Slice(element) => {
            encoder.type_tag(TypeTag::Slice);
            encode_type(scope, element, entries, encoder)?;
        }
        AbiType::Array { element, len } => {
            encoder.type_tag(TypeTag::Array);
            encoder.u32(*len);
            encode_type(scope, element, entries, encoder)?;
        }
        AbiType::ComBox(target) | AbiType::WeakComBox(target) => {
            encoder.type_tag(match ty {
                AbiType::ComBox(_) => TypeTag::ComBox,
                AbiType::WeakComBox(_) => TypeTag::WeakComBox,
                _ => unreachable!(),
            });
            encode_type(scope, target, entries, encoder)?;
        }
        AbiType::ComPtr(target) => {
            encoder.type_tag(TypeTag::ComPtr);
            encode_type(scope, target, entries, encoder)?;
        }
        AbiType::WeakComPtr(target) => {
            // New ownership spelling cannot alias strong or exclusive weak transfers.
            encoder.type_tag(TypeTag::WeakComPtr);
            encode_type(scope, target, entries, encoder)?;
        }
        AbiType::Vector(element) => {
            encoder.type_tag(TypeTag::Vector);
            encode_type(scope, element, entries, encoder)?;
        }
        AbiType::UnsizedSlice(element) => {
            encoder.type_tag(TypeTag::UnsizedSlice);
            encode_type(scope, element, entries, encoder)?;
        }
        AbiType::OutputSlice(element) => {
            // Output-only slots include an initialized-prefix count in ABI v1.
            // Their revision must differ from the retired initialized in/out ABI.
            encoder.type_tag(TypeTag::OutputSlice);
            encode_type(scope, element, entries, encoder)?;
        }
    }
    Ok(())
}

/// Encodes a base reference with disjoint bound/free alternatives.
///
/// Unlike an ordinary type expression, this position has no enclosing type tag.
/// Both alternatives therefore have an explicit discriminator; a free UUID must
/// never be mistaken for a parameter ordinal. This grammar is retained from v11.
fn encode_named(scope: EncodingScope<'_>, name: &NamedType, entries: &[ModuleEntry], encoder: &mut CanonicalEncoder) -> Result<(), ResolutionError> {
    match classify_name(scope, name, entries)? {
        NameBinding::Parameter { position } => {
            encoder.tag(0);
            encoder.u32(position);
        }
        NameBinding::Free(free) => {
            encoder.tag(1);
            encode_free_declaration(scope, free, entries, encoder)?;
        }
    }
    Ok(())
}

/// Encodes a free declaration's owning module, identity and recursive arguments.
/// The caller provides lexical classification; this function never interprets binders.
fn encode_free_declaration(
    scope: EncodingScope<'_>,
    free: FreeName<'_>,
    entries: &[ModuleEntry],
    encoder: &mut CanonicalEncoder,
) -> Result<(), ResolutionError> {
    let name = free.0;
    let key = name
        .binding
        .declaration()
        .ok_or_else(|| error(Some(&entries[scope.module].name), "free reference is not bound"))?;
    let module = key.module;
    let declaration = entries[module]
        .module
        .declarations
        .iter()
        .chain(&entries[module].module.templates)
        .find(|declaration| declaration.name == key.name)
        .ok_or_else(|| error(Some(&entries[module].name), "bound free declaration is absent"))?;
    let id = declaration
        .id
        .explicit
        .unwrap_or_else(|| derive_declaration_id(&entries[module].module.name.qualify(&key.name), declaration.id.version));
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

    /// Appends a type-expression alternative; other grammar tags cannot enter here.
    fn type_tag(&mut self, value: TypeTag) {
        self.tag(value as u8);
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
            | "ComPtr"
            | "WeakComPtr"
            | "ComBox"
            | "WeakComBox"
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
        AbiType::ComPtr(value)
        | AbiType::WeakComPtr(value)
        | AbiType::ComBox(value)
        | AbiType::WeakComBox(value)
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
    match name.binding {
        Binding::Intrinsic(Intrinsic::Unknown) => Some(neocogi_com_core::IUnknown::INTERFACE_ID),
        Binding::Intrinsic(Intrinsic::Bytes) => Some(neocogi_com_core::INcBytesAbi::INTERFACE_ID),
        Binding::Intrinsic(Intrinsic::String) => Some(neocogi_com_core::INcStringAbi::INTERFACE_ID),
        _ => None,
    }
}

#[cfg(test)]
mod reference_encoding_tests {
    use super::*;

    /// Pins the named-only grammar independently of its outer module digest.
    /// Free UUID bytes cannot impersonate a bound ordinal: both cases have tags.
    #[test]
    fn named_references_discriminate_free_identity_and_parameter_positions() {
        let source = crate::parse_module(std::path::Path::new("proof/codec.ncidl"), "#[version(1)] interface B {}").unwrap();
        let entries = build_entries(vec![source]).unwrap();
        let parameters = vec!["A".to_owned(), "B".to_owned()];
        let scope = EncodingScope { module: 0, parameters: &parameters };
        let name = |segments: &[&str]| NamedType {
            span: entries[0].module.declarations[0].span.clone(),
            segments: segments.iter().map(|value| (*value).to_owned()).collect(),
            arguments: Vec::new(),
            binding: if segments.len() == 1 {
                Binding::Parameter {
                    position: parameters.iter().position(|parameter| parameter == segments[0]).unwrap() as u32,
                }
            } else {
                Binding::Declaration(DeclarationKey { module: 0, name: "B".to_owned() })
            },
        };
        for (position, parameter) in ["A", "B"].into_iter().enumerate() {
            let mut reference = CanonicalEncoder::new();
            encode_named(scope, &name(&[parameter]), &entries, &mut reference).unwrap();
            assert_eq!(reference.bytes, [vec![0], (position as u32).to_be_bytes().to_vec()].concat());
            let mut ordinary = CanonicalEncoder::new();
            encode_type(scope, &AbiType::Named(name(&[parameter])), &entries, &mut ordinary).unwrap();
            assert_eq!(ordinary.bytes, [vec![16], (position as u32).to_be_bytes().to_vec()].concat());
        }
        let mut free = CanonicalEncoder::new();
        encode_named(scope, &name(&["proof", "codec", "B"]), &entries, &mut free).unwrap();
        let mut expected = vec![1];
        expected.extend_from_slice(&entries[0].module_id.to_uuid_bytes());
        expected.extend_from_slice(&entries[0].declaration_identities[0].id.to_uuid_bytes());
        expected.extend_from_slice(&0u32.to_be_bytes());
        assert_eq!(free.bytes, expected);
        let mut invalid = name(&["B"]);
        invalid.arguments.push(AbiType::Scalar("u32".to_owned()));
        let error = encode_named(scope, &invalid, &entries, &mut CanonicalEncoder::new()).unwrap_err();
        assert_eq!(error.diagnostics[0].code, "EIDL060");
        assert!(error.to_string().contains("proof/codec.ncidl"));
    }

    /// Every recursive type alternative starts with a distinct byte, regardless
    /// of its payload length or a following sibling's bytes.
    #[test]
    fn type_expression_prefixes_are_disjoint() {
        let source = crate::parse_module(std::path::Path::new("proof/tags.ncidl"), "#[version(1)] interface Item {}").unwrap();
        let entries = build_entries(vec![source]).unwrap();
        let parameters = vec!["T".to_owned()];
        let scope = EncodingScope { module: 0, parameters: &parameters };
        let unit = || Box::new(AbiType::Unit);
        let named = |binding| {
            AbiType::Named(NamedType {
                span: entries[0].module.declarations[0].span.clone(),
                segments: vec!["diagnostic_only".into()],
                arguments: Vec::new(),
                binding,
            })
        };
        let alternatives = [
            (AbiType::Unit, 0),
            (AbiType::Scalar("u8".into()), 1),
            (AbiType::String, 2),
            (named(Binding::Declaration(DeclarationKey { module: 0, name: "Item".into() })), 3),
            (AbiType::Borrowed(unit()), 4),
            (AbiType::Slice(unit()), 5),
            (AbiType::Array { element: unit(), len: 0 }, 6),
            (AbiType::ComPtr(unit()), 9),
            (AbiType::Vector(unit()), 10),
            (AbiType::UnsizedSlice(unit()), 11),
            (AbiType::ComBox(unit()), 12),
            (AbiType::WeakComBox(unit()), 13),
            (AbiType::OutputSlice(unit()), 14),
            (AbiType::WeakComPtr(unit()), 15),
            (named(Binding::Parameter { position: 0 }), 16),
        ];
        let mut prefixes = std::collections::BTreeSet::new();
        for (ty, expected) in alternatives {
            let mut encoder = CanonicalEncoder::new();
            encode_type(scope, &ty, &entries, &mut encoder).unwrap();
            assert_eq!(encoder.bytes[0], expected);
            assert!(prefixes.insert(encoder.bytes[0]));
        }
    }
}
