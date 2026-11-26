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
   concrete access or the existing owning record. Shared interface owners can
   supply weak identities. Boxed owners supply loans from the live owner.
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

Both storage policies publish a complete initialized value. Shared ownership
counts strong and weak references. The implicit weak reference retains
storage through the destructor. A box has one owner, which can lend from
its value. Its destructor runs after those loans end. Provider-local
callbacks destroy the value once and deallocate the original storage.

## Typed storage and implementation proof

The generated `ComObject` contract supplies exact tables, threading evidence
and storage recovery for shared receivers. Boxed implementations additionally
supply `ComExclusiveObject`, which recovers an exclusive target borrower.
The caller and every input must satisfy the declared liveness and aliasing
rules. The generated provider bound checks mutable slots throughout the
interface's inheritance graph.

`LocalCom<T>` derives shared access from its live strong owner. `LocalBox<T>`
derives shared or exclusive access from its unique owner. Neither uses an
independent observer to access boxed fields. Constructing raw pointers or
querying foreign interfaces provides no proof of the concrete Rust type.
Typed storage keeps the private payload aligned and includes a
`MaybeUninit<T>` slot so storage release cannot drop the value twice.

See the [typed lifetime contracts](COM_LIFETIME_ADMISSION.md) and
[ABI specification](../abi.md) for raw ownership, reborrowing and interfaces.
