# COM widget and application contracts

These are the author-facing rules for the component architecture defined in
[COM_DESIGN.md](../architecture/COM_DESIGN.md). The contract crate contains fixed interfaces and
values; the application-facing Rust facade contains typed conveniences. Neither
exposes implementation state through `Any`, untyped properties, or native Rust GPU
objects.

## Linking and ownership

Native applications always link the Rust facade. A dynamic assembly links one shared COM runtime and loads the UI, GFX compositor, selected renderer, and optional Skin plugin. A monolith instead links one runtime implementation plus those component rlibs and registers their descriptors directly. Browser builds use the same static registration model. Never link a separate runtime implementation into each dynamic plugin. Loaded or statically registered plugins remain active for the process lifetime; this design does not support hot unloading.

Shared services use `ComPtr`; widgets use one non-clonable `ComBox`. Containers
consume child owners. Application fields retain cached `WeakComBox` observers,
which cannot prolong a removed value. Dropping the owner rejects new calls;
Drop waits for admitted calls and completes provider destruction before returning. Final destruction releases
ordinary fields without a generic widget close callback or recoverable teardown
result. It must not submit GPU work or invoke application behavior. See
[exclusive ownership](EXCLUSIVE_WIDGET_OWNERSHIP.md).

Initially each widget mounts once, under one parent in one runtime. Attachment checks duplicate membership, cycles, and owner compatibility before publishing an edit. Container track/span settings belong to the parent-child relationship. Do not place them in a generic widget property map. Foreign-runtime widgets and foreign-renderer resources are errors, even if their numerical IDs happen to match.

IDs are scoped to their owner. A static counter compiled into several shared libraries is several counters. Do not assume it provides process-wide identity. QueryInterface's canonical IUnknown projection identifies a live COM object; arbitrary interface-header addresses can differ on the same object.

Default UI and GPU pointers are thread-local. Atomic reference counts do not make UI methods, OS windows, or GPU contexts thread-safe. Do not strengthen a threading marker without the corresponding implementation guarantee, including final destruction on another thread.

## Update, measurement, and external state

The update result describes geometry: preferred measurement or child-placement inputs changed, or they did not. It is not a generic "something happened" flag. A widget may emit an event, change hover/focus painting, or update a paint cache while reporting no geometry change.

The runtime has additional independent invalidation sources:

- Programmatic setters record sticky geometry changes. Several edits before update may coalesce, but the change must survive until the runtime observes it.
- A different measurement constraint, including a resize, invalidates the corresponding cached measurement.
- A skin/font/atlas revision can change glyph, border, icon, padding, or other measurement inputs.
- Child insertion/removal and parent-owned track/span changes invalidate ancestor measurement and placement.
- A descendant's pending change must be collected before accepting a parent's cached result, including when that descendant's update was skipped.

Widget authors must report changes honestly. A no-change result cannot detect mutations to application-owned state read behind a separate shared reference. If a model, shared string, image geometry, font choice, or other external input changes without going through a widget setter, the author must explicitly synchronize the widget and request geometry invalidation before relying on layout. The runtime does not inspect arbitrary user state or infer its dependencies.

For example, changing a button from "Save" to "Save all documents" changes its preferred width. Its setter marks geometry dirty. Changing only a caret blink phase normally affects paint and does not change preferred size. A model containing the button's future text changing elsewhere is invisible until the application applies the new label or explicitly invalidates its measurement.

Do not rely on hidden, clipped, disabled, or otherwise ineligible widgets receiving an update call on every application frame. Update eligibility follows routing/traversal policy, not a timer service. Keep required model synchronization in application code, and carry its geometry invalidation until the runtime collects it. A widget returning no change does not suppress descendant updates or automatically certify the entire subtree's cache.

Call the runtime's update/synchronization operation after programmatic changes and resize before using layout results or rendering. Painting is observational: it may update private rendering caches, but it must not change semantic state, topology, focus/capture, measurement inputs, or publish application-coordination events. A missing or invalidated update commit causes rendering to report that update is required.

Measurement constraints and allocated rectangles have different meanings. Unbounded is different from bounded zero. A child reports desired size under constraints; the parent then assigns an exact rectangle. Children must not silently reapply a sizing policy to that final allocation.

## Borrowed calls, paint, and layout

Generated interfaces have one safe calling convention. `ComPtr<IFooAbi>`,
`ComBox<IFooAbi>`, `WeakComBox<IFooAbi>` and `ComRef<'_, IFooAbi>` lend the generated
`IFooClient` view, so callers use
`pointer.method()` without importing an interface-specific extension trait.
Derived client views dereference through their declared base views for inherited
method lookup. An owned base projection still requires checked `QueryInterface`;
method lookup alone never manufactures another COM reference.

`ComRef` borrows a live interface for a call. A consumer accepting a generic
borrow must use `try_add_ref()` when retaining shared ownership and propagate
failure. There is no infallible borrowed `add_ref()` convenience. A caller already
holding a `ComPtr` clones that proven shared owner; an exclusive borrow returns
`NC_E_INVALID_ARG` from fallible acquisition. Exclusive widgets reject shared retention. Capture weak
identities from `ComBox::downgrade`, `ComBox::query_weak`, or an existing
`WeakComBox`; a generic borrow does not establish an exclusive lifetime layout. A generic `ComRef` therefore
does not promise a non-retainable Rust borrow. Never create a COM
implementation containing a stack `&mut Painter`, borrowed child storage, a
native frame borrow, or a borrowed application `&mut State` and assume a ComRef
prevents it escaping.

Paint/layout services own their storage and issue invocation tokens. The static `Painter<'a>`/layout convenience wrapper hides the token from ordinary drawing calls, but the implementation checks it on every ABI operation. The token expires on every exit, including errors. An old token never becomes valid again in a later frame. Retaining the object remains memory-safe and retaining its expired authority does not permit further painting/layout.

Scoped layout permits a container to measure/place its indexed children. It does not permit topology edits during layout or unrestricted access to the runtime tree. Recursive child calls restore the enclosing scope correctly before continuing the parent. Applications and plugins must not fabricate authority or treat an invocation token as a durable resource identifier.

Use `&mut self` for methods that change object state and store that state in ordinary fields. The generated object-wide call gate permits overlapping shared calls and returns `NC_E_BUSY` for conflicting mutable or recursive entry before it creates a Rust reference. Interior mutability is reserved for genuinely shared infrastructure such as runtime registries and asset caches; release those short borrows before calling an external widget, container, painter, or application handler. Panics are implementation faults and follow the COM abort policy; they are not converted into recoverable statuses.

The same rule applies outside COM objects. Event-loop drivers and other values
with one clear owner use `&mut self` and ordinary fields. `Cell` or `RefCell` is
appropriate only when an interface requires shared observation, ownership is
genuinely shared, or state must be detached before a reentrant foreign call; the
field comment must name that constraint.

## Typed events and application state

A persistent `events::Dispatcher<Processor>` owns ordinary application state and a
standard `std::sync::mpsc::Receiver<Processor::Message>`. Use `events::channel()` or implement the common `IEventReceiver`, and pass
the same projection when constructing observed widgets. Implement `MessageProcessor::handle(&mut self, message)`
with ordinary matching and retain the dispatcher's checked `IApplicationDispatcher`
projection across frames.

`MessageProcessor` defines the behavior and owns its ordinary state; `Dispatcher`
owns that processor and its mailbox. The UI runtime schedules synchronous delivery
through the existing mutable gate. These names describe message processing and
ownership without imposing model/view/controller roles.

