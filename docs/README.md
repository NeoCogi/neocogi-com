# Current COM contracts

The current binary contract is [Core ABI v17](abi-v17.md). Its generation is
defined by `NC_COM_CORE_ABI_VERSION` in
[core metadata](../core/src/metadata/contract.rs). The
[documentation test](../idl/tests/documentation.rs) checks that this entrypoint
selects the specification matching that constant. Older `abi-v*.md` files record
previous drafts and migration history; use the current specification for new code.

The core generation, plugin/runtime discovery versions, and IDL fingerprint
encoding are separate contracts. [Runtime ABI v1](runtime-abi-v1.md)
replaces module identities with plugin provenance and consolidates lookup,
enumeration and acquisition in one plugin-operation table. This runtime guide
also supersedes the older factory activation names in the core drafts. All runtime tables
and plugin descriptor discovery remain v1; roles must use the same checkout. IDL's
canonical module encoding uses v9. Neither number identifies the current object
ABI. The [IDL language reference](../../../docs/contracts/IDL_MODULES.md) owns the fingerprint
and versioning rules, including generic parameters and enum assignments.

## Authoritative guides

- [COM overview and executed factory example](../README.md): contract authoring,
  implementations, plugin export and bootstrap.
- [IDL language and generation](../../../docs/contracts/IDL_MODULES.md): accepted syntax,
  identities, revisions, value shapes and Cargo integration.
- [Component responsibilities](../../../docs/architecture/COM_DESIGN.md): build tools,
  providers, runtime and application ownership.
- [Object/table representation](../../../docs/contracts/COM_INTERFACE_STORAGE.md): handle
  layout and the reason diamond inheritance does not require object headers.
- [Exclusive widget ownership](../../../docs/contracts/EXCLUSIVE_WIDGET_OWNERSHIP.md):
  `ComBox`, weak observations, synchronous Drop and parent-controlled removal.
- [Lifetime errors](../../../docs/contracts/COM_LIFETIME_ERRORS.md): dropped objects versus
  closed services and error propagation from live receivers.
- [Prepared output completion](../../../docs/contracts/COM_OUTPUT_COMPLETION.md): fallible
  preparation and the ownership boundary of consuming ABI calls.
- [Recursive values](../../../docs/contracts/RECURSIVE_VALUE_CONTRACT.md): checked wire
  acquisition, recursion, budgets and metadata.
- [Local NCB persistence](../core/README.md#why-persistence-is-ncb-only): concrete
  bounded cursors, ordered construction, codec history and independent-plugin checks.
- [Event queues](../api/EVENTS.md) and [UI event delivery](../../ui/runtime/docs/EVENTS.md):
  concrete queue implementations, weak origins and deferred application behavior.
- [Independent-build compatibility](compatibility.md): executed compiler/flag
  matrix, sanitizers and platform limits.

Package overviews link here instead of selecting another ABI draft. Changes to a
contract must update its responsible guide and the examples exercising it. The
Button walkthrough executes the exact source included in UI rustdoc; the IDL
documentation test compiles the marked guide blocks and executes the COM bootstrap.
Compile-only snippets cannot prove an ownership or delivery policy correct.
