# neocogi-inspect

Reads an actual plugin's `nc_get_plugin_descriptor_v1` export without
registering or activating its factory. It copies plugin identity/version, hard
dependencies, descriptor/core ABI versions, and full factory IID/signature and
threading capability. There are no contract libraries or module graphs to load.
Opened libraries remain pinned until the short-lived process exits.

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
to [local IDL metadata](../../crates/com/src/idl_metadata.rs).

`plan` rejects duplicate plugin identities, missing hard dependencies, and
cycles, then prints deterministic provider-first order. Disconnected contract
revisions may coexist. `diff --compat` reports changed factory signatures,
identities, target/ABI versions, weakened threading, and added hard dependencies
as breaking; plugin version text is diagnostic.

Inspection maps trusted native code and can execute platform loader initializers.
It never calls the factory bootstrap or recursively locates provider plugins.
The [sample plugin](https://github.com/NeoCogi/neocogi-com/blob/master/examples/com-plugin/README.md) can be built and
inspected with ordinary Cargo, without the assembly driver.
