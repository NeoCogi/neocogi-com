//! Closed declaration specialization before ordinary COM ABI validation.
//!
//! Templates are substituted into concrete values and interfaces in the using module. Their
//! identities depend on the template and argument identities, never on the using
//! module or a Rust crate path. Ordinary resolution then validates the resulting
//! methods, inheritance, borrowed values, and threading exactly once.

use std::collections::{BTreeMap, HashMap};
use neocogi_com_core::{NcGuid, guid_from_name, guid_from_qualified_name};
use crate::{
    AbiType, Declaration, DeclarationKind, DeclarationId, Diagnostic, ExternalModule, Module, NamedType, ResolutionError, SourceSpan, Specialization,
    VariantFields,
};

/// Stops expanding templates that recursively manufacture new argument types.
const MAX_DEPTH: usize = 64;
/// Bounds compiler memory independently of recursion depth.
const MAX_INSTANTIATIONS: usize = 4096;

/// Expands all used struct, enum, and interface templates while preserving their source contracts.
pub(crate) fn expand(modules: Vec<Module>) -> Result<Vec<Module>, ResolutionError> {
    if !modules.iter().flat_map(|module| &module.declarations).any(|declaration| {
        if is_template(declaration) {
            return true;
        }
        let mut copy = declaration.clone();
        let mut generic = false;
        let _ = visit_types(&mut copy, &mut |ty| {
            generic |= has_arguments(ty);
            Ok(())
        });
        generic
    }) {
        return Ok(modules);
    }
    let originals = modules.clone();
    let mut expansion = Expansion {
        originals,
        modules,
        instances: HashMap::new(),
    };
    for index in 0..expansion.modules.len() {
        let declarations = std::mem::take(&mut expansion.modules[index].declarations);
        for mut declaration in declarations {
            match &declaration.kind {
                _ if is_template(&declaration) => {
                    // Canonicalize free names now so imports and aliases do not
                    // change the template fingerprint or instantiated meaning.
                    let parameters = declaration
                        .parameters
                        .iter()
                        .map(|name| {
                            (
                                name.clone(),
                                AbiType::Named(NamedType {
                                    span: declaration.span.clone(),
                                    segments: vec![name.clone()],
                                    arguments: Vec::new(),
                                }),
                            )
                        })
                        .collect();
                    expansion.normalize_declaration(index, &mut declaration, &parameters)?;
                    expansion.modules[index].templates.push(declaration);
                }
                _ => {
                    expansion.normalize_declaration(index, &mut declaration, &BTreeMap::new())?;
                    expansion.lower_declaration(index, &mut declaration, 0)?;
                    expansion.modules[index].declarations.push(declaration);
                }
            }
        }
    }
    // Template names do not denote runtime interfaces. Closed references have
    // already become local declarations, so their source imports are consumed.
    for index in 0..expansion.modules.len() {
        let originals = &expansion.originals;
        for import in &mut expansion.modules[index].imports {
            if let Some(owner) = originals.iter().find(|module| module.name.to_string() == import.module) {
                import.names.retain(|name| !owner.declaration(name).is_some_and(is_template));
            }
        }
        expansion.modules[index].imports.retain(|import| !import.names.is_empty());
        expansion.modules[index].declarations.sort_by(|left, right| left.name.cmp(&right.name));
        expansion.modules[index].templates.sort_by(|left, right| left.name.cmp(&right.name));
        expansion.modules[index].externals.sort_by(|left, right| left.module.cmp(&right.module));
    }
    Ok(expansion.modules)
}

/// Mutable compiler worklist; source declarations remain immutable during substitution.
struct Expansion {
    /// Original names, imports, and templates used for lexical resolution.
    originals: Vec<Module>,
    /// Concrete modules accumulated for ordinary ABI validation.
    modules: Vec<Module>,
    /// One local projection per canonical specialization, including active recursion.
    instances: HashMap<(usize, String), String>,
}

