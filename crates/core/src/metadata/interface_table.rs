//! Compile-time entries for canonical, static interface tables.
//!
//! Contract identities determine table ordering, not per-object storage. The implementation
//! parameter keeps every selected vtable tied to the one Rust value type whose
//! allocation its thunks understand. Only the ABI's common vtable pointer is
//! erased; application values and provider types remain statically typed.
use core::marker::PhantomData;
use crate::{IUnknownVTable, NcInterfaceDescriptor, NcInterfaceId};
use super::inheritance::same_identity;

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
}

/// Selects interface table entries in exactly the canonical descriptor order.
///
/// Generated associated constants evaluate this at compilation. Parent tables
/// already contain only unique interfaces. No runtime ancestry traversal, map,
/// erased application value, or per-parent header allocation is needed.
#[doc(hidden)]
pub const fn __nc_interface_table<T, const N: usize>(
    contracts: &[&'static NcInterfaceDescriptor; N],
    own: ComInterfaceTableEntry<T>,
    parents: &[&[ComInterfaceTableEntry<T>]],
) -> [ComInterfaceTableEntry<T>; N] {
    let mut result = [own; N];
    let mut index = 0;
    while index < N {
        let identity = NcInterfaceId::new(contracts[index].interface_id, contracts[index].abi_signature);
        if same_identity(&identity, &own.identity) {
            result[index] = own;
        } else {
            let mut parent = 0;
            let mut found = false;
            // The canonical set has already rejected conflicting revisions.
            // First occurrence therefore selects the same contract through
            // either side of a diamond, with thunks for this exact T.
            'search: while parent < parents.len() {
                let mut item = 0;
                while item < parents[parent].len() {
                    let candidate = parents[parent][item];
                    if same_identity(&identity, &candidate.identity) {
                        result[index] = candidate;
                        found = true;
                        break 'search;
                    }
                    item += 1;
                }
                parent += 1;
            }
            assert!(found, "canonical interface has no interface table entry");
        }
        index += 1;
    }
    result
}

/// Extracts runtime vtable pointers from the compiler-only interface table entries.
///
/// Generated queries scan only this result. Returning pointer words instead
/// of entry records prevents a runtime reference from retaining descriptor graphs
/// or a redundant per-implementation interface-identity array.
#[doc(hidden)]
pub const fn __nc_interface_vtables<T, const N: usize>(entries: &[ComInterfaceTableEntry<T>; N]) -> [*const IUnknownVTable; N] {
    let mut result = [core::ptr::null(); N];
    let mut index = 0;
    while index < N {
        result[index] = entries[index].vtable;
        index += 1;
    }
    result
}
