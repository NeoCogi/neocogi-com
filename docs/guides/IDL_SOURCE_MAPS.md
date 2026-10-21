# Tracing generated Rust back to IDL

Every Cargo IDL build writes a readable JSON source map beside its Rust bindings:

```text
neocogi_ui_nodes.rs
neocogi_ui_nodes.rs.map
neocogi_ui_nodes.include.rs
neocogi-idl-graph.json
```

Find the actual paths without searching build directories:

```sh
cargo run -p cargo-neocogi --offline -- module generate neocogi.ui.nodes
```

The JSON command result includes `bindings`, `source_map`, `include`, and `dependency_graph`.
A direct Cargo build of the contract crate produces the same files through its
existing `build.rs`. `BuildOutput::source_map` and Cargo's `idl_source_map`
metadata expose the path to build tooling.

## Navigating to generated Rust

Contract crates include the generated **entry file**:

```rust,ignore
include!(concat!(env!("OUT_DIR"), "/neocogi_ui_nodes.include.rs"));
```

The entry loads `neocogi_ui_nodes.rs` through a normal private Rust module and
re-exports its public definitions. Public contract import paths stay the same.
`BuildOutput::include` and Cargo's `idl_include` metadata identify this entry.
The definitions and `.rs.map` remain adjacent, with unchanged map coordinates.

The module entry was introduced when contract attribute expansion caused
rust-analyzer navigation to stop at `include!`. Current IDL generation emits
complete ordinary Rust definitions. Go to Definition reaches the actual emitted
ABI, client, vtable, native value or scalar wrapper definition in that file.
For example, `TextboxAbi` is an emitted interface marker; `LayoutTrack` is a
native Rust enum. Its `Fixed` variant and `extent` field have their own definitions.
The adjacent map connects each definition to the authored IDL identifier.

Existing external contract crates should change `/<stem>.rs` to
`/<stem>.include.rs` in their include statement, rebuild, and reload the
rust-analyzer workspace. New `cargo neocogi module new` crates use the entry
automatically. Build-script and procedural-macro support must be enabled.

Generated declarations and authored members also carry readable documentation:

```rust
/// IDL source: `neocogi/ui/nodes/widgets.ncidl:4:11`.
pub trait Textbox /* generated requirements and methods */ {}
```

The annotation names the originating IDL identifier and its owning logical module.
A source file's normalized `path` is relative to **its owner's** IDL root, which
may belong to another contract package. Each mapping selects one entry from the
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
| Interface method and generated forwarding methods | The authored method identifier |
| Closed generic interface methods | The method in the generic template |
| `ITextChangedEvent::text` | The event interface getter declaration |
| Value wire and codec helpers | The originating value declaration |
| Native enum alternatives and named fields | The originating variant and field |
| Imported re-export | Its individual local imported identifier, including split source parts |

A generated helper without a distinct authored counterpart inherits its nearest
known declaration or member. Positional fields inherit their variant origin.
Origins identify the authored cause; they are not a line-for-line translation
of generated algorithm bodies. Module constants such as `MODULE_ID` have no
individual authored declaration and remain unmapped.

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
reported map paths, resolution to the authored file, unchanged-output reuse, and
location-only edits refreshing the map without changing binding behavior.

The navigation check compiles one independent nodes-contract consumer before
starting the language server. That same Rust source supplies all eleven requests:
Textbox ABI/client/vtable, SliderOptions struct, LayoutTrack/ChildCrossSize enums,
FillStates wrapper/HOVER flag, and the enum/Fixed/extent identifiers in a native
constructor. Each definition must reach emitted nodes Rust and its adjacent map
must match the exact expected authored IDL file and identifier range. Merely
selecting an existing IDL file is insufficient.

From the repository directory, provision the selected toolchain's server and run:

```sh
rustup component add rust-analyzer
python3 scripts/check-idl-navigation.py --rust-analyzer /path/to/rust-analyzer
```

Omit `--rust-analyzer` to use the installed rustup shim. An explicitly selected
standalone server is also supported. The check resolves the active compiler in
repository cwd, respecting caller overrides, and propagates it to the temporary
consumer, server and server-spawned Cargo processes. A temporary directory must
not silently select the user's unrelated default compiler.

The compile-only gate uses the identical source and needs no server:

```sh
python3 scripts/check-idl-navigation.py --compile-only
python3 -m unittest discover -s scripts -p test_idl_navigation.py
```

Compile-only success establishes compilation, not editor navigation. The ordinary
mode is required for the actual server and exact-map acceptance; both paths run
in the existing UI CI job. The focused Python tests cover marker positions,
response forms, Unicode coordinate conversion, range specificity, wrong existing
origins and server EOF. Compiler source-map tests retain their separate purpose.

The script uses cached Cargo dependencies. A caller-selected `CARGO_TARGET_DIR`
is retained, with relative paths interpreted from the invocation directory;
otherwise its scratch target is removed with the temporary consumer. The server
process is shut down on completion or failure. No editor settings are changed.
The original eight-symbol navigation result used rust-analyzer 0.3.3057-standalone;
that historical result predates native enum and direct scalar emission and is not
current acceptance for these eleven requests.
