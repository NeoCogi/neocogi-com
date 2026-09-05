#![no_std]

//! Shared host/plugin contract.
//!
//! This crate is the only place where the published ABI is defined. Hosts and
//! plugins both depend on it, but it intentionally contains no implementations,
//! loader code, or host policy.

use nc_com::{ComPtr, ComRef, NcGuid, NcStatus, com_interface, com_struct};

// Stable plugin identity derived from its descriptor qualified name. Hosts can
// use this for allowlists, dependency metadata, or diagnostics, but it is
// distinct from class/interface identity.
pub const CONTRACT_PLUGIN_ID: NcGuid = nc_com::guid_from_qualified_name("nc.tests.contract.plugin");

// Stable class identity derived from its descriptor qualified name. The host
// asks the registry for this class, then separately verifies the interface ABI
// it expects to call.
pub const CONTRACT_WIDGET_CLASS_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.tests.contract.Widget");

// Records crossing the COM boundary must be ABI-safe value types. #[com_struct]
// gives the generated interface metadata a stable description of this record.
#[com_struct]
#[derive(Debug, Eq, PartialEq)]
pub struct ContractPoint {
    pub x: i32,
    pub y: i32,
}

// auto_guid derives a deterministic interface IID from this semantic name.
// The method list and typed ABI signatures are covered separately by ABI_HASH.
#[com_interface(auto_guid = "nc.tests.contract.Value:v1")]
pub trait IContractValue {
    fn value(&self) -> Result<u32, NcStatus>;
}

// Constructor arguments are passed as COM objects too. That keeps constructor
// input versioned and queryable by IID instead of relying on Rust types.
#[com_interface(auto_guid = "nc.tests.contract.WidgetArgs:v1")]
pub trait IContractWidgetArgs {
    fn seed(&self) -> Result<u32, NcStatus>;
}

// This is the main cross-boundary interface used by the example. Changing
// method order, argument types, return types, COM passing mode, or referenced
// interface hashes changes IContractWidgetAbi::ABI_HASH.
#[com_interface(auto_guid = "nc.tests.contract.Widget:v1")]
pub trait IContractWidget {
    fn add(&self, left: u32, right: u32) -> Result<u32, NcStatus>;
    fn translate(&self, point: ContractPoint) -> Result<ContractPoint, NcStatus>;

    // ComRef is a borrowed COM interface reference. The callee can AddRef it if
    // it needs to keep an owned reference beyond this call.
    fn read_value(&self, value: ComRef<'_, IContractValueAbi>) -> Result<u32, NcStatus>;

    // ComPtr is an owned COM interface pointer. Returning it transfers one COM
    // reference count from plugin to host.
    fn make_value(&self, value: u32) -> Result<ComPtr<IContractValueAbi>, NcStatus>;
}
