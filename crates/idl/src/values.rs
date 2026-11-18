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
/// Exactly one operation is computed per scratch reverse graph.
#[derive(Clone, Copy)]
enum Capability {
    /// Self-contained ABI-copy lowering.
    Copied,
    /// Symmetric format-neutral construction and encoding.
    Persistent,
}

/// After peeling wrappers, a field is local or depends on one cached fact.
enum Requirement<'a> {
    /// The field's own representation decides the answer.
    Local(bool),
    /// The field follows a named definition's capability.
    Named(&'a str),
}

impl Capability {
    /// Peels only wrappers transparent to the selected operation.
    fn requirement(self, mut ty: &ValueType) -> Requirement<'_> {
        loop {
            match ty {
                ValueType::Unit => return Requirement::Local(true),
                ValueType::Text => return Requirement::Local(matches!(self, Self::Persistent)),
                ValueType::Scalar(name) => {
                    return Requirement::Local(match self {
                        Self::Copied => true,
                        Self::Persistent => persistent_scalar(name),
                    });
                }
                ValueType::Named(name) => return Requirement::Named(name),
                ValueType::Array(element, _) => ty = element,
                ValueType::Sequence(element) => match self {
                    Self::Copied => return Requirement::Local(false),
                    Self::Persistent => ty = element,
                },
                ValueType::Object(_) | ValueType::WeakObject(_) | ValueType::WeakBoxObject(_) => return Requirement::Local(false),
            }
        }
    }

    /// Reads the selected provisionally supported or rejected state.
    fn supported(self, definition: &ValueDefinition) -> bool {
        match self {
            Self::Copied => definition.copied,
            Self::Persistent => definition.persistence == Persistence::Supported,
        }
    }

    /// Records a stable negative fact without changing the other operation.
    fn reject(self, definition: &mut ValueDefinition) {
        match self {
            Self::Copied => definition.copied = false,
            Self::Persistent => definition.persistence = Persistence::Unsupported,
        }
    }
}

/// Scalars supported by both directions of the format-neutral codec.
fn persistent_scalar(name: &str) -> bool {
    matches!(
        name,
        "bool" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64" | "Guid" | "Status"
    )
}

#[cfg(test)]
#[derive(Default)]
/// Observes the scheduler and edge traversals without counting elapsed time.
struct PropagationWork {
    /// Rejected definitions scheduled exactly once.
    scheduled: usize,
    /// Reverse edges actually traversed from a rejected definition.
    edges: usize,
}

impl ValueGraph {
    /// Propagates invalidation separately for copied and persistent shapes.
    /// Recursive definitions with only supported leaves retain their capability.
    pub(crate) fn resolve_capabilities(&mut self) {
        for capability in [Capability::Copied, Capability::Persistent] {
            self.resolve_capability(
                capability,
                #[cfg(test)]
                None,
            );
        }
    }

    /// Builds one reverse graph and only visits dependents of negative facts.
    fn resolve_capability(&mut self, capability: Capability, #[cfg(test)] mut work: Option<&mut PropagationWork>) {
        let indices = self
            .definitions
            .keys()
            .enumerate()
            .map(|(index, name)| (name.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let mut dependents = vec![Vec::new(); indices.len()];
        let mut rejected = vec![false; indices.len()];
        let mut pending = Vec::new();
        for (index, definition) in self.definitions.values().enumerate() {
            let mut supported = capability.supported(definition);
            if supported {
                for field in &definition.fields {
                    supported = match capability.requirement(field) {
                        Requirement::Local(supported) => supported,
                        Requirement::Named(name) => match indices.get(name) {
                            Some(&dependency) => {
                                dependents[dependency].push(index);
                                true
                            }
                            None => false,
                        },
                    };
                    if !supported {
                        break;
                    }
                }
            }
            if !supported {
                rejected[index] = true;
                pending.push(index);
            }
        }

        while let Some(rejected_index) = pending.pop() {
            #[cfg(test)]
            if let Some(work) = &mut work {
                work.scheduled += 1;
                work.edges += dependents[rejected_index].len();
            }
            for &dependent in &dependents[rejected_index] {
                if !rejected[dependent] {
                    rejected[dependent] = true;
                    pending.push(dependent);
                }
            }
        }
        for (definition, rejected) in self.definitions.values_mut().zip(rejected) {
            if rejected {
                capability.reject(definition);
            }
        }
    }

    /// Queries the final fact without expanding a named definition.
    fn supports(&self, capability: Capability, ty: &ValueType) -> bool {
        match capability.requirement(ty) {
            Requirement::Local(supported) => supported,
            Requirement::Named(name) => self.definitions.get(name).is_some_and(|definition| capability.supported(definition)),
        }
    }

    /// Checks format-neutral construction, independently of the COM wire codec.
    /// Guid and Status use explicit scalar/array recipes, never native-memory copies.
    pub fn can_persist(&self, ty: &ValueType) -> bool {
        self.supports(Capability::Persistent, ty)
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
    pub fn is_copied(&self, ty: &ValueType) -> bool {
        self.supports(Capability::Copied, ty)
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
