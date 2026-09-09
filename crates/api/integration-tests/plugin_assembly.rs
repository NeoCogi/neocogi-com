//! Multiple plugin implementations can share one binary without duplicate C
//! symbols; only the assembly root decides whether to publish a dynamic export.

/// First independent descriptor; its items remain scoped to this module.
mod first {
    neocogi_com::plugin! {
        plugin_id(auto_guid = "nc.tests.assembly.first:v1"),
        name = "nc.tests.assembly.first:v1",
        version = "0.1.0",
        classes = [],
    }
}

/// Second independent descriptor, with no conflicting exported symbol.
mod second {
    neocogi_com::plugin! {
        plugin_id(auto_guid = "nc.tests.assembly.second:v1"),
        name = "nc.tests.assembly.second:v1",
        version = "0.1.0",
        classes = [],
    }
}

// A dynamic assembly may explicitly select one descriptor function to export.
neocogi_com::export_plugin!(first::plugin_descriptor);

#[test]
fn multiple_descriptors_link_and_the_explicit_export_reuses_its_metadata() {
    let first = first::plugin_descriptor();
    let second = second::plugin_descriptor();
    assert_ne!(first.plugin_id, second.plugin_id);
    let mut out = std::ptr::null();
    // SAFETY: Both metadata and writable output storage satisfy the entrypoint contract.
    assert_eq!(unsafe { nc_get_plugin_descriptor(&mut out) }, neocogi_com::NC_OK);
    assert!(std::ptr::eq(out, first));
    // SAFETY: Null output is the supported defensive error case.
    assert_eq!(unsafe { nc_get_plugin_descriptor(std::ptr::null_mut()) }, neocogi_com::NC_E_POINTER);
}

#[cfg(feature = "static-runtime")]
#[test]
fn static_plugins_publish_into_one_shared_runtime_catalog() {
    let runtime = neocogi_com::Runtime::global();
    assert_eq!(runtime.core_abi_version(), neocogi_com::NC_COM_CORE_ABI_VERSION);
    for descriptor in [first::plugin_descriptor(), second::plugin_descriptor()] {
        // SAFETY: Both plugin descriptors and all nested tables are immutable statics.
        assert_eq!(unsafe { runtime.register_plugin_descriptor(descriptor) }, neocogi_com::NC_OK);
    }
    // A separately acquired facade observes the same catalog, not a local mirror.
    let observer = neocogi_com::Runtime::global();
    let ids = observer.plugin_registry().plugin_ids().unwrap();
    assert!(ids.contains(&first::plugin_descriptor().plugin_id));
    assert!(ids.contains(&second::plugin_descriptor().plugin_id));
    assert_eq!(ids.len(), 2);
}
