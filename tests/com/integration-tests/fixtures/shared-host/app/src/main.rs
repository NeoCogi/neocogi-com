//! A final application selects exactly one static or dynamic runtime route.
extern crate self as shared_host_app;
/// Bindings compiled independently from both native providers.
pub mod contracts {
    include!(env!("SHARED_HOST_BINDINGS"));
}
use neocogi_com::{Runtime, guid_from_qualified_name};

/// Exercises native bootstrap errors or the complete selected-host composition.
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args[1] == "error" {
        // SAFETY: test images grant the prefix contract appropriate to their status.
        let error = unsafe { Runtime::load_host_library(&args[2]) }.unwrap_err();
        assert_eq!(error.kind().to_string(), args[3].to_str().unwrap());
        assert_eq!(error.path(), std::path::Path::new(&args[2]));
        match error.kind() {
            neocogi_com::HostLoadErrorKind::Entrypoint => assert_eq!(error.status(), Some(neocogi_com::NC_E_NOT_FOUND)),
            neocogi_com::HostLoadErrorKind::InvalidApi => {
                let expected = if std::env::var("NC_TEST_HOST_FAILURE").as_deref() == Ok("null") {
                    neocogi_com::NC_E_POINTER
                } else {
                    neocogi_com::NC_E_ABI_VERSION
                };
                assert_eq!(error.status(), Some(expected));
            }
            _ => assert_eq!(error.status(), None),
        }
        println!("host error: {}", error.kind());
        return;
    }
    #[cfg(feature = "static-host")]
    let runtime = neocogi_com_host::initialize_runtime();
    #[cfg(not(feature = "static-host"))]
    // SAFETY: the fixture deploys the actual SDK host shim, retained for process lifetime.
    let runtime = unsafe { Runtime::load_host_library(&args[2]) }.unwrap();
    assert_eq!(runtime.plugins().unwrap().len(), 0);
    // SAFETY: trusted generated providers are mapped and pinned by the selected host.
    unsafe {
        runtime.load_plugin_library(&args[3]);
        runtime.load_plugin_library(&args[4]);
    }
    let root = runtime
        .acquire_plugin_root::<contracts::ICounterAbi>(guid_from_qualified_name("nc.tests.shared_host.a"))
        .unwrap();
    for _ in 0..32 {
        assert_eq!(root.value(), Ok(42));
    }
    assert_eq!(runtime.plugins().unwrap().len(), 2);
    #[cfg(feature = "static-host")]
    let again = neocogi_com_host::initialize_runtime();
    #[cfg(not(feature = "static-host"))]
    // SAFETY: reopening the same pinned runtime image retains that image's one owner.
    let again = unsafe { Runtime::load_host_library(&args[2]) }.unwrap();
    assert_eq!(again.plugins().unwrap().len(), 2);
    assert_eq!(std::thread::spawn(move || again.plugins().unwrap().len()).join().unwrap(), 2);
    drop(root);
    println!("one host: nested value=42; plugins=2");
}
