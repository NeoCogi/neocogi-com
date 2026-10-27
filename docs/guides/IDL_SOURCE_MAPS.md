# Tracing generated Rust back to IDL

Each consumer's shared-source IDL build writes readable Rust and a JSON source
map into its own `OUT_DIR`:

```text
module_<logical-module-UUID>.rs
module_<logical-module-UUID>.rs.map
module_<logical-module-UUID>.include.rs
neocogi-idl-graph.json
```

The UUID is the existing logical module identity. Bounded lowercase filenames
keep case/separator variants distinct on every host. Obtain exact paths from
`BuildOutput::bindings`, `include` and `source_map`; do not reconstruct a filename
from the dotted name. Repository build scripts expose their include paths through
`IDL_<MODULE_NAME>` environment entries in Cargo's build-script JSON:

```sh
cargo check -p neocogi-example-com-host --message-format=json
```

The `build-script-executed` record for `neocogi-example-com-host` contains
`IDL_EXAMPLE_BINDINGS` in its `env` field. This is the consumer's generated entry;
its Rust definitions and `.rs.map` are adjacent. Independently compiled consumers
have their own output directories and maps from the same shared IDL sources.

## Navigating to generated Rust

The consumer includes its returned entry path:

```rust,ignore
include!(env!("IDL_EXAMPLE_BINDINGS"));
```

The entry loads its generated definitions through a normal private Rust module
and re-exports their public surface. It remains relocatable with its adjacent
binding file, and source-map coordinates describe that actual formatted file.
`cargo::Module` builds for separately distributed source dependencies expose the
same `BuildOutput` fields and use their per-package conventional filenames.

The module entry was introduced when contract attribute expansion caused
rust-analyzer navigation to stop at `include!`. Current IDL generation emits
complete ordinary Rust definitions. Go to Definition reaches the actual emitted
ABI, client, vtable, native value or scalar wrapper definition in that file.
For example, `ExampleObjectAbi` is an emitted interface marker; `ExtentReport` is a
native Rust enum. Its `Current` variant and `extent` field have their own definitions.
The adjacent map connects each definition to the authored IDL identifier.

Enable rust-analyzer build-script and procedural-macro support.

Generated declarations and authored members also carry readable documentation:

```rust
/// IDL source: `neocogi/idl_example/objects.ncidl:12:11`.
pub trait ExampleObject /* generated requirements and methods */ {}
```

The annotation names the originating IDL identifier and its owning logical module.
A source file's normalized `path` is relative to **its owner's** IDL root, which
may be the shared source tree or a separately distributed library. Each mapping selects one entry from the
map's `sources` table. Cargo fills that file's `location`, relative to the map's
directory, so tools resolve an authored file as:

```text
file = map.sources[mapping.original.source]
authored file = map directory / file.location
```

Cargo canonicalizes the map directory and the source file before computing the
relative location. Different filesystem roots, such as different Windows drives,
require an absolute location. Pure generation leaves locations unset; the caller
supplies filesystem context using each file's logical owner and path.

Dependency build metadata carries physical source roots separately from semantic
source records. Different checkouts of identical contracts do not conflict merely
because their paths differ. Locations never enter ABI fingerprints. Bundled IDL,
including `neocogi.lang::Option`, is copied from the resolved source into
`OUT_DIR/idl-sources/`; maps point to that exact copy instead of guessing a file
under the consumer's IDL root. These files and metadata are private build artifacts.

Cargo first generates the entire requested local batch and encodes the source
graph without filesystem writes. It then finishes all source locations and
serializes every map before replacing any main `.rs`, `.rs.map`, `.include.rs`
or graph file. Bundled source copies can already change during location finishing;
they require actual files for canonicalization. Main publication is a later
fallible write phase. This ordering preserves old main files on map-finishing
failure without promising rollback on final IO failure or interruption. Generated
text and logical maps are retained across the batch; logical maps are released
as their finished serialized bytes are prepared.

## What is mapped

The compiler parses the final pretty-printed Rust to measure ranges. Formatting,
documentation, long signatures and generated helper bodies are therefore included
in the coordinates. A complete declaration range covers its generated fragment;
nested syntax entries refine that origin for structs, enums, unions, traits,
methods, functions, fields, variants and helpers.

| Generated Rust | IDL origin |
| --- | --- |
| Copied struct or enum | Its declaration identifier |
| Named field or enum alternative | Its authored field or variant identifier |
| Integer enum or bit flag wrapper and associated constants | Its declaration and each authored member |
| Interface method, client method, raw vtable slot and complete provider thunk body | The supplying authored method identifier |
| Closed generic interface methods | The method in the generic template |
| Closed generic roots, clients and wire helpers | The bound template's defining identifier, including imported and memberless templates |
| Value `IDL_TYPE` and interface `IDL_INTERFACE`, including initializer bodies | The owning value/interface declaration identifier |
| `ITextChangedEvent::text` | The event interface getter declaration |
| Value wire and codec helpers | The originating value declaration |
| Native enum alternatives and named fields | The originating variant and field |
| Imported re-export | Its individual local imported identifier, including split source parts |

