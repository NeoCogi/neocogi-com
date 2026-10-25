//! Closed declaration specialization before ordinary COM ABI validation.
//!
//! Templates are substituted into concrete values and interfaces in the using module. Their
//! identities depend on the template and argument identities, never on the using
//! module or a Rust crate path. Ordinary resolution then validates the resulting
//! methods, inheritance, borrowed values, and threading exactly once.

use std::collections::HashMap;
use crate::binding::{Binding, DeclarationKey, SemanticOrigin, semantic_origin};
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
                    // Lexical ownership is already bound. Open parameters stay
                    // positional; substitution never resolves a source spelling.
                    substitute_declaration(&mut declaration, &[])?;
                    expansion.modules[index].templates.push(declaration);
                }
                _ => {
                    substitute_declaration(&mut declaration, &[])?;
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
        expansion.modules[index].imports.retain(|import| {
            !originals
                .iter()
                .find(|module| module.name.to_string() == import.module)
                .and_then(|owner| owner.declaration(&import.name))
                .is_some_and(is_template)
        });
        expansion.modules[index].declarations.sort_by(|left, right| left.name.cmp(&right.name));
        expansion.modules[index].templates.sort_by(|left, right| left.name.cmp(&right.name));
        expansion.modules[index].externals.sort_by(|left, right| left.module.cmp(&right.module));
    }
    Ok(expansion.modules)
}

/// Mutable compiler worklist; source declarations remain immutable during substitution.
struct Expansion {
    /// Immutable bound source declarations; lexical lookup has already finished.
    originals: Vec<Module>,
    /// Concrete modules accumulated for ordinary ABI validation.
    modules: Vec<Module>,
    /// One local projection per canonical specialization, including active recursion.
    instances: HashMap<(usize, NcGuid), Instance>,
}

/// A reserved projection, including an active recursive instantiation. Its
/// finite origin is available before field lowering can reenter the cache.
struct Instance {
    /// Concrete declaration in the consuming module.
    key: DeclarationKey,
    /// Full bound meaning; equal UUIDs alone do not authorize reuse.
    origin: SemanticOrigin,
    /// First request, retained to diagnose both sides of an ambiguity.
    span: SourceSpan,
}

impl Expansion {
    /// Selects a previously bound source declaration, without import/path lookup.
    fn declaration(&self, name: &NamedType) -> Result<(usize, &Declaration), ResolutionError> {
        let key = name.binding.declaration().ok_or_else(|| failure(&name.span, "expected a bound declaration"))?;
        let declaration = self.originals[key.module]
            .declaration(&key.name)
            .or_else(|| self.modules[key.module].declaration(&key.name))
            .ok_or_else(|| failure(&name.span, "bound declaration is absent"))?;
        Ok((key.module, declaration))
    }

