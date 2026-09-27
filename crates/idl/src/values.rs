//! Finite resolved value graph shared by lowering and transported inspection.
//!
//! Named edges retain identity. Indirect recursive sequences therefore do not
//! expand their element definitions, and copied-position eligibility is a fixed
//! point over the same graph used to describe every field.
use std::collections::BTreeMap;

/// Resolved field expression, with ownership independent of a message annotation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueType {
    /// Empty application value.
    Unit,
    /// One built-in scalar name.
    Scalar(String),
    /// UTF-8 owned text.
    Text,
    /// Reference to a named record, enum, or bit-set definition.
    Named(String),
    /// Explicit owned COM capability.
    Object(String),
    /// Indirect owned collection of recursively converted values.
    Sequence(Box<ValueType>),
    /// Inline fixed-size aggregate.
    Array(Box<ValueType>, u32),
    /// Inline tagged absence/presence aggregate.
    Optional(Box<ValueType>),
}
/// A named aggregate's complete source shape, retained once in the graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueDefinition {
    /// Canonical qualified identity used by every named reference.
    pub name: String,
    /// Flattened fields in declaration order, then variant order for an enum.
    pub fields: Vec<ValueType>,
    /// Whether this backend provides a self-contained copied method lowering.
    pub copied: bool,
}
/// Canonical resolved definitions, including definitions reached through imports.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ValueGraph {
    /// One node per canonical name; recursion appears only as a named edge.
    pub definitions: BTreeMap<String, ValueDefinition>,
}
impl ValueGraph {
    /// Computes ownership/use-site eligibility once, monotonically to a fixed point.
    pub(crate) fn resolve_copied(&mut self) {
        loop {
            let rejected = self
                .definitions
                .values()
                .filter(|value| value.copied && !value.fields.iter().all(|ty| self.is_copied(ty)))
                .map(|value| value.name.clone())
                .collect::<Vec<_>>();
            if rejected.is_empty() {
                break;
            }
            for name in rejected {
                self.definitions.get_mut(&name).expect("collected definition exists").copied = false;
            }
        }
    }
    /// Reports implemented copied lowering, rather than merely finite Rust size.
    pub fn is_copied(&self, ty: &ValueType) -> bool {
        match ty {
            ValueType::Scalar(_) => true,
            ValueType::Named(name) => self.definitions.get(name).is_some_and(|value| value.copied),
            ValueType::Unit | ValueType::Text | ValueType::Object(_) | ValueType::Sequence(_) | ValueType::Array(_, _) | ValueType::Optional(_) => false,
        }
    }
}

impl ValueDefinition {
    /// Returns only edges contributing to inline layout; sequences and objects break cycles.
    pub(crate) fn inline_references(&self) -> Vec<&str> {
        fn visit<'a>(ty: &'a ValueType, names: &mut Vec<&'a str>) {
            match ty {
                ValueType::Named(name) => names.push(name),
                ValueType::Array(value, _) | ValueType::Optional(value) => visit(value, names),
                _ => {}
            }
        }
        let mut names = Vec::new();
        for field in &self.fields {
            visit(field, &mut names);
        }
        names
    }
}
