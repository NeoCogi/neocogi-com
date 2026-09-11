# neocogi-inspect

`neocogi-inspect` extracts owned descriptor metadata from an actual `neocogi-com`
plugin shared library.

It detects exactly one `nc_get_factory_plugin_description` or legacy
`nc_get_plugin_descriptor` export and rejects libraries that expose both or
neither. Factory output contains the typed root, direct exact contract
references, and the complete dependency-first canonical module graph, including
declarations and stored IDL source. Legacy output contains classes, interfaces,
and method diagnostics. It does not register the plugin, construct objects, or
recursively load declared neocogi-com plugin dependencies. Opened handles remain
pinned until the short-lived inspector process exits. Normal native loader
behavior, including loading a factory plugin's linked contract DSOs, occurs
before either description symbol is queried.

JSON output uses strict inspection schema 6. Its `activation` enum prevents
factory and legacy-only fields from being mixed. Common fields record producer,
target ABI, core ABI, validation mode, plugin identity, version, and hard plugin
dependencies. Factory contract fingerprints are compatibility authority;
legacy interface `(IID, ABI signature)` pairs remain compatibility authority.
Rendered method text is diagnostic and is not reverse-parsed as a contract.

```sh
neocogi-inspect dump path/to/plugin.so
neocogi-inspect dump path/to/plugin.so --format text
neocogi-inspect dump path/to/plugin.so --output plugin.nccom.json
neocogi-inspect check path/to/plugin.so
neocogi-inspect plan path/to/base.so path/to/dependent.so
neocogi-inspect diff old.nccom.json new.nccom.json
neocogi-inspect diff old.nccom.json new.nccom.json --compat
```

`plan` rejects mixed COM validation modes, conflicting revisions of one logical
contract module, duplicate identities, missing hard dependencies, and cycles.
It then prints the common mode and a deterministic dependency-first activation
order, regardless of activation model.

`diff` strictly parses and semantically validates both JSON artifacts before
comparison. Unknown fields, unsupported schema versions, malformed identities,
duplicate identities, malformed contract graphs, invalid factory roots, and
noncontiguous legacy method indices are rejected. Native description structure
and each signature representation are validated while the library is loaded;
editable JSON diagnostics are not reverse-parsed to reconstruct a contract.

`diff` defaults to exact metadata comparison (`--exact`): it exits with status
`0` only when there are no differences. `--compat` instead performs a
directional old-to-new release check. Legacy mode compares classes and interface
identities. Factory mode compares the root factory and exact module
fingerprints. Both check threading, target/core ABI, validation mode, and hard
plugin dependencies. Compatibility mode exits with status `1` for a breaking or
activation-breaking change.

Schema version and core ABI version are independent; current libraries use
inspection schema 6 and core ABI 7.