    /// Creates concrete references and records template-introduced dependencies.
    fn lower_type(&mut self, target: usize, ty: &mut AbiType, depth: usize) -> Result<(), ResolutionError> {
        match ty {
            AbiType::Named(name) => {
                if !name.arguments.is_empty() {
                    *name = self.instantiate(target, name.clone(), depth + 1)?;
                } else if let Some(key) = name.binding.declaration() {
                    let dependency = self.modules[key.module].name.to_string();
                    self.add_dependency(target, &dependency, &name.span);
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
        let identity = self.type_identity(&AbiType::Named(name.clone()))?;
        let origin = semantic_origin(&AbiType::Named(name.clone()))?;
        if let Some(instance) = self.instances.get(&(target, identity)) {
            if instance.origin != origin {
                let sources = self.originals.iter().flat_map(|module| module.sources.iter().cloned()).collect::<Vec<_>>();
                return Err(ResolutionError {
                    diagnostics: vec![
                        Diagnostic::new("EIDL060", format!("specialization UUID {identity} has conflicting semantic origins"))
                            .with_sources(&sources)
                            .primary(&name.span, format!("conflicting request: {origin:?}"))
                            .secondary(&instance.span, format!("first request: {:?}", instance.origin)),
                    ],
                });
            }
            return Ok(projected(name.span, instance.key.clone()));
        }
        if depth > MAX_DEPTH || self.instances.len() >= MAX_INSTANTIATIONS {
            return Err(failure(&name.span, "type specialization exceeds the compiler depth or instance limit"));
        }
        let (owner, declaration) = self.declaration(&name)?;
        let module_name = self.originals[owner].name.to_string();
        let mut declaration = declaration.clone();
        let substitutions = name.arguments.clone();
        // Mangle the closed nominal type identity, not its display spelling.
        // The UUID suffix cannot be demangled; generation documents the full
        // template and argument spelling beside the resulting Rust declaration.
        let suffix = identity.to_uuid_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let local = format!("{}Instance{suffix}", declaration.name);
        if self.originals[target].declaration(&local).is_some() {
            return Err(failure(&name.span, "generated specialization name collides with an authored declaration"));
        }
        let key = DeclarationKey { module: target, name: local.clone() };
        self.instances.insert(
            (target, identity),
            Instance {
                key: key.clone(),
                origin,
                span: name.span.clone(),
            },
        );
        substitute_declaration(&mut declaration, &substitutions)?;
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
        for argument in &mut declaration.specialization.as_mut().unwrap().arguments {
            self.project_argument(target, argument, depth)?;
        }
        self.add_dependency(target, &module_name, &name.span);
        self.lower_declaration(target, &mut declaration, depth)?;
        self.modules[target].declarations.push(declaration);
        Ok(projected(name.span, key))
    }

    /// Retains an argument's canonical expression together with its actual local
    /// projection. Phantom arguments need this provenance just as stored fields do.
    fn project_argument(&mut self, target: usize, ty: &mut AbiType, depth: usize) -> Result<(), ResolutionError> {
        match ty {
            AbiType::Named(name) => {
                if matches!(name.binding, Binding::Application { .. }) {
                    let closed = self.instantiate(target, name.clone(), depth + 1)?;
                    let key = closed.binding.projection().expect("materialized argument").clone();
                    let Binding::Application { projection, .. } = &mut name.binding else {
                        unreachable!()
                    };
                    *projection = Some(key);
                } else if let Some(key) = name.binding.declaration() {
                    self.add_dependency(target, &self.modules[key.module].name.to_string(), &name.span);
                }
                for argument in &mut name.arguments {
                    self.project_argument(target, argument, depth)?;
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
            | AbiType::Array { element: value, .. } => self.project_argument(target, value, depth)?,
            AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
        }
        Ok(())
    }

    /// Hashes semantic type identities; import spelling and the consumer module are excluded.
    fn type_identity(&self, ty: &AbiType) -> Result<NcGuid, ResolutionError> {
        let mut bytes = Vec::new();
        match ty {
            AbiType::Named(name) => {
                if let Some(identity) = crate::resolve::builtin_object(name) {
                    return Ok(identity.iid);
                }
                if name.is_str() {
                    return Ok(guid_from_qualified_name("neocogi.idl.str"));
                }
                let (owner, declaration) = self.declaration(name)?;
                let module = &self.originals[owner];
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
            AbiType::ComPtr(value)
            | AbiType::WeakComPtr(value)
            | AbiType::ComBox(value)
            | AbiType::WeakComBox(value)
            | AbiType::Borrowed(value)
            | AbiType::Vector(value)
            | AbiType::UnsizedSlice(value)
            | AbiType::Slice(value)
            | AbiType::OutputSlice(value)
            | AbiType::Array { element: value, .. } => {
                bytes.extend_from_slice(match ty {
                    AbiType::ComPtr(_) => b"com_ptr",
                    AbiType::WeakComPtr(_) => b"weak_com_ptr",
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

/// Creates the concrete named view of one already materialized projection.
fn projected(span: SourceSpan, key: DeclarationKey) -> NamedType {
    NamedType {
        span,
        segments: vec![key.name.clone()],
        arguments: Vec::new(),
        binding: Binding::Declaration(key),
    }
}

/// Substitutes bound positions while preserving every free declaration owner.
fn substitute_type(ty: &mut AbiType, substitutions: &[AbiType]) -> Result<(), ResolutionError> {
    match ty {
        AbiType::Named(name) => {
            if let Binding::Parameter { position } = name.binding {
                if let Some(value) = substitutions.get(position as usize) {
                    *ty = value.clone();
                    return Ok(());
                }
            }
            for argument in &mut name.arguments {
                substitute_type(argument, substitutions)?;
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
        | AbiType::Array { element: value, .. } => substitute_type(value, substitutions)?,
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
    }
    // A closed &T where T is Slice<U> uses the ordinary input-buffer convention.
    if let AbiType::Borrowed(value) = ty {
        if let AbiType::UnsizedSlice(element) = value.as_ref() {
            *ty = AbiType::Slice(element.clone());
        }
    }
    Ok(())
}

/// Fields, bases share the same positional substitution.
fn substitute_declaration(declaration: &mut Declaration, substitutions: &[AbiType]) -> Result<(), ResolutionError> {
    visit_types(declaration, &mut |ty| substitute_type(ty, substitutions))
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

/// Preserves the named-only constraint on interface projections.
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
            if name.is_str() {
                return Ok(crate::sha256::digest(b"neocogi.idl.str"));
            }
            let key = name.binding.declaration().ok_or_else(|| failure(&name.span, "generic argument is not bound"))?;
            let module = &graph.modules()[key.module];
            let local = &key.name;
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
        AbiType::ComPtr(value)
        | AbiType::WeakComPtr(value)
        | AbiType::ComBox(value)
        | AbiType::WeakComBox(value)
        | AbiType::Borrowed(value)
        | AbiType::Vector(value)
        | AbiType::UnsizedSlice(value)
        | AbiType::Slice(value)
        | AbiType::OutputSlice(value)
        | AbiType::Array { element: value, .. } => {
            bytes.extend_from_slice(match ty {
                AbiType::ComPtr(_) => b"com_ptr",
                AbiType::WeakComPtr(_) => b"weak_com_ptr",
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
