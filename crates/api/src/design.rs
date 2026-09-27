//! Source-level IDL descriptions attached to generated COM interface types.
//!
//! These immutable records supplement the physical ABI descriptors used by
//! `neocogi-inspect`: ABI signatures identify compatibility, while these records
//! retain declared event, struct, and enum names needed by authoring tools.
//! Generated records stay local to their defining binary. A plugin may copy
//! their values into a versioned COM design catalog for unlinked authoring hosts.

use crate::NcInterfaceDescriptor;

/// Structural field expression; named edges refer to the finite definition table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdlValueType {
    /// Empty value.
    Unit,
    /// Built-in scalar, including its exact integer or float width.
    Scalar(&'static str),
    /// Owned UTF-8 text.
    Text,
    /// Canonical identity of another value definition.
    Named(&'static str),
    /// Canonical identity of an explicitly retained COM capability.
    Object(&'static str),
    /// Explicitly indirect owned sequence.
    Sequence(&'static IdlValueType),
    /// Inline fixed-size array.
    Array(&'static IdlValueType, u32),
}

/// One source field, including the IDL spelling of its declared value type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdlField {
    /// Field identifier, or the zero-based tuple position as decimal text.
    pub name: &'static str,
    /// IDL type expression; named types retain their source spelling.
    pub ty: IdlValueType,
}

/// One case of a tagged IDL enum message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdlVariant {
    /// Declared case identifier.
    pub name: &'static str,
    /// Exact declared integer discriminant used to select the payload.
    pub tag: i64,
    /// Ordered tuple or named case fields; an empty slice denotes a unit case.
    pub fields: &'static [IdlField],
}

/// One declared bit flag or alias with its exact integer value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdlFlag {
    /// Source-level flag identifier.
    pub name: &'static str,
    /// Explicit nonnegative raw bit pattern.
    pub value: u64,
}

/// Authored aggregate shape, separate from physical ABI layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdlShape {
    /// A named record with fields in declaration order.
    Struct(&'static [IdlField]),
    /// A tagged sum with cases in declaration order.
    Enum(&'static [IdlVariant]),
    /// A transparent set of named integer flags and aliases.
    BitEnum(&'static [IdlFlag]),
}

/// A generated IDL aggregate and its public Rust payload location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdlType {
    /// Exact logical module and declaration name.
    pub qualified_name: &'static str,
    /// Public Rust path generated for application handler signatures.
    pub rust_type: &'static str,
    /// Complete source-level struct, enum, or bit-enum shape.
    pub shape: IdlShape,
}

/// One resolved typed stream, including its inherited COM projection.
#[derive(Clone, Copy)]
pub struct IdlEventStream {
    /// Concrete closed-generic interface; must occur in the class's COM chain.
    pub abi: &'static NcInterfaceDescriptor,
    /// Complete event value carried by the stream's single receiver.
    pub message: &'static IdlType,
}

/// A COM class/interface and its complete transitive authoring contract.
#[derive(Clone, Copy)]
pub struct IdlInterface {
    /// Existing physical interface identity.
    pub abi: &'static NcInterfaceDescriptor,
    /// This projection and required base/sibling descriptors in query order.
    pub chain: &'static [&'static NcInterfaceDescriptor],
    /// Public Rust path of the generated concrete projection.
    pub rust_abi: &'static str,
    /// Sole resolved stream, including one inherited through an interface.
    pub stream: Option<IdlEventStream>,
    /// Finite definitions reachable from this interface chain and event value.
    pub values: &'static [IdlType],
}
