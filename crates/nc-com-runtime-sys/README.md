# nc-com-runtime-sys

`nc-com-runtime-sys` contains link-time declarations for the C ABI exported by a
`libnc_com_runtime` shared library.

This crate intentionally has no runtime implementation. It re-exports
`nc-com-core` ABI types and declares `nc_com_runtime_abi_version` plus the five
versioned runtime table getters. Callers must check the runtime ABI version
before calling a getter. They are responsible for linking an
already built `nc-com-runtime` shared library and for making that library
discoverable by the platform loader.

Set `NC_COM_RUNTIME_LINK_DIR` to the directory containing the built runtime
shared library (and, on Windows, its import library) when Cargo also needs to
link a host against it. The build script adds that directory and its `deps`
subdirectory to the native linker search path. The shared library must still be
discoverable through `PATH`, `DYLD_LIBRARY_PATH`, or `LD_LIBRARY_PATH` when the
host runs.

Most Rust hosts should use `nc-com` with its `runtime` feature instead. Use this
crate directly only when building a lower-level wrapper around the shared
runtime C ABI.
