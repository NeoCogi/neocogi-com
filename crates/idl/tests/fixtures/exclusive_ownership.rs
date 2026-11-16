#[cfg(test)]
mod exclusive_tests {
    use super::*;
    use neocogi_com::{com_object, ComBox, WeakComBox, NcStatus};

    #[com_object(IValue, ownership = exclusive)]
    /// Concrete payload whose owner travels through generated method signatures.
    struct Value;
    impl IValue for Value {
        /// Read succeeds only while the exclusive owner still exists.
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(42)
        }
    }
    #[com_object(Factory)]
    /// Shared service creates exclusive widget-like values.
    struct Creator;
    impl Factory for Creator {
        /// Selects the result projection by consuming the freshly allocated owner.
        fn create(&self) -> Result<ComBox<IValueAbi>, NcStatus> {
            Value::into_box(Value).query().map_err(|(status, _unpublished)| status)
        }
    }
    #[com_object(IContainer, ownership = exclusive)]
    /// Sole child ownership lives in one ordinary field.
    struct Container {
        child: Option<ComBox<IValueAbi>>,
    }
    impl IContainer for Container {
        /// Replaces ownership without exposing a detached previous child.
        fn insert(&mut self, child: neocogi_com::ComBox<IValueAbi>) -> Result<(), (NcStatus, neocogi_com::ComBox<IValueAbi>)> {
            self.child = Some(child);
            Ok(())
        }
        /// Gives callers weak access without an independent owner.
        fn child(&self) -> Result<WeakComBox<IValueAbi>, NcStatus> {
            self.child.as_ref().map(ComBox::downgrade).ok_or(neocogi_com::NC_E_NOT_FOUND)
        }
        /// Ordinary destruction replaces fallible domain closure.
        fn remove(&mut self) -> Result<(), NcStatus> {
            self.child = None;
            Ok(())
        }
    }
    /// No Rust-only shortcut participates in factory creation, insertion, or lookup.
    #[test]
    fn factory_container_and_inspection_agree_on_ownership() {
        let factory = Creator::from_value(Creator).query::<FactoryAbi>().unwrap();
        let mut parent = Container::into_box(Container { child: None }).query::<IContainerAbi>().unwrap();
        let child = factory.create().unwrap();
        let weak = child.downgrade();
        parent.insert(child).unwrap();
        assert!(parent.child().unwrap().same_object(&weak));
        assert_eq!(weak.value(), Ok(42));
        parent.remove().unwrap();
        assert_eq!(weak.value(), Err(neocogi_com::NC_E_DROPPED));
        for (descriptor, expected) in [
            (&FactoryAbi::METHOD_DESCRIPTORS[0], "com_box_out"),
            (&IContainerAbi::METHOD_DESCRIPTORS[0], "com_box_in"),
            (&IContainerAbi::METHOD_DESCRIPTORS[1], "weak_com_box_out"),
        ] {
            // SAFETY: generated static descriptors have readable nested metadata.
            let printed = unsafe { neocogi_com::format_method_signature(descriptor) }.unwrap();
            assert!(printed.contains(expected), "{printed}");
        }
    }
    #[com_object(IValue)]
    /// Real shared backing supplies a valid wrong-policy weak result.
    struct SharedValue;
    impl IValue for SharedValue {
        fn value(&self) -> Result<u32, NcStatus> { Ok(7) }
    }

    /// Generated weak-output clients reject shared tokens without manufacturing
    /// boxed owners, both before and after the returned value expires.
    #[test]
    fn generated_boxed_weak_result_rejects_shared_policy() {
        use neocogi_com::{ComPtr, ComRaw, ComObject, WeakComPtr, NC_OK, NC_E_INVALID_ARG};
        // The custom container method uses matching Container storage and merely
        // returns a separately owned shared token from its ordinary field.
        unsafe extern "C" fn wrong_child(object: *mut core::ffi::c_void, output: *mut ComRaw<IValueAbi>) -> NcStatus {
            let state = unsafe { SharedResult::borrow_abi_ref(object) }.unwrap();
            let token = state.weak.clone().into_raw();
            unsafe { *output = ComRaw { object: token.control, vtable: &IValueVtblFor::<SharedValue>::VTABLE } };
            NC_OK
        }
        #[com_object(IContainer)]
        /// Source stays live while its returned observer may already be expired.
        struct SharedResult { weak: WeakComPtr<IValueAbi> }
        impl IContainer for SharedResult {
            fn insert(&mut self, child: ComBox<IValueAbi>) -> Result<(), (NcStatus, ComBox<IValueAbi>)> { Err((NC_E_INVALID_ARG, child)) }
            fn child(&self) -> Result<WeakComBox<IValueAbi>, NcStatus> { Err(NC_E_INVALID_ARG) }
            fn remove(&mut self) -> Result<(), NcStatus> { Ok(()) }
        }
        let value = SharedValue::from_value(SharedValue).query::<IValueAbi>().unwrap();
        let weak = value.downgrade().unwrap();
        let source = SharedResult::from_value(SharedResult { weak: weak.clone() }).query::<IContainerAbi>().unwrap();
        let mut table = IContainerVtblFor::<SharedResult>::VTABLE;
        table.child = wrong_child;
        let raw = source.into_raw();
        let source = unsafe { ComPtr::<IContainerAbi>::from_raw_owned(ComRaw { object: raw.object, vtable: &table }) }.unwrap();
        assert_eq!(source.child().unwrap_err(), NC_E_INVALID_ARG);
        drop(value);
        assert!(weak.upgrade().unwrap().is_none());
        assert_eq!(source.child().unwrap_err(), NC_E_INVALID_ARG);
        drop(source);
    }

}
