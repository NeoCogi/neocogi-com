//! Repeated diamonds exercise physical storage, all-pairs queries, and one owner.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use neocogi_com::{com_object, ComInterface, IUnknown, NcInterfaceId, NcStatus, NC_E_ABI_VERSION, NC_E_BUSY, NC_E_NOINTERFACE, NC_OK};

extern crate self as fixture_diamond;
neocogi_com_test_contracts::diamond!();

impl<T: 'static> IA1 for Diamond<T> {}

impl<T: 'static> IB1 for Diamond<T> {}

impl<T: 'static> IA2 for Diamond<T> {}

impl<T: 'static> IB2 for Diamond<T> {}

impl<T: 'static> IA3 for Diamond<T> {}

impl<T: 'static> IB3 for Diamond<T> {}

impl<T: 'static> IA4 for Diamond<T> {}

impl<T: 'static> IB4 for Diamond<T> {}

impl<T: 'static> IA5 for Diamond<T> {}

impl<T: 'static> IB5 for Diamond<T> {}

impl<T: 'static> IA6 for Diamond<T> {}

impl<T: 'static> IB6 for Diamond<T> {}

#[com_object(ITop)]
/// Generic application storage proves interface storage is independent of Rust state type.
struct Diamond<T: 'static> {
    /// One ordinary mutable counter, shared by all inherited capabilities.
    value: u32,
    /// Arbitrary owned state destroyed exactly once by final Release.
    _lifetime: T,
}
impl<T: 'static> ITop for Diamond<T> {}
impl<T: 'static> IA0 for Diamond<T> {
    /// Reads through the allocation's shared lease.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}
impl<T: 'static> IB0 for Diamond<T> {
    /// Updates directly through the allocation's exclusive lease.
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus> {
        self.value = value;
        Ok(())
    }
}

