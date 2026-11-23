//! Bounded capture of semantic initialization trees, with owned-path cycle checks.

use neocogi_com::raw;
use neocogi_com_sdk::{ComBytesExt, ComPtr, ComRaw, IUnknown, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE};
use neocogi_com_sdk::persistence::{IPersistableAbi, Limits, ObjectRecord};

/// Exports a shared opt-in tree, detects ownership cycles, and bounds traversal.
/// The root consumes one traversal budget after exact identity validation and
/// before any retain, query or provider selection.
///
/// # Safety
/// A present raw pair borrows a live shared root owner for this operation. Its
/// common table must be readable and provide the matching lifecycle operations.
pub(crate) unsafe fn snapshot_with_limits(object: ComRaw<IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
    // SAFETY: The raw contract supplies a live shared lender. This checks only
    // the shape and exact interface before the traversal budget is consumed.
    unsafe { raw::validate_interface(object) }?;
    let mut capture = Capture::new(limits);
    capture.admit()?;
    // SAFETY: The preceding check established exact identity, and the declared
    // shared owner remains alive while AddRef acquires this root for traversal.
    let root = unsafe { ComPtr::clone_from_raw(object) }.ok_or(NC_E_INVALID_ARG)?;
    let record = capture.admitted_object(&root)?;
    // Complete capture first so ownership, cycle and traversal-budget failures
    // preserve their precedence over codec availability and format selection.
    let codec = super::ncb_format(limits.binary)?;
    codec.format.serialize(&record).map_err(|_| NC_E_INVALID_ARG)
}

/// Per-snapshot traversal state, uniquely borrowed throughout recursive capture.
struct Capture {
    /// Immutable caller policy used for every descendant.
    limits: Limits,
    /// Object budget shared across siblings, never reset when unwinding a path.
    remaining: usize,
    /// Aggregate provider-state byte budget left before final document encoding.
    bytes_left: usize,
    /// Identity along the active path. Each recursive frame borrows a live owner.
    active: Vec<*mut core::ffi::c_void>,
    /// Capacity requests made by the real traversal in a bounded test.
    #[cfg(test)]
    child_capacity_requests: Vec<usize>,
}

impl Capture {
    /// Starts one traversal with the full caller policy and no retained owners.
    fn new(limits: Limits) -> Self {
        Self {
            limits,
            remaining: limits.objects,
            bytes_left: limits.binary.bytes,
            active: Vec::new(),
            #[cfg(test)]
            child_capacity_requests: Vec::new(),
        }
    }

    /// Charges one traversal entry before any query or acquisition for that entry.
    fn admit(&mut self) -> Result<(), NcStatus> {
        // Refuse work before querying providers once any traversal budget is spent.
        if self.remaining == 0 || self.active.len() >= self.limits.depth {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.remaining -= 1;
        Ok(())
    }

    /// Captures one already-budgeted object, using the frame's owner to keep its
    /// address valid across descendants and reject cycles through any projection.
    fn admitted_object(&mut self, object: &ComPtr<IUnknown>) -> Result<ObjectRecord, NcStatus> {
        let identity = object.as_raw().object;
        if self.active.contains(&identity) {
            return Err(NC_E_INVALID_ARG);
        }
        let value = object.query::<IPersistableAbi>()?;
        self.active.push(identity);
        let count = value.child_count()?;
        if count as usize > self.remaining || count as usize > self.limits.binary.elements {
            return Err(NC_E_OUT_OF_RANGE);
        }
        let state = value.save_state()?;
        // The SDK owns length/capacity admission and publishes initialized bytes
        // only after a complete copy. Empty state needs no foreign copy call.
        let state = state.to_vec_bounded(self.bytes_left.min(self.limits.binary.elements))?;
        self.bytes_left -= state.len();
        let mut children = Vec::new();
        for index in 0..count {
            let child = value.child(index)?;
            self.admit()?;
            let record = self.admitted_object(&child)?;
            // Grow only for a completed child. Ancestors retain capacity
            // proportional to their acquired children during descent.
            if children.len() == children.capacity() {
                let capacity = children.capacity().saturating_mul(2).max(1).min(count as usize);
                #[cfg(test)]
                self.child_capacity_requests.push(capacity);
                children.try_reserve_exact(capacity - children.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            }
            children.push(record);
        }
        let result = ObjectRecord {
            type_id: value.type_id()?.to_uuid_bytes(),
            schema_version: value.schema_version()?,
            state,
            children,
        };
        // An error discards the traversal. On success, remove the ancestor so a
        // later sibling can reference the same object without creating a cycle.
        self.active.pop();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com_sdk::{INcBytesAbi, NC_E_NOT_FOUND, NcBytes, NcGuid};
    use neocogi_com_sdk::persistence::{IPersistable, IPersistableVtblFor};

    /// Ordinary COM provider with an advertised list and a possible getter failure.
    #[neocogi_com_sdk::com_object(IPersistable)]
    struct Node {
        /// Admitted child count, including getters that may return failure.
        count: u32,
        /// Children with a successful getter at their authored index.
        children: Vec<ComPtr<IUnknown>>,
    }

    impl IPersistable for Node {
        fn type_id(&self) -> Result<NcGuid, NcStatus> {
            Ok(neocogi_com_sdk::guid_from_qualified_name("nc.tests.snapshot_capacity"))
        }

        fn schema_version(&self) -> Result<u32, NcStatus> {
            Ok(1)
        }

        fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
            Ok(NcBytes::from_vec(Vec::new()))
        }

        fn child_count(&self) -> Result<u32, NcStatus> {
            Ok(self.count)
        }

        fn child(&self, index: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
            self.children.get(index as usize).cloned().ok_or(NC_E_NOT_FOUND)
        }
    }

    /// Lends a normal shared owner to the production capture path.
    fn node(count: u32, children: Vec<ComPtr<IUnknown>>) -> ComPtr<IUnknown> {
        Node::from_value(Node { count, children })
    }

    #[test]
    fn only_completed_snapshot_children_request_slots() {
        let leaf = node(0, Vec::new());
        let inner = node(2, vec![leaf.clone()]);
        let outer = node(3, vec![leaf, inner]);
        let root = node(4, vec![outer]);
        let mut capture = Capture::new(Limits::default());
        capture.admit().unwrap();
        assert_eq!(capture.admitted_object(&root), Err(NC_E_NOT_FOUND));
        assert_eq!(capture.child_capacity_requests, [1, 1]);

        let full = node(3, (0..3).map(|_| node(0, Vec::new())).collect());
        let mut capture = Capture::new(Limits::default());
        capture.admit().unwrap();
        assert_eq!(capture.admitted_object(&full).unwrap().children.len(), 3);
        // The allocator may provide extra capacity. Every request still stays
        // within the admitted count, and no leaf requests child slots.
        assert_eq!(capture.child_capacity_requests.first(), Some(&1));
        assert!(capture.child_capacity_requests.iter().all(|&requested| requested <= 3));
    }
}
