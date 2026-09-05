//! Built-in COM interfaces for immutable byte sequences and UTF-8 strings.
//!
//! The module defines their stable identities, C-shaped interface headers and
//! vtables, and the complete typed method metadata used to calculate and
//! validate ABI hashes. Concrete allocation-backed Rust implementations live in
//! the higher-level `nc-com` crate; this core module contains only ABI contracts.

use core::ffi::c_void;

use crate::guid::{NcGuid, guid_from_qualified_name};
use crate::hash::NcAbiHash;
use crate::metadata::{
    NcAbiHashPart, NcAbiPassing, NcAbiPassingDescriptor, NcAbiType, NcAbiTypeDescriptor,
    NcInterfaceDescriptor, NcInterfaceId, NcMethodDescriptor, NcMethodParamDescriptor,
    NcMethodReturnDescriptor, NcStaticUtf8,
};
use crate::object::{ComInterface, IUnknown, IUnknownVTable};
use crate::status::NcStatus;

/// Normative qualified name for [`INcBytesAbi`].
pub const QUALIFIED_NAME_INC_BYTES: &str = "nc.com.INcBytes:v1";

/// Interface identifier derived from [`QUALIFIED_NAME_INC_BYTES`].
pub const IID_INC_BYTES: NcGuid = guid_from_qualified_name(QUALIFIED_NAME_INC_BYTES);

/// Normative qualified name for [`INcStringAbi`].
pub const QUALIFIED_NAME_INC_STRING: &str = "nc.com.INcString:v1";

/// Interface identifier derived from [`QUALIFIED_NAME_INC_STRING`].
pub const IID_INC_STRING: NcGuid = guid_from_qualified_name(QUALIFIED_NAME_INC_STRING);

/// Read-only byte sequence ABI interface.
///
/// This is the C-shaped ABI representation for Rust-owned `Vec<u8>` or
/// bit-preserving `Vec<i8>` data crossing an `nc-com` boundary. Ownership is
/// still carried by COM reference counting; Rust vector layout and allocator
/// ownership never cross the boundary.
#[repr(C)]
pub struct INcBytesAbi {
    /// Function table for this interface.
    pub vtable: *const INcBytesVTable,
    /// Opaque object pointer.
    pub object: *mut c_void,
}

/// Raw vtable for [`INcBytesAbi`].
#[repr(C)]
pub struct INcBytesVTable {
    /// Standard `IUnknown` prefix.
    pub unknown: IUnknownVTable,
    /// Returns the number of bytes.
    ///
    /// # Safety
    ///
    /// `this` must be a live byte-sequence interface pointer. `out` must be
    /// valid for one `u64` write.
    pub len: unsafe extern "C" fn(this: *mut INcBytesAbi, out: *mut u64) -> NcStatus,
    /// Returns the byte at `index`.
    ///
    /// # Safety
    ///
    /// `this` must be a live byte-sequence interface pointer. `out` must be
    /// valid for one `u8` write.
    pub byte_at: unsafe extern "C" fn(this: *mut INcBytesAbi, index: u64, out: *mut u8) -> NcStatus,
    /// Copies `len` bytes starting at `offset` into `out`.
    ///
    /// # Safety
    ///
    /// `this` must be a live byte-sequence interface pointer. When `len` is
    /// nonzero, `out` must be valid for `len` byte writes and must not overlap
    /// the source byte range exposed by `this`. Implementations must validate
    /// the complete request before writing so failures leave `out` unchanged.
    pub copy_to: unsafe extern "C" fn(
        this: *mut INcBytesAbi,
        offset: u64,
        len: u64,
        out: *mut u8,
    ) -> NcStatus,
}

// SAFETY: `INcBytesAbi` is a `repr(C)` interface header with an
// `INcBytesVTable` whose first field is the canonical `IUnknownVTable`. The
// identity/hash below describe that exact vtable, and `iunknown` only casts the
// shared header prefix without dereferencing it.
unsafe impl ComInterface for INcBytesAbi {
    const INTERFACE_ID: NcInterfaceId = NcInterfaceId::new(IID_INC_BYTES, Self::ABI_HASH);

    type VTable = INcBytesVTable;

    /// Reinterprets the shared interface header as its `IUnknown` prefix.
    fn iunknown(this: *mut Self) -> *mut IUnknown {
        this.cast::<IUnknown>()
    }
}

