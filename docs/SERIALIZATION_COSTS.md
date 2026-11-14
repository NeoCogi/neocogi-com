# Serialization size and throughput

The native dylib now always contains NCB and JSON, with NCB selected by default.
The earlier NCB-only dylib measurements below are historical comparisons, not a
current dylib build option. Static rlib consumers retain optional JSON.

## Host persistence linkage repair — 2026-11-14

Host `0.3.0-alpha.1` uses the application's registered `nc.serialization` provider for
named NCB in both directions. Its normal Cargo graph contains no serialization
implementation; dynamic applications can load the serialization dylib without
the host adding a second NCB implementation to their executable. Static
applications register `neocogi_com_serialization::plugin_descriptor()` explicitly.
All COM packages advance together to `0.3.0-alpha.1`; the unpublished host-only
patch is superseded. NCB-v1 bytes remain unchanged.

This architecture is covered by static and real min-size DLL persistence tests;
these are correctness/linkage results, not performance qualification. The UI
reference prototype passed its Vulkan 2% performance gate, but one UI CPU
benchmark case regressed 3.51%. That case and the affected workloads must be
rechecked against the final published host build. No final-build throughput or
executable-size comparison is claimed here.

## Fixed-array construction and diagnostic repair — 2026-11-13

Measured against `d07a53c` with the same `array_costs` source, locked dependencies,
Linux x86_64, Intel Core Ultra 7 155H, and rustc 1.102.0-nightly
(`3ef0c22cd`, LLVM 23.1.5). Both sides used SDK source version `0.2.0` before
preparing alpha.5 metadata. NCB and JSON are included on both sides.

| Ordinary min-size artifact | Before | Array repair | Array + diagnostic repair |
| --- | ---: | ---: | ---: |
| Serialization `.so` | 126,360 | 126,360 | 126,504 |
| Static array example | 173,136 | 170,656 | 170,800 |

These are stripped file bytes from ordinary `cargo build --profile min-size`,
including the checkout's rebuilt target std and deployment flags. Array repair
alone left the dylib byte-identical: most affected constructors are instantiated
in their typed caller. The diagnostic correction adds 144 file bytes to each
measured artifact; the combined static example remains 2,336 bytes smaller.
The dylib exports only `nc_get_plugin_descriptor_v2` and needs only `libc.so.6`.
Version-string metadata changes after this comparison are separate from these
same-version implementation measurements.

Release timing uses five interleaved pairs pinned to CPU 0, with 10,000 base
iterations, 20 warm-ups for successful format/value cases, and timing that
includes decoding and destruction. Format cases create fresh owning COM sources
from already encoded input. COM cases measure local checked wire construction
and receiver acquisition, rather than a vtable method dispatch; they use 100x
iterations for scalar/small, 10x for large/nested, and 1x for owned values.

| Case | Before ns | Combined repair ns | Median time change |
| --- | ---: | ---: | ---: |
| ncb-scalar | 248.7 | 256.1 | +3.0% |
| ncb-small | 675.9 | 697.4 | +3.2% |
| ncb-large | 26,587.0 | 26,958.1 | +1.4% |
| ncb-nested | 59,434.3 | 56,942.9 | -4.2% |
| json-scalar | 182.9 | 178.1 | -2.6% |
| json-small | 1,000.1 | 976.7 | -2.3% |
| json-large | 43,929.5 | 43,586.4 | -0.8% |
| json-nested | 97,059.3 | 98,719.7 | +1.7% |
| com-scalar | 3.4 | 3.6 | +5.9% |
| com-small | 9.9 | 5.0 | -49.5% |
| com-large | 754.0 | 280.9 | -62.7% |
| com-nested | 1,762.8 | 1,370.3 | -22.3% |
| com-owned | 243.3 | 236.8 | -2.7% |
| ncb-wrong-length | 149.4 | 138.4 | -7.4% |
| ncb-late-bool | 7,411.1 | 7,638.3 | +3.1% |

Small/large cases contain 8/512 elements; nested cases contain 32 arrays of 32.
Formats use u64 elements; local COM arrays use bool; owned COM conversion uses
eight Strings. Failure cases decode `[bool; 512]` from an admitted zero count or
an invalid final byte. The source lists every input and iteration multiplier.

