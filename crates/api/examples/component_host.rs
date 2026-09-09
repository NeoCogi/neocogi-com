#[cfg(unix)]
fn main() {
    unix_host::run();
}

#[cfg(not(unix))]
fn main() {
    eprintln!("component_host currently demonstrates Unix dlopen loading only");
}

#[cfg(unix)]
mod unix_host {
    use neocogi_com::{
        ComClassArguments, ComPtr, IUnknown, NC_GET_PLUGIN_DESCRIPTOR_SYMBOL, NcClassDescriptor, NcGetPluginDescriptor, NcPluginDescriptor, NcStatus,
        com_class, validate_plugin_descriptor_with_issues,
    };
    use std::env;
    use std::ffi::{CStr, CString, c_char, c_int, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    /// Callable class contract mirrored from the dynamically loaded component.
    #[allow(dead_code)]
    #[com_class(auto_guid = "nc.examples.component.Greeter")]
    pub trait Greeter {
        /// Creates a greeter from one plain seed value.
        #[constructor]
        fn new(seed: u32) -> Result<Self, NcStatus>;

        /// Returns the code computed by the plugin implementation.
        fn greeting_code(&self) -> Result<u32, NcStatus>;
    }

    pub fn run() {
        let Some(path) = env::args_os().nth(1) else {
            eprintln!("usage: cargo run --example component_host -- <component shared library>");
            return;
        };

        let library = Library::open(Path::new(&path));
        let get_plugin_descriptor = library.get_plugin_descriptor();
        let plugin = unsafe { load_plugin_descriptor(get_plugin_descriptor) };
        let table = unsafe { &*plugin.class_table };
        let classes = unsafe { table.classes() }.expect("component returned a valid class table");

        for class in classes {
            let name = unsafe { class.qualified_name_str() }.unwrap_or("<invalid>");
            println!("class: {name}");
        }

        let class = classes.first().expect("component exports one class");
        let greeter = construct_greeter(class);
        println!("greeting code: {}", greeter.greeting_code().unwrap());
    }

    unsafe fn load_plugin_descriptor(
        get_plugin_descriptor: NcGetPluginDescriptor,
    ) -> &'static NcPluginDescriptor {
        let mut plugin = std::ptr::null();
        let status = unsafe { get_plugin_descriptor(&mut plugin) };
        assert!(status.is_ok(), "plugin descriptor failed: {status}");
        assert!(!plugin.is_null(), "plugin descriptor pointer was null");
        let status = unsafe { validate_plugin_descriptor_with_issues(plugin, |_| {}) };
        assert!(
            status.is_ok(),
            "plugin descriptor validation failed: {status}"
        );
        unsafe { &*plugin }
    }

    fn construct_greeter(class: &NcClassDescriptor) -> ComPtr<GreeterAbi> {
        let arguments = GreeterAbi::arguments(23).expect("valid constructor arguments");
        assert_eq!(class.constructor_abi_hash, GreeterAbi::CONSTRUCTOR_ABI_HASH);
        let constructor = class.construct.expect("class has constructor");
        let mut out = std::ptr::null_mut();
        let status = unsafe { constructor(class, arguments.as_abi_ptr(), &mut out) };
        assert!(status.is_ok(), "constructor failed: {status}");
        let unknown = unsafe { ComPtr::<IUnknown>::from_raw_owned(out) }.expect("constructor output");
        unknown.query::<GreeterAbi>().expect("constructed greeter")
    }

    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
        fn dlerror() -> *const c_char;
    }

    const RTLD_NOW: c_int = 2;

    struct Library {
        handle: *mut c_void,
    }

    impl Library {
        fn open(path: &Path) -> Self {
            let path =
                CString::new(path.as_os_str().as_bytes()).expect("path contains no NUL bytes");
            let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW) };
            assert!(!handle.is_null(), "dlopen failed: {}", dl_error());
            Self { handle }
        }

        fn get_plugin_descriptor(&self) -> NcGetPluginDescriptor {
            let symbol = CString::new(NC_GET_PLUGIN_DESCRIPTOR_SYMBOL).unwrap();
            let ptr = unsafe { dlsym(self.handle, symbol.as_ptr()) };
            assert!(!ptr.is_null(), "dlsym failed: {}", dl_error());
            unsafe { std::mem::transmute::<*mut c_void, NcGetPluginDescriptor>(ptr) }
        }
    }

    impl Drop for Library {
        fn drop(&mut self) {
            unsafe {
                dlclose(self.handle);
            }
        }
    }

    fn dl_error() -> String {
        let error = unsafe { dlerror() };
        if error.is_null() {
            return "unknown dynamic loader error".to_string();
        }
        unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned()
    }
}
