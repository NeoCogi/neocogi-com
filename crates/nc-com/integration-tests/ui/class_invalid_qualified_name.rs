use nc_com::com_object;

#[com_object(
    class_id(auto_guid = "nc.tests.ui.Invalid Class:v1"),
    name = "nc.tests.ui.Invalid Class:v1",
    version = "1.0.0",
    constructor(args = none),
    interfaces()
)]
struct InvalidClassName;

fn main() {}
