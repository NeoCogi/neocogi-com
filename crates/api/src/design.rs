//! Source-level IDL descriptions attached to generated COM interface types.
//!
//! These immutable records supplement explicit method diagnostic descriptors:
//! ABI signatures identify compatibility, while these records
//! retain declared method, struct, and enum names needed by authoring tools.
//! Generated records stay local to their defining binary. A plugin may copy
//! their values into a versioned COM design catalog for unlinked authoring hosts.
//! `neocogi-inspect` reads plugin manifests, not this graph or application layouts.

use crate::{NcAbiReceiver, NcInterfaceDescriptor};

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
    /// Canonical interface identity of shared weak bookkeeping, without a value owner.
    WeakObject(&'static str),
    /// Canonical interface identity of an exclusive weak observer, without an execution lease.
    WeakBoxObject(&'static str),
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
    /// Publishers and catalog readers apply [`crate::validate_rust_path`]; this
    /// spelling is separate from the logical identity and application type resolution.
    pub rust_type: &'static str,
    /// Complete source-level struct, enum, or bit-enum shape.
    pub shape: IdlShape,
}

/// Source-level ownership and borrowing of one method argument or success result.
/// Value graphs describe repeatable data; call-only transfers remain explicit here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdlMethodType {
    /// Repeatable by-value data or a retained shared/weak capability.
    Value(IdlValueType),
    /// One exclusive interface owner transferred through the call.
    ExclusiveObject(&'static str),
    /// Call-scoped view of data or an interface, without escaping ownership.
    Borrowed(IdlValueType),
    /// Read-only caller-provided element sequence.
    InputBuffer(IdlValueType),
    /// Caller-provided storage initialized through checked output completion.
    OutputBuffer(IdlValueType),
}

/// One named method parameter with its source-level call ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdlParameter {
    /// Authored parameter identifier, excluding the receiver.
    pub name: &'static str,
    /// Semantic argument shape independent of its physical pointer/count slots.
    pub ty: IdlMethodType,
}

/// Ordinary method introspection, including inherited methods once per interface identity.
/// This describes callable contracts without assigning application event semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdlMethod {
    /// Authored method identifier.
    pub name: &'static str,
    /// Shared or exclusive access required to call the implementation.
    pub receiver: NcAbiReceiver,
    /// Ordered logical parameters, excluding the receiver.
    pub parameters: &'static [IdlParameter],
    /// Successful return shape; failures use the ordinary COM status protocol.
    pub returns: IdlMethodType,
}

/// A COM class/interface and its complete transitive authoring contract.
#[derive(Clone, Copy)]
pub struct IdlInterface {
    /// Existing physical interface identity.
    pub abi: &'static NcInterfaceDescriptor,
    /// This projection and required base/sibling descriptors in query order.
    pub chain: &'static [&'static NcInterfaceDescriptor],
    /// Public Rust path of the generated concrete projection.
    /// Uses the same [`crate::validate_rust_path`] contract as value bindings.
    pub rust_abi: &'static str,
    /// Base-first method descriptions, with shared ancestors included once.
    pub methods: &'static [IdlMethod],
    /// Finite definitions reachable from this interface chain and method types.
    pub values: &'static [IdlType],
}
