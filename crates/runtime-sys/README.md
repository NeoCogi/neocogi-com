# neocogi-com-runtime-sys

`neocogi-com-runtime-sys` contains link-time declarations for the C ABI exported by a
`libnc_com_shared_runtime` shared library.

This is a client binding, not another runtime instance: it contains no registry
or mutable runtime state. When Cargo unifies the shared-runtime feature through
an application and its plugins, every emitted external call resolves to the
same loader-owned shared object and therefore the same registry. That feature
unification is intentional. Only `static-runtime` must be kept out of separate
plugin DSOs, because static linkage would create an implementation in each image.

This crate intentionally has no runtime implementation. It re-exports
`neocogi-com-core` ABI types and declares `nc_com_runtime_abi_version` plus the five
runtime ABI v1 table getters. There is no contract registry or event seeding.
Callers must check the runtime ABI version before calling a getter.
They are responsible for linking an
already built `neocogi-com-runtime` shared library and for making that library
discoverable by the platform loader.

Set `NC_COM_RUNTIME_LINK_DIR` to the directory containing the built runtime
shared library (and, on Windows, its import library) when Cargo also needs to
link a host against it. The build script adds that directory and its `deps`
subdirectory to the native linker search path. The shared library must still be
discoverable through `PATH`, `DYLD_LIBRARY_PATH`, or `LD_LIBRARY_PATH` when the
host runs.

Most Rust hosts should use `neocogi-com` with its `runtime` feature instead. Use this
crate directly only when building a lower-level wrapper around the shared
runtime C ABI.

## Static binding

Enable `static-runtime` through the `neocogi-com` facade for a static/browser host.
Cargo then links `neocogi-com-runtime` once as an ordinary dependency; the bindings
call the same C symbols, tables, and catalog implementation. No separately built
runtime library or `NC_COM_RUNTIME_LINK_DIR` is needed. A native plugin assembly
must use shared mode consistently: do not enable static-runtime in each DSO,
which would give each binary its own registry. The static feature wins when Cargo
unifies both features, so `--all-features` is unsuitable for shared deployments.

`nc_com_get_persistence_api_v1` provides caller-scoped registry creation,
registration, snapshot delivery, restoration, and context destruction. Contexts
and returned COM objects must be released through their owning runtime/COM
operations; the ABI never transfers Rust allocation ownership.
