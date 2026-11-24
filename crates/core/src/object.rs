//! The root `IUnknown` contract and the marker for C-shaped COM interfaces.
//!
//! This module fixes the canonical `IUnknown` identity, raw handle pair, vtable prefix,
//! and empty method descriptor. Generated and built-in interfaces implement
//! [`ComInterface`] to describe their identity, vtable type, and common lifecycle prefix, independent of object layout.

use core::ffi::c_void;

use crate::guid::NcGuid;
use crate::metadata::{NcInterfaceDescriptor, NcInterfaceId, NcStaticUtf8};
use crate::abi_digest::NcAbiDigest;
use crate::status::NcStatus;
use crate::{ThreadLocal, ThreadSafe, ThreadingModel};

/// Interface identifier for [`IUnknown`].
pub const IID_IUNKNOWN: NcGuid = NcGuid::from_fields(0x0000_0000, 0x0000, 0x0000, [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46]);

/// Reserved normative qualified name for [`IUnknown`].
pub const IUNKNOWN_QUALIFIED_NAME: &str = "IUnknown";

/// Fixed intrinsic ABI signature for the built-in [`IUnknown`] contract.
///
/// Built-ins have no IDL build script, so this checked-in constant plays the
/// same role as the byte string emitted by `neocogi` for ordinary contracts.
static IUNKNOWN_SIGNATURE_BYTES: [u8; NcAbiDigest::SHA256_LEN] = [
    181, 50, 219, 153, 62, 84, 255, 28, 31, 250, 148, 143, 113, 131, 100, 26, 106, 80, 251, 161, 61, 196, 32, 29, 213, 33, 142, 40, 248, 57, 135, 207,
];

/// Canonical ABI validation contract for [`IUnknown`].
pub const IUNKNOWN_ABI_SIGNATURE: NcAbiDigest = NcAbiDigest::from_static(&IUNKNOWN_SIGNATURE_BYTES);

/// Interface identity for [`IUnknown`].
pub const IUNKNOWN_INTERFACE_ID: NcInterfaceId = NcInterfaceId::new(IID_IUNKNOWN, IUNKNOWN_ABI_SIGNATURE);

/// The root contract marker implemented by every COM object.
/// No instance of this marker is stored in the allocation; ComRaw carries its table.
pub enum IUnknown {}

/// C-compatible vtable for [`IUnknown`].
///
/// Construction and generated interface tables select the ownership policy.
/// Begin each callback with the matching live storage and table prefix, and
/// keep the table, metadata, and provider code valid through the call. Owner
/// release can end the storage lifetime before returning. Observe the
/// implementation's threading and reentry rules. Callbacks must return across
/// the C boundary without unwinding.
#[repr(C)]
pub struct IUnknownVTable {
    /// Identity of the actual interface selected by this vtable, including its
    /// complete local contract signature. This points to immutable static metadata;
    /// it describes the projected interface, which may differ from IUnknown.
    pub interface_id: *const NcInterfaceId,

    /// Queries a shared object for an owned interface of the requested identity.
    ///
    /// Success transfers one shared owner with the same allocation address.
    /// Exclusive and scoped prefixes reject owning queries. Every failed query
    /// clears both output fields and transfers no owner.
    ///
    /// # Safety
    ///
    /// `this` must be the address paired with this table, backed by a live shared
    /// owner for a shared query or valid access to the matching exclusive or
    /// scoped storage for a rejecting prefix. `interface_id` must point to a
    /// readable GUID and signature for the call. `out` must be a distinct
    /// writable slot for one [`crate::ComRaw`] with no input alias.
    pub query_interface: unsafe extern "C" fn(this: *mut c_void, interface_id: *const NcInterfaceId, out: *mut crate::ComRaw<IUnknown>) -> NcStatus,

    /// Acquires one shared owner and returns the new count.
    ///
    /// The reported count is capped at `u32::MAX` when the private count is
    /// wider. Exclusive and scoped prefixes return zero and transfer nothing.
    ///
    /// # Safety
    ///
    /// Use the matched pair backed by a live shared owner for a shared retain.
    /// A rejecting prefix requires valid access to its exclusive or scoped
    /// storage for the call. The caller owns a new token only on nonzero return.
    pub add_ref: unsafe extern "C" fn(this: *mut c_void) -> u32,

