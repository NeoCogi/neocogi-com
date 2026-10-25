# NeoCogi IDL modules

Contracts are build-time definitions. Authoritative `.ncidl` sources live under
each subsystem's `schemas/` tree. Consumers generate Rust layouts, typed wrappers
and fixed signatures in their own crates, or import bindings exposed by an
existing ordinary library when exchanging its concrete Rust helper types.
Logical modules require no dedicated Cargo package. No contract shared library,
native description archive, getter, runtime graph or contract registration step
is produced.

`interface` is the only callable contract declaration. It describes public
methods, threading, required projections and source metadata. Rust owns
concrete storage (`#[com_object]`) and plugin acquisition (`#[plugin]`). Ordinary
typed interface methods handle parameterized construction.

Methods begin with ordinary `&self` or `&mut self` receivers. Explicit receiver
lifetimes (including `'static` and `'_`), typed-self syntax and method lifetime
parameters are rejected during parsing, even in unused generic declarations.

A suffix such as `interface Button: IWidget + IActions` gives one physical vtable
prefix (the first entry) and required sibling projections (later entries). All
are queryable on the same object; requirements are transitive and acyclic.
Handles carry an object address and a selected static interface table. Interfaces
add no fields to each object allocation; inherited projections are deduplicated at
compile time. The [object/table storage guide](COM_INTERFACE_STORAGE.md) explains
this settled representation and the earlier reverted experiments. Interfaces
default to local threading. `#[threading(shared)]` requires implementations to
be `Send + Sync`; a derived interface cannot weaken a required shared promise.

See [component design](../architecture/COM_DESIGN.md) for ownership and historical rationale,
and the [plain-Cargo sample](../../examples/com-plugin/README.md) for dynamic and
monolithic builds.

## Generated source provenance

Cargo writes a `.rs.map` JSON sidecar beside each generated Rust file and inserts
readable IDL-origin documentation on declarations and authored members. The map
covers final formatted ranges, including closed generic and message helpers, while
ABI identity remains independent of paths and locations. Build scripts obtain
the exact map path from `BuildOutput::source_map`; the repository exposes include
paths in Cargo’s build-script JSON. The
[source mapping guide](../guides/IDL_SOURCE_MAPS.md) defines the format and lookup rules.

IDL source parsing and complete Rust generation use one explicit eight-MiB
syntax-worker stack policy on every host. Deep valid
output-buffer types therefore do not depend on a build-script main thread's
platform default, including MSVC's one-MiB executable stack. Single-file and
multipart parsers return owned module models and diagnostics with stable source
coordinates. Generation owns lowering, formatting, mapping and syntax destruction
inside one worker per module or bootstrap call. Rust module paths retain validated
identifier spellings and create token syntax only where needed. Original parser
spans exist only during syntax admission; syntax trees and process-local spans
never cross workers. Failure to start an IDL parser
worker returns `ParseError::Worker`; generated-Rust worker failures return
`GenerateError`. Neither publishes partial output. The public `resolve_modules`
operation first admits the ordinary bundled `neocogi.lang` source. Its
`ModuleGraphError::BundledSource(ParseError)` retains parser/worker failures,
including their operating-system cause, rather than asserting that tested
source guarantees an available worker. The subsequent semantic algorithm starts
no syntax worker and retains `ResolutionError` diagnostics through
`ModuleGraphError::Resolution`. Cargo preserves this typed graph failure in
`BuildError::Resolve` before generation, artifact writes or success metadata.
Empty input still fails before bundled admission; the reserved-library-name
check follows bundled parsing. Internal worker panics retain their payloads.

## Closed generic values and interfaces

Structs, enums, and interfaces can declare type parameters. Each use must supply all arguments:

<!-- checked-example: generic-stream -->
```ncidl
#[version(1)] struct Snapshot<T> { value: T }
#[version(1)] interface IValue<T> {
    fn get(&self) -> Result<Snapshot<T>>;
}
#[version(1)] interface Counter: IValue<u32> {}
#[version(1)] interface Counters {
    fn create(&self) -> Result<ComBox<Counter>>;
}
```

A template is a source-level contract, not an open COM vtable. The compiler
substitutes each closed use into an ordinary concrete declaration, then applies
the same field ownership, finite-layout, copied-position, inheritance, threading,
and value-transport checks used for declarations without parameters. Thus
a generic method taking `&T` cannot instantiate T as a scalar; `IValue<u32>`
with scalar-valued `get`/`set` methods is valid. Methods do not declare their
own type parameters. Concrete classes and plugin acquisition live in Rust,
while construction interfaces follow the ordinary interface rules. Type argument bounds
are checked at instantiation; Rust trait bounds, lifetimes, const arguments,
and specialization rules are outside this grammar.

A closed declaration's identity derives from its template identity and ordered argument
identities. Its signature includes the template's exact module revision and
argument ABI signatures. The consuming module and backend Rust import paths are
excluded, so separately built consumers can negotiate the same specialization.
Generated concrete Rust names contain a deterministic identity suffix. They are
compiler names; metadata retains the original template and arguments. Recursive
instantiation is bounded by depth and total instance limits.

Template parameter names are local binders. Fingerprints encode their zero-based
positions, so `Pair<T> { value: T }` and `Pair<Item> { value: Item }` have the same
signature when authored under the same module/name/version. This applies through
nested arguments, recursive values, enum payloads, direct interface bases,
and method inputs/results. Parameter count and order remain significant: exchanging which
parameter a field uses changes the template signature, even if a particular use
supplies the same concrete type for both arguments. Qualified free types, field
names, variant names, and method parameter names are not generic binders and
retain their existing contract meaning. Authored binder names and source spans
remain available for diagnostics; hashing does not rewrite the source model.

