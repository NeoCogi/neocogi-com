//! Recursive contracts and independently compiled contract revisions must remain
//! usable without permitting calls through an incompatible projected interface.

use neocogi_com::{ComPtr, ComRef, IUnknown, NcStatus, NC_E_ABI_VERSION, NC_E_NOT_FOUND, com_object, com_interface};
use std::{cell::Cell, rc::Rc};

/// First revision as understood by the consumer binary.
pub mod consumer {
    use super::*;

    #[com_interface(auto_guid = "nc.tests.recursion.BaseValue:v1", assigned_signature = b"00000000000000000000000000000016")]
    /// Consumer expects a 32-bit value.
    pub trait IBaseValue {
        /// Reads the old contract's scalar.
        fn value(&self) -> Result<u32, NcStatus>;
    }

    #[com_interface(auto_guid = "nc.tests.recursion.Value:v1", assigned_signature = b"00000000000000000000000000000017")]
    /// The empty derived contract changes its signature when the base changes.
    pub trait IValue: IBaseValue {}
    impl IValue for Value {}

    #[com_interface(auto_guid = "nc.tests.recursion.Exchange:v1", assigned_signature = b"00000000000000000000000000000018")]
    /// The surrounding local contract only names the value IID.
    pub trait IExchange {
        /// Calls must validate the value against the producer's full contract.
        fn accept(&self, value: ComRef<'_, IValueAbi>) -> Result<(), NcStatus>;
        /// Returned values must be checked against the consumer's full contract.
        fn produce(&self) -> Result<ComPtr<IValueAbi>, NcStatus>;
    }

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

    #[com_interface(auto_guid = "nc.tests.recursion.BaseValue:v1", assigned_signature = b"00000000000000000000000000000019")]
    /// The producer's revision returns 64 bits, so its full signature differs.
    pub trait IBaseValue {
        /// Reads the producer's scalar.
        fn value(&self) -> Result<u64, NcStatus>;
    }

    #[com_interface(auto_guid = "nc.tests.recursion.Value:v1", assigned_signature = b"00000000000000000000000000000020")]
    /// The empty derived contract changes its signature when the base changes.
    pub trait IValue: IBaseValue {}
    impl IValue for Value {}

    #[com_interface(auto_guid = "nc.tests.recursion.Exchange:v1", assigned_signature = b"00000000000000000000000000000018")]
    /// Same local method contract as the consumer, with producer-side checks.
    pub trait IExchange {
        /// Rejects a borrowed value with the wrong full identity before dispatch.
        fn accept(&self, value: ComRef<'_, IValueAbi>) -> Result<(), NcStatus>;
        /// Transfers a reference which the consumer must reject and release.
        fn produce(&self) -> Result<ComPtr<IValueAbi>, NcStatus>;
    }

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

#[com_interface(auto_guid = "nc.tests.recursion.Node:v1", assigned_signature = b"00000000000000000000000000000021")]
/// A natural self-reference previously failed constant evaluation.
pub trait INode {
    /// Retains and returns the borrowed node without interpreting its state.
    fn echo(&self, node: ComRef<'_, INodeAbi>) -> Result<ComPtr<INodeAbi>, NcStatus>;
}

#[com_object(interfaces(INode))]
/// Stateless echo node, sufficient to exercise recursion and ownership.
pub struct Node;

impl INode for Node {
    /// The explicit AddRef creates exactly the reference transferred back.
    fn echo(&self, node: ComRef<'_, INodeAbi>) -> Result<ComPtr<INodeAbi>, NcStatus> {
        Ok(node.add_ref())
    }
}

#[com_interface(auto_guid = "nc.tests.recursion.Left:v1", assigned_signature = b"00000000000000000000000000000022")]
/// First half of a mutual reference, without storing a strong ownership cycle.
pub trait ILeft {
    /// Provides the opposing capability when available.
    fn right(&self) -> Result<ComPtr<IRightAbi>, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.recursion.Right:v1", assigned_signature = b"00000000000000000000000000000023")]
/// Second half of a mutually recursive contract.
pub trait IRight {
    /// Provides the opposing capability when available.
    fn left(&self) -> Result<ComPtr<ILeftAbi>, NcStatus>;
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
    let node: ComPtr<INodeAbi> = Node::create().query().unwrap();
    let echoed = node.echo(node.as_com_ref()).unwrap();
    assert_eq!(node.query::<IUnknown>().unwrap().as_ptr(), echoed.query::<IUnknown>().unwrap().as_ptr());
    let pair = Pair::create();
    assert_eq!(pair.query::<ILeftAbi>().unwrap().right().unwrap_err(), NC_E_NOT_FOUND);
    assert_eq!(pair.query::<IRightAbi>().unwrap().left().unwrap_err(), NC_E_NOT_FOUND);
}

#[test]
fn full_pointer_checks_reject_incompatible_arguments_and_release_owned_returns() {
    assert_eq!(consumer::IExchangeAbi::ABI_SIGNATURE, producer::IExchangeAbi::ABI_SIGNATURE);
    assert_ne!(consumer::IValueAbi::ABI_SIGNATURE, producer::IValueAbi::ABI_SIGNATURE);
    let calls = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let exchange: ComPtr<consumer::IExchangeAbi> = producer::Exchange::from_value(producer::Exchange {
        calls: calls.clone(),
        drops: drops.clone(),
    })
    .query()
    .unwrap();
    let value: ComPtr<consumer::IValueAbi> = consumer::Value::create().query().unwrap();
    assert_eq!(exchange.accept(value.as_com_ref()), Err(NC_E_ABI_VERSION));
    assert_eq!(calls.get(), 0, "rejected arguments must not enter the implementation");
    // The failed borrowed call must leave its owner's reference alive.
    assert_eq!(value.value(), Ok(42));
    for expected in 1..=3 {
        assert_eq!(exchange.produce().unwrap_err(), NC_E_ABI_VERSION);
        assert_eq!(drops.get(), expected, "each rejected transfer is released exactly once");
    }
}