```rust
let (queue, messages) = neocogi_com::events::channel();
let button = ui.widgets().button_with_queue("Open", Some(queue.as_com_ref()))?;
let picker = ui.file_dialog_with_queue(window.as_com_ref(), Some(queue.as_com_ref()))?;
let app = neocogi_ui::events::Dispatcher::into_local(
    neocogi_ui::events::Dispatcher::new(processor, messages),
);
let owner = app.query::<IApplicationDispatcherAbi>()?;
// Repeat these two calls each frame, retaining app and owner.
ui.update_with_events(drawable_size, owner.as_com_ref())?;
ui.render(drawable_size, renderer.as_com_ref())?;
```

The processor owns everything its deferred behavior needs. Use `app.try_borrow()` or
`try_borrow_mut()` for typed local access between updates; release those guards
before dispatch. They use the same COM borrow gate as foreign interface calls,
without an interface query or a second mutable state container. Frame dispatch
borrows an existing capability and creates no callback/revocation adapter.
The demo host performs this setup automatically for `Host::run(state, receiver)`.

Use `app.downgrade()?` when another local owner needs a non-owning link to the
processor. It returns `WeakLocalCom<Dispatcher<Processor>>`; `upgrade()?` returns
an optional strong local owner without borrowing the value. The promoted owner's
`try_borrow` methods enforce the same gate, and final strong release expires the
weak handle. Concrete local types stay inside Rust; IDL transports shared weak
interface references as `WeakComPtr<I>`. See [typed local ownership](../../crates/com/api/README.md).

