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
Methods and fields accept documentation attributes at their own declaration.
Receivers, parameters and integer expressions accept no attributes, including
documentation or Rust `cfg`; unsupported annotations are rejected before lowering.
Type paths are module-relative without a leading `::`, including built-in types
and `Result<T>`. Plain integer spelling and equivalent collection/type punctuation
normalize without adding Rust attribute semantics.

A suffix such as `interface Button: IWidget + IActions` gives one physical vtable
prefix (the first entry) and required sibling projections (later entries). All
are queryable on the same object; requirements are transitive and acyclic.
Handles carry an object address and a selected static interface table. Interfaces
add no fields to each object allocation; inherited projections are deduplicated at
compile time. The [object/table storage guide](COM_INTERFACE_STORAGE.md) explains
this settled representation and the earlier reverted experiments. Interfaces
default to local threading. `#[threading(send)]` requires Send and
`#[threading(sync)]` requires Sync. `#[threading(shared)]` requires implementations to
be `Send + Sync`; a derived interface cannot weaken a required shared promise.
Generated shared provider traits enforce both bounds on their concrete
implementations, including inherited and required projections. The selected
object/handle mode is separate: an explicit interface list may keep a local
handle over a valid shared implementation, and queries retain that local mode.
A local-only interface can use non-Send/non-Sync state unless its object explicitly
requests thread-safe handles. Shared ownership and shared threading are independent.

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

Authored text is tokenized once before Syn admission. The existing iterative
`proc_macro2` fallback lexer owns comments/literals and token cleanup; actual token
groups are limited to 128 levels before Syn builds its buffer. The frontend then
parses the supported IDL grammar directly, never an unrestricted Rust type,
expression, method body or attribute expression merely to reject it afterward.
The dependency floor is the audited `proc-macro2` 1.0.107 implementation.

Normalized inline types have a maximum height of 96: a leaf has height one, a
constructor adds one, and generic siblings contribute their maximum, not their
sum. Nominal declaration edges are not unfolded. Slice/array sugar has the same
normalized bound as its intrinsic spelling; `Result` is fixed method framing.
Excessive nesting returns an owned `EIDL068` diagnostic at its actual source part
and location. These are compiler complexity limits, independent of runtime COM
value budgets and specialization work/materialization limits. They preserve the
existing 65-level output-type regression and do not impose a nominal graph cap.

`Module` owns admitted declaration/template trees. The public `declarations()`
and `templates()` accessors expose immutable slices; `ResolvedModules::values()`
likewise exposes the resolved value graph by shared borrow. Detached model clones
remain inspectable/editable, but there is no unchecked constructor, mutable
accessor or reinsertion path into these owners. This invariant applies before
bundled-source attachment, binding, specialization and generation. Migrate direct
field readers to these accessors.

Use `Method::is_scoped()` to read the authored delivery annotation.
`Method::return_passing()` and `Parameter::passing()` return the normalized
slot rules for validated closed methods. Parsed declarations and retained open
templates return `None` from those passing queries. For example, a resolved
`Option<&IItem>` exposes its loan payload in `Method::output` and its nullability
in `ReturnPassing::BorrowedLoan`. `ReturnProjection` describes whether Rust
exposes the call status separately.

Generated copied-value layout proofs use shallow sequential constant statements.
Each predicate retains left-to-right short-circuit evaluation, including field,
size/alignment and offset checks; field breadth does not become a linear binary
expression tree. Nominal generator traversals and total input/output allocation
are separate costs, so this is not an arbitrary-graph or process-memory bound.

Text parsing/generation runs in ordinary native compiler/build-script processes.
Rust procedural macro entry points consume rustc-supplied Rust syntax separately;
calling the text APIs in a warmed procedural-macro process is not a supported
context. Worker creation does not reset proc-macro2's process-wide backend state,
and the compiler does not force that state globally.

IDL source parsing and complete Rust generation use one explicit eight-MiB
syntax-worker stack policy on every host. Deep valid
output-buffer types therefore do not depend on a build-script main thread's
platform default, including MSVC's one-MiB executable stack. Single-file and
multipart parsers return owned module models and diagnostics with stable source
coordinates. Generation owns lowering, formatting, mapping and syntax destruction
inside one worker per module or bootstrap call. Rust module paths retain validated
identifier spellings and create token syntax only where needed. Original parser
spans exist only during syntax admission; syntax trees and process-local spans
never cross workers. Failure to start an IDL parser worker returns
`ParseError::Worker`. Rust binding, bootstrap, and scoped adapter generation
report their phase in `GenerateError`. Both error types expose the original
`std::io::Error` through `Error::source()`. Both Cargo builders wrap a generator
failure in `BuildError::Generation` with the logical module name and the
original generator error. These failures precede artifact publication.
The public `resolve_modules`
operation first admits the ordinary bundled `neocogi.lang` source. Its
`ModuleGraphError::BundledSource(ParseError)` retains parser/worker failures,
including their operating-system cause, rather than asserting that tested
source guarantees an available worker. The subsequent semantic algorithm starts
no syntax worker and retains `ResolutionError` diagnostics through
`ModuleGraphError::Resolution`. Cargo preserves this typed graph failure in
`BuildError::Resolve` before generation, artifact writes or success metadata.
Empty input still fails before bundled admission; the reserved-library-name
check follows bundled parsing. Internal worker panics retain their payloads.

Inline-layout/projection cycle validation and module dependency fingerprinting
walk named graph edges with explicit heap-backed frames. Long chains of shallow
authored declarations therefore do not consume one native call frame per edge.
The frames also retain cycle ancestry: inline/projection cycles keep source-located
`EIDL063` errors, and module cycles keep their deterministic dependency path.
Dependency completion retains canonical module-name order and identical digest
bytes; this traversal change does not alter IDL encoding v14 or impose a new
authored graph-depth limit. Syntax nesting and generated-specialization budgets
remain separate policies.

### Nominal traversal and bounded expression ownership

