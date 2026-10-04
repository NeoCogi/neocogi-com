use neocogi_com::{com_interface, com_object, NcStatus, WeakComPtr, WeakLocalCom};

#[com_interface(auto_guid = "nc.tests.weak_local.Proof:v1", assigned_signature = b"00000000000000000000000000000064")]
trait IValue { fn value(&self) -> Result<u32, NcStatus>; }
#[com_object(IValue)]
struct Value;
impl IValue for Value { fn value(&self) -> Result<u32, NcStatus> { Ok(1) } }

fn cannot_recover_concrete_type(foreign: WeakComPtr<IValueAbi>) -> WeakLocalCom<Value> {
    foreign.into()
}
fn main() {}
