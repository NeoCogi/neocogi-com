use neocogi_com::{ComInterface, ComPtr, ComRef, NcAbiSignature, NcStatus, com_interface, format_abi_signature, format_method_signature, guid_from_qualified_name};

mod external_fields {
    use neocogi_com::{ComPtr, NcCopyFromBufferOffset, NcStatus, com_object, com_interface, com_struct};

    // Models a C-layout dependency type with no neocogi-com traits.
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
    fn retain(&self, value: ComRef<'_, IPolicyValueAbi>) -> Result<ComPtr<IPolicyValueAbi>, NcStatus>;
}

fn signature_text(signature: NcAbiSignature) -> String {
    // SAFETY: Macro-generated signatures reference immutable static storage in
    // this test component for the duration of the process.
    unsafe { format_abi_signature(signature) }.expect("valid generated signature view")
}

#[test]
fn generated_interface_iids_and_signatures_are_stable() {
    assert_eq!(IPolicyValueAbi::IID, guid_from_qualified_name("nc.tests.abi_policy.Value:v1"));
    assert_eq!(
        unsafe { IPolicyValueAbi::INTERFACE_DESCRIPTOR.qualified_name.as_str() }.unwrap(),
        "nc.tests.abi_policy.Value:v1"
    );
    assert_eq!(IPolicyValueAbi::IID.to_string(), "be3b63c2-5ea1-5ddd-aa6e-803343b38af3");
    #[cfg(feature = "com-validation-mangling")]
    assert_eq!(
        signature_text(IPolicyValueAbi::ABI_SIGNATURE),
        "ncabi1;Ibe3b63c25ea15dddaa6e803343b38af3M0000000000000005valueR00O0209S"
    );
    assert_eq!(IPolicyServiceAbi::IID.to_string(), "e68a0ddb-0d68-5e76-9d03-4186119e1c45");
    assert_eq!(IPolicyServiceAbi::IID, guid_from_qualified_name("nc.tests.abi_policy.Service:v1"));
    #[cfg(feature = "com-validation-mangling")]
    assert_eq!(
        signature_text(IPolicyServiceAbi::ABI_SIGNATURE),
        concat!(
            "ncabi1;Ie68a0ddb0d685e769d034186119e1c45",
            "M0000000000000003addR00A000000000109A000000010109O0209S",
            "M0000000100000005flushR00O0000S",
            "M0000000200000006retainR00",
            "A00000000050012be3b63c25ea15dddaa6e803343b38af3",
            "O060012be3b63c25ea15dddaa6e803343b38af3S",
        )
    );
    #[cfg(feature = "com-validation-sha-256")]
    assert!(signature_text(IPolicyServiceAbi::ABI_SIGNATURE).starts_with("sha-256:"));
    #[cfg(feature = "com-validation-fnv-1a-128")]
    assert!(signature_text(IPolicyServiceAbi::ABI_SIGNATURE).starts_with("fnv-1a-128:"));
    #[cfg(feature = "com-validation-none")]
    assert_eq!(IPolicyServiceAbi::ABI_SIGNATURE, NcAbiSignature::UNCHECKED);
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

    assert_eq!(unsafe { service_methods[1].name.as_str() }.unwrap(), "flush");
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&service_methods[1]) }.unwrap(),
        "flush() -> void status"
    );

    assert_eq!(unsafe { service_methods[2].name.as_str() }.unwrap(), "retain");
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&service_methods[2]) }.unwrap(),
        format!(
            "retain(value: com_ref<thread_local, interface<{}, {}>>) -> com_ptr_out<thread_local, interface<{}, {}>> status",
            IPolicyValueAbi::IID,
            signature_text(IPolicyValueAbi::ABI_SIGNATURE),
            IPolicyValueAbi::IID,
            signature_text(IPolicyValueAbi::ABI_SIGNATURE)
        )
    );
}