impl INcBytesAbi {
    /// SHA-256 hash of this interface's typed ABI contract.
    pub const ABI_HASH: NcAbiHash = NcAbiHash::from_abi_parts(&[
        NcAbiHashPart::Interface { iid: IID_INC_BYTES },
        NcAbiHashPart::Method {
            index: 0,
            name: "len",
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Out,
            ty: NcAbiType::U64,
        },
        NcAbiHashPart::Status,
        NcAbiHashPart::Method {
            index: 1,
            name: "byte_at",
        },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::Value,
            ty: NcAbiType::U64,
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Out,
            ty: NcAbiType::U8,
        },
        NcAbiHashPart::Status,
        NcAbiHashPart::Method {
            index: 2,
            name: "copy_to",
        },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::CopyToBufferOffset,
            ty: NcAbiType::U8,
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Void,
            ty: NcAbiType::Void,
        },
        NcAbiHashPart::Status,
    ]);

    #[doc(hidden)]
    pub const LEN_PARAMS: [NcMethodParamDescriptor; 0] = [];

    #[doc(hidden)]
    pub const LEN_RETURN: NcMethodReturnDescriptor = NcMethodReturnDescriptor {
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Out),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::U64),
    };

    #[doc(hidden)]
    pub const BYTE_AT_PARAMS: [NcMethodParamDescriptor; 1] = [NcMethodParamDescriptor {
        name: NcStaticUtf8::from_static("index"),
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Value),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::U64),
    }];

    #[doc(hidden)]
    pub const BYTE_AT_RETURN: NcMethodReturnDescriptor = NcMethodReturnDescriptor {
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Out),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::U8),
    };

    #[doc(hidden)]
    pub const COPY_TO_PARAMS: [NcMethodParamDescriptor; 1] = [NcMethodParamDescriptor {
        name: NcStaticUtf8::from_static("out"),
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::CopyToBufferOffset),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::U8),
    }];

    #[doc(hidden)]
    pub const COPY_TO_RETURN: NcMethodReturnDescriptor = NcMethodReturnDescriptor {
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Void),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::Void),
    };

    /// Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [NcMethodDescriptor; 3] = [
        NcMethodDescriptor {
            name: NcStaticUtf8::from_static("len"),
            param_count: Self::LEN_PARAMS.len() as u32,
            params: Self::LEN_PARAMS.as_ptr(),
            returns: Self::LEN_RETURN,
        },
        NcMethodDescriptor {
            name: NcStaticUtf8::from_static("byte_at"),
            param_count: Self::BYTE_AT_PARAMS.len() as u32,
            params: Self::BYTE_AT_PARAMS.as_ptr(),
            returns: Self::BYTE_AT_RETURN,
        },
        NcMethodDescriptor {
            name: NcStaticUtf8::from_static("copy_to"),
            param_count: Self::COPY_TO_PARAMS.len() as u32,
            params: Self::COPY_TO_PARAMS.as_ptr(),
            returns: Self::COPY_TO_RETURN,
        },
    ];

    /// Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: NcInterfaceDescriptor = NcInterfaceDescriptor {
        interface_id: <Self as ComInterface>::IID,
        qualified_name: NcStaticUtf8::from_static(QUALIFIED_NAME_INC_BYTES),
        abi_hash: Self::ABI_HASH,
        method_count: Self::METHOD_DESCRIPTORS.len() as u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
}

/// Read-only UTF-8 string ABI interface.
///
/// This is the C-shaped ABI representation for Rust-owned `String` data crossing
/// an `nc-com` boundary. The bytes exposed by this interface are guaranteed by
/// the object contract to be valid UTF-8 and can be copied in bounded bulk
/// ranges through [`INcStringVTable::copy_to`].
#[repr(C)]
pub struct INcStringAbi {
    /// Function table for this interface.
    pub vtable: *const INcStringVTable,
    /// Opaque object pointer.
    pub object: *mut c_void,
}

/// Raw vtable for [`INcStringAbi`].
#[repr(C)]
pub struct INcStringVTable {
    /// Standard `IUnknown` prefix.
    pub unknown: IUnknownVTable,
    /// Returns the number of UTF-8 bytes.
    ///
    /// # Safety
    ///
    /// `this` must be a live string interface pointer. `out` must be valid for
    /// one `u64` write.
    pub byte_len: unsafe extern "C" fn(this: *mut INcStringAbi, out: *mut u64) -> NcStatus,
    /// Returns the UTF-8 byte at `index`.
    ///
    /// # Safety
    ///
    /// `this` must be a live string interface pointer. `out` must be valid for
    /// one `u8` write.
    pub byte_at:
        unsafe extern "C" fn(this: *mut INcStringAbi, index: u64, out: *mut u8) -> NcStatus,
    /// Copies `len` UTF-8 bytes starting at `offset` into `out`.
    ///
    /// # Safety
    ///
    /// `this` must be a live string interface pointer. When `len` is nonzero,
    /// `out` must be valid for `len` byte writes and must not overlap the
    /// source byte range exposed by `this`. Implementations must validate the
    /// complete request before writing so failures leave `out` unchanged.
    pub copy_to: unsafe extern "C" fn(
        this: *mut INcStringAbi,
        offset: u64,
        len: u64,
        out: *mut u8,
    ) -> NcStatus,
}

