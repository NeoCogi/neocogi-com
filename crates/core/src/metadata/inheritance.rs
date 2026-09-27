//! Compile-time assembly of interface ancestry and deduplicated object metadata.
//!
//! Generated code passes ordinary static references, so these helpers never
//! dereference foreign descriptor pointers during constant evaluation.
//! The ordered set is an input to compilation and explicit inspection. Object
//! construction and querying must not read it at runtime: retaining descriptor
//! pointers there also retains method/type descriptions and their relocations.

use super::{NcInterfaceDescriptor, NcInterfaceId};

/// Builds a derived-first ancestry list from one descriptor and all required projections.
#[doc(hidden)]
pub const fn __nc_interface_chain<const N: usize>(
    own: &'static NcInterfaceDescriptor,
    required: &[&[&'static NcInterfaceDescriptor]],
) -> [&'static NcInterfaceDescriptor; N] {
    let mut result = [own; N];
    let mut output = 1;
    let mut chain = 0;
    // Each parent already describes its distinct projections. Merge those
    // finite sets in declaration order instead of expanding ancestry paths.
    // The same identity must retain one exact signature through every parent.
    while chain < required.len() {
        let mut item = 0;
        while item < required[chain].len() {
            let candidate = required[chain][item];
            let mut previous = 0;
            while previous < output && !same_contract(result[previous], candidate) {
                previous += 1;
            }
            if previous == output {
                assert!(output < N, "interface projection count is too small");
                result[output] = candidate;
                output += 1;
            }
            item += 1;
        }
        chain += 1;
    }
    assert!(N == output, "invalid interface projection count");
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
            if same_contract(current, previous) {
                return true;
            }
            previous_item += 1;
        }
        previous_chain += 1;
    }
    false
}

/// Compares nominal identities and rejects incompatible revisions of one identity.
/// Both metadata merging and provider assembly use this exact contract rule.
pub(crate) const fn same_contract(left: &NcInterfaceDescriptor, right: &NcInterfaceDescriptor) -> bool {
    same_identity(
        &NcInterfaceId::new(left.interface_id, left.abi_signature),
        &NcInterfaceId::new(right.interface_id, right.abi_signature),
    )
}

/// Validates nominal identity consistently for descriptor and typed-vtable merging.
/// Only compact identities are needed by provider bindings; they never own schemas.
pub(crate) const fn same_identity(left: &NcInterfaceId, right: &NcInterfaceId) -> bool {
    if !bytes_equal(&left.iid.to_uuid_bytes(), &right.iid.to_uuid_bytes()) {
        return false;
    }
    assert!(left.abi_signature.__const_eq(right.abi_signature), "conflicting inherited interface contracts");
    true
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
pub const fn __nc_object_interface_count(chains: &[&[&NcInterfaceDescriptor]]) -> usize {
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
    assert!(count <= u32::MAX as usize, "object interface count exceeds the ABI limit");
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NcAbiSignature, NcGuid, NcInterfaceId, NcStaticUtf8};

    /// Builds allocation-free descriptors with independently controlled identity and revision.
    const fn descriptor(id: u32, signature: &'static [u8; 32]) -> NcInterfaceDescriptor {
        NcInterfaceDescriptor {
            interface_id: NcGuid::from_fields(id, 0, 0, [0; 8]),
            qualified_name: NcStaticUtf8::from_static("test.Interface"),
            abi_signature: NcAbiSignature::from_static(signature),
            base: NcInterfaceId::NONE,
            method_count: 0,
            methods: core::ptr::null(),
        }
    }
    /// Shared leaf encountered through both sides of the diamond.
    const ROOT: NcInterfaceDescriptor = descriptor(1, b"00000000000000000000000000000001");
    /// First parent determines the initial discovery order of shared ancestry.
    const LEFT: NcInterfaceDescriptor = descriptor(2, b"00000000000000000000000000000002");
    /// Second parent must reuse the leaf rather than append a second copy.
    const RIGHT: NcInterfaceDescriptor = descriptor(3, b"00000000000000000000000000000003");
    /// Concrete top of the diamond.
    const TOP: NcInterfaceDescriptor = descriptor(4, b"00000000000000000000000000000004");
    /// A compatible identity cannot silently select a different ABI revision.
    const WRONG_ROOT: NcInterfaceDescriptor = descriptor(1, b"00000000000000000000000000000099");
    /// Evaluated at compilation, as generated INTERFACE_CHAIN constants are.
    const DIAMOND: [&NcInterfaceDescriptor; 4] = __nc_interface_chain(&TOP, &[&[&LEFT, &ROOT], &[&RIGHT, &ROOT]]);

    /// Count and canonical enumeration agree without repeating the shared leaf.
    #[test]
    fn diamond_metadata_is_a_distinct_ordered_set() {
        let ids = DIAMOND.map(|descriptor| descriptor.interface_id);
        assert_eq!(ids, [TOP.interface_id, LEFT.interface_id, ROOT.interface_id, RIGHT.interface_id]);
        assert_eq!(__nc_object_interface_count(&[&DIAMOND, &[&LEFT, &ROOT]]), 4);
    }

    /// Metadata construction validates every repeated identity before discarding it.
    #[test]
    #[should_panic(expected = "conflicting inherited interface contracts")]
    fn metadata_merge_rejects_conflicting_shared_ancestor() {
        let _ = __nc_interface_chain::<4>(&TOP, &[&[&LEFT, &ROOT], &[&RIGHT, &WRONG_ROOT]]);
    }
}
