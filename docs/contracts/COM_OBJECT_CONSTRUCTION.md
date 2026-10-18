# Ordinary COM object construction

`#[com_object]` constructors accept fully initialized Rust values. An object is
never exposed through a weak handle before its value exists. Weak self identity
for event delivery is private factory wiring, not a reason for cyclic allocation.

## Construction and exposure

| Ownership | Interface owner constructor | Concrete local owner constructor |
| --- | --- | --- |
| Shared | `T::from_value(value)` → `ComPtr<IUnknown, T::Threading>` | `T::into_local(value)` → `LocalCom<T>` |
| Exclusive | `T::into_box(value)` → `ComBox<IUnknown, T::Threading>` | `T::into_local_box(value)` → `LocalBox<T>` |

Only the selected ownership family's two constructors exist. Each allocates
once, owns the supplied value, and uses the same lifetime/borrow rules as every
interface projection. Local owners retain proof of the concrete implementation;
they introduce no second allocation and permit no foreign-value downcast.
There are no initializer closures or cyclic constructor variants.

A factory that needs a weak source identity performs these steps:

1. Validate application input and acquire the selected receiver.
2. Allocate a complete value with a valid empty event destination.
3. Obtain the weak source interface, then connect the destination using checked
   concrete access or the existing owning record. Shared interface owners are
   downgraded; exclusive local queries already return weak projections.
4. Expose the capability only after that wiring succeeds.

The temporary empty destination belongs to private construction, not an
application-visible initialization phase. No callback or external capability
may observe the object between allocation and connection. The factory-selected
receiver remains fixed after exposure. A later construction error drops the
initialized value normally, releases its receivers, and expires saved weak
identities; no provisional weak owner or rollback publication protocol exists.

## Event producers

Ordinary widgets use the runtime-owned origin stack described in
[widget event origins](WIDGET_EVENT_ORIGINS.md). They store no weak self origin.

Shared service ports may still carry explicit weak identities because window,
popup, menu and choice-menu methods can emit outside widget dispatch. A factory
obtains those identities from initialized objects. Native window/popup
registration emits no events; exclusive manager access spans registration,
ordinary COM allocation and connection of the fresh record's destination.
The private connection functions expose no COM receiver replacement API.
Menu rows are connected before insertion into their declaration.

FileDialog completion can occur immediately from application `cancel`/`close`
calls. Its factory connects the weak completion identity through checked local
access before transferring its widget root into the runtime. Completion remains
synchronous and does not require a widget origin scope.

## Value and allocation lifetime

Generated storage contains the common control header, the shared call gate when
required, and `UnsafeCell<MaybeUninit<T>>`. Every constructor supplies an
initialized `T`; after final owner destruction the allocation can remain live
under weak ownership without a valid `T`. `MaybeUninit` permits that retained
storage and prevents a second automatic value drop. It no longer supports a
partially initialized construction state and cannot be removed merely because
cyclic constructors are gone.

Shared ownership starts at one strong owner; zero is terminal. Exclusive
ownership starts with its owner bit set; clearing it permanently rejects new
calls. Checked call admission, concurrent destruction and weak bookkeeping
follow [the lifetime contract](COM_LIFETIME_ERRORS.md). The final owner destroys
the value once, while its implicit weak owner keeps storage valid through that
destructor. The final weak reference frees storage through the allocating
provider. Each callback remains paired with the exact generated allocation.

Runtime and plugin descriptor ABI remain v1. Core draft 17, fingerprint domain
10, handle layouts, interface metadata, threading and process-wide runtime
authority are unchanged. Historical ABI 11/12 descriptions of cyclic construction
are superseded by this pre-release contract. This change introduces no generic
storage redesign or replacement constructor naming scheme.
