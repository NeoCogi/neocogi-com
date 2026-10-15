#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering},
    };

    #[com_object(Counter)]
    /// Native fields are reachable only through construction-proven local ownership.
    struct CounterObject {
        /// Plain mutable data guarded by the generated COM gate.
        value: u32,
        /// Independent observation of exactly-once final value destruction.
        drops: Arc<AtomicUsize>,
    }
    impl Counter for CounterObject {
        fn value(&self) -> Result<u32, NcStatus> { Ok(self.value) }
        fn set_value(&mut self, value: u32) -> Result<(), NcStatus> { self.value = value; Ok(()) }
    }
    impl Drop for CounterObject {
        fn drop(&mut self) { self.drops.fetch_add(1, Ordering::SeqCst); }
    }
    #[com_object(Shared)]
    /// Shared interface demonstrates that generated weak results retain thread permission.
    struct SharedObject;
    impl Shared for SharedObject {
        fn value(&self) -> Result<u32, NcStatus> { Ok(73) }
    }
    #[com_object(Store)]
    /// Copies weak facts without owning their producers.
    struct StoreObject {
        /// The entire recursive value remains weak after acquisition.
        values: References,
        /// Shared producer's thread permission must survive lowering and transport.
        shared: WeakComPtr<SharedAbi, ThreadSafe>,
    }
    impl Store for StoreObject {
        fn get(&self) -> Result<WeakComPtr<CounterAbi>, NcStatus> { Ok(self.values.one.clone()) }
        fn capture(&mut self, values: NcValueRef<'_, References>) -> Result<(), NcStatus> {
            self.values = values.to_owned()?;
            Ok(())
        }
        fn shared(&self) -> Result<WeakComPtr<SharedAbi, ThreadSafe>, NcStatus> { Ok(self.shared.clone()) }
        /// Enum acquisition preserves the same weak ownership as record fields.
        fn event(&mut self, value: NcValueRef<'_, WeakEvent>) -> Result<(), NcStatus> {
            match value.to_owned()? {
                WeakEvent::Observed(observer) => self.values.one = observer,
                WeakEvent::Empty => {}
            }
            Ok(())
        }
    }
    /// All nested handles name the same allocation with no strong reference.
    fn references(one: &WeakComPtr<CounterAbi>) -> References {
        References { one: one.clone(), many: vec![one.clone()], fixed: [one.clone(), one.clone()] ,
        }
    }
    /// Creates an IDL implementation whose concrete and interface observers agree.
    fn counter() -> (LocalCom<CounterObject>, WeakComPtr<CounterAbi>, Arc<AtomicUsize>) {
        let drops = Arc::new(AtomicUsize::new(0));
        let local = CounterObject::into_local(CounterObject { value: 42, drops: drops.clone() });
        let weak = local.query::<CounterAbi>().unwrap().downgrade().unwrap();
        (local, weak, drops)
    }
    /// Owns only the store; both referenced producers may expire independently.
    fn store(weak: &WeakComPtr<CounterAbi>) -> ComPtr<StoreAbi> {
        let shared = SharedObject::into_local(SharedObject);
        StoreObject::from_value(StoreObject {
            values: references(weak),
            shared: shared.query::<SharedAbi>().unwrap().downgrade().unwrap(),
        }).query().unwrap()
    }

    /// Weak results and recursively borrowed fields work before and after final release.
    #[test]
    fn live_and_expired_tokens_round_trip_without_retaining_the_producer() {
        let (local, weak, drops) = counter();
        let local_weak = local.downgrade().unwrap();
        let mut store = store(&weak);
        let returned = store.get().unwrap();
        assert!(weak.same_object(&returned));
        returned.upgrade().unwrap().unwrap().set_value(43).unwrap();
        assert_eq!(local_weak.upgrade().unwrap().unwrap().try_borrow().unwrap().value, 43);
        let facts = references(&weak);
        store.capture(NcValueRef::new(&facts).unwrap()).unwrap();
        drop(local);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(local_weak.upgrade().unwrap().is_none());
        assert!(store.get().unwrap().upgrade().unwrap().is_none());
        store.capture(NcValueRef::new(&facts).unwrap()).unwrap();
        assert!(weak.same_object(&store.get().unwrap()));
        store.event(NcValueRef::new(&WeakEvent::Observed(weak.clone())).unwrap()).unwrap();
        store.event(NcValueRef::new(&WeakEvent::Empty).unwrap()).unwrap();
        assert!(store.get().unwrap().upgrade().unwrap().is_none());
        let copied = NcValueRef::new(&facts).unwrap().to_owned().unwrap();
        assert!(copied.many[0].upgrade().unwrap().is_none());
        assert!(copied.fixed[1].same_object(&weak));
        let neocogi_com::design::IdlShape::Struct(fields) = References::IDL_TYPE.shape else { panic!("record metadata") };
        assert_eq!(fields[0].ty, neocogi_com::design::IdlValueType::WeakObject("proof.weak.Counter"));
        // SAFETY: these are immutable generated descriptor graphs.
        let signature = unsafe { format_method_signature(&StoreAbi::METHOD_DESCRIPTORS[0]) }.unwrap();
        assert!(signature.contains("weak_com_ptr_out<"), "{signature}");
    }

    /// A generated result preserves a shared interface's cross-thread permission.
    #[test]
    fn shared_threading_survives_idl_lowering() {
        let (local, weak, _) = counter();
        let shared = SharedObject::into_local(SharedObject);
        let store = StoreObject::from_value(StoreObject {
            values: references(&weak),
            shared: shared.query::<SharedAbi>().unwrap().downgrade().unwrap(),
        }).query::<StoreAbi>().unwrap();
        let result = store.shared().unwrap();
        std::thread::spawn(move || assert_eq!(result.upgrade().unwrap().unwrap().value(), Ok(73))).join().unwrap();
        drop(shared);
        assert!(store.shared().unwrap().upgrade().unwrap().is_none());
        drop(local);
    }

    /// Malformed weak fields are valid raw storage with a null or wrong-type token.
    #[test]
    fn malformed_borrowed_tokens_fail_before_store_mutation() {
        let (_local, weak, _) = counter();
        let mut store = store(&weak);
        let facts = references(&weak);
        let encoded = NcValueRef::new(&facts).unwrap();
        // SAFETY: copy initialized C wire storage while its backing remains alive.
        let mut raw = unsafe { encoded.as_ptr().read() };
        raw.one = NcWeakRaw::null();
        // SAFETY: the record and every other field retain valid backing storage.
        assert_eq!(store.capture(unsafe { NcValueRef::from_raw(&raw) }.unwrap()), Err(NC_E_POINTER));
        let unrelated = SharedObject::into_local(SharedObject);
        let wrong = unrelated.query::<SharedAbi>().unwrap().downgrade().unwrap();
        raw.one = wrong.as_raw();
        assert_eq!(store.capture(unsafe { NcValueRef::from_raw(&raw) }.unwrap()), Err(NC_E_NOINTERFACE));
        assert!(store.get().unwrap().same_object(&weak));
    }

    /// An ABI success with a null weak result is rejected by the generated client.
    #[test]
    fn malformed_successful_weak_result_is_checked() {
        // SAFETY: this fixture supplies the required writable output record.
        unsafe extern "C" fn null_result(_: *mut core::ffi::c_void, out: *mut NcWeakRaw) -> NcStatus {
            unsafe { *out = NcWeakRaw::null(); }
            NC_OK
        }
        let (_local, weak, _) = counter();
        let owner = store(&weak);
        let mut table = StoreVtblFor::<StoreObject>::VTABLE;
        table.get = null_result;
        let raw = owner.into_raw();
        // SAFETY: the table preserves Store's identity and lifecycle, changes only
        // the test output, and remains alive until the adopted strong owner drops.
        let malformed = unsafe { ComPtr::<StoreAbi>::from_raw_owned(ComRaw { object: raw.object, vtable: &table }) }.unwrap();
        assert_eq!(malformed.get().unwrap_err(), NC_E_POINTER);
        drop(malformed);
    }
}