impl Expansion {
    /// Resolves an authored path only through its declared module dependencies.
    fn lookup(&self, owner: usize, name: &NamedType) -> Result<(usize, &Declaration), ResolutionError> {
        let module = &self.originals[owner];
        let local = name.segments.last().ok_or_else(|| failure(&name.span, "empty type path"))?;
        let module_name = if name.segments.len() == 1 {
            if module.declaration(local).is_some() {
                module.name.to_string()
            } else {
                module
                    .imports
                    .iter()
                    .find(|import| import.names.contains(local))
                    .map(|import| import.module.clone())
                    .ok_or_else(|| failure(&name.span, format!("unknown type {local}")))?
            }
        } else {
            name.segments[..name.segments.len() - 1].join(".")
        };
        if module_name != module.name.to_string() && !module.externals.iter().any(|external| external.module == module_name) {
            return Err(failure(
                &name.span,
                format!("type {} requires an explicit extern {module_name}", name.rust_path()),
            ));
        }
        let index = self
            .originals
            .iter()
            .position(|module| module.name.to_string() == module_name)
            .ok_or_else(|| failure(&name.span, format!("unknown module {module_name}")))?;
        let declaration = self.originals[index]
            .declaration(local)
            .ok_or_else(|| failure(&name.span, format!("unknown declaration {module_name}.{local}")))?;
        Ok((index, declaration))
    }

