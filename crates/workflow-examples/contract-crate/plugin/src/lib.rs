use contract_abi::{
    CONTRACT_PLUGIN_ID, CONTRACT_WIDGET_CLASS_ID, ContractPoint, IContractValue, IContractValueAbi,
    IContractValueComPtrExt, IContractWidget, IContractWidgetArgsAbi, IContractWidgetArgsComPtrExt,
};
use nc_com::{ComInterface, ComPtr, ComRef, IUnknown, NcClassDescriptor, NcStatus, com_class};

// Plugin-owned value object returned through IContractValueAbi. The host does
// not know this Rust type exists; it only receives the COM interface pointer.
#[com_class(interfaces(contract_abi::IContractValue))]
pub struct PluginValue {
    value: u32,
}

impl IContractValue for PluginValue {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

// Concrete plugin implementation of the published widget contract. The seed is
// supplied by the host through the IContractWidgetArgs constructor object.
#[com_class(
    class_id(CONTRACT_WIDGET_CLASS_ID),
    name = "nc.tests.contract.Widget",
    version = "1.0.0",
    constructor(args = contract_abi::IContractWidgetArgs, function = construct_widget),
    interfaces(contract_abi::IContractWidget)
)]
pub struct PluginWidget {
    seed: u32,
}

impl IContractWidget for PluginWidget {
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
        Ok(self.seed + value.add_ref().value()?)
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

unsafe extern "C" fn construct_widget(
    class: *const NcClassDescriptor,
    arguments: *mut IUnknown,
    out: *mut *mut IUnknown,
) -> NcStatus {
    unsafe {
        // The helper checks class identity, constructor argument identity,
        // nulls, and panic boundaries before returning an owned IUnknown to the
        // host.
        nc_com::construct_class_checked(
            class,
            arguments,
            out,
            PluginWidget::CLASS_ID,
            IContractWidgetArgsAbi::INTERFACE_ID,
            |arguments| {
                let args =
                    nc_com::query_constructor_arguments::<IContractWidgetArgsAbi>(arguments)?;
                let seed = args.seed()?;
                let unknown: ComPtr<IUnknown> = PluginWidget::from_value(PluginWidget { seed });
                Ok(unknown)
            },
        )
    }
}

nc_com::plugin! {
    plugin_id(CONTRACT_PLUGIN_ID),
    name = "nc.tests.contract.plugin",
    version = "1.0.0",
    classes = [PluginWidget],
}
