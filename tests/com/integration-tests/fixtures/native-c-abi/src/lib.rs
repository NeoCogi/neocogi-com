use neocogi_com_sdk::{ComPtr, ComRef, NcCopyFromBufferOffset, NcOutputWriter, NcStatus, NC_E_OUT_OF_RANGE, com_object};

#[allow(dead_code, unused_imports)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/contracts.rs"));
}
use bindings::*;

#[com_object(interfaces(INativeValue))]
pub struct NativeValue {
    value: i32,
}

impl INativeValue for NativeValue {
    fn value(&self) -> Result<i32, NcStatus> {
        Ok(self.value)
    }
}

#[com_object(interfaces(INativeCalculator))]
pub struct NativeCalculator;

/// Inherited calls use the same concrete calculator allocation from C.
impl INativeValue for NativeCalculator {
    fn value(&self) -> Result<i32, NcStatus> {
        Ok(23)
    }
}

impl INativeCalculator for NativeCalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(a + b)
    }

    fn reset(&self) -> Result<(), NcStatus> {
        Ok(())
    }

    fn make_value(&self, value: i32) -> Result<ComPtr<INativeValueAbi>, NcStatus> {
        let unknown = NativeValue::from_value(NativeValue { value });
        unknown.query()
    }

    fn read_value(&self, value: ComRef<'_, INativeValueAbi>) -> Result<i32, NcStatus> {
        value.value()
    }

    fn sum_buffer(&self, input: NcCopyFromBufferOffset<'_, u8>) -> Result<u64, NcStatus> {
        Ok(input.as_slice().iter().fold(input.offset(), |sum, byte| sum.wrapping_add(u64::from(*byte))))
    }

    fn fill_buffer(&self, mut output: NcOutputWriter<'_, u8>) -> Result<(), NcStatus> {
        let offset = output.offset();
        for index in 0..output.capacity() {
            let index = u64::try_from(index).map_err(|_| NC_E_OUT_OF_RANGE)?;
            let value = offset.checked_add(index).ok_or(NC_E_OUT_OF_RANGE)?;
            output.push(u8::try_from(value).map_err(|_| NC_E_OUT_OF_RANGE)?)?;
        }
        Ok(())
    }

    fn transform(&self, input: NativeAggregate) -> Result<NativeAggregate, NcStatus> {
        Ok(NativeAggregate {
            tag: input.tag.wrapping_add(1),
            count: input.count.wrapping_add(2),
            value: input.value.wrapping_add(3),
        })
    }
}

