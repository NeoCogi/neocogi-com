//! The root `IUnknown` contract and the marker for C-shaped COM interfaces.
//!
//! This module fixes the canonical `IUnknown` identity, header, vtable prefix,
//! and empty method descriptor. Generated and built-in interfaces implement
//! [`ComInterface`] to describe their identity, vtable type, and pointer-safe
//! conversion back to the shared `IUnknown` header.

use core::ffi::c_void;

use crate::guid::NcGuid;
use crate::hash::NcAbiHash;
use crate::metadata::{NcAbiHashPart, NcInterfaceDescriptor, NcInterfaceId, NcStaticUtf8};
use crate::status::NcStatus;

/// Interface identifier for [`IUnknown`].
pub const IID_IUNKNOWN: NcGuid = NcGuid::from_fields(
    0x0000_0000,
    0x0000,
    0x0000,
    [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
);

/// Reserved normative qualified name for [`IUnknown`].
pub const IUNKNOWN_QUALIFIED_NAME: &str = "IUnknown";

/// ABI contract hash for [`IUnknown`].
pub const IUNKNOWN_ABI_HASH: NcAbiHash =
    NcAbiHash::from_abi_parts(&[NcAbiHashPart::Interface { iid: IID_IUNKNOWN }]);

/// Interface identity for [`IUnknown`].
pub const IUNKNOWN_INTERFACE_ID: NcInterfaceId =
    NcInterfaceId::new(IID_IUNKNOWN, IUNKNOWN_ABI_HASH);

/// The base interface implemented by every COM object.
#[repr(C)]
pub struct IUnknown {
    /// Function table for `IUnknown`.
    pub vtable: *const IUnknownVTable,
    /// Opaque pointer to generated object storage.
    pub object: *mut c_void,
}

/// C-compatible vtable for [`IUnknown`].
#[repr(C)]
pub struct IUnknownVTable {
    /// Queries the object for another interface by identity.
    ///
    /// # Safety
    ///
    /// `this` must be a live `IUnknown` header for the object being queried.
    /// `interface_id` must point to the expected interface GUID/hash pair, and
    /// `out` must point to a distinct slot valid for one pointer write. Given
    /// those preconditions, the implementation initializes `*out` to null.
    /// `NC_OK` replaces it with one non-null owned interface reference that the
    /// caller must eventually release; every other returned status leaves it
    /// null and transfers no reference. Null or otherwise invalid arguments
    /// violate this safety contract even when an implementation rejects them
    /// defensively.
    pub query_interface: unsafe extern "C" fn(
        this: *mut IUnknown,
        interface_id: *const NcInterfaceId,
        out: *mut *mut c_void,
    ) -> NcStatus,

    /// Increments the object's reference count and returns the new count.
    ///
    /// # Safety
    ///
    /// `this` must be a live `IUnknown` header. The caller only needs a
    /// borrowed reference for the duration of the call; success creates one new
    /// owned reference.
    pub add_ref: unsafe extern "C" fn(this: *mut IUnknown) -> u32,

    /// Decrements the object's reference count and returns the remaining count.
    ///
    /// # Safety
    ///
    /// `this` must own one live reference. The call consumes that reference and
    /// may destroy the object immediately when the remaining count reaches zero.
    pub release: unsafe extern "C" fn(this: *mut IUnknown) -> u32,
}

/// Metadata and pointer conversion implemented by every COM ABI interface type.
///
/// Prefer defining interfaces with `#[com_interface]` from the `nc-com` crate.
/// The macro generates this implementation together with the interface header,
/// vtable, identity, and ABI hash. The trait remains public and unsealed for
/// low-level foreign-interface adapters, but handwritten implementations are an
/// unsafe extension point.
///
/// Sealing cannot distinguish macro output from handwritten downstream code:
/// an attribute procedural macro expands to ordinary code in the consumer
/// crate and has no privileged access to a private sealing supertrait. A private
/// seal would block the generated implementation too, while a public hidden seal
/// would provide no enforcement. The unsafe implementation boundary prevents
/// safe handwritten implementations without blocking generated interfaces.
///
/// # Safety
///
/// An implementation is a proof that all of the following invariants hold:
///
/// - `Self` is the exact C-compatible interface header identified by
///   [`Self::INTERFACE_ID`]. Its first field is a live pointer to `Self::VTable`,
///   and the vtable starts with a valid [`IUnknownVTable`] prefix.
/// - `Self::INTERFACE_ID`, including its ABI hash, describes the actual vtable
///   method order, signatures, passing modes, and referenced interface contracts.
/// - Every live `*mut Self` stored in a [`crate::ComPtr`] or [`crate::ComRef`]
///   remains backed by its interface header, vtable, and object storage for the
///   reference's lifetime.
/// - A successful `QueryInterface` for `Self::INTERFACE_ID` returns a genuine
///   `Self` header and transfers exactly one owned reference.
/// - [`ComInterface::iunknown`] performs only the representation-preserving
///   pointer conversion. It must not dereference `this`, must map null to null,
///   and must map every live `Self` pointer to the live [`IUnknown`] header for
///   the same object and reference-count identity.
///
/// Violating these invariants can make safe operations such as
/// [`crate::ComPtr::query`], `Clone`, or `Drop` cause undefined behavior.
pub unsafe trait ComInterface {
    /// Stable interface identity used by `QueryInterface`.
    const INTERFACE_ID: NcInterfaceId;

    /// Stable interface identifier used by `QueryInterface`.
    const IID: NcGuid = Self::INTERFACE_ID.iid;

    /// SHA-256 hash of this interface's typed ABI contract.
    const ABI_HASH: NcAbiHash = Self::INTERFACE_ID.abi_hash;

    /// The C-compatible vtable type for this interface.
    type VTable;

    /// Converts an interface pointer to its `IUnknown` header.
    fn iunknown(this: *mut Self) -> *mut IUnknown;
}

// SAFETY: `IUnknown` is the canonical interface header and vtable prefix. Its
// pointer conversion is the identity operation, and the constants describe the
// exact built-in root-interface contract.
unsafe impl ComInterface for IUnknown {
    const INTERFACE_ID: NcInterfaceId = IUNKNOWN_INTERFACE_ID;

    type VTable = IUnknownVTable;

    /// Returns the pointer unchanged because this is already `IUnknown`.
    fn iunknown(this: *mut Self) -> *mut IUnknown {
        this
    }
}

impl IUnknown {
    /// SHA-256 hash of this interface's typed ABI contract.
    pub const ABI_HASH: NcAbiHash = IUNKNOWN_ABI_HASH;
    /// Stable identity for this interface ABI contract.
    pub const INTERFACE_ID: NcInterfaceId = IUNKNOWN_INTERFACE_ID;
    /// Canonical zero-method descriptor for the root COM interface.
    ///
    /// Every class descriptor contains exactly one mapping to this interface
    /// contract. `#[com_class]` inserts that mapping automatically; users list
    /// only the class's non-root interfaces.
    pub const INTERFACE_DESCRIPTOR: NcInterfaceDescriptor = NcInterfaceDescriptor {
        interface_id: IID_IUNKNOWN,
        qualified_name: NcStaticUtf8::from_static(IUNKNOWN_QUALIFIED_NAME),
        abi_hash: IUNKNOWN_ABI_HASH,
        method_count: 0,
        methods: core::ptr::null(),
    };
}

#[cfg(test)]
mod tests {
    use super::{
        ComInterface, IID_IUNKNOWN, IUNKNOWN_INTERFACE_ID, IUNKNOWN_QUALIFIED_NAME, IUnknown,
    };

    #[test]
    fn iunknown_interface_identity_is_stable() {
        assert_eq!(IID_IUNKNOWN, <IUnknown as ComInterface>::IID);
        assert_eq!(
            IID_IUNKNOWN.to_uuid_bytes(),
            [
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x46,
            ]
        );
        assert_eq!(
            IUnknown::ABI_HASH.bytes,
            [
                0xd2, 0x2a, 0x3f, 0x97, 0x21, 0xef, 0x1e, 0x10, 0x8f, 0x84, 0x76, 0xe8, 0xdd, 0xb7,
                0x45, 0x4e, 0xe8, 0x02, 0x48, 0x59, 0xca, 0xa8, 0x3f, 0xb9, 0x7e, 0xe0, 0xa5, 0x35,
                0xb6, 0x50, 0x7e, 0x27,
            ]
        );
        assert_eq!(
            IUNKNOWN_INTERFACE_ID,
            <IUnknown as ComInterface>::INTERFACE_ID
        );
        assert_eq!(IUNKNOWN_INTERFACE_ID, IUnknown::INTERFACE_ID);
        assert_eq!(IUnknown::INTERFACE_DESCRIPTOR.interface_id, IID_IUNKNOWN);
        assert_eq!(
            unsafe { IUnknown::INTERFACE_DESCRIPTOR.qualified_name.as_str() },
            Ok(IUNKNOWN_QUALIFIED_NAME)
        );
        assert_eq!(IUnknown::INTERFACE_DESCRIPTOR.abi_hash, IUnknown::ABI_HASH);
        assert_eq!(IUnknown::INTERFACE_DESCRIPTOR.method_count, 0);
        assert!(IUnknown::INTERFACE_DESCRIPTOR.methods.is_null());
    }

    #[test]
    fn iunknown_pointer_conversion_is_null_preserving_and_non_dereferencing() {
        assert!(IUnknown::iunknown(core::ptr::null_mut()).is_null());

        let raw = core::ptr::NonNull::<IUnknown>::dangling().as_ptr();
        assert_eq!(IUnknown::iunknown(raw), raw);
    }
}