    /// Qualifies free names and substitutes parameters without expanding concrete declarations yet.
    fn normalize_type(&self, owner: usize, ty: &mut AbiType, substitutions: &BTreeMap<String, AbiType>) -> Result<(), ResolutionError> {
        match ty {
            AbiType::Named(name) => {
                if crate::resolve::builtin_object(name).is_some() || name.segments.as_slice() == ["str"] {
                    if !name.arguments.is_empty() {
                        return Err(failure(&name.span, "built-in types do not accept type arguments"));
                    }
                    return Ok(());
                }
                if name.segments.len() == 1 {
                    if let Some(value) = substitutions.get(&name.segments[0]) {
                        if !name.arguments.is_empty() {
                            return Err(failure(&name.span, "a type parameter cannot accept arguments"));
                        }
                        *ty = value.clone();
                        return Ok(());
                    }
                }
                let (index, declaration) = self.lookup(owner, name)?;
                let expected = declaration.parameters.len();
                if name.arguments.len() != expected {
                    return Err(failure(
                        &name.span,
                        format!("{} expects {expected} type arguments, received {}", declaration.name, name.arguments.len()),
                    ));
                }
                name.segments = self.originals[index]
                    .name
                    .to_string()
                    .split('.')
                    .map(str::to_owned)
                    .chain([declaration.name.clone()])
                    .collect();
                for argument in &mut name.arguments {
                    self.normalize_type(owner, argument, substitutions)?;
                }
            }
            AbiType::ComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value)
            | AbiType::Borrowed(value)
            | AbiType::Vector(value)
            | AbiType::UnsizedSlice(value)
            | AbiType::Slice(value)
            | AbiType::OutputSlice(value)
            | AbiType::Array { element: value, .. } => {
                self.normalize_type(owner, value, substitutions)?;
            }
            AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
        }
        // A generic T may close to Slice<U>; normalize &T just like &[U].
        if let AbiType::Borrowed(value) = ty {
            if let AbiType::UnsizedSlice(element) = value.as_ref() {
                *ty = AbiType::Slice(element.clone());
            }
        }
        Ok(())
    }

    /// Applies the same substitution traversal to fields, bases, methods, and results.
    fn normalize_declaration(&self, owner: usize, declaration: &mut Declaration, substitutions: &BTreeMap<String, AbiType>) -> Result<(), ResolutionError> {
        visit_types(declaration, &mut |ty| self.normalize_type(owner, ty, substitutions))
    }

    /// Creates concrete references and records template-introduced dependencies.
    fn lower_type(&mut self, target: usize, ty: &mut AbiType, depth: usize) -> Result<(), ResolutionError> {
        match ty {
            AbiType::Named(name) => {
                if !name.arguments.is_empty() {
                    *name = self.instantiate(target, name.clone(), depth + 1)?;
                } else if name.segments.len() > 1 {
                    let dependency = name.segments[..name.segments.len() - 1].join(".");
                    if dependency == self.modules[target].name.to_string() {
                        name.segments = vec![name.segments.last().unwrap().clone()];
                    } else {
                        self.add_dependency(target, &dependency, &name.span);
                    }
                }
            }
            AbiType::ComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value)
            | AbiType::Borrowed(value)
            | AbiType::Vector(value)
            | AbiType::UnsizedSlice(value)
            | AbiType::Slice(value)
            | AbiType::OutputSlice(value)
            | AbiType::Array { element: value, .. } => {
                self.lower_type(target, value, depth)?;
            }
            AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
        }
        Ok(())
    }

    /// Lowers one fully substituted declaration; all subsequent checks use existing ABI rules.
    fn lower_declaration(&mut self, target: usize, declaration: &mut Declaration, depth: usize) -> Result<(), ResolutionError> {
        visit_types(declaration, &mut |ty| self.lower_type(target, ty, depth))
    }

    /// Adds a lexical dependency needed by an instantiated template's method body.
    fn add_dependency(&mut self, target: usize, name: &str, span: &SourceSpan) {
        let module = &mut self.modules[target];
        if module.name.to_string() != name && !module.externals.iter().any(|external| external.module == name) {
            module.externals.push(ExternalModule { span: span.clone(), module: name.into() });
        }
    }

    /// Materializes a closed value or interface once in the consumer's module.
    fn instantiate(&mut self, target: usize, mut name: NamedType, depth: usize) -> Result<NamedType, ResolutionError> {
        let key = name.rust_path();
        if let Some(local) = self.instances.get(&(target, key.clone())) {
            return Ok(NamedType {
                span: name.span,
                segments: vec![local.clone()],
                arguments: Vec::new(),
            });
        }
        if depth > MAX_DEPTH || self.instances.len() >= MAX_INSTANTIATIONS {
            return Err(failure(&name.span, "type specialization exceeds the compiler depth or instance limit"));
        }
        let module_name = name.segments[..name.segments.len() - 1].join(".");
        let owner = self
            .originals
            .iter()
            .position(|module| module.name.to_string() == module_name)
            .ok_or_else(|| failure(&name.span, "missing template module"))?;
        let mut declaration = self.originals[owner]
            .declaration(name.segments.last().unwrap())
            .ok_or_else(|| failure(&name.span, "missing declaration template"))?
            .clone();
        let substitutions = declaration
            .parameters
            .iter()
            .cloned()
            .zip(name.arguments.iter().cloned())
            .collect::<BTreeMap<_, _>>();
        let identity = self.type_identity(&AbiType::Named(name.clone()))?;
        let suffix = identity.to_uuid_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let local = format!("{}Instance{suffix}", declaration.name);
        if self.originals[target].declaration(&local).is_some() {
            return Err(failure(&name.span, "generated specialization name collides with an authored declaration"));
        }
        self.instances.insert((target, key), local.clone());
        self.normalize_declaration(owner, &mut declaration, &substitutions)?;
        let arguments = std::mem::take(&mut name.arguments);
        declaration.specialization = Some(Specialization { template: name.clone(), arguments });
        declaration.name = local.clone();
        declaration.id = DeclarationId {
            version: declaration.id.version,
            explicit: Some(identity),
        };
        declaration.span = name.span.clone();
        declaration.parameters.clear();
        // Materialize nested arguments even when the template does not store
        // them. Adapters and inspection can then resolve the same closed type.
        for mut argument in declaration.specialization.as_ref().unwrap().arguments.clone() {
            self.lower_type(target, &mut argument, depth)?;
        }
        self.add_dependency(target, &module_name, &name.span);
        self.lower_declaration(target, &mut declaration, depth)?;
        self.modules[target].declarations.push(declaration);
        Ok(NamedType {
            span: name.span,
            segments: vec![local],
            arguments: Vec::new(),
        })
    }

    /// Hashes semantic type identities; import spelling and the consumer module are excluded.
    fn type_identity(&self, ty: &AbiType) -> Result<NcGuid, ResolutionError> {
        let mut bytes = Vec::new();
        match ty {
            AbiType::Named(name) => {
                if let Some(identity) = crate::resolve::builtin_object(name) {
                    return Ok(identity.iid);
                }
                if name.segments.as_slice() == ["str"] {
                    return Ok(guid_from_qualified_name("neocogi.idl.str"));
                }
                let module_name = name.segments[..name.segments.len() - 1].join(".");
                let module = self
                    .originals
                    .iter()
                    .find(|module| module.name.to_string() == module_name)
                    .ok_or_else(|| failure(&name.span, "generic argument has no owning module"))?;
                let declaration = module
                    .declaration(name.segments.last().unwrap())
                    .ok_or_else(|| failure(&name.span, "unknown generic argument"))?;
                let id = declaration
                    .id
                    .explicit
                    .unwrap_or_else(|| crate::resolve::derive_declaration_id(&module.name.qualify(&declaration.name), declaration.id.version));
                if name.arguments.is_empty() {
                    return Ok(id);
                }
                bytes.extend_from_slice(&id.to_uuid_bytes());
                for argument in &name.arguments {
                    bytes.extend_from_slice(&self.type_identity(argument)?.to_uuid_bytes());
                }
            }
            AbiType::ComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value)
            | AbiType::Borrowed(value)
            | AbiType::Vector(value)
            | AbiType::UnsizedSlice(value)
            | AbiType::Slice(value)
            | AbiType::OutputSlice(value)
            | AbiType::Array { element: value, .. } => {
                bytes.extend_from_slice(match ty {
                    AbiType::ComPtr(_) => b"com_ptr",
                    AbiType::ComBox(_) => b"com_box",
                    AbiType::WeakComBox(_) => b"weak_com_box",
                    AbiType::Borrowed(_) => b"borrow",
                    AbiType::Vector(_) => b"vector",
                    AbiType::UnsizedSlice(_) => b"unsized_slice",
                    AbiType::Slice(_) => b"slice",
                    AbiType::OutputSlice(_) => b"output",
                    _ => b"array",
                });
                if let AbiType::Array { len, .. } = ty {
                    bytes.extend_from_slice(&len.to_be_bytes());
                }
                bytes.extend_from_slice(&self.type_identity(value)?.to_uuid_bytes());
            }
            _ => bytes.extend_from_slice(ty.source_name().as_bytes()),
        }
        Ok(guid_from_name(guid_from_qualified_name("neocogi.idl.closed-type:v1"), &bytes))
    }
}

