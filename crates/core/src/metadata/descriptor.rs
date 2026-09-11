//! Physical C-compatible interface metadata records.
//!
//! The structures in this module form the immutable pointer graph returned by
//! generated interface descriptions: methods, parameters, return contracts,
//! and static text. Accessor methods
//! validate null/count shapes before turning raw arrays into borrowed Rust views.
//!
//! Descriptor graphs are immutable, library-owned metadata. Relationships use
//! pointer/count pairs so hosts can inspect them without sharing Rust
//! allocation ownership. Accessors validate local shape such as null/count
//! consistency, but callers remain responsible for keeping the exporting
//! module loaded for every returned borrow.

use super::contract::{NcAbiPassingDescriptor, NcAbiTypeDescriptor, NcInterfaceId};
use crate::guid::NcGuid;
use crate::signature::NcAbiSignature;
use crate::status::{NC_E_INVALID_ARG, NC_E_POINTER, NcStatus};

/// C-stable typed ABI descriptor for one method parameter.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcMethodParamDescriptor {
    /// Static parameter name metadata for diagnostics.
    pub name: NcStaticUtf8,
    /// ABI passing mode for this parameter.
    pub passing: NcAbiPassingDescriptor,
    /// ABI type for this parameter.
    pub ty: NcAbiTypeDescriptor,
}

// SAFETY: Parameter descriptors are immutable static metadata.
unsafe impl Sync for NcMethodParamDescriptor {}

/// C-stable typed ABI descriptor for one method success return value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcMethodReturnDescriptor {
    /// ABI passing mode for this return.
    pub passing: NcAbiPassingDescriptor,
    /// ABI type for this return.
    pub ty: NcAbiTypeDescriptor,
}

/// Static UTF-8 metadata stored in descriptor tables.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcStaticUtf8 {
    /// Pointer to UTF-8 bytes that remain valid while the shared library is
    /// loaded.
    pub data: *const u8,
    /// Number of bytes at `data`.
    pub len: u32,
}

// SAFETY: This descriptor is immutable ABI metadata. Producers must keep `data`
// valid while the descriptor is visible to a host.
unsafe impl Sync for NcStaticUtf8 {}

impl NcStaticUtf8 {
    /// Empty static UTF-8 metadata.
    pub const EMPTY: Self = Self { data: core::ptr::null(), len: 0 };

    /// Creates static UTF-8 metadata from a string literal or other static
    /// string.
    pub const fn from_static(value: &'static str) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len() as u32,
        }
    }

    /// Returns the byte length.
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Returns whether the metadata string is empty.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows the metadata bytes.
    ///
    /// # Safety
    ///
    /// If `self.len != 0`, `self.data` must point to `self.len` bytes that
    /// remain valid for the returned lifetime.
    pub unsafe fn as_bytes<'a>(self) -> Result<&'a [u8], NcStatus> {
        if self.len == 0 {
            return Ok(&[]);
        }
        if self.data.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller guarantees that non-empty metadata points to
        // `len` bytes valid for the returned lifetime; null was rejected above.
        Ok(unsafe { core::slice::from_raw_parts(self.data, self.len as usize) })
    }

    /// Borrows the metadata as UTF-8 text.
    ///
    /// # Safety
    ///
    /// The same pointer validity requirements as [`NcStaticUtf8::as_bytes`]
    /// apply.
    pub unsafe fn as_str<'a>(self) -> Result<&'a str, NcStatus> {
        // SAFETY: The caller supplied the same metadata lifetime and pointer
        // guarantees required by `as_bytes`; UTF-8 is validated afterward.
        core::str::from_utf8(unsafe { self.as_bytes()? }).map_err(|_| NC_E_INVALID_ARG)
    }
}

/// Static descriptor for one ABI method.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcMethodDescriptor {
    /// Static method name metadata.
    pub name: NcStaticUtf8,
    /// One of the `NC_ABI_RECEIVER_*` values describing Rust dispatch access.
    pub receiver: u32,
    /// Number of logical method parameters.
    pub param_count: u32,
    /// Pointer to `param_count` typed parameter descriptors.
    pub params: *const NcMethodParamDescriptor,
    /// Typed success-return descriptor.
    pub returns: NcMethodReturnDescriptor,
}

// SAFETY: Method descriptors are immutable static metadata; pointer fields
// reference other immutable metadata owned by the exporting component.
unsafe impl Sync for NcMethodDescriptor {}

