# COM serialization formats

Serialization formats are ordinary COM providers. The canonical interfaces live
in [formats.ncidl](../../crates/core/schemas/neocogi/com/serialization/formats.ncidl).
Core owns their checked-in ABI, clients and scoped adapters; the SDK facade owns
ordinary provider thunks and the runtime-selectable `Format` convenience API.
The bootstrap freshness test derives these halves from the single NCIDL source.
The reusable `neocogi-com-serialization` rlib owns every codec. The separate
`neocogi-com-serialization-dylib` crate only exports its native descriptor entry
point, following the platform implementation/shim split. No host implementation
is linked merely to use a codec. Host persistence acquires the named `ncb` format
from `nc.serialization` in its own catalog. The application explicitly registers
the static descriptor or loads the dylib; the host embeds no implementation and
never selects a default, registers or loads an implicit fallback. Root and format
capabilities stay retained for the operation and callbacks run outside catalog
locks. Missing providers return `NC_E_NOT_FOUND`; selection failures propagate.

This supersedes the 0.1.0 constraint that value traits accept concrete NCB cursors.
The retired generic Rust serializer/access traits are not restored. The new
format boundary consists of explicit C-compatible COM calls.

## Contracts and ownership

| Interface | Responsibility | Lifetime |
| --- | --- | --- |
| `ISerializationPlugin` | Select `ncb` or `json`; NCB is always default | Shared static or dynamic plugin root |
| `ISerializationDiagnostics` | Read the first failure and copy its message/path | Borrowed projection of a failed owning session |
| `ISerializationFormat` | Create bounded document sessions | Shared COM factory; suitable for a plugin root |
| `ISerializer` | Accept typed scalars, named fields and container operations; finish one document | Exclusive `ComBox` for owned implementations |
| `IDeserializer` | Supply typed scalars and named/ordered children; require complete input | Exclusive `ComBox` owning its input |
| `ISerializationOutput` | Copy committed output into fresh caller storage | Borrowed projection of a completed serializer |

`ComSerialize` and `ComDeserialize` remain local, statically typed value traits.
Their arguments are `serialization::Serializer` and `Deserializer`, which hold
borrowed COM capabilities. Each operation invokes the selected provider's vtable.
The application constructs its own Rust values; format providers never receive
Rust trait objects, `String`, `Vec`, native field offsets or private object memory.

Text arguments use the ordinary readable byte-buffer ABI. They must have offset
zero and valid UTF-8. Names and text are copied or consumed before the call
returns. Source strings use a length-admission call followed by copying into a
fresh output buffer; the client checks the exact reported and written lengths
and validates UTF-8 before constructing its own `String`.

The format factory acquires borrowed input before returning an owning source.
NCB owns its input bytes; JSON owns a parsed syntax tree, with no retained
caller pointers. A syntax failure produces a failed source with queryable details,
rather than discarding the explanation during factory creation.
The NCB objects retain concrete codec state in ordinary fields. Their generated
COM object gate admits each mutable call before borrowing that state. No provider
lock or interior mutability replaces the exclusive document owner.

## Session and container states

An owned session transitions from **Active** to either **Finished** or
**Failed**. `finish` requires exactly one complete root and balanced scopes;
a source also rejects trailing input. Successful finish freezes output and
rejects further format operations, including a second finish. A format or schema
failure retains the first provider diagnostic and rejects all later operations. Discard the
session on failure. ABI validation rejects malformed arguments before provider
entry; such rejected calls do not consume the document. Reentrant calls return
`NC_E_BUSY` before a second Rust mutable reference is formed.

Serialization names the next record field before writing its value. Every
sequence emits its declared element count. Maps alternate typed keys and values
for the declared pair count. A present option has one child and absence has none.
A case has zero or one typed payload; tuple or record payloads form their own
child container. Matching begin/end kinds and complete child counts are required.

Deserialization opens a typed value scope before constructing it and closes that
scope only after its selected representation is consumed. Record constructors
request declared field names; a provider may look up those names independently
of encoded order. False means genuine absence, never malformed or truncated
input. Completion rejects unknown or unconsumed fields. Required fields fail on
absence. Explicit default-backed named records retain their `Default` members
for missing fields; NCB can omit only a trailing suffix because it is positional.

Payload cases are probed by both declaration ordinal and authored name. NCB uses
the ordinal; named formats can use the name. Numeric IDL enums and flags retain
their declared raw integer representation. Decoded maps reject duplicate keys.
A format that cannot represent a value must return an error; successful encoding
must preserve that value's semantics.

