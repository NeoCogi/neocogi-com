//! Bounded capture of semantic initialization trees, with owned-path cycle checks.

use neocogi_com_core::{binary, ComPtr, ComRef, IUnknown, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
use neocogi_com_core::persistence::{IPersistableAbi, Limits, ObjectRecord};

/// Exports only opt-in state, detects ownership cycles, and bounds child traversal.
pub(crate) fn snapshot_with_limits(object: ComRef<'_, IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
    let mut remaining = limits.objects;
    let mut active = Vec::new();
    let mut bytes_left = limits.binary.bytes;
    let record = capture(object, limits, &mut remaining, &mut bytes_left, &mut active)?;
    binary::to_vec_with_limits(&record, limits.binary).map_err(|_| NC_E_INVALID_ARG)
}

/// Recursively captures semantic state while keeping identity checks local.
fn capture(
    object: ComRef<'_, IUnknown>,
    limits: Limits,
    remaining: &mut usize,
    bytes_left: &mut usize,
    active: &mut Vec<ComPtr<IUnknown>>,
) -> Result<ObjectRecord, NcStatus> {
    if *remaining == 0 || active.len() >= limits.depth {
        return Err(NC_E_OUT_OF_RANGE);
    }
    *remaining -= 1;
    let identity = object.add_ref().query::<IUnknown>()?;
    if active.iter().any(|parent| parent.as_mut_ptr() == identity.as_mut_ptr()) {
        return Err(NC_E_INVALID_ARG);
    }
    let value = identity.query::<IPersistableAbi>()?;
    active.push(identity);
    let count = value.child_count()?;
    if count as usize > *remaining || count as usize > limits.binary.elements {
        return Err(NC_E_OUT_OF_RANGE);
    }
    let state = value.save_state()?;
    // Inspect the remote byte count before copying consumer-owned storage.
    let state = super::bytes::copy_bounded(&state, (*bytes_left).min(limits.binary.elements))?;
    if state.len() > *bytes_left || state.len() > limits.binary.elements {
        return Err(NC_E_OUT_OF_RANGE);
    }
    *bytes_left -= state.len();
    let mut children = Vec::with_capacity(count as usize);
    for index in 0..count {
        children.push(capture(value.child(index)?.as_com_ref(), limits, remaining, bytes_left, active)?);
    }
    let result = ObjectRecord {
        type_id: value.type_id()?.to_uuid_bytes(),
        schema_version: value.schema_version()?,
        state,
        children,
    };
    active.pop();
    Ok(result)
}
