//! The final application lends its linked runtime to separately built providers.
extern crate self as linked_host_app;
/// Bindings compiled independently from both native providers.
pub mod contracts {
    include!(env!("LINKED_HOST_BINDINGS"));
}
use neocogi_com::guid_from_qualified_name;

/// Loads the two generated providers and checks nested access to the linked host.
fn main() {
    let mut paths = std::env::args_os().skip(1);
    let b = paths.next().expect("path to provider B");
    let a = paths.next().expect("path to provider A");
    let runtime = neocogi_com_host::initialize_runtime();
    assert_eq!(runtime.plugins().unwrap().len(), 0);
    // SAFETY: trusted generated providers are mapped and pinned by the selected host.
    unsafe {
        runtime.load_plugin_library(&b);
        runtime.load_plugin_library(&a);
    }
    let root = runtime
        .acquire_plugin_root::<contracts::ICounterAbi>(guid_from_qualified_name("nc.tests.linked_host.a"))
        .unwrap();
    for _ in 0..32 {
        assert_eq!(root.value(), Ok(42));
    }
    assert_eq!(runtime.plugins().unwrap().len(), 2);
    let again = neocogi_com_host::initialize_runtime();
    assert_eq!(again.plugins().unwrap().len(), 2);
    assert_eq!(std::thread::spawn(move || again.plugins().unwrap().len()).join().unwrap(), 2);
    drop(root);
    println!("one host: nested value=42; plugins=2");
}
