#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(env!("NC_COM_TEST_CONTRACTS")); }
extern crate self as fixture_ui_interface_noncolliding_generated_names;
test_contracts::ui_interface_noncolliding_generated_names!();

fn main() {
    let _value = INoncollidingGeneratedNamesClient { value: 7 };
}

/// Public marker association and named wrapper resolve to the same borrowed method type.
fn wrapper(owner: &neocogi_com::ComPtr<INoncollidingGeneratedNamesAbi>) -> &INoncollidingGeneratedNamesWrapper {
    let associated: &<INoncollidingGeneratedNamesAbi as neocogi_com::ComRustInterface>::Wrapper<neocogi_com::ThreadLocal> = owner;
    associated
}

/// Foreign generated bindings can name the opaque Rust pointer through the SDK export.
fn raw<T: neocogi_com::ComInterface, M: neocogi_com::ThreadingModel>(pointer: &neocogi_com::ComInterfacePointer<T, M>) -> neocogi_com::ComRaw<T> {
    pointer.as_raw()
}
