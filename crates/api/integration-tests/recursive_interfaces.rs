//! Recursive contracts and independently compiled contract revisions must remain
//! usable without permitting calls through an incompatible projected interface.

use neocogi_com::{ComPtr, ComRef, IUnknown, NcStatus, NC_E_ABI_VERSION, NC_E_NOT_FOUND, com_object};
use std::{cell::Cell, rc::Rc};

/// First revision as understood by the consumer binary.
pub mod consumer {
    use super::*;

    #[allow(unused_imports)]
    pub use fixture_recursive_consumer::{
        ConsumerIBaseValueAbi as IBaseValueAbi, ConsumerIBaseValueVTable as IBaseValueVTable, ConsumerIBaseValueVtblFor as IBaseValueVtblFor,
        ConsumerIBaseValueClient as IBaseValueClient, ConsumerIBaseValue as IBaseValue,
    };

    #[allow(unused_imports)]
    pub use fixture_recursive_consumer::{
        ConsumerIValueAbi as IValueAbi, ConsumerIValueVTable as IValueVTable, ConsumerIValueVtblFor as IValueVtblFor, ConsumerIValueClient as IValueClient,
        ConsumerIValue as IValue,
    };
    impl IValue for Value {}

    #[allow(unused_imports)]
    pub use fixture_recursive_consumer::{
        ConsumerIExchangeAbi as IExchangeAbi, ConsumerIExchangeVTable as IExchangeVTable, ConsumerIExchangeVtblFor as IExchangeVtblFor,
        ConsumerIExchangeClient as IExchangeClient, ConsumerIExchange as IExchange,
    };

    #[com_object(interfaces(IValue))]
    /// Valid consumer-side value object used as a borrowed argument.
    pub struct Value;

    impl IBaseValue for Value {
        /// A mismatched receiver must never reach this method.
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(42)
        }
    }
}

/// Producer revision deliberately reuses the IID with an incompatible value ABI.
pub mod producer {
    use super::*;

    #[allow(unused_imports)]
    pub use fixture_recursive_producer::{
        ProducerIBaseValueAbi as IBaseValueAbi, ProducerIBaseValueVTable as IBaseValueVTable, ProducerIBaseValueVtblFor as IBaseValueVtblFor,
        ProducerIBaseValueClient as IBaseValueClient, ProducerIBaseValue as IBaseValue,
    };

    #[allow(unused_imports)]
    pub use fixture_recursive_producer::{
        ProducerIValueAbi as IValueAbi, ProducerIValueVTable as IValueVTable, ProducerIValueVtblFor as IValueVtblFor, ProducerIValueClient as IValueClient,
        ProducerIValue as IValue,
    };
    impl IValue for Value {}

    #[allow(unused_imports)]
    pub use fixture_recursive_producer::{
        ProducerIExchangeAbi as IExchangeAbi, ProducerIExchangeVTable as IExchangeVTable, ProducerIExchangeVtblFor as IExchangeVtblFor,
        ProducerIExchangeClient as IExchangeClient, ProducerIExchange as IExchange,
    };

    #[com_object(interfaces(IValue))]
    /// Tracks exactly-once final release of an incompatible owned return.
    pub struct Value {
        /// Shared test observation; stays inside this binary.
        pub drops: Rc<Cell<u32>>,
    }

    impl IBaseValue for Value {
        /// Never called by the consumer after its pointer check fails.
        fn value(&self) -> Result<u64, NcStatus> {
            Ok(42)
        }
    }

    impl Drop for Value {
        /// Records final allocation destruction.
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    #[com_object(interfaces(IExchange))]
    /// Exchange tracks dispatch and transferred-object destruction separately.
    pub struct Exchange {
        /// Number of calls entering the implementation after pointer validation.
        pub calls: Rc<Cell<u32>>,
        /// Number of returned value allocations finally released.
        pub drops: Rc<Cell<u32>>,
    }

    impl IExchange for Exchange {
        /// The generated thunk must reject the mismatch before reaching here.
        fn accept(&self, _value: ComRef<'_, IValueAbi>) -> Result<(), NcStatus> {
            self.calls.set(self.calls.get() + 1);
            Ok(())
        }

