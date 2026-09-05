//! Multiple plugin implementations can share one binary without duplicate C
//! symbols; only the assembly root decides whether to publish a dynamic export.

/// First independent descriptor; its items remain scoped to this module.
mod first {
    nc_com::plugin! {
        plugin_id(auto_guid = "nc.tests.assembly.first:v1"),
        name = "nc.tests.assembly.first:v1",
        version = "0.1.0",
        classes = [],
    }
}

/// Second independent descriptor, with no conflicting exported symbol.
mod second {
    nc_com::plugin! {
        plugin_id(auto_guid = "nc.tests.assembly.second:v1"),
        name = "nc.tests.assembly.second:v1",
        version = "0.1.0",
        classes = [],
    }
}

// A dynamic assembly may explicitly select one descriptor function to export.
nc_com::export_plugin!(first::plugin_descriptor);

#[test]
fn multiple_descriptors_link_and_the_explicit_export_reuses_its_metadata() {
    let first = first::plugin_descriptor();
    let second = second::plugin_descriptor();
    assert_ne!(first.plugin_id, second.plugin_id);
    let mut out = std::ptr::null();
    // SAFETY: Both metadata and writable output storage satisfy the entrypoint contract.
    assert_eq!(unsafe { nc_get_plugin_descriptor(&mut out) }, nc_com::NC_OK);
    assert!(std::ptr::eq(out, first));
    // SAFETY: Null output is the supported defensive error case.
    assert_eq!(unsafe { nc_get_plugin_descriptor(std::ptr::null_mut()) }, nc_com::NC_E_POINTER);
}
