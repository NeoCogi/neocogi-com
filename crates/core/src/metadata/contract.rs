//! Canonical logical types used to describe COM ABI contracts.
//!
//! Rust enums describe logical value types, passing modes and threading promises.
//! Physical metadata records use fixed-width integer tags whose accepted encodings
//! are named by the `NC_ABI_*` constants. ABI signature generation belongs to the
//! IDL resolver and is intentionally absent from this runtime crate.
//!
//! # Why physical tags use integers
//!
//! An initialized `u32` can hold an unknown or reserved code until a reader rejects
//! it. A Rust enum may hold only its declared discriminants, even with `repr(u32)`;
//! reading an invalid enum is undefined behavior before an ordinary match can
//! reject it. Readers therefore check raw integers before interpreting them as
//! logical variants, using checked conversion rather than transmutation.
//!
//! Flat constants are the current naming API, not a requirement of C interoperability.
//! An enum could organize the known encodings locally with checked conversion;
//! the raw record would still need integer storage that permits validation.

use crate::guid::NcGuid;
use crate::signature::NcAbiSignature;

/// Current experimental draft core ABI version.
///
/// This identifies the current binary generation; it does not mean core ABI v17 has
/// reached its ratification milestone.
pub const NC_COM_CORE_ABI_VERSION: u32 = 17;

/// Stable identity for one COM interface ABI contract.
///
/// This pairs the interface GUID with the generated ABI signature that describes the
/// exact lowered vtable contract.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NcInterfaceId {
    /// Stable interface GUID.
    pub iid: NcGuid,
    /// Canonical interface ABI validation contract.
    pub abi_signature: NcAbiSignature,
}

impl NcInterfaceId {
    /// Sentinel used when no interface is present.
    pub const NONE: Self = Self {
        iid: NcGuid::nil(),
        abi_signature: NcAbiSignature::EMPTY,
    };

    /// Creates an interface identity from its GUID and ABI signature.
    pub const fn new(iid: NcGuid, abi_signature: NcAbiSignature) -> Self {
        Self { iid, abi_signature }
    }

    /// Returns whether this is the no-interface sentinel.
    pub const fn is_none(self) -> bool {
        self.iid.is_nil() && self.abi_signature.is_empty()
    }
}

/// Logical Rust type used to construct generated method and field metadata.
///
/// [`NcAbiTypeDescriptor::from_abi_type`] lowers this enum to a physical record.
/// This enum's native representation is not the shared-library metadata layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcAbiType {
    /// No value is present.
    Void,
    /// C-compatible boolean value.
    Bool,
    /// Unsigned 8-bit integer.
    U8,
    /// Unsigned 16-bit integer.
    U16,
    /// Unsigned 32-bit integer.
    U32,
    /// Unsigned 64-bit integer.
    U64,
    /// Pointer-width unsigned integer.
    USize,
    /// Signed 8-bit integer.
    I8,
    /// Signed 16-bit integer.
    I16,
    /// Signed 32-bit integer.
    I32,
    /// Signed 64-bit integer.
    I64,
    /// Pointer-width signed integer.
    ISize,
    /// IEEE-754 binary32 floating-point value.
    F32,
    /// IEEE-754 binary64 floating-point value.
    F64,
    /// [`NcGuid`] value.
    NcGuid,
    /// [`crate::NcStatus`] value.
    NcStatus,
    /// Declaration-scoped ABI-safe aggregate identity.
    Aggregate(NcAbiSignature),
    /// COM interface reference or pointer type.
    Interface {
        /// Referenced interface GUID.
        iid: NcGuid,
    },
}

