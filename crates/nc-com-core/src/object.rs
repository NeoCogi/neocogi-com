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
    /// Identity of the actual interface header using this vtable, including its
    /// complete local contract hash. This points to immutable static metadata;
    /// it describes the projected interface, which may differ from IUnknown.
    pub interface_id: *const NcInterfaceId,

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

// SAFETY: Vtables contain only function pointers and immutable static identity
// metadata. Sharing the table does not grant cross-thread access to an object.
unsafe impl Sync for IUnknownVTable {}

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
///   method order, signatures, passing modes, and referenced interface IIDs.
///   Every COM pointer argument/return is checked against its expected full
///   identity before invoking any interface-specific method. The common prefix
///   publishes this actual interface identity in immutable static storage.
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

/// Describes the primary callable ABI of one registered COM class.
///
/// A class contract is also a valid COM pointer projection, so it reuses the
/// same header, vtable, ownership, and safe-call machinery as optional
/// interfaces. The distinction is semantic and registry-visible: a class
/// contract has one construction identity and is the typed result of activating
/// that class. Authors declare it directly with `#[com_class]`; they do not
/// write a matching one-off interface.
///
/// Optional interfaces remain useful when several classes expose the same
/// capability through `QueryInterface`. They are not required merely to call a
/// class's own methods.
///
/// # Safety
///
/// `CLASS_ID` must identify the registered class whose primary projection is
/// `Self`. `QUALIFIED_NAME` and `VERSION` must be the exact validated static
/// text published by that class's descriptor. Implementations must expose
/// every chain in `INTERFACE_CHAINS` on the same identity, including
/// `Self::INTERFACE_ID`, so construction can validate the returned primary
/// projection before safe code receives it.
pub unsafe trait ComClassContract: ComInterface {
    /// Borrowed, typed arguments accepted by this class constructor.
    type Arguments<'a>: ComClassArguments;

    /// Stable registry identity used to activate this class.
    const CLASS_ID: NcGuid;

    /// SHA-256 hash of the class's primary callable ABI.
    const CLASS_ABI_HASH: NcAbiHash = Self::ABI_HASH;

    /// Normative static class name stored in registry metadata.
    const QUALIFIED_NAME: &'static str;

    /// Static class contract version stored in registry metadata.
    const VERSION: &'static str;

    /// Complete interface ancestry lists required by every implementation.
    ///
    /// The callable class projection and each additional class supertrait are
    /// separate `QueryInterface` capabilities on one COM identity.
    const INTERFACE_CHAINS: &'static [&'static [&'static crate::NcInterfaceDescriptor]];

    /// ABI hash of the constructor frame, or zero for no arguments.
    const CONSTRUCTOR_ABI_HASH: NcAbiHash;
}

/// Supplies one immutable, call-scoped constructor frame to the COM runtime.
///
/// Class macros implement this trait for generated argument wrappers. The
/// wrapper retains the Rust lifetimes of borrowed buffers and COM references,
/// while this small erased boundary lets the runtime route any class without a
/// type registry, reflection system, or dynamically typed value bag.
///
/// # Safety
///
/// For non-zero [`Self::ABI_HASH`], [`Self::as_abi_ptr`] must return a non-null
/// pointer to a frame whose exact layout is described by that hash. The frame
/// and every borrow reachable from it must remain valid and immutable through
/// the constructor call. A zero hash must return null and denotes no arguments.
pub unsafe trait ComClassArguments {
    /// ABI hash describing the pointed-to constructor frame.
    const ABI_HASH: NcAbiHash;

    /// Returns the raw frame pointer borrowed for one constructor call.
    fn as_abi_ptr(&self) -> *const c_void;
}

// SAFETY: Unit is the canonical no-argument representation. Its zero hash and
// null pointer agree with the runtime's empty-frame contract.
unsafe impl ComClassArguments for () {
    const ABI_HASH: NcAbiHash = NcAbiHash::ZERO;

    /// Returns null because this value carries no constructor frame.
    fn as_abi_ptr(&self) -> *const c_void {
        core::ptr::null()
    }
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
        base: crate::NcInterfaceId::NONE,
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
            [0x7c, 0x07, 0x6b, 0x86, 0x36, 0xac, 0x22, 0x3b, 0x15, 0x5d, 0xe0, 0x50, 0xeb, 0x04, 0x26, 0x08, 0x09, 0x37, 0x57, 0xb7, 0xd6, 0x9d, 0x09, 0xc0, 0xf6, 0x4c, 0xee, 0xd0, 0xe1, 0xf0, 0xd8, 0x4e,]
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
