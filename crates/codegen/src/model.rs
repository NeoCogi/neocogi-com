//! Normalized method model shared by parsing and code generation.
//!
//! `syn` syntax is lowered once into this representation so vtable generation,
//! safe wrappers, descriptor metadata, and generated documentation all agree on
//! ownership and pointer shape. Keeping the original Rust type beside its ABI
//! type allows diagnostics and wrappers to use the source-level spelling.

use syn::{Attribute, Ident, Type};

/// One validated COM method after source syntax has been lowered.
pub struct AbiMethod {
    /// Method name, also used as the generated vtable field name.
    pub ident: Ident,
    /// User documentation propagated to generated safe and raw methods.
    pub doc_attrs: Vec<Attribute>,
    /// Borrowing permission required while dispatching the method.
    pub receiver: AbiReceiver,
    /// Parameters in source and vtable order.
    pub args: Vec<AbiArg>,
    /// Successful payload; failure is always represented by `NcStatus`.
    pub ok: AbiReturn,
}

/// Borrowing mode declared by one COM method receiver.
#[derive(Clone, Copy)]
pub enum AbiReceiver {
    /// The method observes its implementation through a shared borrow.
    Shared,
    /// The method may mutate its implementation through an exclusive borrow.
    Mutable,
}

/// One method parameter and its ABI lowering.
pub struct AbiArg {
    /// Source-level parameter identifier.
    pub ident: Ident,
    /// Positional local used only by generated clients, vtable slots and thunks.
    /// Separate from `ident` so source names never enter implementation scopes,
    /// including when expanded tokens are printed and reparsed without hygiene.
    pub binding: Ident,
    /// Rust type appearing in the safe trait method.
    pub rust_ty: Type,
    /// Raw type appearing in the generated C vtable slot.
    pub abi_ty: Type,
    /// Ownership and pointer transformation applied during lowering.
    pub kind: AbiArgKind,
}

/// Ownership/passing rule selected for a method parameter.
pub enum AbiArgKind {
    /// Unique owner passed through a writable C slot. Success consumes it;
    /// every rejection restores the original owner, including provider errors.
    /// The Rust signature exposes ComBox and an ownership-bearing error tuple.
    ComBox {
        /// Exact generated interface transferred into the receiving object.
        interface_ty: Box<Type>,
        /// Provider threading guarantee preserved by the transferred owner.
        mode_ty: Option<Box<Type>>,
    },
    /// Concrete generated message borrowed through its own C record.
    BorrowedValue {
        /// Concrete application value whose checked wire is borrowed.
        value_ty: Box<Type>,
    },
    /// ABI-safe scalar or aggregate passed directly by value.
    Plain,
    /// Call-scoped source buffer lowered to a const element pointer plus bounds.
    CopyFromBufferOffset {
        /// Application element acquired from each checked readable wire.
        element_ty: Box<Type>,
    },
    /// Write-only copied output lowered to offset, capacity, wire pointer and prefix-count pointer.
    OutputBuffer {
        /// Application element converted into each fresh output slot.
        element_ty: Box<Type>,
    },
    /// Required call-scoped interface lowered to its raw object/table pair.
    ComRef {
        /// Generated ABI interface type borrowed by the parameter.
        interface_ty: Box<Type>,
        /// Optional explicit `ThreadLocal` or `ThreadSafe` marker.
        mode_ty: Option<Box<Type>>,
    },
}

/// Successful method return and its ABI lowering.
pub struct AbiReturn {
    /// Success type appearing in the source-level `Result`.
    pub rust_ty: Type,
    /// Raw output-pointee type, or unit for a status-only method.
    pub abi_ty: Type,
    /// Ownership and output-slot transformation applied during lowering.
    pub kind: AbiReturnKind,
}

/// Ownership/passing rule selected for a successful return.
pub enum AbiReturnKind {
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
pub enum InterfaceOwnership {
    /// Another shared owner whose lifetime ends with Release.
    Shared,
    /// The one exclusive owner; queries cannot duplicate this token.
    Exclusive,
    /// A cached observer that retains storage but never the concrete value.
    WeakExclusive,
    /// One shared weak header token; promotion is required before value access.
    WeakShared,
}

/// Complete callable contract supplied by the bound IDL graph.
/// Identity, signature and bases are required resolved data, never syntax options.
pub struct InterfacePlan {
    /// Public provider trait name, retaining the compiler's source location.
    pub name: Ident,
    /// Visibility of every public companion emitted for this declaration.
    pub visibility: syn::Visibility,
    /// Declaration documentation and source-navigation attributes.
    pub docs: Vec<Attribute>,
    /// Canonical non-nil callable identity validated by the resolver.
    pub id: neocogi_com_core::NcGuid,
    /// Computed declaration signature in the current canonical grammar.
    pub signature: [u8; 32],
    /// Canonical logical declaration name for explicit inspection.
    pub qualified_name: String,
    /// Minimum threading capability promised by this interface.
    pub threading: crate::InterfaceThreading,
    /// Ordered bound provider paths; first entry supplies the physical prefix.
    pub bases: Vec<syn::Path>,
    /// One authoritative ownership/wire plan for traits, calls and descriptions.
    pub methods: Vec<AbiMethod>,
}
