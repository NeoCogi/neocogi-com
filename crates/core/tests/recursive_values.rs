//! End-to-end ownership and malformed-wire tests for the recursive value contract.
#![cfg(feature = "alloc")]

use neocogi_com_core::{
    ComValue, NcStatus, NcValueRef, ValueBudget, ValueOption, ValueSequenceStorage, ValueSlice, NC_E_INVALID_ARG, NC_E_INVALID_TAG, NC_E_OUT_OF_RANGE,
};

/// A finite application tree with an explicitly indirect recursive edge.
#[derive(Debug, PartialEq)]
struct Node {
    /// Restricted scalar must use a checked byte even inside a recursive tree.
    enabled: bool,
    /// Child ownership is finite and independent after decoding.
    children: Vec<Node>,
}
/// The recursive wire has finite size because sequence storage is indirect.
#[repr(C)]
#[derive(Clone, Copy)]
struct NodeWire {
    /// Unrestricted byte checked before becoming a Rust bool.
    enabled: u8,
    /// Points at raw child records, never at application Nodes.
    children: ValueSlice<NodeWire>,
}
/// Concrete guards own every level of converted sequence backing.
struct NodeStorage {
    /// Vec indirection also gives recursively defined storage a finite size.
    _children: ValueSequenceStorage<Node>,
}
// SAFETY: fields are converted recursively, and guards retain all wire backing.
unsafe impl ComValue for Node {
    type Wire = NodeWire;
    type Storage = NodeStorage;
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(NodeWire, NodeStorage), NcStatus> {
        budget.value(|budget| {
            let (enabled, ()) = self.enabled.encode_value(budget)?;
            let (children, storage) = self.children.encode_value(budget)?;
            Ok((NodeWire { enabled, children }, NodeStorage { _children: storage }))
        })
    }
    unsafe fn decode_value(wire: NodeWire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|budget| {
            // SAFETY: caller supplies initialized raw fields and live pointees.
            Ok(Self {
                enabled: unsafe { bool::decode_value(wire.enabled, budget) }?,
                children: unsafe { Vec::<Node>::decode_value(wire.children, budget) }?,
            })
        })
    }
}

#[test]
fn recursive_guard_survives_moves_and_acquires_independent_children() {
    let source = Node {
        enabled: true,
        children: vec![Node { enabled: false, children: vec![] }],
    };
    let guard = Box::new(NcValueRef::new(&source).unwrap());
    // Moving the guard cannot invalidate its separately allocated wire elements.
    let moved = *guard;
    let decoded = moved.to_owned().unwrap();
    assert_eq!(decoded, source);
    drop(moved);
    drop(source);
    assert!(!decoded.children[0].enabled);
}

#[test]
fn invalid_boolean_inside_sequence_is_rejected_without_typed_bool_borrow() {
    let raw_children = [NodeWire {
        enabled: 7,
        children: ValueSlice { pointer: core::ptr::null(), len: 0 },
    }];
    let raw = NodeWire {
        enabled: 1,
        children: ValueSlice { pointer: raw_children.as_ptr(), len: 1 },
    };
    // SAFETY: storage is readable and initialized; invalid scalar values are intentional.
    assert_eq!(unsafe { Node::decode_value(raw, &mut ValueBudget::default()) }, Err(NC_E_INVALID_ARG));
}

#[test]
fn optional_tag_selects_whether_payload_may_be_read() {
    let absent = ValueOption::<u8> {
        present: 0,
        payload: core::mem::MaybeUninit::uninit(),
    };
    let invalid = ValueOption::<u8> {
        present: 9,
        payload: core::mem::MaybeUninit::uninit(),
    };
    // SAFETY: absent and invalid tags do not require initialized payload storage.
    assert_eq!(unsafe { Option::<bool>::decode_value(absent, &mut ValueBudget::default()) }, Ok(None));
    assert_eq!(
        unsafe { Option::<bool>::decode_value(invalid, &mut ValueBudget::default()) },
        Err(NC_E_INVALID_TAG)
    );
}

#[test]
fn zero_sized_values_still_consume_work_and_nested_values_share_limits() {
    let values = vec![(); 4];
    assert!(matches!(values.encode_value(&mut ValueBudget::new(64, 4, 8)), Err(NC_E_OUT_OF_RANGE)));
    let text = vec![String::from("abcd"), String::from("efgh")];
    let guard = NcValueRef::new(&text).unwrap();
    // Two allocations must share the remaining bytes after the outer Vec storage.
    let mut budget = ValueBudget::new(2 * core::mem::size_of::<String>() + 7, 100, 8);
    assert_eq!(
        unsafe { Vec::<String>::decode_value(guard.as_ptr().read(), &mut budget) },
        Err(NC_E_OUT_OF_RANGE)
    );
    let tree = Node {
        enabled: true,
        children: vec![Node { enabled: false, children: vec![] }],
    };
    assert!(matches!(tree.encode_value(&mut ValueBudget::new(4096, 100, 2)), Err(NC_E_OUT_OF_RANGE)));
}

#[test]
fn array_fields_check_each_boolean_and_preflight_text_shape() {
    let raw = neocogi_com_core::ValueArray { elements: [1, 0, 3] };
    assert_eq!(unsafe { <[bool; 3]>::decode_value(raw, &mut ValueBudget::default()) }, Err(NC_E_INVALID_ARG));
    // Oversized lengths are rejected before dereferencing the intentionally null pointer.
    let raw = ValueSlice {
        pointer: core::ptr::null(),
        len: u64::MAX,
    };
    assert_eq!(unsafe { String::decode_value(raw, &mut ValueBudget::default()) }, Err(NC_E_OUT_OF_RANGE));
}