Fixed-array decoding requires the admitted sequence count to equal the target
array length before constructing any element. Existing provider/facade count
failures retain precedence over the typed length check. Construction uses the
actual element layout and one initialized-prefix count, with no per-element
completion tag or allocator requirement. On error or ordinary Rust unwinding,
only completed elements are destroyed; a child owns its own unfinished fields.
Once all elements exist, ordinary array ownership takes over. A later sequence
end, value leave or document finish error drops that completed value normally.

This bounds avoidable construction metadata, not total native stack usage.
Returned arrays, containing records, element temporaries and alignment can still
require substantial space, and Rust does not guarantee elimination of all return
copies. Applications needing heap collection storage can use existing `Vec<T>`
or `Box<[T]>` decoding. Logical document limits are not a native-stack allocator.

JSON length and duplicate-key failures identify the enclosing container, without
inventing a next child index. Selected or missing record fields retain their
field name. A failure while entering a child identifies the attempted child;
failed entry does not advance that cursor. The first diagnostic stays immutable.

## Limits and NCB

Defaults bound a complete document to 16 MiB, each container to 65,536 entries,
root-to-leaf construction to 64 values, and total values to 1,000,000. Every
provider owns format-specific syntax/framing limits. The typed facade also
bounds foreign collection counts and string-allocation requests before reserving
storage. Initial sequence reservation remains capped at 4 KiB.

NCB retains its independent format version 1 and existing bytes: `NCB\x01`,
little-endian scalars, u32 counts/ordinals, declaration-order record fields and
8-byte pointer-sized integers. Names are available to format providers but do
not enter NCB output. No COM pointer, native padding or ABI signature is saved.
The NCB protocol stack has a fixed ceiling of 128 open values in addition to
configured limits, keeping fixed-buffer operations allocation-free.

`neocogi_com_serialization::ncb::{to_vec,to_slice,from_slice}` and measurement are NCB conveniences
using the same COM protocol. Borrowed input and fixed output use nonretainable
scoped COM objects: AddRef, owning QueryInterface and weak references are rejected.
A required `UnsafeCell` and busy flag admit one mutable foreign call at a time;
no reference to the provider survives releasing that gate. These capabilities
are valid only while their enclosing Rust scope is retained. They cannot be
returned or retained as ordinary owning format objects.

NCB conveniences preserve detailed local byte-offset errors. COM format methods
return `NcStatus`; no Rust error representation crosses an artifact boundary.
`ncb::NcbSerializer::create` and `ncb::NcbDeserializer::create` construct ordinary
owning objects. The strict native source constructor returns a native NCB error
on failed admission. `ncb::format` selects NCB; `Format::new` selects any compatible
COM factory supplied by an application or acquired from a plugin.

## Feature selection and JSON grammar

NCB is unconditional, including with `--no-default-features`; there is no `ncb`
feature to disable. The rlib defaults enable allocation and its static `plugin_descriptor`; neither
NCB nor optional JSON requires standard-library support. The rlib's `json` feature
adds the custom JSON codec for static users. The dylib always links both NCB and
JSON, including with `--no-default-features`; it has no format feature switches.
Unknown names (and disabled JSON in static builds) return `NC_E_NOINTERFACE`.
`default_format` always selects NCB.
Fixed-buffer NCB also works in a `no_std` consumer without default features.

JSON uses our own bounded UTF-8 parser and compact writer. It has no external
runtime parser/serialization dependencies. The private finite syntax enum stores
only JSON syntax; application payloads and dynamic Rust value traits never enter
it. Parsing is single-pass after UTF-8 admission. Unescaped strings and output
text runs are copied in bulk. Error coordinates/path strings are constructed only
on failure. Rust's standard numeric conversion/formatting preserves finite floats.

| Typed value | JSON representation |
| --- | --- |
| Unit, absent option | `null` (distinguished by the requested typed operation) |
| Boolean, integer, finite float, string | Ordinary scalar syntax |
| Record | Object/mapping using authored field names |
| Tuple or sequence | Array/sequence |
| Present option | `{"$some": value}` |
| Enum case | `{"$case": "Name"}` plus optional `"$value"` payload |
| Map with arbitrary typed keys | `{"$map": [[key, value], ...]}` |

