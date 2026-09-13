//! Runtime checks for standalone IDL module-description libraries.

use neocogi_com::{NC_CONTRACT_DESCRIPTION_ABI_VERSION, NC_OK, validate_contract_module_description};
use neocogi_idl_example_consumer_contracts::MODULE as CONSUMER_MODULE;
use neocogi_idl_example_contracts::MODULE as EXAMPLE_MODULE;

/// Confirms an importing module reaches its exact shared dependency description.
#[test]
fn shared_description_reaches_exact_dependency() {
    // SAFETY: Cargo linked both fingerprint-qualified description libraries and
    // keeps them loaded for the lifetime of this test process.
    let consumer = unsafe { CONSUMER_MODULE.description() }.unwrap();
    assert_eq!(consumer.description_abi_version, NC_CONTRACT_DESCRIPTION_ABI_VERSION);

    // SAFETY: Successful root validation established the pointer/count shape
    // for the shared library's immutable dependency table.
    let dependencies = unsafe { consumer.dependencies() }.unwrap();
    assert_eq!(dependencies.len(), 1);
    let dependency = &dependencies[0];
    assert_eq!(dependency.module_id, EXAMPLE_MODULE.module_id);
    assert_eq!(dependency.abi_fingerprint, EXAMPLE_MODULE.abi_fingerprint);

    let get = dependency.get.expect("generated dependency must retain its metadata getter");
    // SAFETY: The linked dependency library remains loaded for the whole test.
    let description = unsafe { get() };
    // SAFETY: The returned pointer and its immutable tables remain library-owned.
    assert_eq!(unsafe { validate_contract_module_description(description) }, NC_OK);
}
