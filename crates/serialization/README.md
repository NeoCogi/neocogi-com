# COM serialization

Reusable `rlib` implementation of the COM serialization formats. NCB is always
included and is the default; there is no feature that disables it. `json` adds our custom lightweight JSON codec to the same plugin. The separate
`neocogi-com-serialization-dylib` package is the thin native entry point and
always enables both NCB and JSON in its shared library.

```rust
use neocogi_com_sdk::serialization::Limits;
let format = neocogi_com_serialization::ncb::format(Limits::default());
let bytes = format.serialize(&42u32).unwrap();
assert_eq!(format.deserialize::<u32>(&bytes).unwrap(), 42);
```

`factory()` acquires the static selection service directly. `plugin_descriptor()`
registers it with an explicitly owned host runtime. Both paths use the same COM
interfaces as native loading. `default_format()` always returns NCB;
`format("json")` returns absence when its feature is disabled.
Names cross COM using `NcCopyFromBufferOffset::new(0, name.as_bytes())`.

Default features provide the static plugin and owned sessions using `alloc`.
The implementation needs no `std` or runtime API feature. Without default
features, NCB fixed-buffer functions and measurement remain allocation-free and
work with `no_std`. `alloc` adds owned NCB objects; `std` optionally enables SDK
standard-library support; `plugin` supplies static registration. Optional JSON
also works with allocation only.

Failures expose `ISerializationDiagnostics` through COM. Convenience operations
return an owned error with a message, logical field/index path and source location
when available. JSON syntax errors include exact byte offsets and line/column; NCB reports
its exact failure cursor. The JSON parser/writer has no runtime dependencies
beyond the SDK and Rust core/alloc libraries.
Text floats must be finite and preserve their bits, including negative zero.

See the [normative format contract](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0-alpha.2/docs/contracts/SERIALIZATION_FORMATS.md)
for value grammar, limits, ownership and diagnostic semantics.

Build the deployed shared library with
`cargo build --profile min-size -p neocogi-com-serialization-dylib`.
The ordinary Cargo profile uses nightly, rebuilt target std, immediate
abort and the platform's size flags. Both formats are built in; NCB stays default.
Static rlib consumers can omit its `json` feature for NCB alone.
Artifacts live under `target/min-size` unless a target is explicitly selected.
Independent SDK consumers and qualification retain stable compiler support.

`cargo build --profile min-size -p neocogi-com-serialization --features json --example format_costs`
builds `format_costs`, whose
round trips include allocations, copying, parsing and typed construction. The
[deployment configuration](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0-alpha.2/.cargo/config.toml)
keeps full host procedural-macro diagnostics while stripping target Debug and
panic location data. JSON numbers use functional numeric formatting, so Debug
stripping cannot alter saved values.
