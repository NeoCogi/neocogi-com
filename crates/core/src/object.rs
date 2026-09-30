//! The root `IUnknown` contract and the marker for C-shaped COM interfaces.
//!
//! This module fixes the canonical `IUnknown` identity, raw handle pair, vtable prefix,
//! and empty method descriptor. Generated and built-in interfaces implement
//! [`ComInterface`] to describe their identity, vtable type, and shared lifecycle prefix, independent of object layout.

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

/// SHA-256 of `neocogi-com:core-abi:v13\0` followed by the preceding v12 digest.
/// Precomputed signature for the built-in [`IUnknown`] contract.
///
/// Built-ins have no IDL build script, so this checked-in constant plays the
/// same role as the byte string emitted by `neocogi` for ordinary contracts.
static IUNKNOWN_SIGNATURE_BYTES: [u8; NcAbiSignature::SHA256_LEN] = [181, 50, 219, 153, 62, 84, 255, 28, 31, 250, 148, 143, 113, 131, 100, 26, 106, 80, 251, 161, 61, 196, 32, 29, 213, 33, 142, 40, 248, 57, 135, 207,
];

/// Canonical ABI validation contract for [`IUnknown`].
pub const IUNKNOWN_ABI_SIGNATURE: NcAbiSignature = NcAbiSignature::from_static(&IUNKNOWN_SIGNATURE_BYTES);

/// Interface identity for [`IUnknown`].
pub const IUNKNOWN_INTERFACE_ID: NcInterfaceId = NcInterfaceId::new(IID_IUNKNOWN, IUNKNOWN_ABI_SIGNATURE);

/// The root contract marker implemented by every COM object.
/// No instance of this marker is stored in the allocation; ComRaw carries its table.
pub enum IUnknown {}

/// C-compatible vtable for [`IUnknown`].
#[repr(C)]
pub struct IUnknownVTable {
    /// Identity of the actual interface selected by this vtable, including its
    /// complete local contract signature. This points to immutable static metadata;
    /// it describes the projected interface, which may differ from IUnknown.
    pub interface_id: *const NcInterfaceId,

    /// Queries the object for another interface by identity.
    ///
    /// # Safety
    ///
    /// `this` must be the live allocation address paired with this table.
    /// `interface_id` must point to the expected interface GUID/signature pair, and
    /// `out` must point to a distinct slot valid for one ComRaw record write. Given
    /// those preconditions, the implementation initializes both fields of `*out` to null.
    /// `NC_OK` replaces it with one present owned interface pair with the same object address that the
    /// caller must eventually release; every other returned status leaves it
    /// null and transfers no reference. Null or otherwise invalid arguments
    /// violate this safety contract even when an implementation rejects them
    /// defensively.
    pub query_interface: unsafe extern "C" fn(this: *mut c_void, interface_id: *const NcInterfaceId, out: *mut crate::ComRaw<IUnknown>) -> NcStatus,

    /// Retains shared ownership and returns the new count. Exclusive objects
    /// return zero without acquiring ownership; callers must check rejection.
    ///
    /// # Safety
    ///
    /// `this` must be the live allocation address paired with this table. The caller only needs a
    /// borrowed reference for the duration of the call; success creates one new
    /// owned reference only when the returned count is nonzero.
    pub add_ref: unsafe extern "C" fn(this: *mut c_void) -> u32,

    /// Decrements the object's reference count and returns the remaining count.
    ///
    /// # Safety
    ///
    /// `this` must own one live reference. The call consumes that reference and
    /// may destroy the object immediately when the remaining count reaches zero.
    pub release: unsafe extern "C" fn(this: *mut c_void) -> u32,

    /// Optional weak-lifetime capability. None reports unsupported weak references.
    /// A live borrowed `this` and a distinct writable output are required. Success
    /// transfers one canonical weak token; failures clear output and transfer none.
    /// All projections of an object must return the same control identity. A successful token uses the common header
    /// at the canonical object address, retained through final weak release.
    pub downgrade: Option<unsafe extern "C" fn(this: *mut c_void, out: *mut crate::NcWeakRaw) -> NcStatus>,
}