Traversal of the already admitted/materialized nominal graph must use native
call depth independent of its named-edge path length. This includes semantic
diagnostics and metadata discovery, not just successful layout/fingerprint
resolution. A visited set prevents repeated work or cycles; it does not bound
native call depth along a finite chain. Traversal scratch borrows declarations
and expressions rather than recursively copying their definitions.

This rule is distinct from three existing ownership/admission domains:

- Inline expressions stop at named identities and have admitted height 96.
  Sealed `Module`/`ResolvedModules` ownership cannot install unchecked replacements;
  detached public models and their ordinary Rust traits have no such guarantee.
- New generic materialization retains combined descent, instance, work, storage
  and prospective-height admission before reads/copies. It is not an arbitrary
  walk through an already materialized graph.
- Syntax parsing, token/AST construction, printing, mapping and destruction keep
  their existing syntax-worker owner. A larger worker is not a nominal graph cap.

The operation contracts are deliberately separate:

| Consumer | Ordering, identity and scope obligation |
| --- | --- |
| Layout/projection and module fingerprints | Explicit ancestry/completion frames; source-located cycles and canonical digest order |
| Persistence failure explanation | Per-request visited canonical value names; first unsupported leaf in field/variant DFS order; prune supported subgraphs |
| Interface ancestry | Derived-first, authored base order; compare full signature before IID dedup |
| Inherited methods | Base-first DFS completion order; mark IID on entry; keep each supplying type scope |
| Reachable metadata | Interface visits use concrete supplying module-qualified declaration names; data uses canonical names after requesting-module local projection; resolve children lazily in authored order; emit canonical-name-sorted definitions |

The executable diamond regression independently generates both peers, checks
preorder identities versus postorder methods and sorted reachable definitions,
and makes real inherited/value COM calls. Existing graph regressions freeze
fingerprint bytes and finite/cyclic resolver behavior. Persistence explanations now use borrowed field-iterator frames and a per-query
visited-name set; unary public capability predicates use loops even for detached
inputs. The first unsupported leaf and requested field labels remain unchanged.
Interface ancestry and inherited-method collection likewise retain borrowed heap
frames with their distinct preorder/completion and IID rules. A watchdog-isolated
2,049-interface regression exercises both production collectors on a 1 MiB test
thread; token creation, inspection and destruction stay on that thread with
shallow inline syntax. Reachable-value discovery uses a borrowed request frontier
and canonical output map; scheduling can scale with sibling width and does not
claim O(depth) scratch. Names resolve only when visited. Discovery returns the
actual bound declarations to syntax emission rather than copying their shapes.

### Traversal inventory and remaining boundaries

Every new consumer of an admitted nominal edge must preserve this contract on
success, rejection and cleanup of its own traversal state. Either use concrete
heap work or identify an existing bound checked before descent; a cycle set or
later codec limit is not such a bound. The current owner/check inventory is:

| Operation / edge | Responsible owner or stopping rule | Qualification / limit |
| --- | --- | --- |
| Persistence explanation follows named fields | Borrowed field iterators and per-call canonical-name membership; wrappers peel locally | Real 64-record ×95-wrapper EIDL064 regression, cycle-plus-later-sibling/root-specific reasons, imported specialization labels |
| Interface ancestry follows bases | Borrowed lazy base iterators; full32-before-IID-dedup | Production 2,049-interface small-stack collector, independent diamond metadata/COM baseline |
| Method descriptions follow bases | Borrowed completion frames; IID-on-entry | Same depth and diamond baseline, imported/generic supplying-scope consumer |
| Reachable interface/data definitions | Concrete borrowed request frontier; concrete supplying-name interface visits, canonical data visits after local projection | Deep and mixed discovery, recursive siblings, enum and phantom arguments, preferred/supplying projection and public-generation consumers |
| Layout/projection and module fingerprints | Existing explicit ancestry/postorder frames | Finite/late-cycle small-stack and frozen fingerprint regressions |
| Public capability predicates and inline array references | Unary loops; cached facts/names stop at nominal leaves | Constructor rules and detached 32K borrowed-query regression; detached fixture teardown is explicitly separate |
| Bound field conversion, field/copy validation, canonical encoding, internal type display | Admitted inline height96, no unfolding of named declaration bodies | Existing resolution/identity/maximum-height tests; verify that stopping rule when adding a constructor |
| Binding and retained semantic origins | Bound declaration keys / nominal identities stop ownership; retained inline arguments admitted | Canonical-origin/parameter-position and actual supplying-projection tests |
| New specialization materialization | Combined descent256, new-materialization64, instances4096, monotone work/storage and prospective inline96 admission before copies; exact active reuse checked first | Existing growth/cache/pre-copy/composed-height/partial-failure tests; these limits are distinct from arbitrary nominal graph length |
| Source syntax before admission | Token groups128 and parser descent192 before recursive structural assembly; normalized inline height96 | Existing malformed/max-height admission and source-part diagnostics |
| Generated syntax, token/AST clone/format/map/drop | Existing syntax worker, bounded inline construction; flat field breadth does not manufacture deep proof ASTs | Maximum-inline, broad-field real-call and worker-start/publication-failure tests; no arbitrary total output/heap promise |
| Admitted Module/ResolvedModules clone/Debug/drop | Owned inline trees are admitted; names/keys do not recursively own other declarations | Maximum-expression success/error ownership tests, with streamed formatting and no retained duplicate model |
| Public detached model spelling and ordinary Rust traits; direct Rust macro inputs | Caller-owned inputs do not inherit sealed IDL admission | Outside the nominal-graph guarantee; no arbitrary-depth Clone/Debug/Drop or host Rust compiler stack promise |
| Core COM value conversion | Shared ValueBudget charged before descent; exact/max-cost preflight proofs retain active validation and typed partial owners | Existing value/buffer/recursive consumer tests; defaults depth64, 8MiB, 1M values; permissive custom budgets remain caller policy |
| NCB protocol | Fixed128 open values checked on entry, in addition to configured limits | Existing fixed-buffer and format tests; no compiler graph state or allocation added |
| JSON | Logical entry bounded before another frame; parser syntax budget before child parse; writer completed syntax checked before byte emission | Existing numeric/path/format tests; writer tagged framing may temporarily exceed syntax128 but is bounded by logical depth and fixed framing overhead |
| Host persistence | Capture checks the caller’s object limits before descendant callbacks and codec selection. Restore validates all recipes before postorder construction. | Safe and raw save and restore tests exercise these rules. Capture uses its own limits and consumes native stack space. |
| Host dependency planner and runtime query | Iterative dependency ordering; flat distinct contract sets and compact tables | Existing planner/query tests; no runtime schema retention or shared compiler graph |
| Generated application values, provider callbacks, trusted tool source-tree copying | Ordinary application ownership/caller policy and trusted fixture traversal | No universal arbitrary-depth destructor/callback/filesystem guarantee is implied |

