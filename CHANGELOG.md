# Changelog

All notable changes to this project will be documented in this file.

This project follows semantic versioning once it reaches a public compatibility
contract.

## Unreleased

- Migrated the multi-plugin examples to callable class contracts shared by
  hosts and implementations, removing their parallel class-ID constants and
  one-object interfaces.
- Added `#[com_struct(c_struct)]` for external C-layout fields without nc-com
  traits, with compiler checks for nested FFI layouts. Documented the author's
  plain-value obligations and the limits of field size/alignment fingerprints;
  nc-com needs no dependency on the external type's library.
- Defined explicit runtime resource ceilings of 1,024 retained dynamic modules
  and 64 simultaneous debug messengers, with fatal module activation exhaustion,
  recoverable messenger capacity errors, checked monotonic handles, and
  boundary tests that prevent zero or reused identifiers.
- Expanded high-risk ABI regression coverage with native C offset-buffer and
  aggregate calls, targeted descriptor/table-prefix layout assertions,
  executable ABI-spec snippets, local Markdown-link validation, supported
  little-endian target checks, and an i686 Linux execution row.
- Unified dynamic and direct descriptor registration around one validated
  `PluginRegistrationBatch`, moving graph traversal and record assembly outside
  the transaction lock while preserving atomic state-dependent checks and
  publication.
- Added shared allocation-free const validation for descriptor qualified names
  and versions, rejected invalid interface/class/plugin literals during macro
  expansion, and checked macro-generated ABI counts before `u32` narrowing.
- Rejected named lifetimes, including `'static`, on generated `ComRef`,
  `NcCopyFromBufferOffset`, and `NcCopyToBufferOffset` interface parameters so
  call-scoped raw ABI pointers and buffers cannot escape through safe Rust.
- Initial `nc-com` runtime and macro crates for Rust-to-Rust compatibility
  through a portable C ABI.
- Made `ComInterface` an unsealed unsafe trait and taught `#[com_interface]` to
  generate its audited unsafe implementation, preventing safe handwritten
  interface metadata/pointer conversions from violating `ComPtr` invariants.
- Made generated tagged-enum ABI values opaque to safe Rust while preserving
  their C layout and safe `TryFrom` conversion, with explicit safe unknown-tag
  construction and an unsafe raw-parts escape hatch for FFI adapters.
- Made diagnostic method-signature formatting unsafe for public raw descriptor
  records, with an explicit transitive pointer-validity contract. Generated
  static metadata and descriptors from retained loaded modules remain supported.
- Removed plugin initialization callbacks from the ABI v1 descriptor, macro,
  and inspection schema. Registration is now one blocking transaction, and
  hosts explicitly load dependencies before dependents.
- Replaced the context-bearing runtime C API with versioned runtime, plugin
  registry, type registry, inspection, and debug tables behind six exported
  discovery symbols.
- Removed redundant pre-alpha Rust convenience aliases: raw COM ownership is
  adopted only through `ComPtr::from_raw_owned`, interface queries preserve
  their `NcStatus`, and string slices use the explicitly UTF-8-named `NcString`
  constructors.
