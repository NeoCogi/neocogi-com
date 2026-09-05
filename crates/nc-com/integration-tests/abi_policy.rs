use nc_com::{
    ComInterface, ComPtr, ComRef, NcAbiHash, NcStatus, com_interface, format_method_signature,
    guid_from_qualified_name,
};
use std::fmt::Write;

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
    assert_eq!(
        IPolicyValueAbi::IID,
        guid_from_qualified_name("nc.tests.abi_policy.Value:v1")
    );
    assert_eq!(
        unsafe {
            IPolicyValueAbi::INTERFACE_DESCRIPTOR
                .qualified_name
                .as_str()
        }
        .unwrap(),
        "nc.tests.abi_policy.Value:v1"
    );
    assert_eq!(
        IPolicyValueAbi::IID.to_string(),
        "be3b63c2-5ea1-5ddd-aa6e-803343b38af3"
    );
    assert_eq!(
        IPolicyValueAbi::ABI_HASH.bytes,
        [
            0x40, 0x18, 0x65, 0x75, 0x17, 0x4c, 0xfd, 0xcc, 0x14, 0xa0, 0xbc, 0x33, 0xeb, 0x43,
            0x4f, 0x88, 0x0a, 0x9d, 0x79, 0x6b, 0xe7, 0x65, 0xdb, 0x94, 0x86, 0x08, 0xb2, 0x64,
            0x28, 0x38, 0xca, 0xb7,
        ]
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
        [
            0x05, 0xcb, 0x37, 0x56, 0x14, 0x97, 0x6d, 0xf8, 0x3c, 0x38, 0x38, 0xc9, 0x62, 0x2f,
            0xb0, 0x5b, 0xe1, 0xb9, 0x5c, 0x33, 0x5d, 0x2d, 0x6a, 0xc7, 0x6e, 0xa4, 0x04, 0xb5,
            0x49, 0x4c, 0x00, 0x9c,
        ]
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
