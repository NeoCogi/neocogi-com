#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(env!("NC_COM_TEST_CONTRACTS")); }
extern crate self as fixture_ui_interface_noncolliding_generated_names;
test_contracts::ui_interface_noncolliding_generated_names!();

fn main() {
    let _value = INoncollidingGeneratedNamesClient { value: 7 };
}

/// Public marker association and named wrapper resolve to the same borrowed method type.
fn wrapper(owner: &neocogi_com_sdk::ComPtr<INoncollidingGeneratedNamesAbi>) -> &INoncollidingGeneratedNamesWrapper {
    let associated: &<INoncollidingGeneratedNamesAbi as neocogi_com_sdk::ComRustInterface>::Wrapper<neocogi_com_sdk::ThreadLocal> = owner;
    associated
}

/// Foreign generated bindings can name the opaque Rust pointer through the SDK export.
fn raw<T: neocogi_com_sdk::ComInterface, M: neocogi_com_sdk::ThreadingModel>(pointer: &neocogi_com_sdk::ComInterfacePointer<T, M>) -> neocogi_com_sdk::ComRaw<T> {
    pointer.as_raw()
}
