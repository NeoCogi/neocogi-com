use nc_com::{
    ComInterface, ComPtr, ComRef, NcAbiHash, NcStatus, com_interface, format_method_signature,
    guid_from_qualified_name,
};
use std::fmt::Write;

mod external_fields {
    use nc_com::{ComPtr, NcCopyFromBufferOffset, NcStatus, com_object, com_interface, com_struct};

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

    #[com_object(interfaces(IExternalFields))]
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
        [0x14, 0xf6, 0x56, 0xd1, 0xb5, 0x81, 0xce, 0x49, 0xc3, 0xa9, 0x60, 0xf3, 0xe8, 0xfd, 0xb2, 0xd2, 0x17, 0xe3, 0x6e, 0x18, 0xd1, 0xd1, 0x18, 0x79, 0xe5, 0x8d, 0x41, 0x44, 0x6a, 0xf0, 0x2e, 0x7b,]
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
        [0x3c, 0x78, 0xe2, 0x6c, 0x86, 0xfa, 0xe9, 0xa7, 0x82, 0xbc, 0xb2, 0x73, 0x7b, 0x68, 0x56, 0x54, 0x65, 0x5b, 0x31, 0x7f, 0xb3, 0x27, 0x04, 0xc1, 0x51, 0x00, 0xb9, 0x5c, 0xe1, 0x7e, 0xb2, 0x94,]
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
