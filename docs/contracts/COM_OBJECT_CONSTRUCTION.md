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

## Observer connections

A factory can connect application-defined observers after allocating a fully
initialized object and before returning its public interface. Obtain weak identity
through the selected ownership policy, then finish private wiring through checked
local access. The [host/provider example](../../examples/com-plugin/README.md)
uses ordinary COM interfaces for notifications and keeps its queue in the host.

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

Runtime, plugin descriptor and core ABI remain v1. Handle layouts,
threading and process-wide runtime authority are unchanged by typed storage.
The compiler owns the separate [canonical signature encoding](IDL_MODULES.md#canonical-signatures).
Earlier descriptions of cyclic construction are superseded by this pre-release
contract. The typed storage consolidation
below preserves the two constructor names for each ownership family.

## Typed storage and implementation proof

`ComObject` is the single unsafe provider-local contract for threading, compact
vtable bindings and checked concrete borrows. Generated objects no longer define
a separately named storage companion or implement `ComAbiObject` and
`ThreadSafeComObject`. Both primary-interface and explicit-list providers prove
threading through the same mode-dependent bound, including generic specializations.

Facade support owns `Storage<T, Access>`: shared providers select the existing
call gate; exclusive providers select zero-sized access and the common header's
combined owner/borrow state. The macro emits only bindings, small typed borrow
adapters and the selected ownership family's two constructors. Constructor support
is unsafe because the generated adapters and allocation must select exactly the
same concrete layout. This is construction proof, never a foreign-object downcast.

The common header remains at offset zero, values retain their concrete alignment,
and final weak release frees through the allocating provider. `MaybeUninit` keeps
storage valid after value destruction. Non-generic lifecycle algorithms and
synchronous exclusive destruction are unchanged. No shared/exclusive runtime
policy branch or second borrow gate is added. All process registration remains
inside the one host-owned `nc_com` runtime; typed storage has no registry state.

This change reduces generated unsafe layout code. Generic support still
monomorphizes; neither binary-size reduction nor faster dispatch follows from
source consolidation alone.
