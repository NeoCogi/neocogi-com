//! Native link metadata for the one Cargo-owned runtime artifact.
//!
//! Consumers import this crate as `_` at their native boundary. Native linker
//! arguments propagate through Rust libraries to the final executable or DLL,
//! unlike `cargo::rustc-link-arg` from a dependency's build script. Cargo builds
//! the runtime as a normal target artifact with the selected application profile.
//!
//! Unix binaries record the exact build-tree runtime directory as an rpath and
//! can start without Cargo's loader environment. These paths are development
//! linkage, not a relocatable installation. MSVC receives its native library
//! search directory; Windows execution still requires an explicit DLL layout.
//!
//! This workspace integration uses nightly `link_arg_attribute`; portable COM,
//! IDL and object macros retain their stable Rust consumer boundary.
#![feature(link_arg_attribute)]
#![deny(missing_docs)]

neocogi_com_macros::__nc_link_runtime_artifact!("CARGO_CDYLIB_DIR_COM_LIBRARY");
