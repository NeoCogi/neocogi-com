//! Bounded copying from provider-owned byte objects into runtime-owned memory.

use neocogi_com_core::{ComPtr, INcBytesAbi, NcStatus, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE};

/// Checks a foreign length before reserving storage or asking the provider to copy.
pub(super) fn copy_bounded(bytes: &ComPtr<INcBytesAbi>, maximum: usize) -> Result<Vec<u8>, NcStatus> {
    let pointer = bytes.as_raw();
    let mut length = 0;
    // SAFETY: The COM owner keeps the interface live and length is writable.
    let status = unsafe { ((*pointer.vtable).len)(pointer.object, &mut length) };
    if status.is_err() {
        return Err(status);
    }
    let length = usize::try_from(length).map_err(|_| NC_E_OUT_OF_RANGE)?;
    if length > maximum {
        return Err(NC_E_OUT_OF_RANGE);
    }
    let mut result = Vec::new();
    result.try_reserve_exact(length).map_err(|_| NC_E_OUT_OF_MEMORY)?;
    result.resize(length, 0);
    // SAFETY: The destination contains length initialized bytes and cannot
    // overlap provider storage. Failed copies are discarded with this vector.
    let status = unsafe { ((*pointer.vtable).copy_to)(pointer.object, 0, length as u64, result.as_mut_ptr()) };
    if status.is_err() {
        return Err(status);
    }
    Ok(result)
}
