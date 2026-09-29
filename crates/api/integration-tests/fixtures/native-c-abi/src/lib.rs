use neocogi_com::{ComPtr, ComRef, NcCopyFromBufferOffset, NcCopyToBufferOffset, NcStatus, NC_E_OUT_OF_RANGE, com_object, com_interface, com_struct};

#[com_struct(assigned_signature = b"00000000000000000000000000000040")]
#[derive(Debug, Default, PartialEq)]
pub struct NativeAggregate {
    pub tag: u8,
    pub count: u32,
    pub value: u64,
}

#[com_interface(auto_guid = "nc.tests.native_c_abi.Value:v1", assigned_signature = b"00000000000000000000000000000041")]
pub trait INativeValue {
    fn value(&self) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.native_c_abi.Calculator:v1", assigned_signature = b"00000000000000000000000000000042")]
pub trait INativeCalculator: INativeValue {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
    fn reset(&self) -> Result<(), NcStatus>;
    fn make_value(&self, value: i32) -> Result<ComPtr<INativeValueAbi>, NcStatus>;
    fn read_value(&self, value: ComRef<'_, INativeValueAbi>) -> Result<i32, NcStatus>;
    fn sum_buffer(&self, input: NcCopyFromBufferOffset<'_, u8>) -> Result<u64, NcStatus>;
    fn fill_buffer(&self, output: NcCopyToBufferOffset<'_, u8>) -> Result<(), NcStatus>;
    fn transform(&self, input: NativeAggregate) -> Result<NativeAggregate, NcStatus>;
}

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

    fn fill_buffer(&self, mut output: NcCopyToBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        let offset = output.offset();
        for (index, byte) in output.as_mut_slice().iter_mut().enumerate() {
            let index = u64::try_from(index).map_err(|_| NC_E_OUT_OF_RANGE)?;
            let value = offset.checked_add(index).ok_or(NC_E_OUT_OF_RANGE)?;
            *byte = u8::try_from(value).map_err(|_| NC_E_OUT_OF_RANGE)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com::{
        ComInterface, IUnknown, NC_E_POINTER, NC_OK, NcDebugApiV1, NcInspectionApiV1, NcInterfaceDescriptor, NcInterfaceId, NcPluginRegistryV1, NcRuntimeApiV1,
    };

    #[repr(C)]
    #[derive(Default)]
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
        aggregate: NativeAggregate,
    }

    #[repr(C)]
    struct NativeAbiLayouts {
        interface_descriptor_size: usize,
        interface_qualified_name_offset: usize,
        interface_method_count_offset: usize,
        interface_methods_offset: usize,
        runtime_core_abi_version_offset: usize,
        runtime_plugin_register_offset: usize,
        plugin_registry_first_function_offset: usize,
        inspection_first_function_offset: usize,
        debug_first_function_offset: usize,
        aggregate_size: usize,
        aggregate_align: usize,
        aggregate_count_offset: usize,
        aggregate_value_offset: usize,
    }

    unsafe extern "C" {
        fn nc_c_exercise_calculator(
            unknown: neocogi_com::ComRaw<IUnknown>,
            calculator_interface_id: *const NcInterfaceId,
            results: *mut NativeCResults,
            pointer_status: NcStatus,
        ) -> NcStatus;

        fn nc_c_validate_abi_layouts(layouts: *const NativeAbiLayouts) -> i32;
        /// Consumes the supplied sole owner and verifies weak expiration in C.
        fn nc_c_exercise_weak(owned: neocogi_com::ComRaw<IUnknown>, requested: *const NcInterfaceId, closed: NcStatus) -> i32;

    }

    /// Plain C retains/releases the weak token and promotes the exact generated contract.
    #[test]
    fn native_c_can_consume_weak_lifetimes() {
        let owner = NativeCalculator::from_value(NativeCalculator);
        // SAFETY: C consumes this sole owner and balances every acquired strong/weak reference.
        assert_eq!(
            unsafe { nc_c_exercise_weak(owner.into_raw(), &INativeCalculatorAbi::INTERFACE_ID, neocogi_com::NC_E_CLOSED) },
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
            runtime_plugin_register_offset: core::mem::offset_of!(NcRuntimeApiV1, register_factory_plugin_description),
            plugin_registry_first_function_offset: core::mem::offset_of!(NcPluginRegistryV1, plugin_count),
            inspection_first_function_offset: core::mem::offset_of!(NcInspectionApiV1, plugin_info),
            debug_first_function_offset: core::mem::offset_of!(NcDebugApiV1, create_messenger),
            aggregate_size: core::mem::size_of::<NativeAggregate>(),
            aggregate_align: core::mem::align_of::<NativeAggregate>(),
            aggregate_count_offset: core::mem::offset_of!(NativeAggregate, count),
            aggregate_value_offset: core::mem::offset_of!(NativeAggregate, value),
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
        assert_eq!(results.aggregate, NativeAggregate { tag: 4, count: 7, value: 14 });
        assert_eq!(unsafe { nc_c_validate_abi_layouts(&native_abi_layouts()) }, 1);
    }
}