/// Static descriptor for one ABI interface.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcInterfaceDescriptor {
    /// Stable interface ID derived from `qualified_name`, except for `IUnknown`.
    pub interface_id: NcGuid,
    /// Normative qualified name from which `interface_id` is derived; the
    /// canonical root interface instead uses the reserved name `IUnknown`.
    pub qualified_name: NcStaticUtf8,
    /// Canonical validation signature of the complete lowered interface ABI contract.
    ///
    /// This must match the canonical signature encoded from `methods`.
    pub abi_signature: NcAbiSignature,
    /// Explicit generated base contract, or `NONE` for the implicit IUnknown root.
    /// Methods below describe only the slots introduced by this interface.
    pub base: NcInterfaceId,
    /// Number of locally introduced method descriptors.
    pub method_count: u32,
    /// Pointer to `method_count` descriptors.
    pub methods: *const NcMethodDescriptor,
}

// SAFETY: Interface descriptors are immutable static metadata; `methods` is a
// read-only table that must remain live while the descriptor is exported.
unsafe impl Sync for NcInterfaceDescriptor {}

impl NcInterfaceDescriptor {
    /// Borrows the normative qualified interface name.
    ///
    /// # Safety
    ///
    /// `self.qualified_name` must reference metadata that remains valid for the
    /// returned lifetime.
    pub unsafe fn qualified_name_str<'a>(&self) -> Result<&'a str, NcStatus> {
        // SAFETY: The caller guarantees the interface descriptor and its
        // static string metadata remain valid for the returned lifetime.
        unsafe { self.qualified_name.as_str() }
    }
}

impl NcInterfaceDescriptor {
    /// Borrows the method descriptor slice.
    ///
    /// # Safety
    ///
    /// If `self.method_count != 0`, `self.methods` must point to
    /// `self.method_count` descriptors that remain valid for the returned
    /// lifetime.
    pub unsafe fn methods<'a>(&self) -> Result<&'a [NcMethodDescriptor], NcStatus> {
        if self.method_count == 0 {
            return Ok(&[]);
        }
        if self.methods.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller guarantees `methods` points to `method_count`
        // descriptors valid for the returned lifetime; null/count was checked.
        Ok(unsafe { core::slice::from_raw_parts(self.methods, self.method_count as usize) })
    }
}

impl NcMethodDescriptor {
    /// Borrows the parameter descriptor slice.
    ///
    /// # Safety
    ///
    /// If `self.param_count != 0`, `self.params` must point to
    /// `self.param_count` descriptors that remain valid for the returned
    /// lifetime.
    pub unsafe fn params<'a>(&self) -> Result<&'a [NcMethodParamDescriptor], NcStatus> {
        if self.param_count == 0 {
            return Ok(&[]);
        }
        if self.params.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: The caller guarantees `params` points to `param_count`
        // descriptors valid for the returned lifetime; null/count was checked.
        Ok(unsafe { core::slice::from_raw_parts(self.params, self.param_count as usize) })
    }
}

/// One hard neocogi-com plugin dependency.
///
/// Dependency metadata is declarative. It names a plugin that must already be
/// registered before the descriptor containing this entry may be registered;
/// it does not cause the runtime to locate or load another library.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcPluginDependencyDescriptor {
    /// Stable identity of the required plugin.
    pub plugin_id: NcGuid,
}

#[cfg(test)]
mod tests {
    use std::string::ToString as _;

    use crate::NcGuid;
    #[cfg(feature = "_com-validation-exact")]
    use crate::{NC_ABI_RECEIVER_MUTABLE, NC_ABI_RECEIVER_SHARED, NcAbiAggregateKind, NcAbiFieldLayout, NcAbiSignaturePart, NcAbiPassing, NcAbiReceiver, NcAbiType};
    use crate::{
        NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE, NC_ABI_VALIDATION_FNV_1A_128, NC_ABI_VALIDATION_MANGLING, NC_ABI_VALIDATION_NONE,
        NC_ABI_VALIDATION_SHA_256, NC_COM_CORE_ABI_VERSION, NcAbiSignature, NcAbiThreadingMode, NcAbiValidationMode, NcInterfaceDescriptor, NcInterfaceId,
        NcPluginDependencyDescriptor,
    };

    #[test]
    fn core_abi_identity_is_stable() {
        assert_eq!(NC_COM_CORE_ABI_VERSION, 7);
    }

