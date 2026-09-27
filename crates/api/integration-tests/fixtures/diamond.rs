//! Repeated diamonds exercise physical storage, all-pairs queries, and one owner.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use neocogi_com::{com_interface, com_object, ComInterface, IUnknown, NcInterfaceId, NcStatus, NC_E_ABI_VERSION, NC_E_BUSY, NC_E_NOINTERFACE, NC_OK};

#[com_interface(auto_guid = "test.diamond.A0:v1", assigned_signature = b"00000000000000000000000000000001")]
/// Shared read capability reached by every left and right ancestry path.
trait IA0 {
    /// Reads the one concrete counter under the common borrow gate.
    fn value(&self) -> Result<u32, NcStatus>;
}
#[com_interface(auto_guid = "test.diamond.B0:v1", assigned_signature = b"00000000000000000000000000000002")]
/// Shared mutation capability; the implementation uses exclusive mutable access.
trait IB0 {
    /// Replaces the one concrete counter through any acquired projection.
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus>;
}

/// Adds two distinct contracts requiring the same two parents at each level.
macro_rules! level {
    ($left:ident, $right:ident, $base_left:ident, $base_right:ident, $left_id:literal, $right_id:literal) => {
        #[com_interface(auto_guid = $left_id, assigned_signature = b"00000000000000000000000000000003")]
        /// Left branch of one repeated diamond; no additional method slots.
        trait $left: $base_left + $base_right {}
        #[com_interface(auto_guid = $right_id, assigned_signature = b"00000000000000000000000000000004")]
        /// Right branch shares both parents instead of owning separate state.
        trait $right: $base_left + $base_right {}
        impl<T: 'static> $left for Diamond<T> {}
        impl<T: 'static> $right for Diamond<T> {}
    };
}
level!(IA1, IB1, IA0, IB0, "test.diamond.A1:v1", "test.diamond.B1:v1");
level!(IA2, IB2, IA1, IB1, "test.diamond.A2:v1", "test.diamond.B2:v1");
level!(IA3, IB3, IA2, IB2, "test.diamond.A3:v1", "test.diamond.B3:v1");
level!(IA4, IB4, IA3, IB3, "test.diamond.A4:v1", "test.diamond.B4:v1");
level!(IA5, IB5, IA4, IB4, "test.diamond.A5:v1", "test.diamond.B5:v1");
level!(IA6, IB6, IA5, IB5, "test.diamond.A6:v1", "test.diamond.B6:v1");
#[com_interface(auto_guid = "test.diamond.Top:v1", assigned_signature = b"00000000000000000000000000000005")]
/// Complete 15-contract graph formerly expanded to 255 physical headers.
trait ITop: IA6 + IB6 {}

#[com_object(ITop)]
/// Generic application storage proves header count is independent of Rust state type.
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
    /// Counts one concrete destructor, independent of the number of headers.
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// The actual generated storage reserves exactly one slot per distinct contract.
#[test]
fn repeated_diamond_has_linear_physical_storage() {
    assert_eq!(ITopAbi::INTERFACE_CHAIN.len(), 15);
    assert_eq!(Diamond::<()>::COM_INTERFACE_COUNT, 16); // Includes canonical IUnknown.
    let header_bytes = core::mem::size_of::<IUnknown>();
    let start = core::mem::offset_of!(DiamondComObject<()>, interfaces);
    let end = core::mem::offset_of!(DiamondComObject<()>, value);
    assert_eq!(end - start, 16 * header_bytes);
    // A second state specialization must use exactly the same header set.
    assert_eq!(core::mem::offset_of!(DiamondComObject<Lifetime>, value) - start, 16 * header_bytes);
    let object = Diamond::from_value(Diamond { value: 7, _lifetime: () });
    let top = object.query::<ITopAbi>().unwrap();
    assert_eq!(top.value(), Ok(7)); // Calls the inherited physical vtable prefix.
}