For coalesced publication, implement `MessageProcessor::apply_pending_changes` (or the
host's `AppState::apply_pending_changes`) and return `ApplyOutcome::ChangesApplied`
only after consuming pending work. Return `NothingPending` on an idle call. The
hook runs when no message is available, including after sender disconnection,
and can run several times per update. It runs under the same mutable gate with no
traversal borrow. Geometry edits still need ordinary notifications; renderer work
belongs after the completed update. An error preserves already committed edits;
the processor decides whether an uncompleted request remains available for retry.

The generated `IApplicationDispatcher` version 3 returns `ApplicationDispatch`:
`MessageHandled`, `PendingChangesApplied`, or `Idle`. Applied pending changes
request another private-dispatch/layout pass even without a geometry notification.
This distinction avoids treating a newly posted child message as an idle boundary.

Version 3 also provides `input_boundary(InputBoundary)` for ordered Begin/End
bookkeeping around one actually consumed input and its settled messages. Keep
geometry edits and new raw input outside this hook. If commands are decoded
later, retain markers in their event FIFO and copy each command's input identity
before consuming subsequent markers. Native polling is not this ownership boundary.
See the [dispatch contract](WIDGET_EVENT_DISPATCH.md) for failures and budget rules.

Separate dialogs can bind their same protocol to different application destinations.
When reusing one dialog across requests, request identity remains application/domain
policy; binding does not manufacture an invocation ID absent from the protocol.

Declare cross-COM facts as immutable interfaces derived from `IEvent`; pass the
common `EventBinding` into the factory. The behavioral parent creates its receiver
before construction. Ordinary widgets retain that receiver in `WidgetState` and post
through their existing host; the runtime supplies the current weak origin. See
[owner-scoped origins](WIDGET_EVENT_ORIGINS.md) for authority, composite forwarding
and producers that must also emit outside widget dispatch.
Exclusive widgets use `#[com_object(Contract, ownership = exclusive)]`; shared services
retain the default ownership policy. No later binding operation exists.

Construct the concrete snapshot and move it into `state.post(event)` through
mutable producer state. Unobserved instances discard the owner immediately.
Native update algorithms return errors directly, and record committed geometry
before fallible snapshot/delivery work. For recoverable domain failures, the common
`events::error(status, message)` reports through the same queue and source stack;
transport failures still propagate a status. See the
[failure contract](COM_EVENT_INTERFACES.md#reported-failures).

| Event interface | Producers | Captured data |
| --- | --- | --- |
| `IActivatedEvent` | Buttons, list rows, menu commands | Marker; source identity is in the envelope |
| `ICheckboxChangedEvent` | Checkbox | Checked state |
| `INumericChangedEvent` | Slider, Number | Finite value |
| `IScrollbarChangedEvent` | Scrollbar | Nonnegative offset |
| `ITextChangedEvent`, `ITextSubmittedEvent` | Textbox, TextArea | Immutable text; changed also carries byte cursor |
| `IChoiceChangedEvent` | Combo | `ChoiceSelection` |
| `IFileAcceptedEvent`, `IFileCancelledEvent` | FileDialog | Exact native path or cancellation |

Commit state before constructing and posting the `ComBox` snapshot. Posting is
mutable and consumes a concrete event; an absent destination drops it immediately. The receiver captures only the weak origin and event owner;
application behavior reads getters after the emitting call releases its mutable gate.
Built-in constructors validate domains. A consumer must check domain restrictions it
relies on when reading an independently implemented event.

`#[events(...)]` advertises the emitted interfaces. `IDL_INTERFACE.events` describes
their identities and actual getters; catalog registration validates the reachable
value graph. The loader checks observable keys before construction and the advertised
interface after it. See [event metadata](IDL_EVENT_METADATA.md).

`*_with_queue` passes `IEventReceiver`; `*_unobserved` chooses permanent non-delivery.
There is no later attachment API. Windows, popups, menus, built-ins and plugins use
this same construction discipline. Widget destruction releases its receiver.
Shared window/popup/menu services can use `EventPort::close`. Queued facts survive
source removal. The standard channel returns `NC_E_CLOSED` with the event owner
on disconnection; other receivers choose their own capacity/error policy. Delivery
errors do not roll back committed state. See [COM event objects](COM_EVENT_INTERFACES.md).

Behavioral composites bind their children to queues targeting a private typed
channel, then own the receiver and model as fields of the concrete widget.
Advertise optional `IWidgetDispatcher` on the same exclusive allocation, implement
its `dispatch_next(&mut self)`, and mount the actual widget.
There is no separate event-owner identity or second mount/retire lifecycle.
Mutate local fields through `&mut self`; call distinct children through COM.
Passive structural containers such as `ScrollArea` omit `IWidgetDispatcher`
and need no inbox merely because they have children. Removing a subtree drops its exclusive owner and stops new dispatch. An admitted
call returns before its own value is destroyed; no close retry queue is needed.

The runtime schedules widgets automatically, including in `Ui::update`.
It settles child messages and layout before dispatching an application message,
then settles any resulting private work before continuing. Authors need no private
dispatch loop, child registry or update-cycle object. The [events guide](../../crates/ui/runtime/docs/EVENTS.md)
defines the full descendant-first schedule, destination ownership, work budget,
reentrancy and failure rules. Posting does not itself request a platform frame.
Layout/paint remain observational, and asset installation belongs after update.

The same transport serves ordinary-Cargo plugins and static monoliths. Plugin
descriptor and runtime table ABIs remain v1. Generated bindings depend on COM
alone; regenerate contracts and rebuild consumers together after wire changes.

Layout settlement does not repeat a widget's ordinary behavior for the same input.
Requests such as caret reveal, made after an editor's ordinary update, remain pending
until its next eligible update; hidden editors retain those requests. Authors must
distinguish geometry invalidation from behavior that needs another update and retain
such requests explicitly. Shared external state still requires explicit synchronization.

The [events guide](../../crates/ui/runtime/docs/EVENTS.md) gives a complete small example;
the [independent extension](../../examples/ui-extension) exercises the same generated
protocol through both ordinary-Cargo plugins and static monoliths.

## Paths, buffers, resources, and errors

An immutable COM string is UTF-8 display text. It is not a native filesystem path. Native path capabilities distinguish Unix bytes from Windows UTF-16 code units, and the static facade converts those to `Path`/`PathBuf`. Preserve exact native selection independently of its potentially lossy display label. Directory reads and asset replacement validate their complete input before publishing a new live state.

Copy buffers are typed, borrowed for one call, and define explicit offset/count semantics. A whole-string replacement requires offset zero, validates UTF-8, and copies before returning. Validate bounds and overflow before writes or allocations. Immutable returned snapshots own their storage and release through their producer's COM reference counting.

The complete resource, replacement and authoring contract is documented in [COM_ASSETS.md](COM_ASSETS.md). Obtain the current bundle through `IWidgetHost::assets` and refresh any independent widget cache when its identity changes. Logical asset metadata and font tables may be cached per immutable bundle revision. Do not introduce a COM call per glyph or per vertex. Rendering crosses the boundary in bulk, ordered frames/batches; resource references are typed capabilities with validated renderer ownership. Preserve source ordering across ordinary UI geometry and custom render items.

Native GPU integrations belong in the same platform/render plugin as their backend. Rust `wgpu::RenderPass`, `CommandEncoder`, Ash owners, SDL windows, and `Rc<glow::Context>` do not become ABI-safe because the application facade is generic. Keep the backend-specific Rust integration local and expose concrete scene/mesh/camera/render-item interfaces outward. OS-window/context lifetime must cover GPU resource destruction.

A frame finishes explicitly and reports submission/presentation errors. Dropping an unfinished frame abandons it. A successful upload/replacement is published only after validation and backend preparation succeed. On failure, the previous valid bundle remains active. NcStatus reports the recoverable failure category; preserve useful backend diagnostics through the documented diagnostic capability.


Window capabilities expose current manager state through copied snapshots. A child inherits its family root's fixed layer; changing a managed child/dialog layer fails. Hiding a parent excludes its structural family, and showing it restores child-local visibility intent without reopening modal dialogs. Popups have a distinct interface, obey the active owner's modal policy, and retain their definition when hidden. Explicit close destroys the corresponding subtree; retained weak widget handles expire. Parent capabilities are authenticated by their manager-owned COM identity before a widget's mount-once claim is committed. Unknown option bits, malformed visibility/clip tags, invalid dimensions and invalid fixed layers are rejected before mutation.

## Widget class descriptions and shared protocols

Declare each concrete produced control as an IDL class, for example
`Button: IWidget + ILeafLayout`. Put its control-specific editing methods on
that class and its construction parameters on a factory method. Keep an interface only when unrelated classes need
the same independently queryable protocol. The implementation uses
`#[com_object(Button)]` on the concrete Rust widget type and stores its semantic
fields directly beside the common runtime helper; it
does not declare a matching `IButton` or repeat a list of projections.

Query `IWidget` when an API needs the common node protocol and `ILeafLayout` when
the layout runtime needs leaf measurement. Those projections preserve the class
identity and refer to the same allocation and state. The static facade hides these
checked projections from application code. IDL class definitions across crates should
name protocol supertraits through their module path when needed so generated
companion types resolve together.

Use the generated `UiFactory` as the typed construction surface. The core plugin's root creates
`UiRuntime`; the separately activated nodes plugin returns `NodesPluginFactory`, whose typed
`bind(&UiRuntime)` method creates `UiFactory`. `UiRuntime::construction_host()` returns only the core
`IConstructionHost`, allowing binding and facade construction to authenticate the exact host
identity without making core expose a concrete nodes projection. Do not model construction as a
redundant `IUiFactory`, class-ID lookup, or private runtime pointer. Hosts call typed generated
methods, and `Ui::from_interfaces` rejects a factory bound to a different runtime.

## Common state, input and node ownership

Each concrete widget embeds one private `WidgetRuntime`. That non-generic helper
contains `neocogi_ui::com::widget::WidgetState` plus host-backed resource and
rendering machinery; it cannot store a semantic widget implementation. `WidgetState`
owns the host authority, common options, mount-once claim and
sticky geometry flag. Neither helper owns a runtime tree, application callback, or
additional widget allocation. Concrete class contracts require `IWidget`, whose
protocol includes host/mount, effective properties, requested flag mutations,
update, take_change, and paint. Leaf measurement remains the shared `ILeafLayout` protocol. Button
activation and button-specific styling live directly on `Button`; no
activation-only, style-only, or lifetime interface is required for that class.

`WidgetState` contains plain fields. Mutating contract methods receive `&mut self`,
and the generated COM object provides the exclusivity needed to reach them. Do not
wrap ordinary widget fields in `RefCell<Option<T>>`, temporarily remove the value,
or add a second implementation-level busy flag; those mechanisms duplicate the
receiver contract and make state flow harder to follow.

A semantic setter checks `state.writable()` before borrowing its data, validates
all replacement inputs, commits atomically, and calls
`state.record_change(GeometryChange::Changed)` when measurement/placement inputs
actually differ. Common flags use `state.replace_flags`, `state.set_flags`, and
`state.clear_flags`. These operate on the current requested value stored inside
the widget, with `&mut self`; they never restore a construction snapshot or copy
effective properties back into requested state. Return the
update's own geometry result; NO_CHANGE must never erase a prior setter marker.
The runtime collects markers across the authoritative forest, including hidden
surfaces and descendants, before using measurement caches. Hidden/clipped widgets
still skip their ordinary update; authors must not rely on that update to notice
externally shared mutations or to flush a setter's pending marker.

`replace_flags(flags)` replaces the entire requested bitset, clearing omitted
bits. `set_flags(flags)` adds selected bits; `clear_flags(flags)` removes selected
bits. All three reject unknown bits before mutation, including unknown clear-mask
bits. Actual changes mark geometry dirty; no-ops neither add a marker nor erase
an earlier one. Lifetime and mutation-phase checks apply even to no-ops.

`WidgetState::requested_flags()` exposes requested configuration to the concrete
implementation. `IWidget::properties().options` reports effective behavior and
may add transient mouse-wheel or participation flags. It is an observation, not
a configuration snapshot to feed back into a mutation. Custom widgets implement
all three methods through their same common state, as shown in the independent
[UI extension](../../examples/ui-extension/src/lib.rs). Window-specific
`WindowOptions` remain a distinct interface contract.

The app queues raw input and then updates explicitly:

```rust,ignore
ui.input(UiInput::Key(KeyStroke {
    key: KeyCode::Tab, value: 0, pressed: true, repeat: false, modifiers: 0,
}), "")?;
ui.input(UiInput::Text, "complete Unicode commit ☃")?;
ui.update(drawable_size)?;
ui.render(drawable_size, renderer.as_com_ref())?;
```

UiInput uses drawable coordinates and carries no invented drag or pointer delta.
The runtime preserves queue order, computes held state/deltas, selects recipients
and localizes WidgetEvent pointer payloads to content coordinates. Modifier bits
are Alt=1, Ctrl=2, Shift=4, Super=8; printable logical keys are separate from text
commits. Function keys use numbers 1–24. Key-repeat is valid only on presses.
A cancel event clears held/focus/capture state in order; widgets must also handle
an eventless update with changed focus/active snapshots. Update every visible
widget even when it received no routed event: animation, shared-state
synchronization and editor blur commits can change geometry then.

Widget update implementations call `decode_widget_event(event, text)` before
using input. This checks the ABI tag, complete UTF-8 with offset zero, key and
button values, and the rule that only Text has a nonempty text argument. A text
borrow expires on return; copy it if it must be retained. WidgetUpdate is an
owned snapshot of content geometry, clipping, interaction and the runtime's
shared semantic action. It contains no painter or pointer into the runtime.
Properties are refreshed after update even when geometry is unchanged, so
mode-dependent focus/text-input policy is visible to the next routed event.

The runtime takes ownership of its retained context for each transaction and restores it
on every return path. No runtime RefCell guard spans a foreign call. Reentrant
runtime operations receive BUSY while the slot is occupied; ordinary peer
widget setters remain available during update. Measurement, painting and
property/change observation reject mutation through the common host authority.
Explicit topology mutations respect observational phases. Removal drops owners;
weak handles then report expiration. An active traversal owns its temporary child
sequence without retaining a RefCell borrow across foreign calls. If the editor
is destroyed, that sequence is dropped when traversal unwinds; the invalidated
layout result must not be submitted as a valid frame.

A failed measurement does not publish a zero-size preference. The runtime reports
the component error, leaves the node and its ancestor path dirty, and retries on
a later update. Widget authors should return an error for temporarily unavailable
measurement inputs and resume returning a valid nonnegative size after recovery.
Do not return a fabricated successful size to hide a failed resource lookup.

## Built-in editing and painting

Built-in leaf and container classes are their concrete semantic allocations.
Concrete `Textbox`, `TextBlock`, `Slider`, and `Number` operations live directly
on their callable class projections. Shared protocols such as
`ITextContent` and `ITextEditor` remain only where independently handled internal
content needs the same behavior. Every call edits the same state used by input,
measurement, and painting.

Linear and Grid keep their topology edits and parent-owned relationship metadata
on their concrete class projections. They require `IContainerLayout` only for
the runtime's shared measurement and placement protocol; `IContainer` remains a
reusable protocol for independently defined domain containers.

A Textbox removes CR and LF during construction, replacement and committed text
input. Its cursor is a byte offset at a Unicode scalar boundary, not a glyph,
grapheme or visual column. Replacing text moves the cursor to the end even if
normalization leaves the text unchanged; that cursor-only operation does not
invalidate measurement. TextBlock normalizes CR/CRLF to LF. Word wrapping breaks
at ASCII spaces and keeps an oversized word intact. These are explicit existing
policies, not promises of grapheme editing or a full Unicode line-break engine.

Numeric constructors validate finite values, finite ascending slider spans,
nonnegative steps and precision through `f32::DIGITS`. Setters reject nonfinite
values; sliders clamp valid values to their range. Checkbox checked state, swatch
color, scrollbar offset/range and button center-fill selection affect paint only.
Programmatic setters do not manufacture user-originated events. Label, font,
icon, wrapping and numeric-label changes retain a sticky geometry marker.

Button and ListBox expose `set_image(texture, fit_width)` directly on their
interface contracts. The operation
retains an immutable `ITexture`, validates its positive dimensions once and then
commits the edit. A failed dimension query leaves the previous content intact;
recursive image editing during that query reports BUSY. Measurement and paint
use the immutable size and do not call back into the image. Image setters dirty
geometry but emit no activation. `clear_image` preserves the label and font;
selecting a button icon replaces its image, and selecting an image replaces its
icon. A retained widget can retain its image after logical window closure; clear
the image before removal, or remove the widget owner to release it.

Width fitting preserves aspect ratio using the allocated content width. Ordinary
inline placement uses the source dimensions bounded by available width/height,
matching the existing control layout policy. Images always use white tint,
independently of the theme's label foreground. For example:

```rust,ignore
let images = host.renderer.query::<ITextureFactoryAbi>()?;
let image = images.create_texture(size, NcCopyFromBufferOffset::new(0, &rgba))?;
let mut button = host.ui.widgets().button_unobserved("Preview")?;
button.set_image(image.as_com_ref(), true)?;
// The button owns a reference; this local image reference may now be dropped.
```

Font and icon capabilities select semantic roles or resolve an exact named
resource once in the widget's runtime bundle. Unknown names leave the previous
selection intact. Normal measurement and painting retain stable resolved
identities and perform no COM calls per glyph. FillStates controls unframed
button center fill; the outer FRAME option remains owned by retained traversal.

Call `WidgetUpdate::validate()` or `WidgetPaint::validate()` before consuming a
copied phase snapshot, and decode event/text input with `decode_widget_event`.
Painting receives effective clipping, enabled/hover/focus/press state and logical
window activation as copied values. It has no mutable runtime context.

Every `IPainter` operation has a mutable receiver. A widget therefore uses a
mutable `Painter` facade while recording, and neocogi-com rejects conflicting or
reentrant access through another retained projection. The concrete recorder owns
ordinary invocation and frame-context fields; it does not use interior-mutable
slots to simulate mutation through `&self`.

`IPainter::triangles` remains available for custom widgets that already own a
complete local atlas batch. The input offset is zero, its length is a multiple
of three, and every position and UV component is finite. Built-in classes do not
pre-expand their visuals through that path. They issue compact frame, icon, text,
and image operations, which the runtime appends to its one display list and
expands during the single final renderer pass.


## Local custom leaves and solid drawing

The skin plugin's `StyleCompiler` class supports typed palette and metric editors.
`compiler.derive_style(base, metrics, PaletteEdit::Preserve.into())` keeps the base
theme artwork; `PaletteEdit::Flat(palette)` compiles complete flat appearance tables.
Both return a new immutable bundle in the current catalog without installing it
or changing the retained tree. Call `ui.install_assets(candidate, renderer)` through
a mutable `Ui` reference to publish it through the same validation and GPU preparation
as any other revision.
Invalid metrics or unknown palette tags leave the installed resources unchanged.
The shared `Color` and `FlatPalette` records are plain contract values; they carry
no renderer or parser implementation. `Ui` and `WidgetFactory` clones retain their
existing COM projections. Event handlers may prepare a candidate through shared access;
the host installs it through a mutable `Ui` reference after dispatch returns.

Derived styles use the ordinary portable bundle publication/install path, including
its pixel copy and renderer preparation. This API does not promise a palette-only
fast path. Applications should derive once per committed edit rather than every
animation frame, and preserve image-backed artwork when only metrics are edited.

`WidgetFactory::leaf(content, options)` stores a concrete `LeafContent` inline in
one COM allocation implementing `IWidget` and `ILeafLayout`. Its common state
supplies the same mount-once, closure, options and sticky invalidation rules as
built-ins. Content methods use static Rust dispatch; there is no second widget
tree or erased application value. Use a direct COM class with `WidgetState` when
the component also needs its own public editing interfaces.

Content is stored directly in the COM allocation. Widget phases receive an
ordinary mutable object reference, while the adapter may call a content method
with the narrower receiver declared by `LeafContent`. Recursive entry through any
projection returns `NC_E_BUSY` while that outer mutable COM phase is active. A
failed update records a conservative geometry change
because application code may have partly changed its model. Successful updates
must still report CHANGED truthfully. The external state, hidden-widget and
skipped-update obligations above apply equally to this helper. Animation that
changes only drawing can return NO_CHANGE.

`Painter::colored_triangles` takes local positions and packed RGBA colors. Solid
content never needs the atlas's white-pixel location: the UI supplies it while
executing its existing solid geometry commands. `stroke_line` and `fill_polygon`
share that geometry path. All points must be finite; line widths are nonnegative,
and triangle counts are multiples of three. Polygon filling retains the painter's
existing simple-polygon policy; applications must supply a non-self-intersecting
boundary in order.

`let inner = painter.clipped(rect)?;` creates a borrowed painter value with a
narrower clip. Commands issued through `inner` intersect that clip with the real
widget clip; commands issued through the original painter keep the original clip.
There is no mutable clip stack, callback, or restoration operation. The copied
scope still expires after the widget paint call, including scopes or raw painter
interfaces retained by a component. Negative extents are rejected and empty clips
draw nothing. Coordinates remain content-local after clipping.

## Container topology and edge metadata

An independently implemented container exposes `IWidget` and `IContainerLayout`
on one COM object. It exposes exactly one layout family: adding `ILeafLayout`
to that object is rejected. Obtain a private `IChildrenEditor` from the widget's
host during construction, retain it in the container, and return its `IChildren`
view from `IContainerLayout::children`. That view's canonical identity must stay
unchanged for the container's lifetime. The runtime accepts only collections
issued by that exact host. One collection can belong to only one mounted
container. Arbitrary look-alike collection objects and shared collections are
rejected before attachment.

The view and editor deliberately have different COM identities over one actual
child sequence. Querying the view cannot recover edit authority. Expose concrete
editing methods that keep Linear tracks, Grid spans or application-specific edge
metadata synchronized; do not return the private editor to callers who are not
responsible for that metadata. Child references do not transfer tree ownership.
Widgets have one `ComBox` owner. Container insertion consumes it; application
fields keep cached `WeakComBox` projections. The container exclusively owns its
`IChildrenEditor`; retaining a view cannot preserve removed children. Dropping the
owner releases ordinary child and queue fields. `IWidget::close`, `Closed` widget
state, and `ChildEditResult` no longer exist.

`WidgetState` tracks `Unmounted | Mounted` only. A failed consuming insertion or
replacement destroys its candidate but leaves existing topology unchanged.
Indices, host identity, layout capabilities, collection ownership and prospective
cycles are checked before a claim commits. Reentrant topology edits during
attachment validation return `UI_E_BUSY`. Children cannot be detached for reuse.

A successful editor call returns `Ok(())`. Prepare fallible metadata work before
that call and publish the validated metadata after it. Destruction has no
recoverable status and cannot turn a committed replacement into an error:

```rust
let edge = validate_edge(edge)?;
self.editor.replace(index, child)?; // Consumes ComBox<IWidget>.
self.edges[index as usize] = edge;  // Publish matching validated metadata.
Ok(())
```

Owner removal rejects new calls before concrete destruction. An admitted child
can finish after its parent is destroyed; waiting could deadlock self-removal.
Use ordinary `Drop` for local cleanup. The full thread and ABI contract is in
[exclusive ownership](EXCLUSIVE_WIDGET_OWNERSHIP.md).

## Scoped container layout

`IContainerLayout::measure` and `place` receive an `ILayoutService` and a
`LayoutScope`. The service owns the active child sequence. It never stores a
borrowed runtime node, mutable tree reference, or pointer to the caller's stack.
The container receives indexed operations and copied geometry only. The provided
scope is valid for that synchronous call; retaining the COM service does not
extend the scope. Tokens never wrap or become valid again.

Measurement permits `child_count` and `measure_child` only. Placement also permits
exact child allocation, copied content-extent reads, responsive participation and
explicit reporting of a changed child placement dependency. Placement-only calls
during measurement return `UI_E_BUSY`. A valid zero constraint remains different
from an unbounded constraint. Preferred child sizes include each child's own
frame. The container resolves tracks and relationships before passing an exact
rectangle to `place_child`; the runtime does not reinterpret that policy.

Nested child containers receive a fresh token. Their parent's token returns
`UI_E_EXPIRED_SCOPE` while the child token is active, then becomes valid again
when the child call returns. Direct reentry while a service method temporarily
owns its frame returns `UI_E_BUSY`. After the overall invocation ends, every
method rejects the old scope, including empty or otherwise harmless reads. All
success and failure paths restore the parent frame and authority.

Use the scoped service for child layout access. The separate `IChildren` view can
report its stable copied count while a layout owns the sequence, but `child(index)`
returns `UI_E_BUSY` then. A built-in algorithm validates the scoped count before
running, so an access error cannot be mistaken for an empty child sequence while
its edge metadata still contains entries.

Placement returns one `ContainerGeometry`: logical content extent, descendant
viewport and translation, and overflow propagation policy. The runtime clips the
viewport to the container's content allocation and publishes the result only
after the call succeeds. `HIDDEN` participation preserves ownership while gating
update, paint, input and focus for the entire subtree; `DISABLED` preserves update
and paint while excluding subtree input. Hidden children do not contribute active
overflow. Setter-change collection still visits hidden owned descendants.

Report every changed measurement or placement dependency through a truthful
update result or sticky setter marker. This includes shared scrolling offsets,
responsive participation policy, expanded/collapsed state, and layout inputs
changed indirectly by another object. `child_placement_changed` is for an
already active placement pass whose descendant depends on shared state despite
receiving the same rectangle; it does not replace the writer's responsibility to
request the initial layout. Independent plugins must honor these same ownership,
phase and invalidation rules.

A failed child measurement cannot become a cached zero preference. Failed
container measurement and placement retain invalidation and require a successful
retry before rendering. An implementation may have placed some children before
an error, but that partial pass is never a valid renderable frame. Restore owned
semantic state on all error paths and do not retain references obtained from
short local context wrappers after returning through COM.

## Composed controls and indirect invalidation

A composed control owns ordinary COM children. It shares explicit scalar data
only where two of those children need the same state. ScrollArea's viewport and
its real scrollbars share offset/range data; Disclosure's header and layout share
expansion data. TextArea's actual editor requests caret reveal through that same
scroll model. These models hold no parent widget, runtime, application, or installed COM
receiver reference. A child must not need to upgrade a weak parent handle to
perform an ordinary edit.

Composition does not reinstate the removed wrapper design. The registered outer
interface still owns its own runtime, topology, and outer behavior directly. Each child
is a separately targetable retained role with its own identity. Do not introduce a
generic core containing the outer class's algorithm, or duplicate a child's state
on the parent merely to make forwarding convenient.

The public capabilities preserve each composition's structural roles:

- The `ScrollArea` class permits replacing its one application content root and
  requires the shared `IScrolling` protocol. It does not expose `IContainer`
  methods that could remove the transform surface or either scrollbar.
- The `Disclosure` class returns its actual text header and actual Linear body.
  Edit body children through the returned Linear capability, and edit the label
  or header options through the returned text capability. The fixed header/body
  pair itself cannot be removed through a generic container interface.
- The `TextArea` class is the composed root. Its direct editing calls forward to
  one actual editor child. Querying this root for `IScrolling` returns the same
  outer identity; querying it for the `ScrollArea` class projection fails. There
  is no operation that replaces its fixed editor with
  arbitrary content. The child exposes `IMultilineEditor`, has its own ordinary
  widget identity, and is the composition's single keyboard Tab stop.

For example, a static client can edit actual child capabilities without mirroring
plugin state:

```rust
let mut section = ui.widgets().disclosure("Details", DisclosureStyle::Tree, false)?;
let mut body = section.body()?;
let editor_owner = ui.widgets().text_area_unobserved("first line\nsecond line", TextWrapping::Word, true)?;
let mut editor = editor_owner.handle();
body.insert(0, editor_owner, LinearEdge::new(LayoutTrack::Content))?;
section.header()?.set_text("Editable details")?;
section.set_expanded(true)?;
editor.set_cursor(0)?;
// Synchronize geometry and pending caret reveal before rendering.
ui.update(drawable_size)?;
```

`Changed | NoChange` from an ordinary update does not automatically detect changes
in shared models. In particular, clicking a disclosure header need not change the
header's own preferred size, and moving a text cursor need not change text size.
Nevertheless, expansion and caret reveal can change an ancestor's placement.
The writer of that shared model must retain a sticky notification, and the
owning layout widget must collect it through `take_change`. That notification
must survive skipped updates, collapsed ancestors, and intermediate render
preflight checks. The built-in models do this explicitly; independent plugins
must supply the same discipline for their own shared data.

A requested scroll offset may temporarily exceed the previous maximum. Placement
clamps it against freshly measured content. This lets `scroll_to_end` accompany a
content replacement without losing the new end to the old range. `ScrollInfo`
therefore distinguishes current requests from the last installed ranges and
viewport. Programmatic setters are silent: they do not fabricate user events.
The standalone scrollbar offset setter clamps immediately to its installed range.
Disabling a scroll composite clears its requests and ranges and changes the
structural children's participation on the next placement.

TextArea normalizes CRLF and lone CR to LF. Cursor offsets count UTF-8 bytes and
clamp to the preceding Unicode scalar boundary. Word wrapping breaks at ASCII
spaces and preserves an oversized word as one span. Text replacement, including
a text-identical replacement, requests caret reveal; so does cursor positioning.
These setters conservatively invalidate geometry because reveal can move the
viewport even when text dimensions are unchanged. Font and wrapping changes also
reset the preferred visual column and request reveal. Visible ordinary updates
resolve those pending requests; hidden editors retain them until eligible again.
Widget/application writers must run update before rendering after these edits.

Destroying a composite releases its child owners. Retained header, body, editor
and bar observers report expiration; they cannot preserve or remount a subtree.
Observers never transfer topology authority.

## Combo and compact menus

Create a combo through `Ui::combo_box_with_queue(parent, items, selection, queue)`, not the ordinary
widget factory. The runtime needs the parent up front because it constructs one
private popup definition with a real structural owner at the same time as the
unmounted header. The caller then inserts the returned `ComboWidget` into that
window's retained tree. The popup capability, row presentation, and coordination
events are deliberately not exposed.

The control owns its display labels and represents the committed stable leaf ID with
the NCIDL-generated `ChoiceSelection`. `ChoiceSelection::None` is not leaf zero: it
renders the runtime-owned `Select…` fallback unless `set_placeholder` replaces it.
The facade consumes and returns that generated contract type directly instead of
declaring a parallel Rust selection record. The flat `combo_box` and `replace_items`
helpers assign IDs equal to item ordinals. `combo_menu` and `replace_menu` accept the
generated core `ChoiceMenuEntry` preorder model and shared UTF-8 text directly, allowing
arbitrary stable IDs and nested submenus without another handwritten public tree:

```rust,no_run
use neocogi_ui::{ChoiceMenuEntry, ChoiceSelection, MenuTextRange};

let text = "FruitAppleBanana";
let entries = [
    ChoiceMenuEntry::Submenu {
        label: MenuTextRange { first: 0, count: 5 },
        children: 2,
    },
    ChoiceMenuEntry::Choice {
        id: 100,
        label: MenuTextRange { first: 5, count: 5 },
    },
    ChoiceMenuEntry::Choice {
        id: 200,
        label: MenuTextRange { first: 10, count: 6 },
    },
];
let combo = ui.combo_menu_with_queue(window.as_com_ref(), text, &entries,
    ChoiceSelection::Selected(100), Some(queue.as_com_ref()))?;
```

Every submenu is followed immediately by its declared number of direct children;
nested descendants follow their owning child branch. Construction and replacement
atomically reject malformed counts, empty branches, invalid UTF-8 ranges, duplicate
leaf IDs, nesting beyond 32 branches, or an unknown selected ID. Programmatic edits
remain silent; `IChoiceChangedEvent::selection()` returns the captured `ChoiceSelection`
only after a pointer or keyboard choice commits. Display labels remain independently
available through `choice_label` and `display_text`; generic application data is
mapped from the committed stable ID.

Opening restores the retained submenu path containing the committed leaf and copies
that leaf into the transient keyboard candidate. The popup paints pointer hover,
then keyboard candidate, then committed selection: leaving the rows therefore
restores the keyboard/committed fallback. Arrows move only the candidate, Enter or a
row press commits, and Escape/outside dismissal preserves the prior selection.

Popup placement uses the header's complete outer border box rather than its inset
content rectangle. Its horizontal extent remains fixed to that box while row
content determines height, so theme-specific frames cannot shift the left edge or
extend the right edge.

The header and composition manager share one runtime-private typed state capability.
That allocation owns ordinary text, generated preorder entries, stable selection,
placeholder, revision, and lifecycle fields; every mutation has an exclusive
`&mut self` receiver. The manager associates it with a generic `PopupHandle`, while
the root forest receives only compact choice-menu values and ordinary relational
submenu edges. No generic Rust payload, `Any`, `Rc<RefCell<_>>` semantic model,
public popup handle, or mirrored menu tree crosses the boundary. Applications map
the committed stable ID back to their domain values.

Menus retain their compact representation: one surface owns direct row values,
with no synthetic widget tree per row. The runtime exposes `IMenus` alongside its
interface and window manager. A root `MenuBuilder` owns one native declaration;
child builders address append-only submenus inside it. Keep the root alive until
mounting. Dropping an unconsumed root expires its child builders and row sources.
Only root builders can be mounted, in their creating runtime, once. Failed owner,
shape or destination validation leaves the complete declaration usable.

```rust
let menus = ui.menus()?;
let bar = menus.create("")?;
let file = bar.submenu("File")?;
let menu_queue = app_queue.query::<neocogi_com::events::IEventReceiverAbi>()?;
let open = file.item_with_queue("Open", Some(menu_queue.as_com_ref()))?;
open.set_shortcut_hint("Ctrl+O")?; // Presentation text, not a shortcut binding.
open.set_mark(MenuMark::Checked { checked: false })?;
file.separator()?;
file.submenu("Recent")?.item("Document")?;
menus.attach_bar(window.as_com_ref(), &bar)?;
// The actual compact rows now belong exclusively to the manager's forest.
open.set_label("Open file")?;
```

A bar's direct entries must be submenus. `attach_bar` installs one intrinsic bar
on a window; trying to attach another reports `UI_E_POLICY`. Dynamic bar
replacement is not exposed. For an ordinary standalone popup, build a root with
items/separators/submenus and call `menus.popup(window, &builder)`, then use the
returned `IPopup` capability's `show()` method for current-pointer placement or
`show_at(anchor)` for an exact screen anchor. Its hide/close methods retain their
ordinary transient-lifetime behavior. Successful mounting consumes declarations;
subsequent builder appends report `UI_E_CLOSED`.

Menu item interfaces resolve the same native row before and after that transfer.
They expose label, enabled state, check/radio role, hint and validated font
selection. Retaining an item keeps its COM projection allocated without keeping
a menu surface alive. Closing its popup/window makes further edits report
`UI_E_CLOSED` and ends its activation source. Radio-group exclusivity, command
shortcuts and application actions remain application policy; presentation edits
are silent and do not implement those behaviors implicitly.

F10 and an unchorded Alt tap enter a window's bar; arrows navigate headings and
submenus, skipping disabled rows and separators. Enter/Space submits the selected
row and closes the active path, restoring ordinary application focus. A shown
standalone menu starts with no row selected: Down selects its first actionable
row before Enter. Submission emits the shared `IActivatedEvent` marker. The
envelope identifies the source independently of its editable display label.
All destinations preserve their consumer's emission-order FIFO.

## Native paths and standard file pickers

`NativePath` owns an immutable `INativePath` snapshot. Its `unit_count` counts
Unix bytes or Windows UTF-16 code units, excluding any trailing terminator.
The two copy methods validate their exact requested ranges before writing and
reject a mismatched encoding. Unix invalid UTF-8 and Windows unpaired surrogates
are preserved. Decoding a different platform encoding is an error. `display()`
is explicitly display-only UTF-8; two distinct native names may produce identical
replacement-character labels and must remain distinct selections.

The native picker is a UI-owned COM component containing actual COM buttons,
textboxes, list rows, Linear containers and scrolling composites. Its internal
private inbox owns child messages and its ordinary mutable model consumes them
through optional `IWidgetDispatcher::dispatch_next`. Its public `IFileDialog`, widget and layout
projections share one COM allocation and borrow gate; the root Linear value is
embedded directly. Dropping client handles leaves a mounted picker alive.
Removing its owner disconnects its private inbox after admitted calls return; it
does not post a new completion during destruction. A picker handle is weak. The
static facade depends only on contracts and COM support; Rust model state never
crosses the ABI.

Construct the component with its completion queue, then open it from an ordinary
application handler:

```rust
let completions = app_queue.query::<neocogi_com::events::IEventReceiverAbi>()?;
let mut picker = ui.file_dialog_with_queue(window.as_com_ref(), Some(completions.as_com_ref()))?;
let mut request = FileDialogRequest::new(native_directory);
request.title = "Choose a document".to_owned();
picker.open(&request)?;
ui.update_with_events(drawable_size, owner.as_com_ref())?;
```

The common queue receives an owned `IFileAcceptedEvent` or `IFileCancelledEvent`.
The handler queries the accepted interface and calls `neocogi_com::path::decode`
on its `path()` result when it needs a host-native `PathBuf`. Cancellation is a
separate marker interface; display text is never reconstructed as a lookup key.
Acceptance is lexical: application code decides whether the path must exist or
have a particular type before using it. A selected row retains its exact native
path while the filename field displays a possibly lossy basename. A real user
filename edit clears that selection; a programmatic display update does not.
Leading and trailing spaces are filename content. An unedited lossy directory
field still denotes the committed native directory; a real path edit makes its
UTF-8 text authoritative for the next navigation request.

Directory reads are synchronous and transactional. Every directory entry and
metadata lookup must succeed before either new column is published. Broken
symlinks and unreadable entries reject the whole candidate. A failed initial
open reports `UI_E_IO` (-1008), leaves the picker idle with its prior listing,
and exposes the OS error through `diagnostic()`. Failed navigation displays its
diagnostic while preserving the previous native directory, rows and selected
path. Successful navigation replaces only dynamic content; fixed controls and
scroll models remain, with offsets clamped against the new content on layout.
Overlapping opens report `UI_E_BUSY` without changing the active request.
Browser targets have no native filesystem access; native directory operations
there report the platform's unsupported-operation error rather than presenting
a fabricated filesystem picker.

The private inbox preserves emission order across its child controls and participates
in descendant-first runtime passes. It shares the update's finite work budget with
application consumption. Navigation rebuilds rows inside the same mutable update
that handles the navigation message. Application
callbacks may open an already prepared picker through its ordinary interface;
the following layout establishes its modal scope before later queued input.
The picker maps its window's lifecycle output into its private queue. Use its
constructor-bound completion receiver for application outcomes. Its private
window receives the picker's sole lifecycle receiver during construction; no
operation can install another receiver later. The returned window remains
available for ordinary geometry, visibility and destruction operations.

Cancellation hides the reusable modal and completes one active request. Reopening
before a previously queued Hidden notification is delivered does not cancel the
new visible request. Hiding an active modal or its owner queues lifecycle cancellation; explicit picker close cancels before removing its owner. Destroying the window or
runtime disconnects the picker without synthesizing a cancellation event.

Each completion is already a committed request outcome in the consumer's inbox.
Dispatcher/runtime destruction or remapping cannot erase those outcomes.
An active request is abandoned during owner destruction; explicit cancellation
settles it while still alive. An already completed request is not completed again. Failed consumers consume the attempted outcome and preserve
unattempted successors. This is the same ownership rule as every other output;
there is no separate finished/terminal queue. No producer retains application state.

## Complete frames, images and application GPU work

`IRenderer::render` prepares and explicitly finishes one complete frame synchronously.
There is no cloneable COM frame or borrowed native render-pass object. UI records translated
semantic operations in painter order. Each nine-patch crosses into the retained GFX compositor
as one closed typed operation; GFX owns its source lookup, nine-cell expansion, visibility,
final clipping, and atlas batching alongside text and icon expansion. It then publishes the owned
`IRenderStream`. A retained stream/token expires
on success and failure, including zero-length reads and indexed resource queries.

GFX validates foreign geometry before classifying its visibility. Triangles fully
inside the effective clip retain their vertices directly; triangles wholly outside
one clip edge are discarded. Other triangles use attribute-preserving polygon
clipping, including triangles whose vertices are all outside but enclose the clip.
Fully visible atlas/image rectangles keep their original UV endpoints; only partial
intersections interpolate them. These are private CPU paths with the same ordered
renderer stream and COM contracts.

Text resolves its operation/viewport intersection once for the whole run. UTF-8,
font and buffer validation still applies to invisible text; empty or zero-area
runs keep their existing no-geometry behavior. Solid geometry reuses the white
tile cached when the current immutable resource revision was installed.

A stream contains a bulk vertex buffer and ordered `RenderBatch` records. Atlas
ranges contain positive multiples of three vertices; image ranges contain four
vertices. Ranges must partition the buffer without gaps, overlap or unused trailing
vertices. Custom items contain positive content/clip rectangles, with the clip inside
both content and viewport. Unknown tags, invalid rectangles, non-finite vertex
components, truncated copies, zero invocation tokens and excess counts fail before
GPU execution. Limits are 16M vertices and 1M batches per complete frame. Adjacent
atlas triangles are coalesced. The native plugin makes two bulk reads, plus one owned
resource fetch for each image/custom batch, and reuses its CPU allocations.

The backend detaches its complete mutable state before calling a foreign stream or
querying resource identities. Native render/upload/scene-creation reentry returns
`UI_E_BUSY`; scene edits also reject preparation/execution reentry. All source reads
and resource authentication finish before a native frame is acquired. A failed
preparation discards already resolved resource snapshots and allows retry. GPU
execution failure is reported by explicit finish and does not present the failed
frame. Immediate GL work may already have executed: abandonment is not GPU rollback,
and native device/surface loss can require constructing a replacement renderer.

Create immutable images through the chosen renderer's `ITextureFactory`. Uploads
must have offset zero and exactly width*height*4 RGBA bytes, with positive dimensions
satisfying the common image validation/allocation policy. Publication follows a
successful upload. `ITexture::size` reports immutable source dimensions; a painter's
destination rectangle changes scaling without changing that image. No public native
texture ID is accepted across COM.

`Painter::texture` draws the complete source. `texture_region` accepts an explicit
normalized `TextureRegion`: finite, ordered edges in `[0, 1]`. Destination clipping
interpolates within that source region. Built-in paint forwarding preserves ordered
atlas/image boundaries and cropped source coordinates through the same executor;
it releases queued image references on forwarding errors and after completed
frames. Keeping a copied frame's vertices does not keep the image alive.

Resources belong to one native window/renderer instance, even when two instances
use the same GPU API. The native plugin authenticates canonical COM identity against
its own weak registry; a matching ABI or a claimed numeric owner does not prove
membership. Foreign images and scenes report `UI_E_OWNER` directly from the renderer.
The UI's frame API reports backend failures as `UI_E_RENDER`; query
`IRenderDiagnostics::diagnostic` on the renderer for the underlying preparation or
native-driver detail. Returned diagnostic strings are immutable snapshots.

An image keeps its native allocation, window and context alive after the public
window interface is released. A prepared draw keeps its image alive until finish or
abandonment. Final COM release immediately removes registry authentication before
that address can be reused; final native-image release deletes while idle, or queues
deletion until the active ownership transaction exits. Deletion selects the owning
GL context, and destructors never submit or present. Idle frame storage retains no
resource references, so it cannot form a device/resource cycle. Plugins remain loaded
for the process lifetime; hot unloading is unsupported.

The renderer platform exposes `IMeshScenes::create_scene` and `IMeshScene: IRenderItem`.
The scene is one COM object. It accepts complete finite mesh records and triangle
indices, plus atomically replaced column-major projection/view/model transforms.
Offsets are zero; limits are 4M vertices and 12M indices, every index must name a
vertex, and two empty buffers clear the mesh. Invalid mesh or transform edits leave
the prior scene intact. The native renderer snapshots shared immutable mesh buffers
before preparing actual depth-tested GPU work; it does not substitute CPU UI
triangles for application rendering.

The reusable Vulkan mesh renderer uploads each immutable mesh once and shares its native
vertex/index buffers across draws and frame slots. Camera updates, draw reordering,
temporary hiding and swapchain resizing do not replace the geometry. A replacement
mesh receives its own upload; each submitted slot retains the previous upload until
that slot's fence completes. Unreferenced CPU meshes leave the upload cache during
the next preparation, and outstanding slots keep their GPU buffers alive. These
owners remain private to the renderer and do not retain the COM scene or window.

Use `Painter::texture(rect, rgba, image)` and `Painter::item(rect, scene)` at the
intended source position. The full example is `demos/smoke/src/gpu.rs`. Scene edits
normally change paint only: render again; a UI update is needed if widget geometry or
other retained inputs also changed. Perform scene edits before painting. The GPU
owner can detect its own active preparation/execution, but cannot detect a separate
UI plugin's earlier paint phase. Widget/application authors must uphold observational
paint and synchronize externally shared scene/model changes; no automatic dependency
tracking is implied by `GeometryChange::NoChange`.

Backend-specific application renderers are compiled into the selected GPU plugin.
Their concrete command enums, GL context, Vulkan command buffers, WGPU encoders and
render passes remain private Rust values inside that binary. An independently
loaded GPU extension cannot borrow these native Rust objects through the generic
COM API. Such an extension would need a separately designed backend-specific ABI;
this migration intentionally provides typed scene/resource capabilities instead.

## Platform input and native composition

`UiRuntime: IInputSink` exposes the same ordered input queue through inherited
calls or a separately retained input projection. Cache that projection once in
the host loop. `IPlatformWindow::poll(destination)` borrows it only for the
synchronous call; the platform never owns an application input destination.
An application interceptor can implement `IInputSink` as a concrete COM class,
retain the runtime input projection and forward accepted events explicitly.

Drain `PollStatus::Event` until `IDLE`; `CLOSE` requests host shutdown. A failed
input delivery consumes the attempted event and preserves later queued events.
Do not replay the failed event automatically: a destination may have committed
part of its work before returning an error. Recursive polling of the same window
returns `UI_E_BUSY`, including during delivery. Native queues are per-window;
process quit is broadcast. Consecutive pointer motion may be coalesced within one
window's queue, but a button/key/text transition always separates motion runs.

After draining, read `size()`, call `update`/`update_with_events`, synchronize
`text_input(ui.text_input_area()?)`, and render. A changed physical size triggers
measurement through the ordinary update contract. Skip rendering zero-sized
or minimized drawables. Pointer positions and IME rectangles use physical
pixels, including captured positions outside the drawable; the platform converts
logical SDL or CSS coordinates at its own boundary.

The demo host runs its application `frame()` callback after platform polling and
before one `update_with_events()` call. Frame edits, input handlers, nested
component messages and their resulting layout changes settle before rendering.
Work that a handler defers to `frame()`, such as installing a prepared theme,
runs on the following frame. Nested event delivery itself completes within the
current update; nesting does not add a frame of delay per level.

`text_input_area()` reports only committed editable focus. Pending input,
geometry edits and removed focus must not expose stale rectangles. It is a
placement hint, not editor content or an editable text buffer. Synchronize it
after successful updates. Repeated activation preserves an in-progress native
composition. SDL's process-wide IME is controlled only by the focused native
window, so synchronizing an unfocused window cannot disable another's editor.

Logical key presses/releases carry the complete modifier snapshot and repeat
state. They never synthesize committed text. SDL TextInput and the DOM text
commit are authoritative, including complete Unicode strings; composition
previews remain platform-owned. Widget/app authors must not insert key scalars
again on the text path. Focus/capture cancellation is queued in input order and
resets held state. Fractional wheel motion is accumulated in physical pixels;
focus cancellation discards the previous gesture's remainder.


## Rejected exclusive arguments

An owning parameter has an ownership-bearing error, generated by IDL and checked
by the typed IDL backend. Do not replace it with `NcStatus` in a forwarding method:

```rust,ignore
fn insert(&mut self, child: ComBox<IWidgetAbi>)
    -> Result<(), (NcStatus, ComBox<IWidgetAbi>)> {
    if let Err(status) = self.validate_destination() {
        return Err((status, child));
    }
    self.editor.insert(0, child)
}
```

Finish fallible preparation before accepting a child. A method returning an error
must return that same untouched owner; it must not destroy or replace it. This
also lets an application retry a rejected widget without rebuilding it:

```rust,ignore
match row.insert(0, button, edge) {
    Ok(()) => {},
    Err((status, button)) => {
        report(status);
        // The call has finished. The application chooses retry or destruction.
        drop(button);
    },
}
```

Generated COM adapters reserve their conversion scratch before calling a method
that can accept an owner. Such methods require a finite maximum cost for copied
results and converted writable buffers; the maximum must fit before dispatch.
Your implementation still prepares its own fallible work before committing.
Completion does not allocate replacement buffers, and all converted outputs are
validated before any is published. Direct numeric buffers remain in-place.
[Prepared output completion](COM_OUTPUT_COMPLETION.md) explains the IDL/Rust
contract, its conservative enum limit, and failure-injection tests.

Fresh private builders may deliberately discard returned candidates after a
failed ABI call, provided those candidates cannot own an active receiver. There
is no implicit tuple-to-status conversion that would hide that decision. See
[ABI v17](../../crates/com/docs/abi-v17.md) for wire slots, forwarding, multiple inputs
and the requirement to return valid output after accepting ownership.

For construction code that deliberately abandons failed candidates, convert to
`ComBox<IWidgetAbi>` before the shared insertion/window-building body. Borrow the
parent's existing generated client view for both owning and weak facade storage.
The full demo and shared layout samples use `neocogi_ui_samples::append` this way.
There is no reason to query a rejected owner back to Button, Linear, or another
concrete interface merely to destroy it. Keep the typed facade operation when the
caller needs the original typed owner for retry, and keep ownership-bearing errors
through every forwarding provider.

## Copied output getters

An IDL `&mut [T]` or `&mut Slice<T>` produces fresh values; it is not a mutable
input. Providers implement `NcOutputWriter<'_, T>`, write using `push` or
`extend_from_slice`, and never read destination contents. Generated callers pass
`NcOutputBuffer<T>::output(offset, capacity)` and read its published `as_slice()`
after success. Short results expose only their initialized prefix. A true
transformation takes a distinct `&[T]` input and output argument. See
[the complete contract](COM_OUTPUT_BUFFERS.md) for forwarding, allocation-free
identity storage, failure semantics, wire validation, and ABI generation 17.

Private content created by `WidgetFactory::dispatching_leaf` receives a borrowed
`LeafDispatchContext` in `DispatchingLeafContent::dispatch_next`. After committing
any preferred-size input, call `context.geometry_changed()` before a later fallible
operation or return. This updates the inline wrapper's sticky marker and notifies
its host in one operation. A later `LeafContent::update` result is too late to
repair hit testing for the next already queued input. Failed helper callbacks
also retain conservative invalidation; passive leaves advertise no dispatcher.