/// Logical Rust description of how an argument or result crosses the ABI boundary.
///
/// [`NcAbiPassingDescriptor`] stores the corresponding raw integer encodings;
/// this enum's native representation does not cross the shared-library boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcAbiPassing {
    /// Shared weak header returned through an out slot; promotion is separate.
    WeakComPtrOut {
        /// Threading capability retained after the referenced value expires.
        mode: NcAbiThreadingMode,
    },
    /// Exclusive owner offered in a writable slot, cleared by the receiver on transfer.
    ComBoxIn {
        /// Threading capability preserved by ownership transfer.
        mode: NcAbiThreadingMode,
    },
    /// The single exclusive owner returned through an out slot.
    ComBoxOut {
        /// Threading capability preserved by ownership transfer.
        mode: NcAbiThreadingMode,
    },
    /// Cached weak token returned through an out slot; owns no initialized value.
    WeakComBoxOut {
        /// Threading capability preserved by the observer.
        mode: NcAbiThreadingMode,
    },
    /// Null or a borrowed COM interface valid for the current call.
    OptionalComRef {
        /// Threading capability of a present borrowed pointer.
        mode: NcAbiThreadingMode,
    },
    /// Pointer to one recursively lowered raw value record, borrowed for this call.
    BorrowedValue,
    /// No success value is carried.
    Void,
    /// Copied value whose checked raw representation appears in the ABI signature.
    Value,
    /// Copied success value written as checked raw storage through an out pointer.
    Out,
    /// Caller buffer copied into the callee/object at an explicit offset.
    CopyFromBufferOffset,
    /// Built-in exact-range byte copy with offset/length/pointer; not generic output.
    CopyToBufferOffset,
    /// Fresh copied output with offset, capacity, wire pointer and initialized-prefix slot.
    OutputBuffer,
    /// Borrowed COM interface pointer valid for the current call.
    ComRef {
        /// Threading capability carried by the borrowed pointer.
        mode: NcAbiThreadingMode,
    },
    /// Owned COM interface pointer returned through an out slot.
    ComPtrOut {
        /// Threading capability carried by the owned pointer.
        mode: NcAbiThreadingMode,
    },
}

/// COM pointer and IDL object/factory threading contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcAbiThreadingMode {
    /// Carries no permission for calls or final release from another thread.
    ///
    /// For object metadata this is the strongest capability the producer promises.
    /// For a pointer it may also be a conservative view of a thread-safe object.
    ThreadLocal,
    /// Positively guarantees that calls and final release may occur from any
    /// thread, including concurrent calls through aliases.
    ThreadSafe,
}

/// Borrowing permission required by one generated COM method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NcAbiReceiver {
    /// The method receives `&self` and may overlap other shared calls.
    Shared,
    /// The method receives `&mut self` and requires exclusive object access.
    Mutable,
}

impl NcAbiReceiver {
    /// Returns the C-stable `NC_ABI_RECEIVER_*` value.
    pub const fn as_raw(self) -> u32 {
        match self {
            Self::Shared => NC_ABI_RECEIVER_SHARED,
            Self::Mutable => NC_ABI_RECEIVER_MUTABLE,
        }
    }

    /// Decodes one C-stable `NC_ABI_RECEIVER_*` value.
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            NC_ABI_RECEIVER_SHARED => Some(Self::Shared),
            NC_ABI_RECEIVER_MUTABLE => Some(Self::Mutable),
            _ => None,
        }
    }
}

impl NcAbiThreadingMode {
    /// Returns the C-stable `NC_ABI_THREADING_*` value.
    pub const fn as_raw(self) -> u32 {
        match self {
            Self::ThreadLocal => NC_ABI_THREADING_THREAD_LOCAL,
            Self::ThreadSafe => NC_ABI_THREADING_THREAD_SAFE,
        }
    }

    /// Decodes one C-stable `NC_ABI_THREADING_*` value.
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            NC_ABI_THREADING_THREAD_LOCAL => Some(Self::ThreadLocal),
            NC_ABI_THREADING_THREAD_SAFE => Some(Self::ThreadSafe),
            _ => None,
        }
    }
}

// These constants name raw descriptor encodings, separately from NcAbiType's
// logical variants and payloads. Integer storage permits rejection of reserved
// or unknown tags; see NcAbiTypeDescriptor's representation rationale.

