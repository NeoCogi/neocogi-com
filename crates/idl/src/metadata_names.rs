//! IDL-owned associated metadata names, shared by emission and source admission.

/// Per-value reflection occupies the native value's inherent namespace.
pub(crate) const VALUE_METADATA_MEMBER: &str = "IDL_TYPE";
/// Interface authoring metadata occupies its generated ABI marker's namespace.
pub(crate) const INTERFACE_METADATA_MEMBER: &str = "IDL_INTERFACE";