// SAFETY: Vtables contain only function pointers and
// immutable static identity metadata. Sharing the table does not grant cross-thread access to an object.
unsafe impl Sync for IUnknownVTable {}

/// Metadata and raw-pair contract implemented by every COM ABI interface type.
///
/// Prefer defining interfaces with `#[com_interface]` from the `neocogi-com` crate.
/// The macro generates this implementation together with the interface marker,
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
/// - Self is a marker for an exact interface contract. Its VTable has the
///   common IUnknownVTable prefix and methods accepting the allocation address.
/// - Every published ComRaw<Self> pairs that table with its matching live object.
///   Tables and identities remain immutable and live while any reference exists.
/// - Exact identity checks precede typed argument/result dispatch. Borrowed
///   inherited method views may use a physically compatible base table prefix.
/// - `Self::Threading` is the minimum implementation guarantee. Shared
///   contracts require thread-safe calls, ownership, and destruction; a
///   pointer may retain a stronger guarantee or explicitly weaken it.
/// - QueryInterface preserves the object address, selects the exact requested
///   table, and transfers exactly one reference on success. Failure clears both
///   output fields. AddRef and Release share the allocation's single count.
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
}

/// An interface eligible to serve as a plugin's zero-argument root.
/// Typed methods on this interface construct every parameterized object.
///
/// # Safety
///
/// Implementors must be generated from an IDL `factory` declaration. The
/// marker certifies construction policy without changing the interface ABI.
pub unsafe trait ComFactoryInterface: ComInterface {}

/// A concrete object contract whose required projections belong to one COM identity.
///
/// # Safety
/// Implementations must be generated from an explicit IDL `class` declaration.
/// The marker certifies the class role without altering its callable ABI.
pub unsafe trait ComClassInterface: ComInterface {}

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

// SAFETY: IUnknown names the canonical common vtable contract. Its marker stores
// no object data; the constants describe the exact built-in root interface.
unsafe impl ComInterface for IUnknown {
    const INTERFACE_ID: NcInterfaceId = IUNKNOWN_INTERFACE_ID;

    type Threading = ThreadLocal;
    type VTable = IUnknownVTable;
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

    /// Erasure preserves both words without dereferencing a foreign allocation.
    #[test]
    fn unknown_raw_pair_is_two_words_and_absence_is_canonical() {
        assert_eq!(core::mem::size_of::<crate::ComRaw<IUnknown>>(), 2 * core::mem::size_of::<usize>());
        assert!(crate::ComRaw::<IUnknown>::null().erase().is_null());
    }
}

/// Finds a canonical table by the identity already stored in its common prefix.
///
/// This shared lookup has no implementation-type parameter and reads no inspection
/// descriptors. The caller owns reference transfer, so failed lookup cannot leak a
/// reference and successful lookup alone does not extend the allocation's lifetime.
///
/// # Safety
/// Every table must have a permanently valid IUnknownVTable prefix with a readable
/// generated identity and remain immutable while this function runs. The caller
/// must pair the returned table with an allocation of its matching implementation.
#[doc(hidden)]
pub unsafe fn __nc_find_interface(vtables: &[*const IUnknownVTable], requested: &NcInterfaceId) -> Result<*const IUnknownVTable, crate::NcStatus> {
    for &vtable in vtables {
        // SAFETY: generated compact tables retain only valid permanent vtables.
        let actual = unsafe { &*(*vtable).interface_id };
        if actual.iid == requested.iid {
            return match actual.abi_signature == requested.abi_signature {
                true => Ok(vtable),
                false => Err(crate::NC_E_ABI_VERSION),
            };
        }
    }
    Err(crate::NC_E_NOINTERFACE)
}
