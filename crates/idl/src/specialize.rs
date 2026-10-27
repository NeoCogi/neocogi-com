//! Closed declaration specialization before ordinary COM ABI validation.
//!
//! Templates are substituted into concrete values and interfaces in the using module. Their
//! identities depend on the template and argument identities, never on the using
//! module or a Rust crate path. Ordinary resolution then validates the resulting
//! methods, inheritance, borrowed values, and threading exactly once.

use std::collections::HashMap;
use crate::binding::{Binding, DeclarationKey, SemanticOrigin, semantic_origin_named};

pub(crate) mod budget;
use budget::{Budget, Descent, Limits};
#[cfg(test)]
mod tests;
use neocogi_com_core::{NcGuid, guid_from_name, guid_from_qualified_name};
use crate::{
    AbiType, Declaration, DeclarationKind, DeclarationId, Diagnostic, ExternalModule, Module, NamedType, ResolutionError, SourceSpan, Specialization,
    VariantFields,
};

/// Expands one admitted batch with one private compiler allowance.
pub(crate) fn expand(modules: Vec<Module>, limits: Limits) -> Result<Vec<Module>, ResolutionError> {
    // This immutable copy is authored source, not manufactured expansion.
    let originals = modules.clone();
    let mut expansion = Expansion {
        originals,
        modules,
        instances: HashMap::new(),
        budget: Budget::new(limits),
    };
    for index in 0..expansion.modules.len() {
        let declarations = std::mem::take(&mut expansion.modules[index].declarations);
        for mut declaration in declarations {
            match &declaration.kind {
                _ if is_template(&declaration) => {
                    // Lexical ownership is already bound. Open parameters stay
                    // positional; substitution never resolves a source spelling.
                    normalize_declaration(&mut declaration)?;
                    expansion.modules[index].templates.push(declaration);
                }
                _ => {
                    normalize_declaration(&mut declaration)?;
                    expansion.lower_declaration(index, &mut declaration, 0, Descent::default(), false)?;
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
    /// One cumulative generated-work allowance for all modules and cache hits.
    budget: Budget,
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
    fn lower_type(
        &mut self,
        target: usize,
        mut ty: &mut AbiType,
        depth: usize,
        mut at: Descent,
        generated: bool,
        span: &SourceSpan,
    ) -> Result<(), ResolutionError> {
        loop {
            if generated {
                self.budget.enter(at, span)?;
            }
            match ty {
                AbiType::Named(name) => {
                    if !name.arguments.is_empty() {
                        let owned = name.take();
                        *name = self.instantiate(target, owned, depth + 1, at.child())?;
                    } else if let Some(key) = name.binding.declaration() {
                        let dependency = self.modules[key.module].name.to_string();
                        self.add_dependency(target, &dependency, &name.span, generated)?;
                    }
                    return Ok(());
                }
                AbiType::ComPtr(child)
                | AbiType::WeakComPtr(child)
                | AbiType::ComBox(child)
                | AbiType::WeakComBox(child)
                | AbiType::Borrowed(child)
                | AbiType::Vector(child)
                | AbiType::UnsizedSlice(child)
                | AbiType::Slice(child)
                | AbiType::OutputSlice(child)
                | AbiType::Array { element: child, .. } => {
                    ty = child;
                    at = at.child();
                }
                AbiType::Unit | AbiType::Scalar(_) | AbiType::String => return Ok(()),
            }
        }
    }

    /// Lowers substituted roots. Authored nongeneric normalization is outside the quota.
    fn lower_declaration(&mut self, target: usize, declaration: &mut Declaration, depth: usize, at: Descent, generated: bool) -> Result<(), ResolutionError> {
        let span = std::mem::replace(&mut declaration.span, SourceSpan::placeholder());
        let result = visit_types(declaration, &mut |ty| self.lower_type(target, ty, depth, at, generated, &span));
        declaration.span = span;
        result
    }

    /// Charges only newly introduced generated dependency metadata.
    fn add_dependency(&mut self, target: usize, name: &str, span: &SourceSpan, generated: bool) -> Result<(), ResolutionError> {
        let module = &mut self.modules[target];
        if module.name.to_string() != name && !module.externals.iter().any(|external| external.module == name) {
            if generated {
                self.budget.allocate(std::mem::size_of::<ExternalModule>() + name.len(), span)?;
                self.budget.span(span, span)?;
            }
            module.externals.push(ExternalModule { span: span.clone(), module: name.into() });
        }
        Ok(())
    }

    /// Materializes a closed value or interface once in the consumer's module.
    fn instantiate(&mut self, target: usize, mut name: NamedType, depth: usize, at: Descent) -> Result<NamedType, ResolutionError> {
        self.budget.read_named(&name, at, &name.span, false)?;
        let identity = self.named_identity(&name, &name.arguments)?;
        self.budget.read_named(&name, at, &name.span, true)?;
        let origin = semantic_origin_named(&name, &name.arguments)?;
        if let Some(instance) = self.instances.get(&(target, identity)) {
            self.budget.compare_origin(&instance.origin, at, &name.span)?;
            self.budget.compare_origin(&origin, at, &name.span)?;
            if instance.origin != origin {
                let sources = self.originals.iter().flat_map(|module| module.sources.iter().cloned()).collect::<Vec<_>>();
                return Err(ResolutionError {
                    diagnostics: vec![
                        Diagnostic::new("EIDL060", format!("specialization UUID {identity} has conflicting semantic origins"))
                            .with_sources(&sources)
                            .primary(&name.span, "conflicting request")
                            .secondary(&instance.span, "first request"),
                    ],
                });
            }
            self.budget.key(&instance.key, &name.span)?;
            self.budget.allocate(instance.key.name.len(), &name.span)?;
            return Ok(projected(name.span, instance.key.clone()));
        }
        if depth > self.budget.limits.instances_depth || self.instances.len() >= self.budget.limits.instances {
            return Err(failure(&name.span, "type specialization exceeds the compiler depth or instance limit"));
        }
        let source_key = name.binding.declaration().ok_or_else(|| failure(&name.span, "expected a bound declaration"))?;
        let owner = source_key.module;
        let source = self.originals[owner]
            .declaration(&source_key.name)
            .or_else(|| self.modules[owner].declaration(&source_key.name))
            .ok_or_else(|| failure(&name.span, "bound declaration is absent"))?;
        let mut declaration = self.budget.copy_declaration(source, at.child(), &name.span)?;
        self.budget.allocate(
            self.originals[owner].name.segments().iter().map(String::len).sum::<usize>() + self.originals[owner].name.segments().len() - 1,
            &name.span,
        )?;
        let module_name = self.originals[owner].name.to_string();
        // Mangle the closed nominal type identity, not its display spelling.
        // The UUID suffix cannot be demangled; generation documents the full
        // template and argument spelling beside the resulting Rust declaration.
        // Reserve cache/projection keys, generated names, span copies and vector
        // bookkeeping before constructing them. Model units conservatively include
        // hash-table/container slack, rather than promising exact allocator bytes.
        self.budget
            .allocate(4 * std::mem::size_of::<Instance>() + 2 * std::mem::size_of::<Declaration>() + 128, &name.span)?;
        for _ in 0..6 {
            self.budget.allocate(declaration.name.len() + 40, &name.span)?;
        }
        for _ in 0..3 {
            self.budget.span(&name.span, &name.span)?;
        }
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
        substitute_declaration(&mut declaration, &name.arguments, &mut self.budget, at.child(), &name.span)?;
        let arguments = std::mem::take(&mut name.arguments);
        self.budget.header(&name, &name.span)?;
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
            self.project_argument(target, argument, depth, at.child(), &name.span)?;
        }
        self.add_dependency(target, &module_name, &name.span, true)?;
        self.lower_declaration(target, &mut declaration, depth, at.child(), true)?;
        self.modules[target].declarations.push(declaration);
        Ok(projected(name.span, key))
    }

    /// Retains an argument's canonical expression together with its actual local
    /// projection. Phantom arguments need this provenance just as stored fields do.
    fn project_argument(&mut self, target: usize, ty: &mut AbiType, depth: usize, at: Descent, span: &SourceSpan) -> Result<(), ResolutionError> {
        self.budget.enter(at, span)?;
        match ty {
            AbiType::Named(name) => {
                if matches!(name.binding, Binding::Application { .. }) {
                    let request = self.budget.copy_named(name, at.child(), span)?;
                    let closed = self.instantiate(target, request, depth + 1, at.child())?;
                    self.budget.key(closed.binding.projection().expect("materialized argument"), span)?;
                    let key = closed.binding.projection().expect("materialized argument").clone();
                    let Binding::Application { projection, .. } = &mut name.binding else {
                        unreachable!()
                    };
                    *projection = Some(key);
                } else if let Some(key) = name.binding.declaration() {
                    self.add_dependency(target, &self.modules[key.module].name.to_string(), &name.span, true)?;
                }
                for argument in &mut name.arguments {
                    self.project_argument(target, argument, depth, at.child(), span)?;
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
            | AbiType::Array { element: value, .. } => self.project_argument(target, value, depth, at.child(), span)?,
            AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
        }
        Ok(())
    }

    /// Hashes semantic type identities; import spelling and the consumer module are excluded.
    fn type_identity(&self, ty: &AbiType) -> Result<NcGuid, ResolutionError> {
        let mut bytes = Vec::new();
        match ty {
            AbiType::Named(name) => return self.named_identity(name, &name.arguments),
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
    /// Legacy identity encoding over a borrowed template and argument slice.
    fn named_identity(&self, name: &NamedType, arguments: &[AbiType]) -> Result<NcGuid, ResolutionError> {
        let mut bytes = Vec::new();
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
        if arguments.is_empty() {
            return Ok(id);
        }
        bytes.extend_from_slice(&id.to_uuid_bytes());
        for argument in arguments {
            bytes.extend_from_slice(&self.type_identity(argument)?.to_uuid_bytes());
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

/// Source normalization never consumes generated work or copies an argument.
fn normalize_declaration(declaration: &mut Declaration) -> Result<(), ResolutionError> {
    visit_types(declaration, &mut |ty| substitute_type(ty, &[], None, Descent::default(), None))
}

/// Substitutes each occurrence through the same fallible copy boundary.
fn substitute_type(
    ty: &mut AbiType,
    substitutions: &[AbiType],
    mut budget: Option<&mut Budget>,
    at: Descent,
    span: Option<&SourceSpan>,
) -> Result<(), ResolutionError> {
    if let Some(budget) = budget.as_deref_mut() {
        budget.enter(at, span.expect("generated request"))?;
    }
    match ty {
        AbiType::Named(name) => {
            if let Binding::Parameter { position } = name.binding {
                if let Some(value) = substitutions.get(position as usize) {
                    *ty = budget
                        .as_deref_mut()
                        .expect("substitution owner")
                        .copy_type(value, at.child(), span.expect("generated request"))?;
                    return Ok(());
                }
            }
            for argument in &mut name.arguments {
                substitute_type(argument, substitutions, budget.as_deref_mut(), at.child(), span)?;
            }
        }
        AbiType::ComPtr(child)
        | AbiType::WeakComPtr(child)
        | AbiType::ComBox(child)
        | AbiType::WeakComBox(child)
        | AbiType::Borrowed(child)
        | AbiType::Vector(child)
        | AbiType::UnsizedSlice(child)
        | AbiType::Slice(child)
        | AbiType::OutputSlice(child)
        | AbiType::Array { element: child, .. } => substitute_type(child, substitutions, budget, at.child(), span)?,
        AbiType::Unit | AbiType::Scalar(_) | AbiType::String => {}
    }
    // Move the existing child: normalization does not need a second owned copy.
    if matches!(ty, AbiType::Borrowed(child) if matches!(child.as_ref(), AbiType::UnsizedSlice(_))) {
        let AbiType::Borrowed(child) = std::mem::replace(ty, AbiType::Unit) else {
            unreachable!()
        };
        let AbiType::UnsizedSlice(element) = *child else { unreachable!() };
        *ty = AbiType::Slice(element);
    }
    Ok(())
}

/// Fields and named interface bases use the same positional substitution owner.
fn substitute_declaration(
    declaration: &mut Declaration,
    substitutions: &[AbiType],
    budget: &mut Budget,
    at: Descent,
    span: &SourceSpan,
) -> Result<(), ResolutionError> {
    visit_types(declaration, &mut |ty| substitute_type(ty, substitutions, Some(budget), at, Some(span)))
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
    // Only source metadata is retained for the named-only constraint error;
    // generated declaration admission reserves these two visitor-span copies.
    let span = name.span.clone();
    let mut ty = AbiType::Named(name.take());
    let result = visit(&mut ty);
    match ty {
        AbiType::Named(value) => {
            *name = value;
            result
        }
        _ => result.and_then(|()| Err(failure(&span, "an interface base must resolve to a named interface"))),
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
    named_signature(graph, &specialization.template, &specialization.arguments)
}

/// Recursively fingerprints the concrete ABI arguments and their declaring modules.
fn type_signature(graph: &crate::ResolvedModules, ty: &AbiType) -> Result<[u8; 32], ResolutionError> {
    let mut bytes = Vec::from(b"neocogi-idl:specialized-abi:v1\0".as_slice());
    match ty {
        AbiType::Named(name) => return named_signature(graph, name, &name.arguments),
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

/// Legacy signature bytes from borrowed bound arguments; no temporary AST.
fn named_signature(graph: &crate::ResolvedModules, name: &NamedType, arguments: &[AbiType]) -> Result<[u8; 32], ResolutionError> {
    let mut bytes = Vec::from(b"neocogi-idl:specialized-abi:v1\0".as_slice());
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
    if arguments.is_empty() {
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
    for argument in arguments {
        bytes.extend_from_slice(&type_signature(graph, argument)?);
    }
    Ok(crate::sha256::digest(&bytes))
}
