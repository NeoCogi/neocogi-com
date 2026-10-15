use neocogi_com::{
    ComInterface, ComPtr, ComRef, IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_OK, NcAbiSignature, NcPluginDescriptor, NcStatus,
    write_plugin_root, com_object, com_interface, format_abi_signature, format_method_signature,
    guid_from_qualified_name, validate_plugin_descriptor,
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

    // SAFETY: this external C record contains only unrestricted f32 leaves.
    // External layout alone is insufficient; the adapter explicitly provides
    // the same checked conversion contract required of every other field.
    unsafe impl neocogi_com::ComValue for Pair {
        type Wire = Self;
        type Storage = ();
        fn encode_value(&self, budget: &mut neocogi_com::ValueBudget) -> Result<(Self, ()), NcStatus> {
            budget.value(|_| Ok((*self, ())))
        }
        unsafe fn decode_value(wire: Self, budget: &mut neocogi_com::ValueBudget) -> Result<Self, NcStatus> {
            budget.value(|_| Ok(wire))
        }
    }
    // SAFETY: Pair has no pointer or ownership-bearing field.
    unsafe impl neocogi_com::ComInlineValue for Pair {}

    #[com_struct(c_struct, assigned_signature = b"00000000000000000000000000000010")]
    #[derive(Debug, PartialEq)]
    struct Sample {
        pairs: [Pair; 2],
    }

    #[com_interface(auto_guid = "nc.tests.abi_policy.ExternalFields:v1", assigned_signature = b"00000000000000000000000000000011")]
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
    fn external_c_fields_require_an_explicit_checked_codec() {
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

#[com_interface(auto_guid = "nc.tests.abi_policy.Value:v1", assigned_signature = b"00000000000000000000000000000012")]
pub trait IPolicyValue {
    fn value(&self) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.abi_policy.Service:v1", assigned_signature = b"00000000000000000000000000000013")]
pub trait IPolicyService {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
    fn flush(&self) -> Result<(), NcStatus>;
    fn retain(&self, value: ComRef<'_, IPolicyValueAbi>) -> Result<ComPtr<IPolicyValueAbi>, NcStatus>;
}

#[com_interface(
    assigned_guid = "9be73186-f123-47bc-8140-4a57825eeb21",
    name = "nc.tests.abi_policy.DescribedValue",
    threading = local,
    assigned_signature = b"00000000000000000000000000000014"
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

#[com_interface(
    assigned_guid = "564e2ee9-f2de-4e30-959f-a82315a64394",
    name = "nc.tests.abi_policy.ValueFactory",
    threading = local,
    assigned_signature = b"00000000000000000000000000000015"
)]
/// Typed provider that owns construction policy for `DescribedValue`.
pub trait ValueFactory {
    /// Creates one initialized class instance without a runtime class registry.
    fn create(&self, value: i32) -> Result<ComPtr<DescribedValueAbi>, NcStatus>;
}

#[com_object(ValueFactory)]
/// Stateless factory implementation used by the constructor-free path.
struct ValueFactoryObject;

impl ValueFactory for ValueFactoryObject {
    /// Allocates the concrete value directly and returns its typed projection.
    fn create(&self, value: i32) -> Result<ComPtr<DescribedValueAbi>, NcStatus> {
        DescribedValueObject::from_value(DescribedValueObject { value }).query()
    }
}

/// Acquires one owned root; object construction remains on its typed methods.
#[neocogi_com::plugin(name = "nc.tests.factory_plugin",
    version = "1.0.0")]
fn acquire_root() -> Result<neocogi_com::ComPtr<ValueFactoryAbi>, neocogi_com::NcStatus> {
    ValueFactoryObject::from_value(ValueFactoryObject).query()}

/// Confirms factories create described objects without runtime construction.
#[test]
fn idl_descriptions_separate_shape_from_construction() {
    let factory: ComPtr<ValueFactoryAbi> = ValueFactoryObject::from_value(ValueFactoryObject).query().unwrap();
    let mut value = factory.create(11).unwrap();
    assert_eq!(value.value().unwrap(), 11);
    value.replace(29).unwrap();
    assert_eq!(value.value().unwrap(), 29);
    // Identity has one authoritative interface descriptor, without object aliases.
    assert_eq!(
        unsafe { DescribedValueAbi::INTERFACE_DESCRIPTOR.qualified_name.as_str() }.unwrap(),
        "nc.tests.abi_policy.DescribedValue"
    );
}

/// Confirms factory implementation metadata and typed bootstrap agree.
#[test]
fn factory_bootstrap_transfers_the_exact_typed_root() {
    let mut unknown = neocogi_com::ComRaw::<IUnknown>::null();
    // SAFETY: The local output slot accepts one erased owned factory pair.
    assert_eq!(unsafe { write_plugin_root(&mut unknown, acquire_root) }, NC_OK);
    // SAFETY: Successful acquisition transferred exactly one live reference.
    let factory =
        unsafe { ComPtr::<ValueFactoryAbi>::__nc_from_raw_owned_checked(unknown.cast()) }.expect("acquisition did not return the exact declared factory table");
    let value = factory.create(41).expect("interface method failed");
    assert_eq!(value.value().expect("value method failed"), 41);
}

/// Confirms plugin metadata derives root identity and capability from IDL.
#[test]
fn factory_plugin_metadata_has_no_repeated_root_contract() {
    let description = plugin_descriptor();
    assert_eq!(description.description_abi_version, 1);
    assert_eq!(description.root_factory, ValueFactoryAbi::INTERFACE_ID);
    assert_eq!(description.root_factory_threading, NC_ABI_THREADING_THREAD_LOCAL);
    // SAFETY: The macro-generated root and child tables are immutable statics.
    assert_eq!(unsafe { validate_plugin_descriptor(description) }, NC_OK);

    let mut exported = core::ptr::null::<NcPluginDescriptor>();
    // SAFETY: `exported` supplies one writable pointer slot to the generated
    // dynamic-library entrypoint.
    assert_eq!(unsafe { neocogi_com::return_plugin_descriptor(&mut exported, plugin_descriptor()) }, NC_OK);
    assert!(core::ptr::eq(exported, description));
    // The renamed C export preserves the null-output error contract.
    assert_eq!(unsafe { neocogi_com::return_plugin_descriptor(core::ptr::null_mut(), plugin_descriptor()) }, neocogi_com::NC_E_POINTER);
}

fn signature_text(signature: NcAbiSignature) -> String {
    // SAFETY: Macro-generated signatures reference immutable static storage in
    // this test component for the duration of the process.
    unsafe { format_abi_signature(signature) }
}

#[test]
fn generated_interface_iids_and_signatures_are_stable() {
    assert_eq!(IPolicyValueAbi::IID, guid_from_qualified_name("nc.tests.abi_policy.Value:v1"));
    assert_eq!(
        unsafe { IPolicyValueAbi::INTERFACE_DESCRIPTOR.qualified_name.as_str() }.unwrap(),
        "nc.tests.abi_policy.Value:v1"
    );
    assert_eq!(IPolicyValueAbi::IID.to_string(), "be3b63c2-5ea1-5ddd-aa6e-803343b38af3");
    assert_eq!(IPolicyServiceAbi::IID.to_string(), "e68a0ddb-0d68-5e76-9d03-4186119e1c45");
    assert_eq!(IPolicyServiceAbi::IID, guid_from_qualified_name("nc.tests.abi_policy.Service:v1"));
    assert!(signature_text(IPolicyServiceAbi::ABI_SIGNATURE).starts_with("sha-256:"));
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