/// Shared instrumentation observes destruction after every COM handle is gone.
struct Lifetime {
    /// An atomic observation outlives the concrete value without retaining it.
    drops: Arc<AtomicUsize>,
}
impl Drop for Lifetime {
    /// Counts one concrete destructor, independent of the number of interfaces.
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Measures real allocated payload placement without naming generated storage.
fn payload_offset<T: neocogi_com::ComObject>(local: &neocogi_com::LocalCom<T>) -> usize {
    let root = local.query::<IUnknown>().unwrap();
    let value = local.try_borrow().unwrap();
    (&*value as *const T) as usize - root.as_raw().object as usize
}

/// Interface count adds no payload storage; the common control header and call gate have fixed size.
#[test]
fn repeated_diamond_adds_no_per_object_interface_storage() {
    assert_eq!(ITopAbi::INTERFACE_CHAIN.len(), 15);
    assert_eq!(Diamond::<()>::COM_INTERFACE_COUNT, 16); // Includes canonical IUnknown.
    assert_eq!(
        payload_offset(&Diamond::into_local(Diamond { value: 0, _lifetime: () })),
        core::mem::size_of::<neocogi_com::lifetime::NcWeakControl>() + core::mem::size_of::<usize>()
    );
    // A second state specialization must use exactly the same contract set.
    assert_eq!(
        payload_offset(&Diamond::into_local(Diamond {
            value: 0,
            _lifetime: Lifetime { drops: Arc::new(AtomicUsize::new(0)) }
        })),
        core::mem::size_of::<neocogi_com::lifetime::NcWeakControl>() + core::mem::size_of::<usize>()
    );
    let object = Diamond::from_value(Diamond { value: 7, _lifetime: () });
    let top = object.query::<ITopAbi>().unwrap();
    assert_eq!(top.value(), Ok(7)); // Calls the inherited physical vtable prefix.
}

/// Calls the common raw query slot so every source/target pair is exercised.
///
/// # Safety
/// Source is a live projection of the retained test object. Success transfers
/// one reference; the caller must release it through the returned common prefix.
unsafe fn query(source: neocogi_com::ComRaw<IUnknown>, identity: &NcInterfaceId) -> Result<neocogi_com::ComRaw<IUnknown>, NcStatus> {
    // Begin non-null so failures must actively clear the caller's output slot.
    let mut result = source.cast();
    // SAFETY: the fixture supplies a live allocation and valid independent argument slots.
    let status = unsafe { ((*source.vtable).query_interface)(source.object, identity, &mut result) };
    match status {
        NC_OK => {
            assert!(!result.is_null());
            Ok(result.cast())
        }
        status => {
            assert!(result.is_null(), "failed queries must clear the output");
            Err(status)
        }
    }
}

/// Every successful raw acquisition balances exactly one owned reference.
///
/// # Safety
/// Source owns one reference returned by query; this consumes it exactly once.
unsafe fn release(source: neocogi_com::ComRaw<IUnknown>) {
    // SAFETY: source's complete vtable and allocation remain live until this release.
    unsafe {
        ((*source.vtable).release)(source.object);
    }
}

/// Every pair selects the same canonical addresses and one reference-count identity.
#[test]
fn all_projection_pairs_share_addresses_state_and_lifetime() {
    let drops = Arc::new(AtomicUsize::new(0));
    let object = Diamond::from_value(Diamond {
        value: 5,
        _lifetime: Lifetime { drops: drops.clone() },
    });
    let identities = core::iter::once(IUnknown::INTERFACE_ID)
        .chain(
            ITopAbi::INTERFACE_CHAIN
                .iter()
                .map(|descriptor| NcInterfaceId::new(descriptor.interface_id, descriptor.abi_signature)),
        )
        .collect::<Vec<_>>();
    let mut canonical = Vec::new();
    // SAFETY: object owns the allocation throughout all raw query/release pairs.
    unsafe {
        for identity in &identities {
            let projection = query(object.as_raw(), identity).unwrap();
            canonical.push(projection);
            release(projection);
        }
        for source in &identities {
            let source = query(object.as_raw(), source).unwrap();
            for (index, target) in identities.iter().enumerate() {
                let target = query(source, target).unwrap();
                assert_eq!(target, canonical[index]);
                let unknown = query(target, &IUnknown::INTERFACE_ID).unwrap();
                assert_eq!(unknown, object.as_raw());
                release(unknown);
                release(target);
            }
            // Failure must transfer no reference and must clear its output.
            let incompatible = NcInterfaceId::new(IA0Abi::IID, neocogi_com::NcAbiSignature::from_static(b"99999999999999999999999999999999"));
            assert_eq!(query(source, &incompatible), Err(NC_E_ABI_VERSION));
            let absent = NcInterfaceId::new(neocogi_com::guid_from_qualified_name("test.diamond.Absent:v1"), IA0Abi::ABI_SIGNATURE);
            assert_eq!(query(source, &absent), Err(NC_E_NOINTERFACE));
            release(source);
        }
    }
    let mut writer = object.query::<IB0Abi>().unwrap();
    let reader = object.query::<IA0Abi>().unwrap();
    writer.set_value(42).unwrap();
    assert_eq!(reader.value(), Ok(42));
    drop((object, writer));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(reader.value(), Ok(42));
    drop(reader);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Duplicate ancestry must not create a second authority for borrowing application state.
#[test]
fn every_projection_uses_the_same_exclusive_borrow_gate() {
    let local = Diamond::into_local(Diamond { value: 8, _lifetime: () });
    let top = local.query::<ITopAbi>().unwrap();
    let reader = local.query::<IA0Abi>().unwrap();
    let mut writer = local.query::<IB0Abi>().unwrap();
    {
        let mut lease = local.try_borrow_mut().unwrap();
        lease.value = 17;
        assert_eq!(top.value(), Err(NC_E_BUSY));
        assert_eq!(reader.value(), Err(NC_E_BUSY));
        assert_eq!(writer.set_value(19), Err(NC_E_BUSY));
    }
    {
        let lease = local.try_borrow().unwrap();
        assert_eq!(reader.value(), Ok(lease.value));
        assert_eq!(writer.set_value(19), Err(NC_E_BUSY));
    }
    writer.set_value(19).unwrap();
    assert_eq!(top.value(), Ok(19));
}

/// Different modules can use the same local trait spelling with distinct identities.
mod first {
    #[allow(unused_imports)]
    pub use super::{
        FirstIValueAbi as IValueAbi, FirstIValueVTable as IValueVTable, FirstIValueVtblFor as IValueVtblFor, FirstIValueClient as IValueClient,
        FirstIValue as IValue,
    };
}
/// The second spelling must not collide through an invented storage field name.
mod second {
    #[allow(unused_imports)]
    pub use super::{
        SecondIValueAbi as IValueAbi, SecondIValueVTable as IValueVTable, SecondIValueVtblFor as IValueVtblFor, SecondIValueClient as IValueClient,
        SecondIValue as IValue,
    };
}
#[com_object(interfaces(first::IValue, second::IValue))]
/// A flat object uses identities instead of derived per-interface field names.
struct Namespaced;
impl first::IValue for Namespaced {}
impl second::IValue for Namespaced {}

/// Same-named interface paths no longer require an artificial authoring restriction.
#[test]
fn distinct_qualified_interfaces_can_share_a_final_name() {
    let object = Namespaced::from_value(Namespaced);
    let first = object.query::<first::IValueAbi>().unwrap();
    let second = first.query::<second::IValueAbi>().unwrap();
    assert_eq!(second.query::<IUnknown>().unwrap().as_raw(), object.as_raw());
    assert_eq!(Namespaced::COM_INTERFACE_COUNT, 3);
}

/// The ABI conflict must be diagnosed by compilation before publication is possible.
#[test]
fn conflicting_diamond_cannot_construct_an_object() {
    trybuild::TestCases::new().compile_fail("integration-tests/ui/interface_conflicting_diamond.rs");
}

#[com_object(interfaces())]
/// An owner with no public capabilities still has a canonical IUnknown projection.
struct UnknownOnly<T: 'static> {
    /// Generic local state must not be needed to infer the absent provider list.
    value: T,
}

/// Empty authored interface lists must compile and retain ordinary local access.
#[test]
fn unknown_only_generic_owner_has_no_interface_storage() {
    let owner = UnknownOnly::into_local(UnknownOnly { value: 42_u32 });
    let unknown = owner.query::<IUnknown>().unwrap();
    assert_eq!(UnknownOnly::<u32>::COM_INTERFACE_COUNT, 1);
    assert_eq!(owner.try_borrow().unwrap().value, 42);
    assert!(matches!(unknown.query::<IA0Abi>(), Err(NC_E_NOINTERFACE)));
    assert_eq!(
        payload_offset(&owner),
        core::mem::size_of::<neocogi_com::lifetime::NcWeakControl>() + core::mem::size_of::<usize>()
    );
}

#[com_object(interfaces(IA0, IB0, IA1))]
/// Explicit roots can overlap ancestors of another explicitly listed interface.
struct OverlappingRoots {
    /// Shared counter proves the selected providers use this exact implementation.
    value: u32,
}
impl IA0 for OverlappingRoots {
    /// Reads the counter through either the explicit root or inherited prefix.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}
impl IB0 for OverlappingRoots {
    /// Mutates the same state without a per-projection receiver object.
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus> {
        self.value = value;
        Ok(())
    }
}
impl IA1 for OverlappingRoots {}

/// Deduplication applies across authored roots as well as inside a single diamond.
#[test]
fn explicit_roots_reuse_their_inherited_contracts() {
    let object = OverlappingRoots::from_value(OverlappingRoots { value: 7 });
    let root = object.query::<IA0Abi>().unwrap();
    let child = object.query::<IA1Abi>().unwrap();
    assert_eq!(child.query::<IA0Abi>().unwrap().as_raw(), root.as_raw());
    assert_eq!(child.value(), Ok(7));
    assert_eq!(OverlappingRoots::COM_INTERFACE_COUNT, 4);
    assert_eq!(
        payload_offset(&OverlappingRoots::into_local(OverlappingRoots { value: 0 })),
        core::mem::size_of::<neocogi_com::lifetime::NcWeakControl>() + core::mem::size_of::<usize>()
    );
}

/// Forces extra padding before concrete state; interface count must not affect
/// the allocation address or its required alignment.
#[repr(align(64))]
struct AlignedState {
    /// Application state remains directly accessible through the local owner.
    marker: u64,
}

/// Direct object addresses must preserve alignment without assuming the state offset.
#[test]
fn generic_aligned_state_keeps_direct_object_access_correct() {
    let local = Diamond::into_local(Diamond {
        value: 17,
        _lifetime: AlignedState { marker: 5 },
    });
    let top = local.query::<ITopAbi>().unwrap();
    let base = top.query::<IA0Abi>().unwrap();
    let unknown = top.query::<IUnknown>().unwrap();
    // Every queried view carries the aligned allocation address unchanged.
    let owner = unknown.as_raw().object;
    assert_eq!(top.as_raw().object, owner);
    assert_eq!(base.as_raw().object, owner);
    assert_eq!(
        owner as usize % core::mem::align_of::<neocogi_com::__ComSharedStorage<Diamond<AlignedState>>>(),
        0
    );
    {
        let mut value = local.try_borrow_mut().unwrap();
        value.value = 29;
        value._lifetime.marker = 11;
    }
    assert_eq!(top.value(), Ok(29));
    assert_eq!(local.try_borrow().unwrap()._lifetime.marker, 11);
}
