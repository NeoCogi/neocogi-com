use neocogi_com::{com_object, NcStatus, WeakComPtr, WeakLocalCom};

extern crate self as fixture_ui_weak_local_construction_proof;
neocogi_com_test_contracts::ui_weak_local_construction_proof!();
#[com_object(IValue)]
struct Value;
impl IValue for Value {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }
}

fn cannot_recover_concrete_type(foreign: WeakComPtr<IValueAbi>) -> WeakLocalCom<Value> {
    foreign.into()
}
fn main() {}