// SAFETY: `INcStringAbi` is a `repr(C)` interface header with an
// `INcStringVTable` whose first field is the canonical `IUnknownVTable`. The
// identity/hash below describe that exact vtable, and `iunknown` only casts the
// shared header prefix without dereferencing it.
unsafe impl ComInterface for INcStringAbi {
    const INTERFACE_ID: NcInterfaceId = NcInterfaceId::new(IID_INC_STRING, Self::ABI_HASH);

    type VTable = INcStringVTable;

    /// Reinterprets the shared interface header as its `IUnknown` prefix.
    fn iunknown(this: *mut Self) -> *mut IUnknown {
        this.cast::<IUnknown>()
    }
}

impl INcStringAbi {
    /// SHA-256 hash of this interface's typed ABI contract.
    pub const ABI_HASH: NcAbiHash = NcAbiHash::from_abi_parts(&[
        NcAbiHashPart::Interface {
            iid: IID_INC_STRING,
        },
        NcAbiHashPart::Method {
            index: 0,
            name: "byte_len",
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Out,
            ty: NcAbiType::U64,
        },
        NcAbiHashPart::Status,
        NcAbiHashPart::Method {
            index: 1,
            name: "byte_at",
        },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::Value,
            ty: NcAbiType::U64,
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Out,
            ty: NcAbiType::U8,
        },
        NcAbiHashPart::Status,
        NcAbiHashPart::Method {
            index: 2,
            name: "copy_to",
        },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::CopyToBufferOffset,
            ty: NcAbiType::U8,
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Void,
            ty: NcAbiType::Void,
        },
        NcAbiHashPart::Status,
    ]);

    #[doc(hidden)]
    pub const BYTE_LEN_PARAMS: [NcMethodParamDescriptor; 0] = [];

    #[doc(hidden)]
    pub const BYTE_LEN_RETURN: NcMethodReturnDescriptor = NcMethodReturnDescriptor {
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Out),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::U64),
    };

    #[doc(hidden)]
    pub const BYTE_AT_PARAMS: [NcMethodParamDescriptor; 1] = [NcMethodParamDescriptor {
        name: NcStaticUtf8::from_static("index"),
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Value),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::U64),
    }];

    #[doc(hidden)]
    pub const BYTE_AT_RETURN: NcMethodReturnDescriptor = NcMethodReturnDescriptor {
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Out),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::U8),
    };

    #[doc(hidden)]
    pub const COPY_TO_PARAMS: [NcMethodParamDescriptor; 1] = [NcMethodParamDescriptor {
        name: NcStaticUtf8::from_static("out"),
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::CopyToBufferOffset),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::U8),
    }];

    #[doc(hidden)]
    pub const COPY_TO_RETURN: NcMethodReturnDescriptor = NcMethodReturnDescriptor {
        passing: NcAbiPassingDescriptor::from_abi_passing(NcAbiPassing::Void),
        ty: NcAbiTypeDescriptor::from_abi_type(NcAbiType::Void),
    };

    /// Static descriptors for this interface's ABI methods.
    pub const METHOD_DESCRIPTORS: [NcMethodDescriptor; 3] = [
        NcMethodDescriptor {
            name: NcStaticUtf8::from_static("byte_len"),
            param_count: Self::BYTE_LEN_PARAMS.len() as u32,
            params: Self::BYTE_LEN_PARAMS.as_ptr(),
            returns: Self::BYTE_LEN_RETURN,
        },
        NcMethodDescriptor {
            name: NcStaticUtf8::from_static("byte_at"),
            param_count: Self::BYTE_AT_PARAMS.len() as u32,
            params: Self::BYTE_AT_PARAMS.as_ptr(),
            returns: Self::BYTE_AT_RETURN,
        },
        NcMethodDescriptor {
            name: NcStaticUtf8::from_static("copy_to"),
            param_count: Self::COPY_TO_PARAMS.len() as u32,
            params: Self::COPY_TO_PARAMS.as_ptr(),
            returns: Self::COPY_TO_RETURN,
        },
    ];

    /// Static ABI descriptor for this interface.
    pub const INTERFACE_DESCRIPTOR: NcInterfaceDescriptor = NcInterfaceDescriptor {
        interface_id: <Self as ComInterface>::IID,
        qualified_name: NcStaticUtf8::from_static(QUALIFIED_NAME_INC_STRING),
        abi_hash: Self::ABI_HASH,
        method_count: Self::METHOD_DESCRIPTORS.len() as u32,
        methods: Self::METHOD_DESCRIPTORS.as_ptr(),
    };
}

#[cfg(test)]
mod tests {
    use super::{
        ComInterface, IID_INC_BYTES, IID_INC_STRING, INcBytesAbi, INcStringAbi,
        QUALIFIED_NAME_INC_BYTES, QUALIFIED_NAME_INC_STRING,
    };

