//! One expansion's monotone allowance, checked before generated copies and reads.
//!
//! Charges are conservative model-storage units, not allocator/RSS measurements.
//! Read preflights pay for themselves plus the subsequent walk and its bounded
//! hash scratch. Copy preflights include every owned field before derived Clone.
//! Walks iterate siblings lazily; native recursion is limited by combined descent.

use std::mem::size_of;
use crate::{AbiType, Declaration, DeclarationKind, NamedType, ResolutionError, SourceSpan, VariantFields};
use crate::binding::{Binding, DeclarationKey, SemanticOrigin};
use super::failure;

/// Compiler implementation limits; never ABI identity inputs or public options.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    /// Cumulative structural work, including preflight and subsequent operation.
    pub(crate) work: usize,
    /// Cumulative generated model/scratch storage; freed transients are not refunded.
    pub(crate) bytes: usize,
    /// Combined active structural and reentrant specialization descent.
    pub(crate) descent: usize,
    /// New materialization depth, checked after exact active-cache reuse.
    pub(crate) instances_depth: usize,
    /// Consumer-local projections across this whole expansion.
    pub(crate) instances: usize,
    /// Test-only denial at an actual construction boundary, before Clone.
    #[cfg(test)]
    pub(crate) denied_copy: Option<CopyKind>,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            work: 1_048_576,
            bytes: 32 * 1024 * 1024,
            descent: 256,
            instances_depth: 64,
            instances: 4096,
            #[cfg(test)]
            denied_copy: None,
        }
    }
}

/// By-value active path; it is never reset when entering another declaration.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Descent(
    /// Structural/reentrant steps already active on this lowering path.
    pub(super) usize,
);
impl Descent {
    /// Advances even through unary lowering implemented as a loop.
    pub(super) fn child(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// The two real copy boundaries used by private production-wiring regressions.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CopyKind {
    /// Template declaration, including documentation and all type roots.
    Declaration,
    /// Substituted argument, including phantom provenance/projection requests.
    Type,
}

/// Sole mutable owner of generated work/storage admission for one module batch.
pub(super) struct Budget {
    /// Limits and remaining amounts are kept together, never reset per instance.
    pub(super) limits: Limits,
    /// Remaining work units.
    work: usize,
    /// Remaining cumulative owned-storage units.
    bytes: usize,
}
impl Budget {
    /// Starts one allowance for the complete specialization operation.
    pub(super) fn new(limits: Limits) -> Self {
        Self {
            limits,
            work: limits.work,
            bytes: limits.bytes,
        }
    }

    /// Checks active descent and charges a real generated traversal step.
    pub(super) fn enter(&mut self, at: Descent, span: &SourceSpan) -> Result<(), ResolutionError> {
        if at.0 > self.limits.descent {
            return Err(failure(span, "type specialization exceeds the compiler nesting limit"));
        }
        self.work = self
            .work
            .checked_sub(1)
            .ok_or_else(|| failure(span, "type specialization exceeds the compiler work limit"))?;
        Ok(())
    }

    /// Reserves storage before allocation. Diagnostic finalization is exempt:
    /// one fixed message and one requesting span, never a generated tree/corpus.
    pub(super) fn allocate(&mut self, bytes: usize, span: &SourceSpan) -> Result<(), ResolutionError> {
        self.bytes = self
            .bytes
            .checked_sub(bytes)
            .ok_or_else(|| failure(span, "type specialization exceeds the compiler storage limit"))?;
        Ok(())
    }

    /// Reserves a vector's logical element storage with checked arithmetic.
    fn elements<T>(&mut self, len: usize, span: &SourceSpan) -> Result<(), ResolutionError> {
        self.allocate(
            len.checked_mul(size_of::<T>())
                .ok_or_else(|| failure(span, "type specialization exceeds the compiler storage limit"))?,
            span,
        )
    }

    /// Copies one complete generated declaration only after complete admission.
    pub(super) fn copy_declaration(&mut self, value: &Declaration, at: Descent, span: &SourceSpan) -> Result<Declaration, ResolutionError> {
        self.declaration(value, at, span)?;
        #[cfg(test)]
        self.allow_copy(CopyKind::Declaration, span)?;
        Ok(value.clone())
    }

    /// Fallible type copy: every occurrence independently pays before Clone.
    pub(super) fn copy_type(&mut self, value: &AbiType, at: Descent, span: &SourceSpan) -> Result<AbiType, ResolutionError> {
        self.ty(value, at, span, true)?;
        #[cfg(test)]
        self.allow_copy(CopyKind::Type, span)?;
        Ok(value.clone())
    }

    /// Reserves the unavoidable retained canonical argument used during projection.
    pub(super) fn copy_named(&mut self, value: &NamedType, at: Descent, span: &SourceSpan) -> Result<NamedType, ResolutionError> {
        self.named(value, at, span, true)?;
        #[cfg(test)]
        self.allow_copy(CopyKind::Type, span)?;
        Ok(value.clone())
    }

