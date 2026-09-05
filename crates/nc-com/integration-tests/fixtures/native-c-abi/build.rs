fn main() {
    println!("cargo:rerun-if-changed=native/caller.c");

    cc::Build::new()
        .file("native/caller.c")
        .std("c11")
        .warnings(true)
        .compile("native_c_abi_fixture");
}
