//! Compile-time placement of canonical one-pointer interface headers.
//!
//! Generated implementations use these records only to construct static vtables.
//! Runtime dispatch reads one byte offset from the actual projection's vtable;
//! it never searches this layout or retains method/parameter schema pointers.

use super::{NcInterfaceDescriptor, NcInterfaceId, inheritance::same_identity};

/// Compile-time association between one exact contract and its storage offset.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct __ComHeaderLayout {
    /// Contract occupying this header, with its complete signature.
    pub identity: NcInterfaceId,
    /// Bytes from the owning allocation to this embedded interface header.
    pub object_offset: usize,
}

/// Places the canonical contract order in one contiguous header array.
/// `first` is obtained with `offset_of!` on the concrete generated storage type.
#[doc(hidden)]
pub const fn __nc_header_layout<const N: usize>(contracts: &[&NcInterfaceDescriptor; N], first: usize) -> [__ComHeaderLayout; N] {
    let mut result = [__ComHeaderLayout {
        identity: NcInterfaceId::NONE,
        object_offset: 0,
    }; N];
    let mut index = 0;
    while index < N {
        // The generated allocation already proves this array fits. Checked
        // arithmetic also makes misuse of this compiler helper fail explicitly.
        let displacement = match index.checked_mul(core::mem::size_of::<crate::IUnknown>()) {
            Some(value) => value,
            None => panic!("interface header displacement overflow"),
        };
        let object_offset = match first.checked_add(displacement) {
            Some(value) => value,
            None => panic!("interface header offset overflow"),
        };
        result[index] = __ComHeaderLayout {
            identity: NcInterfaceId::new(contracts[index].interface_id, contracts[index].abi_signature),
            object_offset,
        };
        index += 1;
    }
    result
}

/// Selects the outer projection's offset while constructing an inherited vtable.
/// Every inherited prefix must use the actual outer identity, not its own base
/// identity: inherited methods receive the outer header as their `this` pointer.
#[doc(hidden)]
pub const fn __nc_interface_offset(layout: &[__ComHeaderLayout], identity: &NcInterfaceId) -> usize {
    let mut index = 0;
    while index < layout.len() {
        if same_identity(&layout[index].identity, identity) {
            return layout[index].object_offset;
        }
        index += 1;
    }
    panic!("interface vtable has no header in this implementation's storage")
}
