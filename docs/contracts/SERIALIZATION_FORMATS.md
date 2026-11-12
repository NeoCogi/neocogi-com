# COM serialization formats

Serialization formats are ordinary COM providers. The canonical interfaces live
in [formats.ncidl](../../crates/core/schemas/neocogi/com/serialization/formats.ncidl).
Core owns their checked-in ABI, clients and scoped adapters; the facade owns
ordinary object storage and the supplied NCB implementation. The IDL bootstrap
freshness test derives both halves from this source, including owned object
storage from `serialization/objects.rs.in`. No compiler or host runtime is linked
just to use a format.

This supersedes the 0.1.0 constraint that value traits accept concrete NCB cursors.
The retired generic Rust serializer/access traits are not restored. The new
format boundary consists of explicit C-compatible COM calls.

## Contracts and ownership

| Interface | Responsibility | Lifetime |
| --- | --- | --- |
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

The format factory copies borrowed input before returning an owning source.
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

`binary::to_vec`, `to_slice`, `from_slice` and measurement are NCB conveniences
using the same COM protocol. Borrowed input and fixed output use nonretainable
scoped COM objects: AddRef, owning QueryInterface and weak references are rejected.
A required `UnsafeCell` and busy flag admit one mutable foreign call at a time;
no reference to the provider survives releasing that gate. These capabilities
are valid only while their enclosing Rust scope is retained. They cannot be
returned or retained as ordinary owning format objects.

NCB conveniences preserve detailed local byte-offset errors. COM format methods
return `NcStatus`; no Rust error representation crosses an artifact boundary.
`NcbSerializer::create` and `NcbDeserializer::create` construct ordinary owning
objects. `Format::ncb` selects the NCB factory; `Format::new` selects any compatible
COM factory supplied by an application or acquired from a plugin.

## Usage and object persistence

```rust,ignore
use neocogi_com::serialization::{Format, Limits};
let format = Format::ncb(Limits::default());
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

Discovery remains ABI v1 on `0.2.0-wip`. These interfaces are documented in the
[ABI v2 development specification](../abi-v2.md); selecting ABI v2 remains a
0.2.0 release step.

## Verification

The binary persistence suite retains golden bytes, exact resource boundaries,
allocation-free measurement, malformed input and partial-construction cleanup.
Format tests exercise independently implemented named JSON record/case providers and
a separately built native plugin, using the same typed values through COM.
The JSON provider is a test fixture for its selected schema, not a supplied
production JSON codec. Scoped tests reject retention, invalid raw scalar storage
and reentrant mutation. Core and facade reduced-feature builds remain required.
