# neocogi-inspect

`neocogi-inspect` extracts owned metadata from an actual factory-plugin shared
library without registering or activating it.

The library must export `nc_get_factory_plugin_description`. Native loader
behavior first maps the plugin's fingerprint-qualified contract libraries; the
inspector then validates the plugin description, follows its exact contract
getters, and copies the complete canonical module graph. Opened libraries remain
pinned until the short-lived inspector process exits.

The tool does not call the root factory, construct objects, mutate a runtime
registry, discover plugin dependency paths, or recursively load declared
NeoCogi plugins.

```sh
neocogi-inspect dump path/to/plugin.so
neocogi-inspect dump path/to/plugin.so --format text
neocogi-inspect dump path/to/plugin.so --output plugin.nccom.json
neocogi-inspect check path/to/plugin.so
neocogi-inspect plan path/to/base.so path/to/dependent.so
neocogi-inspect diff old.nccom.json new.nccom.json
neocogi-inspect diff old.nccom.json new.nccom.json --compat
```

JSON output uses strict inspection schema 7. It records producer/target data,
core ABI, validation mode, plugin identity/version/dependencies, and one direct
factory record containing its root identity/threading, direct module references,
and dependency-first canonical module graph. Contract fingerprints are
compatibility authority; stored IDL source and names support diagnostics.

`plan` rejects mixed validation modes, conflicting revisions of a logical
contract module, duplicate plugin/declaration identities, missing hard plugin
dependencies, and cycles. It then prints a deterministic dependency-first load
order.

`diff` strictly parses and semantically validates both schema-7 artifacts.
Unknown fields, unsupported schema versions, malformed identities/fingerprints,
invalid factory roots, and malformed module graphs are rejected. Exact mode
reports every metadata difference. Compatibility mode performs a directional
old-to-new check: target/core/validation changes, root-factory incompatibility,
tightened plugin dependencies, or changed contract fingerprints are breaking.

Inspection schema version and core ABI version are independent. Current output
uses schema 7 and core ABI 7.
