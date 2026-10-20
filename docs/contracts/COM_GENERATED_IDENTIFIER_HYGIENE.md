# Generated COM identifiers

Authored names identify public fields, methods, types and descriptor entries.
They must not become implementation-local variables in generated conversions.
The generator must preserve values regardless of a legal author's spelling;
reserving a growing list of names such as `budget` or `raw_0` is not a solution.
Name ownership has three separate rules: support operations name their actual
trait/type/macro owner; private implementation definitions use private lexical or
trait scopes; intentionally public generated members have explicit admission.

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

`PrivateNames` owns each emitting scope. Allocate type/item names with `fresh`
and executable pattern/parameter names with `binding`; allocate buffer/owner
families together so every derived suffix is disjoint from authored syntax.
Seed the allocator with all authored tokens and configured support paths before
introducing generated declarations. Raw identifiers have their ordinary identity.

Freshness against the input is necessary but does not reveal unrelated ambient
constants. Rust can interpret even a mixed-site identifier pattern as such a
constant. Anonymous implementation scopes therefore include private imports of
`::core::convert::identity` under each registered binding name. An imported
function can be shadowed by a local pattern; the ambient constant cannot capture
it. These imports are never called or instantiated and create no runtime state.
They must remain inside the implementation scope and away from authored references.
Removing an apparently unused import can reintroduce the printed-source defect.

Authored field types resolve before any private binding scope exists. A private,
lexically scoped trait binds each original field as an associated type in an impl
for the application type. Wire/storage helpers and decoders consume its canonical
projections. This keeps `Self`, `Self::WIDTH`, recursive `Vec<Self>` and opaque
type macros attached to their application owner without rewriting macro tokens.
Independent wire/decoder expansions own separate anonymous scopes. Persistent
construction imposes no `ComValue` bound and no public native-type projection is
added: public application types may retain private field types.

Generated raw/storage helper declarations allow `private_interfaces` locally:
the lexical field-resolution trait deliberately stays private, while the actual
wire field types normalize to the existing raw representations. This does not
publish native values or add runtime state. A separate consumer crate reads and
mutates those public raw fields through `ComValue::Wire`, then decodes/re-encodes
them; it verifies that no native-field visibility is required. Ordinary copied
field proofs live with this same native-type owner rather than a sibling scope.

The converter qualifies its support types and variants through `::core`, including
`Option`, `Some`, `None`, `Result`, `Ok`, and `Err`. A user's own `Option` is still
a legal application type. COM references continue to use the configured runtime
crate path. No runtime name lookup, ownership layer, or allocation is added.

Wire, payload, alternative and storage types live inside an anonymous constant
scope. Their identifiers are allocated against the complete authored type/member
syntax in that scope. They are not sibling items in the application's module;
the supported wire/storage locators are `<Value as ComValue>::Wire` and
`<Value as ComValue>::Storage`. Handwritten foreign-slot tests obtain an initialized
wire through the codec and mutate its raw fields to test malformed contents.
Former `__ValueValueWire`/payload/storage spellings impose no application restriction.

Every codec operation and proof explicitly selects `ComValue`, including
`FIXED_COST`, `BORROW_WIRE`, `encode_prechecked`, `decode_prechecked`, and associated
wire/storage types. An inherent application operation with the same spelling must
have no effect, whether its type is incompatible or it returns a plausible wrong
value. Enum alternatives named `Wire` or `Storage` remain legal. Signature bytes
are promoted static byte literals, without a generator-named sibling static.

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

Provider/client generic parameters use fresh identifiers against the full trait
syntax, including base, argument/return paths and runtime aliases. The general
Rust macro preserves interfaces inside local blocks; it does not assume every
authored type can be rewritten as `self::Type`. The IDL emitter knows its module
and emits owner-qualified local paths. Private method parameter/return metadata
arrays are promoted inline inside `METHOD_DESCRIPTORS`; no `__NC_METHOD_*`
associated implementation constants are emitted.

## Object constructor generics

Rust implementation type and const parameters retain their authored names,
including defaults, raw identifiers and where clauses. Ordinary constructors add
no generic interface parameter to the implementation's `impl`, so no fresh
interface binder or reserved application name is needed. Cyclic constructors
and their binder-selection logic have been removed; factories connect weak
identities after ordinary allocation instead.

The `local_com` integration suite constructs generic shared/exclusive objects
using `I`, raw/private-looking type and const parameters, and a private-looking
concrete object name. Both ordinary constructor entrypoints for each ownership
mode execute real getter/setter ABI calls and verify normal weak expiration.
This removes the constructor-binder problem; other generator-owned support-name
qualification requirements in this document remain applicable.

Object constructor parameters and table-evaluation locals use the same private
binding policy, including authored const generics. Tables are evaluated directly
in `ComObject::VTABLES`; there is no inherent `COM_VTABLES` forwarding member.
Public constructors and `COM_INTERFACE_COUNT` remain deliberate API members.

## IDL source emission and exported enum macros

