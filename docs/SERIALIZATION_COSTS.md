# Serialization size and throughput

Measured on 2026-11-11, Linux x86_64, Intel Core Ultra 7 155H. The complete
platform-style workflow uses `nightly-2026-09-29` (Rust 1.101.0-nightly,
`c1070d693`, LLVM 23.1.1) and rebuilt target std. NCB is unconditional; JSON is our
own parser/writer and adds no runtime parser/framework dependency. YAML is removed.

## Complete deployment configuration

`cargo build --profile min-size` uses the ordinary
[workspace configuration](../.cargo/config.toml). It rebuilds target std,
uses immediate abort, fat LTO, one codegen unit, symbol/path stripping,
`force-unwind-tables=no`, `location-detail=none` and `fmt-debug=none`. Host build
scripts and procedural macros keep their ordinary formatting and diagnostics.
The checkout pins the required nightly and `rust-src`; no build alias, task
runner or extra configuration file selects the deployment settings.

The earlier 428,928-byte JSON file was built with precompiled std and retained
panic/backtrace symbolization. Those measurements did not use this complete
workflow and are superseded by the table below. The deployed library now needs
only `libc.so.6`; it exports only `nc_get_plugin_descriptor_v1`.

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

Install `nightly-2026-09-29` with the `rust-src` component before these builds.

```sh
cargo build --profile min-size -p neocogi-com-serialization-dylib
cargo build --profile min-size -p neocogi-com-serialization-dylib --features json
cargo build --profile min-size -p neocogi-com-serialization --example format_costs
cargo build --profile min-size -p neocogi-com-serialization --features json --example format_costs
```

Artifacts reside under `target/min-size`; an explicit `--target` adds that target's
subdirectory. The static example resides in its `examples` subdirectory. Run it
three times with argument `2000` and take the per-format median.
Add `--config 'profile.min-size.opt-level="s"'` to compare the alternative size
policy. Cargo's ordinary `--message-format=json` reports exact artifact paths.
`size -A PATH` reports the ELF sections.

The development checkout uses pinned nightly. SDK qualification uses independent
source copies without checkout Cargo/toolchain settings and keeps large build
outputs on the checkout filesystem. Downstream consumers retain Rust 1.85 support.
No custom allocator or panic implementation is needed.

Artifact copies, export lists, structured measurements and raw timing CSVs for
this local run are retained at `/tmp/neocogi-serialization-final-costs`. They sit
outside Cargo targets and survive the required pre/post-commit cleans.
