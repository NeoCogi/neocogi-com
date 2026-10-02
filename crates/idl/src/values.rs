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
}
/// Symmetric persistence availability inferred from every reachable value field.
/// This is separate from transporting a value that already exists in memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Persistence {
    /// Both codecs exist for the entire value, including recursively reachable fields.
    Supported,
    /// A reachable live capability has no object reconstruction contract.
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
    /// Inferred availability of both persistence directions.
    pub persistence: Persistence,
}
/// Canonical resolved definitions, including definitions reached through imports.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ValueGraph {
    /// One node per canonical name; recursion appears only as a named edge.
    pub definitions: BTreeMap<String, ValueDefinition>,
}
impl ValueGraph {
    /// Removes unsupported operations monotonically until all named edges agree.
    /// Recursive sequence codecs begin provisionally supported;
    /// missing leaf operations propagate through every cycle before validation.
    pub(crate) fn resolve_capabilities(&mut self) {
        loop {
            let rejected = self
                .definitions
                .values()
                .filter_map(|value| {
                    let copied = value.copied && value.fields.iter().all(|ty| self.is_copied(ty));
                    let persistence = match value.persistence {
                        Persistence::Supported if !value.fields.iter().all(|ty| self.can_persist(ty)) => Persistence::Unsupported,
                        state => state,
                    };
                    (copied != value.copied || persistence != value.persistence).then(|| (value.name.clone(), copied, persistence))
                })
                .collect::<Vec<_>>();
            if rejected.is_empty() {
                break;
            }
            for (name, copied, persistence) in rejected {
                let value = self.definitions.get_mut(&name).expect("collected definition exists");
                value.copied = copied;
                value.persistence = persistence;
            }
        }
    }

    /// Checks format-neutral construction, independently of the COM wire codec.
    /// Guid and Status use explicit scalar/array recipes, never native-memory copies.
    pub fn can_persist(&self, ty: &ValueType) -> bool {
        match ty {
            ValueType::Unit | ValueType::Text => true,
            ValueType::Scalar(name) => matches!(
                name.as_str(),
                "bool" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64" | "Guid" | "Status"
            ),
            ValueType::Named(name) => self
                .definitions
                .get(name)
                .is_some_and(|value| value.persistence == Persistence::Supported),
            ValueType::Object(_) => false,
            ValueType::Sequence(value) | ValueType::Array(value, _) => self.can_persist(value),
        }
    }

    /// Finds an unsupported reachable leaf for a source-located format-policy diagnostic.
    /// A visited set prevents recursive records from hiding a later failing sibling.
    pub(crate) fn persistence_failure(&self, ty: &ValueType) -> Option<String> {
        /// Walks unsupported dependencies only; revisited edges do not consume the sibling search.
        fn visit(graph: &ValueGraph, ty: &ValueType, visited: &mut BTreeSet<String>) -> Option<String> {
            if graph.can_persist(ty) {
                return None;
            }
            match ty {
                ValueType::Object(name) => Some(format!(
                    "ComPtr<{name}> transports an existing object; persistence requires an explicit object reconstruction contract"
                )),
                ValueType::Scalar(name) => Some(format!("{name} has no format-neutral ComDeserialize implementation")),
                ValueType::Named(name) => {
                    let Some(value) = graph.definitions.get(name) else {
                        return Some(format!("{name} has no resolved value definition"));
                    };
                    match value.persistence {
                        Persistence::Unsupported if visited.insert(name.clone()) => value.fields.iter().find_map(|field| visit(graph, field, visited)),
                        Persistence::Unsupported | Persistence::Supported => None,
                    }
                }
                ValueType::Sequence(value) | ValueType::Array(value, _) => visit(graph, value, visited),
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
            ValueType::Unit | ValueType::Text | ValueType::Object(_) | ValueType::Sequence(_) => false,
        }
    }
}

impl ValueDefinition {
    /// Returns only edges contributing to inline layout; sequences and objects break cycles.
    pub(crate) fn inline_references(&self) -> Vec<&str> {
        fn visit<'a>(ty: &'a ValueType, names: &mut Vec<&'a str>) {
            match ty {
                ValueType::Named(name) => names.push(name),
                ValueType::Array(value, _) => visit(value, names),
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