Semantic binding runs once for every authored declaration, including unused
templates. Bare parameters bind to their zero-based positions; other names bind
to exact declarations or language intrinsics. A fully qualified reference to the
current module needs no `extern`; an explicit self-`extern` remains invalid.
Missing names, missing external dependencies and invalid stored borrows retain
the same source-aware diagnostics whether the graph contains generics or not.
Specialization, capability analysis, encoding and Rust generation consume those
bindings instead of interpreting source spelling again.

A closed generic use carries both its canonical template/argument identity and
its concrete supplying Rust projection. Inherited interface methods and their
value metadata follow that supplying declaration, including nested values. A projection
already materialized in the requesting module may be used locally; an unrelated
module with the same canonical specialization never supplies metadata or creates
a new dependency. Rust paths and compiler-local declaration keys do not enter ABI
signatures.

Callable interfaces, including unused templates with explicit IDs, reject the
nil UUID with `EIDL074`. This admission matches typed plugin export. Nominal
value IDs have a separate policy and may still explicitly use nil.

### Canonical signatures

Unit fields and payloads are checked copied data, including through fixed arrays
and buffers; their wire byte must be zero. Root `Result<()>` still has no output
slot, and bare unit inputs remain excluded. The
[recursive value contract](RECURSIVE_VALUE_CONTRACT.md) defines conversion and budgets.

