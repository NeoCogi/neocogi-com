use neocogi_com::{ComPtr, ComRef, NcCopyFromBufferOffset, NcOutputWriter, NcStatus, NC_E_OUT_OF_RANGE, com_object};

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
    use neocogi_com::{ComInterface, IUnknown, NC_E_POINTER, NC_OK, NcDebugApiV1, NcInterfaceDescriptor, NcInterfaceId, NcRuntimeApiV1, NcPluginInfoV1};

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
        aggregate: <NativeAggregate as neocogi_com::ComValue>::Wire,
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
        fn nc_c_exercise_signature(object: neocogi_com::ComRaw<IUnknown>, expected: *const NcInterfaceId, mismatch: NcStatus) -> i32;

        fn nc_c_exercise_calculator(
            unknown: neocogi_com::ComRaw<IUnknown>,
            calculator_interface_id: *const NcInterfaceId,
            results: *mut NativeCResults,
            pointer_status: NcStatus,
        ) -> NcStatus;

        fn nc_c_validate_abi_layouts(layouts: *const NativeAbiLayouts) -> i32;
        /// Consumes the supplied sole owner and verifies weak expiration in C.
        fn nc_c_exercise_weak(
            owned: neocogi_com::ComRaw<IUnknown>,
            requested: *const NcInterfaceId,
            dropped: NcStatus,
            retain: unsafe extern "C" fn(*mut neocogi_com::lifetime::NcWeakControl),
            release: unsafe extern "C" fn(*mut neocogi_com::lifetime::NcWeakControl),
            upgrade: unsafe extern "C" fn(*mut neocogi_com::lifetime::NcWeakControl, *const NcInterfaceId, *mut neocogi_com::ComRaw<IUnknown>) -> NcStatus,
        ) -> i32;

    }

    /// An independent C caller can negotiate using its own fixed-width digest.
    #[test]
    fn native_c_checks_signature_contents_and_absence() {
        let owner = NativeCalculator::from_value(NativeCalculator);
        // SAFETY: The sole owner and static identity remain live; C balances
        // its successful query and never retains the borrowed stack digest.
        assert_eq!(
            unsafe { nc_c_exercise_signature(owner.as_raw(), &INativeCalculatorAbi::INTERFACE_ID, neocogi_com::NC_E_ABI_VERSION) },
            1
        );
    }

    unsafe extern "C" {
        /// Exercises the generated slot using C-owned raw tag and payload records.
        fn nc_c_exercise_choice(
            object: *mut core::ffi::c_void,
            echo: unsafe extern "C" fn(
                *mut core::ffi::c_void,
                <NativeChoice as neocogi_com::ComValue>::Wire,
                *mut <NativeChoice as neocogi_com::ComValue>::Wire,
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
            unsafe { nc_c_exercise_choice(raw.object, (*raw.vtable).echo, neocogi_com::NC_E_INVALID_TAG, neocogi_com::NC_E_INVALID_ARG) },
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
                    neocogi_com::NC_E_DROPPED,
                    neocogi_com::lifetime::nc_weak_retain,
                    neocogi_com::lifetime::nc_weak_release,
                    neocogi_com::lifetime::nc_weak_upgrade,
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
            runtime_core_abi_version_offset: core::mem::offset_of!(NcRuntimeApiV1, core_abi_version),
            runtime_plugin_register_offset: core::mem::offset_of!(NcRuntimeApiV1, register_plugin_descriptor),
            runtime_plugin_count_offset: core::mem::offset_of!(NcRuntimeApiV1, plugin_count),
            runtime_plugin_info_offset: core::mem::offset_of!(NcRuntimeApiV1, plugin_info),
            runtime_size: core::mem::size_of::<NcRuntimeApiV1>(),
            runtime_plugin_infos_offset: core::mem::offset_of!(NcRuntimeApiV1, plugin_infos),
            plugin_info_size: core::mem::size_of::<NcPluginInfoV1>(),
            plugin_factory_offset: core::mem::offset_of!(NcPluginInfoV1, factory),
            plugin_dependencies_offset: core::mem::offset_of!(NcPluginInfoV1, dependencies),
            plugin_dependency_count_offset: core::mem::offset_of!(NcPluginInfoV1, dependency_count),
            debug_first_function_offset: core::mem::offset_of!(NcDebugApiV1, create_messenger),
            host_size: core::mem::size_of::<neocogi_com::NcHostApiV1>(),
            host_runtime_version_offset: core::mem::offset_of!(neocogi_com::NcHostApiV1, runtime_abi_version),
            host_core_version_offset: core::mem::offset_of!(neocogi_com::NcHostApiV1, core_abi_version),
            host_runtime_offset: core::mem::offset_of!(neocogi_com::NcHostApiV1, runtime),
            host_debug_offset: core::mem::offset_of!(neocogi_com::NcHostApiV1, debug),
            host_persistence_offset: core::mem::offset_of!(neocogi_com::NcHostApiV1, persistence),
            aggregate_size: core::mem::size_of::<<NativeAggregate as neocogi_com::ComValue>::Wire>(),
            aggregate_align: core::mem::align_of::<<NativeAggregate as neocogi_com::ComValue>::Wire>(),
            aggregate_count_offset: core::mem::offset_of!(<NativeAggregate as neocogi_com::ComValue>::Wire, count),
            aggregate_value_offset: core::mem::offset_of!(<NativeAggregate as neocogi_com::ComValue>::Wire, value),
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
            unsafe { <NativeAggregate as neocogi_com::ComValue>::decode_value(results.aggregate, &mut Default::default()) }.unwrap(),
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
        output: unsafe extern "C" fn(*mut core::ffi::c_void, u64, u64, *mut <NativeChoice as neocogi_com::ComValue>::Wire, *mut u64, *mut u32) -> NcStatus,
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
        unsafe { nc_c_exercise_choice_output(raw.object, (*raw.vtable).output, neocogi_com::NC_E_POINTER) },
        1
    );
}

#[cfg(test)]
mod host_abi;
