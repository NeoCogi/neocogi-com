//! Single-base layout, identity and ownership checks across generated interfaces.

#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
use neocogi_com::{com_object, ComInterface, ComPtr, ComRef, IUnknown, NcStatus, NC_E_NOINTERFACE};
use std::{cell::Cell, rc::Rc};

/// Interface verbs belong to the contract, rather than boolean handle probes.
mod predicate {
    use super::*;

    #[allow(unused_imports)]
    pub use fixture_inheritance::{
        PredicateIPredicateAbi as IPredicateAbi, PredicateIPredicateVTable as IPredicateVTable, PredicateIPredicateVtblFor as IPredicateVtblFor,
        PredicateIPredicateClient as IPredicateClient, PredicateIPredicate as IPredicate,
    };

    #[com_object(IPredicate)]
    /// Stateless provider reached through both owning and borrowed clients.
    struct Predicate;

    impl IPredicate for Predicate {
        fn is(&self) -> Result<bool, NcStatus> {
            Ok(true)
        }
    }

    /// Compiles the contract and calls its real generated ABI in both views.
    #[test]
    fn interface_predicate_is_not_shadowed_by_a_handle_helper() {
        let owner = Predicate::from_value(Predicate).query::<IPredicateAbi>().unwrap();
        assert_eq!(owner.is(), Ok(true));
        assert_eq!(owner.as_com_ref().is(), Ok(true));
    }
}

/// A separate module exercises fully qualified base companion paths.
mod contract {
    use super::*;

    #[allow(unused_imports)]
    pub use fixture_inheritance::{
        ContractIWidgetAbi as IWidgetAbi, ContractIWidgetVTable as IWidgetVTable, ContractIWidgetVtblFor as IWidgetVtblFor,
        ContractIWidgetClient as IWidgetClient, ContractIWidget as IWidget,
    };
}

extern crate self as fixture_inheritance;
test_contracts::inheritance!();

#[com_object(interfaces(IToggle))]
#[derive(Default)]
/// Concrete storage exposing the complete inherited interface chain.
struct Toggle {
    /// Authoritative state shared by every COM projection.
    value: u32,
    /// Observation retained after the one allocation is finally released.
    drops: Rc<Cell<u32>>,
}