    #[test]
    fn validation_mode_raw_values_and_names_are_stable() {
        let modes = [
            (NcAbiValidationMode::NONE, NC_ABI_VALIDATION_NONE, "none"),
            (NcAbiValidationMode::SHA_256, NC_ABI_VALIDATION_SHA_256, "sha-256"),
            (NcAbiValidationMode::FNV_1A_128, NC_ABI_VALIDATION_FNV_1A_128, "fnv-1a-128"),
            (NcAbiValidationMode::MANGLING, NC_ABI_VALIDATION_MANGLING, "mangling"),
        ];
        for (mode, raw, name) in modes {
            assert_eq!(mode.as_raw(), raw);
            assert_eq!(NcAbiValidationMode::from_raw(raw), Some(mode));
            assert_eq!(mode.as_str(), Some(name));
            assert_eq!(mode.to_string(), name);
        }
        assert_eq!(NcAbiValidationMode::from_raw(u32::MAX), None);
    }

    #[test]
    fn interface_id_none_requires_nil_iid_and_empty_signature() {
        assert!(NcInterfaceId::NONE.is_none());
        assert!(!NcInterfaceId::new(NcGuid::nil(), NcAbiSignature::from_static_str("ncabi1;test")).is_none());
    }

    #[test]
    fn threading_mode_raw_values_are_stable_and_checked() {
        assert_eq!(NcAbiThreadingMode::ThreadLocal.as_raw(), NC_ABI_THREADING_THREAD_LOCAL);
        assert_eq!(NcAbiThreadingMode::ThreadSafe.as_raw(), NC_ABI_THREADING_THREAD_SAFE);
        assert_eq!(
            NcAbiThreadingMode::from_raw(NC_ABI_THREADING_THREAD_LOCAL),
            Some(NcAbiThreadingMode::ThreadLocal)
        );
        assert_eq!(NcAbiThreadingMode::from_raw(NC_ABI_THREADING_THREAD_SAFE), Some(NcAbiThreadingMode::ThreadSafe));
        assert_eq!(NcAbiThreadingMode::from_raw(u32::MAX), None);
    }

    #[cfg(feature = "_com-validation-exact")]
    #[test]
    fn receiver_modes_are_stable_and_change_method_signatures() {
        assert_eq!(NcAbiReceiver::Shared.as_raw(), NC_ABI_RECEIVER_SHARED);
        assert_eq!(NcAbiReceiver::Mutable.as_raw(), NC_ABI_RECEIVER_MUTABLE);
        assert_eq!(NcAbiReceiver::from_raw(2), None);

        let shared = NcAbiSignature::from_abi_parts(&[NcAbiSignaturePart::Receiver { receiver: NcAbiReceiver::Shared }]);
        let mutable = NcAbiSignature::from_abi_parts(&[NcAbiSignaturePart::Receiver { receiver: NcAbiReceiver::Mutable }]);
        assert_ne!(shared, mutable);
    }