/// C-stable ABI type tag for no value (void).
pub const NC_ABI_TYPE_VOID: u32 = 0x00;
/// C-stable ABI type tag for `bool`.
pub const NC_ABI_TYPE_BOOL: u32 = 0x01;
/// C-stable ABI type tag for `u8`.
pub const NC_ABI_TYPE_U8: u32 = 0x02;
/// C-stable ABI type tag for `u16`.
pub const NC_ABI_TYPE_U16: u32 = 0x03;
/// C-stable ABI type tag for `u32`.
pub const NC_ABI_TYPE_U32: u32 = 0x04;
/// C-stable ABI type tag for `u64`.
pub const NC_ABI_TYPE_U64: u32 = 0x05;
/// C-stable ABI type tag for `usize`.
pub const NC_ABI_TYPE_USIZE: u32 = 0x06;
/// C-stable ABI type tag for `i8`.
pub const NC_ABI_TYPE_I8: u32 = 0x07;
/// C-stable ABI type tag for `i16`.
pub const NC_ABI_TYPE_I16: u32 = 0x08;
/// C-stable ABI type tag for `i32`.
pub const NC_ABI_TYPE_I32: u32 = 0x09;
/// C-stable ABI type tag for `i64`.
pub const NC_ABI_TYPE_I64: u32 = 0x0a;
/// C-stable ABI type tag for `isize`.
pub const NC_ABI_TYPE_ISIZE: u32 = 0x0b;
/// C-stable ABI type tag for `f32`.
pub const NC_ABI_TYPE_F32: u32 = 0x0c;
/// C-stable ABI type tag for `f64`.
pub const NC_ABI_TYPE_F64: u32 = 0x0d;
/// C-stable ABI type tag for [`NcGuid`].
pub const NC_ABI_TYPE_NC_GUID: u32 = 0x0e;
/// C-stable ABI type tag for [`crate::NcStatus`].
pub const NC_ABI_TYPE_NC_STATUS: u32 = 0x0f;
/// Value `0x10` is reserved because ABI signatures are views, not plain values.
/// C-stable ABI type tag for a structural aggregate signature.
pub const NC_ABI_TYPE_AGGREGATE: u32 = 0x11;
/// C-stable ABI type tag for an interface GUID/signature pair.
pub const NC_ABI_TYPE_INTERFACE: u32 = 0x12;

/// C-stable passing tag for no success value.
pub const NC_ABI_PASSING_VOID: u32 = 0x00;
/// C-stable passing tag for a direct plain value.
pub const NC_ABI_PASSING_VALUE: u32 = 0x01;
/// C-stable passing tag for a plain success-value out pointer.
pub const NC_ABI_PASSING_OUT: u32 = 0x02;
/// C-stable passing tag for readable raw elements at a logical operation offset.
pub const NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET: u32 = 0x03;
/// C-stable passing tag for writable raw elements at a logical operation offset.
pub const NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET: u32 = 0x04;
/// C-stable passing tag for a call-scoped borrowed COM pointer.
pub const NC_ABI_PASSING_COM_REF: u32 = 0x05;
/// C-stable passing tag for an owned COM pointer success out slot.
pub const NC_ABI_PASSING_COM_PTR_OUT: u32 = 0x06;
/// Call-scoped pointer to a recursively lowered raw value representation.
pub const NC_ABI_PASSING_BORROWED_VALUE: u32 = 0x07;
/// Nullable borrowed interface input; distinct from a required capability.
pub const NC_ABI_PASSING_OPTIONAL_COM_REF: u32 = 0x08;
/// Exclusive owner input transferred through a cleared writable slot.
pub const NC_ABI_PASSING_COM_BOX_IN: u32 = 0x09;
/// Exclusive owner output transferred through an out slot.
pub const NC_ABI_PASSING_COM_BOX_OUT: u32 = 0x0a;
/// Cached weak exclusive token returned without value ownership.
pub const NC_ABI_PASSING_WEAK_COM_BOX_OUT: u32 = 0x0b;
/// One shared weak header token returned through an NcWeakRaw output slot.
pub const NC_ABI_PASSING_WEAK_COM_PTR_OUT: u32 = 0x0d;
/// One-way copied result capacity with a separate initialized-prefix count.
pub const NC_ABI_PASSING_OUTPUT_BUFFER: u32 = 0x0c;

