use nc_com::{
    ComInterface, ComPtr, ComRef, IUnknown, NC_ABI_THREADING_THREAD_SAFE, NC_COM_CORE_ABI_VERSION,
    NcClassDescriptor, NcClassTable, NcConstructorFn, NcGetPluginDescriptor, NcGuid,
    NcPluginDescriptor, NcStaticUtf8, NcStatus, ThreadSafe, com_class, com_interface,
};

const CROSS_VERSION_CLASS_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.tests.cross_version.Service");

#[com_interface(auto_guid = "nc.tests.cross_version.Number:v1")]
pub trait ICrossVersionNumber {
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.cross_version.Args:v1")]
pub trait ICrossVersionArgs {
    fn seed(&self) -> Result<u32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.cross_version.Service:v1")]
pub trait ICrossVersionService {
    fn add(&self, left: u32, right: u32) -> Result<u32, NcStatus>;
    fn reset(&self) -> Result<(), NcStatus>;
    fn read_number(&self, number: ComRef<'_, ICrossVersionNumberAbi>) -> Result<u32, NcStatus>;
    fn make_number(&self, value: u32) -> Result<ComPtr<ICrossVersionNumberAbi>, NcStatus>;
}

#[com_class(interfaces(ICrossVersionNumber))]
pub struct CrossVersionNumber {
    value: u32,
}

impl ICrossVersionNumber for CrossVersionNumber {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

#[com_class(thread_safe, interfaces(ICrossVersionService))]
pub struct CrossVersionService {
    seed: u32,
}

impl ICrossVersionService for CrossVersionService {
    fn add(&self, left: u32, right: u32) -> Result<u32, NcStatus> {
        Ok(self.seed + left + right)
    }

    fn reset(&self) -> Result<(), NcStatus> {
        Ok(())
    }

    fn read_number(&self, number: ComRef<'_, ICrossVersionNumberAbi>) -> Result<u32, NcStatus> {
        Ok(self.seed + number.add_ref().value()?)
    }

    fn make_number(&self, value: u32) -> Result<ComPtr<ICrossVersionNumberAbi>, NcStatus> {
        let unknown: ComPtr<IUnknown> = CrossVersionNumber::from_value(CrossVersionNumber {
            value: self.seed + value,
        });
        unknown.query()
    }
}

static CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
    class_id: CROSS_VERSION_CLASS_ID,
    qualified_name: NcStaticUtf8::from_static("nc.tests.cross_version.Service"),
    class_version: NcStaticUtf8::from_static("1.0.0"),
    constructor_args: ICrossVersionArgsAbi::INTERFACE_ID,
    threading: NC_ABI_THREADING_THREAD_SAFE,
    construct: Some(construct_service as NcConstructorFn),
    interface_count: CrossVersionService::COM_INTERFACES.len() as u32,
    interfaces: CrossVersionService::COM_INTERFACES.as_ptr(),
}];

static CLASS_TABLE: NcClassTable = NcClassTable {
    class_count: CLASSES.len() as u32,
    classes: CLASSES.as_ptr(),
};

static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    plugin_id: nc_com::guid_from_qualified_name("nc.tests.cross_version"),
    qualified_name: NcStaticUtf8::from_static("nc.tests.cross_version"),
    plugin_version: NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: core::ptr::null(),
    class_table: &CLASS_TABLE,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_plugin_descriptor(out: *mut *const NcPluginDescriptor) -> NcStatus {
    unsafe { nc_com::return_plugin_descriptor(out, &PLUGIN) }
}

const _: NcGetPluginDescriptor = nc_get_plugin_descriptor;

unsafe extern "C" fn construct_service(
    class: *const NcClassDescriptor,
    arguments: *mut IUnknown,
    out: *mut *mut IUnknown,
) -> NcStatus {
    unsafe {
        nc_com::construct_class_checked(
            class,
            arguments,
            out,
            CROSS_VERSION_CLASS_ID,
            ICrossVersionArgsAbi::INTERFACE_ID,
            |arguments| {
                let args = nc_com::query_constructor_arguments::<ICrossVersionArgsAbi>(arguments)?;
                let unknown: ComPtr<IUnknown, ThreadSafe> =
                    CrossVersionService::from_value(CrossVersionService { seed: args.seed()? });
                Ok(unknown)
            },
        )
    }
}
