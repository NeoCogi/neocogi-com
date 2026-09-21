//! Bounded capture of semantic initialization trees, with owned-path cycle checks.

use neocogi_com_core::{binary, ComPtr, ComRef, IUnknown, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
use neocogi_com_core::persistence::{IPersistableAbi, Limits, ObjectRecord};

/// Exports only opt-in state, detects ownership cycles, and bounds child traversal.
pub(crate) fn snapshot_with_limits(object: ComRef<'_, IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
    let mut capture = Capture {
        limits,
        remaining: limits.objects,
        bytes_left: limits.binary.bytes,
        active: Vec::new(),
    };
    let record = capture.object(object)?;
    binary::to_vec_with_limits(&record, limits.binary).map_err(|_| NC_E_INVALID_ARG)
}

/// Per-snapshot traversal state, uniquely borrowed throughout recursive capture.
struct Capture {
    /// Immutable caller policy used for every descendant.
    limits: Limits,
    /// Object budget shared across siblings, never reset when unwinding a path.
    remaining: usize,
    /// Aggregate provider-state byte budget left before final document encoding.
    bytes_left: usize,
    /// Canonical COM identities retained only along the current ownership path.
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
        // Canonical IUnknown identity catches cycles even through different projections.
        let identity = object.add_ref().query::<IUnknown>()?;
        if self.active.iter().any(|parent| parent.as_mut_ptr() == identity.as_mut_ptr()) {
            return Err(NC_E_INVALID_ARG);
        }
        let value = identity.query::<IPersistableAbi>()?;
        self.active.push(identity);
        let count = value.child_count()?;
        if count as usize > self.remaining || count as usize > self.limits.binary.elements {
            return Err(NC_E_OUT_OF_RANGE);
        }
        let state = value.save_state()?;
        // Inspect the remote byte count before copying consumer-owned storage.
        let state = super::bytes::copy_bounded(&state, self.bytes_left.min(self.limits.binary.elements))?;
        if state.len() > self.bytes_left || state.len() > self.limits.binary.elements {
            return Err(NC_E_OUT_OF_RANGE);
        }
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