/// Calls the common raw query slot so every source/target pair is exercised.
///
/// # Safety
/// Source is a live projection of the retained test object. Success transfers
/// one reference; the caller must release it through the returned common prefix.
unsafe fn query(source: *mut IUnknown, identity: &NcInterfaceId) -> Result<*mut IUnknown, NcStatus> {
    // Begin non-null so failures must actively clear the caller's output slot.
    let mut result = source.cast();
    // SAFETY: the fixture supplies a live header and valid independent argument slots.
    let status = unsafe { ((*(*source).vtable).query_interface)(source, identity, &mut result) };
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
unsafe fn release(source: *mut IUnknown) {
    // SAFETY: source's complete vtable and allocation remain live until this release.
    unsafe {
        ((*(*source).vtable).release)(source);
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
            let projection = query(object.as_mut_ptr(), identity).unwrap();
            canonical.push(projection);
            release(projection);
        }
        for source in &identities {
            let source = query(object.as_mut_ptr(), source).unwrap();
            for (index, target) in identities.iter().enumerate() {
                let target = query(source, target).unwrap();
                assert_eq!(target, canonical[index]);
                let unknown = query(target, &IUnknown::INTERFACE_ID).unwrap();
                assert_eq!(unknown, object.as_mut_ptr());
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
    use super::*;
    #[com_interface(auto_guid = "test.diamond.first.Value:v1", assigned_signature = b"00000000000000000000000000000006")]
    /// One independent capability.
    pub trait IValue {}
}
/// The second spelling must not collide through an invented storage field name.
mod second {
    use super::*;
    #[com_interface(auto_guid = "test.diamond.second.Value:v1", assigned_signature = b"00000000000000000000000000000007")]
    /// A different capability sharing only its unqualified Rust name.
    pub trait IValue {}
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
    assert_eq!(second.query::<IUnknown>().unwrap().as_mut_ptr(), object.as_mut_ptr());
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
fn unknown_only_generic_owner_has_one_header() {
    let owner = UnknownOnly::into_local(UnknownOnly { value: 42_u32 });
    let unknown = owner.query::<IUnknown>().unwrap();
    assert_eq!(UnknownOnly::<u32>::COM_INTERFACE_COUNT, 1);
    assert_eq!(owner.try_borrow().unwrap().value, 42);
    assert!(matches!(unknown.query::<IA0Abi>(), Err(NC_E_NOINTERFACE)));
    let start = core::mem::offset_of!(UnknownOnlyComObject<u32>, interfaces);
    let end = core::mem::offset_of!(UnknownOnlyComObject<u32>, value);
    assert_eq!(end - start, core::mem::size_of::<IUnknown>());
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
fn explicit_roots_reuse_their_inherited_headers() {
    let object = OverlappingRoots::from_value(OverlappingRoots { value: 7 });
    let root = object.query::<IA0Abi>().unwrap();
    let child = object.query::<IA1Abi>().unwrap();
    assert_eq!(child.query::<IA0Abi>().unwrap().as_mut_ptr(), root.as_mut_ptr());
    assert_eq!(child.value(), Ok(7));
    assert_eq!(OverlappingRoots::COM_INTERFACE_COUNT, 4);
    let start = core::mem::offset_of!(OverlappingRootsComObject, interfaces);
    let end = core::mem::offset_of!(OverlappingRootsComObject, value);
    assert_eq!(end - start, 4 * core::mem::size_of::<IUnknown>());
}

/// Forces padding between the header array and concrete state without changing
/// the required alignment of each individual COM interface header.
#[repr(align(64))]
struct AlignedState {
    /// Application state remains directly accessible through the local owner.
    marker: u64,
}

/// Owner recovery must use header placement, not assume where padded Rust state starts.
#[test]
fn generic_aligned_state_keeps_owner_recovery_and_mutable_access_correct() {
    use neocogi_com::__nc_object_from_interface;
    let local = Diamond::into_local(Diamond {
        value: 17,
        _lifetime: AlignedState { marker: 5 },
    });
    let top = local.query::<ITopAbi>().unwrap();
    let base = top.query::<IA0Abi>().unwrap();
    let unknown = top.query::<IUnknown>().unwrap();
    // SAFETY: these independently acquired projections retain one live allocation.
    unsafe {
        let owner = __nc_object_from_interface(unknown.as_mut_ptr());
        assert_eq!(__nc_object_from_interface(top.as_mut_ptr().cast()), owner);
        assert_eq!(__nc_object_from_interface(base.as_mut_ptr().cast()), owner);
        assert_eq!(owner as usize % core::mem::align_of::<DiamondComObject<AlignedState>>(), 0);
    }
    {
        let mut value = local.try_borrow_mut().unwrap();
        value.value = 29;
        value._lifetime.marker = 11;
    }
    assert_eq!(top.value(), Ok(29));
    assert_eq!(local.try_borrow().unwrap()._lifetime.marker, 11);
}
