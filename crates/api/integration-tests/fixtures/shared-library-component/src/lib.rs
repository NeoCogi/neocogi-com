use neocogi_com::{
    ComPtr, IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_COM_CORE_ABI_VERSION, NcAbiSignature, NcAbiSignaturePart, NcAbiPassing, NcAbiType, NcClassDescriptor,
    NcClassTable, NcConstructorFn, NcGetPluginDescriptor, NcGuid, NcPluginDependencyDescriptor, NcPluginDescriptor, NcStaticUtf8, NcStatus, com_object,
    com_interface,
};

const WIDGET_CLASS_ID: NcGuid =
    neocogi_com::guid_from_qualified_name("nc.tests.shared_component.TestWidget");

#[com_interface(auto_guid = "nc.tests.shared_component.Widget:v1")]
pub trait IWidget {
    fn value(&self) -> Result<u32, NcStatus>;
}

const WIDGET_CONSTRUCTOR_ABI_SIGNATURE: NcAbiSignature = neocogi_com::abi_signature!(&[NcAbiSignaturePart::Arg {
    index: 0,
    passing: NcAbiPassing::Value,
    ty: NcAbiType::U32,
}]);

/// Concrete constructor frame shared structurally with the host fixture.
#[repr(C)]
struct WidgetArguments {
    /// Base value copied into the constructed widget.
    base: u32,
}

#[com_object(interfaces(IWidget))]
pub struct Widget {
    value: u32,
}

impl IWidget for Widget {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

static CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
    class_id: WIDGET_CLASS_ID,
    qualified_name: NcStaticUtf8::from_static("nc.tests.shared_component.TestWidget"),
    class_version: NcStaticUtf8::from_static("1.0.0"),
    constructor_abi_signature: WIDGET_CONSTRUCTOR_ABI_SIGNATURE,
    threading: NC_ABI_THREADING_THREAD_LOCAL,
    construct: Some(construct_widget as NcConstructorFn),
    interface_count: Widget::COM_INTERFACES.len() as u32,
    interfaces: Widget::COM_INTERFACES.as_ptr(),
}];

static CLASS_TABLE: NcClassTable = NcClassTable {
    class_count: CLASSES.len() as u32,
    classes: CLASSES.as_ptr(),
};

static PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    validation_mode: neocogi_com::NC_ABI_VALIDATION_MODE,
    plugin_id: neocogi_com::guid_from_qualified_name("nc.tests.shared_component"),
    qualified_name: NcStaticUtf8::from_static("nc.tests.shared_component"),
    plugin_version: NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: core::ptr::null(),
    class_table: &CLASS_TABLE,
};

static MISSING_DEPENDENCIES: [NcPluginDependencyDescriptor; 1] = [NcPluginDependencyDescriptor {
    plugin_id: NcGuid::from_fields(0xffff_ffff, 0xffff, 0xffff, [0xff; 8]),
}];

static MISSING_DEPENDENCY_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    validation_mode: neocogi_com::NC_ABI_VALIDATION_MODE,
    plugin_id: PLUGIN.plugin_id,
    qualified_name: PLUGIN.qualified_name,
    plugin_version: PLUGIN.plugin_version,
    dependency_count: MISSING_DEPENDENCIES.len() as u32,
    dependencies: MISSING_DEPENDENCIES.as_ptr(),
    class_table: &CLASS_TABLE,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_plugin_descriptor(out: *mut *const NcPluginDescriptor) -> NcStatus {
    let plugin = if std::env::var_os("NC_COM_TEST_MISSING_PLUGIN_DEPENDENCY").is_some() {
        &MISSING_DEPENDENCY_PLUGIN
    } else {
        &PLUGIN
    };
    unsafe { neocogi_com::return_plugin_descriptor(out, plugin) }
}

const _: NcGetPluginDescriptor = nc_get_plugin_descriptor;

unsafe extern "C" fn construct_widget(class: *const NcClassDescriptor, arguments: *const core::ffi::c_void, out: *mut *mut IUnknown) -> NcStatus {
    unsafe {
        neocogi_com::construct_class_checked(class, arguments, out, WIDGET_CLASS_ID, WIDGET_CONSTRUCTOR_ABI_SIGNATURE, |arguments| {
            // SAFETY: The checked wrapper matched this non-null frame's ABI
            // signature before the plugin reads its `repr(C)` fields.
            let args = &*arguments.cast::<WidgetArguments>();
            let unknown: ComPtr<IUnknown> = Widget::from_value(Widget { value: args.base + 10 });
            Ok(unknown)
        })
    }
}
