//! Bounded capture of semantic initialization trees, with owned-path cycle checks.

use neocogi_com::{ComPtr, ComRef, ComBytesExt, IUnknown, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE};
use neocogi_com::persistence::{IPersistableAbi, Limits, ObjectRecord};

/// Exports a shared opt-in tree, detects ownership cycles, and bounds traversal.
/// A live exclusive root is a valid borrow but cannot be retained along the
/// capture path; after budget admission it returns `NC_E_INVALID_ARG` before
/// persistence getters run. Exhausted budgets retain OUT_OF_RANGE precedence.
pub(crate) fn snapshot_with_limits(object: ComRef<'_, IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
    let mut capture = Capture::new(limits);
    let record = capture.object(object)?;
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
    /// Strong owners retain allocation identity only along the current ancestor path.
    active: Vec<ComPtr<IUnknown>>,
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

    /// Captures one object while rejecting cycles only on its active ancestor path.
    fn object(&mut self, object: ComRef<'_, IUnknown>) -> Result<ObjectRecord, NcStatus> {
        // Refuse work before querying providers once any traversal budget is spent.
        if self.remaining == 0 || self.active.len() >= self.limits.depth {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.remaining -= 1;
        // A borrow does not promise shared ownership. Reject non-shareable roots
        // through the Result before querying any persistence provider methods.
        // Tables identify dispatch views; retained allocation identity detects cycles
        // through every valid projection without another exact-interface query.
        let identity = object.try_add_ref()?;
        if self.active.iter().any(|parent| parent.as_com_ref().same_object(identity.as_com_ref())) {
            return Err(NC_E_INVALID_ARG);
        }
        let value = identity.query::<IPersistableAbi>()?;
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
            let record = self.object(child.as_com_ref())?;
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
        // Only successful paths pop: on error the complete Capture is dropped,
        // releasing every retained ancestor. Repeated noncyclic references remain legal.
        self.active.pop();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com::{INcBytesAbi, NC_E_NOT_FOUND, NcBytes, NcGuid};
    use neocogi_com::persistence::{IPersistable, IPersistableVtblFor};

    /// Ordinary COM provider with an advertised list and a possible getter failure.
    #[neocogi_com::com_object(IPersistable)]
    struct Node {
        /// Admitted child count, including getters that may return failure.
        count: u32,
        /// Children with a successful getter at their authored index.
        children: Vec<ComPtr<IUnknown>>,
    }

    impl IPersistable for Node {
        fn type_id(&self) -> Result<NcGuid, NcStatus> {
            Ok(neocogi_com::guid_from_qualified_name("nc.tests.snapshot_capacity"))
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
        assert_eq!(capture.object(root.as_com_ref()), Err(NC_E_NOT_FOUND));
        assert_eq!(capture.child_capacity_requests, [1, 1]);

        let full = node(3, (0..3).map(|_| node(0, Vec::new())).collect());
        let mut capture = Capture::new(Limits::default());
        assert_eq!(capture.object(full.as_com_ref()).unwrap().children.len(), 3);
        // The allocator may provide extra capacity. Every request still stays
        // within the admitted count, and no leaf requests child slots.
        assert_eq!(capture.child_capacity_requests.first(), Some(&1));
        assert!(capture.child_capacity_requests.iter().all(|&requested| requested <= 3));
    }
}
