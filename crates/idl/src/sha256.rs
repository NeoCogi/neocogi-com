//! IDL fingerprint hashing shares the implementation used for intrinsic types.
//!
//! Tool-owned declaration hashing runs here; normal runtime dispatch retains
//! only its final digest, never the source graph or a serializer for it.
pub(crate) use neocogi_com_core::__nc_abi_digest as digest;
