//! Prospective normalized inline shape, checked before substitution allocates it.
//! This is independent of combined reentrant Descent and nominal graph length.
use super::{
    budget::{Budget, Descent},
    failure,
};
use crate::{IdlType, Declaration, DeclarationKind, NamedType, ResolutionError, SourceSpan, VariantFields};
use crate::binding::Binding;

/// The only child shape that changes height when a source borrow is normalized.
#[derive(Clone, Copy)]
enum Form {
    /// A borrowed unsized slice will normalize to a single Slice constructor.
    UnsizedSlice,
    /// All other shapes retain an ordinary wrapper when borrowed.
    Other,
}
/// A transient result of a borrowed shape walk, never a second retained model.
struct Shape {
    /// Maximum inline path length including this node.
    height: usize,
    /// Root shape needed to match post-order slice normalization exactly.
    form: Form,
}
impl Shape {
    /// Admits a prospective node before any corresponding owned node is built.
    fn new(height: usize, form: Form, span: &SourceSpan) -> Result<Self, ResolutionError> {
        if height > crate::nesting::TYPE_HEIGHT {
            return Err(failure(span, "resulting type exceeds the compiler inline nesting limit"));
        }
        Ok(Self { height, form })
    }
}

impl Budget {
    /// Checks all substituted type roots while source and arguments remain borrowed.
    /// Charge each read through the existing monotone owner. Do not lower its
    /// unrelated Descent256/cache64 limits or copy an excessive intermediate tree.
    pub(super) fn admit_substitution(&mut self, source: &Declaration, arguments: &[IdlType], at: Descent, span: &SourceSpan) -> Result<(), ResolutionError> {
        let mut ty = |value: &IdlType| self.shape(value, Some(arguments), at, span).map(|_| ());
        match &source.kind {
            DeclarationKind::Struct(record) => {
                for field in &record.fields {
                    ty(&field.ty)?;
                }
            }
            DeclarationKind::Enum(enumeration) => {
                for variant in &enumeration.variants {
                    match &variant.fields {
                        VariantFields::Unit => {}
                        VariantFields::Tuple(fields) => {
                            for field in fields {
                                ty(field)?;
                            }
                        }
                        VariantFields::Named(fields) => {
                            for field in fields {
                                ty(&field.ty)?;
                            }
                        }
                    }
                }
            }
            DeclarationKind::BitEnum(_) => {}
            DeclarationKind::Interface(interface) => {
                for base in &interface.bases {
                    self.named_shape(base, Some(arguments), at, span)?;
                }
                for method in &interface.methods {
                    for parameter in &method.parameters {
                        self.shape(&parameter.ty, Some(arguments), at, span)?;
                    }
                    self.shape(&method.output, Some(arguments), at, span)?;
                }
            }
        }
        Ok(())
    }
    /// Parameter replacement copies the already-admitted argument as-is. Other
    /// source nodes normalize after their children, matching substitute_type.
    fn named_shape(&mut self, name: &NamedType, arguments: Option<&[IdlType]>, at: Descent, span: &SourceSpan) -> Result<Shape, ResolutionError> {
        self.enter(at, span)?;
        if let (Some(arguments), Binding::Parameter { position }) = (arguments, &name.binding) {
            if let Some(value) = arguments.get(*position as usize) {
                return self.shape(value, None, at.child(), span);
            }
        }
        let mut height = 1;
        for child in &name.arguments {
            height = height.max(1 + self.shape(child, arguments, at.child(), span)?.height);
        }
        Shape::new(height, Form::Other, span)
    }
    /// All borrowed recursive inputs were admitted by parser/previous construction.
    /// No declaration edge is followed, so long shallow graphs spend no height.
    fn shape(&mut self, ty: &IdlType, arguments: Option<&[IdlType]>, at: Descent, span: &SourceSpan) -> Result<Shape, ResolutionError> {
        if let IdlType::Named(name) = ty {
            return self.named_shape(name, arguments, at, span);
        }
        self.enter(at, span)?;
        match ty {
            IdlType::Unit | IdlType::Scalar(_) | IdlType::String => Shape::new(1, Form::Other, span),
            IdlType::Named(_) => unreachable!("named shape handled above"),
            IdlType::ComPtr(child)
            | IdlType::WeakComPtr(child)
            | IdlType::ComBox(child)
            | IdlType::WeakComBox(child)
            | IdlType::Vector(child)
            | IdlType::Slice(child)
            | IdlType::OutputSlice(child)
            | IdlType::Array { element: child, .. } => {
                let child = self.shape(child, arguments, at.child(), span)?;
                Shape::new(1 + child.height, Form::Other, span)
            }
            IdlType::UnsizedSlice(child) => {
                let child = self.shape(child, arguments, at.child(), span)?;
                Shape::new(1 + child.height, Form::UnsizedSlice, span)
            }
            IdlType::Borrowed(child) | IdlType::BorrowedMut(child) => {
                let child = self.shape(child, arguments, at.child(), span)?;
                let height = if arguments.is_some() && matches!(child.form, Form::UnsizedSlice) {
                    child.height
                } else {
                    child.height + 1
                };
                Shape::new(height, Form::Other, span)
            }
        }
    }
}
