//! Compile-time assembly of interface ancestry and deduplicated class metadata.
//!
//! Generated code passes ordinary static references, so these helpers never
//! dereference foreign descriptor pointers during constant evaluation.

use super::{NcClassInterfaceDescriptor, NcInterfaceDescriptor};

/// Builds a derived-first ancestry list from one local descriptor and its base.
#[doc(hidden)]
pub const fn __nc_interface_chain<const N: usize>(
    own: &'static NcInterfaceDescriptor,
    base: &[&'static NcInterfaceDescriptor],
) -> [&'static NcInterfaceDescriptor; N] {
    assert!(N == base.len() + 1, "invalid interface ancestry length");
    let mut result = [own; N];
    let mut index = 0;
    while index < base.len() {
        result[index + 1] = base[index];
        index += 1;
    }
    result
}

/// Reports whether an interface appeared earlier in declaration/ancestry order.
/// Repeated IIDs must have identical contracts, even when inherited indirectly.
const fn repeated(chains: &[&[&NcInterfaceDescriptor]], chain: usize, item: usize) -> bool {
    let current = chains[chain][item];
    let mut previous_chain = 0;
    while previous_chain <= chain {
        let limit = if previous_chain == chain { item } else { chains[previous_chain].len() };
        let mut previous_item = 0;
        while previous_item < limit {
            let previous = chains[previous_chain][previous_item];
            if bytes_equal(&current.interface_id.to_uuid_bytes(), &previous.interface_id.to_uuid_bytes()) {
                assert!(
                    current.abi_signature.__const_eq(previous.abi_signature),
                    "conflicting inherited interface contracts"
                );
                return true;
            }
            previous_item += 1;
        }
        previous_chain += 1;
    }
    false
}

/// Const-compatible byte equality for GUIDs and signatures.
const fn bytes_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Counts unique contracts across the declared interface ancestry lists.
#[doc(hidden)]
pub const fn __nc_class_interface_count(chains: &[&[&NcInterfaceDescriptor]]) -> usize {
    let mut count = 0;
    let mut chain = 0;
    while chain < chains.len() {
        let mut item = 0;
        while item < chains[chain].len() {
            if !repeated(chains, chain, item) {
                count += 1;
            }
            item += 1;
        }
        chain += 1;
    }
    assert!(count <= u32::MAX as usize, "class interface count exceeds the ABI limit");
    count
}

/// Emits one metadata row per distinct IID in the same order used by queries.
#[doc(hidden)]
pub const fn __nc_class_interfaces<const N: usize>(chains: &[&[&'static NcInterfaceDescriptor]]) -> [NcClassInterfaceDescriptor; N] {
    assert!(N == __nc_class_interface_count(chains), "invalid class interface count");
    let mut result = [NcClassInterfaceDescriptor { interface_descriptor: core::ptr::null() }; N];
    let mut output = 0;
    let mut chain = 0;
    while chain < chains.len() {
        let mut item = 0;
        while item < chains[chain].len() {
            if !repeated(chains, chain, item) {
                result[output].interface_descriptor = chains[chain][item];
                output += 1;
            }
            item += 1;
        }
        chain += 1;
    }
    result
}
