# Generated COM identifiers

Authored names identify public fields, methods, types and descriptor entries.
They must not become implementation-local variables in generated conversions.
The generator must preserve values regardless of a legal author's spelling;
reserving a growing list of names such as `budget` or `raw_0` is not a solution.

## Value conversion

Consider a copied declaration with `first: bool` and `raw_0: u8`. Previously,
destructuring introduced both authored names as locals. Encoding `first` then
introduced another `raw_0`, and encoding the second field read that temporary.
A source value of `73` could silently become `1` through a successful COM call.
A field named `budget` could instead make accepted IDL fail Rust compilation.

The common copied/owned value converter now separates members from bindings:

```rust,ignore
let Data { first: __nc_field_0, raw_0: __nc_field_1 } = *self;
let (__nc_wire_field_0, __nc_storage_field_0) = encode(&__nc_field_0)?;
let (__nc_wire_field_1, __nc_storage_field_1) = encode(&__nc_field_1)?;
DataWire { first: __nc_wire_field_0, raw_0: __nc_wire_field_1 }
```

This is schematic Rust; the actual converter uses each field's `ComValue`
implementation. Records and named enum alternatives use explicit member-to-local
patterns. Tuple fields use the same positional bindings. Ordinary and prechecked
encoding/decoding share this rule. Wire member names, declaration order, tags,
resource accounting and ABI signatures are unchanged.

Generated local identifiers use the private `private_ident` helper with
`Span::mixed_site()`. Authored type/member tokens retain their source context.
Structural separation is required even with hygienic tokens: printing generated
Rust and reparsing it loses syntax context, as happens for core's checked-in
persistence interfaces. A private prefix alone is not a collision guarantee.

The converter qualifies its support types and variants through `::core`, including
`Option`, `Some`, `None`, `Result`, `Ok`, and `Err`. A user's own `Option` is still
a legal application type. COM references continue to use the configured runtime
crate path. No runtime name lookup, ownership layer, or allocation is added.

## Method calls and buffer helpers

The normalized `AbiArg` holds the authored `ident` for documentation and
inspection, plus an ordinal private `binding` for generated execution. A source
parameter called `status`, `result`, `this`, or `__nc_out` therefore never declares
that local in a client or provider body. For example:

```rust,ignore
// Authored provider trait: names and types stay intact.
fn copy(&self, status: NcOutputWriter<'_, Data>) -> Result<u32, NcStatus>;

// Schematic client: its positional Rust argument name is implementation detail.
fn copy(&self, __nc_arg_0: NcOutput<'_, Data>) -> Result<u32, NcStatus> {
    let mut __nc_arg_0 = __nc_arg_0.prepare()?;
    let status = call_provider(/* output storage, buffer bounds and pointer */);
    status_to_result(status)?;
    unsafe { __nc_arg_0.convert_output()?; }
    __nc_arg_0.publish();
    // Decode and return the provider's output.
}
```

The actual client retains its existing result-acquisition order and ownership
handshake. Buffer offset/capacity/pointer/written-count/backing names and owner slots derive from
the private binding, never from source spelling. Rust calls are positional, and
raw C slots follow the authoritative lowered model. Output migration in ABI 17
adds a distinct count slot without deriving any helper from authored spelling. Inspection still reports
`status`, not `__nc_arg_0`.

The parameter-name blacklist and its duplicate raw-name collision scan are gone.
Public method conflicts with lifecycle operations and physical vtable prefixes
still require rejection: those are actual API members, not local variables.
Core owns `NC_COM_HANDLE_METHOD_NAMES` and `is_reserved_com_method`; IDL resolution
and handwritten interface macros use that one policy. The union includes public
receiver helpers on `ComPtr`, `ComRef`, `ComBox` and `WeakComBox`, their implemented
helper traits and native lifecycle operations. Raw identifiers have the same
identity: `r#downgrade` is rejected just like `downgrade`. Associated functions
without receivers do not shadow the generated client and are outside this rule.
The removed boolean `ComPtr::is` probe is no longer reserved; an interface may
define `is()`. Capability selection uses `query` with its complete error status.

An accepted `downgrade() -> Result<WeakComPtr<...>>` previously selected the
handle's lifetime helper before its Deref client, silently bypassing the provider
with the expected result type. Contracts must instead use a distinct name such
as `weak_capability`. IDL reports `EIDL100` at the authored method; macro diagnostics
point at the Rust identifier. Unused templates, inherited projections and closed generics obey the
same rule. Ordinary parameters named `clone`, `downgrade` or `query` remain legal.
No second client, extension-trait lowering or runtime dispatch layer is introduced.

A source audit regression parses actual public handle receiver implementations
and requires their membership in the core inventory, including all configurations.
Explicit checks cover helper trait defaults such as `Clone::clone_from`. Adding a
public helper therefore requires reviewing the callable-contract name policy.
Support paths in interface expansion are qualified through `::core`, while
application types retain their original syntax. No ABI version change is needed.

Core's persistence ABI and facade provider bindings are regenerated from the same
interface expansion. Keeping those printed files synchronized is required; a
macro-only fix would leave a second, stale implementation in the repository.

## IDL source emission and exported enum macros

The rule also applies before procedural expansion. IDL emits absolute support
paths in method result signatures, catalog optional fields, dense-enum canonical
names and event-binding conversion helpers. Exported integer-enum and flag macros
qualify their `ComValue` support types and conversion traits. Fixing only the
recursive record codec would leave these generated consumers vulnerable to the
same local `Option` declaration.

`Option`, `Some` and `None` remain legal local IDL type names. `Result` remains a
reserved IDL method-return form; this change does not alter that grammar or
resolver contract. Handwritten Rust interfaces may use locally named types with
fully qualified result signatures. Authored types remain distinct from the
compiler's support language at every affected emission stage.

## Regression evidence

`crates/com/idl/tests/identifier_hygiene.rs` resolves accepted IDL, compiles the
actual generated bindings in a consumer crate, and executes real COM calls.
Distinct sentinel values detect silent field substitution. Coverage includes
copied/owned records, named/tuple/unit enum alternatives, nested collections,
`raw_0`, `budget`, `storage`, `wire`, `result`, a local `Option`, and handwritten
Rust fields matching the private prefix. Fixed/prechecked conversions and the
shared dynamic budget are exercised. Existing checked-value tests retain the
malformed-bool, invalid-tag and resource-limit checks.

The same consumer tests also execute input/output buffers named `status` and
`result`, scalar arguments named `this` and `object`, old/new private buffer helper
spellings, and successful/rejected exclusive-owner calls. They verify the original
descriptor names and exact owner identity. A handwritten interface shadows core
support names with application items to check that those names remain usable.

`crates/com/macros/src/identifier_hygiene_tests.rs` expands the IDL value/interface
macros, prints the complete output, and runs the same semantic assertions in a
fresh consumer crate. This verifies the path that cannot retain macro hygiene.
The persistence-generation test independently checks the committed built-ins.
The shared shadowing fixture includes dense integer enums, flags, and interface
metadata beside local `Option`/`Some`/`None` types. Constructor/event tests compile
local shadowing beside generated binding helpers and verify `raw_0`/`budget`
sentinels received from the real application channel after posting an event.
Both consumer tests put Cargo artifacts under the workspace `target/message-proof`
directory so ordinary workspace `cargo clean` also removes them.

Public companion-item and constructor-alias collisions are a separate namespace
resolution problem. This contract does not claim to resolve that audit finding.
