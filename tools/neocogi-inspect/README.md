# neocogi-inspect

Reads an actual plugin's `nc_get_plugin_descriptor_v1` export without
registering or activating its factory. It copies plugin identity/version, hard
dependencies, descriptor/core ABI versions, and full factory IID/signature and
threading capability. There are no contract libraries or module graphs to load.
Opened libraries remain pinned until the short-lived process exits.

Install the matching SDK release after publication:

```sh
cargo install neocogi-inspect --version '=0.0.0' --locked
```

The [first application and plugin](https://github.com/NeoCogi/neocogi-com/blob/master/examples/getting-started/README.md)
walkthrough creates a library you can inspect.

```sh
neocogi-inspect dump path/to/plugin.so
neocogi-inspect dump path/to/plugin.so --format text
neocogi-inspect dump path/to/plugin.so --output plugin.nccom.json
neocogi-inspect check path/to/plugin.so
neocogi-inspect plan path/to/base.so path/to/dependent.so
neocogi-inspect diff old.nccom.json new.nccom.json
neocogi-inspect diff old.nccom.json new.nccom.json --compat
```

Pass values to `--output` / `-o` and `--format` / `-f` as separate arguments.
Output files use the exact supplied native path.

JSON uses strict schema 9 and plugin descriptor ABI 1. The core object ABI
is checked against the current COM runtime contract.
Unknown fields and unsupported schema versions are rejected. The artifact records
the plugin and its root factory. Source-level method and value definitions belong
to [local IDL metadata](https://docs.rs/neocogi-com/latest/neocogi_com/idl_metadata/index.html).

`plan` rejects duplicate plugin identities, missing hard dependencies, and
cycles, then prints deterministic provider-first order. Disconnected contract
revisions may coexist. `diff --compat` reports changed factory signatures,
identities, target/ABI versions, weakened threading, and added hard dependencies
as breaking; plugin version text is diagnostic.

Inspection maps trusted native code and can execute platform loader initializers.
It never calls the factory bootstrap or recursively locates provider plugins.
On Windows, paths select explicit files; bare names do not request PATH discovery.
This open searches ordinary imports beside the selected DLL and in System32,
subject to Windows module reuse/redirection. Colocate private native dependencies.
Later provider loads and delay loads follow the provider's own policy. This is
executable inspection, not a sandbox or publisher authentication. Successfully
opened mappings stay pinned until the inspector exits; inspection errors remain
recoverable. See the
[native loading contract](https://github.com/NeoCogi/neocogi-com/blob/master/docs/abi-v1.md#runtime-and-global-registry).
The [sample plugin](https://github.com/NeoCogi/neocogi-com/blob/master/examples/com-plugin/README.md) can be built and
inspected with ordinary Cargo, without the assembly driver.
