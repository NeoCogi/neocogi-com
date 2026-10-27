//! Links the independently compiled helper selected by the Windows policy test.

/// The parent test owns this import library and keeps it alive during the build.
fn main() {
    println!("cargo:rerun-if-env-changed=NC_COM_POLICY_IMPORT_DIR");
    let directory = std::env::var("NC_COM_POLICY_IMPORT_DIR").expect("build through windows_plugin_paths integration test");
    println!("cargo:rustc-link-search=native={directory}");
    println!("cargo:rustc-link-lib=dylib=nc_policy_helper_83d97");
}
