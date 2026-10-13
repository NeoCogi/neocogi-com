# NeoCogi IDL modules

Contracts are build-time definitions. Each logical module owns an ordinary Cargo
crate containing generated Rust layouts, typed wrappers, and fixed signatures.
No contract shared library, native description archive, getter, runtime graph,
or contract registration step is produced.

`interface` is the common callable model. `factory` marks the only parameterless
construction roots. Concrete data defines event values; an ordinary queue argument
selects their receiver during construction. Objects are private implementation values.
The old `trait` and callback `port` declarations are removed. `class` declares a concrete object contract whose projections are implemented by one COM identity; private state and method bodies remain in Rust. Construction still belongs to an explicit factory.

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
ABI identity remains independent of paths and locations. Run
`cargo neocogi module generate <module>` to obtain the exact map path. The
[source mapping guide](../guides/IDL_SOURCE_MAPS.md) defines the format and lookup rules.

## Closed generic values, interfaces, and concrete classes

Structs, enums, and interfaces can declare type parameters. Each use must supply all arguments:

<!-- checked-example: generic-stream -->
```ncidl
#[version(1)] struct Snapshot<T> { value: T }
#[version(1)] interface IValue<T> {
    fn get(&self) -> Result<Snapshot<T>>;
}
#[version(1)] class Counter: IValue<u32> {}
#[version(1)] factory Counters {
    fn create(&self) -> Result<ComBox<Counter>>;
}
```

A template is a source-level contract, not an open COM vtable. The compiler
substitutes each closed use into an ordinary concrete declaration, then applies
the same field ownership, finite-layout, copied-position, inheritance, threading,
and delivery checks used for declarations without parameters. Thus
a generic method taking `&T` cannot instantiate T as a scalar; `IValue<u32>`
with scalar-valued `get`/`set` methods is valid. Methods,
classes, and factories do not declare generic parameters. Type argument bounds
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
nested arguments, recursive values, enum payloads, interface bases, and method
inputs/results. Parameter count and order remain significant: exchanging which
parameter a field uses changes the template signature, even if a particular use
supplies the same concrete type for both arguments. Qualified free types, field
names, variant names, and method parameter names are not generic binders and
retain their existing contract meaning. Authored binder names and source spans
remain available for diagnostics; hashing does not rewrite the source model.

