#[cfg(test)]
mod exclusive_tests {
    use super::*;
    use neocogi_com::{com_object, com_factory, ComBox, WeakComBox, NcStatus};

    #[com_object(IValue, ownership = exclusive)]
    /// Concrete payload whose owner travels through generated method signatures.
    struct Value;
    impl IValue for Value {
        /// Read succeeds only while the exclusive owner still exists.
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(42)
        }
    }
    #[com_factory(Factory)]
    /// Shared service creates exclusive widget-like values.
    struct Creator;
    impl Factory for Creator {
        /// Selects the result projection by consuming the freshly allocated owner.
        fn create(&self) -> Result<ComBox<IValueAbi>, NcStatus> {
            Value::into_box(Value).query()
        }
    }
    #[com_object(IContainer, ownership = exclusive)]
    /// Sole child ownership lives in one ordinary field.
    struct Container {
        child: Option<ComBox<IValueAbi>>,
    }
    impl IContainer for Container {
        /// Replaces ownership without exposing a detached previous child.
        fn insert(&mut self, child: neocogi_com::ComBoxInput<'_, IValueAbi>) -> Result<(), NcStatus> {
            self.child = Some(child.take()?);
            Ok(())
        }
        /// Gives callers weak access without an independent owner.
        fn child(&self) -> Result<WeakComBox<IValueAbi>, NcStatus> {
            self.child.as_ref().map(ComBox::downgrade).ok_or(neocogi_com::NC_E_CLOSED)
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
        assert_eq!(weak.value(), Err(neocogi_com::NC_E_CLOSED));
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
}