The rule also applies before procedural expansion. IDL emits absolute support
paths in method result signatures, catalog optional fields, dense-enum canonical
names and event-binding conversion helpers. Exported integer-enum and flag macros
qualify their `ComValue` support types and conversion traits. Fixing only the
recursive record codec would leave these generated consumers vulnerable to the
same local `Option` declaration.

Integer-enum and flag codecs explicitly select `ComValue` prechecked operations.
Flag defaults select `::core::default::Default`, and enum predicates select
`::core::matches!`. Flag validation embeds the union-of-declared-bits expression;
there is no private associated `KNOWN_BITS` member. A legal authored flag may use
that spelling. Generated standard derives use absolute core trait paths.

`Option`, `Some` and `None` remain legal local IDL type names. `Result` remains a
reserved IDL method-return form; this change does not alter that grammar or
resolver contract. Handwritten Rust interfaces may use locally named types with
fully qualified result signatures. Authored types remain distinct from the
compiler's support language at every affected emission stage.

## Plugin support and trait operations

Support qualification also applies to the value and macro namespaces independently.
Plugin descriptors use `::core::option::Option::Some` for the acquisition adapter
and `::core::env!` for an omitted package version. A provider's `Some` function or
local `env!` macro must not change generated metadata or acquisition behavior.
Explicit version literals keep their existing meaning. This is separate from
the exact hexadecimal Cargo-owner encoding described in [IDL modules](IDL_MODULES.md).

The public `const fn plugin_descriptor()` accessor owns its private descriptor/dependency
statics and C acquisition adapter lexically. Allocate their names against the
authored acquisition, dependency, interface and GUID references, including names
inside constant-path expressions. Moving fixed names into a function without
protecting those references is insufficient. The descriptor retains static
lifetime, provider-local typed acquisition and the same native discovery symbol.
Static metadata composition uses this accessor rather than private sibling names;
the shared-runtime DLL probe follows that rule as well.

The trailing-default persistence policy invokes
`<Self as ::core::default::Default>::default()`. It must not select an inherent
`Self::default`, even one returning the same type with plausible different values.
Derive, copied-record and owned-record entrypoints share that operation. Defaults
apply only to absent trailing fields; malformed or truncated present fields fail.

| Emitting family | Required execution coverage |
| --- | --- |
| Copied/owned values and enums | IDL consumer sentinels and printed value expansions |
| Interface clients, thunks, buffers and metadata | Normal/printed interface consumers, owner-return and callable-name regressions |
| Concrete objects and threading proofs | Normal/printed shared/exclusive construction under support-shaped names |
| IDL helpers, dense enums and flags | Source-emission shadowing and constructor/event consumer fixtures |
| Plugin descriptor and acquisition adapter | Separate `Some` and `env!` shadowing cases, wrong-type and same-type wrong-value controls, explicit version, and actual C acquisition/getter calls |
| Default decoding from derive/copied/owned records | Inherent methods returning Self or an unrelated type; empty/partial/full/malformed/truncated record semantics, normal and printed |

Each new emitting entrypoint must identify its support references and join the
applicable normal/printed consumer coverage. A successful check in another
emitting family does not establish its name-resolution contract. Private syntax
contexts alone do not protect printed source, and injective generated names do
not qualify references to external types, variants, macros or trait operations.

Descriptor method/parameter names are emitted from the normalized authored model
as string literals. Documentation/assertion/panic macros name their core owner;
an application `stringify!`, `matches!`, `concat!`, `assert!` or `panic!` cannot
replace a generated support operation.

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

## Intentional public names and acceptance

Core's `NcComInterfaceCompanion` inventory owns Abi/VTable/VtblFor/Client suffixes;
IDL admission, interface expansion and object path derivation consume it. Core's
`NcComDenseEnumMember` inventory owns dense COUNT/ALL and names-policy NAMES/
canonical_name. IDL reports actual conflicts with EIDL021 at authored source,
including unused templates. NAMES remains legal when the names policy is absent;
ordinary integer enums do not reserve dense helpers. Private implementation names
never enter these public inventories. Handwritten declarations retain the ordinary
Rust restriction against duplicating an intentional public generated member.
IDL's existing enum/flag casing rules already exclude lowercase inherent helper
names such as `raw`, `from_raw` and `canonical_name`; these fail in parsing rather
than requiring another generated-name blacklist.

`name_ownership_tests` checks both emitted text and normal expansion through real
consumers: copied/owned records/enums, trait operations/proofs, public/private
namespaces, local/imported/private-looking types, raw identifiers, inherited and
mutable calls, owner rejection/consumption, buffers, C plugin acquisition and NCB
conversion, including application `Self` and opaque field-type macros. In
addition to explicit wrong-type/wrong-value controls, the harness
extracts actual generated pattern bindings and synthesizes ambient constants for
them, excluding all authored identifiers. A private spelling change cannot evade
that regression. Full printing drops macro syntax contexts; semantic values,
budget boundaries, descriptors and real calls must remain correct.