/// Identifies reusable declarations before their parameters are substituted.
fn is_template(declaration: &Declaration) -> bool {
    !declaration.parameters.is_empty()
}

/// Visits each type root once; recursive type structure belongs to the supplied operation.
pub(crate) fn visit_types(declaration: &mut Declaration, visit: &mut impl FnMut(&mut AbiType) -> Result<(), ResolutionError>) -> Result<(), ResolutionError> {
    match &mut declaration.kind {
        DeclarationKind::Struct(record) => {
            for field in &mut record.fields {
                visit(&mut field.ty)?;
            }
        }
        DeclarationKind::Enum(enumeration) => {
            for variant in &mut enumeration.variants {
                match &mut variant.fields {
                    VariantFields::Unit => {}
                    VariantFields::Tuple(fields) => {
                        for field in fields {
                            visit(field)?;
                        }
                    }
                    VariantFields::Named(fields) => {
                        for field in fields {
                            visit(&mut field.ty)?;
                        }
                    }
                }
            }
        }
        DeclarationKind::BitEnum(_) => {}
        DeclarationKind::Interface(interface) => {
            for base in &mut interface.bases {
                visit_named(base, visit)?;
            }
            for method in &mut interface.methods {
                for parameter in &mut method.parameters {
                    visit(&mut parameter.ty)?;
                }
                visit(&mut method.output)?;
            }
        }
    }
    Ok(())
}

/// Preserves the named-only constraint on interface projections and legacy ports.
fn visit_named(name: &mut NamedType, visit: &mut impl FnMut(&mut AbiType) -> Result<(), ResolutionError>) -> Result<(), ResolutionError> {
    let mut ty = AbiType::Named(name.clone());
    visit(&mut ty)?;
    match ty {
        AbiType::Named(value) => {
            *name = value;
            Ok(())
        }
        _ => Err(failure(&name.span, "an interface base must resolve to a named interface")),
    }
}

/// Reports specialization failures at the author's concrete use site.
fn failure(span: &SourceSpan, message: impl Into<String>) -> ResolutionError {
    ResolutionError {
        diagnostics: vec![Diagnostic::new("EIDL060", message).primary(span, "invalid type specialization")],
    }
}