Frequency, thermals and system load were not fixed. Sample ranges overlap for
many format cases, so the table does not establish a universal speedup or zero
regression. Local COM array conversion improved substantially in this run;
late invalid-boolean rejection increased from 7,411.1 to 7,638.3 ns. An earlier
interleaved batch saw a larger late-error increase, which reinforces the need to
retain raw samples instead of treating one median as a stable platform property.
The repair removes stack amplification and preserves ownership across those
failures; it does not promise arbitrary large by-value results fit a small stack.

Reproduce the instantiating static target and timing driver:

```sh
cargo build --profile min-size -p neocogi-com-serialization --features json --example array_costs
cargo build --profile min-size -p neocogi-com-serialization-dylib
cargo run --release -p neocogi-com-serialization --features json --example array_costs -- 10000
```

Build both revisions with the identical compiler/profile/features and source,
then interleave saved release executables on the same CPU without concurrent
builds. Exact commands, compiler, hashes, sizes, all samples/ranges and retained
fixture sources are under `/tmp/neocogi-array-fix-2026-11-13`. Native one-MiB
regressions cover NCB/JSON aligned-ZST arrays, split COM backing, acquisition and
actual generated safe/raw calls; interpreter tests independently cover cleanup.

## Alpha.2 native plugin — 2026-11-12

The ordinary `cargo build --profile min-size -p neocogi-com-serialization-dylib`
artifact from the alpha.2 sources contains both formats without an opt-in flag.
Measured on Linux x86_64 with rustc 1.101.0-nightly (`c1070d693`) and the checkout's
unchanged min-size/std-rebuild policy:

| Native artifact | File bytes | `.text` bytes |
| --- | ---: | ---: |
| Alpha.2, built-in NCB and JSON | 126,360 | 95,017 |
| Historical alpha.1, NCB and JSON enabled | 126,360 | 95,017 |
| Historical alpha.1, NCB only | 39,504 | 28,537 |

Both formats are now mandatory in the dylib; its default artifact grows by
86,856 bytes relative to the retired NCB-only configuration. It matches the
previous JSON-enabled artifact's measured size. The actual loaded-plugin test
exercises both formats, numeric oracles, grammar and owned diagnostics from this
ordinary build and confirms NCB remains default. No new throughput benchmark
was run for this packaging change; the codec algorithms and prior workload
measurements below are unchanged.

## Historical ABI-v2 alpha.1 direct-width numeric repair — 2026-11-12

Measured on Linux x86_64, Intel Core Ultra 7 155H, with rolling `nightly`
resolving to rustc 1.101.0-nightly (`c1070d693`, LLVM 23.1.1). Baseline and repair
use the same alpha cohort, compiler, lock graph and ordinary `min-size` policy.
The baseline codec is from `91d7e46`; the repair preserves direct requested-width
JSON floating conversion, including external decimal/integer midpoints and `-0`.
Both measured revisions kept NCB unconditional, optional JSON and the same native COM interfaces.

| Artifact (`z`) | Alpha baseline bytes / `.text` | Direct-width cache bytes / `.text` | Change in file bytes |
| --- | ---: | ---: | ---: |
| NCB-only plugin `.so` | 39,504 / 28,537 | 39,504 / 28,537 | 0 |
| NCB + JSON plugin `.so` | 124,712 / 93,423 | 126,360 / 95,017 | +1,648 |
| NCB-only static example | 77,144 / 61,305 | 77,144 / 61,305 | 0 |
| NCB + JSON static example | 151,208 / 117,157 | 152,840 / 118,751 | +1,632 |

The plugin exports only `nc_get_plugin_descriptor_v2` and needs only `libc.so.6`.
These are actual stripped ELF files and GNU `size -A` sections, not type-size or
source-size estimates. No runtime parser/framework dependency was added.

The cache keeps finite f64 and directly parsed optional f32 results for floating
syntax. Exact integers convert directly; parsed signed zero retains its sign.
This needs no retained source text or per-number allocation. It does parse each
fractional/exponent token at both widths, including f64-only documents.

A correct deferred-conversion prototype was also tested through the owning COM
regressions. It kept one owned numeric-token String with checked offsets and NUL
separators, cached admitted f64, and parsed f32 only on acquisition. Its JSON
plugin was **128,408 bytes** (`.text` 97,050), and its static example was 154,904
bytes (`.text` 120,784). That was larger than the cache and required more numeric
allocations; the prototype was removed rather than shipping both paths.

