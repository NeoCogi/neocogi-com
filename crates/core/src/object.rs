//! The root `IUnknown` contract and the marker for C-shaped COM interfaces.
//!
//! This module fixes the canonical `IUnknown` identity, header, vtable prefix,
//! and empty method descriptor. Generated and built-in interfaces implement
//! [`ComInterface`] to describe their identity, vtable type, and pointer-safe
//! conversion back to the shared `IUnknown` header.

use core::ffi::c_void;

use crate::guid::NcGuid;
use crate::metadata::{NcInterfaceDescriptor, NcInterfaceId, NcStaticUtf8};
use crate::ptr::ComPtr;
use crate::signature::NcAbiSignature;
use crate::status::NcStatus;
use crate::{ThreadLocal, ThreadSafe, ThreadingModel};

/// Interface identifier for [`IUnknown`].
pub const IID_IUNKNOWN: NcGuid = NcGuid::from_fields(0x0000_0000, 0x0000, 0x0000, [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46]);

/// Reserved normative qualified name for [`IUnknown`].
pub const IUNKNOWN_QUALIFIED_NAME: &str = "IUnknown";

/// Precomputed SHA-256 signature for the built-in [`IUnknown`] contract.
///
/// Built-ins have no IDL build script, so this checked-in constant plays the
/// same role as the byte string emitted by `neocogi` for ordinary contracts.
static IUNKNOWN_SIGNATURE_BYTES: [u8; NcAbiSignature::SHA256_LEN] = [
    0, 26, 125, 27, 153, 74, 143, 106, 94, 128, 109, 209, 111, 235, 46, 62, 237, 132, 84, 36, 240, 76, 210, 0, 33, 186, 162, 232, 218, 82, 142, 198,
];

/// Canonical ABI validation contract for [`IUnknown`].
pub const IUNKNOWN_ABI_SIGNATURE: NcAbiSignature = NcAbiSignature::from_static(&IUNKNOWN_SIGNATURE_BYTES);

/// Interface identity for [`IUnknown`].
pub const IUNKNOWN_INTERFACE_ID: NcInterfaceId = NcInterfaceId::new(IID_IUNKNOWN, IUNKNOWN_ABI_SIGNATURE);

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
    /// complete local contract signature. This points to immutable static metadata;
    /// it describes the projected interface, which may differ from IUnknown.
    pub interface_id: *const NcInterfaceId,

    /// Queries the object for another interface by identity.
    ///
    /// # Safety
    ///
    /// `this` must be a live `IUnknown` header for the object being queried.
    /// `interface_id` must point to the expected interface GUID/signature pair, and
    /// `out` must point to a distinct slot valid for one pointer write. Given
    /// those preconditions, the implementation initializes `*out` to null.
    /// `NC_OK` replaces it with one non-null owned interface reference that the
    /// caller must eventually release; every other returned status leaves it
    /// null and transfers no reference. Null or otherwise invalid arguments
    /// violate this safety contract even when an implementation rejects them
    /// defensively.
    pub query_interface: unsafe extern "C" fn(this: *mut IUnknown, interface_id: *const NcInterfaceId, out: *mut *mut c_void) -> NcStatus,

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
/// Prefer defining interfaces with `#[com_interface]` from the `neocogi-com` crate.
/// The macro generates this implementation together with the interface header,
/// vtable, identity, and ABI signature. The trait remains public and unsealed for
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
/// - `Self::INTERFACE_ID`, including its ABI signature, describes the actual vtable
///   method order, signatures, passing modes, and referenced interface IIDs.
///   Every COM pointer argument/return is checked against its expected full
///   identity before invoking any interface-specific method. The common prefix
///   publishes this actual interface identity in immutable static storage.
/// - `Self::Threading` is the minimum implementation guarantee. Shared
///   contracts require thread-safe calls, ownership, and destruction; a
///   pointer may retain a stronger guarantee or explicitly weaken it.
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

    /// Canonical validation form of this interface's typed ABI contract.
    const ABI_SIGNATURE: NcAbiSignature = Self::INTERFACE_ID.abi_signature;

    /// Minimum threading capability required of an implementation.
    type Threading: ThreadingModel;

    /// The C-compatible vtable type for this interface.
    type VTable;

    /// Converts an interface pointer to its `IUnknown` header.
    fn iunknown(this: *mut Self) -> *mut IUnknown;
}

/// An interface eligible to serve as a plugin's zero-argument root.
/// Typed methods on this interface construct every parameterized object.
///
/// # Safety
///
/// Implementors must be generated from an IDL `factory` declaration. The
/// marker certifies construction policy without changing the interface ABI.
pub unsafe trait ComFactoryInterface: ComInterface {}

