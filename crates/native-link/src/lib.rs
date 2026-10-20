//! Native linkage for one target-profile runtime staged beside the application.
//!
//! Cargo artifact paths are compiler inputs. The staging macro returns only a
//! filename, while native search metadata is supplied by build.rs. Unix runtime
//! lookup stays inside the application: app/lib for executables and the same
//! directory for DLLs. Windows additionally receives nc_com.dll beside the exe.
//! This private integration uses nightly; portable COM/IDL consumers remain stable.
#![feature(link_arg_attribute)]
#![deny(missing_docs)]

const _: &str = neocogi_native_layout::stage_library!("CARGO_CDYLIB_FILE_COM_LIBRARY_nc_com");

// The final native assembly owns this import. Facade/runtime capability features
// contain declarations only and cannot suppress it or retain an implementation.
#[cfg_attr(not(target_env = "msvc"), link(name = "nc_com"))]
#[cfg_attr(target_env = "msvc", link(name = "nc_com.dll.lib", kind = "dylib", modifiers = "+verbatim"))]
unsafe extern "C" {}

#[cfg(all(unix, not(target_os = "macos")))]
#[allow(clippy::duplicated_attributes, reason = "ordered link-arg attributes supply an option and its value")]
#[link(kind = "link-arg", name = "-rpath")]
#[link(kind = "link-arg", name = "$ORIGIN/lib:$ORIGIN")]
unsafe extern "C" {}

#[cfg(target_os = "macos")]
#[allow(clippy::duplicated_attributes, reason = "ordered link-arg attributes supply each option and its value")]
#[link(kind = "link-arg", name = "-rpath")]
#[link(kind = "link-arg", name = "@loader_path/lib")]
#[link(kind = "link-arg", name = "-rpath")]
#[link(kind = "link-arg", name = "@loader_path")]
unsafe extern "C" {}
