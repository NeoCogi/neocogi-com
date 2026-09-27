//! Compile-time provider bindings for canonical, fixed-size object header arrays.
//!
//! Contract identities determine ordering and storage size. The implementation
//! parameter keeps every selected vtable tied to the one Rust value type whose
//! allocation its thunks understand. Only the ABI's common header pointer is
//! erased; application values and provider types remain statically typed.
use core::marker::PhantomData;
use crate::{IUnknownVTable, NcInterfaceDescriptor, NcInterfaceId};
use super::inheritance::same_identity;

/// One exact interface contract and its statically bound provider vtable.
/// Generated associated constants compose these before runtime allocation.
#[doc(hidden)]
pub struct __ComProjection<T> {
    /// Exact identity used only while the compiler selects unique providers.
    /// Deliberately excludes pointers to method, parameter, or type descriptions.
    identity: NcInterfaceId,
    /// Common prefix of the complete, permanently live interface vtable.
    vtable: *const IUnknownVTable,
    /// Prevents accidentally mixing tables for different object implementations.
    implementor: PhantomData<fn() -> T>,
}

// A provider binding borrows immutable compiler-generated tables and owns no T.
impl<T> Copy for __ComProjection<T> {}
impl<T> Clone for __ComProjection<T> {
    /// Copies the static binding without requiring the implementation to be Clone.
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> __ComProjection<T> {
    /// Binds a complete interface vtable to its identity and concrete provider.
    ///
    /// # Safety
    /// The vtable must be permanently live, have an IUnknownVTable prefix, and
    /// implement the exact supplied identity/signature. Every thunk must use
    /// the allocation and lifetime contract of T's generated COM object. Its
    /// interface header must have the same two-field layout as IUnknown.
    pub const unsafe fn new(identity: NcInterfaceId, vtable: *const IUnknownVTable) -> Self {
        Self {
            identity,
            vtable,
            implementor: PhantomData,
        }
    }
}

/// Selects provider bindings in exactly the canonical descriptor order.
///
/// Generated associated constants evaluate this at compilation. Parent tables
/// already contain only unique interfaces. No runtime ancestry traversal, map,
/// erased application value, or per-parent header allocation is needed.
#[doc(hidden)]
pub const fn __nc_projection_table<T, const N: usize>(
    contracts: &[&'static NcInterfaceDescriptor; N],
    own: __ComProjection<T>,
    parents: &[&[__ComProjection<T>]],
) -> [__ComProjection<T>; N] {
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
            assert!(found, "canonical interface has no provider binding");
        }
        index += 1;
    }
    result
}

/// Discards compiler-only identities after binding every header to its typed vtable.
///
/// Generated construction indexes only this result. Returning pointer words instead
/// of provider records prevents a runtime reference from retaining descriptor graphs
/// or a redundant per-implementation interface-identity array.
#[doc(hidden)]
pub const fn __nc_projection_vtables<T, const N: usize>(projections: &[__ComProjection<T>; N]) -> [*const IUnknownVTable; N] {
    let mut result = [core::ptr::null(); N];
    let mut index = 0;
    while index < N {
        result[index] = projections[index].vtable;
        index += 1;
    }
    result
}
