//! Bounded capture of semantic initialization trees, with owned-path cycle checks.

use neocogi_com::{ComPtr, ComRef, INcBytesComPtrExt, IUnknown, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
use neocogi_com::persistence::{IPersistableAbi, Limits, ObjectRecord};

/// Exports a shared opt-in tree, detects ownership cycles, and bounds traversal.
/// A live exclusive root is a valid borrow but cannot be retained along the
/// capture path; after budget admission it returns `NC_E_INVALID_ARG` before
/// persistence getters run. Exhausted budgets retain OUT_OF_RANGE precedence.
pub(crate) fn snapshot_with_limits(object: ComRef<'_, IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
    let mut capture = Capture {
        limits,
        remaining: limits.objects,
        bytes_left: limits.binary.bytes,
        active: Vec::new(),
    };
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
}

impl Capture {
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
        let mut children = Vec::with_capacity(count as usize);
        for index in 0..count {
            children.push(self.object(value.child(index)?.as_com_ref())?);
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
