//! Human-readable rendering of structured method ABI descriptors.
//!
//! Signatures produced here are diagnostics, not an ABI authority or a format
//! that consumers should parse back into contracts. The renderer validates the
//! local passing/type combinations it reads and emits deterministic text for
//! inspection tools and compatibility reports.
//!
//! Rendered signatures are diagnostic output only. Compatibility is determined
//! by the typed descriptors and their canonical ABI hash, so this module must
//! never become a second parser or source of contract truth.

use core::fmt::Write;

use crate::{
    NC_ABI_PASSING_COM_PTR_OUT, NC_ABI_PASSING_COM_REF, NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET,
    NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET, NC_ABI_PASSING_OUT, NC_ABI_PASSING_VALUE,
    NC_ABI_PASSING_VOID, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE,
    NC_ABI_TYPE_AGGREGATE, NC_ABI_TYPE_BOOL, NC_ABI_TYPE_F32, NC_ABI_TYPE_F64, NC_ABI_TYPE_I8,
    NC_ABI_TYPE_I16, NC_ABI_TYPE_I32, NC_ABI_TYPE_I64, NC_ABI_TYPE_INTERFACE, NC_ABI_TYPE_ISIZE,
    NC_ABI_TYPE_NC_ABI_HASH, NC_ABI_TYPE_NC_GUID, NC_ABI_TYPE_NC_STATUS, NC_ABI_TYPE_U8,
    NC_ABI_TYPE_U16, NC_ABI_TYPE_U32, NC_ABI_TYPE_U64, NC_ABI_TYPE_USIZE, NC_ABI_TYPE_VOID,
    NC_E_INVALID_ARG, NcAbiHash, NcAbiPassingDescriptor, NcAbiTypeDescriptor, NcMethodDescriptor,
    NcMethodParamDescriptor, NcMethodReturnDescriptor, NcStatus,
};

use alloc::string::String;

/// Renders the diagnostic typed ABI signature for a method descriptor.
///
/// The returned text is derived from the structured parameter and return ABI
/// descriptors, so it is diagnostic metadata rather than a separate source of
/// compatibility truth.
///
/// # Safety
///
/// Every pointer transitively reached through `method` must satisfy the ABI-v1
/// descriptor contract for the duration of this call. In particular, a
/// non-empty method or parameter name must point to its declared number of
/// readable bytes, and nonzero `param_count` requires `params` to point to that
/// many readable [`NcMethodParamDescriptor`] values. The descriptor graph must
/// remain immutable while it is read.
///
/// Generated descriptors and descriptors accepted from successfully loaded
/// plugins meet these requirements while their exporting module remains mapped.
/// Callers handling arbitrary public descriptor records must establish the
/// pointer validity separately; null/count and UTF-8 validation cannot prove
/// that a non-null address is readable.
pub unsafe fn format_method_signature(method: &NcMethodDescriptor) -> Result<String, NcStatus> {
    // SAFETY: The caller guarantees that the method name's pointer/count pair
    // references readable bytes for this call. `as_str` checks null and UTF-8.
    let method_name = unsafe { method.name.as_str()? };
    // SAFETY: The caller guarantees that the parameter pointer/count pair is
    // readable for this call.
    let params = unsafe { method_params(method)? };

    let mut out = String::new();
    out.push_str(method_name);
    out.push('(');
    for (index, param) in params.iter().enumerate() {
        if index != 0 {
            out.push_str(", ");
        }
        // SAFETY: Each parameter is in the caller-validated descriptor graph,
        // including its transitive name bytes.
        unsafe { write_param(&mut out, param)? };
    }
    out.push_str(") -> ");
    write_return(&mut out, &method.returns)?;
    out.push_str(" status");
    Ok(out)
}

/// Borrows the method parameter table after forwarding the caller's graph contract.
unsafe fn method_params(
    method: &NcMethodDescriptor,
) -> Result<&[NcMethodParamDescriptor], NcStatus> {
    // SAFETY: The caller guarantees that the method's parameter pointer/count
    // pair is readable for the returned borrow.
    unsafe { method.params() }
}

/// Appends one named parameter and its passing/type contract.
unsafe fn write_param(out: &mut String, param: &NcMethodParamDescriptor) -> Result<(), NcStatus> {
    // SAFETY: The caller guarantees that the parameter name's pointer/count
    // pair references readable bytes. `as_str` checks null and UTF-8.
    out.push_str(unsafe { param.name.as_str()? });
    out.push_str(": ");
    write_passing_type(out, param.passing, &param.ty)
}

/// Appends and validates the method's return passing contract.
fn write_return(out: &mut String, returns: &NcMethodReturnDescriptor) -> Result<(), NcStatus> {
    match returns.passing.tag {
        NC_ABI_PASSING_VOID => {
            validate_threading_local(returns.passing.threading)?;
            if returns.ty.tag != NC_ABI_TYPE_VOID {
                return Err(NC_E_INVALID_ARG);
            }
            out.push_str("void");
        }
        NC_ABI_PASSING_OUT => {
            validate_threading_local(returns.passing.threading)?;
            validate_non_interface_value_type(&returns.ty)?;
            out.push_str("out<");
            write_type(out, &returns.ty)?;
            out.push('>');
        }
        NC_ABI_PASSING_COM_PTR_OUT => {
            validate_interface_type(&returns.ty)?;
            out.push_str("com_ptr_out<");
            write_threading(out, returns.passing.threading)?;
            out.push_str(", ");
            write_type(out, &returns.ty)?;
            out.push('>');
        }
        _ => return Err(NC_E_INVALID_ARG),
    }
    Ok(())
}

