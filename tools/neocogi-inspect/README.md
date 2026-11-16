# neocogi-inspect

Reads an actual plugin's `nc_get_plugin_descriptor_v3` export without
registering or activating its factory. It copies plugin identity/version, hard
dependencies, descriptor/core ABI versions, and full factory IID/signature and
threading capability. There are no contract libraries or module graphs to load.
Opened libraries remain pinned until the short-lived process exits.

These development sources inspect the [current WIP ABI versions](../../docs/abi-v3.md#abi-identities-and-versions). Build the matching
CLI from this checkout with `cargo build -p neocogi-inspect` when inspecting
development providers. The published `0.3.0` inspector expects ABI v2/core
generation 19 and cannot inspect these incompatible providers. See the
[provider migration guide](../../docs/guides/ABI_V3_MIGRATION.md).

For published ABI-v2 providers, install the released `0.3.0` inspector:

```sh
cargo install neocogi-inspect --version '=0.3.0' --locked
```

The [first application and plugin](https://github.com/NeoCogi/neocogi-com/blob/v0.3.0/examples/getting-started/README.md)
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

JSON uses strict schema 9 and plugin descriptor ABI 2. The core object ABI
is checked against the current COM runtime contract.
Unknown fields and unsupported schema versions are rejected. The artifact records
the plugin and its root factory. Source-level method and value definitions belong
to [local IDL metadata](https://docs.rs/neocogi-com/0.3.0/neocogi_com/idl_metadata/index.html).

`plan` rejects duplicate plugin identities, missing hard dependencies, and
cycles, then prints deterministic provider-first order. Disconnected contract
revisions may coexist. `diff --compat` reports changed factory signatures,
identities, target/ABI versions, weakened threading, and added hard dependencies
as breaking; plugin version text is diagnostic. Text differences use canonical JSON
spellings and escaped strings, preserving concrete values in ordinary and
min-size builds alike.
Native inspection failures also preserve the loader phase, path, platform or
validation detail and any ABI status in both profiles.

Inspection maps trusted native code and can execute platform loader initializers.
It never calls the factory bootstrap or recursively locates provider plugins.
On Windows, paths select explicit files; bare names do not request PATH discovery.
This open searches ordinary imports beside the selected DLL and in System32,
subject to Windows module reuse/redirection. Colocate private native dependencies.
Later provider loads and delay loads follow the provider's own policy. This is
executable inspection, not a sandbox or publisher authentication. Successfully
opened mappings stay pinned until the inspector exits; inspection errors remain
recoverable. See the
[native loading contract](../../docs/abi-v3.md#runtime-and-global-registry).
The [sample plugin](https://github.com/NeoCogi/neocogi-com/blob/v0.3.0/examples/com-plugin/README.md) can be built and
inspected with ordinary Cargo, without the assembly driver.