/// C-stable threading value requiring calls and final release on one thread.
pub const NC_ABI_THREADING_THREAD_LOCAL: u32 = 0x00;
/// C-stable threading value permitting cross-thread calls and final release.
pub const NC_ABI_THREADING_THREAD_SAFE: u32 = 0x01;

/// C-stable method receiver value for shared `&self` dispatch.
pub const NC_ABI_RECEIVER_SHARED: u32 = 0x00;
/// C-stable method receiver value for exclusive `&mut self` dispatch.
pub const NC_ABI_RECEIVER_MUTABLE: u32 = 0x01;

/// C-stable typed ABI descriptor for one method argument or return type.
///
/// The raw tag remains an integer so reserved or unknown codes can be inspected
/// and rejected before semantic use. Replacing it with a Rust enum would assume
/// a valid discriminant when reading the record; `repr(u32)` fixes representation
/// but does not permit undeclared variants. [`NcAbiType`] is the local logical
/// description used when constructing known descriptors.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcAbiTypeDescriptor {
    /// Raw type encoding; valid producers use one of the `NC_ABI_TYPE_*` values.
    /// Readers must reject unsupported codes before interpreting the type.
    pub tag: u32,
    /// Declaration-scoped aggregate signature when `tag == NC_ABI_TYPE_AGGREGATE`.
    ///
    /// The IDL tool derives this from the exact module revision and declaration
    /// ID. Application-memory offsets are not published. The generated concrete
    /// wire type and checked field codecs establish the actual foreign layout.
    pub aggregate_signature: NcAbiSignature,
    /// Interface identity when `tag == NC_ABI_TYPE_INTERFACE`.
    pub interface: NcInterfaceId,
}

impl NcAbiTypeDescriptor {
    /// Describes an interface using its complete expected identity.
    ///
    /// The GUID identifies the declaration, while its generator-assigned
    /// signature pins the exact transitive module revision. Keeping both values
    /// explicit also permits recursive interface contracts.
    pub const fn from_interface(interface: NcInterfaceId) -> Self {
        let mut out = Self::from_abi_type(NcAbiType::Interface { iid: interface.iid });
        out.interface = interface;
        out
    }

    /// Builds a descriptor for a local typed-metadata value. An Interface input
    /// contains only an IID and therefore leaves its signature empty; publish
    /// complete interface metadata with [`Self::from_interface`] instead.
    /// Validation rejects incomplete interface descriptors before plugin
    /// registration.
    pub const fn from_abi_type(ty: NcAbiType) -> Self {
        let mut out = Self {
            tag: NC_ABI_TYPE_VOID,
            aggregate_signature: NcAbiSignature::EMPTY,
            interface: NcInterfaceId::NONE,
        };
        match ty {
            NcAbiType::Void => out.tag = NC_ABI_TYPE_VOID,
            NcAbiType::Bool => out.tag = NC_ABI_TYPE_BOOL,
            NcAbiType::U8 => out.tag = NC_ABI_TYPE_U8,
            NcAbiType::U16 => out.tag = NC_ABI_TYPE_U16,
            NcAbiType::U32 => out.tag = NC_ABI_TYPE_U32,
            NcAbiType::U64 => out.tag = NC_ABI_TYPE_U64,
            NcAbiType::USize => out.tag = NC_ABI_TYPE_USIZE,
            NcAbiType::I8 => out.tag = NC_ABI_TYPE_I8,
            NcAbiType::I16 => out.tag = NC_ABI_TYPE_I16,
            NcAbiType::I32 => out.tag = NC_ABI_TYPE_I32,
            NcAbiType::I64 => out.tag = NC_ABI_TYPE_I64,
            NcAbiType::ISize => out.tag = NC_ABI_TYPE_ISIZE,
            NcAbiType::F32 => out.tag = NC_ABI_TYPE_F32,
            NcAbiType::F64 => out.tag = NC_ABI_TYPE_F64,
            NcAbiType::NcGuid => out.tag = NC_ABI_TYPE_NC_GUID,
            NcAbiType::NcStatus => out.tag = NC_ABI_TYPE_NC_STATUS,
            NcAbiType::Aggregate(signature) => {
                out.tag = NC_ABI_TYPE_AGGREGATE;
                out.aggregate_signature = signature;
            }
            NcAbiType::Interface { iid } => {
                out.tag = NC_ABI_TYPE_INTERFACE;
                out.interface = NcInterfaceId::new(iid, NcAbiSignature::EMPTY);
            }
        }
        out
    }
}

