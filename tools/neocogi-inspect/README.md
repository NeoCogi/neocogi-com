# neocogi-inspect

Reads an actual plugin's `nc_get_plugin_descriptor` export without
registering or activating its factory. It copies plugin identity/version, hard
dependencies, the native ABI version, and full factory IID/signature and
threading capability. There are no contract libraries or module graphs to load.
Opened libraries remain pinned until the short-lived process exits.

This checkout inspects WIP ABI v6. Build it from the same SDK revision
as the provider with `cargo build -p neocogi-inspect`.

Version `0.4.0` inspects providers using
[provisional ABI v3/core generation 21](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/docs/abi-v3.md#abi-identities-and-versions).
Install the matching inspector to follow the
[standalone walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/examples/getting-started/README.md).

```sh
cargo install neocogi-inspect --version '=0.4.0' --locked
```

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

Usage errors exit with status 2. Inspection, artifact, and output I/O failures
exit with status 1. Exact differences and incompatible comparisons write their
report and exit with status 1. Other successful operations exit with status 0.
Errors go to stderr when the stream is writable.

Inspection JSON uses strict schema 11. This format requires inspection through
the current native header family. Live inspection checks the
[current native ABI](../../docs/abi.md#abi-identities-and-versions).
Saved artifacts record one `abi_version` field that `diff` compares.
Unknown fields and unsupported schema versions are rejected. The artifact records
the plugin and its root factory. Source-level method and value definitions belong
to [local IDL metadata](https://docs.rs/neocogi-com-sdk/0.4.0/neocogi_com_sdk/idl_metadata/index.html).

`plan` rejects duplicate plugin identities, missing hard dependencies, and
cycles. Successful plans list providers before dependents in deterministic order.
A cycle error lists all plugin identities blocked by cycles, including plugins
that depend on them. JSON plans preserve source pathnames as Unicode strings.
After inspection and dependency validation, a source pathname outside Unicode
returns an error before stdout or the output file is written. Native `check`,
`dump`, and `plan --format text` remain available for those source names. Output
destination paths remain native paths.
Disconnected contract revisions may coexist. `diff --compat` reports changed
factory signatures, identities, target/ABI versions, and added hard dependencies
as breaking. For matching factory identities and signatures, weakened threading
is breaking and strengthened threading is additive. When the signature changes,
threading differences provide diagnostic context for that breaking change. Plugin
version text is diagnostic. Text differences use canonical JSON spellings and
escaped strings, preserving concrete values in ordinary and min-size builds alike.
Native inspection failures also preserve the loader phase, path, platform or
validation detail and any ABI status in both profiles.

Inspection maps trusted native code and can execute platform loader initializers.
The selected native image must export `nc_get_plugin_descriptor`. Windows export
forwarders and Apple reexports count. A getter available only through an
ordinary native import fails in `FindSymbol`.
Inspection never calls the factory bootstrap or recursively locates provider plugins.
On Windows, paths select explicit files; bare names do not request PATH discovery.
This open searches ordinary imports beside the selected DLL and in System32,
subject to Windows module reuse/redirection. Colocate private native dependencies.
Export forward targets follow the native Windows loader policy. Later provider
loads and delay loads follow the provider's own policy. This is
executable inspection, not a sandbox or publisher authentication. Successfully
opened mappings stay pinned until the inspector exits; inspection errors remain
recoverable. See the
[native loading contract](../../docs/abi.md#runtime-and-global-registry).
The [sample plugin](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/examples/com-plugin/README.md) can be built and
inspected with ordinary Cargo, without the assembly driver.