    #[test]
    fn builtin_data_interface_identities_are_stable() {
        assert_eq!(
            IID_INC_BYTES.to_uuid_bytes(),
            [
                0xb0, 0xb0, 0xd6, 0xd2, 0xf1, 0x70, 0x54, 0xe9, 0xb6, 0xd5, 0x14, 0x3f, 0x94, 0x52,
                0xb9, 0x51,
            ]
        );
        assert_eq!(IID_INC_BYTES, <INcBytesAbi as ComInterface>::IID);
        assert_eq!(
            unsafe { INcBytesAbi::INTERFACE_DESCRIPTOR.qualified_name.as_str() },
            Ok(QUALIFIED_NAME_INC_BYTES)
        );
        assert_eq!(
            INcBytesAbi::ABI_HASH.bytes,
            [0x6a, 0xa1, 0x0a, 0x90, 0xea, 0xb4, 0xc2, 0xbc, 0x00, 0x1f, 0x86, 0x2c, 0x4a, 0x33, 0x32, 0x1d, 0x60, 0x02, 0xae, 0xfb, 0xdf, 0xa2, 0x71, 0x45, 0xfb, 0x09, 0x0b, 0x48, 0x04, 0x1d, 0x87, 0xfc,]
        );
        assert_eq!(
            INcBytesAbi::INTERFACE_DESCRIPTOR.interface_id,
            <INcBytesAbi as ComInterface>::IID
        );
        assert_eq!(
            INcBytesAbi::INTERFACE_DESCRIPTOR.abi_hash,
            INcBytesAbi::ABI_HASH
        );

        assert_eq!(
            IID_INC_STRING.to_uuid_bytes(),
            [
                0x01, 0xa5, 0xb1, 0x8c, 0xbe, 0x18, 0x5d, 0x39, 0xb7, 0x1f, 0x20, 0x2d, 0xf7, 0xc8,
                0x46, 0x64,
            ]
        );
        assert_eq!(IID_INC_STRING, <INcStringAbi as ComInterface>::IID);
        assert_eq!(
            unsafe { INcStringAbi::INTERFACE_DESCRIPTOR.qualified_name.as_str() },
            Ok(QUALIFIED_NAME_INC_STRING)
        );
        assert_eq!(
            INcStringAbi::ABI_HASH.bytes,
            [0x85, 0x57, 0x41, 0x99, 0x1f, 0x89, 0xe0, 0x9e, 0x40, 0x14, 0x30, 0xd0, 0x4d, 0x18, 0x7b, 0x0b, 0x9b, 0xe2, 0xb7, 0xfb, 0x7a, 0xfb, 0x44, 0x9d, 0x65, 0xed, 0x89, 0x2e, 0x37, 0x9d, 0x3d, 0x3a,]
        );
        assert_eq!(
            INcStringAbi::INTERFACE_DESCRIPTOR.interface_id,
            <INcStringAbi as ComInterface>::IID
        );
        assert_eq!(
            INcStringAbi::INTERFACE_DESCRIPTOR.abi_hash,
            INcStringAbi::ABI_HASH
        );
        assert_eq!(INcStringAbi::INTERFACE_DESCRIPTOR.method_count, 3);
        // SAFETY: The built-in interface descriptor references its static
        // three-element method table.
        let methods = unsafe {
            core::slice::from_raw_parts(
                INcStringAbi::INTERFACE_DESCRIPTOR.methods,
                INcStringAbi::INTERFACE_DESCRIPTOR.method_count as usize,
            )
        };
        assert_eq!(unsafe { methods[2].name.as_str() }, Ok("copy_to"));
        assert_eq!(methods[2].param_count, 1);
        // SAFETY: The built-in copy method descriptor references its static
        // one-element parameter table.
        let copy_params = unsafe { core::slice::from_raw_parts(methods[2].params, 1) };
        assert_eq!(
            copy_params[0].passing,
            crate::NcAbiPassingDescriptor::from_abi_passing(
                crate::NcAbiPassing::CopyToBufferOffset
            )
        );
    }

    #[test]
    fn builtin_iunknown_conversions_preserve_the_header_pointer_without_dereferencing() {
        assert!(INcBytesAbi::iunknown(core::ptr::null_mut()).is_null());
        assert!(INcStringAbi::iunknown(core::ptr::null_mut()).is_null());

        let bytes = core::ptr::NonNull::<INcBytesAbi>::dangling().as_ptr();
        let string = core::ptr::NonNull::<INcStringAbi>::dangling().as_ptr();
        assert_eq!(INcBytesAbi::iunknown(bytes), bytes.cast());
        assert_eq!(INcStringAbi::iunknown(string), string.cast());
    }
}