/// Appends one supported parameter passing mode and its nested type.
fn write_passing_type(
    out: &mut String,
    passing: NcAbiPassingDescriptor,
    ty: &NcAbiTypeDescriptor,
) -> Result<(), NcStatus> {
    match passing.tag {
        NC_ABI_PASSING_VALUE => {
            validate_threading_local(passing.threading)?;
            validate_non_interface_value_type(ty)?;
            out.push_str("value<");
            write_type(out, ty)?;
            out.push('>');
        }
        NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET => {
            validate_threading_local(passing.threading)?;
            validate_non_interface_value_type(ty)?;
            out.push_str("copy_from_buffer_offset<");
            write_type(out, ty)?;
            out.push('>');
        }
        NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET => {
            validate_threading_local(passing.threading)?;
            validate_non_interface_value_type(ty)?;
            out.push_str("copy_to_buffer_offset<");
            write_type(out, ty)?;
            out.push('>');
        }
        NC_ABI_PASSING_COM_REF => {
            validate_interface_type(ty)?;
            out.push_str("com_ref<");
            write_threading(out, passing.threading)?;
            out.push_str(", ");
            write_type(out, ty)?;
            out.push('>');
        }
        _ => return Err(NC_E_INVALID_ARG),
    }
    Ok(())
}

/// Appends a diagnostic name for a validated ABI type descriptor.
fn write_type(out: &mut String, ty: &NcAbiTypeDescriptor) -> Result<(), NcStatus> {
    match ty.tag {
        NC_ABI_TYPE_VOID => out.push_str("void"),
        NC_ABI_TYPE_BOOL => out.push_str("bool"),
        NC_ABI_TYPE_U8 => out.push_str("u8"),
        NC_ABI_TYPE_U16 => out.push_str("u16"),
        NC_ABI_TYPE_U32 => out.push_str("u32"),
        NC_ABI_TYPE_U64 => out.push_str("u64"),
        NC_ABI_TYPE_USIZE => out.push_str("usize"),
        NC_ABI_TYPE_I8 => out.push_str("i8"),
        NC_ABI_TYPE_I16 => out.push_str("i16"),
        NC_ABI_TYPE_I32 => out.push_str("i32"),
        NC_ABI_TYPE_I64 => out.push_str("i64"),
        NC_ABI_TYPE_ISIZE => out.push_str("isize"),
        NC_ABI_TYPE_F32 => out.push_str("f32"),
        NC_ABI_TYPE_F64 => out.push_str("f64"),
        NC_ABI_TYPE_NC_GUID => out.push_str("NcGuid"),
        NC_ABI_TYPE_NC_STATUS => out.push_str("NcStatus"),
        NC_ABI_TYPE_NC_ABI_HASH => out.push_str("NcAbiHash"),
        NC_ABI_TYPE_AGGREGATE => {
            if ty.aggregate_hash.bytes == [0; 32] {
                return Err(NC_E_INVALID_ARG);
            }
            out.push_str("aggregate<");
            write_hash(out, ty.aggregate_hash);
            out.push('>');
        }
        NC_ABI_TYPE_INTERFACE => {
            validate_interface_type(ty)?;
            write!(out, "interface<{}, ", ty.interface.iid).map_err(|_| NC_E_INVALID_ARG)?;
            write_hash(out, ty.interface.abi_hash);
            out.push('>');
        }
        _ => return Err(NC_E_INVALID_ARG),
    }
    Ok(())
}

/// Appends the diagnostic name for a raw threading tag.
fn write_threading(out: &mut String, threading: u32) -> Result<(), NcStatus> {
    match threading {
        NC_ABI_THREADING_THREAD_LOCAL => out.push_str("thread_local"),
        NC_ABI_THREADING_THREAD_SAFE => out.push_str("thread_safe"),
        _ => return Err(NC_E_INVALID_ARG),
    }
    Ok(())
}

/// Requires the neutral threading tag used by non-interface passing modes.
fn validate_threading_local(threading: u32) -> Result<(), NcStatus> {
    if threading == NC_ABI_THREADING_THREAD_LOCAL {
        Ok(())
    } else {
        Err(NC_E_INVALID_ARG)
    }
}

/// Rejects void and interface descriptors from direct value positions.
fn validate_non_interface_value_type(ty: &NcAbiTypeDescriptor) -> Result<(), NcStatus> {
    if ty.tag == NC_ABI_TYPE_VOID || ty.tag == NC_ABI_TYPE_INTERFACE {
        Err(NC_E_INVALID_ARG)
    } else {
        Ok(())
    }
}

/// Requires a non-nil interface identity with a nonzero ABI hash.
fn validate_interface_type(ty: &NcAbiTypeDescriptor) -> Result<(), NcStatus> {
    if ty.tag != NC_ABI_TYPE_INTERFACE
        || ty.interface.iid.is_nil()
        || ty.interface.abi_hash.bytes == [0; 32]
    {
        Err(NC_E_INVALID_ARG)
    } else {
        Ok(())
    }
}

/// Appends a lowercase hexadecimal representation of all hash bytes.
fn write_hash(out: &mut String, hash: NcAbiHash) {
    for byte in hash.bytes {
        let _ = write!(out, "{byte:02x}");
    }
}
