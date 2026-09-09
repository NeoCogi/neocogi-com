use neocogi_com::com_class;

#[com_class(auto_guid = "nc.tests.ui.Invalid Class:v1")]
trait InvalidClassName {
    #[constructor]
    fn new() -> Result<Self, neocogi_com::NcStatus>;
}

fn main() {}
