use nc_com::com_class;

#[com_class(
    class_id(auto_guid = "nc.tests.ui.EmptyVersion:class:v1"),
    name = "nc.tests.ui.EmptyVersion:class:v1",
    version = "",
    constructor(args = none),
    interfaces()
)]
struct EmptyVersion;

#[com_class(
    class_id(auto_guid = "nc.tests.ui.WhitespaceVersion:class:v1"),
    name = "nc.tests.ui.WhitespaceVersion:class:v1",
    version = "1.0 beta",
    constructor(args = none),
    interfaces()
)]
struct WhitespaceVersion;

fn main() {}
