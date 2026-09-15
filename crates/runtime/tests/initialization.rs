//! Production C-ABI first-use checks in a test process with its own runtime image.
//!
//! Unit tests share a parallel harness and cannot reliably observe the
//! unseeded state of their singleton. This fixture calls the exported tables
//! in a separate test executable before any other runtime operation.

use nc_com_shared_runtime as _;
use neocogi_com_core::{NC_E_POINTER, NcContractRegistryV2, NcRuntimeApiV2};

unsafe extern "C" {
    /// Returns the immutable runtime operation table exported by the linked image.
    fn nc_com_get_runtime_api_v2() -> *const NcRuntimeApiV2;
    /// Returns the immutable contract registry table exported by the linked image.
    fn nc_com_get_contract_registry_v2() -> *const NcContractRegistryV2;
}

/// First use seeds the intrinsic events module without a host startup call.
#[test]
fn first_api_call_seeds_one_builtin_without_host_initialization() {
    // SAFETY: Both C getters return immutable process-lifetime table pointers.
    let runtime = unsafe { &*nc_com_get_runtime_api_v2() };
    // SAFETY: The linked runtime exports a complete immutable v2 registry table.
    let contracts = unsafe { &*nc_com_get_contract_registry_v2() };

    assert_eq!(unsafe { (runtime.register_contract_module)(core::ptr::null()) }, NC_E_POINTER);
    assert_eq!(unsafe { (contracts.module_count)() }, 1);
    let mut module_id = neocogi_com_core::NcGuid::nil();
    let mut written = 0;
    assert_eq!(unsafe { (contracts.module_ids)(&mut module_id, 1, &mut written) }, neocogi_com_core::NC_OK);
    assert_eq!(written, 1);
    assert_eq!(module_id, neocogi_com_events_contracts::MODULE_ID);
}