/// Compile-time connection between a Rust factory implementation and its IDL interface.
///
/// `#[com_factory(Factory)]` is the normal and only generated implementation.
/// Plugin description macros use this association to obtain the root factory ID
/// and threading capability without asking implementation code to repeat them.
///
/// # Safety
///
/// `Interface` must be the exact IDL factory whose callable interfaces are
/// implemented by `Self`'s generated COM storage.
pub unsafe trait ComFactoryImplementation {
    /// Constructor-free IDL factory interface implemented by this Rust type.
    type Interface: ComFactoryInterface;
}

/// Safe construction policy for a plugin's root factory implementation.
///
/// `#[com_factory]` implements this trait automatically for unit structs. A
/// stateful factory can implement it explicitly and initialize private state
/// without exposing that state or an erased constructor frame through the ABI.
/// The typed return guarantees that plugin activation cannot accidentally
/// publish an unrelated COM object as its declared root factory.
pub trait ComFactoryBootstrap: ComFactoryImplementation + Sized {
    /// Creates one owned instance of the exact associated factory interface.
    fn create_factory() -> Result<ComPtr<Self::Interface, <Self::Interface as ComInterface>::Threading>, NcStatus>;
}

/// Compile-time proof that an implementation satisfies a interface's mode.
///
/// This hidden helper lets generated object storage derive threading from the
/// IDL interface. Local objects need only own `'static` state, while shared
/// objects must also be safe to send and access across threads.
#[doc(hidden)]
pub trait __ComSupportsInterfaceThreading<M: ThreadingModel> {}

impl<T: 'static> __ComSupportsInterfaceThreading<ThreadLocal> for T {}
impl<T: Send + Sync + 'static> __ComSupportsInterfaceThreading<ThreadSafe> for T {}

// SAFETY: `IUnknown` is the canonical interface header and vtable prefix. Its
// pointer conversion is the identity operation, and the constants describe the
// exact built-in root-interface contract.
unsafe impl ComInterface for IUnknown {
    const INTERFACE_ID: NcInterfaceId = IUNKNOWN_INTERFACE_ID;

    type Threading = ThreadLocal;
    type VTable = IUnknownVTable;

    /// Returns the pointer unchanged because this is already `IUnknown`.
    fn iunknown(this: *mut Self) -> *mut IUnknown {
        this
    }
}

impl IUnknown {
    /// Canonical validation form of this interface's typed ABI contract.
    pub const ABI_SIGNATURE: NcAbiSignature = IUNKNOWN_ABI_SIGNATURE;
    /// Stable identity for this interface ABI contract.
    pub const INTERFACE_ID: NcInterfaceId = IUNKNOWN_INTERFACE_ID;
    /// Canonical zero-method descriptor for the root COM interface.
    ///
    /// Every generated object contains exactly one mapping to this interface
    /// contract. `#[com_object]` inserts that mapping automatically; users list
    /// only the additional interfaces their object implements.
    pub const INTERFACE_DESCRIPTOR: NcInterfaceDescriptor = NcInterfaceDescriptor {
        interface_id: IID_IUNKNOWN,
        qualified_name: NcStaticUtf8::from_static(IUNKNOWN_QUALIFIED_NAME),
        abi_signature: IUNKNOWN_ABI_SIGNATURE,
        base: crate::NcInterfaceId::NONE,
        method_count: 0,
        methods: core::ptr::null(),
    };
}

#[cfg(test)]
mod tests {
    use super::{ComInterface, IID_IUNKNOWN, IUNKNOWN_INTERFACE_ID, IUNKNOWN_QUALIFIED_NAME, IUnknown};

    #[test]
    fn iunknown_interface_identity_is_stable() {
        assert_eq!(IID_IUNKNOWN, <IUnknown as ComInterface>::IID);
        assert_eq!(
            IID_IUNKNOWN.to_uuid_bytes(),
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,]
        );
        assert!(!IUnknown::ABI_SIGNATURE.is_empty());
        assert_eq!(IUNKNOWN_INTERFACE_ID, <IUnknown as ComInterface>::INTERFACE_ID);
        assert_eq!(IUNKNOWN_INTERFACE_ID, IUnknown::INTERFACE_ID);
        assert_eq!(IUnknown::INTERFACE_DESCRIPTOR.interface_id, IID_IUNKNOWN);
        assert_eq!(unsafe { IUnknown::INTERFACE_DESCRIPTOR.qualified_name.as_str() }, Ok(IUNKNOWN_QUALIFIED_NAME));
        assert_eq!(IUnknown::INTERFACE_DESCRIPTOR.abi_signature, IUnknown::ABI_SIGNATURE);
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