A generated helper without a distinct authored counterpart inherits its nearest
known declaration or member. Positional fields inherit their variant origin.
Origins identify the authored cause; they are not a line-for-line translation
of generated algorithm bodies. Module constants such as `MODULE_ID` have no
individual authored declaration and remain unmapped.

Every method-specific representation consumes the origin already retained in its
shared method plan. Inherited calls use the supplying base's method; imported
closed generic methods use their template method rather than the application
expression or enclosing interface. Synthetic helpers without a specific method
cause continue using declaration-level fallback.

Ordinary generation completes each declaration with its authoring metadata before
mapping its formatted fragment once. There is no separate unmapped metadata tail.
For closed generics, the existing bound template key selects the defining
identifier; the complete application expression remains the semantic use-site
span for diagnostics. Authored members retain their more specific origins.
The allocation-free core bootstrap deliberately omits facade authoring metadata.

The map describes the final emitted `.rs` file, including complete Rust contract
definitions and generated client methods. It does not describe a separately
expanded Rust dump. Application-owned object/serialization macros remain separate
from direct IDL emission. Native values' associated checked wire and storage
definitions are implementation details, not public tag/payload companions.
Provenance documentation accompanies the actual emitted declarations and methods,
including the checked-in persistence bootstrap.

## JSON format and lookup

This is NeoCogi's readable range-map schema **2** (a build format, independent of runtime/descriptor ABI v1), not JavaScript Source Map v3.
Each mapping has a `kind`, generated Rust `name`, `generated` range and `original`
source-file index/range. The top-level `sources` array owns module/path/location
records once for all ranges. The ranges use one-based lines and one-based **Unicode scalar**
columns, matching the parser. A tab counts as one character. Starts are included;
ends are excluded. Columns are neither UTF-8 byte offsets nor UTF-16 code units.

An illustrative entry:

```json
{
  "kind": "field",
  "name": "Size::width",
  "generated": {
    "start": { "line": 28, "column": 5 },
    "end": { "line": 30, "column": 19 }
  },
  "original": {
    "source": 0,
    "range": {
      "start": { "line": 3, "column": 15 },
      "end": { "line": 3, "column": 20 }
    }
  }
}
```

Here `sources[0]` is a single record such as
`{"module":"example.widgets","path":"example/widgets/types.ncidl","location":"../../contract/idl/example/widgets/types.ncidl"}`.
Every range in that file reuses the same index. An imported template uses its
provider module and provider location; it does not inherit the consumer's root.

Tools should use the most specific enclosing range. The compiler exposes that
operation directly:

```rust,no_run
use neocogi_idl::{RustSourceMap, SourceMapPosition};
# fn example(json: &str) -> Result<(), serde_json::Error> {
let map: RustSourceMap = serde_json::from_str(json)?;
assert_eq!(map.schema, 2);
if let Some(entry) = map.lookup(SourceMapPosition { line: 28, column: 10 }) {
    let file = &map.sources[entry.original.source];
    println!("{}:{}:{}", file.path,
        entry.original.range.start.line, entry.original.range.start.column);
}
# Ok(())
# }
```

`generate_rust_with_source_map` returns `GeneratedRust { source, source_map }`.
`generate::generate_rust_with_paths_and_source_map` additionally accepts explicit
Rust import locations. Existing string-returning generation functions remain
available and include the provenance annotations; callers needing a map should
use the paired result and persist both artifacts together.

Maps and annotations do not enter declaration IDs, ABI fingerprints, runtime
registration, or plugin loading. Moving IDL declarations or renaming source parts
updates provenance while preserving the same contract ABI. Cargo writes unchanged
outputs only when their contents differ. These are build/debugging artifacts and
are not automatically copied into application deployment packages.

The normal module entry enables navigation to generated **Rust** without an
editor plugin. The `.rs.map` describes the next step, from Rust to **IDL**;
rustc and rust-analyzer do not read this custom sidecar or automatically redirect
diagnostics/navigation to IDL. The provenance comments and lookup API expose that
origin explicitly.

## Validation

Tests cover Unicode columns, multipart ordering, formatting, scalar wrappers and
constants, native fields/variants, closed generic interfaces, method bodies, imports, EOF
boundaries and JSON round-tripping. A real Cargo module-generation test verifies
reported map paths, resolution to the authored file, exact bundled-source copies
and generated metadata/COM calls. Cargo unit tests separately check complete
recursive output preservation on generation/graph rejection, main-file preservation
on a later map-finishing error, and exact bytes/known old timestamps on an unchanged
successful repeat. The shared-consumer test checks actual Cargo cache reuse and
source-membership invalidation, which differ from avoiding unchanged file writes.

Run the maintained source-map integration suite from the workspace root:

```sh
cargo test -p neocogi-idl --test source_maps
```

The suite checks emitted mappings and authored locations while compiling
independent consumers and executing their generated COM calls. Editor-specific
navigation integrations can consume the same maps and annotations.