The inventory is a maintained ownership/validation map, not a proof from a text
search that every possible Rust call graph is bounded. Runtime logical budgets,
format framing, compiler expression height, nominal graph length and rustc trait
proof recursion are separate. No native ABI version, IDL-v14 fingerprint,
NCB-v1 byte, callback ordering or caller-limit policy changes follow from this
traversal repair. Windows CI runs the native small-caller selections explicitly;
a Linux result alone is not evidence of Windows execution.

## Closed generic values and interfaces

Structs, enums, and interfaces can declare type parameters. Each use must supply all arguments:

<!-- checked-example: generic-stream -->
```ncidl
struct Snapshot<T> { value: T }
interface IValue<T> {
    fn get(&self) -> Result<Snapshot<T>>;
}
interface Counter: IValue<u32> {}
interface Counters {
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
instantiation is bounded by one generated-work allowance for the complete module
batch, in addition to materialization-depth and instance limits.

Template parameter names are local binders. Fingerprints encode their zero-based
positions, so `Pair<T> { value: T }` and `Pair<Item> { value: Item }` have the same
signature when authored under the same qualified name. This applies through
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
nil UUID with `EIDL074` and the canonical `IUnknown` UUID with `EIDL075`.
Every owned COM object supplies the canonical `IUnknown` contract. Nominal
value IDs follow a separate policy and may still explicitly use nil.

### Interface loans and return projections

```rust,ignore
interface Catalog {
    fn find(&self, id: u64) -> Option<ComPtr<IItem>>;
    fn item(&self) -> Option<&IItem>;
    fn item_mut(&mut self) -> Option<&mut IItem>;
    fn use_item(&self, item: Option<&mut IItem>) -> Result<()>;
    #[scoped]
    fn tooltip(&self) -> Result<Option<&INcString>>;
}
```

Shared `&I` and exclusive `&mut I` lower to `ComRef` and `ComMut` in
generated Rust. Optional loans admit a both-null pair. An ordinary borrowed
return carries the borrow of its receiver after native return. A mutable
receiver that returns a shared loan still stays mutably borrowed. A shared
receiver cannot issue an ordinary mutable return. Generated providers use
the authored access of receivers, including inherited methods.

A `#[scoped]` method delivers an optional loan through a synchronous
visitor. The generated Rust provider invokes a higher-ranked continuation
while its lender is active. The declaring marker's `with_...` helper
requires the proper live receiver loan and passes the returned view to the
visitor. First-base prefix witnesses preserve the original physical view.
The visitor result cannot carry the loan beyond the callback.
Open interface templates check the outer result shape even when unused.
A bound template parameter can supply the result or the payload of the
bundled `Option` after substitution. The closed method checks the resulting
interface target and access rules.
Receiver-bound and scoped loan results occupy methods with borrowed or copied
inputs. Methods that consume boxed inputs or write to output buffers use a
separate call so their transfer and publication rules stay explicit.

A required `ComPtr<I>` result transfers one strong owner. An optional
transfer uses both-null for successful absence. A boxed result transfers
the sole owner. A borrowed result transfers no ownership. Native success
initializes the output slot. Shape and exact identity are checked before
forming a typed loan or adopting a transfer.

Direct optional owned and visitor results project failure as absence.
The new ordinary borrowed and copied direct results panic after a checked
dispatch or decoding failure. Optional absence on those methods is a
successful native value. A checked `Result` projects the actual status.
A direct unit method uses the existing status-only slot. A consuming boxed
method requires `Result` to recover its owners on rejection.

The resolved bundled `neocogi.lang::Option` identifies optional positions.
Stored values and copied options keep their existing enum encoding.
The ABI signature records direction, access, lifetime form and optionality.
The Rust binding identity also records the selected status projection.

### Canonical signatures

Unit fields and payloads are checked copied data, including through fixed arrays
and buffers; their wire byte must be zero. Root `Result<()>` still has no output
slot, and bare unit inputs remain excluded. The
[recursive value contract](RECURSIVE_VALUE_CONTRACT.md) defines conversion and budgets.

