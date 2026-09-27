//! Finite resolved value graph shared by lowering and transported inspection.
//!
//! Named edges retain identity. Indirect recursive sequences therefore do not
//! expand their element definitions, and copied-position eligibility is a fixed
//! point over the same graph used to describe every field.
use std::collections::{BTreeMap, BTreeSet};

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
/// Availability of the IDL-requested format-neutral record decoder.
/// This is separate from transporting a value that already exists in memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Deserialization {
    /// The declaration does not request a generated decoder.
    NotRequested,
    /// The requested decoder and every field decoder are available.
    Supported,
    /// The declaration requests decoding but at least one field lacks support.
    Unsupported,
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
    /// Requested local decoding capability after recursive field validation.
    pub deserialization: Deserialization,
}
/// Canonical resolved definitions, including definitions reached through imports.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ValueGraph {
    /// One node per canonical name; recursion appears only as a named edge.
    pub definitions: BTreeMap<String, ValueDefinition>,
}
impl ValueGraph {
    /// Removes unsupported operations monotonically until all named edges agree.
    /// Requested recursive sequence decoders begin provisionally supported;
    /// missing leaf operations propagate through every cycle before validation.
    pub(crate) fn resolve_capabilities(&mut self) {
        loop {
            let rejected = self
                .definitions
                .values()
                .filter_map(|value| {
                    let copied = value.copied && value.fields.iter().all(|ty| self.is_copied(ty));
                    let deserialization = match value.deserialization {
                        Deserialization::Supported if !value.fields.iter().all(|ty| self.can_deserialize(ty)) => Deserialization::Unsupported,
                        state => state,
                    };
                    (copied != value.copied || deserialization != value.deserialization).then(|| (value.name.clone(), copied, deserialization))
                })
                .collect::<Vec<_>>();
            if rejected.is_empty() {
                break;
            }
            for (name, copied, deserialization) in rejected {
                let value = self.definitions.get_mut(&name).expect("collected definition exists");
                value.copied = copied;
                value.deserialization = deserialization;
            }
        }
    }

    /// Checks format-neutral construction, independently of the COM wire codec.
    /// Guid and Status have wire codecs but no declared serialization format.
    pub fn can_deserialize(&self, ty: &ValueType) -> bool {
        match ty {
            ValueType::Unit | ValueType::Text => true,
            ValueType::Scalar(name) => matches!(
                name.as_str(),
                "bool" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64"
            ),
            ValueType::Named(name) => self
                .definitions
                .get(name)
                .is_some_and(|value| value.deserialization == Deserialization::Supported),
            ValueType::Object(_) => false,
            ValueType::Sequence(value) | ValueType::Array(value, _) | ValueType::Optional(value) => self.can_deserialize(value),
        }
    }

    /// Finds an actual unsupported leaf or missing declaration policy for diagnostics.
    /// A visited set prevents recursive records from hiding a later failing sibling.
    pub(crate) fn deserialization_failure(&self, ty: &ValueType) -> Option<String> {
        /// Walks unsupported dependencies only; revisited edges do not consume the sibling search.
        fn visit(graph: &ValueGraph, ty: &ValueType, visited: &mut BTreeSet<String>) -> Option<String> {
            if graph.can_deserialize(ty) {
                return None;
            }
            match ty {
                ValueType::Object(name) => Some(format!(
                    "ComPtr<{name}> transports an existing object; deserialization requires an explicit object reconstruction contract"
                )),
                ValueType::Scalar(name) => Some(format!("{name} has no format-neutral ComDeserialize implementation")),
                ValueType::Named(name) => {
                    let Some(value) = graph.definitions.get(name) else {
                        return Some(format!("{name} has no resolved value definition"));
                    };
                    match value.deserialization {
                        Deserialization::NotRequested => Some(format!(
                            "{name} does not declare a generated decoder; referenced records/enums must also request #[deserialize] (bit-set decoders are not exposed by IDL)"
                        )),
                        Deserialization::Unsupported if visited.insert(name.clone()) => value.fields.iter().find_map(|field| visit(graph, field, visited)),
                        Deserialization::Unsupported | Deserialization::Supported => None,
                    }
                }
                ValueType::Sequence(value) | ValueType::Array(value, _) | ValueType::Optional(value) => visit(graph, value, visited),
                ValueType::Unit | ValueType::Text => None,
            }
        }
        visit(self, ty, &mut BTreeSet::new())
    }

    /// Reports implemented copied lowering, rather than merely finite Rust size.
    pub fn is_copied(&self, ty: &ValueType) -> bool {
        match ty {
            ValueType::Scalar(_) => true,
            ValueType::Array(element, _) => self.is_copied(element),
            ValueType::Named(name) => self.definitions.get(name).is_some_and(|value| value.copied),
            ValueType::Unit | ValueType::Text | ValueType::Object(_) | ValueType::Sequence(_) | ValueType::Optional(_) => false,
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
