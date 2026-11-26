//! Closed declaration specialization before ordinary COM ABI validation.
//!
//! Templates are substituted into concrete values and interfaces in the using module. Their
//! identities depend on the template and argument identities, never on the using
//! module or a Rust crate path. Ordinary resolution then validates the resulting
//! methods, inheritance, borrowed values, and threading exactly once.

use std::collections::HashMap;
use crate::binding::{Binding, DeclarationKey, SemanticOrigin, semantic_origin_named};
use crate::model::{ParameterPassing, ReturnPassing};
use crate::resolve::ContractDigests;

pub(crate) mod budget;
mod height;
use budget::{Budget, Descent, Limits};
#[cfg(test)]
mod tests;
use neocogi_com::{NcGuid, guid_from_name, guid_from_qualified_name};
use crate::{IdlType, Declaration, DeclarationKind, Diagnostic, ExternalModule, Module, NamedType, ResolutionError, SourceSpan, Specialization, VariantFields};

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
        mut ty: &mut IdlType,
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
                IdlType::Named(name) => {
                    if !name.arguments.is_empty() {
                        let owned = name.take();
                        *name = self.instantiate(target, owned, depth + 1, at.child())?;
                    } else if let Some(key) = name.binding.declaration() {
                        let dependency = self.modules[key.module].name.to_string();
                        self.add_dependency(target, &dependency, &name.span, generated)?;
                    }
                    return Ok(());
                }
                IdlType::ComPtr(child)
                | IdlType::WeakComPtr(child)
                | IdlType::ComBox(child)
                | IdlType::Borrowed(child)
                | IdlType::BorrowedMut(child)
                | IdlType::Vector(child)
                | IdlType::UnsizedSlice(child)
                | IdlType::Slice(child)
                | IdlType::OutputSlice(child)
                | IdlType::Array { element: child, .. } => {
                    ty = child;
                    at = at.child();
                }
                IdlType::Unit | IdlType::Scalar(_) | IdlType::String => return Ok(()),
            }
        }
    }

    /// Lowers substituted roots. Authored nongeneric normalization is outside the quota.
    fn lower_declaration(&mut self, target: usize, declaration: &mut Declaration, depth: usize, at: Descent, generated: bool) -> Result<(), ResolutionError> {
        let span = std::mem::replace(&mut declaration.span, SourceSpan::placeholder());
        let result = self
            .classify_methods(declaration)
            .and_then(|()| visit_types(declaration, &mut |ty| self.lower_type(target, ty, depth, at, generated, &span)));
        declaration.span = span;
        result
    }

    /// Normalizes closed callable roots after substitution and before ordinary
    /// enum instantiation. Storage, arguments to templates, and open expressions
    /// continue through the regular value-specialization path.
    fn classify_methods(&self, declaration: &mut Declaration) -> Result<(), ResolutionError> {
        let DeclarationKind::Interface(interface) = &mut declaration.kind else {
            return Ok(());
        };
        for method in &mut interface.methods {
            for parameter in &mut method.parameters {
                let optional = self.strip_standard_option(&mut parameter.ty, |ty| matches!(ty, IdlType::Borrowed(_) | IdlType::BorrowedMut(_)));
                parameter.passing = self.parameter_passing(&parameter.ty, optional)?;
            }
            let optional = self.strip_standard_option(&mut method.output, |ty| {
                matches!(ty, IdlType::Borrowed(_) | IdlType::BorrowedMut(_) | IdlType::ComPtr(_))
            });
            method.return_passing = match &method.output {
                IdlType::Unit => ReturnPassing::Unit,
                IdlType::Scalar(_) | IdlType::Named(_) | IdlType::Array { .. } => ReturnPassing::Copied,
                IdlType::ComPtr(_) => ReturnPassing::SharedOwner { optional },
                IdlType::ComBox(_) => ReturnPassing::BoxOwner,
                IdlType::WeakComPtr(_) => ReturnPassing::SharedObserver,
                IdlType::Borrowed(_) | IdlType::BorrowedMut(_) => {
                    let mutable = matches!(method.output, IdlType::BorrowedMut(_));
                    if method.scoped && optional {
                        ReturnPassing::ScopedLoan { mutable }
                    } else {
                        ReturnPassing::BorrowedLoan { mutable, optional }
                    }
                }
                _ => ReturnPassing::Invalid,
            };
        }
        Ok(())
    }

    /// Reads the payload of the bundled Option application chosen by binding.
    fn standard_option_payload<'a>(&self, named: &'a NamedType) -> Option<&'a IdlType> {
        let Binding::Application { template, .. } = &named.binding else {
            return None;
        };
        if !crate::standard::is_optional_declaration(&self.originals[template.module].name, &template.name) {
            return None;
        }
        let [payload] = named.arguments.as_slice() else {
            unreachable!("binding checked standard Option arity");
        };
        Some(payload)
    }

    /// Moves the admitted standard Option payload without cloning any subtree.
    fn strip_standard_option(&self, ty: &mut IdlType, accepts: impl FnOnce(&IdlType) -> bool) -> bool {
        let IdlType::Named(named) = ty else {
            return false;
        };
        if !self.standard_option_payload(named).is_some_and(accepts) {
            return false;
        }
        *ty = named.arguments.pop().expect("a verified standard Option payload");
        true
    }

    /// Identifies the interface contract selected for a borrowed target.
    fn is_interface_target(&self, name: &NamedType) -> Result<bool, ResolutionError> {
        Ok(!name.is_str() && (crate::resolve::builtin_object(name).is_some() || matches!(self.declaration(name)?.1.kind, DeclarationKind::Interface(_))))
    }

    /// Explains the method slot spelling before an argument application becomes a stored enum.
    fn reject_optional_loan_argument(&self, name: &NamedType) -> Result<(), ResolutionError> {
        let Some(IdlType::Borrowed(target) | IdlType::BorrowedMut(target)) = self.standard_option_payload(name) else {
            return Ok(());
        };
        let IdlType::Named(target) = target.as_ref() else {
            return Ok(());
        };
        if !self.is_interface_target(target)? {
            return Ok(());
        }
        Err(ResolutionError {
            diagnostics: vec![
                Diagnostic::new("EIDL102", "an optional interface loan cannot be used as a generic value argument")
                    .primary(&name.span, "optional loan wrapped here")
                    .note("put the bundled Option<T> at the method input or result root and specialize T with &I or &mut I")
                    .note("use Option<ComPtr<I>> when the value needs to store an owned interface reference"),
            ],
        })
    }

    /// Selects the semantic input once from its already-bound source owner.
    fn parameter_passing(&self, ty: &IdlType, optional: bool) -> Result<ParameterPassing, ResolutionError> {
        let passing = match ty {
            IdlType::Scalar(_) | IdlType::Named(_) | IdlType::Array { .. } => ParameterPassing::Copied,
            IdlType::Slice(_) => ParameterPassing::InputBuffer,
            IdlType::OutputSlice(_) => ParameterPassing::OutputBuffer,
            IdlType::Borrowed(_) if optional => ParameterPassing::SharedLoan { optional },
            IdlType::Borrowed(target) => match target.as_ref() {
                IdlType::Named(name) if name.is_str() => ParameterPassing::BorrowedText,
                IdlType::Named(name) => {
                    if self.is_interface_target(name)? {
                        ParameterPassing::SharedLoan { optional: false }
                    } else {
                        ParameterPassing::BorrowedValue
                    }
                }
                _ => ParameterPassing::Invalid,
            },
            IdlType::BorrowedMut(_) => ParameterPassing::MutableLoan { optional },
            IdlType::ComBox(_) => ParameterPassing::ConsumingBox,
            _ => ParameterPassing::Invalid,
        };
        Ok(passing)
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
                return Err(ResolutionError {
                    diagnostics: vec![
                        Diagnostic::new("EIDL060", format!("specialization UUID {identity} has conflicting semantic origins"))
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
        // Prospective normalized height is distinct from active specialization
        // descent. Reject before copying/substituting the source declaration.
        self.budget.admit_substitution(source, &name.arguments, at.child(), &name.span)?;
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
        declaration.explicit_id = Some(identity);
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
    fn project_argument(&mut self, target: usize, ty: &mut IdlType, depth: usize, at: Descent, span: &SourceSpan) -> Result<(), ResolutionError> {
        self.budget.enter(at, span)?;
        match ty {
            IdlType::Named(name) => {
                if matches!(name.binding, Binding::Application { .. }) {
                    self.reject_optional_loan_argument(name)?;
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
            IdlType::ComPtr(value)
            | IdlType::WeakComPtr(value)
            | IdlType::ComBox(value)
            | IdlType::Borrowed(value)
            | IdlType::BorrowedMut(value)
            | IdlType::Vector(value)
            | IdlType::UnsizedSlice(value)
            | IdlType::Slice(value)
            | IdlType::OutputSlice(value)
            | IdlType::Array { element: value, .. } => self.project_argument(target, value, depth, at.child(), span)?,
            IdlType::Unit | IdlType::Scalar(_) | IdlType::String => {}
        }
        Ok(())
    }

    /// Hashes semantic type identities; import spelling and the consumer module are excluded.
    fn type_identity(&self, ty: &IdlType) -> Result<NcGuid, ResolutionError> {
        let mut bytes = Vec::new();
        match ty {
            IdlType::Named(name) => return self.named_identity(name, &name.arguments),
            IdlType::ComPtr(value)
            | IdlType::WeakComPtr(value)
            | IdlType::ComBox(value)
            | IdlType::Borrowed(value)
            | IdlType::BorrowedMut(value)
            | IdlType::Vector(value)
            | IdlType::UnsizedSlice(value)
            | IdlType::Slice(value)
            | IdlType::OutputSlice(value)
            | IdlType::Array { element: value, .. } => {
                let tag: &[u8] = match ty {
                    IdlType::ComPtr(_) => b"com_ptr",
                    IdlType::WeakComPtr(_) => b"weak_com_ptr",
                    IdlType::ComBox(_) => b"com_box",
                    IdlType::Borrowed(_) => b"borrow",
                    IdlType::BorrowedMut(_) => b"borrow_mut",
                    IdlType::Vector(_) => b"vector",
                    IdlType::UnsizedSlice(_) => b"unsized_slice",
                    IdlType::Slice(_) => b"slice",
                    IdlType::OutputSlice(_) => b"output",
                    _ => b"array",
                };
                bytes.extend_from_slice(tag);
                if let IdlType::Array { len, .. } = ty {
                    bytes.extend_from_slice(&len.to_be_bytes());
                }
                bytes.extend_from_slice(&self.type_identity(value)?.to_uuid_bytes());
            }
            _ => bytes.extend_from_slice(ty.source_name().as_bytes()),
        }
        Ok(guid_from_name(guid_from_qualified_name("neocogi.idl.closed-type:v1"), &bytes))
    }
    /// Legacy identity encoding over a borrowed template and argument slice.
    fn named_identity(&self, name: &NamedType, arguments: &[IdlType]) -> Result<NcGuid, ResolutionError> {
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
            .explicit_id
            .unwrap_or_else(|| crate::resolve::derive_declaration_id(&module.name.qualify(&declaration.name)));
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
    ty: &mut IdlType,
    substitutions: &[IdlType],
    mut budget: Option<&mut Budget>,
    at: Descent,
    span: Option<&SourceSpan>,
) -> Result<(), ResolutionError> {
    if let Some(budget) = budget.as_deref_mut() {
        budget.enter(at, span.expect("generated request"))?;
    }
    match ty {
        IdlType::Named(name) => {
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
        IdlType::ComPtr(child)
        | IdlType::WeakComPtr(child)
        | IdlType::ComBox(child)
        | IdlType::Borrowed(child)
        | IdlType::BorrowedMut(child)
        | IdlType::Vector(child)
        | IdlType::UnsizedSlice(child)
        | IdlType::Slice(child)
        | IdlType::OutputSlice(child)
        | IdlType::Array { element: child, .. } => substitute_type(child, substitutions, budget, at.child(), span)?,
        IdlType::Unit | IdlType::Scalar(_) | IdlType::String => {}
    }
    // Move the existing child. Borrowing a substituted unsized slice keeps
    // the established readable/writable buffer meaning of the authored borrow.
    let mutable = match ty {
        IdlType::Borrowed(child) if matches!(child.as_ref(), IdlType::UnsizedSlice(_)) => Some(false),
        IdlType::BorrowedMut(child) if matches!(child.as_ref(), IdlType::UnsizedSlice(_)) => Some(true),
        _ => None,
    };
    if let Some(mutable) = mutable {
        let (IdlType::Borrowed(child) | IdlType::BorrowedMut(child)) = std::mem::replace(ty, IdlType::Unit) else {
            unreachable!("matched borrow")
        };
        let IdlType::UnsizedSlice(element) = *child else {
            unreachable!("matched slice")
        };
        *ty = if mutable { IdlType::OutputSlice(element) } else { IdlType::Slice(element) };
    }
    Ok(())
}

/// Fields and named interface bases use the same positional substitution owner.
fn substitute_declaration(
    declaration: &mut Declaration,
    substitutions: &[IdlType],
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
pub(crate) fn visit_types(declaration: &mut Declaration, visit: &mut impl FnMut(&mut IdlType) -> Result<(), ResolutionError>) -> Result<(), ResolutionError> {
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
fn visit_named(name: &mut NamedType, visit: &mut impl FnMut(&mut IdlType) -> Result<(), ResolutionError>) -> Result<(), ResolutionError> {
    // Only source metadata is retained for the named-only constraint error;
    // generated declaration admission reserves these two visitor-span copies.
    let span = name.span.clone();
    let mut ty = IdlType::Named(name.take());
    let result = visit(&mut ty);
    match ty {
        IdlType::Named(value) => {
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
pub(crate) fn signature(graph: &crate::ResolvedModules, specialization: &Specialization) -> Result<ContractDigests, ResolutionError> {
    named_signature(graph, &specialization.template, &specialization.arguments)
}

/// Begins parallel preimages with independent domains for native and Rust proofs.
fn specialization_preimages() -> (Vec<u8>, Vec<u8>) {
    (
        b"neocogi-idl:specialized-abi:v1\0".to_vec(),
        b"neocogi-idl:specialized-rust-binding:v1\0".to_vec(),
    )
}

/// Recursively fingerprints concrete arguments and their declaring modules.
fn type_signature(graph: &crate::ResolvedModules, ty: &IdlType) -> Result<ContractDigests, ResolutionError> {
    let (mut native, mut rust) = specialization_preimages();
    match ty {
        IdlType::Named(name) => return named_signature(graph, name, &name.arguments),
        IdlType::ComPtr(value)
        | IdlType::WeakComPtr(value)
        | IdlType::ComBox(value)
        | IdlType::Borrowed(value)
        | IdlType::BorrowedMut(value)
        | IdlType::Vector(value)
        | IdlType::UnsizedSlice(value)
        | IdlType::Slice(value)
        | IdlType::OutputSlice(value)
        | IdlType::Array { element: value, .. } => {
            let tag: &[u8] = match ty {
                IdlType::ComPtr(_) => b"com_ptr",
                IdlType::WeakComPtr(_) => b"weak_com_ptr",
                IdlType::ComBox(_) => b"com_box",
                IdlType::Borrowed(_) => b"borrow",
                IdlType::BorrowedMut(_) => b"borrow_mut",
                IdlType::Vector(_) => b"vector",
                IdlType::UnsizedSlice(_) => b"unsized_slice",
                IdlType::Slice(_) => b"slice",
                IdlType::OutputSlice(_) => b"output",
                _ => b"array",
            };
            native.extend_from_slice(tag);
            rust.extend_from_slice(tag);
            if let IdlType::Array { len, .. } = ty {
                native.extend_from_slice(&len.to_be_bytes());
                rust.extend_from_slice(&len.to_be_bytes());
            }
            let child = type_signature(graph, value)?;
            native.extend_from_slice(&child.native);
            rust.extend_from_slice(&child.rust);
        }
        _ => {
            let name = ty.source_name();
            native.extend_from_slice(name.as_bytes());
            rust.extend_from_slice(name.as_bytes());
        }
    }
    Ok(ContractDigests {
        native: crate::sha256::digest(&native),
        rust: crate::sha256::digest(&rust),
    })
}

/// Derives exact template and argument proofs without materializing a second AST.
fn named_signature(graph: &crate::ResolvedModules, name: &NamedType, arguments: &[IdlType]) -> Result<ContractDigests, ResolutionError> {
    if let Some(identity) = crate::resolve::builtin_object(name) {
        // SAFETY: built-in interface signatures refer to permanent core constants.
        let digest = crate::sha256::digest(unsafe { identity.abi_signature.as_digest().expect("static core signature") });
        return Ok(ContractDigests { native: digest, rust: digest });
    }
    if name.is_str() {
        let digest = crate::sha256::digest(b"neocogi.idl.str");
        return Ok(ContractDigests { native: digest, rust: digest });
    }
    let key = name.binding.declaration().ok_or_else(|| failure(&name.span, "generic argument is not bound"))?;
    let module = &graph.modules()[key.module];
    let local = &key.name;
    if arguments.is_empty() {
        return module
            .declaration(local)
            .map(|identity| ContractDigests {
                native: identity.abi_signature,
                rust: identity.rust_binding,
            })
            .ok_or_else(|| failure(&name.span, "generic argument declaration is absent"));
    }
    let template = module
        .source
        .templates
        .iter()
        .find(|template| &template.name == local)
        .ok_or_else(|| failure(&name.span, "declaration template is absent"))?;
    let id = template
        .explicit_id
        .unwrap_or_else(|| crate::resolve::derive_declaration_id(&module.source.name.qualify(local)));
    let (mut native, mut rust) = specialization_preimages();
    native.extend_from_slice(&module.abi_fingerprint);
    rust.extend_from_slice(&module.rust_fingerprint);
    native.extend_from_slice(&id.to_uuid_bytes());
    rust.extend_from_slice(&id.to_uuid_bytes());
    for argument in arguments {
        let proof = type_signature(graph, argument)?;
        native.extend_from_slice(&proof.native);
        rust.extend_from_slice(&proof.rust);
    }
    Ok(ContractDigests {
        native: crate::sha256::digest(&native),
        rust: crate::sha256::digest(&rust),
    })
}
