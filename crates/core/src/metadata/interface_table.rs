//! Compile-time entries for canonical, static interface tables.
//!
//! Contract identities determine table ordering, not per-object storage. The implementation
//! parameter keeps every selected vtable tied to the one Rust value type whose
//! allocation its thunks understand. Only the ABI's common vtable pointer is
//! erased; application values and provider types remain statically typed.
use core::marker::PhantomData;
use crate::{IUnknown, IUnknownVTable, NcInterfaceDescriptor, NcInterfaceId};

/// One interface identity and its vtable for the concrete implementation `T`.
///
/// Generated `INTERFACE_TABLE` constants compose these entries at compile time.
/// `T` prevents combining vtables for incompatible object allocations, without
/// storing or owning a value. Runtime queries use only the resulting vtable
/// pointers; this Rust helper is not part of the C ABI.
#[doc(hidden)]
pub struct ComInterfaceTableEntry<T> {
    /// Exact identity used only while the compiler selects unique providers.
    /// Deliberately excludes pointers to method, parameter, or type descriptions.
    identity: NcInterfaceId,
    /// Common prefix of the complete, permanently live interface vtable.
    vtable: *const IUnknownVTable,
    /// Prevents accidentally mixing tables for different object implementations.
    implementor: PhantomData<fn() -> T>,
}

// An entry borrows immutable compiler-generated tables and owns no T.
impl<T> Copy for ComInterfaceTableEntry<T> {}
impl<T> Clone for ComInterfaceTableEntry<T> {
    /// Copies the static entry without requiring the implementation to be Clone.
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> ComInterfaceTableEntry<T> {
    /// Creates an entry for a complete interface vtable and its exact identity.
    ///
    /// # Safety
    /// The vtable must be permanently live, have an IUnknownVTable prefix, and
    /// implement the exact supplied identity/signature. Every thunk must use
    /// the allocation and lifetime contract of T's generated COM object.
    /// Methods and lifecycle slots receive the allocation address directly.
    pub const unsafe fn new(identity: NcInterfaceId, vtable: *const IUnknownVTable) -> Self {
        Self {
            identity,
            vtable,
            implementor: PhantomData,
        }
    }
    /// Verifies an indexed entry against the descriptor from the same projection.
    #[doc(hidden)]
    pub const fn checked_contract(self, descriptor: &NcInterfaceDescriptor) -> Self {
        assert!(
            same_identity(&self.identity, &descriptor_identity(descriptor)),
            "interface projection table entry mismatch"
        );
        self
    }
}

/// Reads only the identity part of a physical projection descriptor in const code.
const fn descriptor_identity(descriptor: &NcInterfaceDescriptor) -> NcInterfaceId {
    NcInterfaceId::new(descriptor.interface_id, descriptor.abi_signature)
}

/// Checks the root's unique entry against the implicit IUnknown and earlier
/// independent roots. The first authored supplier keeps its index on duplicates.
const fn seen_in_earlier_roots(chains: &[&[&NcInterfaceDescriptor]], root: usize, entry: usize) -> bool {
    let current = descriptor_identity(chains[root][entry]);
    if same_identity(&current, &IUnknown::INTERFACE_ID) {
        return true;
    }
    let mut earlier = 0;
    while earlier < root {
        let mut index = 0;
        while index < chains[earlier].len() {
            if same_identity(&current, &descriptor_identity(chains[earlier][index])) {
                return true;
            }
            index += 1;
        }
        earlier += 1;
    }
    false
}

/// Counts canonical IUnknown and the distinct projections of already-unique roots.
///
/// Rust attribute macros can compose roots that were resolved independently.
/// Only cross-root duplicates need comparison, so one wide root is linear.
#[doc(hidden)]
pub const fn __nc_count_object_interfaces(chains: &[&[&NcInterfaceDescriptor]]) -> usize {
    let mut count = 1;
    let mut root = 0;
    while root < chains.len() {
        let mut entry = 0;
        while entry < chains[root].len() {
            if !seen_in_earlier_roots(chains, root, entry) {
                count += 1;
            }
            entry += 1;
        }
        root += 1;
    }
    assert!(count <= u32::MAX as usize, "object interface count exceeds the ABI limit");
    count
}

/// Assembles compact runtime pointers in canonical query order at compile time.
///
/// Descriptors and typed entries share their selected root's ordered positions.
/// Even a discarded duplicate must have the exact descriptor and provider pair.
#[doc(hidden)]
pub const fn __nc_object_interface_vtables<T, const N: usize>(
    unknown: *const IUnknownVTable,
    chains: &[&[&NcInterfaceDescriptor]],
    providers: &[&[ComInterfaceTableEntry<T>]],
) -> [*const IUnknownVTable; N] {
    assert!(chains.len() == providers.len(), "object has differing interface and provider root counts");
    let mut result = [unknown; N];
    let mut output = 1;
    let mut root = 0;
    while root < chains.len() {
        assert!(
            chains[root].len() == providers[root].len(),
            "interface and provider chains have differing lengths"
        );
        let mut entry = 0;
        while entry < chains[root].len() {
            let provider = providers[root][entry].checked_contract(chains[root][entry]);
            if !seen_in_earlier_roots(chains, root, entry) {
                assert!(output < N, "object interface projection count is too small");
                result[output] = provider.vtable;
                output += 1;
            }
            entry += 1;
        }
        root += 1;
    }
    assert!(output == N, "invalid object interface projection count");
    result
}

/// Rejects incompatible digests once two contracts claim the same nominal IID.
const fn same_identity(left: &NcInterfaceId, right: &NcInterfaceId) -> bool {
    if !left.iid.const_eq(right.iid) {
        return false;
    }
    assert!(left.abi_signature.__const_eq(right.abi_signature), "conflicting inherited interface contracts");
    true
}