Templates and concrete declarations now share the explicit binary declaration
encoder under `neocogi-idl:module-abi:v9`. The former separate template string
encoding and its Debug-formatted policy values are removed. Both paths encode
parameter positions, resolved free identities, member order, authored enum
assignment presence, and declaration policies. Regenerate bindings and rebuild
all contracts/consumers after adopting this encoding; there is no compatibility
adapter for the old signatures. Module-wide revision checking is retained.
Changing implicit enum values to explicit assignments still requires a declaration
version increment; see [contract changes and versions](#contract-changes-and-versions).

`class` uses the same callable ABI machinery as `interface`, with a distinct
contract role. Its first required interface provides the physical vtable prefix;
other required interfaces are sibling projections on the same COM identity.
Factories return the class projection. No constructor or implementation body is
embedded in IDL, and a class can describe non-UI objects as well as widgets.

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
`struct Loop<T> { child: Option<Loop<T>> }` fails when instantiated. A generic
value may itself be an argument to `EventBinding`, and all cases still belong
to that class's single event stream.

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
class Button: IWidget {
    fn label(&self) -> Result<ComPtr<INcString>>;
    fn set_label(&mut self, label: &[u8]) -> Result<()>;
    fn set_image(&mut self, image: &ITexture) -> Result<()>;
}

#[version(1)]
factory UiFactory {
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
factory UiPluginFactory {
    fn create_runtime(&self) -> Result<ComPtr<UiRuntime>>;
}
```

These blocks illustrate the language; their small example contracts are independent
of the production UI topology. The generic examples above and both modules here
are extracted from this document, resolved, generated and compiled by
`cargo test -p neocogi-idl --test documentation`. The
[COM README](../../crates/com/README.md#idl) adds an executable
implementation and factory bootstrap checked by the same test.

The production `neocogi.ui.core` module owns shared node,
window, menu, and popup protocols. `neocogi.ui.nodes` imports core and owns
built-in object interfaces, specialized events, and `UiFactory`. Core
`UiRuntime::construction_host()` returns its canonical `IConstructionHost`; core never imports nodes. An
independently activated `NodesPluginFactory` binds that runtime and returns `UiFactory`. The
generated core module owns `ChoiceMenuEntry`, `MenuTextRange`, `ChoiceSelection`, `IChoiceMenu`,
and its changed event because reusable hierarchy, popup lifecycle, and committed selection are
core menu policy. Nodes owns Combo presentation and the other concrete controls, so the
handwritten facade duplicates none of those records.

The optional suffix after an enum name selects its discriminant or tag type:
`u8`, `u16`, `u32`, `i8`, `i16`, or `i32`. Omitting it defaults to `u32`.
Representation is IDL syntax (`enum Name: u8`), not a Rust `#[repr]` attribute.
An enum whose variants carry no payload generates a transparent fixed-width
integer newtype and preserves unknown values. An enum with tuple or named
payloads generates a tagged sum representation. This keeps ordinary protocol
enumerations compact while making payload layout explicit.

Enum classification follows the declaration, before generic specialization. If
any variant has tuple or named fields (including a `()` payload), the whole declaration
is a tagged sum. Every variant then receives its zero-based declaration ordinal;
explicit `= value` assignments are rejected on payload and unit variants alike
with `EIDL073`. This also applies to unused templates. Only wholly fieldless
integer enums may assign numeric values, including sparse and signed values.
`enum Choice<T> { Empty, Value(T) }` is valid;
`enum Choice<T> { Empty = 0, Value(T) }` is not. These tags are defined by IDL,
independently of Rust's native enum representation or payload size. The copied
`com_enum` and owning `com_value` Rust macros enforce the same rule: neither
`#[com_tag(...)]` nor explicit Rust discriminants may override a payload tag.
IDL emits ordinary variants; both macros assign the same zero-based ordinals.

A bit enum requires an explicit `u8`, `u16`, or `u32` representation and an
explicit nonnegative value for every flag. It generates a transparent value
with private storage, declared constants, `raw()`/`from_raw()`, validity and
containment queries, and typed bitwise operators. Any combination containing
only declared bits is valid; unknown bits remain representable so component
boundaries can reject them without losing the received ABI value.

IDL identifiers are already their generated Rust API names; generation never
silently rewrites them. The parser enforces the corresponding conventions:

- `struct`, `enum`, `bitenum`, `interface`, `class`, and `factory` names use
  `UpperCamelCase`.
- Ordinary enum variants use `UpperCamelCase` just like Rust enum variants.
- Bit-enum flags use `SCREAMING_SNAKE_CASE` because they are combinable typed
  constants rather than mutually exclusive variants.
- Struct fields, named enum payload fields, methods, and parameters use
  `snake_case`.

Public generated names share the declaration/import namespace. The resolver
checks interface companions and codec helper names before emission; collisions
report `EIDL021`. Event declarations generate no per-product helper names.
`#[events(...)]` is validated separately against immutable event interfaces, with
invalid contracts reported as `EIDL065`. See [event metadata](IDL_EVENT_METADATA.md).

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
class Editor {
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

For several event kinds, declare concrete interfaces derived from `IEvent` and
advertise them using `#[events(...)]`. A factory accepts the common `EventBinding`;
its Observed case contains `IEventReceiver`. The parent supplies that receiver
before publication. Events transfer as exclusive `ComBox<IEvent>` capabilities.
There is no later attachment or replacement operation. See
[typed class event streams](#typed-class-event-streams) and
[the ownership guide](CLASS_EVENT_STREAMS.md).

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
- A bare struct or enum parameter is copied by value.
- `&Interface` and `&Factory` are borrowed COM references valid only
  for the call.
- A bare interface or factory return is one owned COM reference.
- `&str` is a borrowed UTF-8 view; `String` is an owned immutable COM string.
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

## Module crates

A module is distributed as an ordinary Cargo package so consumers use a normal
dependency and Cargo observes source changes:

```text
contracts/neocogi-ui-core-contracts/
  Cargo.toml
  build.rs
  src/lib.rs
  idl/neocogi/ui/core/
    geometry.ncidl
    input.ncidl
    resources.ncidl
    widgets.ncidl
```

The package declares a stable Cargo `links` value for its logical module. Its
build script invokes the version-pinned `neocogi-idl` library rather than
an executable found on `PATH`:

```rust
fn main() {
    neocogi_idl::cargo::Module::directory("idl", "neocogi/ui/core")
        .build()
        .expect("failed to build neocogi.ui.core");
}
```

`Module::new("idl", "neocogi/ui/core.ncidl")` remains the single-file form. It has
the same compiler semantics and is useful for small modules; it is not a
different contract format.

The reusable parser/resolver and build integration live in `crates/com/idl`
and `crates/com/idl`. These COM-owned compiler packages are not private
modules inside `cargo-neocogi`: contract build scripts must work during an
ordinary direct Cargo build as well as during an assembled build. The example
contract crates under `tools/idl` remain compiler fixtures.

The helper emits precise `cargo::rerun-if-changed` directives. Modifying the
module source therefore regenerates bindings during the
next Cargo build. The generated Rust library includes bindings from `OUT_DIR`:

```rust
#![no_std]

include!(concat!(env!("OUT_DIR"), "/neocogi_ui_core.include.rs"));
```

The module crate's handwritten `src` must not redeclare a contract `struct`,
`enum`, `bitenum`, `interface`, or `factory`. Those declarations belong in
`.ncidl`.

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

Every contract package declares `links = "nc_contract_<module>"`. Its build
script publishes every source part and a machine-readable transitive IDL graph
through Cargo metadata. A dependent module reads that exact graph from Cargo's `DEP_*`
environment variable, so it neither searches the workspace nor guesses where a
registry dependency was unpacked. Conflicting sources for one logical module
fail resolution. The dependency crate remains an ordinary Rust dependency as
well, giving generated bindings the expected imported ABI types.

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
Graph schema 5 contains sources and logical module ownership; it contains no
native artifact paths, linkage mode, or builtin/runtime-owned module tags.
An ordinary `cargo build` performs the entire compilation. No external CLI,
nested Cargo invocation, target rustc invocation, C compiler, or archiver is
launched by the contract build helper.

Canonical module encoding v9 includes interface roles and the current COM ABI explicitly. The resolver
includes exact transitive module fingerprints in each declaration
signature. The plugin descriptor publishes its root factory's full IID and
signature. Runtime activation compares both before calling bootstrap;
`QueryInterface` and typed pointer transfers continue to check exact identity.
Different disconnected revisions can coexist. The runtime does not register a
module graph or impose a process-wide single revision for each module.

`WeakComPtr<I>` uses ownership tag 15, distinct from shared strong references
(9), exclusive owners (12), and exclusive weak observations (13). Closed generic
identities also preserve this distinction. Existing spellings keep their encoding.

`neocogi.com.events` is a logical contract module owned by the COM facade. It
publishes `IEvent`, `EventOrigin`, `IEventReceiver`, and the ordinary
`EventBinding` enum. Producers advertise concrete event interfaces through `#[events(...)]`. UI scheduling uses `neocogi.ui.core::IWidgetDispatcher::dispatch_next` and the
separate, borrowed `IApplicationDispatcher` for application messages. Neither
the IDL nor the COM process runtime owns a UI event loop.
Consumers depend on `neocogi-com` with `macros`; no runtime contract registration
or connection lease is required.

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
from the strong `Object` case; UI design catalog schema 10 carries that distinction.

`#[com_object(Interface)]` implements a reusable interface, and
`#[com_class(Class)]` implements a concrete class contract. Both use private Rust
fields. Shared providers use `from_value(initialized_value)`; providers marked
`ownership = exclusive` use `into_box(initialized_value)` or its cyclic/local
variants. Only the selected constructor family exists. Ordinary unit objects receive no default
constructor. `#[com_factory(Factory)]` additionally implements
`ComFactoryImplementation<Interface = FactoryAbi>`. Unit factories receive
`ComFactoryBootstrap`; stateful factories implement that safe typed policy
explicitly. Only factory bootstrap is callable without arguments through the
runtime; later construction uses typed factory methods.

```rust,ignore
neocogi_com::factory_plugin! {
    name = "example.plugin",
    version = "1.0.0",
    factory = RootFactoryImplementation,
}

#[cfg(feature = "dynamic-plugin")]
neocogi_com::export_factory_plugin!();
```

Hard provider requirements can be listed as `dependencies = [PLUGIN_ID]`.
There is no `contracts` list. Plugin descriptor ABI v1 contains the full factory
identity, threading capability, bootstrap pointer, plugin metadata, and provider
IDs. Its native getter is `nc_get_plugin_descriptor_v1`.

Dynamic loading validates and registers one plugin atomically. Providers must
already be registered; registration invokes neither factories nor dependency
loaders. Factory activation then checks the requested signature before bootstrap.
A monolith links plugin rlibs, registers their Rust descriptor getters directly,
and uses the same activation operation against one `static-runtime` instance.

## Tool ownership and migration

- `neocogi-idl` owns parsing, semantic validation, and canonical fingerprints.
- `neocogi-idl` owns source discovery, graph exchange, and Rust generation.
- `cargo-neocogi` optionally manages modules and assembles applications/plugins.
  Its IDL tools can inspect complete source declarations without shipping them.
- `neocogi-inspect` reads plugin/dependency/factory metadata as inspection schema
  9. It compares exact factory signatures and plans provider load order.

Use `interface` for reusable capabilities, `class` for concrete object contracts, and `factory` for construction.
Replace callback `port` declarations and observer annotations with ordinary values
and a factory accepting `EventBinding`. Widget implementations
use `#[com_class(Contract, ownership = exclusive)]` and ordinary fields for the
typed queue and cached weak origin. Shared services use the default shared policy
and may use the common Rust `EventPort`. Replace `ComObjectDescription` uses with `ComInterface`,
`ComFactoryDescription` with `ComFactoryInterface`, and the implementation-side
`ComClass` marker with `ComObject`. Factory implementation associations use
`Interface` instead of `Description`. Use interface `IID`, `INTERFACE_ID`, and
`INTERFACE_DESCRIPTOR` instead of duplicate `OBJECT_*` constants. Low-level
handwritten `ComInterface` implementations must specify `type Threading`.
Remove contract registration and `MODULE` imports, and rebuild against runtime
ABI v1 and the versioned plugin descriptor getter. No legacy adapter retains the
removed runtime contract mechanism.

### Generated authoring metadata

Each IDL-generated COM interface projection exposes `*Abi::IDL_INTERFACE` beside
its physical `INTERFACE_DESCRIPTOR`. The former points at the latter, retains
its required interface chain, and describes the declared event interfaces and
their complete inherited getter results. See [event metadata](IDL_EVENT_METADATA.md).
Every struct, enum, and bit enum exposes `Type::IDL_TYPE`; postable messages also
expose `Message::IDL_MESSAGE`. These records include ordered struct fields,
enum cases and payloads, and named bit flags. A custom-widget designer can inspect these records through
the registered concrete COM interface type, then verify a constructed object by
`QueryInterface` against the descriptor's identity and ABI signature. This is
source-derived metadata; UI documents do not redeclare Rust paths, event types,
or message shapes. Physical descriptors remain the authority for ABI validation.

Generated IDL records are Rust-side static data inside their defining binary.
For independent widget plugins, the root factory implements the shared
`IDesignWidgetCatalog` COM interface. Its `describe` method copies a bounded,
versioned `DesignWidgetManifest` built from those records across COM. The
standalone designer first inspects the plugin's validated plugin descriptor,
activates its exact root identity, queries the catalog, and reads the owned
struct/enum/message and property metadata. `create_widget` constructs a real
widget through COM; the host verifies its advertised IID and ABI signature.
The catalog does not transfer Rust references or expose private widget pointers.
A consuming Rust application still links the generated contract crate to name
its concrete event getter types and optional form fields.

### Typed class event streams

`IEventReceiver` accepts a checked weak `EventOrigin` and transfers an exclusive
`ComBox<IEvent>`. Concrete event interfaces derive from `IEvent` and expose immutable
getters. The common `EventBinding` selects permanent Unobserved or Observed delivery.
Factories accept it before publishing a product; no attachment method is generated.

`#[events(...)]` declares emitted interface identities. `IDL_INTERFACE.events`
contains their real getter metadata, independently of the widget's interface chain.
Hosts use `events::channel()` or implement `IEventReceiver` once for their inbox.
Messages keep the exclusive COM owner until handling. Unsupported event queries
return `None`; actual ABI/provider errors remain errors.

`EventPort` is a nongeneric shared-service convenience with explicit terminal
closure. Exclusive widgets store their common receiver and cached weak origin.
`com_class(Contract, ownership = exclusive)` generates allocation and weak-call
admission, without event installation or callback adapters. See
[COM event objects](COM_EVENT_INTERFACES.md).

The queue acquires facts synchronously; application handlers run later with
`&mut self` after traversal releases its borrows. No parent state pointer or Rust
closure crosses COM. Catalogs transport declared event interfaces and getter metadata; custom factories
receive the common constructor binding. See
[the detailed ownership, state and teardown rules](CLASS_EVENT_STREAMS.md).


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
factory Widgets {
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
wrappers. See [exclusive widget ownership](EXCLUSIVE_WIDGET_OWNERSHIP.md) and
[current core ABI](../../crates/com/docs/README.md) for lifetime and concurrency rules.