/// C-stable typed ABI descriptor for one method argument or return passing mode.
///
/// Integer fields permit checked admission of incoming tags and threading codes,
/// for the same reason as [`NcAbiTypeDescriptor`]. [`NcAbiPassing`] describes
/// logical passing modes locally; [`NcAbiThreadingMode::from_raw`] checks the
/// raw threading encoding before constructing a Rust enum value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcAbiPassingDescriptor {
    /// Raw passing encoding; readers must validate against `NC_ABI_PASSING_*`.
    pub tag: u32,
    /// Raw `NC_ABI_THREADING_*` encoding for COM pointer modes, checked on admission.
    pub threading: u32,
}

impl NcAbiPassingDescriptor {
    /// Builds a C-stable descriptor from a canonical ABI passing mode.
    pub const fn from_abi_passing(passing: NcAbiPassing) -> Self {
        match passing {
            NcAbiPassing::WeakComPtrOut { mode } => Self {
                tag: NC_ABI_PASSING_WEAK_COM_PTR_OUT,
                threading: threading_descriptor(mode),
            },
            NcAbiPassing::ComBoxIn { mode } => Self {
                tag: NC_ABI_PASSING_COM_BOX_IN,
                threading: threading_descriptor(mode),
            },
            NcAbiPassing::ComBoxOut { mode } => Self {
                tag: NC_ABI_PASSING_COM_BOX_OUT,
                threading: threading_descriptor(mode),
            },
            NcAbiPassing::WeakComBoxOut { mode } => Self {
                tag: NC_ABI_PASSING_WEAK_COM_BOX_OUT,
                threading: threading_descriptor(mode),
            },
            NcAbiPassing::OptionalComRef { mode } => Self {
                tag: NC_ABI_PASSING_OPTIONAL_COM_REF,
                threading: threading_descriptor(mode),
            },
            NcAbiPassing::BorrowedValue => Self {
                tag: NC_ABI_PASSING_BORROWED_VALUE,
                threading: NC_ABI_THREADING_THREAD_LOCAL,
            },
            NcAbiPassing::Void => Self {
                tag: NC_ABI_PASSING_VOID,
                threading: NC_ABI_THREADING_THREAD_LOCAL,
            },
            NcAbiPassing::Value => Self {
                tag: NC_ABI_PASSING_VALUE,
                threading: NC_ABI_THREADING_THREAD_LOCAL,
            },
            NcAbiPassing::Out => Self {
                tag: NC_ABI_PASSING_OUT,
                threading: NC_ABI_THREADING_THREAD_LOCAL,
            },
            NcAbiPassing::CopyFromBufferOffset => Self {
                tag: NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET,
                threading: NC_ABI_THREADING_THREAD_LOCAL,
            },
            NcAbiPassing::CopyToBufferOffset => Self {
                tag: NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET,
                threading: NC_ABI_THREADING_THREAD_LOCAL,
            },
            NcAbiPassing::OutputBuffer => Self {
                tag: NC_ABI_PASSING_OUTPUT_BUFFER,
                threading: NC_ABI_THREADING_THREAD_LOCAL,
            },
            NcAbiPassing::ComRef { mode } => Self {
                tag: NC_ABI_PASSING_COM_REF,
                threading: threading_descriptor(mode),
            },
            NcAbiPassing::ComPtrOut { mode } => Self {
                tag: NC_ABI_PASSING_COM_PTR_OUT,
                threading: threading_descriptor(mode),
            },
        }
    }
}

/// Converts the logical threading enum into the stable raw descriptor tag.
const fn threading_descriptor(mode: NcAbiThreadingMode) -> u32 {
    mode.as_raw()
}
