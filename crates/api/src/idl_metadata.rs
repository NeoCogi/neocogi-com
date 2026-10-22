//! Static descriptions of generated IDL interfaces and value types.
//!
//! The compiler attaches `IDL_INTERFACE` to an interface's generated ABI marker
//! and `IDL_TYPE` to a generated record or enum. These constants describe method
//! names, receiver access, argument/result ownership and reachable value fields.
//! Tools can read that information without depending on the IDL compiler.
//!
//! Records and their references stay inside the binary that defines them. A
//! domain may copy their contents into its own transport format; these Rust
//! structs are not a cross-library ABI. Physical descriptors and signatures
//! remain responsible for COM call compatibility. This module stores no global
//! registry and assigns no application meaning to methods or value types.

use crate::{NcAbiReceiver, NcInterfaceDescriptor};

/// Structural field expression; named edges refer to the finite definition table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueType {
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
    Sequence(&'static ValueType),
    /// Inline fixed-size array.
    Array(&'static ValueType, u32),
}

/// One declared field paired with its resolved structural type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldMetadata {
    /// Field identifier, or the zero-based tuple position as decimal text.
    pub name: &'static str,
    /// Structural type expression; named edges use canonical qualified identities.
    pub ty: ValueType,
}

/// One case of a tagged IDL enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VariantMetadata {
    /// Declared case identifier.
    pub name: &'static str,
    /// Exact declared integer discriminant used to select the payload.
    pub tag: i64,
    /// Ordered tuple or named case fields; an empty slice denotes a unit case.
    pub fields: &'static [FieldMetadata],
}

/// One declared bit flag or alias with its exact integer value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlagMetadata {
    /// Source-level flag identifier.
    pub name: &'static str,
    /// Explicit nonnegative raw bit pattern.
    pub value: u64,
}

/// Authored aggregate shape, separate from physical ABI layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeShape {
    /// A named record with fields in declaration order.
    Struct(&'static [FieldMetadata]),
    /// A tagged sum with cases in declaration order.
    Enum(&'static [VariantMetadata]),
    /// A transparent set of named integer flags and aliases.
    BitEnum(&'static [FlagMetadata]),
}

/// A generated IDL aggregate and its public Rust payload location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeMetadata {
    /// Exact logical module and declaration name.
    pub qualified_name: &'static str,
    /// Public Rust path for tools that generate typed consumers.
    /// Readers apply [`crate::validate_rust_path`]; this spelling does not enter
    /// logical identity or establish that the reader can resolve the type.
    pub rust_type: &'static str,
    /// Complete source-level struct, enum, or bit-enum shape.
    pub shape: TypeShape,
}

/// Source-level ownership and borrowing of one method argument or success result.
/// Value graphs describe repeatable data; call-only transfers remain explicit here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodType {
    /// Repeatable by-value data or a retained shared/weak capability.
    Value(ValueType),
    /// One exclusive interface owner transferred through the call.
    ExclusiveObject(&'static str),
    /// Call-scoped view of data or an interface, without escaping ownership.
    Borrowed(ValueType),
    /// Read-only caller-provided element sequence.
    InputBuffer(ValueType),
    /// Caller-provided storage initialized through checked output completion.
    OutputBuffer(ValueType),
}

/// One named method parameter with its source-level call ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParameterMetadata {
    /// Authored parameter identifier, excluding the receiver.
    pub name: &'static str,
    /// Semantic argument shape independent of its physical pointer/count slots.
    pub ty: MethodType,
}

/// Ordinary method introspection, including inherited methods once per interface identity.
/// This describes callable contracts without assigning application event semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodMetadata {
    /// Authored method identifier.
    pub name: &'static str,
    /// Shared or exclusive access required to call the implementation.
    pub receiver: NcAbiReceiver,
    /// Ordered logical parameters, excluding the receiver.
    pub parameters: &'static [ParameterMetadata],
    /// Successful return shape; failures use the ordinary COM status protocol.
    pub returns: MethodType,
}

/// One callable interface, inherited methods and reachable value definitions.
#[derive(Clone, Copy)]
pub struct InterfaceMetadata {
    /// Existing physical interface identity.
    pub abi: &'static NcInterfaceDescriptor,
    /// This projection and required base/sibling descriptors in query order.
    pub chain: &'static [&'static NcInterfaceDescriptor],
    /// Public Rust path of the generated concrete projection.
    /// Uses the same [`crate::validate_rust_path`] contract as value bindings.
    pub rust_abi: &'static str,
    /// Base-first method descriptions, with shared ancestors included once.
    pub methods: &'static [MethodMetadata],
    /// Finite definitions reachable from this interface chain and method types.
    pub values: &'static [TypeMetadata],
}