#[com_object(interfaces(INativeChoice))]
/// Stateless native implementation reached only after foreign input validation.
struct Choice;
impl INativeChoice for Choice {
    fn echo(&self, value: NativeChoice) -> Result<NativeChoice, NcStatus> {
        Ok(value)
    }
    /// A short query reports required capacity without initializing any prefix.
    fn output(&self, mut output: NcOutputWriter<'_, NativeChoice>) -> Result<u32, NcStatus> {
        if output.capacity() >= 2 {
            output.extend_from_slice(&[NativeChoice::Flag(true), NativeChoice::Empty])?;
        }
        Ok(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com_sdk::{ComInterface, IUnknown, NC_E_POINTER, NC_OK, NcDebugApiV4, NcInterfaceDescriptor, NcInterfaceId, NcPluginApiV4, NcPluginInfoV4};

    #[repr(C)]
    struct NativeCResults {
        sum: i32,
        made_value: i32,
        borrowed_value: i32,
        reset_status: i32,
        null_reset_status: i32,
        null_method_status: i32,
        add_ref_count: u32,
        release_extra_count: u32,
        release_query_count: u32,
        release_value_count: u32,
        buffer_sum: u64,
        filled_buffer: [u8; 4],
        null_copy_from_status: i32,
        null_copy_to_status: i32,
        zero_copy_from_status: i32,
        zero_copy_to_status: i32,
        aggregate: <NativeAggregate as neocogi_com_sdk::ComValue>::AbiRepr,
    }

    impl Default for NativeCResults {
        fn default() -> Self {
            // SAFETY: all fields are unrestricted scalar wire values; zero is valid.
            unsafe { core::mem::zeroed() }
        }
    }

    #[repr(C)]
    struct NativeAbiLayouts {
        interface_descriptor_size: usize,
        interface_qualified_name_offset: usize,
        interface_method_count_offset: usize,
        interface_methods_offset: usize,
        runtime_core_abi_version_offset: usize,
        runtime_plugin_register_offset: usize,
        runtime_plugin_count_offset: usize,
        runtime_plugin_info_offset: usize,
        runtime_size: usize,
        runtime_plugin_infos_offset: usize,
        plugin_info_size: usize,
        plugin_factory_offset: usize,
        plugin_dependencies_offset: usize,
        plugin_dependency_count_offset: usize,
        debug_first_function_offset: usize,
        host_size: usize,
        host_runtime_version_offset: usize,
        host_core_version_offset: usize,
        host_runtime_offset: usize,
        host_debug_offset: usize,
        host_persistence_offset: usize,
        aggregate_size: usize,
        aggregate_align: usize,
        aggregate_count_offset: usize,
        aggregate_value_offset: usize,
    }

    unsafe extern "C" {
        /// Borrows one owner and checks C-local digest copies, every byte and absence.
        fn nc_c_exercise_signature(object: neocogi_com_sdk::ComRaw<IUnknown>, expected: *const NcInterfaceId, mismatch: NcStatus) -> i32;

        fn nc_c_exercise_calculator(
            unknown: neocogi_com_sdk::ComRaw<IUnknown>,
            calculator_interface_id: *const NcInterfaceId,
            results: *mut NativeCResults,
            pointer_status: NcStatus,
        ) -> NcStatus;

        fn nc_c_validate_abi_layouts(layouts: *const NativeAbiLayouts) -> i32;
        /// C supplies both concrete lifetime layouts and its own allocation callbacks.
        fn nc_c_exercise_split_lifetimes(
            shared_root: *const neocogi_com_sdk::IUnknownVTable,
            exclusive_root: *const neocogi_com_sdk::IUnknownVTable,
            dropped: NcStatus,
            invalid: NcStatus,
            weak_retain: unsafe extern "C" fn(*mut neocogi_com_sdk::lifetime::NcSharedLifetimeState),
            weak_release: unsafe extern "C" fn(*mut neocogi_com_sdk::lifetime::NcSharedLifetimeState),
            upgrade: unsafe extern "C" fn(*mut neocogi_com_sdk::lifetime::NcSharedLifetimeState, *const NcInterfaceId, *mut neocogi_com_sdk::ComRaw<IUnknown>) -> NcStatus,
            boxed_retain: unsafe extern "C" fn(*mut neocogi_com_sdk::lifetime::NcBoxedLifetimeState),
            boxed_release: unsafe extern "C" fn(*mut neocogi_com_sdk::lifetime::NcBoxedLifetimeState),
        ) -> i32;

        /// Consumes the supplied sole owner and verifies weak expiration in C.
        fn nc_c_exercise_weak(
            owned: neocogi_com_sdk::ComRaw<IUnknown>,
            requested: *const NcInterfaceId,
            dropped: NcStatus,
            retain: unsafe extern "C" fn(*mut neocogi_com_sdk::lifetime::NcSharedLifetimeState),
            release: unsafe extern "C" fn(*mut neocogi_com_sdk::lifetime::NcSharedLifetimeState),
            upgrade: unsafe extern "C" fn(*mut neocogi_com_sdk::lifetime::NcSharedLifetimeState, *const NcInterfaceId, *mut neocogi_com_sdk::ComRaw<IUnknown>) -> NcStatus,
        ) -> i32;

    }

    /// Borrows a shared fixture token; its typed ABI argument proves backing.
    unsafe fn clone_fixture_shared(control: *mut neocogi_com_sdk::lifetime::NcSharedLifetimeState) -> neocogi_com_sdk::NcSharedWeakRaw {
        let borrowed = core::mem::ManuallyDrop::new(unsafe { neocogi_com_sdk::WeakComPtr::<IUnknown>::from_raw_owned(neocogi_com_sdk::NcSharedWeakRaw { control }) }.unwrap());
        neocogi_com_sdk::WeakComPtr::clone(&borrowed).into_raw()
    }
    /// Binding-owned shared retain callback; no lifetime kind is discovered.
    unsafe extern "C" fn fixture_weak_retain(control: *mut neocogi_com_sdk::lifetime::NcSharedLifetimeState) {
        let _ = unsafe { clone_fixture_shared(control) };
    }
    /// Consumes one shared token using its declared state.
    unsafe extern "C" fn fixture_weak_release(control: *mut neocogi_com_sdk::lifetime::NcSharedLifetimeState) {
        drop(unsafe { neocogi_com_sdk::WeakComPtr::<IUnknown>::from_raw_owned(neocogi_com_sdk::NcSharedWeakRaw { control }) });
    }
    /// Borrows one boxed fixture token; this callback's ABI declares boxed backing.
    unsafe extern "C" fn fixture_boxed_weak_retain(control: *mut neocogi_com_sdk::lifetime::NcBoxedLifetimeState) {
        let raw = neocogi_com_sdk::ComRaw::<IUnknown> { object: control.cast(), vtable: &neocogi_com_sdk::lifetime::EXCLUSIVE_UNKNOWN_VTABLE };
        let borrowed = core::mem::ManuallyDrop::new(unsafe { neocogi_com_sdk::WeakComBox::<IUnknown>::from_raw_owned(raw) }.unwrap());
        let _ = neocogi_com_sdk::WeakComBox::clone(&borrowed).into_raw();
    }
    /// Consumes a boxed observer directly through its declared policy.
    unsafe extern "C" fn fixture_boxed_weak_release(control: *mut neocogi_com_sdk::lifetime::NcBoxedLifetimeState) {
        let raw = neocogi_com_sdk::ComRaw::<IUnknown> { object: control.cast(), vtable: &neocogi_com_sdk::lifetime::EXCLUSIVE_UNKNOWN_VTABLE };
        drop(unsafe { neocogi_com_sdk::WeakComBox::<IUnknown>::from_raw_owned(raw) });
    }

    /// Test-only foreign adapter uses checked token adoption, typed promotion,
    /// then dynamic exact selection. Every temporary owner is balanced on error.
    unsafe extern "C" fn fixture_weak_upgrade(
        object: *mut neocogi_com_sdk::lifetime::NcSharedLifetimeState,
        requested: *const NcInterfaceId,
        output: *mut neocogi_com_sdk::ComRaw<IUnknown>,
    ) -> NcStatus {
        use neocogi_com_sdk::{ComRaw, WeakComPtr, NC_E_DROPPED, NC_E_POINTER, NC_OK};
        if output.is_null() {
            return NC_E_POINTER;
        }
        unsafe { *output = ComRaw::null() };
        if object.is_null() || requested.is_null() {
            return NC_E_POINTER;
        }
        let raw = unsafe { clone_fixture_shared(object) };
        let weak = match unsafe { WeakComPtr::<IUnknown>::__nc_from_raw_owned_checked(raw) } {
            Ok(weak) => weak,
            Err(status) => return status,
        };
        let owner = match weak.upgrade() {
            Ok(Some(owner)) => owner,
            Ok(None) => return NC_E_DROPPED,
            Err(status) => return status,
        };
        match unsafe { neocogi_com_sdk::raw::__nc_query_interface(owner.as_raw(), &*requested) } {
            Ok(raw) => {
                unsafe { *output = raw };
                NC_OK
            }
            Err(status) => status,
        }
    }

    /// Independent C allocation declarations exercise both policies and expired-kind checks.
    #[test]
    fn native_c_uses_distinct_shared_and_boxed_lifetime_states() {
        use neocogi_com_sdk::lifetime::{SHARED_UNKNOWN_VTABLE, EXCLUSIVE_UNKNOWN_VTABLE};
        // SAFETY: C retains every allocation, immutable metadata and output for the
        // complete call and frees both C-owned allocations through its callbacks.
        assert_eq!(
            unsafe {
                nc_c_exercise_split_lifetimes(
                    &SHARED_UNKNOWN_VTABLE,
                    &EXCLUSIVE_UNKNOWN_VTABLE,
                    neocogi_com_sdk::NC_E_DROPPED,
                    neocogi_com_sdk::NC_E_INVALID_ARG,
                    fixture_weak_retain,
                    fixture_weak_release,
                    fixture_weak_upgrade,
                    fixture_boxed_weak_retain,
                    fixture_boxed_weak_release,
                )
            },
            1
        );
    }

    /// An independent C caller can negotiate using its own fixed-width digest.
    #[test]
    fn native_c_checks_signature_contents_and_absence() {
        let owner = NativeCalculator::from_value(NativeCalculator);
        // SAFETY: The sole owner and static identity remain live; C balances
        // its successful query and never retains the borrowed stack digest.
        assert_eq!(
            unsafe { nc_c_exercise_signature(owner.as_raw(), &INativeCalculatorAbi::INTERFACE_ID, neocogi_com_sdk::NC_E_ABI_VERSION) },
            1
        );
    }

    unsafe extern "C" {
        /// Exercises the generated slot using C-owned raw tag and payload records.
        fn nc_c_exercise_choice(
            object: *mut core::ffi::c_void,
            echo: unsafe extern "C" fn(
                *mut core::ffi::c_void,
                <NativeChoice as neocogi_com_sdk::ComValue>::AbiRepr,
                *mut <NativeChoice as neocogi_com_sdk::ComValue>::AbiRepr,
            ) -> NcStatus,
            invalid_tag: NcStatus,
            invalid_bool: NcStatus,
        ) -> i32;
    }

    /// C sees a fixed wire record even though the Rust provider uses a native enum.
    #[test]
    fn native_c_validates_copied_enum_tags_and_boolean_payloads() {
        let choice = Choice::from_value(Choice).query::<INativeChoiceAbi>().unwrap();
        let raw = choice.as_raw();
        // SAFETY: the owned object/table outlive the C call; C provides initialized
        // wire inputs and writable outputs and never retains either pointer.
        assert_eq!(
            unsafe { nc_c_exercise_choice(raw.object, (*raw.vtable).echo, neocogi_com_sdk::NC_E_INVALID_TAG, neocogi_com_sdk::NC_E_INVALID_ARG) },
            1
        );
    }

    /// Plain C retains/releases the weak token and promotes the exact generated contract.
    #[test]
    fn native_c_can_consume_weak_lifetimes() {
        let owner = NativeCalculator::from_value(NativeCalculator);
        // SAFETY: C consumes this sole owner and balances every acquired strong/weak reference.
        assert_eq!(
            unsafe {
                nc_c_exercise_weak(
                    owner.into_raw(),
                    &INativeCalculatorAbi::INTERFACE_ID,
                    neocogi_com_sdk::NC_E_DROPPED,
                    fixture_weak_retain,
                    fixture_weak_release,
                    fixture_weak_upgrade,
                )
            },
            1
        );
    }

    fn native_abi_layouts() -> NativeAbiLayouts {
        NativeAbiLayouts {
            interface_descriptor_size: core::mem::size_of::<NcInterfaceDescriptor>(),
            interface_qualified_name_offset: core::mem::offset_of!(NcInterfaceDescriptor, qualified_name),
            interface_method_count_offset: core::mem::offset_of!(NcInterfaceDescriptor, method_count),
            interface_methods_offset: core::mem::offset_of!(NcInterfaceDescriptor, methods),
            runtime_core_abi_version_offset: core::mem::offset_of!(NcPluginApiV4, core_abi_version),
            runtime_plugin_register_offset: core::mem::offset_of!(NcPluginApiV4, register_plugin_descriptor),
            runtime_plugin_count_offset: core::mem::offset_of!(NcPluginApiV4, plugin_count),
            runtime_plugin_info_offset: core::mem::offset_of!(NcPluginApiV4, plugin_info),
            runtime_size: core::mem::size_of::<NcPluginApiV4>(),
            runtime_plugin_infos_offset: core::mem::offset_of!(NcPluginApiV4, plugin_infos),
            plugin_info_size: core::mem::size_of::<NcPluginInfoV4>(),
            plugin_factory_offset: core::mem::offset_of!(NcPluginInfoV4, factory),
            plugin_dependencies_offset: core::mem::offset_of!(NcPluginInfoV4, dependencies),
            plugin_dependency_count_offset: core::mem::offset_of!(NcPluginInfoV4, dependency_count),
            debug_first_function_offset: core::mem::offset_of!(NcDebugApiV4, create_messenger),
            host_size: core::mem::size_of::<neocogi_com_sdk::NcHostApiV4>(),
            host_runtime_version_offset: core::mem::offset_of!(neocogi_com_sdk::NcHostApiV4, runtime_abi_version),
            host_core_version_offset: core::mem::offset_of!(neocogi_com_sdk::NcHostApiV4, core_abi_version),
            host_runtime_offset: core::mem::offset_of!(neocogi_com_sdk::NcHostApiV4, runtime),
            host_debug_offset: core::mem::offset_of!(neocogi_com_sdk::NcHostApiV4, debug),
            host_persistence_offset: core::mem::offset_of!(neocogi_com_sdk::NcHostApiV4, persistence),
            aggregate_size: core::mem::size_of::<<NativeAggregate as neocogi_com_sdk::ComValue>::AbiRepr>(),
            aggregate_align: core::mem::align_of::<<NativeAggregate as neocogi_com_sdk::ComValue>::AbiRepr>(),
            aggregate_count_offset: core::mem::offset_of!(<NativeAggregate as neocogi_com_sdk::ComValue>::AbiRepr, count),
            aggregate_value_offset: core::mem::offset_of!(<NativeAggregate as neocogi_com_sdk::ComValue>::AbiRepr, value),
        }
    }

    #[test]
    fn native_c_can_consume_generated_com_abi() {
        let unknown: ComPtr<IUnknown> = NativeCalculator::from_value(NativeCalculator);
        let mut results = NativeCResults::default();

        let status = unsafe { nc_c_exercise_calculator(unknown.as_raw(), &INativeCalculatorAbi::INTERFACE_ID, &mut results, NC_E_POINTER) };

        assert_eq!(status, NC_OK);
        assert_eq!(results.sum, 5);
        assert_eq!(results.made_value, 42);
        assert_eq!(results.borrowed_value, 42);
        assert_eq!(results.reset_status, NC_OK.code());
        assert_eq!(results.null_reset_status, NC_E_POINTER.code());
        assert_eq!(results.null_method_status, NC_E_POINTER.code());
        assert_eq!(results.add_ref_count, 3);
        assert_eq!(results.release_extra_count, 2);
        assert_eq!(results.release_query_count, 1);
        assert_eq!(results.release_value_count, 0);
        assert_eq!(results.buffer_sum, 25);
        assert_eq!(results.filled_buffer, [7, 8, 9, 10]);
        assert_eq!(results.null_copy_from_status, NC_E_POINTER.code());
        assert_eq!(results.null_copy_to_status, NC_E_POINTER.code());
        assert_eq!(results.zero_copy_from_status, NC_OK.code());
        assert_eq!(results.zero_copy_to_status, NC_OK.code());
        assert_eq!(
            unsafe { <NativeAggregate as neocogi_com_sdk::ComValue>::decode_value(&results.aggregate, &mut Default::default()) }.unwrap(),
            NativeAggregate { tag: 4, count: 7, value: 14 }
        );
        assert_eq!(unsafe { nc_c_validate_abi_layouts(&native_abi_layouts()) }, 1);
    }
}

#[cfg(test)]
unsafe extern "C" {
    /// C reads the explicit prefix count and verifies inactive tail bytes stay untouched.
    fn nc_c_exercise_choice_output(
        object: *mut core::ffi::c_void,
        output: unsafe extern "C" fn(*mut core::ffi::c_void, u64, u64, *mut <NativeChoice as neocogi_com_sdk::ComValue>::AbiRepr, *mut u64, *mut u32) -> NcStatus,
        pointer_status: NcStatus,
    ) -> i32;
}

/// Independently written C storage need not contain valid native enum/bool values on entry.
#[cfg(test)]
#[test]
fn c_output_getter_uses_fresh_wire_storage_and_separate_prefix_count() {
    let source = Choice::from_value(Choice).query::<INativeChoiceAbi>().unwrap();
    let raw = source.as_raw();
    // SAFETY: C invokes the exact raw signature using live, aligned foreign slots.
    assert_eq!(
        unsafe { nc_c_exercise_choice_output(raw.object, (*raw.vtable).output, neocogi_com_sdk::NC_E_POINTER) },
        1
    );
}

#[cfg(test)]
mod host_abi;
