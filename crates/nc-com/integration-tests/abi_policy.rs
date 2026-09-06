use nc_com::{
    ComInterface, ComPtr, ComRef, NcAbiHash, NcStatus, com_interface, format_method_signature,
    guid_from_qualified_name,
};
use std::fmt::Write;

mod external_fields {
    use nc_com::{ComPtr, NcCopyFromBufferOffset, NcStatus, com_class, com_interface, com_struct};

    // Models a C-layout dependency type with no nc-com traits.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Pair {
        x: f32,
        y: f32,
    }

    #[com_struct(c_struct)]
    #[derive(Debug, PartialEq)]
    struct Sample {
        pairs: [Pair; 2],
    }

    #[com_interface(auto_guid = "nc.tests.abi_policy.ExternalFields:v1")]
    trait IExternalFields {
        fn echo(&self, value: Sample) -> Result<Sample, NcStatus>;
        fn sum(&self, values: NcCopyFromBufferOffset<'_, Sample>) -> Result<f32, NcStatus>;
    }

    #[com_class(interfaces(IExternalFields))]
    struct ExternalFields;

    impl IExternalFields for ExternalFields {
        fn echo(&self, value: Sample) -> Result<Sample, NcStatus> {
            Ok(value)
        }
        fn sum(&self, values: NcCopyFromBufferOffset<'_, Sample>) -> Result<f32, NcStatus> {
            Ok(values.as_slice().iter().flat_map(|value| value.pairs).map(|pair| pair.x + pair.y).sum())
        }
    }

    #[test]
    fn external_c_fields_pass_by_value_and_buffer_without_com_traits() {
        let interface: ComPtr<IExternalFieldsAbi> = ExternalFields::from_value(ExternalFields).query().unwrap();
        let value = Sample {
            pairs: [Pair { x: 1.25, y: -2.5 }, Pair { x: 4.0, y: 0.125 }],
        };
        assert_eq!(interface.echo(value).unwrap(), value);
        assert_eq!(interface.sum(NcCopyFromBufferOffset::new(0, &[value, value])).unwrap(), 5.75);
    }

    #[test]
    fn c_struct_rejects_rust_layout_fields() {
        trybuild::TestCases::new().compile_fail("integration-tests/ui/struct_c_layout_rust_field.rs");
    }
}

#[com_interface(auto_guid = "nc.tests.abi_policy.Value:v1")]
pub trait IPolicyValue {
    fn value(&self) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.abi_policy.Service:v1")]
pub trait IPolicyService {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
    fn flush(&self) -> Result<(), NcStatus>;
    fn retain(
        &self,
        value: ComRef<'_, IPolicyValueAbi>,
    ) -> Result<ComPtr<IPolicyValueAbi>, NcStatus>;
}

fn hash_hex(hash: NcAbiHash) -> String {
    let mut out = String::with_capacity(64);
    for byte in hash.bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[test]
fn generated_interface_iids_and_hashes_are_stable() {
    assert_eq!(IPolicyValueAbi::IID, guid_from_qualified_name("nc.tests.abi_policy.Value:v1"));
    assert_eq!(
        unsafe { IPolicyValueAbi::INTERFACE_DESCRIPTOR.qualified_name.as_str() }.unwrap(),
        "nc.tests.abi_policy.Value:v1"
    );
    assert_eq!(
        IPolicyValueAbi::IID.to_string(),
        "be3b63c2-5ea1-5ddd-aa6e-803343b38af3"
    );
    assert_eq!(
        IPolicyValueAbi::ABI_HASH.bytes,
        [0xda, 0x6a, 0xaa, 0x8c, 0xae, 0x87, 0xc9, 0x9a, 0x2c, 0x54, 0x49, 0x27, 0x22, 0xc6, 0x8d, 0xa8, 0xa6, 0x2f, 0xf5, 0x23, 0x7d, 0x06, 0x32, 0x3a, 0x4c, 0x2c, 0x5f, 0xce, 0xc6, 0x73, 0xa2, 0x82,]
    );
    assert_eq!(
        IPolicyServiceAbi::IID.to_string(),
        "e68a0ddb-0d68-5e76-9d03-4186119e1c45"
    );
    assert_eq!(
        IPolicyServiceAbi::IID,
        guid_from_qualified_name("nc.tests.abi_policy.Service:v1")
    );
    assert_eq!(
        IPolicyServiceAbi::ABI_HASH.bytes,
        [0x52, 0x54, 0x87, 0x5f, 0xe6, 0x4e, 0x5e, 0xd7, 0x38, 0x4f, 0xa9, 0xd8, 0x7b, 0x99, 0x3a, 0x1c, 0x35, 0x5d, 0x84, 0xf4, 0xbd, 0x46, 0x0a, 0xc3, 0x88, 0xd8, 0x5f, 0xe4, 0xb4, 0x90, 0x4c, 0x80,]
    );
}

#[test]
fn generated_method_order_and_signatures_are_stable() {
    let value_methods = IPolicyValueAbi::METHOD_DESCRIPTORS;
    assert_eq!(value_methods.len(), 1);
    assert_eq!(unsafe { value_methods[0].name.as_str() }.unwrap(), "value");

    let service_methods = IPolicyServiceAbi::METHOD_DESCRIPTORS;
    assert_eq!(service_methods.len(), 3);

    assert_eq!(unsafe { service_methods[0].name.as_str() }.unwrap(), "add");
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&service_methods[0]) }.unwrap(),
        "add(a: value<i32>, b: value<i32>) -> out<i32> status"
    );

    assert_eq!(
        unsafe { service_methods[1].name.as_str() }.unwrap(),
        "flush"
    );
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&service_methods[1]) }.unwrap(),
        "flush() -> void status"
    );

    assert_eq!(
        unsafe { service_methods[2].name.as_str() }.unwrap(),
        "retain"
    );
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&service_methods[2]) }.unwrap(),
        format!(
            "retain(value: com_ref<thread_local, interface<{}, {}>>) -> com_ptr_out<thread_local, interface<{}, {}>> status",
            IPolicyValueAbi::IID,
            hash_hex(IPolicyValueAbi::ABI_HASH),
            IPolicyValueAbi::IID,
            hash_hex(IPolicyValueAbi::ABI_HASH)
        )
    );
}