    /// Consumes one shared owner or the sole exclusive owner.
    ///
    /// Shared release returns the remaining count, capped at `u32::MAX` when
    /// the private count is wider. Final shared release destroys the value.
    /// Exclusive release closes admission, waits for admitted methods, destroys
    /// the value synchronously in the provider, and returns zero. Boxed weak
    /// observers can keep the allocation storage alive after value destruction.
    ///
    /// Rust [`crate::ComBox`] destruction calls the typed boxed lifetime operation.
    /// A raw exclusive owner calls the matching prefix callback. A scoped
    /// release stub is a defensive no-op for a pair that grants no owner.
    ///
    /// # Safety
    ///
    /// Transfer one valid owner of the policy selected by the table exactly once.
    /// A weak token or borrowed [`crate::ComRef`] supplies no owner for this call.
    /// Before releasing an exclusive owner, end borrows on the calling thread
    /// and release locks or resources needed by admitted calls to finish.
    pub release: unsafe extern "C" fn(this: *mut c_void) -> u32,

    /// The optional callback acquires shared weak ownership.
    ///
    /// When the callback is absent, [`crate::ComPtr::downgrade`] reports
    /// `NC_E_NOINTERFACE`. Boxed prefixes supply a callback that rejects
    /// shared downgrade with `NC_E_INVALID_ARG` on valid arguments.
    /// Success transfers one shared [`crate::NcSharedWeakRaw`]
    /// at the canonical allocation address for every interface of the object.
    /// The weak token keeps the shared header alive through final weak release.
    /// Every callback failure clears the output and transfers no weak token.
    ///
    /// # Safety
    ///
    /// `this` must be backed by a live shared owner for a shared downgrade,
    /// or valid matching storage for a rejecting prefix. `out` must be a
    /// distinct writable weak-token slot with no input alias.
    pub downgrade: Option<unsafe extern "C" fn(this: *mut c_void, out: *mut crate::NcSharedWeakRaw) -> NcStatus>,
}

// SAFETY: Vtables contain only function pointers and
// immutable static identity metadata. Sharing the table does not grant cross-thread access to an object.
unsafe impl Sync for IUnknownVTable {}

/// Metadata and raw-pair contract implemented by every COM ABI interface type.
///
/// Define callable contracts in `.ncidl`; the compiler emits this implementation.
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
///   [`IUnknownVTable`] prefix and methods accepting the allocation address.
/// - Every present `ComRaw<Self>` pairs an allocation address with the matching
///   table. Storage, immutable table and identity, and provider code stay valid
///   for the access the pair grants. A [`crate::ComRef`] borrows that access from
///   an owner or scope and acquires no token itself. A boxed weak observer keeps
///   storage available for checked method entry, which checks value liveness.
/// - Exact identity checks precede typed argument/result dispatch. Borrowed
///   inherited method views may use a physically compatible base table prefix.
/// - `Self::Threading` is the minimum implementation guarantee. Shared
///   contracts require thread-safe calls, ownership, and destruction. A
///   pointer may retain a stronger guarantee or explicitly weaken it.
/// - The common callbacks implement the construction-selected ownership policy.
///   Shared AddRef and Release account for a single set of strong owners,
///   including built-in headerless data such as bytes, strings and child lists.
///   Boxed Release consumes the sole exclusive owner. A scoped pair only lends
///   call access, with no ownership transfer through its prefix.
///
/// Violating these invariants can make safe operations such as
/// [`crate::ComPtr::query`], `Clone`, or `Drop` cause undefined behavior.
pub unsafe trait ComInterface {
    /// Stable interface identity used by `QueryInterface`.
    const INTERFACE_ID: NcInterfaceId;

    /// Stable interface identifier used by `QueryInterface`.
    const IID: NcGuid = Self::INTERFACE_ID.iid;

    /// Canonical validation form of this interface's typed ABI contract.
    const ABI_SIGNATURE: NcAbiDigest = Self::INTERFACE_ID.abi_signature;

    /// Minimum threading capability required of an implementation.
    type Threading: ThreadingModel;

    /// The C-compatible vtable type for this interface.
    type VTable;
}

/// Compile-time proof that an implementation satisfies the selected class mode.
///
/// This hidden helper lets generated object storage derive threading from the
/// primary IDL interface or explicit object policy. Local class modes need only
/// own `'static` state, while shared class modes must also be safe to send and
/// access across threads. Separately, every generated shared provider trait
/// requires Send + Sync even when the selected class mode is local.
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
    pub const ABI_SIGNATURE: NcAbiDigest = IUNKNOWN_ABI_SIGNATURE;
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

    /// Casting preserves both words without dereferencing a foreign allocation.
    #[test]
    fn unknown_raw_pair_is_two_words_and_absence_is_canonical() {
        assert_eq!(core::mem::size_of::<crate::ComRaw<IUnknown>>(), 2 * core::mem::size_of::<usize>());
        assert!(crate::ComRaw::<IUnknown>::null().cast::<crate::IUnknown>().is_null());
    }
}