Templates and concrete declarations share the explicit binary declaration
encoder under `neocogi-idl:module-abi:v16`. The former separate template string
encoding and its Debug-formatted policy values are removed. Both paths encode
parameter positions, resolved free identities, member order, authored enum
assignment presence, and declaration policies. Regenerate bindings and rebuild
all contracts/consumers after adopting this encoding; there is no compatibility
adapter for the old signatures. Module-wide revision checking is retained.
Changing implicit enum values to explicit assignments changes the fingerprint even
when the resolved integers agree; see [contract changes](#contract-changes).

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
Canonical encoding v16 contains ordinary value and callable-interface contracts;
IDL has no event-reference slots. Rebuild affected contracts, providers and hosts
together, including built-in persistence bindings. Nominal IDs, the native ABI version, source-graph schema and the host-owned
runtime registry are separate contracts. Closed generic interfaces work as object
contracts and plugin roots; open templates have no callable ABI.

Generic values can nest and recurse through explicit sequences:

<!-- checked-example: generic-values -->
```ncidl
struct Node<T> { value: T, children: Vec<Node<T>> }

enum Change<T>: u8 { Set(T), Clear }

struct Handle<I> { object: ComPtr<I> }

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
bounded to 64 new specialization levels and 4,096 consumer-local instances across
one module batch. Exact active recursive reuse is checked before admitting a new
instance. Expansion also admits at most 1,048,576 generated structural visits,
32 MiB of cumulative conservative model/scratch storage charges, and 256 combined
active descent levels. Descent composes expression traversal, substitution and
reentrant materialization; entering another declaration does not reset it.
Individually shallow expressions may therefore exceed the combined nesting bound.

The same owner computes prospective normalized inline height before copying a
template or substituting its arguments. A valid template and a valid argument
can compose beyond height 96; that request returns source-located `EIDL060`
with an inline-nesting reason before building the excessive type. The walk
charges generated work and preserves slice normalization, including `&T` when
`T` is an unsized slice. Height is independent of the combined active descent
and materialization/cache limits above.

Copies, identity/origin reads, comparisons and projection work are checked before
the generated operation. Each repeated substitution pays independently, including
phantom arguments. Cache hits consume work; freed transients do not refund storage.
Copied documentation, source-span metadata, names and bound keys are included.
Authored source normalization is outside this generated allowance. Recursively
inventing arguments or exhausting any allowance returns source-located EIDL060,
with a distinct work, storage, nesting, or materialization-depth/instance reason.
Returning the limit diagnostic permits one requesting source-span metadata copy
and fixed text outside the exhausted allowance; it does not copy/render the
failing type, its origin tree or the source corpus.

These are compiler implementation limits, not ABI identity inputs or an exact
process-memory/allocator-success guarantee. Parsing authored input, native Rust/C
compilation and IO have their ordinary costs. After expansion, consumers borrow
admitted finite arguments: canonical admission retains one exact origin per
closed declaration, signatures stop at nominal leaves, and metadata walks named
edges with visited sets per consumer root. Documentation renders the bound
qualified spelling directly; diagnostic closed-type labels truncate explicitly
at 4 KiB. No disposable type AST is constructed just to inspect or render it.
Exact semantic-origin checks, nominal UUIDs/signature bytes and supplying Rust
projections remain independent of these limits.

## Source language

The IDL deliberately resembles Rust without inheriting Rust's unstable native
ABI. The generator owns every binary representation. This complete two-module
example uses a small texture contract as its dependency:

<!-- checked-example: language-render -->
```ncidl
// File: example/render.ncidl
interface ITexture {}
```

<!-- checked-example: language-widgets -->
```ncidl
// IDL root: contracts/idl
// File:     contracts/idl/example/widgets.ncidl
// Module:   example.widgets

extern example.render;

use example.render::ITexture;

struct WidgetOptions {
    enabled: bool,
    width: i32,
    height: i32,
}

interface IImageConsumer {
    fn set_image(&self, image: &ITexture) -> Result<()>;
}

enum FlowDirection: u8 {
    Horizontal,
    Vertical,
}

bitenum WindowOptions: u16 {
    NONE = 0,
    RESIZABLE = 1,
    TITLED = 2,
}

interface IWidget {
    fn enabled(&self) -> Result<bool>;
    fn set_enabled(&mut self, enabled: bool) -> Result<()>;
}

interface Button: IWidget {
    fn label(&self) -> Result<ComPtr<INcString>>;
    fn set_label(&mut self, label: &[u8]) -> Result<()>;
    fn set_image(&mut self, image: &ITexture) -> Result<()>;
}

interface UiFactory {
    fn create_button(
        &self,
        label: &[u8],
        options: WidgetOptions,
    ) -> Result<ComBox<Button>>;
}

interface UiRuntime {
    fn widgets(&self) -> Result<ComPtr<UiFactory>>;
}

interface UiPluginFactory {
    fn create_runtime(&self) -> Result<ComPtr<UiRuntime>>;
}
```

These blocks illustrate the language; their small example contracts are independent
of any external application framework. The marked examples on this page are
extracted from this document, resolved, generated and compiled by
`cargo test -p neocogi-tooling-tests --test documentation`. The
[COM README](../COM.md#idl) adds an executable
implementation and factory bootstrap checked by the same test.

The optional suffix after an enum name selects its integer representation and
allowed discriminant range:
`u8`, `u16`, `u32`, `i8`, `i16`, or `i32`. Omitting it defaults to `u32`.
Representation is IDL syntax (`enum Name: u8`), not a Rust `#[repr]` attribute.
An enum whose variants carry no payload generates a transparent fixed-width
integer newtype with that representation and preserves unknown integer storage.
Checked live COM conversion still validates the declared domain.

An enum with tuple or named payloads generates a native Rust enum. Its foreign
`ComValue::AbiRepr` is a C record with an explicit `i64` tag and recursively lowered
active payload. The suffix bounds the declaration ordinals; it does not narrow
this foreign tag. Native Rust enum layout is not the foreign ABI and is not
promised stable. See [native copied enums](RECURSIVE_VALUE_CONTRACT.md#native-copied-enums)
for the checked wire contract.

<!-- checked-example: enum-representation -->
```ncidl
/// Fieldless modes use the selected integer representation.
enum Mode: u8 {
    /// Disabled mode.
    Off = 0,
    /// Enabled mode.
    On = 1,
}

/// A payload enum has a separate checked foreign wire.
enum Choice: u8 {
    /// No active payload.
    Empty,
    /// Carries one byte.
    Data(u8),
}
```

`Mode` has a one-byte integer wire. `Choice` still has an eight-byte `i64`
foreign tag: `Empty` is ordinal 0 and `Data` is ordinal 1, despite the `: u8`
suffix. Access its foreign record through `<Choice as ComValue>::AbiRepr` and the
checked codecs, independently of the native Rust enum's size or layout.

After generating these declarations into an `enums` module, this check observes
the actual associated wire types and round-trips both payload-enum variants:

<!-- checked-example: enum-wire-check -->
```rust
/// The source suffix and the physical payload tag have separate responsibilities.
#[test]
fn documented_enum_wire_contract() {
    use enums::{Choice, Mode};

    for (native, expected) in [(Mode::Off, 0_u8), (Mode::On, 1_u8)] {
        let wire: <Mode as neocogi_com_sdk::ComValue>::AbiRepr =
            neocogi_com_sdk::encode_inline(&native).unwrap();
        let integer: u8 = wire;
        assert_eq!(core::mem::size_of_val(&wire), 1);
        assert_eq!(integer, expected);
    }

    for (native, expected) in [(Choice::Empty, 0_i64), (Choice::Data(7), 1_i64)] {
        let wire: <Choice as neocogi_com_sdk::ComValue>::AbiRepr =
            neocogi_com_sdk::encode_inline(&native).unwrap();
        let tag: i64 = wire.tag;
        assert_eq!(core::mem::size_of_val(&wire.tag), 8);
        assert_eq!(tag, expected);
        // SAFETY: encode_inline initialized the active wire record. These copied
        // scalar variants retain no borrowed backing storage during decoding.
        let decoded = unsafe { neocogi_com_sdk::decode_inline::<Choice>(&wire) }.unwrap();
        match (native, decoded) {
            (Choice::Empty, Choice::Empty) => {}
            (Choice::Data(expected), Choice::Data(actual)) => assert_eq!(actual, expected),
            _ => panic!("variant changed"),
        }
    }
}
```

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

Rust 2024 strict and reserved keywords, including `gen`, are rejected at the
authored identifier before generation, even in unused templates. Weak keywords
such as `union` remain valid names. Raw identifiers are outside the IDL member
grammar; generation does not add raw prefixes or silently rename members.

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
/// Complete editor value after one committed user edit.
struct TextChanged {
    /// Owned text captured at the edit.
    text: String,
    /// Scalar-aligned UTF-8 byte cursor within that text.
    cursor: u64,
}

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
an ordinary enum and is copied when its payload is copied. Recursively copied
arrays support direct arguments, results, and buffer elements. Every fixed
array reachable through a live value wire must have positive length. Zero-length
arrays remain available for local values and stored NCB sequences.

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

In ordinary tagged value positions, the bundled `Option<T>` follows enum rules.
Copied payloads allow copied arguments/results, and owned payloads require
borrowing the complete enum. Root interface results follow their declared owner
or scoped loan transfer contract.
A method can borrow an entire tagged value with `&Option<ComPtr<I>>`.
Decoding it into an owned value retains a present capability. At a method root,
the bundled `Option<&I>` instead describes an optional interface input or a
borrowed result. An explicit scoped method delivers within its callback. To specialize such a slot, declare `Option<T>` at its root and
supply `&I` as the argument. See [interface loans](#interface-loans-and-return-projections)
and [generic value ownership](IDL_GENERIC_VALUES.md#copied-and-owned-optional-values).
Regenerate contracts and rebuild consumers after signature changes; contract
crates remain ordinary Rust dependencies without shared-library artifacts.

A zero-based unit enum used as a complete table axis may opt into generated
domain metadata:

```rust
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

Ordinary declarations need no identity attribute:

```rust
interface IWidget {}
```

The exact qualified name derives the nominal UUID. An imported ABI, a moved
declaration, or an identity retained during migration can use an explicit UUID:

```rust
#[id("9be73186-f123-47bc-8140-4a57825eeb21")]
interface IWidget {}
```

The explicit UUID does not change the diagnostic name. The example remains
named `neocogi.ui.core.IWidget`. The current `#[id]` spelling takes one UUID string;
duplicate overrides are rejected. Deprecated `#[version(N)]` attributes and the
number in `#[id(N, "UUID")]` are accepted but ignored, with a source-located
`WIDL001` warning for every occurrence. Their integers never enter semantic
identity or fingerprints, even when they differ between consumers.

### Contract changes

Nominal identity and exact contract compatibility have separate authorities.
The qualified name, or an explicit UUID override, identifies the declaration.
Its SHA-256 ABI signature captures the canonical typed module, including
transitive dependency fingerprints. Changing member names, types, order,
ownership, threading, numeric assignments or declaration policies changes the
signature automatically. No manual declaration version or historical-source
comparison is required. Connection boundaries check both nominal identity and
exact signature; matching layout alone is insufficient.

For example, these successive revisions retain the same nominal ID and have
different signatures, even though their resolved integers are identical:

```rust
// Original contract, compiled separately.
enum State: u32 { Empty, Ready }
```

```rust
// Replacement contract, compiled separately.
enum State: u32 { Empty = 0, Ready = 1 }
```

Changes that cannot be expressed in the IDL, such as changing a quantity's units
without changing its field type, are not detectable by hashing. Use a distinct
named type or interface to express that semantic distinction. Duplicate names
in one module are rejected (`EIDL010`); declarations in different modules have
different qualified identities. Keeping an explicit UUID does not bypass exact
signature checking.

Comments, formatting, source-part placement, backend Rust paths, and consistent
renaming of generic parameter binders preserve signatures when their positions
and uses are unchanged. Field, variant and method member names retain their
contract meaning. Independently built plugins may use different supported
compilers and flags, but must agree on the exact transitive contract.

This supersedes the mandatory declaration-version policy from 0.1.0. Remove
`#[version(N)]` and replace `#[id(N, "UUID")]` with `#[id("UUID")]` to clear
deprecation warnings. `Module::warnings` retains structured nonfatal diagnostics
for parser API callers; both Cargo build helpers emit them through
`cargo::warning`. Warnings retain original source-part coordinates and do not
enter generated contract data. Malformed attribute syntax and duplicate explicit
UUID overrides remain errors.

Generated nominal IDs and signatures change: rebuild contracts, providers and
hosts together. Compiler encoding v16 records the versionless contract; COM
discovery uses ABI v2 for SDK 0.3.0, ABI v3 for SDK 0.4.0, and ABI v6 on this development branch. V16 distinguishes borrowed returns, scoped delivery and threading evidence.

Generated values and interface markers carry a hidden Rust binding proof. A
consumer checks the actual selected Rust type, including a type reached through
a facade reexport. The proof combines the native module fingerprint, the Rust
method projections in canonical order, and the transitive dependency proofs.
Closed specializations use the defining template and exact argument proofs, so
independent using modules can agree. The proof is used only while compiling
Rust bindings. Native negotiation continues to use the IID and ABI signature.

### Module names and dependencies

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

Logical module segments use nonraw ASCII identifiers: a letter or underscore,
then letters, digits or underscores. Standalone `_`, Rust 2024 strict/reserved
keywords (including `gen`) and the anchors `self`, `Self`, `super`, `crate` are
rejected. Weak keywords such as `union`, near-keywords such as `types`, and
`_private` remain legal. Source file/directory identity, dotted `extern`/`use`,
qualified type prefixes and logical binding-registration keys share this rule.
Invalid names fail before generation/output publication, including unused
registrations and template references. `r#` is not an alternative logical identity
spelling. Syn may reject a forbidden keyword in Rust-like type syntax before
logical prefix validation; neither route makes that name legal.

Spelling, case and underscores are preserved in logical identity bytes, UUID
inputs and fingerprints. A standalone source module can have one segment;
external module references require at least two. Organizational filenames within
a selected multipart module do not define that module's name. Existing metadata
length bounds still apply where names are transported; the Rust binding path's
bound is not an additional per-segment IDL rule.

Backend Rust binding paths retain their separate grammar: a logical module such
as `proof.types` may map to `api::r#type`. That raw Rust location is valid metadata
and never rewrites the logical module identity.

An authored import of a closed interface also exposes its complete generated
Rust family: `ITexture`, `ITextureAbi`, `ITextureVTable`, `ITextureVtblFor` and
`ITextureWrapper`. The importing module reexports the existing items from its
selected supplying Rust route; it creates no replacement interface identity or
table. For coherent generated bindings, both owner and imported paths name the
same Rust types. Providers can use `#[com_object(consumer::ITexture)]`, implement
`consumer::ITexture`, and query/call through `ComPtr<consumer::ITextureAbi>` or
`ComRef<'_, consumer::ITextureAbi>`. Cargo include entries explicitly forward the
whole family so consumer globs cannot make its named companions ambiguous.
Admission reserves all those public names at the authored import identifier;
conflicts with local or separately imported declarations are source-aware IDL
errors before generation or publication. Native value imports expose one name.
Open-template imports are consumed by specialization, including unused imports;
closed instances keep their existing consumer-local generated family.

### Multi-file module composition

Large contracts may place declarations in any number of `.ncidl` files below
one module directory. Every file is parsed independently and then merged before
external dependencies and imports are connected or types are resolved.
References therefore do not depend on filenames, declaration placement,
filesystem enumeration order, or a hand-maintained part list.

The merged model is canonicalized as follows:

- source parts are retained by normalized relative path for diagnostics and
  generated source provenance;
- external dependencies are sorted by their canonical module name;
- equivalent imports are merged, and imported modules and names are sorted;
- top-level declarations are sorted by their exact identifier;
- field, enum-case, base-interface, method, and parameter order is retained
  exactly because those sequences are ABI-significant.

The canonical ABI encoder excludes part paths, comments, and formatting.
Reordering the same source parts preserves deterministic generated output.
Within the same logical module, moving or renaming parts, splitting a file, or
combining files without changing the contract preserves the module fingerprint
and executable binding behavior. Generated origin comments and source-map
locations follow the authored source, so complete Rust bindings need not remain
byte-identical. See the [source-map guide](../guides/IDL_SOURCE_MAPS.md).
Changing an ABI-significant member sequence still changes the fingerprint.

### Type and ownership rules

- Fixed-width integers, `f32`, `f64`, ABI-defined `bool`, and `Status` are
  value types. `Status` preserves a raw `NcStatus`, including extension codes;
  method failures continue to use the implicit error side of `Result<T>`.
- A bare record or enum parameter is copied only when its complete recursive
  shape has self-contained wires. Owning named values use `&Value`; providers
  receive `NcValueRef` and explicitly acquire data to retain it.
- `&Interface` and `&mut Interface` lend shared or mutable call access,
  respectively. `Option<&Interface>` permits absence.
- Owned object results require an explicit ownership wrapper such as
  `ComPtr<Interface>` or `ComBox<Interface>`; a bare interface is not a value.
- `&str` lowers to a readable byte view for text. Providers validate UTF-8 when
  acquiring text. Stored `String` is owned UTF-8 data; an owned COM string object
  uses the explicit `ComPtr<INcString>` capability.
- `&[T]` is a borrowed contiguous input slice whose element type is ABI-safe.
- `&mut [T]` is fresh caller-owned output storage filled synchronously by the callee;
  old contents are never input. Providers receive `NcOutputWriter`, callers pass
  `NcOutputBuffer::output`, and only the reported prefix is acquired. A real
  transformation takes explicit readable input plus separate output. This does
  not grant mutable access to an object or retained collection. See
  [directional copied output](COM_OUTPUT_BUFFERS.md).
- Every call returns `NcStatus` at the ABI. Direct and fallible unit calls have
  no output slot. Scoped borrowed results deliver through a continuation.
  Other results initialize an output slot on success.
- The bundled `neocogi.lang` `Option<T>` follows ordinary enum rules in value
  positions. At method roots it can declare optional COM slots, as described
  in [interface loans and return projections](#interface-loans-and-return-projections).
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
fingerprint. `#[persistence(sequence)]` is rejected: declaration shape selects a named
record or an ordered tuple independently of the chosen format. The old `#[deserialize]`/`if_supported`
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
Bundled modules such as `neocogi.lang` participate through ordinary IDL references.
A project `.module` or `.import` route claiming a bundled module fails with
`EIDL067` before source discovery or output changes.
Both routes resolve shared IDL and compute the normal transitive ABI. Every external
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
borrowed inherited wrappers use `<BaseAbi as ComRustInterface>::Wrapper<M>`.
The checked marker therefore selects both representations, even if a facade
also exposes an unrelated old `BaseVTable` or `BaseWrapper` convenience name.
Each direct base provider's `with_identity` constructor must produce the
vtable type associated with the checked marker. The resolved ancestry supplies
each inherited descriptor and provider entry through the same direct parent
and index. Those entries must agree on the complete interface identity.
Public trait/Abi/VTable/VtblFor/Wrapper names remain available. No extra query,
reference-count operation, core/compiler dependency or runtime graph is added.

A custom Rust facade selected to supply an authored closed-interface import must
expose all five public names, even for an otherwise unused import. Previously
partial supplying facades must add the missing original exports. This does not
apply to an unrelated handwritten trait alias used directly by an object macro;
such a path must supply the marker/provider names that the macro derives beside
it. The IDL compiler does not resolve arbitrary Rust aliases or inspect missing
Rust items; unavailable supplying names cause Rust compilation errors.

Availability does not certify arbitrary sibling aliases. A handwritten facade
may expose a current checked marker alongside older unused named Wrapper/VTable
items. Those aliases are forwarded as supplied; inherited code still uses the
checked marker's associated representations. Marker identity checks do not
claim that every independently authored sibling is coherent. Whole-family Rust
type equality is guaranteed for coherent generated supplying routes.

Add other
subsystems with `.include_root("../../ui/schemas")`. Every reachable module must have
exactly one source owner across all roots; root order cannot shadow a definition.
Repeating a canonical root is harmless. Source maps preserve the actual owning root.
Unmapped imports, conflicting Rust locations/local-import policies, and simultaneous
file/directory owners fail the build. Rerun directives track source parts. Generated
filenames use existing logical module UUIDs to avoid case/separator collisions and
remain bounded; callers use each returned include path rather than guessing names.

Both Cargo builders complete every requested module's generation and source-graph
encoding before output materialization. Complete Rust-path and generator-worker
errors therefore preserve prior output, including navigation copies. Physical
source-map finishing is a separate fallible phase: all maps are resolved and
serialized before any main binding/map/include or graph is overwritten. Bundled
navigation copies and directories may change during finishing. Final publication
uses write-if-changed and can be partial on IO failure; neither phase promises a
filesystem transaction. Success metadata/results follow the complete publication.

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

For incremental builds, keep the watched directories input-only. Automatic
discovery watches a missing file or directory candidate's nearest existing
parent so adding a competing source reruns the build. Cargo also sees build
writes beneath that directory. Use a dedicated schema subtree or direct Cargo's
build output outside the watched directories. When a watch contains the
current `OUT_DIR`, the builder emits one warning with the paths. Source
selection still checks every candidate.

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
Cargo-specific routes are selected by `Module::build`. The packaged builder
parses its own source once to select its `links` owner and dependency records,
then uses that parsed module in resolution. The raw source record is still
checked during incoming graph merges. Source graph schema 7
requires providers with this namespace and rejects older sidecars. Runtime and
plugin discovery and the core baseline follow the [ABI version table](../abi.md#abi-identities-and-versions); source routes do not change
the [canonical signature encoding](#canonical-signatures).

### Diagnostics

Parsing, module merging, name resolution, and type checking use structured
diagnostics with stable `EIDL` codes. Each source label retains its normalized
path and exact range. Duplicate declarations, fields, enum cases, bit flags,
methods, parameters, external dependencies, imports, discriminants, and bases
identify both the later occurrence and the original occurrence. Type-category
errors identify the use and the declaration that establishes the actual
category. Each label can retain the exact source it points to, independently of
the compiler graph and other diagnostics. When two authored inputs claim one
logical module and path, duplicate-module labels retain the two input excerpts.
Construct handwritten labels with `DiagnosticLabel::new` and inspect an
attached file with `DiagnosticLabel::source`.

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

`WeakComPtr<I>` uses ownership tag 15. Shared strong references use 9 and
exclusive owners use 12. Tag 13 is reserved. Closed generic identities encode
the declared ownership category.

## Implementation and plugin publication

Shared implementations constructed with `into_local(value)` support
`LocalCom<T>::downgrade() -> Result<WeakLocalCom<T>, NcStatus>`. Promotion returns
`Result<Option<LocalCom<T>>, NcStatus>` and acquires an ordinary strong owner
before typed borrowing. This concrete Rust access remains in the constructing
binary; IDL names only its interface projection as `WeakComPtr<Interface>`.

For example, a contract can return `Result<WeakComPtr<Service>>`, or borrow a
record containing `observer: WeakComPtr<Service>`. Nested records, arrays,
sequences, options and closed generic fields preserve weak ownership. Acquisition
checks presence and exact interface on producer-guaranteed shared backing without
promotion, so expired
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

A plugin defines a typed acquisition function taking the host-supplied `RuntimeHandle`
in its implementation crate:

```rust,ignore
#[neocogi_com_sdk::plugin(name = "example.plugin")]
fn acquire_root(_runtime: neocogi_com_sdk::RuntimeHandle) -> Result<neocogi_com_sdk::ComPtr<api::IRootAbi>, neocogi_com_sdk::NcStatus> {
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
There is no `contracts` list. Plugin descriptor ABI v6 contains the full factory
identity, threading capability, bootstrap pointer, plugin metadata, and provider
IDs. Its native getter is `nc_get_plugin_descriptor`.

Dynamic loading validates and registers one plugin atomically. Providers must
already be registered; registration invokes neither factories nor dependency
loaders. Root acquisition then checks the requested signature before bootstrap.
A monolith links plugin rlibs, registers their Rust descriptor getters directly,
and uses the same acquisition operation against the one implementation explicitly retained by its final monolith owner.

## Tool ownership and migration

The compiler lowers the bound graph directly into typed method and value plans.
The same library's `neocogi_idl::codegen` module emits provider traits, C wires/vtables, method wrappers, thunks,
metadata and persistence implementations from those plans. Generated contracts
contain complete Rust instead of interface/value attributes whose expansions
would have to recover ownership by parsing Rust types again. Core depends on
neither compiler nor procedural macros; the explicit interface-only bootstrap emits its
checked-in persistence ABI and the facade's provider half from the same plan.

Copied native records have ordinary Rust application layout. COM owns their
explicit C wire layout and checked conversion. Direct borrowing still requires
the codec's complete `BORROW_ABI_REPR` size, alignment, offset and bit-validity proof.
It cannot infer identity from the application's representation attribute.

Generated contract definitions need no procedural macro feature. A package that
also compiles concrete Rust objects, including handwritten event snapshots, must
request `neocogi-com-sdk/macros` in its own normal dependency. Its name ending in
`contracts` does not make those implementations generated-only code. Enable `alloc`
for owned values and `std` where required by local code.

- `neocogi-idl` owns parsing, semantic validation, and canonical fingerprints.
- `neocogi-idl` owns source discovery, graph exchange, and Rust generation.
- Each consumer’s build script generates its selected IDL; Cargo builds applications/plugins.
  Its IDL tools can inspect complete source declarations without shipping them.
- `neocogi-inspect` reads plugin, dependency and factory metadata in
  [inspection schema 11](../../tools/neocogi-inspect/README.md). It compares exact
  factory signatures and plans provider load order.

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
ABI v6 and the stable `nc_get_plugin_descriptor` getter. No legacy adapter retains the
removed runtime contract mechanism.

### Generated IDL metadata

[`neocogi_com_sdk::idl_metadata`](../../crates/com/src/idl_metadata.rs) contains
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
Factory results transfer the sole owner. Container parameters consume it.
A container can lend from a child field:

```rust,ignore
interface IContainer {
    fn insert(&mut self, child: ComBox<IWidget>) -> Result<()>;
    fn child(&self, index: u32) -> Option<&IWidget>;
}

interface Widgets {
    fn button(&self) -> Result<ComBox<Button>>;
}
```

Owning and borrowed positions have distinct canonical fingerprints and
inspection passing modes. Boxed ownership transfers follow method slots
rather than repeatable value fields. A
consuming Rust parameter returns its original owner on failure. IDL derives the
Rust error tuple from the parameters: `Result<R, (NcStatus, ComBox<I>)>` for one
owner, or `(NcStatus, owner1, owner2, ...)` for several, in parameter order.
Success consumes every owner; rejection preserves them and the destination.
The raw slot is null on success and unchanged on failure. Forwarding providers
propagate the complete returned-owner error. There is no public transfer wrapper.
Shared `ComPtr` inputs retain their existing prohibition; borrow services with `&T`.

Checked owner results and admitted owner inputs use the allocation's canonical
table after independently validating the incoming IID/full digest. Input lookup
resolves that table once without changing the caller's slot; adoption waits for
all argument checks and receiver admission. The implementation must return the
same admitted canonical owner on rejection. Generated thunks verify it, then
restore the original caller pair, including any valid equivalent copied table.
This preserves exact rejected slots while canonicalizing the implementation's
owner. Unchecked unsafe raw transfers retain their caller-proven table pairing.

Consuming methods may combine owners with checked writable slices and copied
results. Generated callers and providers reserve conversion storage and admit
the maximum possible output cost before invoking the implementation. The shared
COM macro requires an allocation-free `MAX_COST` for each converted output codec;
there is no separate IDL budget calculation. Excessive known output limits reject
at preparation and return the owners. Non-consuming methods keep actual-output
budgeting. See [prepared output completion](COM_OUTPUT_COMPLETION.md) for an IDL
example, enum limits, and real generated-call regressions.

The generated interface declares its minimum threading capability. Each Rust
handle carries the capability that applies to its access.

| Handle | Where its capability comes from |
| --- | --- |
| `ComBox` and `ComPtr` | The provider's class when constructed, or the declared target of a transferred owner. |
| `WeakComPtr` | The shared owner it observes. |
| `ComRef` and `ComMut` | The handle they borrow from, or the declared target of an interface argument or result. |

See [threading capabilities](../abi.md#threading-capabilities) for each handle's transfer and sharing rules.

## Interpreting generated names

| Generated spelling | What it represents | How to interpret it |
| --- | --- | --- |
| Cargo `nc_contract_<hex>` | Exact dotted logical IDL module name | Strip the prefix, decode hex pairs, read UTF-8; dots, underscores and case survive. |
| `__nc_dependencies::module_<UUIDhex>` | Build-generated typed route to a transitive owner | The existing logical module UUID names the alias; generated comments retain the module name. This re-exports its Rust types and does not create a runtime identity. |
| Local `<module_with_underscores>.rs` and `__nc_idl_<stem>` | Package-local output/module name | Dots become underscores, so this spelling alone is not reversible. The generated entry names the original module explicitly. |
| `NameInstance<hex>` | Nominal identity of a closed generic IDL type | Decode hex to UUID bytes only. Read the generated closed-type comment for the template and arguments; the UUID cannot recover them. |

Cargo ownership keys are never native library filenames. Specialization UUIDs
are never ABI signatures. Generated comments keep the original logical module
or closed type beside these names. Their spelling and build-sidecar schemas evolve independently of
the native ABI version. See
[generic name documentation](IDL_GENERIC_VALUES.md#reading-generated-specialization-names)
and [source-map lookup](../guides/IDL_SOURCE_MAPS.md).

Value persistence traverses the [COM format interfaces](SERIALIZATION_FORMATS.md).
Named records supply field names and payload enums supply case names and ordinals.
NCB remains positional; named formats may select fields independently of encoded
order. The explicit default policy retains genuinely absent fields. The value
traits use format-neutral borrowed facades, while live COM wire validation and
object reconstruction retain their separate contracts.