Templates and concrete declarations share the explicit binary declaration
encoder under `neocogi-idl:module-abi:v13`. The former separate template string
encoding and its Debug-formatted policy values are removed. Both paths encode
parameter positions, resolved free identities, member order, authored enum
assignment presence, and declaration policies. Regenerate bindings and rebuild
all contracts/consumers after adopting this encoding; there is no compatibility
adapter for the old signatures. Module-wide revision checking is retained.
Changing implicit enum values to explicit assignments still requires a declaration
version increment; see [contract changes and versions](#contract-changes-and-versions).

Base reference slots encode a discriminator before either the bound
parameter ordinal or the free module/declaration IDs and recursively encoded
arguments. Only a bare identifier binds: `B` in `interface Forward<B>: B {}`
is positional, while `library::B` remains a free declaration even with a binder
named `B`. Bound parameters cannot accept arguments. Ordinary type and named-only
reference positions use one lexical classifier; hashing does not resolve a bound
name as a declaration or mutate the authored syntax model.

Every recursive type expression has a disjoint tag. A bound
parameter uses tag 16 followed by its u32 ordinal; fresh output uses tag 14
followed by its element type. Base slots retain the explicit bound/free grammar
described above.

Interfaces have no implementation-role discriminator in their canonical encoding.
Canonical encoding v13 contains ordinary value and callable-interface contracts;
IDL has no event-reference slots. Rebuild affected contracts, providers and hosts
together, including built-in persistence bindings. Nominal IDs, core ABI v1,
source-graph schema and the host-owned runtime registry are separate contracts;
runtime discovery remains ABI v1. Closed generic interfaces work as object
contracts and plugin roots; open templates have no callable ABI.

Generic values can nest and recurse through explicit sequences:

<!-- checked-example: generic-values -->
```ncidl
#[version(1)]
struct Node<T> { value: T, children: Vec<Node<T>> }

#[version(1)]
enum Change<T>: u8 { Set(T), Clear }

#[version(1)]
struct Handle<I> { object: ComPtr<I> }

#[version(1)]
interface ITreeInput {
    fn accept(&mut self, change: &Change<Node<String>>) -> Result<()>;
    fn inspect(&self, handle: &Handle<IUnknown>) -> Result<()>;
}
```

`Handle<IUnknown>` is valid: the interface argument becomes the referent of an
explicit owner. Replacing `ComPtr<I>` with `I` in that template makes this use
invalid. Likewise, `Node<bool>` is finite because its children are a sequence;
`struct Loop<T> { child: Option<Loop<T>> }` fails when instantiated. Generic values can appear in ordinary interface methods and aggregate fields.

Closed definitions have consumer-local Rust names and consumer-independent
contract identities. Inspection reports canonical value identities and the
actual Rust projection available in the generating crate. The expansion is
bounded to 64 specialization levels and 4,096 instances; recursively inventing
new argument types fails with an IDL diagnostic.

## Source language

The IDL deliberately resembles Rust without inheriting Rust's unstable native
ABI. The generator owns every binary representation. This complete two-module
example uses a small texture contract as its dependency:

<!-- checked-example: language-render -->
```ncidl
// File: example/render.ncidl
#[version(1)]
interface ITexture {}
```

<!-- checked-example: language-widgets -->
```ncidl
// IDL root: contracts/idl
// File:     contracts/idl/example/widgets.ncidl
// Module:   example.widgets

extern example.render;

use example.render::ITexture;

#[version(1)]
struct WidgetOptions {
    enabled: bool,
    width: i32,
    height: i32,
}

#[version(1)]
interface IImageConsumer {
    fn set_image(&self, image: &ITexture) -> Result<()>;
}

#[version(1)]
enum FlowDirection: u8 {
    Horizontal,
    Vertical,
}

#[version(1)]
bitenum WindowOptions: u16 {
    NONE = 0,
    RESIZABLE = 1,
    TITLED = 2,
}

#[version(1)]
interface IWidget {
    fn enabled(&self) -> Result<bool>;
    fn set_enabled(&mut self, enabled: bool) -> Result<()>;
}

#[version(1)]
interface Button: IWidget {
    fn label(&self) -> Result<ComPtr<INcString>>;
    fn set_label(&mut self, label: &[u8]) -> Result<()>;
    fn set_image(&mut self, image: &ITexture) -> Result<()>;
}

#[version(1)]
interface UiFactory {
    fn create_button(
        &self,
        label: &[u8],
        options: WidgetOptions,
    ) -> Result<ComBox<Button>>;
}

#[version(1)]
interface UiRuntime {
    fn widgets(&self) -> Result<ComPtr<UiFactory>>;
}

#[version(3)]
interface UiPluginFactory {
    fn create_runtime(&self) -> Result<ComPtr<UiRuntime>>;
}
```

These blocks illustrate the language; their small example contracts are independent
of any external application framework. The generic examples above and both modules here
are extracted from this document, resolved, generated and compiled by
`cargo test -p neocogi-idl --test documentation`. The
[COM README](../COM.md#idl) adds an executable
implementation and factory bootstrap checked by the same test.

The optional suffix after an enum name selects its discriminant or tag type:
`u8`, `u16`, `u32`, `i8`, `i16`, or `i32`. Omitting it defaults to `u32`.
Representation is IDL syntax (`enum Name: u8`), not a Rust `#[repr]` attribute.
An enum whose variants carry no payload generates a transparent fixed-width
integer newtype and preserves unknown values. An enum with tuple or named
payloads generates a tagged sum representation. This keeps ordinary protocol
enumerations compact while making payload layout explicit.

Fieldless enum and flag wrappers, constants, checked live codecs, nominal
signatures and eligible raw-integer persistence are emitted directly by the typed
Rust backend. Generated output contains ordinary structs and impls, with no
intermediate scalar macro grammar or formatter text rewriting. Source maps visit
those ordinary Rust nodes: scalar wrappers are structs and their authored values
are associated constants, each retaining its exact IDL origin. Payload alternatives
remain Rust enum variants. The compact integer representation does not permit
skipping live enum-domain or known-bit validation.

Enum classification follows the declaration, before generic specialization. If
any variant has tuple or named fields (including a `()` payload), the whole declaration
is a tagged sum. Every variant then receives its zero-based declaration ordinal;
explicit `= value` assignments are rejected on payload and unit variants alike
with `EIDL073`. This also applies to unused templates. Only wholly fieldless
integer enums may assign numeric values, including sparse and signed values.
`enum Choice<T> { Empty, Value(T) }` is valid;
`enum Choice<T> { Empty = 0, Value(T) }` is not. These tags are defined by IDL,
independently of Rust's native enum representation or payload size. The typed
backend emits the already-resolved ordinals directly; no Rust declaration
frontend can override them.

A bit enum requires an explicit `u8`, `u16`, or `u32` representation and an
explicit nonnegative value for every flag. It generates a transparent value
with private storage, declared constants, `raw()`/`from_raw()`, validity and
containment queries, and typed bitwise operators. Any combination containing
only declared bits is valid; unknown bits remain representable so component
boundaries can reject them without losing the received ABI value.

IDL identifiers are already their generated Rust API names; generation never
silently rewrites them. The parser enforces the corresponding conventions:

- `struct`, `enum`, `bitenum`, and `interface` names use
  `UpperCamelCase`.
- Ordinary enum variants use `UpperCamelCase` just like Rust enum variants.
- Bit-enum flags use `SCREAMING_SNAKE_CASE` because they are combinable typed
  constants rather than mutually exclusive variants.
- Struct fields, named enum payload fields, methods, and parameters use
  `snake_case`.

Public generated names share the declaration/import namespace. The resolver
checks interface companions and codec helper names before emission; collisions
report `EIDL021`. Dense enum variants also cannot duplicate their generated public
COUNT/ALL members, or NAMES when `#[names(...)]` is enabled; ordinary enums retain
those spellings. Admission and emission share core's typed public-name inventories.
Private code-generation helper names are isolated, rather than reserved from IDL.
IDL rejects `#[events(...)]` as an unsupported attribute. Application-defined
observer interfaces use the ordinary method and value rules.

### Concrete messages and streams

Declare ordinary data and use it in an explicitly declared delivery interface or typed stream:

```rust
#[version(1)]
/// Complete editor value after one committed user edit.
struct TextChanged {
    /// Owned text captured at the edit.
    text: String,
    /// Scalar-aligned UTF-8 byte cursor within that text.
    cursor: u64,
}

#[version(1)]
interface Editor {
    /// Ordinary application command, independent from its constructor receiver.
    fn set_text(&mut self, text: &[u8]) -> Result<()>;
}
```

Records and enums use one recursive value model. Fields can contain scalars,
other records/enums, `String`, fixed arrays, `Option<T>`, owned sequence fields `Vec<T>`
(owned as `Vec<T>`), and explicit object capabilities (`ComPtr` or `WeakComPtr`).
There is no `#[message]` annotation or implicit per-record receiver interface.

Every field uses its checked wire representation. A nested `bool` is a `u8`
validated as 0 or 1; enum decoding matches the tag before reading the selected
payload. This applies to copied slots and method buffers as well as borrowed
values. A caller creates the borrowed call guard with `NcValueRef::new(&value)?`;
the callee uses `to_owned()` to acquire independent data.

Inline cycles, including `Option<Self>`, fail resolution. Explicit sequences
permit finite recursive trees. Named references in the resolved type graph do
not expand indefinitely. Unsupported copied method positions fail at the IDL
source; ownership-bearing values use borrowed named-value parameters. `Option<T>` is
an ordinary enum and is copied when its payload is copied. Recursively copied arrays support direct
arguments/results and buffer elements.

Conversions share cumulative byte, value-count and depth budgets. Defaults are
8 MiB, one million values, and 64 levels. Foreign callers must still provide
readable initialized active raw storage; validation cannot prove an arbitrary
pointer is live. See [the complete contract](RECURSIVE_VALUE_CONTRACT.md).

The [host/provider example](../../examples/com-plugin/README.md) declares its own
observer interface and notification values. Its host owns the delivery queue.

The removed `event: Message;` and `#[observer]` syntax is rejected. There are no
per-variant receiver setters or named-output relays. Application-private messages,
callbacks, and queues stay inside the consuming binary. Message wire validation
is shared by ordinary borrowed-message methods and typed stream delivery.

Optional values use the ordinary bundled `Option<T>` enum. Copied payloads allow
copied arguments/results; owned payloads require borrowing the complete enum.
An optional object input is `&Option<ComPtr<Interface>>`, not `Option<&Interface>`.
The former has an ordinary enum tag and retains a capability only when decoded;
there is no IDL-specific nullable pointer lowering. See [generic values and
collection ownership](IDL_GENERIC_VALUES.md) for the complete contract.
Regenerate contracts and rebuild consumers after signature changes; contract
crates remain ordinary Rust dependencies without shared-library artifacts.

A zero-based unit enum used as a complete table axis may opt into generated
domain metadata:

```rust
#[version(1)]
#[dense]
#[names(snake_case)]
enum FontRole: u8 {
    Body,
    Heading,
}
```

`#[dense]` requires an unsigned representation and exact discriminants
`0..COUNT`; it generates `COUNT` and `ALL`. `#[names(snake_case)]` additionally
generates `NAMES` and `canonical_name()`, which returns `None` for an unknown
ABI integer. The generator deliberately does not add an `index()` method:
implementations that store a dense table can bounds-check `value.raw() as
usize`, while payload enums retain their semantic structure instead of being
flattened into arithmetic offsets. These authored policies do not change native layout, but they do participate
in the shared declaration fingerprint, equally for templates and concrete enums.

The ordinary identity form is:

```rust
#[version(1)]
interface IWidget {}
```

An imported ABI, a moved declaration, or an identity retained during migration
can use an explicit UUID:

```rust
#[id(1, "9be73186-f123-47bc-8140-4a57825eeb21")]
interface IWidget {}
```

The explicit UUID does not change the diagnostic name. The example remains
named `neocogi.ui.core.IWidget`.

### Contract changes and versions

The initial production IDL baseline uses `#[version(1)]` for every declaration,
including bundled library types. Test fixtures and examples may use other
versions to exercise contract evolution and compatibility.

A declaration version identifies an authored contract, not merely its physical
layout or the integers ultimately carried across COM. Increment `#[version(N)]`
when changing that contract. In particular, replacing implicit enum values with
explicit assignments, or the reverse, requires a new version even when every
resolved numeric value is unchanged. Changing member names, types, order,
ownership or numeric values also requires a new version.

These are successive revisions of one module's source file, compiled separately:

```rust
// Original contract, in the original checkout.
#[version(1)]
enum State: u32 {
    Empty,
    Ready,
}
```

```rust
// Replacement contract, in the updated checkout.
#[version(2)]
enum State: u32 {
    Empty = 0,
    Ready = 1,
}
```

Keeping version 1 in the replacement is a contract-authoring error; identical
resolved tags do not make that edit a supported same-version change. Do not put
both declarations in one module: duplicate names are rejected (`EIDL010`), even
if their versions differ. Putting them in different modules instead gives them
different qualified identities, not two spellings of the same type.

For ordinary `#[version(N)]` declarations, the qualified name and version derive
the nominal ID. An explicit `#[id(N, "UUID")]` still has a version that must be
incremented on contract changes; retaining its UUID does not bypass the exact
revision signature. Module-wide fingerprints and transitive dependency revisions
remain part of compatibility checking. Independently built plugins may use
different supported compiler versions and flags, but must agree on the versioned
contract and supported ABI; matching layout alone is insufficient.

The documented exceptions are changes that do not alter the contract: comments,
formatting, source-part placement, backend Rust paths, and consistent renaming of
generic parameter binders while preserving their positions and uses. The
parameter-position rule does not authorize changes to field/variant names or
explicit enum assignments under an unchanged version.

This is an authoring requirement, not a claim that the current compiler compares
against previously released source. Compilation resolves the supplied graph; it
does not load a historical contract baseline to enforce version increments.
Fingerprints provide compatibility checks, not permission to reuse a version
after a contract edit. Both encoders now preserve authored enum assignment presence,
so the fingerprint also detects an implicit-to-explicit edit when the author
incorrectly retains the version. This additional check does not replace the
requirement to increment that version.

External dependencies and lexical imports are deliberately separate:

```rust
extern neocogi.renderer;
use neocogi.renderer::{ITexture, Size};
```

`extern` names one exact module dependency and therefore contributes a build-time
dependency edge and the dependency fingerprint to the importing module. `use` only
introduces selected declarations as unqualified names. A `use` without its
matching `extern` is rejected; an external declaration may instead be referred
to explicitly as `neocogi::render::ITexture`. External module names are always
fully qualified and never resolved relative to the current namespace.

### Multi-file module composition

Large contracts may place declarations in any number of `.ncidl` files below
one module directory. Every file is parsed independently and then merged before
external dependencies and imports are connected or types are resolved.
References therefore do not depend on filenames, declaration placement,
filesystem enumeration order, or a hand-maintained part list.

The merged model is canonicalized as follows:

- source parts are retained by normalized relative path for diagnostics only;
- external dependencies are sorted by their canonical module name;
- equivalent imports are merged, and imported modules and names are sorted;
- top-level declarations are sorted by their exact identifier;
- field, enum-case, base-interface, method, and parameter order is retained
  exactly because those sequences are ABI-significant.

The canonical ABI encoder excludes part paths, comments, and formatting. Moving
or renaming a part, splitting one file, combining files, or changing discovery
order consequently preserves the module fingerprint and produces byte-identical
Rust bindings. Changing an ABI-significant member
sequence still changes the fingerprint.

### Type and ownership rules

- Fixed-width integers, `f32`, `f64`, ABI-defined `bool`, and `Status` are
  value types. `Status` preserves a raw `NcStatus`, including extension codes;
  method failures continue to use the implicit error side of `Result<T>`.
- A bare record or enum parameter is copied only when its complete recursive
  shape has self-contained wires. Owning named values use `&Value`; providers
  receive `NcValueRef` and explicitly acquire data to retain it.
- `&Interface` is a borrowed COM reference valid only for the call.
- Owned object results require an explicit ownership wrapper such as
  `ComPtr<Interface>` or `ComBox<Interface>`; a bare interface is not a value.
- `&str` lowers to a readable byte view for text. Providers validate UTF-8 when
  acquiring text. Stored `String` is owned UTF-8 data; an owned COM string object
  uses the explicit `ComPtr<INcString>` capability.
- `&[T]` is a borrowed contiguous input slice whose element type is ABI-safe.
- `&mut [T]` is fresh caller-owned output storage filled synchronously by the callee;
  old contents are never input. Providers receive `NcOutputWriter`, clients pass
  `NcOutputBuffer::output`, and only the reported prefix is acquired. A real
  transformation takes explicit readable input plus separate output. This does
  not grant mutable access to an object or retained collection. See
  [directional copied output](COM_OUTPUT_BUFFERS.md).
- `Result<T>` lowers to an `NcStatus` return plus an initialized output slot on
  success. `Result<()>` has no output slot.
- `Option<T>` is an ordinary enum in the bundled `neocogi.lang` library, with
  the same tag, payload, metadata, and capability rules as authored enums.
- `usize`, `isize`, arbitrary Rust references and layouts are not IDL wire types.
  `Vec`, `Slice`, `Array`, and `ComPtr` are explicit IDL constructors with checked
  lowering; Rust container layouts never become their foreign representation.

Generated data values automatically implement both `ComSerialize` and
`ComDeserialize` when all reachable fields support persistence. The same graph
proves support for nested/recursive generic values; live COM object references
have no automatic reconstruction contract, so enclosing values receive neither
trait. Attempting to persist one is a compile-time trait error.

Payload enums use their ordinal-and-payload protocol. Fieldless integer enums
and bit sets persist their declared scalar, preserving unknown raw values.
`Guid` uses sixteen canonical UUID bytes; `Status` uses its signed raw code.
NCB records always encode fields in declaration order. All fields are required
unless `#[persistence(default)]` retains an omitted trailing suffix from a
host-provided Rust `Default`. Truncated or invalid present fields remain errors.
The default policy requires supported field codecs and enters the shared
fingerprint. `#[persistence(sequence)]` is rejected because there is no alternate
named-field representation. The old `#[deserialize]`/`if_supported`
opt-ins are removed. See [automatic value persistence](IDL_GENERIC_VALUES.md)
for the detailed format and capability rules. No COM object machinery is added
by these static trait implementations.

## Shared IDL sources

IDL modules do not require dedicated Cargo packages. Keep authoritative sources
under the owning subsystem’s `schemas/` namespace tree and run the compiler in each consuming
application or ordinary library's build script:

```text
crates/render/schemas/neocogi/renderer/
  geometry.ncidl
  resources.ncidl
  frames.ncidl
crates/render/core/
  Cargo.toml
  build.rs
  src/abi/mod.rs
```

`neocogi-idl` is a version-pinned build dependency, not a binary found on `PATH`.
The logical name selects a single `.ncidl` file or a multipart directory:

```rust
fn main() {
    for module in neocogi_idl::cargo::SharedSources::new("../schemas")
        .module("neocogi.renderer", "neocogi_render_core::abi")
        .build().expect("generate renderer bindings") {
        println!("cargo::rustc-env=RENDERER_BINDINGS={}", module.include.display());
    }
}
```

The consumer supplies the named self alias and includes its own output:

```rust
extern crate self as neocogi_render_core;
pub mod abi {
    include!(env!("RENDERER_BINDINGS"));
}
```

No `links` owner, `DEP_*` exchange or contract package is needed. Each reachable
non-bundled module has an explicit Rust location: `.module(name, rust_path)` emits
that consumer's bindings; `.import(name, rust_path)` uses definitions exposed by
an existing ordinary library when Rust helper calls exchange those concrete types.
Both resolve shared IDL and compute the normal transitive ABI. Every external
Rust declaration actually selected by emission must also match its resolved IDL
identity at compilation. Generated anonymous const assertions compare a value's
`ComValueType::VALUE_SIGNATURE`, or an interface marker's complete
`ComInterface::INTERFACE_ID`, with the resolver's finished declaration identity.
This includes emitted imports even when otherwise unused, fully qualified names,
fields, borrowed/buffer/owned types, and supplying projections of closed generics.
Local definitions are emitted directly; unused path mappings and semantic-only
open templates create no extra Rust dependency. Normal and printed generation,
shared roots and packaged source graphs use this same selection/check owner.

Module `MODULE_ID`/`ABI_FINGERPRINT` constants are descriptive facts: a facade can
reexport current constants with an older explicit type. Equal size/alignment is
also insufficient. A mismatch names the logical declaration and selected Rust
path and rejects compilation, before runtime queries can accept an advertised
contract that differs from its codec. No runtime registry entry or extra object
state is created, and canonical ABI signatures are unchanged.

Inherited physical table fields use `<BaseAbi as ComInterface>::VTable` and
borrowed inherited clients use `<BaseAbi as ComClientInterface>::Client<M>`.
The checked marker therefore selects both representations, even if a facade
also exposes an unrelated old `BaseVTable` or `BaseClient` convenience name.
The selected `BaseVtblFor<T>` constructor must type-check against that associated
table; existing typed provider-projection identity/ancestry checks still apply.
Public trait/Abi/VTable/VtblFor/Client names remain available. No extra query,
reference-count operation, core/compiler dependency or runtime graph is added.

Add other
subsystems with `.include_root("../../ui/schemas")`. Every reachable module must have
exactly one source owner across all roots; root order cannot shadow a definition.
Repeating a canonical root is harmless. Source maps preserve the actual owning root.
Unmapped imports, conflicting Rust locations/local-import policies, and simultaneous
file/directory owners fail the build. Rerun directives track source parts. Generated
filenames use existing logical module UUIDs to avoid case/separator collisions and
remain bounded; callers use each returned include path rather than guessing names.

Independent consumers obtain distinct Rust types with matching ABI layouts, IIDs
and signatures. Their COM calls negotiate that exact identity, and generated
bindings own no runtime state. The host's sole runtime registry is unaffected.
The standalone host/provider and independent peer exercise this model. Native
Rust helper calls still require their declared Rust types: compile helpers with
local bindings or explicitly import the helper library's binding definitions.
The [example contracts](../../examples/schemas) and
[COM test fixtures](../../schemas) each own a shared source tree. Ordinary Cargo
build scripts generate the selected bindings. Bundled language and persistence
sources ship with the SDK so crate archives build independently of a checkout.

Handwritten source must not redeclare an available contract `struct`, `enum`,
`bitenum`, or `interface`. Those declarations belong in `.ncidl`.

Generated Rust is syntax-aware formatted source rather than a serialized token
stream. It retains ordinary multiline layout and renders IDL documentation as
`///` comments, so compiler diagnostics and inspected `OUT_DIR` files remain
readable. Generation uses an in-process printer and does not launch `rustfmt`
from every contract build.
The build helper also resolves and hashes the complete typed module graph once.
It derives a declaration-scoped signature from that fingerprint and each
declaration ID, then embeds the final 32 bytes directly in generated Rust. This
keeps distinct structs and enums distinguishable in typed descriptors while
making the exact module revision the compatibility unit. rustc still compiles
the generated layouts and typed metadata. Nominal declaration hashes are not
recomputed. Intrinsic arrays derive a constant-size const digest from an existing
element identity and length; runtime dispatch retains no schema serializer or
hasher.
Handwritten source may add inherent Rust conveniences to generated values—such
as validation, `Default`, or local conversions—because Rust coherence requires
those implementations to live in the generated type's
defining crate. Such helpers do not enter the contract ABI
fingerprint. Runtime state, JSON document models, caches, concrete factory
fields, and other implementation-only types live in their respective
implementation crates.

### Separately distributed source dependencies

When a separately distributed library uses `cargo::Module` to publish its IDL
source graph, that existing package declares the `links` value returned by
`neocogi_idl::cargo::contract_links_name(module)`: `nc_contract_` followed by the
lowercase hexadecimal UTF-8 bytes of the **exact dotted IDL module name**.
For example, `a_b.c` becomes `nc_contract_615f622e63`, while `a.b_c` becomes
`nc_contract_612e625f63`. Demangling strips `nc_contract_`, decodes hex pairs,
and reads UTF-8. This is a reversible Cargo ownership key, not a native library
filename, plugin identity or ABI fingerprint. Case remains distinct even when
Cargo uppercases the key in `DEP_<KEY>_IDL_GRAPH`. The library author writes this
key into Cargo.toml and invokes `cargo::Module` from build.rs; there is no module
scaffolding command. `SharedSources` generation from shared subsystem sources
requires neither a links key nor a source-publishing package. No old-key aliases
are read. Generated underscore file stems remain local to each package and do not
establish global ownership.

Its build
script publishes every source part and a machine-readable transitive IDL graph
through Cargo metadata. A dependent module reads that exact graph from Cargo's `DEP_*`
environment variable, so it neither searches the workspace nor guesses where a
registry dependency was unpacked. Conflicting sources for one logical module
fail resolution. The dependency crate remains an ordinary Rust dependency as
well, giving generated bindings the expected imported ABI types.

A consumer declares Cargo dependencies for its authored `extern` modules. Closing
an imported template can introduce free types from that template's dependencies;
those do **not** require additional direct consumer dependencies. Cargo preserves
which direct provider supplied each source graph, and the generator accesses
indirect types through that provider's compiler-owned `__nc_dependencies` namespace.
For example, if consumer depends on bridge and bridge depends on leaf, a closed
bridge template can contain leaf's `Fixed` without consumer naming the leaf crate.
The route re-exports the original Rust type; it never creates another declaration.
This also applies to fully qualified free types without an authored `use`.

Each provider re-exports its complete transitive source-owner set under flat
`module_<UUIDhex>` aliases. The suffix is the existing logical module UUID, not an
ABI signature or reversibly encoded source name; generated comments retain the
dotted module name. Routes remain one provider hop long even across deeper graphs.
An authored direct dependency takes precedence; otherwise sorted logical extern
names choose the first provider for a shared ancestor. Conflicting sources or
owner locations still fail the normal graph merge before generation.

Canonical owner Rust paths remain in inspection metadata. Only Rust type/import
access and the hidden provider namespace use consumer-specific routes. Neither
those routes nor filesystem roots affect logical IDs or ABI signatures. Pure
`generate_rust_with_paths` callers continue supplying accessible owner locations;
Cargo-specific routes are selected by `Module::build`. Source graph schema 7
requires providers with this namespace and rejects older sidecars. Runtime and
plugin discovery and the core baseline remain ABI v1; source routes do not change
the [canonical signature encoding](#canonical-signatures).

### Diagnostics

Parsing, module merging, name resolution, and type checking use structured
diagnostics with stable `EIDL` codes. Each source label retains its normalized
path and exact range. Duplicate declarations, fields, enum cases, bit flags,
methods, parameters, external dependencies, imports, discriminants, and bases
identify both the later occurrence and the original occurrence. Type-category
errors identify the use and the declaration that establishes the actual
category.

Semantic validation collects independent errors from the complete module graph
and renders them by normalized path, position, code, and message. The result is
stable when Cargo or a test supplies source files and modules in a different
order. Invalid copied-value recursion is diagnosed as an IDL layout cycle
instead of leaking through as a generated-Rust compiler error.

```text
error[EIDL060]: struct Resource field texture must use a copied value type
 --> neocogi/ui/core/resources.ncidl:18:14
  |
18 |     texture: ITexture,
  |              ^^^^^^^^ expected a copied value, found interface
 --> neocogi/ui/core/rendering.ncidl:42:11
  |
42 | interface ITexture {
   |           -------- interface "ITexture" is declared here
 = help: copied struct, enum, or bit enum
```

## Build graph and exact compatibility

`Module::build()` writes generated bindings and a versioned source graph to
`OUT_DIR`, and publishes the graph path through Cargo's `links`/`DEP_*` metadata.
The versioned graph contains semantic source records and logical module
ownership, plus separate physical roots for source-map navigation. Roots are
build-only locations and do not enter source equality or ABI fingerprints. The
graph contains no native artifact paths, linkage mode or runtime ownership tags.
An ordinary `cargo build` performs the entire compilation. No external CLI,
nested Cargo invocation, target rustc invocation, C compiler, or archiver is
launched by the contract build helper.

The [canonical module encoding](#canonical-signatures) includes threading,
required projections and the exact identities of referenced core interfaces.
The numeric core discovery version is a separate admission check, not a
fingerprint input. The resolver includes exact transitive module fingerprints in each declaration
signature. The plugin descriptor publishes its root factory's full IID and
signature. Factory acquisition compares both before calling bootstrap;
`QueryInterface` and typed pointer transfers continue to check exact identity.
Different disconnected revisions can coexist. The runtime does not register a
module graph or impose a process-wide single revision for each module.

`WeakComPtr<I>` uses ownership tag 15, distinct from shared strong references
(9), exclusive owners (12), and exclusive weak observations (13). Closed generic
identities also preserve this distinction. Existing spellings keep their encoding.

## Implementation and plugin publication

Shared implementations constructed with `into_local(value)` support
`LocalCom<T>::downgrade() -> Result<WeakLocalCom<T>, NcStatus>`. Promotion returns
`Result<Option<LocalCom<T>>, NcStatus>` and acquires an ordinary strong owner
before typed borrowing. This concrete Rust access remains in the constructing
binary; IDL names only its interface projection as `WeakComPtr<Interface>`.

For example, a contract can return `Result<WeakComPtr<Service>>`, or borrow a
record containing `observer: WeakComPtr<Service>`. Nested records, arrays,
sequences, options and closed generic fields preserve weak ownership. Acquisition
checks the exact interface and shared ownership kind without promotion, so expired
observers remain valid transported values. Calling their target requires a
successful `upgrade`. Direct weak inputs, copied weak aggregate parameters/results,
and implicit persistence of weak references are unsupported. Borrow a named
record to transfer weak facts. Structural metadata distinguishes `WeakObject`
from the strong `Object` case.

`#[com_object(Interface)]` implements any closed interface contract using private
Rust fields. Shared providers use `from_value(initialized_value)`; providers marked
`ownership = exclusive` use `into_box(initialized_value)`. Typed local access uses
`into_local(value)` or `into_local_box(value)`, respectively. Only the selected
ordinary constructor family exists; weak identities can be connected privately
after allocation and before the factory returns. Unit objects also require
explicit initialized values. Open generic IDL interfaces have no callable ABI;
a generated closed specialization can be used exactly like any other interface.

A plugin defines a typed acquisition function taking the host-supplied `Runtime`
in its implementation crate:

```rust,ignore
#[neocogi_com::plugin(name = "example.plugin")]
fn acquire_root(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<api::IRootAbi>, neocogi_com::NcStatus> {
    Root::from_value(Root::new()).query()
}
```

The return must explicitly spell `Result<ComPtr<InterfaceAbi>, NcStatus>` (qualified
paths are accepted). Rust checks its exact threading and error type. The macro
reads the generated interface identity and threading rather than inferring ABI
from method bodies. Version defaults to `CARGO_PKG_VERSION`. It generates a static
`plugin_descriptor()` accessor and the discovery export under `dynamic-plugin`.
Successful acquisition transfers one reference, without promising a singleton.
Initialization errors are normalized and the raw output remains empty on failure.
The generated C adapter receives the host API pointer and factory output slot;
the safe Rust function receives access to that same host-owned global registry.
Descriptor discovery only lends static metadata and does not acquire a factory.
Ordinary interface methods own parameterized object construction; there are no
IDL `class` or `factory` roles, constructor registries or erased argument bags.

Hard provider requirements can be listed as `dependencies = [PLUGIN_ID]`.
There is no `contracts` list. Plugin descriptor ABI v1 contains the full factory
identity, threading capability, bootstrap pointer, plugin metadata, and provider
IDs. Its native getter is `nc_get_plugin_descriptor_v1`.

Dynamic loading validates and registers one plugin atomically. Providers must
already be registered; registration invokes neither factories nor dependency
loaders. Root acquisition then checks the requested signature before bootstrap.
A monolith links plugin rlibs, registers their Rust descriptor getters directly,
and uses the same acquisition operation against the one implementation explicitly retained by its final monolith owner.

## Tool ownership and migration

The compiler lowers the bound graph directly into typed method and value plans.
The same library's `neocogi_idl::codegen` module emits provider traits, C wires/vtables, clients, thunks,
metadata and persistence implementations from those plans. Generated contracts
contain complete Rust instead of interface/value attributes whose expansions
would have to recover ownership by parsing Rust types again. Core depends on
neither compiler nor procedural macros; the explicit interface-only bootstrap emits its
checked-in persistence ABI and the facade's provider half from the same plan.

Copied native records have ordinary Rust application layout. COM owns their
explicit C wire layout and checked conversion. Direct borrowing still requires
the codec's complete `BORROW_WIRE` size, alignment, offset and bit-validity proof.
It cannot infer identity from the application's representation attribute.

Generated contract definitions need no procedural macro feature. A package that
also compiles concrete Rust objects, including handwritten event snapshots, must
request `neocogi-com/macros` in its own normal dependency. Its name ending in
`contracts` does not make those implementations generated-only code. Enable `alloc`
for owned values and `std` where required by local code.

- `neocogi-idl` owns parsing, semantic validation, and canonical fingerprints.
- `neocogi-idl` owns source discovery, graph exchange, and Rust generation.
- Each consumer’s build script generates its selected IDL; Cargo builds applications/plugins.
  Its IDL tools can inspect complete source declarations without shipping them.
- `neocogi-inspect` reads plugin/dependency/factory metadata as inspection schema
  9. It compares exact factory signatures and plans provider load order.

Use `interface` for public capabilities, required projections and typed object
construction methods. Remove former `class` and `factory` keywords; their names,
methods, threading and source metadata remain on ordinary interfaces.
Implement storage with `#[com_object]` and plugin acquisition with `#[plugin]`.
Exclusive objects use `ownership = exclusive`; shared services use the default
shared policy. Keep event receivers in explicit private fields initialized by
ordinary interface methods. Use interface `IID`, `INTERFACE_ID` and
`INTERFACE_DESCRIPTOR`. Low-level
handwritten `ComInterface` implementations must specify `type Threading`.
Remove contract registration and `MODULE` imports, and rebuild against runtime
ABI v1 and the versioned plugin descriptor getter. No legacy adapter retains the
removed runtime contract mechanism.

### Generated IDL metadata

[`neocogi_com::idl_metadata`](../../crates/api/src/idl_metadata.rs) contains
static descriptions of interfaces and values for tools that need to read a
contract without linking the IDL compiler. `InterfaceAbi::IDL_INTERFACE` is an
`InterfaceMetadata`: it describes methods, receiver access, argument/result
ownership, and reachable value definitions. `Value::IDL_TYPE` is a `TypeMetadata`
describing record fields, enum cases or bit flags. These records describe the
source contract; physical interface descriptors still define ABI compatibility.

Local interface records hold `qualified_name: &'static str`, an
`InterfaceIdentity` containing the IID and required `&'static [u8; 32]` signature,
and a static slice of those identities. Safe construction enforces the text and
digest storage lifetimes. It does not prove handwritten records describe a real
provider; domain admission and actual COM queries retain that responsibility.
Local readers follow no raw descriptor pointers. The compiler emits both local
and physical identities from the same resolved declarations, preserving exact
digest bytes without changing canonical encoding or the physical ABI.

The local interface chain is derived-first in declared-parent order, with shared
ancestors once and required siblings retained. A no-base interface contains only
itself; IUnknown is supplied separately by object construction. Methods remain
base-first and retain their bound supplying scopes. Generation rejects repeated
IIDs with different signatures before deduplicating them. Full generated-chain
comparisons protect agreement with physical projection order.

For example, a tool reading an interface with `fn set_value(&mut self, value: u32)`
finds a mutable method named `set_value`, one `u32` parameter named `value`, and
a unit result. It can inspect that data without calling the method or owning an
implementation. The metadata constants create no runtime registration.

Generated IDL records are Rust-side static data inside their defining binary.
Applications can define explicit COM interfaces to copy the metadata they need
across a component boundary. Such interfaces own their data format and validation;
Rust references and private implementation pointers do not cross COM.

## Exclusive object transfers

`ComBox<T>` is a compiler-defined exclusive owner for a closed object contract.
Factory results transfer the sole owner; container parameters consume it.
`WeakComBox<T>` results transfer cached observers without retaining the value:

```rust,ignore
#[version(1)]
interface IContainer {
    fn insert(&mut self, child: ComBox<IWidget>) -> Result<()>;
    fn child(&self, index: u32) -> Result<WeakComBox<IWidget>>;
}

#[version(1)]
interface Widgets {
    fn button(&self) -> Result<ComBox<Button>>;
}
```

These spellings have distinct canonical fingerprints and COM inspection passing
modes. They are method transfers, not repeatable struct/enum value fields. A
consuming Rust parameter returns its original owner on failure. IDL derives the
Rust error tuple from the parameters: `Result<R, (NcStatus, ComBox<I>)>` for one
owner, or `(NcStatus, owner1, owner2, ...)` for several, in parameter order.
Success consumes every owner; rejection preserves them and the destination.
The raw slot is null on success and unchanged on failure. Forwarding providers
propagate the complete returned-owner error. There is no public transfer wrapper.
Shared `ComPtr` inputs retain their existing prohibition; borrow services with `&T`.

Consuming methods may combine owners with checked writable slices and copied
results. Generated callers and providers reserve conversion storage and admit
the maximum possible output cost before invoking the implementation. The shared
COM macro requires an allocation-free `MAX_COST` for each converted output codec;
there is no separate IDL budget calculation. Excessive known output limits reject
at preparation and return the owners. Non-consuming methods keep actual-output
budgeting. See [prepared output completion](COM_OUTPUT_COMPLETION.md) for an IDL
example, enum limits, and real generated-call regressions.

The provider threading guarantee is preserved for all three owner/observer
wrappers. See the [current core ABI](../abi-v1.md) for lifetime and concurrency rules.

## Interpreting generated names

| Generated spelling | What it represents | How to interpret it |
| --- | --- | --- |
| Cargo `nc_contract_<hex>` | Exact dotted logical IDL module name | Strip the prefix, decode hex pairs, read UTF-8; dots, underscores and case survive. |
| `__nc_dependencies::module_<UUIDhex>` | Build-generated typed route to a transitive owner | The existing logical module UUID names the alias; generated comments retain the module name. This re-exports its Rust types and does not create a runtime identity. |
| Local `<module_with_underscores>.rs` and `__nc_idl_<stem>` | Package-local output/module name | Dots become underscores, so this spelling alone is not reversible. The generated entry names the original module explicitly. |
| `NameInstance<hex>` | Nominal identity of a closed generic IDL type | Decode hex to UUID bytes only. Read the generated closed-type comment for the template and arguments; the UUID cannot recover them. |

Cargo ownership keys are never native library filenames. Specialization UUIDs
are never ABI signatures. Generated comments keep the original logical module
or closed type beside these names. Their spelling and build-sidecar schemas do
not advance runtime or plugin descriptor discovery: both remain v1. See
[generic name documentation](IDL_GENERIC_VALUES.md#reading-generated-specialization-names)
and [source-map lookup](../guides/IDL_SOURCE_MAPS.md).
