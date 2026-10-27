#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
use neocogi_com::{
    ComInterface, ComPtr, IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_OK, NcAbiSignature, NcPluginDescriptor, NcStatus, write_plugin_root, com_object,
    format_abi_signature, format_method_signature, guid_from_qualified_name, validate_plugin_descriptor,
};

mod external_fields {
    use neocogi_com::{ComPtr, NcCopyFromBufferOffset, NcStatus, com_object};

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
        type Wire = <ContractPair as neocogi_com::ComValue>::Wire;
        type Storage = ();
        fn encode_value(&self, budget: &mut neocogi_com::ValueBudget) -> Result<(Self::Wire, ()), NcStatus> {
            neocogi_com::ComValue::encode_value(&ContractPair { x: self.x, y: self.y }, budget)
        }
        unsafe fn decode_value(wire: Self::Wire, budget: &mut neocogi_com::ValueBudget) -> Result<Self, NcStatus> {
            // SAFETY: the caller supplies the explicit generated ContractPair wire.
            let pair = unsafe { <ContractPair as neocogi_com::ComValue>::decode_value(wire, budget) }?;
            Ok(Self { x: pair.x, y: pair.y })
        }
    }
    // SAFETY: Pair has no pointer or ownership-bearing field.
    unsafe impl neocogi_com::ComInlineValue for Pair {}

    #[allow(unused_imports)]
    pub use fixture_abi_policy::{ExternalFieldsContractPair as ContractPair};

    #[allow(unused_imports)]
    pub use fixture_abi_policy::{ExternalFieldsSample as Sample};

    #[allow(unused_imports)]
    pub use fixture_abi_policy::{
        ExternalFieldsIExternalFieldsAbi as IExternalFieldsAbi, ExternalFieldsIExternalFieldsVTable as IExternalFieldsVTable,
        ExternalFieldsIExternalFieldsVtblFor as IExternalFieldsVtblFor, ExternalFieldsIExternalFieldsClient as IExternalFieldsClient,
        ExternalFieldsIExternalFields as IExternalFields,
    };

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
            pairs: [ContractPair { x: 1.25, y: -2.5 }, ContractPair { x: 4.0, y: 0.125 }],
        };
        assert_eq!(interface.echo(value).unwrap(), value);
        assert_eq!(interface.sum(NcCopyFromBufferOffset::new(0, &[value, value])).unwrap(), 5.75);
    }

    #[test]
    fn native_adapter_uses_the_generated_contract_wire() {
        use neocogi_com::ComValue;
        let interface: ComPtr<IExternalFieldsAbi> = ExternalFields::from_value(ExternalFields).query().unwrap();
        let native = Pair { x: 1.25, y: -2.5 };
        let (pair_wire, _storage) = native.encode_value(&mut Default::default()).unwrap();
        let contract_pair = unsafe { ContractPair::decode_value(pair_wire, &mut Default::default()) }.unwrap();
        let (input, _storage) = Sample { pairs: [contract_pair; 2] }.encode_value(&mut Default::default()).unwrap();
        let raw = interface.as_raw();
        let mut output = core::mem::MaybeUninit::uninit();
        // SAFETY: real checked input wire, live owned provider and writable exact output.
        assert_eq!(unsafe { ((*raw.vtable).echo)(raw.object, input, output.as_mut_ptr()) }, neocogi_com::NC_OK);
        let decoded = unsafe { Sample::decode_value(output.assume_init(), &mut Default::default()) }.unwrap();
        let (returned, _) = decoded.pairs[0].encode_value(&mut Default::default()).unwrap();
        let result = unsafe { Pair::decode_value(returned, &mut Default::default()) }.unwrap();
        assert_eq!(result, native);
    }
}

extern crate self as fixture_abi_policy;
test_contracts::abi_policy!();

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
#[neocogi_com::plugin(name = "nc.tests.factory_plugin", version = "1.0.0")]
fn acquire_root(_runtime: ::neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<ValueFactoryAbi>, neocogi_com::NcStatus> {
    ValueFactoryObject::from_value(ValueFactoryObject).query()
}

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
    assert_eq!(unsafe { write_plugin_root(&mut unknown, || acquire_root(crate::host_runtime())) }, NC_OK);
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
    assert_eq!(
        unsafe { neocogi_com::return_plugin_descriptor(core::ptr::null_mut(), plugin_descriptor()) },
        neocogi_com::NC_E_POINTER
    );
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
        "nc.tests.abi_policy.IPolicyValue"
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

/// Initializes this final test executable's selected host and admits its static tables.
fn host_runtime() -> neocogi_com::Runtime {
    // SAFETY: the explicit implementation owns its complete process-lived graph.
    unsafe { neocogi_com::Runtime::__from_host_api(neocogi_com_host::initialize_process()) }.expect("test host API is compatible")
}