    #[test]
    fn interface_descriptor_c_layout_includes_qualified_name() {
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(core::mem::size_of::<NcInterfaceDescriptor>(), 96);
            assert_eq!(core::mem::align_of::<NcInterfaceDescriptor>(), 8);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, interface_id), 0);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, qualified_name), 16);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, abi_signature), 32);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, method_count), 80);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, methods), 88);
        }

        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(core::mem::size_of::<NcInterfaceDescriptor>(), 64);
            assert_eq!(core::mem::align_of::<NcInterfaceDescriptor>(), 4);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, interface_id), 0);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, qualified_name), 16);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, abi_signature), 24);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, method_count), 56);
            assert_eq!(core::mem::offset_of!(NcInterfaceDescriptor, methods), 60);
        }
    }

    #[test]
    fn plugin_dependency_c_layout_is_stable() {
        assert_eq!(core::mem::size_of::<NcPluginDependencyDescriptor>(), 16);
        assert_eq!(core::mem::align_of::<NcPluginDependencyDescriptor>(), 4);
        assert_eq!(core::mem::offset_of!(NcPluginDependencyDescriptor, plugin_id), 0);
    }

    #[cfg(feature = "_com-validation-exact")]
    #[test]
    fn method_signature_changes_for_method_identity_and_status() {
        let base = NcAbiSignature::from_abi_parts(&[
            NcAbiSignaturePart::Method { index: 0, name: "load" },
            NcAbiSignaturePart::Return {
                passing: NcAbiPassing::Out,
                ty: NcAbiType::U32,
            },
            NcAbiSignaturePart::Status,
        ]);

        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 1, name: "load" },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Out,
                    ty: NcAbiType::U32,
                },
                NcAbiSignaturePart::Status,
            ])
        );
        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 0, name: "store" },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Out,
                    ty: NcAbiType::U32,
                },
                NcAbiSignaturePart::Status,
            ])
        );
        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 0, name: "load" },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Out,
                    ty: NcAbiType::U32,
                },
            ])
        );
    }

    #[cfg(feature = "_com-validation-exact")]
    #[test]
    fn method_signature_changes_for_argument_and_return_contracts() {
        let base = NcAbiSignature::from_abi_parts(&[
            NcAbiSignaturePart::Method { index: 0, name: "calculate" },
            NcAbiSignaturePart::Arg {
                index: 0,
                passing: NcAbiPassing::Value,
                ty: NcAbiType::U32,
            },
            NcAbiSignaturePart::Arg {
                index: 1,
                passing: NcAbiPassing::Value,
                ty: NcAbiType::I32,
            },
            NcAbiSignaturePart::Return {
                passing: NcAbiPassing::Out,
                ty: NcAbiType::U64,
            },
            NcAbiSignaturePart::Status,
        ]);

        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 0, name: "calculate" },
                NcAbiSignaturePart::Arg {
                    index: 0,
                    passing: NcAbiPassing::Value,
                    ty: NcAbiType::U64,
                },
                NcAbiSignaturePart::Arg {
                    index: 1,
                    passing: NcAbiPassing::Value,
                    ty: NcAbiType::I32,
                },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Out,
                    ty: NcAbiType::U64,
                },
                NcAbiSignaturePart::Status,
            ])
        );
        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 0, name: "calculate" },
                NcAbiSignaturePart::Arg {
                    index: 1,
                    passing: NcAbiPassing::Value,
                    ty: NcAbiType::I32,
                },
                NcAbiSignaturePart::Arg {
                    index: 0,
                    passing: NcAbiPassing::Value,
                    ty: NcAbiType::U32,
                },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Out,
                    ty: NcAbiType::U64,
                },
                NcAbiSignaturePart::Status,
            ])
        );
        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 0, name: "calculate" },
                NcAbiSignaturePart::Arg {
                    index: 0,
                    passing: NcAbiPassing::Value,
                    ty: NcAbiType::U32,
                },
                NcAbiSignaturePart::Arg {
                    index: 1,
                    passing: NcAbiPassing::Value,
                    ty: NcAbiType::I32,
                },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Out,
                    ty: NcAbiType::U32,
                },
                NcAbiSignaturePart::Status,
            ])
        );
        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 0, name: "calculate" },
                NcAbiSignaturePart::Arg {
                    index: 0,
                    passing: NcAbiPassing::Value,
                    ty: NcAbiType::U32,
                },
                NcAbiSignaturePart::Arg {
                    index: 1,
                    passing: NcAbiPassing::Value,
                    ty: NcAbiType::I32,
                },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Void,
                    ty: NcAbiType::Void,
                },
                NcAbiSignaturePart::Status,
            ])
        );
    }

    #[cfg(feature = "_com-validation-exact")]
    #[test]
    fn method_signature_changes_for_com_pointer_modes_and_interface_identity() {
        let interface_guid = guid(1);
        let other_interface_guid = guid(2);

        let base = NcAbiSignature::from_abi_parts(&[
            NcAbiSignaturePart::Interface { iid: interface_guid },
            NcAbiSignaturePart::Method { index: 0, name: "consume" },
            NcAbiSignaturePart::Arg {
                index: 0,
                passing: NcAbiPassing::ComRef { mode: NcAbiThreadingMode::ThreadLocal },
                ty: NcAbiType::Interface { iid: interface_guid },
            },
            NcAbiSignaturePart::Return {
                passing: NcAbiPassing::ComPtrOut { mode: NcAbiThreadingMode::ThreadLocal },
                ty: NcAbiType::Interface { iid: interface_guid },
            },
            NcAbiSignaturePart::Status,
        ]);

        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Interface { iid: other_interface_guid },
                NcAbiSignaturePart::Method { index: 0, name: "consume" },
                NcAbiSignaturePart::Arg {
                    index: 0,
                    passing: NcAbiPassing::ComRef { mode: NcAbiThreadingMode::ThreadLocal },
                    ty: NcAbiType::Interface { iid: interface_guid },
                },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::ComPtrOut { mode: NcAbiThreadingMode::ThreadLocal },
                    ty: NcAbiType::Interface { iid: interface_guid },
                },
                NcAbiSignaturePart::Status,
            ])
        );
        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Interface { iid: interface_guid },
                NcAbiSignaturePart::Method { index: 0, name: "consume" },
                NcAbiSignaturePart::Arg {
                    index: 0,
                    passing: NcAbiPassing::ComRef { mode: NcAbiThreadingMode::ThreadSafe },
                    ty: NcAbiType::Interface { iid: interface_guid },
                },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::ComPtrOut { mode: NcAbiThreadingMode::ThreadLocal },
                    ty: NcAbiType::Interface { iid: interface_guid },
                },
                NcAbiSignaturePart::Status,
            ])
        );
        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Interface { iid: interface_guid },
                NcAbiSignaturePart::Method { index: 0, name: "consume" },
                NcAbiSignaturePart::Arg {
                    index: 0,
                    passing: NcAbiPassing::ComRef { mode: NcAbiThreadingMode::ThreadLocal },
                    ty: NcAbiType::Interface { iid: other_interface_guid },
                },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::ComPtrOut { mode: NcAbiThreadingMode::ThreadLocal },
                    ty: NcAbiType::Interface { iid: interface_guid },
                },
                NcAbiSignaturePart::Status,
            ])
        );
        assert_ne!(
            base,
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Interface { iid: interface_guid },
                NcAbiSignaturePart::Method { index: 0, name: "consume" },
                NcAbiSignaturePart::Arg {
                    index: 0,
                    passing: NcAbiPassing::ComRef { mode: NcAbiThreadingMode::ThreadLocal },
                    ty: NcAbiType::Interface { iid: interface_guid },
                },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::ComPtrOut { mode: NcAbiThreadingMode::ThreadSafe },
                    ty: NcAbiType::Interface { iid: interface_guid },
                },
                NcAbiSignaturePart::Status,
            ])
        );
    }

    #[cfg(feature = "_com-validation-exact")]
    #[test]
    fn method_signature_changes_for_offset_buffer_contracts() {
        fn offset_buffer_signature(passing: NcAbiPassing, index: u32, ty: NcAbiType) -> NcAbiSignature {
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 0, name: "copy" },
                NcAbiSignaturePart::Arg { index, passing, ty },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Void,
                    ty: NcAbiType::Void,
                },
                NcAbiSignaturePart::Status,
            ])
        }

        fn offset_buffer_signature_without_status(passing: NcAbiPassing) -> NcAbiSignature {
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Method { index: 0, name: "copy" },
                NcAbiSignaturePart::Arg { index: 0, passing, ty: NcAbiType::U8 },
                NcAbiSignaturePart::Return {
                    passing: NcAbiPassing::Void,
                    ty: NcAbiType::Void,
                },
            ])
        }

        let copy_to = offset_buffer_signature(NcAbiPassing::CopyToBufferOffset, 0, NcAbiType::U8);
        let copy_from = offset_buffer_signature(NcAbiPassing::CopyFromBufferOffset, 0, NcAbiType::U8);
        assert_ne!(copy_to, copy_from);

        for passing in [NcAbiPassing::CopyToBufferOffset, NcAbiPassing::CopyFromBufferOffset] {
            let base = offset_buffer_signature(passing, 0, NcAbiType::U8);

            assert_ne!(base, offset_buffer_signature(passing, 1, NcAbiType::U8));
            assert_ne!(base, offset_buffer_signature(passing, 0, NcAbiType::U16));
            assert_ne!(base, offset_buffer_signature_without_status(passing));
            assert_ne!(
                base,
                NcAbiSignature::from_abi_parts(&[
                    NcAbiSignaturePart::Method { index: 0, name: "copy" },
                    NcAbiSignaturePart::Arg {
                        index: 0,
                        passing: NcAbiPassing::Value,
                        ty: NcAbiType::U8,
                    },
                    NcAbiSignaturePart::Return {
                        passing: NcAbiPassing::Void,
                        ty: NcAbiType::Void,
                    },
                    NcAbiSignaturePart::Status,
                ])
            );
        }
    }

    #[cfg(feature = "_com-validation-exact")]
    #[test]
    fn aggregate_signature_changes_for_structural_shape_and_variants() {
        fn aggregate_signature(
            kind: NcAbiAggregateKind,
            size: usize,
            align: usize,
            fields: [NcAbiFieldLayout; 2],
            variants: [NcAbiSignaturePart<'_>; 2],
        ) -> NcAbiSignature {
            NcAbiSignature::from_abi_parts(&[
                NcAbiSignaturePart::Aggregate { kind },
                NcAbiSignaturePart::Layout { size, align },
                NcAbiSignaturePart::Field {
                    index: fields[0].index,
                    offset: fields[0].offset,
                    ty: fields[0].ty,
                },
                NcAbiSignaturePart::Field {
                    index: fields[1].index,
                    offset: fields[1].offset,
                    ty: fields[1].ty,
                },
                variants[0],
                variants[1],
            ])
        }

        let payload_a_fields = [NcAbiFieldLayout { index: 0, offset: 0, ty: NcAbiType::U32 }];
        let payload_b_fields = [NcAbiFieldLayout { index: 0, offset: 0, ty: NcAbiType::U64 }];
        let payload_a = NcAbiSignature::from_aggregate_layout(NcAbiAggregateKind::Struct, 4, 4, &payload_a_fields);
        let payload_b = NcAbiSignature::from_aggregate_layout(NcAbiAggregateKind::Struct, 8, 8, &payload_b_fields);
        assert_ne!(
            payload_a,
            NcAbiSignature::from_aggregate_layout(NcAbiAggregateKind::Struct, 8, 4, &payload_a_fields)
        );
        assert_ne!(
            payload_a,
            NcAbiSignature::from_aggregate_layout(NcAbiAggregateKind::Struct, 4, 8, &payload_a_fields)
        );
        assert_ne!(
            payload_a,
            NcAbiSignature::from_aggregate_layout(
                NcAbiAggregateKind::Struct,
                4,
                4,
                &[NcAbiFieldLayout { index: 0, offset: 1, ty: NcAbiType::U32 }],
            )
        );

        let base_fields = [
            NcAbiFieldLayout { index: 0, offset: 0, ty: NcAbiType::U32 },
            NcAbiFieldLayout {
                index: 1,
                offset: 8,
                ty: NcAbiType::Aggregate(payload_a),
            },
        ];
        let base_variants = [
            NcAbiSignaturePart::Variant { index: 0, tag: 1, ty: NcAbiType::Void },
            NcAbiSignaturePart::Variant {
                index: 1,
                tag: 2,
                ty: NcAbiType::Aggregate(payload_a),
            },
        ];
        let base = aggregate_signature(NcAbiAggregateKind::TaggedEnum, 16, 8, base_fields, base_variants);

        assert_ne!(base, aggregate_signature(NcAbiAggregateKind::Union, 16, 8, base_fields, base_variants,));
        assert_ne!(
            base,
            aggregate_signature(
                NcAbiAggregateKind::TaggedEnum,
                16,
                8,
                [
                    NcAbiFieldLayout {
                        index: 1,
                        offset: 8,
                        ty: NcAbiType::Aggregate(payload_a),
                    },
                    NcAbiFieldLayout { index: 0, offset: 0, ty: NcAbiType::U32 },
                ],
                base_variants,
            )
        );
        assert_ne!(
            base,
            aggregate_signature(
                NcAbiAggregateKind::TaggedEnum,
                16,
                8,
                [
                    NcAbiFieldLayout { index: 0, offset: 4, ty: NcAbiType::U32 },
                    NcAbiFieldLayout {
                        index: 1,
                        offset: 8,
                        ty: NcAbiType::Aggregate(payload_a),
                    },
                ],
                base_variants,
            )
        );
        assert_ne!(
            base,
            aggregate_signature(
                NcAbiAggregateKind::TaggedEnum,
                16,
                8,
                base_fields,
                [
                    NcAbiSignaturePart::Variant { index: 0, tag: 7, ty: NcAbiType::Void },
                    NcAbiSignaturePart::Variant {
                        index: 1,
                        tag: 2,
                        ty: NcAbiType::Aggregate(payload_a),
                    },
                ],
            )
        );
        assert_ne!(
            base,
            aggregate_signature(
                NcAbiAggregateKind::TaggedEnum,
                16,
                8,
                base_fields,
                [
                    NcAbiSignaturePart::Variant { index: 0, tag: 1, ty: NcAbiType::Void },
                    NcAbiSignaturePart::Variant {
                        index: 1,
                        tag: 2,
                        ty: NcAbiType::Aggregate(payload_b),
                    },
                ],
            )
        );
    }

    #[cfg(feature = "_com-validation-exact")]
    fn guid(byte: u8) -> NcGuid {
        NcGuid::from_fields(0x8c6b_7200 | u32::from(byte), 0x6ce7, 0x42fa, [byte; 8])
    }
}
