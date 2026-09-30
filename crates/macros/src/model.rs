//! Normalized method model shared by parsing and code generation.
//!
//! `syn` syntax is lowered once into this representation so vtable generation,
//! safe wrappers, descriptor metadata, and generated documentation all agree on
//! ownership and pointer shape. Keeping the original Rust type beside its ABI
//! type allows diagnostics and wrappers to use the source-level spelling.

use syn::{Attribute, Ident, Type};

/// One validated COM method after source syntax has been lowered.
pub(crate) struct AbiMethod {
    /// Method name, also used as the generated vtable field name.
    pub(crate) ident: Ident,
    /// User documentation propagated to generated safe and raw methods.
    pub(crate) doc_attrs: Vec<Attribute>,
    /// Borrowing permission required while dispatching the method.
    pub(crate) receiver: AbiReceiver,
    /// Parameters in source and vtable order.
    pub(crate) args: Vec<AbiArg>,
    /// Successful payload; failure is always represented by `NcStatus`.
    pub(crate) ok: AbiReturn,
}

/// Borrowing mode declared by one COM method receiver.
#[derive(Clone, Copy)]
pub(crate) enum AbiReceiver {
    /// The method observes its implementation through a shared borrow.
    Shared,
    /// The method may mutate its implementation through an exclusive borrow.
    Mutable,
}

/// One method parameter and its ABI lowering.
pub(crate) struct AbiArg {
    /// Source-level parameter identifier.
    pub(crate) ident: Ident,
    /// Rust type appearing in the safe trait method.
    pub(crate) rust_ty: Type,
    /// Raw type appearing in the generated C vtable slot.
    pub(crate) abi_ty: Type,
    /// Ownership and pointer transformation applied during lowering.
    pub(crate) kind: AbiArgKind,
}

/// Ownership/passing rule selected for a method parameter.
pub(crate) enum AbiArgKind {
    /// Unique owner offered through a mutable slot; the receiver clears it when
    /// taking ownership, so all pre-dispatch rejection paths remain leak-free.
    ComBox {
        /// Exact generated interface transferred into the receiving object.
        interface_ty: Box<Type>,
        /// Provider threading guarantee preserved by the transferred owner.
        mode_ty: Option<Box<Type>>,
    },
    /// Concrete generated message borrowed through its own C record.
    BorrowedValue { value_ty: Box<Type> },
    /// ABI-safe scalar or aggregate passed directly by value.
    Plain,
    /// Call-scoped source buffer lowered to a const element pointer plus bounds.
    CopyFromBufferOffset { element_ty: Box<Type> },
    /// Call-scoped destination buffer lowered to a mutable element pointer plus bounds.
    CopyToBufferOffset { element_ty: Box<Type> },
    /// Borrowed interface lowered to its raw ABI pointer.
    ComRef {
        /// Whether a null pointer denotes an explicitly absent capability.
        optional: bool,
        /// Generated ABI interface type borrowed by the parameter.
        interface_ty: Box<Type>,
        /// Optional explicit `ThreadLocal` or `ThreadSafe` marker.
        mode_ty: Option<Box<Type>>,
    },
}

/// Successful method return and its ABI lowering.
pub(crate) struct AbiReturn {
    /// Success type appearing in the source-level `Result`.
    pub(crate) rust_ty: Type,
    /// Raw output-pointee type, or unit for a status-only method.
    pub(crate) abi_ty: Type,
    /// Ownership and output-slot transformation applied during lowering.
    pub(crate) kind: AbiReturnKind,
}

/// Ownership/passing rule selected for a successful return.
pub(crate) enum AbiReturnKind {
    /// Status-only `Result<(), NcStatus>` with no output pointer.
    Void,
    /// ABI-safe value returned through an output pointer.
    Plain,
    /// Owned interface reference transferred through an output pointer.
    ComPtr {
        /// Distinguishes shared owners, unique owners, and weak control tokens.
        ownership: InterfaceOwnership,
        /// Generated ABI interface type transferred to the caller.
        interface_ty: Box<Type>,
        /// Optional explicit `ThreadLocal` or `ThreadSafe` marker.
        mode_ty: Option<Box<Type>>,
    },
}

/// Lifetime token transferred by an object-valued method result.
#[derive(Clone, Copy)]
pub(crate) enum InterfaceOwnership {
    /// Another shared owner whose lifetime ends with Release.
    Shared,
    /// The one exclusive owner; queries cannot duplicate this token.
    Exclusive,
    /// A cached observer that retains storage but never the concrete value.
    Weak,
}
