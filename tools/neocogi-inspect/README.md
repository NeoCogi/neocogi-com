# neocogi-inspect

`neocogi-inspect` extracts owned descriptor metadata from an actual `neocogi-com`
plugin shared library.

It resolves `nc_get_plugin_descriptor`, validates the descriptor graph, and
prints plugin, dependency, class, interface, and method metadata as JSON or
text. It does not register the plugin, construct plugin objects, or recursively
load declared neocogi-com dependencies. Opened handles remain pinned until the
short-lived inspector process exits. Normal native loader behavior can still
run before the descriptor symbol is queried.

Class output records the declared `thread_local` or `thread_safe` construction
capability. JSON output uses the strict, versioned `neocogi-com.inspection` schema
and records its schema version, producer version, target triple, pointer width,
endianness, core ABI version, interface identities/signatures, and derived method
diagnostics. It intentionally omits structured parameter, return, and aggregate
definitions. Text output labels the same derived diagnostic text as
`typed_signature`; signature text is not compatibility authority.

```sh
neocogi-inspect dump path/to/plugin.so
neocogi-inspect dump path/to/plugin.so --format text
neocogi-inspect dump path/to/plugin.so --output plugin.nccom.json
neocogi-inspect check path/to/plugin.so
neocogi-inspect plan path/to/base.so path/to/dependent.so
neocogi-inspect diff old.nccom.json new.nccom.json
neocogi-inspect diff old.nccom.json new.nccom.json --compat
```

`plan` rejects duplicate plugin IDs, missing hard dependencies, and cycles, then
prints a deterministic dependency-first activation order. It inspects every
supplied artifact before reporting per-artifact failures.

`diff` strictly parses and semantically validates both JSON artifacts before
comparison. Unknown fields, unsupported schema versions, malformed identities,
duplicate class/interface identities, and noncontiguous method indices are
rejected. The descriptor's typed metadata and advertised signature are validated
while the library is loaded; editable JSON diagnostics are not reverse-parsed
to reconstruct that contract.

`diff` defaults to exact metadata comparison (`--exact`): it exits with status
`0` only when there are no differences. `--compat` instead performs a
directional old-to-new release check. It uses each interface's `(IID, ABI signature)`
identity, requires old classes and interfaces to remain available, and checks
constructor, threading, target/core ABI, and hard-dependency compatibility. New
classes/interfaces and stronger threading are compatible; version labels and
rendered method diagnostics do not affect the result. Compatibility mode exits
with status `1` only for a breaking or activation-breaking change.

Inspection schema 4 records each interface's explicit `base` IID/signature and its
locally declared methods. Inherited methods are found through the base entry
in the same class. Dumps validate that base contracts exist and are acyclic;
diffs report base changes explicitly. Schema version and core ABI version are
independent; current libraries use core ABI 6.
