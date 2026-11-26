#[cfg(test)]
mod exclusive_tests {
    use super::*;
    use neocogi_com_sdk::{com_object, ComBox, ComMut, ComRef, NcStatus};

    #[com_object(IValue, ownership = exclusive)]
    /// Concrete payload whose owner travels through generated method signatures.
    struct Value(u32);
    impl IValue for Value {
        /// Read succeeds only while the exclusive owner still exists.
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(self.0)
        }
        fn set(&mut self, value: u32) -> Result<(), NcStatus> {
            self.0 = value;
            Ok(())
        }
    }
    /// Private forwarding observation prevents aliasing identical promoted tables.
    static COPIED_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    /// Forwards the same generated checked method with observable copied dispatch.
    unsafe extern "C" fn copied_value_call(object: *mut core::ffi::c_void, output: *mut u32) -> NcStatus {
        COPIED_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // SAFETY: only the matching Value allocation and valid raw output reach this table.
        unsafe { (IValueVtblFor::<Value>::VTABLE.value)(object, output) }
    }
    /// Factory results deliberately use a distinct valid table to test checked callers.
    static COPIED_VALUE_TABLE: IValueVTable = IValueVTable {
        value: copied_value_call,
        ..IValueVtblFor::<Value>::VTABLE
    };
    #[com_object(Factory)]
    /// Shared service creates exclusive widget-like values.
    struct Creator;
    impl Factory for Creator {
        /// Selects the result projection by consuming the freshly allocated owner.
        fn create(&self) -> Result<ComBox<IValueAbi>, NcStatus> {
            let raw = Value::into_box(Value(42)).query::<IValueAbi>().map_err(|(status, _unpublished)| status)?.into_raw();
            // SAFETY: copied metadata has the exact generated identity/thunks for
            // this concrete Value. Transfer the sole owner, without retaining it.
            Ok(unsafe { ComBox::from_raw_owned(neocogi_com_sdk::ComRaw { object: raw.object, vtable: &COPIED_VALUE_TABLE }) }.unwrap())
        }
    }
    /// Only the dedicated child process deliberately violates returned-view identity.
    const REPLACED_VIEW_CHILD: &str = "NC_TEST_REPLACED_ADMITTED_BOXED_VIEW";
    #[com_object(IContainer, ownership = exclusive)]
    /// Sole child ownership lives in one ordinary field.
    struct Container {
        child: Option<ComBox<IValueAbi>>,
    }
    impl IContainer for Container {
        /// Replaces ownership without exposing a detached previous child.
        fn insert(&mut self, child: neocogi_com_sdk::ComBox<IValueAbi>) -> Result<(), (NcStatus, neocogi_com_sdk::ComBox<IValueAbi>)> {
            if std::env::var_os(REPLACED_VIEW_CHILD).is_some() {
                let raw = child.into_raw();
                // SAFETY: the copied table still matches the real Value allocation.
                // The ownership/view rejection protocol is deliberately violated,
                // so generated code must fail before restoring/publishing a pair.
                let changed = unsafe { ComBox::from_raw_owned(neocogi_com_sdk::ComRaw { object: raw.object, vtable: &COPIED_VALUE_TABLE }) }.unwrap();
                return Err((neocogi_com_sdk::NC_E_INVALID_ARG, changed));
            }
            self.child = Some(child);
            Ok(())
        }
        fn child(&self) -> Option<ComRef<'_, IValueAbi>> {
            self.child.as_ref().map(ComBox::as_com_ref)
        }
        fn child_mut(&mut self) -> Option<ComMut<'_, IValueAbi>> {
            self.child.as_mut().map(ComBox::as_com_mut)
        }
        fn edit(&self, child: Option<ComMut<'_, IValueAbi>>) -> Result<(), NcStatus> {
            if let Some(mut child) = child {
                child.set(57)?;
            }
            Ok(())
        }
        /// Ordinary destruction replaces fallible domain closure.
        fn remove(&mut self) -> Result<(), NcStatus> {
            self.child = None;
            Ok(())
        }
    }
    /// A returned table different from the admitted canonical table must not be
    /// silently paired with original caller metadata. The C boundary cannot unwind.
    #[test]
    #[cfg(not(miri))]
    fn changed_admitted_owner_view_aborts_before_rejection_publication() {
        if std::env::var_os(REPLACED_VIEW_CHILD).is_some() {
            let child = Value::into_box(Value(42)).query::<IValueAbi>().unwrap();
            let mut parent = Container::into_box(Container { child: None }).query::<IContainerAbi>().unwrap();
            let _ = parent.insert(child);
            std::process::exit(97);
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "exclusive_tests::changed_admitted_owner_view_aborts_before_rejection_publication", "--nocapture"])
            .env(REPLACED_VIEW_CHILD, "1")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_ne!(output.status.code(), Some(97), "invalid returned view was published");
        assert!(String::from_utf8_lossy(&output.stderr).contains("rejected COM method did not return its admitted owner"));
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(output.status.signal(), Some(6));
        }
    }

    /// No Rust-only shortcut participates in factory creation, insertion, or lookup.
    #[test]
    fn factory_container_and_inspection_agree_on_ownership() {
        let factory = Creator::from_value(Creator).query::<FactoryAbi>().unwrap();
        let mut parent = Container::into_box(Container { child: None }).query::<IContainerAbi>().unwrap();
        let copied_calls = COPIED_CALLS.load(std::sync::atomic::Ordering::SeqCst);
        let child = factory.create().unwrap();
        assert_ne!(child.as_raw().vtable, &COPIED_VALUE_TABLE as *const _);
        assert_eq!(child.as_raw().vtable, child.query_ref::<IValueAbi>().unwrap().as_raw().vtable);
        assert_eq!(child.value(), Ok(42));
        assert_eq!(COPIED_CALLS.load(std::sync::atomic::Ordering::SeqCst), copied_calls);
        let identity = child.as_raw();
        parent.insert(child).unwrap();
        let loan = parent.child().unwrap();
        assert_eq!(loan.as_raw(), identity);
        assert_eq!(loan.value(), Ok(42));
        let mut loan = parent.child_mut().unwrap();
        assert_eq!(loan.as_raw(), identity);
        loan.set(62).unwrap();
        assert_eq!(parent.child().unwrap().value(), Ok(62));
        let mut other = factory.create().unwrap();
        parent.edit(None).unwrap();
        parent.edit(Some(other.as_com_mut())).unwrap();
        assert_eq!(other.value(), Ok(57));
        parent.remove().unwrap();
        assert!(parent.child().is_none());
        for (descriptor, expected) in [
            (&FactoryAbi::METHOD_DESCRIPTORS[0], "com_box_out"),
            (&IContainerAbi::METHOD_DESCRIPTORS[0], "com_box_in"),
            (&IContainerAbi::METHOD_DESCRIPTORS[1], "optional_com_ref_out"),
            (&IContainerAbi::METHOD_DESCRIPTORS[2], "optional_com_mut_out"),
            (&IContainerAbi::METHOD_DESCRIPTORS[3], "optional_com_mut"),
        ] {
            // SAFETY: generated static descriptors have readable nested metadata.
            let printed = unsafe { neocogi_com_sdk::format_method_signature(descriptor) }.unwrap();
            assert!(printed.contains(expected), "{printed}");
        }
    }

}