impl Drop for Toggle {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

impl contract::IWidget for Toggle {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
    fn button(&self) -> Result<ComPtr<IButtonAbi>, NcStatus> {
        Toggle::from_value(Toggle {
            value: self.value,
            drops: Rc::new(Cell::new(0)),
        })
        .query()
    }
}

impl IAudit for Toggle {
    /// Reads the same implementation value through the sibling interface table.
    fn active(&self) -> Result<bool, NcStatus> {
        Ok(self.value != 0)
    }
}

impl IButton for Toggle {
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus> {
        self.value = value;
        Ok(())
    }
}

impl IToggle for Toggle {
    fn copy_value(&mut self, source: ComRef<'_, contract::IWidgetAbi>) -> Result<(), NcStatus> {
        self.value = source.value()?;
        Ok(())
    }
}

#[test]
fn inherited_methods_and_queries_share_one_allocation() {
    let drops = Rc::new(Cell::new(0));
    let unknown = Toggle::from_value(Toggle { value: 7, drops: drops.clone() });
    let mut toggle = unknown.query::<IToggleAbi>().unwrap();
    let button = toggle.query::<IButtonAbi>().unwrap();
    let widget = toggle.query::<contract::IWidgetAbi>().unwrap();
    assert_eq!(toggle.value(), Ok(7));
    toggle.set_value(19).unwrap();
    assert_eq!(widget.value(), Ok(19));
    assert_eq!(button.value(), Ok(19));
    let source = Toggle::from_value(Toggle { value: 23, drops: Rc::new(Cell::new(0)) })
        .query::<contract::IWidgetAbi>()
        .unwrap();
    toggle.copy_value(source.as_com_ref()).unwrap();
    assert_eq!(toggle.button().unwrap().value(), Ok(23));
    assert_eq!(widget.query::<IUnknown>().unwrap().as_raw(), unknown.as_raw());
    assert_eq!(button.query::<IUnknown>().unwrap().as_raw(), unknown.as_raw());
    assert_eq!(widget.as_raw(), button.query::<contract::IWidgetAbi>().unwrap().as_raw());
    assert_eq!(Toggle::COM_INTERFACE_COUNT, 5);
    let audit = toggle.query::<IAuditAbi>().unwrap();
    assert_eq!(audit.active(), Ok(true));
    assert_eq!(audit.query::<IUnknown>().unwrap().as_raw(), unknown.as_raw());
    drop(audit);
    // Independent physical base projections advertise their own exact identity.
    unsafe {
        neocogi_com::raw::validate_interface::<IToggleAbi>(toggle.as_raw()).unwrap();
        neocogi_com::raw::validate_interface::<contract::IWidgetAbi>(widget.as_raw()).unwrap();
        assert_eq!(
            neocogi_com::raw::validate_interface::<contract::IWidgetAbi>(toggle.as_raw().cast()),
            Err(NC_E_NOINTERFACE)
        );
    }
    drop((unknown, toggle, button));
    assert_eq!(drops.get(), 0);
    drop(widget);
    assert_eq!(drops.get(), 1);
}

#[test]
fn inherited_vtable_is_a_physical_prefix_and_signs_the_base() {
    assert_eq!(std::mem::offset_of!(IButtonVTable, base), 0);
    assert_eq!(std::mem::offset_of!(IButtonVTable, set_value), std::mem::size_of::<contract::IWidgetVTable>());
    assert_eq!(std::mem::offset_of!(IToggleVTable, copy_value), std::mem::size_of::<IButtonVTable>());
    assert_eq!(IButtonAbi::INTERFACE_DESCRIPTOR.base, contract::IWidgetAbi::INTERFACE_ID);
    assert_eq!(IToggleAbi::INTERFACE_DESCRIPTOR.base, IButtonAbi::INTERFACE_ID);
    assert_eq!(IToggleAbi::INTERFACE_CHAIN.len(), 4);
}

#[path = "fixtures/diamond.rs"]
mod diamond;

/// Contract selection changes only the table; every view retains one allocation.
#[test]
fn interface_pairs_share_one_object_without_embedded_headers() {
    use neocogi_com::ComRaw;
    let word = core::mem::size_of::<usize>();
    assert_eq!(core::mem::size_of::<ComPtr<IToggleAbi>>(), 2 * word);
    assert_eq!(core::mem::size_of::<Option<ComPtr<IToggleAbi>>>(), 2 * word);
    assert_eq!(core::mem::size_of::<ComRef<'_, IToggleAbi>>(), 2 * word);
    assert_eq!(core::mem::size_of::<Option<ComRef<'_, IToggleAbi>>>(), 2 * word);
    assert_eq!(core::mem::size_of::<ComRaw<IToggleAbi>>(), 2 * word);
    let owner = Toggle::from_value(Toggle { value: 41, drops: Rc::new(Cell::new(0)) });
    let toggle = owner.query::<IToggleAbi>().unwrap();
    let button = owner.query::<IButtonAbi>().unwrap();
    let widget = owner.query::<contract::IWidgetAbi>().unwrap();
    let audit = owner.query::<IAuditAbi>().unwrap();
    let projections = [
        owner.as_raw(),
        toggle.as_raw().cast::<IUnknown>(),
        button.as_raw().cast::<IUnknown>(),
        widget.as_raw().cast::<IUnknown>(),
        audit.as_raw().cast::<IUnknown>(),
    ];
    let identities = [
        IUnknown::INTERFACE_ID,
        IToggleAbi::INTERFACE_ID,
        IButtonAbi::INTERFACE_ID,
        contract::IWidgetAbi::INTERFACE_ID,
        IAuditAbi::INTERFACE_ID,
    ];
    for (projection, identity) in projections.into_iter().zip(identities) {
        assert_eq!(projection.object, owner.as_raw().object);
        // SAFETY: each queried pair retains its immutable matching table.
        assert_eq!(unsafe { *(*projection.vtable).interface_id }, identity);
    }
    assert_ne!(toggle.as_raw().vtable.cast::<()>(), button.as_raw().vtable.cast::<()>());
    assert_eq!(toggle.value(), Ok(41));
    assert_eq!(button.value(), Ok(41));
    assert_eq!(widget.value(), Ok(41));
    // The common lifetime header and one call gate precede Rust state;
    // adding interface projections contributes no per-object fields.
    let local = Toggle::into_local(Toggle::default());
    let root = local.query::<IUnknown>().unwrap();
    let state = local.try_borrow().unwrap();
    let value_address = (&*state as *const Toggle) as usize;
    assert_eq!(
        value_address - root.as_raw().object as usize,
        core::mem::size_of::<neocogi_com::lifetime::NcWeakControl>() + word
    );
}

/// Required borrowed pairs reject every null shape before generated dispatch.
mod pair_inputs {
    use neocogi_com::{com_object, ComPtr, ComRaw, ComRef, IUnknown, NcStatus, NC_E_POINTER, NC_OK};

    #[allow(unused_imports)]
    pub use fixture_inheritance::{
        PairInputsIAcceptPairAbi as IAcceptPairAbi, PairInputsIAcceptPairVTable as IAcceptPairVTable, PairInputsIAcceptPairVtblFor as IAcceptPairVtblFor,
        PairInputsIAcceptPairClient as IAcceptPairClient, PairInputsIAcceptPair as IAcceptPair,
    };

    #[com_object(IAcceptPair)]
    /// Local state can be inspected through the same borrow gate as its COM calls.
    struct Receiver {
        /// Calls that reached the typed implementation after raw validation.
        calls: usize,
    }
    impl IAcceptPair for Receiver {
        /// Uses an ordinary mutable borrow; raw input validation precedes this method.
        fn accept(&mut self, value: ComRef<'_, IUnknown>) -> Result<bool, NcStatus> {
            self.calls += 1;
            let _ = value;
            Ok(true)
        }
    }

    /// Required interface inputs admit exactly a complete present pair.
    #[test]
    fn required_pairs_validate_both_words_before_entering_rust() {
        let local = Receiver::into_local(Receiver { calls: 0 });
        let receiver = local.query::<IAcceptPairAbi>().unwrap();
        let root: ComPtr<IUnknown> = receiver.query().unwrap();
        let source = receiver.as_raw();
        let present = root.as_raw();
        let mut output = 99u8;
        // SAFETY: source/output are live; rejected pairs require no pointer reads.
        unsafe {
            for malformed in [
                ComRaw::null(),
                ComRaw {
                    object: present.object,
                    vtable: core::ptr::null(),
                },
                ComRaw {
                    object: core::ptr::null_mut(),
                    vtable: present.vtable,
                },
            ] {
                output = 99;
                assert_eq!(((*source.vtable).accept)(source.object, malformed, &mut output), NC_E_POINTER);
                assert_eq!(output, 99);
            }
            assert_eq!(((*source.vtable).accept)(source.object, present, &mut output), NC_OK);
        }
        assert_eq!(output, 1);
        assert_eq!(local.try_borrow().unwrap().calls, 1);
    }
}
