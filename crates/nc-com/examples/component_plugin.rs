use nc_com::{
    ComInterface, ComPtr, IUnknown, NcClassDescriptor, NcGuid, NcStatus, ThreadSafe, com_class,
    com_interface,
};

const GREETER_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.component.Greeter");

const COMPONENT_PLUGIN_ID: NcGuid = nc_com::guid_from_qualified_name("nc.examples.component");

#[com_interface(auto_guid = "nc.examples.component.Greeter:v1")]
pub trait IGreeter {
    fn greeting_code(&self) -> Result<u32, NcStatus>;
}

#[com_interface(auto_guid = "nc.examples.component.GreeterArgs:v1")]
pub trait IGreeterArgs {
    fn seed(&self) -> Result<u32, NcStatus>;
}

#[com_class(
    thread_safe,
    class_id(GREETER_CLASS_ID),
    name = "nc.examples.component.Greeter",
    version = "1.0.0",
    constructor(args = IGreeterArgs, function = construct_greeter),
    interfaces(IGreeter)
)]
pub struct Greeter {
    greeting_code: u32,
}

impl IGreeter for Greeter {
    fn greeting_code(&self) -> Result<u32, NcStatus> {
        Ok(self.greeting_code)
    }
}

unsafe extern "C" fn construct_greeter(
    class: *const NcClassDescriptor,
    arguments: *mut IUnknown,
    out: *mut *mut IUnknown,
) -> NcStatus {
    unsafe {
        nc_com::construct_class_checked(
            class,
            arguments,
            out,
            Greeter::CLASS_ID,
            IGreeterArgsAbi::INTERFACE_ID,
            |arguments| {
                let args = nc_com::query_constructor_arguments::<IGreeterArgsAbi>(arguments)?;
                let unknown: ComPtr<IUnknown, ThreadSafe> = Greeter::from_value(Greeter {
                    greeting_code: args.seed()? + 100,
                });
                Ok(unknown)
            },
        )
    }
}

nc_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_ID),
    name = "nc.examples.component",
    version = "1.0.0",
    classes = [Greeter],
}