Nested options preserve `None`, `Some(None)` and `Some(Some(value))` distinctly.
Non-finite floats are rejected by the text formats. Signed/unsigned integer
endpoints and finite float bits, including negative zero/subnormals, round-trip.
JSON validates number grammar before converting: no leading zeros, incomplete
fractions/exponents or non-finite values. Integral syntax must fit i64/u64;
floating syntax must fit finite f64. An accepted number requested as f32 or f64
converts directly to that width with correctly rounded conversion, without an
intermediate f64 rounding for f32. `-0`, `-0.0` and `-0e0` preserve floating sign;
integer `-0` still acquires as zero. Correctly rounded underflow to signed zero
is permitted. Requested-width overflow fails that typed acquisition with
`NC_E_OUT_OF_RANGE`; finite f64 syntax beyond f32 range remains a valid source.
Integer acquisition requires integral syntax even for spellings such as `1.0`.
These external-input rules supplement the own-writer bit-round-trip guarantee.
The JSON source retains concrete numeric values for both floating widths and no
caller input borrow or per-number allocation; finite f64 and optional f32 are
parsed directly while the validated token is available. The parser preserves
signed versus unsigned integral spelling so typed floating acquisition can
retain integral negative zero. String parsing validates every escape and
surrogate pair, rejects lone surrogates/raw ASCII controls, and preserves Unicode.
Duplicate object keys, trailing commas and trailing data are rejected. Decoded
maps also reject duplicate typed keys.

Text syntax has a separate maximum depth of 128 and bounded overhead for explicit
map/case framing: syntax depth is at most `min(3*depth+4,128)`, total syntax nodes
at most `3*values+16`, and syntax containers at most `max(elements,2)`. Writers
check the same bounds before emission. Logical budgets remain exact and retain
encoder/decoder symmetry. Output bytes, including escaping/framing, are bounded
before each append. NCB remains the direct byte codec; it builds no syntax tree.

## Failure diagnostics

Owning supplied sessions expose `ISerializationDiagnostics`. Read-only calls
return the first failure's status, message length, complete copied UTF-8 message,
logical path, byte offset and line/column when available. Short output storage
publishes no prefix. Queries never replace the original failure. Active or
successfully finished sessions report that no failure exists.

NCB supplies its exact failure cursor and native diagnostic. JSON syntax
errors include an exact zero-based byte offset and one-based line/column. Columns
count Unicode scalars, and lines count LF separators. Typed text failures report the expected type and a field/index path,
for example `$.items[0].count`. A semantic path does not fabricate a source
coordinate after parsing. Unavailable offsets use `u64::MAX`, and unavailable
line/column use zero in COM. The native NCB helpers retain their existing error
representation and allocation-free diagnostics.

`Format::serialize` and `deserialize` return a caller-owned `serialization::Error`
with `status`, `message`, `path` and optional coordinates. They copy the diagnostic
before releasing the failed COM session. Low-level creation/traversal methods
still return `NcStatus`; callers retaining a session can query details themselves.
Providers lacking a usable diagnostic projection retain their original status
with a generic fallback. Diagnostic copies are bounded to 64 KiB each in the
convenience API. Successful traversal does no diagnostic formatting; field keys
move from the syntax map into path components, and a path string is built only
on failure.

## Usage and object persistence

```rust,ignore
use neocogi_com::serialization::{Format, Limits};
let format = neocogi_com_serialization::ncb::format(Limits::default());
let bytes = format.serialize(&state)?;
let restored: State = format.deserialize(&bytes)?;
```

The format is selected at runtime; the same value implementations work with a
different factory. Callers can also use the interfaces and typed facades directly
for document sessions and output-copy control.

Handwritten value codecs now accept `Serializer`/`Deserializer` and return
`NcStatus`. Named records pair `begin_record`/`end_record` and select each `field`
before its value; tuples retain ordered sequence operations. A decoder is called
inside an existing value scope: use `read` for each child and delegate directly
for a transparent scalar wrapper. Anonymous aggregate enum payloads open one
additional `value` scope. These rules keep encoder and decoder budgets symmetric.

Application value formatting and live object reconstruction remain separate.
`IPersistable` exports provider-owned state bytes and children; the recipe's
factory owns those bytes' schema and interpretation. The existing runtime
snapshot envelope remains NCB v1 by default. The format interfaces add no global
registry, filesystem access, loader policy or automatic object-memory capture.

Discovery uses ABI v2 for `0.2.0`. These interfaces are documented in the
[ABI v2 specification](../abi-v2.md); NCB persistence retains format version 1.

## Verification

The [size and throughput report](../SERIALIZATION_COSTS.md) records the deployed
shared library, static feature choices and historical configurable-shim results,
including profile choices and reproduction commands. The current dylib always
contains both formats.

The binary persistence suite retains golden bytes, exact resource boundaries,
allocation-free measurement, malformed input and partial-construction cleanup.
Format tests exercise independently implemented named JSON record/case providers and
a separately built native plugin, using the same typed values through COM.
An independent JSON fixture also verifies that implementations need not share
the supplied codecs' Rust storage or parser. Production JSON tests cover all
value shapes, numeric endpoints, finite floating bits, limits and diagnostics. Scoped tests reject retention, invalid raw scalar storage
and reentrant mutation. Core and facade reduced-feature builds remain required.