        /// Constructs a fresh producer value and transfers one reference.
        fn produce(&self) -> Result<ComPtr<IValueAbi>, NcStatus> {
            Value::from_value(Value { drops: self.drops.clone() }).query()
        }
    }
}

extern crate self as fixture_recursive_interfaces;
neocogi_com_test_contracts::recursive_interfaces!();
extern crate self as fixture_recursive_consumer;
neocogi_com_test_contracts::recursive_consumer!();
extern crate self as fixture_recursive_producer;
neocogi_com_test_contracts::recursive_producer!();

#[com_object(interfaces(INode))]
/// Stateless echo node, sufficient to exercise recursion and ownership.
pub struct Node;

impl INode for Node {
    /// The explicit AddRef creates exactly the reference transferred back.
    fn echo(&self, node: ComRef<'_, INodeAbi>) -> Result<ComPtr<INodeAbi>, NcStatus> {
        node.try_add_ref()
    }
}

#[com_object(interfaces(ILeft, IRight))]
/// Descriptors and both vtables must evaluate without a const cycle.
pub struct Pair;

impl ILeft for Pair {
    /// No stored peer is needed to verify contract construction.
    fn right(&self) -> Result<ComPtr<IRightAbi>, NcStatus> {
        Err(NC_E_NOT_FOUND)
    }
}

impl IRight for Pair {
    /// Leaves ownership acyclic while exercising the second thunk.
    fn left(&self) -> Result<ComPtr<ILeftAbi>, NcStatus> {
        Err(NC_E_NOT_FOUND)
    }
}

#[test]
fn self_and_mutual_references_compile_and_preserve_identity() {
    let node: ComPtr<INodeAbi> = Node::from_value(Node).query().unwrap();
    let echoed = node.echo(node.as_com_ref()).unwrap();
    assert_eq!(node.query::<IUnknown>().unwrap().as_raw(), echoed.query::<IUnknown>().unwrap().as_raw());
    let pair = Pair::from_value(Pair);
    assert_eq!(pair.query::<ILeftAbi>().unwrap().right().unwrap_err(), NC_E_NOT_FOUND);
    assert_eq!(pair.query::<IRightAbi>().unwrap().left().unwrap_err(), NC_E_NOT_FOUND);
}

/// Deliberately inconsistent dependency metadata exercises the raw boundary
/// which the ordinary IDL compiler will not synthesize with equal signatures.
static FOREIGN_EXCHANGE: consumer::IExchangeVTable = consumer::IExchangeVTable {
    unknown: neocogi_com::IUnknownVTable {
        interface_id: &<consumer::IExchangeAbi as neocogi_com::ComInterface>::INTERFACE_ID,
        ..producer::IExchangeVtblFor::<producer::Exchange>::VTABLE.unknown
    },
    accept: foreign_accept,
    produce: foreign_produce,
};

/// Forwards the same two-word C input to the real producer's checked thunk.
/// The negative test supplies readable, strongly retained consumer storage.
unsafe extern "C" fn foreign_accept(object: *mut core::ffi::c_void, value: neocogi_com::ComRaw<consumer::IValueAbi>) -> NcStatus {
    // SAFETY: object is the live concrete producer; marker casting preserves
    // both raw words. Its thunk checks the expected value identity before use.
    unsafe { (producer::IExchangeVtblFor::<producer::Exchange>::VTABLE.accept)(object, value.cast()) }
}

/// Publishes one owned, readable value with deliberately incompatible metadata.
unsafe extern "C" fn foreign_produce(object: *mut core::ffi::c_void, output: *mut neocogi_com::ComRaw<consumer::IValueAbi>) -> NcStatus {
    let mut value = neocogi_com::ComRaw::<producer::IValueAbi>::null();
    // SAFETY: object is retained by the test; the local output is aligned and
    // writable. Success transfers one real producer reference into value.
    let status = unsafe { (producer::IExchangeVtblFor::<producer::Exchange>::VTABLE.produce)(object, &mut value) };
    // SAFETY: the generated caller grants this output slot. The marker changes
    // no pointer/lifecycle representation; the client must reject and release it.
    unsafe { output.write(value.cast()) };
    status
}

#[test]
fn full_pointer_checks_reject_incompatible_arguments_and_release_owned_returns() {
    // Genuine dependency revisions change the complete surrounding contract.
    assert_ne!(consumer::IExchangeAbi::ABI_SIGNATURE, producer::IExchangeAbi::ABI_SIGNATURE);
    assert_ne!(consumer::IValueAbi::ABI_SIGNATURE, producer::IValueAbi::ABI_SIGNATURE);
    let calls = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let exchange_owner: ComPtr<producer::IExchangeAbi> = producer::Exchange::from_value(producer::Exchange {
        calls: calls.clone(),
        drops: drops.clone(),
    })
    .query()
    .unwrap();
    assert!(matches!(exchange_owner.query::<consumer::IExchangeAbi>(), Err(NC_E_ABI_VERSION)));
    assert_eq!(calls.get(), 0);
    // The raw fixture deliberately advertises the consumer exchange while
    // forwarding typed slots to a different value revision. Storage and shared
    // lifetime remain the real producer's; no incompatible Rust value is formed.
    let raw = exchange_owner.as_raw();
    let exchange = unsafe {
        ComRef::<consumer::IExchangeAbi>::__nc_from_raw_borrowed_checked(neocogi_com::ComRaw {
            object: raw.object,
            vtable: &FOREIGN_EXCHANGE,
        })
    }
    .unwrap();
    let value: ComPtr<consumer::IValueAbi> = consumer::Value::from_value(consumer::Value).query().unwrap();
    assert_eq!(exchange.accept(value.as_com_ref()), Err(NC_E_ABI_VERSION));
    assert_eq!(calls.get(), 0, "rejected arguments must not enter the implementation");
    // The failed borrowed call must leave its owner's reference alive.
    assert_eq!(value.value(), Ok(42));
    for expected in 1..=3 {
        assert_eq!(exchange.produce().unwrap_err(), NC_E_ABI_VERSION);
        assert_eq!(drops.get(), expected, "each rejected transfer is released exactly once");
    }
}
