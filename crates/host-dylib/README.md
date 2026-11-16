# neocogi-com-host-dylib

**WIP ABI v3:** this unreleased bootstrap contract is provisional and not frozen.

Optional native `cdylib` shim over `neocogi-com-host`. The application loads one
selected runtime image instead of linking the static host implementation.
The shim contains no registry or service implementation of its own.

Its `nc_get_host_api_v3` export returns the implementation's canonical
`NcHostApiV3` through a caller-owned output slot. Null output returns
`NC_E_POINTER`; repeated successful calls return the same immutable graph.
The application retains the image for process lifetime and passes its `RuntimeHandle`
to all providers, including nested acquisitions. Select one static or dynamic
host owner; separate runtime images can still own separate registries.

Build with `cargo build -p neocogi-com-host-dylib`. Applications use the SDK's
unsafe `RuntimeHandle::load_host_library` with an explicit trusted filesystem path.
Plugin libraries continue receiving host access during factory acquisition;
they do not load or initialize another host. This runtime image is not a plugin
and is not opened with `RuntimeHandle::load_plugin_library`.

This development shim uses ABI v3/core generation 21, matching the supplied
plugin acquisition ABI. No Rust ABI crosses this bootstrap boundary. Static and Wasm
applications continue selecting the ordinary host rlib.