### Timing and allocation tradeoff

Five interleaved runs of each saved baseline/cache/token executable were pinned
to CPU 0. Each timed numeric workload used 30 warm-ups and 2,000 fresh owning COM
decodes, including construction and destruction. The existing static example
used 50 warm-ups and 5,000 complete round trips. Values below are medians in ns;
frequency/thermals were not fixed, so small differences are not speedup claims.

| Workload | Baseline | Cache (selected) | Deferred tokens |
| --- | ---: | ---: | ---: |
| Mixed example JSON round trip | 63,488 | 63,936 | 65,131 |
| Mixed example NCB round trip | 22,463 | 22,540 | 22,641 |
| 256 f32 midpoint decimals | 31,663 | 35,804 | 40,070 |
| 256 f64-only exponent numbers | 34,394 | 37,515 | 37,214 |
| 64 long strings | 24,263 | 24,348 | 24,200 |
| 32 long fractional tokens | 12,648 | 15,276 | 13,337 |

The cache increased median mixed JSON time by about 0.7% in this local run.
Baseline samples ranged from 63,181 to 63,513 ns and cache samples from 63,724 to
65,476 ns; these ranges do not establish unchanged performance. Numeric-heavy
decode pays about 9–21% for retaining correct results
at both widths. Deferred tokens improve long-token f64 acquisition but are slower
for f32 and larger, with more allocations. The cache is the smaller, simpler
choice with better f32 throughput; it does not claim zero cost for correctness.
The old midpoint f32 workload produced incorrect bits, so its timing is a
before-repair cost baseline, not an equivalent correct implementation.

The exact input arrays were generated by joining repeated tokens with commas:

| Target type | Token repeated | Count / input bytes |
| --- | --- | ---: |
| `Vec<f32>` | `1.0000000596046448` | 256 / 4,865 |
| `Vec<f64>` | `1.2345678901234567e100` | 256 / 5,889 |
| `Vec<String>` | JSON string containing `text-0123456789` repeated 16 times | 64 / 15,553 |
| `Vec<f64>` | `0.` followed by `1234567890` repeated 30 times | 32 / 9,697 |

A temporary static driver used the ordinary owning JSON factory and
`Format::deserialize` for those types. A forwarding System allocator counted one
additional decode outside timing; counting was disabled during timing. Counts
include reallocations and cumulative requested bytes, not peak live memory.

| Decode | Baseline/cache allocations / requested bytes | Deferred tokens allocations / requested bytes |
| --- | ---: | ---: |
| f32 array | 12 / 26,232 | 22 / 44,670 |
| f64 array | 12 / 27,256 | 22 / 49,786 |
| Strings | 138 / 39,032 | 138 / 39,056 |
| Long tokens | 9 / 3,960 | 16 / 42,338 |

The selected cache left all measured allocation counts and requested bytes
unchanged. The native source retains concrete syntax values rather than caller
pointers; a regression mutates/drops input before generated typed construction.

