# neocogi-inspect

Reads an actual plugin's `nc_get_factory_plugin_description_v1` export without
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

JSON uses strict schema 9, plugin descriptor ABI 1, and core object ABI 7.
Unknown fields and unsupported schema versions are rejected. Module/declaration
inventories are a build-time IDL-tooling concern.

`plan` rejects duplicate plugin identities, missing hard dependencies, and
cycles, then prints deterministic provider-first order. Disconnected contract
revisions may coexist. `diff --compat` reports changed factory signatures,
identities, target/ABI versions, weakened threading, and added hard dependencies
as breaking; plugin version text is diagnostic.

Inspection maps trusted native code and can execute platform loader initializers.
It never calls the factory bootstrap or recursively locates provider plugins.
The [sample plugin](../../examples/com-plugin/README.md) can be built and
inspected with ordinary Cargo, without the assembly driver.
