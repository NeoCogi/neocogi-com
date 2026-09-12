use neocogi_com::com_interface;

#[com_interface(auto_guid = "nc.tests.ui.PatternArg:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IPatternArg {
    fn value(&self, mut value: u32) -> Result<u32, neocogi_com::NcStatus>;
}

fn main() {}