    /// Bounds a borrowed identity walk or reserves the larger exact-origin output.
    pub(super) fn read_named(&mut self, value: &NamedType, at: Descent, span: &SourceSpan, origin: bool) -> Result<(), ResolutionError> {
        self.named(value, at, span, origin)
    }

    /// Reserves exact equality's traversal without reducing meaning to a digest.
    pub(super) fn compare_origin(&mut self, value: &SemanticOrigin, at: Descent, span: &SourceSpan) -> Result<(), ResolutionError> {
        self.enter(at, span)?;
        self.enter(at, span)?;
        match value {
            SemanticOrigin::Application(_, arguments) => {
                for argument in arguments {
                    self.compare_origin(argument, at.child(), span)?;
                }
            }
            SemanticOrigin::Constructed(_, _, child) => self.compare_origin(child, at.child(), span)?,
            SemanticOrigin::Unit | SemanticOrigin::Scalar(_) | SemanticOrigin::Text | SemanticOrigin::Intrinsic(_) | SemanticOrigin::Nominal(_) => {}
        }
        Ok(())
    }

    /// Counts span payload; the containing record accounts for fixed headers.
    pub(super) fn span(&mut self, value: &SourceSpan, span: &SourceSpan) -> Result<(), ResolutionError> {
        for text in value.owned_strings() {
            self.allocate(text.len(), span)?;
        }
        Ok(())
    }

    /// Counts an owned bound key, independently of source path spelling.
    pub(super) fn key(&mut self, value: &DeclarationKey, span: &SourceSpan) -> Result<(), ResolutionError> {
        self.allocate(size_of::<DeclarationKey>(), span)?;
        self.allocate(value.name.len(), span)
    }

    /// Fixed metadata of a named type, excluding its separately visited arguments.
    pub(super) fn header(&mut self, value: &NamedType, span: &SourceSpan) -> Result<(), ResolutionError> {
        let NamedType {
            span: source,
            segments,
            arguments: _,
            binding,
        } = value;
        self.allocate(size_of::<AbiType>(), span)?;
        self.span(source, span)?;
        self.strings(segments, span)?;
        match binding {
            Binding::Declaration(key) => self.key(key, span)?,
            Binding::Application { template, projection } => {
                self.key(template, span)?;
                if let Some(key) = projection {
                    self.key(key, span)?;
                }
            }
            Binding::Unbound | Binding::Parameter { position: _ } | Binding::Intrinsic(_) => {}
        }
        Ok(())
    }

    /// One lazy bounded named walk; no wide child list is enqueued in advance.
    fn named(&mut self, value: &NamedType, at: Descent, span: &SourceSpan, copy: bool) -> Result<(), ResolutionError> {
        self.enter(at, span)?;
        self.enter(at, span)?; // reserve the subsequent read/copy, not just estimator work
        if copy {
            self.header(value, span)?;
        } else {
            self.allocate(64, span)?;
        }
        for argument in &value.arguments {
            self.ty(argument, at.child(), span, copy)?;
        }
        Ok(())
    }

    /// Exhaustive type storage/traversal policy. Recursive frames compose with
    /// the caller's active lowering path, including a substitution destination.
    fn ty(&mut self, value: &AbiType, at: Descent, span: &SourceSpan, copy: bool) -> Result<(), ResolutionError> {
        if let AbiType::Named(name) = value {
            return self.named(name, at, span, copy);
        }
        self.enter(at, span)?;
        self.enter(at, span)?;
        self.allocate(if copy { size_of::<AbiType>() } else { 64 }, span)?;
        match value {
            AbiType::Scalar(name) => {
                if copy {
                    self.allocate(name.len(), span)?;
                }
            }
            AbiType::Named(_) => unreachable!("named policy handled above"),
            AbiType::ComPtr(child)
            | AbiType::WeakComPtr(child)
            | AbiType::ComBox(child)
            | AbiType::WeakComBox(child)
            | AbiType::Borrowed(child)
            | AbiType::Vector(child)
            | AbiType::UnsizedSlice(child)
            | AbiType::Slice(child)
            | AbiType::OutputSlice(child)
            | AbiType::Array { element: child, len: _ } => self.ty(child, at.child(), span, copy)?,
            AbiType::Unit | AbiType::String => {}
        }
        Ok(())
    }

    /// Owned string vectors include headers as well as payload bytes.
    fn strings(&mut self, values: &[String], span: &SourceSpan) -> Result<(), ResolutionError> {
        self.elements::<String>(values.len(), span)?;
        for value in values {
            self.enter(Descent::default(), span)?;
            self.allocate(value.len(), span)?;
        }
        Ok(())
    }

