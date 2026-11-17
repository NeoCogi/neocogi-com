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
    /// Shared weak COM capability; neither transport nor persistence owns its value.
    WeakObject(String),
    /// Observer of an exclusive owner; retains bookkeeping without borrowing the value.
    WeakBoxObject(String),
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
    /// Unary wrappers are walked iteratively, including detached public inputs;
    /// named leaves read cached facts without expanding their definitions.
    pub fn can_persist(&self, mut ty: &ValueType) -> bool {
        // Public detached expressions need not have passed compiler admission.
        // Unary wrappers therefore use constant native stack even for those inputs.
        loop {
            match ty {
                ValueType::Unit | ValueType::Text => return true,
                ValueType::Scalar(name) => {
                    return matches!(
                        name.as_str(),
                        "bool" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64" | "Guid" | "Status"
                    );
                }
                ValueType::Named(name) => return self.definitions.get(name).is_some_and(|value| value.persistence == Persistence::Supported),
                ValueType::Object(_) | ValueType::WeakObject(_) | ValueType::WeakBoxObject(_) => return false,
                ValueType::Sequence(value) | ValueType::Array(value, _) => ty = value,
            }
        }
    }

    /// Finds the first unsupported leaf in declaration/variant DFS order.
    /// Borrowed field iterators own nominal descent on the heap; unary wrappers
    /// are peeled locally. Visited names last for this entire search, preventing
    /// cycles from hiding later siblings without caching a root-independent witness.
    pub(crate) fn persistence_failure(&self, ty: &ValueType) -> Option<String> {
        let mut visited = BTreeSet::new();
        let mut fields = vec![std::slice::from_ref(ty).iter()];
        while let Some(frame) = fields.last_mut() {
            let Some(mut ty) = frame.next() else {
                fields.pop();
                continue;
            };
            if self.can_persist(ty) {
                continue;
            }
            loop {
                match ty {
                    ValueType::Object(name) => {
                        return Some(format!(
                            "ComPtr<{name}> transports an existing object; persistence requires an explicit object reconstruction contract"
                        ));
                    }
                    ValueType::WeakObject(name) => {
                        return Some(format!(
                            "WeakComPtr<{name}> transports lifetime bookkeeping; persistence requires an explicit object reconstruction contract"
                        ));
                    }
                    ValueType::WeakBoxObject(name) => {
                        return Some(format!(
                            "WeakComBox<{name}> transports exclusive lifetime bookkeeping; persistence requires an explicit object reconstruction contract"
                        ));
                    }
                    ValueType::Scalar(name) => return Some(format!("{name} has no format-neutral ComDeserialize implementation")),
                    ValueType::Named(name) => {
                        let Some(value) = self.definitions.get(name) else {
                            return Some(format!("{name} has no resolved value definition"));
                        };
                        if value.persistence == Persistence::Unsupported && visited.insert(name.as_str()) {
                            // Enter once, then finish these fields before any later sibling.
                            // Iterators retain only active ancestry, not a wide sibling frontier.
                            fields.push(value.fields.iter());
                        }
                        break;
                    }
                    ValueType::Sequence(value) | ValueType::Array(value, _) => ty = value,
                    ValueType::Unit | ValueType::Text => break,
                }
            }
        }
        None
    }

    /// Reports implemented copied lowering, rather than merely finite Rust size.
    /// Only arrays unwrap; sequences remain owned values regardless of their leaf.
    /// The borrowed query does not require detached inputs to have admitted height.
    pub fn is_copied(&self, mut ty: &ValueType) -> bool {
        loop {
            match ty {
                ValueType::Unit | ValueType::Scalar(_) => return true,
                ValueType::Array(element, _) => ty = element,
                ValueType::Named(name) => return self.definitions.get(name).is_some_and(|value| value.copied),
                ValueType::Text | ValueType::Object(_) | ValueType::WeakObject(_) | ValueType::WeakBoxObject(_) | ValueType::Sequence(_) => return false,
            }
        }
    }
}

impl ValueDefinition {
    /// Returns only edges contributing to inline layout; sequences and objects break cycles.
    pub(crate) fn inline_references(&self) -> Vec<&str> {
        let mut names = Vec::new();
        for mut ty in &self.fields {
            while let ValueType::Array(child, _) = ty {
                ty = child;
            }
            if let ValueType::Named(name) = ty {
                names.push(name.as_str());
            }
        }
        names
    }
}

#[cfg(test)]
mod tests;
