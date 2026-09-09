use contract_abi::{CONTRACT_PLUGIN_ID, ContractPoint, ContractWidget, IContractValue, IContractValueAbi, IContractValueComPtrExt};
use neocogi_com::{ComPtr, ComRef, IUnknown, NcStatus, com_object};

// Plugin-owned value object returned through IContractValueAbi. The host does
// not know this Rust type exists; it only receives the COM interface pointer.
#[com_object(interfaces(contract_abi::IContractValue))]
pub struct PluginValue {
    value: u32,
}

impl IContractValue for PluginValue {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

// Concrete plugin implementation of the published widget class. The generated
// class thunk decodes the shared typed constructor frame before calling `new`.
#[com_object(contract_abi::ContractWidget)]
pub struct PluginWidget {
    /// Seed copied from the host's typed constructor frame.
    seed: u32,
}

impl ContractWidget for PluginWidget {
    /// Creates the private implementation value from the shared class contract.
    fn new(seed: u32) -> Result<Self, NcStatus> {
        Ok(Self { seed })
    }

    fn add(&self, left: u32, right: u32) -> Result<u32, NcStatus> {
        Ok(self.seed + left + right)
    }

    fn translate(&self, point: ContractPoint) -> Result<ContractPoint, NcStatus> {
        Ok(ContractPoint {
            x: point.x + self.seed as i32,
            y: point.y - self.seed as i32,
        })
    }

    fn read_value(&self, value: ComRef<'_, IContractValueAbi>) -> Result<u32, NcStatus> {
        // ComRef is borrowed. add_ref() turns it into an owned ComPtr while this
        // method reads from it, using normal COM reference-counting rules.
        Ok(self.seed + value.value()?)
    }

    fn make_value(&self, value: u32) -> Result<ComPtr<IContractValueAbi>, NcStatus> {
        // Create the private Rust object, erase it to IUnknown, then query the
        // public interface promised by the contract.
        let unknown: ComPtr<IUnknown> = PluginValue::from_value(PluginValue {
            value: self.seed + value,
        });
        unknown.query()
    }
}

neocogi_com::plugin! {
    plugin_id(CONTRACT_PLUGIN_ID),
    name = "nc.tests.contract.plugin",
    version = "1.0.0",
    classes = [PluginWidget],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
neocogi_com::export_plugin!();