    /// Common owned source metadata for declaration/field/method/variant records.
    fn metadata(&mut self, source: &SourceSpan, docs: &[String], name: &str, span: &SourceSpan) -> Result<(), ResolutionError> {
        self.span(source, span)?;
        self.strings(docs, span)?;
        self.allocate(name.len(), span)
    }

    /// Exhaustive declaration fields: adding owned model fields forces an update.
    fn declaration(&mut self, value: &Declaration, at: Descent, span: &SourceSpan) -> Result<(), ResolutionError> {
        let Declaration {
            span: source,
            docs,
            name,
            id: _,
            parameters,
            specialization,
            kind,
        } = value;
        self.enter(at, span)?;
        self.enter(at, span)?;
        self.allocate(size_of::<Declaration>(), span)?;
        self.metadata(source, docs, name, span)?;
        self.strings(parameters, span)?;
        if let Some(crate::Specialization { template, arguments }) = specialization {
            self.named(template, at.child(), span, true)?;
            for argument in arguments {
                self.ty(argument, at.child(), span, true)?;
            }
        }
        match kind {
            DeclarationKind::Struct(crate::Struct { fields, persistence: _ }) => self.fields(fields, at, span)?,
            DeclarationKind::Enum(crate::Enum { repr: _, dense: _, names: _, variants }) => {
                self.elements::<crate::EnumVariant>(variants.len(), span)?;
                for crate::EnumVariant {
                    span: source,
                    docs,
                    name,
                    discriminant: _,
                    fields,
                } in variants
                {
                    self.enter(at, span)?;
                    self.metadata(source, docs, name, span)?;
                    match fields {
                        VariantFields::Unit => {}
                        VariantFields::Tuple(types) => {
                            for ty in types {
                                self.ty(ty, at.child(), span, true)?;
                            }
                        }
                        VariantFields::Named(fields) => self.fields(fields, at, span)?,
                    }
                }
            }
            DeclarationKind::BitEnum(crate::BitEnum { repr: _, flags }) => {
                self.elements::<crate::BitFlag>(flags.len(), span)?;
                for crate::BitFlag { span: source, docs, name, value: _ } in flags {
                    self.enter(at, span)?;
                    self.metadata(source, docs, name, span)?;
                }
            }
            DeclarationKind::Interface(crate::Interface { bases, methods, threading: _ }) => {
                for base in bases {
                    self.named(base, at.child(), span, true)?;
                    // Substitution and lowering each retain one error-site span
                    // while moving a named base through the shared type visitor.
                    self.span(&base.span, span)?;
                    self.span(&base.span, span)?;
                }
                self.elements::<crate::Method>(methods.len(), span)?;
                for crate::Method {
                    span: source,
                    docs,
                    name,
                    receiver: _,
                    parameters,
                    output,
                } in methods
                {
                    self.enter(at, span)?;
                    self.metadata(source, docs, name, span)?;
                    self.elements::<crate::Parameter>(parameters.len(), span)?;
                    for crate::Parameter { span: source, name, ty } in parameters {
                        self.span(source, span)?;
                        self.allocate(name.len(), span)?;
                        self.ty(ty, at.child(), span, true)?;
                    }
                    self.ty(output, at.child(), span, true)?;
                }
            }
        }
        Ok(())
    }

    /// Visits field records without allocating a parallel field/type model.
    fn fields(&mut self, fields: &[crate::Field], at: Descent, span: &SourceSpan) -> Result<(), ResolutionError> {
        self.elements::<crate::Field>(fields.len(), span)?;
        for crate::Field { span: source, docs, name, ty } in fields {
            self.enter(at, span)?;
            self.metadata(source, docs, name, span)?;
            self.ty(ty, at.child(), span, true)?;
        }
        Ok(())
    }

    /// Private denial proves the production construction call is actually wired.
    #[cfg(test)]
    fn allow_copy(&self, kind: CopyKind, span: &SourceSpan) -> Result<(), ResolutionError> {
        if self.limits.denied_copy == Some(kind) {
            Err(failure(span, "test denied generated copy"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vector-size arithmetic rejects before allocation, even with maximum quota.
    #[test]
    fn semantic_budget_storage_arithmetic_is_checked() {
        let span = SourceSpan::placeholder();
        let mut budget = Budget::new(Limits { bytes: usize::MAX, ..Limits::default() });
        assert!(
            budget
                .elements::<String>(usize::MAX, &span)
                .unwrap_err()
                .to_string()
                .contains("compiler storage limit")
        );
        let mut budget = Budget::new(Limits { bytes: 1, work: 1, ..Limits::default() });
        budget.allocate(1, &span).unwrap();
        assert!(budget.allocate(1, &span).is_err());
        budget.enter(Descent::default(), &span).unwrap();
        assert!(budget.enter(Descent::default(), &span).is_err());
    }
}