/// Detects generic references without changing ordinary nongeneric diagnostic behavior.
fn has_arguments(ty: &AbiType) -> bool {
    match ty {
        AbiType::Named(name) => !name.arguments.is_empty(),
        AbiType::ComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value)
        | AbiType::Borrowed(value)
        | AbiType::Vector(value)
        | AbiType::UnsizedSlice(value)
        | AbiType::Slice(value)
        | AbiType::OutputSlice(value)
        | AbiType::Array { element: value, .. } => has_arguments(value),
        _ => false,
    }
}

/// Computes a closed contract signature from its template and exact argument revisions.
/// The using module is deliberately absent, allowing independently built consumers
/// to negotiate the same specialization without sharing generated Rust definitions.
pub(crate) fn signature(graph: &crate::ResolvedModules, specialization: &Specialization) -> Result<[u8; 32], ResolutionError> {
    let mut name = specialization.template.clone();
    name.arguments = specialization.arguments.clone();
    type_signature(graph, &AbiType::Named(name))
}

/// Recursively fingerprints the concrete ABI arguments and their declaring modules.
fn type_signature(graph: &crate::ResolvedModules, ty: &AbiType) -> Result<[u8; 32], ResolutionError> {
    let mut bytes = Vec::from(b"neocogi-idl:specialized-abi:v1\0".as_slice());
    match ty {
        AbiType::Named(name) => {
            if let Some(identity) = crate::resolve::builtin_object(name) {
                // SAFETY: built-in interface signatures refer to permanent core constants.
                return Ok(crate::sha256::digest(unsafe {
                    identity.abi_signature.as_digest().expect("static core signature")
                }));
            }
            if name.segments.as_slice() == ["str"] {
                return Ok(crate::sha256::digest(b"neocogi.idl.str"));
            }
            let module_name = name.segments[..name.segments.len() - 1].join(".");
            let module = graph.module(&module_name).ok_or_else(|| failure(&name.span, "generic type module is absent"))?;
            let local = name.segments.last().unwrap();
            if name.arguments.is_empty() {
                return module
                    .declaration(local)
                    .map(|identity| identity.abi_signature)
                    .ok_or_else(|| failure(&name.span, "generic argument declaration is absent"));
            }
            let template = module
                .source
                .templates
                .iter()
                .find(|template| &template.name == local)
                .ok_or_else(|| failure(&name.span, "declaration template is absent"))?;
            let id = template
                .id
                .explicit
                .unwrap_or_else(|| crate::resolve::derive_declaration_id(&module.source.name.qualify(local), template.id.version));
            bytes.extend_from_slice(&module.abi_fingerprint);
            bytes.extend_from_slice(&id.to_uuid_bytes());
            for argument in &name.arguments {
                bytes.extend_from_slice(&type_signature(graph, argument)?);
            }
        }
        AbiType::ComPtr(value) | AbiType::ComBox(value) | AbiType::WeakComBox(value)
        | AbiType::Borrowed(value)
        | AbiType::Vector(value)
        | AbiType::UnsizedSlice(value)
        | AbiType::Slice(value)
        | AbiType::OutputSlice(value)
        | AbiType::Array { element: value, .. } => {
            bytes.extend_from_slice(match ty {
                AbiType::ComPtr(_) => b"com_ptr",
                    AbiType::ComBox(_) => b"com_box",
                    AbiType::WeakComBox(_) => b"weak_com_box",
                AbiType::Borrowed(_) => b"borrow",
                AbiType::Vector(_) => b"vector",
                AbiType::UnsizedSlice(_) => b"unsized_slice",
                AbiType::Slice(_) => b"slice",
                AbiType::OutputSlice(_) => b"output",
                _ => b"array",
            });
            if let AbiType::Array { len, .. } = ty {
                bytes.extend_from_slice(&len.to_be_bytes());
            }
            bytes.extend_from_slice(&type_signature(graph, value)?);
        }
        _ => bytes.extend_from_slice(ty.source_name().as_bytes()),
    }
    Ok(crate::sha256::digest(&bytes))
}
