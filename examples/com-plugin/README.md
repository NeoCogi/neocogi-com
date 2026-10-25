# One provider, dynamic or static

The provider independently generates bindings from shared [IDL sources](../schemas/neocogi/idl_example)
with private ordinary Rust state and typed factory methods. It depends on the
COM facade, runs IDL as a build dependency, and receives runtime access from its host:

```rust,ignore
#[neocogi_com::plugin(name = "nc.examples.factory_plugin_component")]
fn acquire_root(_runtime: neocogi_com::Runtime)
    -> Result<neocogi_com::ComPtr<crate::contracts::ExampleFactoryAbi>, neocogi_com::NcStatus>
{
    DynamicExampleFactory::from_value(DynamicExampleFactory).query()
}
```

The separate [host](../com-host) owns the runtime implementation. Its
`#[neocogi_com::com_main]` entry receives a copyable handle and registers/acquires
the provider. A dynamic provider uses the same host tables without importing
runtime symbols or embedding an implementation. Core/runtime/plugin remain ABI v1;
rebuild pre-release participants together.

From the repository root on Linux:

```sh
cargo rustc -p neocogi-example-com-plugin --lib --crate-type cdylib --features dynamic-plugin
cargo build -p neocogi-example-com-host --bin neocogi-example-com
target/debug/neocogi-example-com "$PWD/target/debug/libnc_example_com.so"
```

Deploy the executable and provider library. No COM runtime library, native bridge,
import library or COM loader environment is needed. macOS uses `.dylib` and Windows
uses `.dll`; ordinary system dependencies still apply. `--release` selects
`target/release` for both commands.

For a static provider:

```sh
cargo run -p neocogi-example-com-host --features monolith
```

Cargo links the provider rlib and runtime into the host. Only the executable is
deployed. Both compositions print `factory object: 44 x 31` after checking typed
construction, inherited/mutable calls, copied output, domain notifications,
weak-origin promotion/expiry, persistence and final release.

The host implements a concrete observer and queues owned `ExtentNotification`
facts with weak producer identity; UI is not involved. Its persistence adapter
retains that observer separately from the saved dimensions and supplies it during
restoration. The runtime-free [state schema](../com-host/support/extent_state.rs)
is shared as source between assemblies; dynamic host code links no provider just
to obtain schema facts.

The [compatibility driver](../../scripts/compatibility.py) builds the
host and providers independently with selected compiler/panic/LTO/layout policies,
removes targets, and executes a relocated deployment. Its independent peer B
publishes a host-visible marker and acquires A through supplied access. The host
also checks that a conflicting descriptor leaves the shared catalog unchanged.
The [static relocation test](../com-host/tests/build_modes.rs) covers the monolith.
See [qualification](../../docs/abi-v1.md#independent-build-compatibility).

The allocation probe executes 10,000 delivered notifications and 10,000 unobserved
updates, checking that unobserved updates allocate nothing:

```sh
cargo run -p neocogi-example-com-host --example event_allocations --features monolith
```
