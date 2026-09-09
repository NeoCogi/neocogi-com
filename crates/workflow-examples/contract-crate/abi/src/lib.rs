#![no_std]

//! Shared host/plugin contract.
//!
//! This crate is the only place where the published ABI is defined. Hosts and
//! plugins both depend on it, but it intentionally contains no implementations,
//! loader code, or host policy.

use neocogi_com::{ComPtr, ComRef, NcGuid, NcStatus, com_class, com_interface, com_struct};

// Stable plugin identity derived from its descriptor qualified name. Hosts can
// use this for allowlists, dependency metadata, or diagnostics, but it is
// distinct from class/interface identity.
pub const CONTRACT_PLUGIN_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.tests.contract.plugin");

// Stable class identity derived from its descriptor qualified name. The host
// asks the registry for this class, then separately verifies the interface ABI
// it expects to call.
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

// This is the constructible cross-boundary class used by the example. Its
// constructor is part of the same shared contract as its callable methods, so
// no one-off constructor object or duplicate primary interface is required.
#[com_class(auto_guid = "nc.tests.contract.Widget")]
pub trait ContractWidget {
    /// Creates a widget from one plain ABI seed value.
    #[constructor]
    fn new(seed: u32) -> Result<Self, NcStatus>;

    /// Adds two values after applying the constructor seed.
    fn add(&self, left: u32, right: u32) -> Result<u32, NcStatus>;

    /// Translates one ABI-safe point by the constructor seed.
    fn translate(&self, point: ContractPoint) -> Result<ContractPoint, NcStatus>;

    // ComRef is a borrowed COM interface reference. The callee can AddRef it if
    // it needs to keep an owned reference beyond this call.
    fn read_value(&self, value: ComRef<'_, IContractValueAbi>) -> Result<u32, NcStatus>;

    // ComPtr is an owned COM interface pointer. Returning it transfers one COM
    // reference count from plugin to host.
    fn make_value(&self, value: u32) -> Result<ComPtr<IContractValueAbi>, NcStatus>;
}
