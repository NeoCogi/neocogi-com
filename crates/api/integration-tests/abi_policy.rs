use neocogi_com::{
    ComFactoryImplementation, ComInterface, ComPtr, ComRef, IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_OK, NcAbiSignature, NcContractFingerprint,
    NcContractModuleDescription, NcContractModuleReference, NcFactoryPluginDescription, NcStatus, activate_factory_implementation, com_class_description,
    com_factory, com_factory_description, com_interface, com_object, factory_plugin, format_abi_signature, format_method_signature, guid_from_qualified_name,
    validate_factory_plugin_description,
};

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

#[com_class_description(
    id = "9be73186-f123-47bc-8140-4a57825eeb21",
    name = "nc.tests.abi_policy.DescribedValue",
    version = 3,
    threading = local
)]
/// Constructor-free class shape used to exercise IDL implementation storage.
pub trait DescribedValue: IPolicyValue {
    /// Replaces the stored value through the class's primary projection.
    fn replace(&mut self, value: i32) -> Result<(), NcStatus>;
}

#[com_object(DescribedValue)]
/// Private state implementing a class that can only be obtained from a factory.
struct DescribedValueObject {
    /// Current integer returned by both implemented projections.
    value: i32,
}

impl IPolicyValue for DescribedValueObject {
    /// Reads the class's current integer through its required interface.
    fn value(&self) -> Result<i32, NcStatus> {
        Ok(self.value)
    }
}

impl DescribedValue for DescribedValueObject {
    /// Mutates the class through its generated exclusive-call thunk.
    fn replace(&mut self, value: i32) -> Result<(), NcStatus> {
        self.value = value;
        Ok(())
    }
}

#[com_factory_description(
    id = "564e2ee9-f2de-4e30-959f-a82315a64394",
    name = "nc.tests.abi_policy.ValueFactory",
    version = 1,
    threading = local
)]
/// Typed provider that owns construction policy for `DescribedValue`.
pub trait ValueFactory {
    /// Creates one initialized class instance without a runtime class registry.
    fn create(&self, value: i32) -> Result<ComPtr<DescribedValueAbi>, NcStatus>;
}

#[com_factory(ValueFactory)]
/// Stateless factory implementation used by the constructor-free path.
struct ValueFactoryObject;

impl ValueFactory for ValueFactoryObject {
    /// Allocates the concrete value directly and returns its typed projection.
    fn create(&self, value: i32) -> Result<ComPtr<DescribedValueAbi>, NcStatus> {
        DescribedValueObject::from_value(DescribedValueObject { value }).query()
    }
}

/// Represents the linked contract getter without resolving it in macro tests.
unsafe extern "C" fn get_factory_contract() -> *const NcContractModuleDescription {
    // Descriptor validation does not call contract code. Runtime graph tests
    // use complete generated modules when exercising registration.
    core::ptr::null()
}

/// Exact contract revision referenced by this compile-time plugin fixture.
const FACTORY_CONTRACT: NcContractModuleReference = NcContractModuleReference {
    module_id: neocogi_com::NcGuid::from_fields(1, 2, 3, [4; 8]),
    abi_fingerprint: NcContractFingerprint::new([5; 32]),
    get: Some(get_factory_contract),
};

factory_plugin! {
    name = "nc.tests.factory_plugin",
    version = "1.0.0",
    contracts = [FACTORY_CONTRACT],
    factory = ValueFactoryObject,
}

neocogi_com::export_factory_plugin!();

/// Confirms factories create described objects without runtime construction.
#[test]
fn idl_descriptions_separate_shape_from_construction() {
    let factory: ComPtr<ValueFactoryAbi> = ValueFactoryObject::from_value(ValueFactoryObject).query().unwrap();
    let mut value = factory.create(11).unwrap();
    assert_eq!(value.value().unwrap(), 11);
    value.replace(29).unwrap();
    assert_eq!(value.value().unwrap(), 29);
    assert_eq!(DescribedValueAbi::OBJECT_QUALIFIED_NAME, "nc.tests.abi_policy.DescribedValue");
    assert_eq!(DescribedValueAbi::OBJECT_VERSION, 3);
}

/// Confirms factory implementation metadata and typed bootstrap agree.
#[test]
fn factory_bootstrap_transfers_the_exact_typed_root() {
    fn assert_description<T: ComFactoryImplementation<Description = ValueFactoryAbi>>() {}
    // The generated association lets plugin metadata obtain identity without
    // repeating a factory ID beside the implementation.
    assert_description::<ValueFactoryObject>();

    let mut unknown = core::ptr::null_mut::<IUnknown>();
    // SAFETY: The local output slot accepts one owned IUnknown reference.
    assert_eq!(unsafe { activate_factory_implementation::<ValueFactoryObject>(&mut unknown) }, NC_OK);
    // SAFETY: Successful activation transferred exactly one live reference.
    let unknown = unsafe { ComPtr::<IUnknown>::from_raw_owned(unknown) }.expect("factory activation returned null");
    let factory = unknown
        .query::<ValueFactoryAbi>()
        .expect("activated object omitted its declared factory interface");
    let value = factory.create(41).expect("factory method failed");
    assert_eq!(value.value().expect("value method failed"), 41);
}

/// Confirms plugin metadata derives root identity and capability from IDL.
#[test]
fn factory_plugin_metadata_has_no_repeated_root_contract() {
    let description = factory_plugin_description();
    assert_eq!(description.root_factory_id, ValueFactoryAbi::OBJECT_ID);
    assert_eq!(description.root_factory_threading, NC_ABI_THREADING_THREAD_LOCAL);
    // SAFETY: The macro-generated root and child tables are immutable statics.
    assert_eq!(unsafe { validate_factory_plugin_description(description) }, NC_OK);

    let mut exported = core::ptr::null::<NcFactoryPluginDescription>();
    // SAFETY: `exported` supplies one writable pointer slot to the generated
    // dynamic-library entrypoint.
    assert_eq!(unsafe { nc_get_factory_plugin_description(&mut exported) }, NC_OK);
    assert!(core::ptr::eq(exported, description));
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