Reproduce artifact sizes and the mixed workload using the ordinary commands in
[Reproduce](#reproduce), keeping the compiler/flags/inputs identical on both
revisions and copying artifacts outside targets before `cargo clean`. The local
comparison retains exact temporary driver source/manifest, prototype source,
commands, Cargo artifact records, raw timings, allocator counts, compiler details
and all saved binaries under `/tmp/neocogi-alpha-fix`. The numeric probe is a
local workload comparison, not another maintained runtime API or benchmark crate.

## Historical pre-alpha measurements

Measured on 2026-11-11, Linux x86_64, Intel Core Ultra 7 155H. The complete
platform-style workflow uses `nightly-2026-09-29` (Rust 1.101.0-nightly,
`c1070d693`, LLVM 23.1.1) and rebuilt target std. NCB is unconditional; JSON is our
own parser/writer and adds no runtime parser/framework dependency. YAML is removed.
These measurements describe the pre-alpha ABI v1 checkout; they are historical
size/throughput evidence rather than qualification of the ABI v2 alpha candidate.

## Complete deployment configuration

`cargo build --profile min-size` uses the ordinary
[workspace configuration](../.cargo/config.toml). It rebuilds target std,
uses immediate abort, fat LTO, one codegen unit, symbol/path stripping,
`force-unwind-tables=no`, `location-detail=none` and `fmt-debug=none`. Host build
scripts and procedural macros keep their ordinary formatting and diagnostics.
The checkout follows `nightly` and requests `rust-src`; no build alias, task
runner or extra configuration file selects the deployment settings.
The measurements below retain the exact compiler revision used for that run.

The earlier 428,928-byte JSON file was built with precompiled std and retained
panic/backtrace symbolization. Those measurements did not use this complete
workflow and are superseded by the table below. The deployed library now needs
only `libc.so.6`; it exports only `nc_get_plugin_descriptor_v2`.

## Artifact sizes

| Optimization | Optional JSON | Shared library bytes | Shared `.text` bytes | Static example bytes | Example `.text` bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| `z` (default) | Off | 39,488 | 28,537 | 77,144 | 61,305 |
| `z` (default) | On | 124,712 | 93,423 | 151,208 | 117,162 |
| `s` | Off | 39,912 | 28,842 | 79,664 | 63,828 |
| `s` | On | 133,744 | 102,442 | 161,104 | 127,047 |

File bytes come from the actual stripped ELF artifacts; `.text` comes from GNU
`size -A`. File size includes data, vtables, relocation tables and alignment.
The static executable is [format_costs.rs](../crates/serialization/examples/format_costs.rs).
It keeps every enabled factory live. These are local measurements of this
implementation, not cross-platform estimates. The default `z` policy minimizes
both measured artifacts; `s` offers a measured throughput tradeoff.

## Throughput

Median of three runs with 50 warm-up round trips and 2,000 timed round trips.
The value contains 32 records, Unicode labels, a map and an optional integer.
Timing includes new COM sessions, allocation, encoding, output copy, parsing/input
copying, typed construction and destruction. It uses `black_box`; it does not
time only an already allocated primitive codec. CPU scheduling/frequency were not
fixed, so the figures are indicative.

| Optimization | Example configuration | Format | Encoded bytes | Round trips/second | Nanoseconds/round trip |
| --- | --- | --- | ---: | ---: | ---: |
| `z` | NCB only | NCB | 954 | 39,670 | 25,208 |
| `z` | NCB + JSON | NCB | 954 | 38,383 | 26,053 |
| `z` | NCB + JSON | JSON | 2,127 | 14,823 | 67,461 |
| `s` | NCB only | NCB | 954 | 52,491 | 19,051 |
| `s` | NCB + JSON | NCB | 954 | 52,206 | 19,155 |
| `s` | NCB + JSON | JSON | 2,127 | 18,559 | 53,881 |

JSON floats use functional scientific formatting, so target Debug stripping
cannot corrupt saved values. The native integration test loads this exact
nightly deployment build into a stable-built host, round-trips float-bearing
records, checks copied error details, and compares byte-mutated JSON documents
with an independent parser in the unpublished test executable. That reference
parser is not a dependency of either deployed crate.

## Reproduce

Install `nightly` with the `rust-src` component before these builds.

```sh
cargo build --profile min-size -p neocogi-com-serialization-dylib
cargo build --profile min-size -p neocogi-com-serialization --example format_costs
cargo build --profile min-size -p neocogi-com-serialization --features json --example format_costs
```

Artifacts reside under `target/min-size`; an explicit `--target` adds that target's
subdirectory. The static example resides in its `examples` subdirectory. Run it
three times with argument `2000` and take the per-format median.
Add `--config 'profile.min-size.opt-level="s"'` to compare the alternative size
policy. Cargo's ordinary `--message-format=json` reports exact artifact paths.
`size -A PATH` reports the ELF sections.

The development checkout follows `nightly`. SDK qualification uses independent
source copies without checkout Cargo/toolchain settings and keeps large build
outputs on the checkout filesystem. Downstream consumers retain Rust 1.85 support.
No custom allocator or panic implementation is needed.

Artifact copies, export lists, structured measurements and raw timing CSVs for
this local run are retained at `/tmp/neocogi-serialization-final-costs`. They sit
outside Cargo targets and survive the required pre/post-commit cleans.
