#![allow(clippy::upper_case_acronyms)]

use nc_com::com_object;

#[com_object(
    class_id(auto_guid = "nc.tests.ui.Foo:class:v1"),
    name = "nc.tests.ui.Foo:class:v1",
    version = "1.0.0",
    constructor(args = none),
    interfaces()
)]
struct Foo;

#[com_object(
    class_id(auto_guid = "nc.tests.ui.FOO:class:v1"),
    name = "nc.tests.ui.FOO:class:v1",
    version = "1.0.0",
    constructor(args = none),
    interfaces()
)]
struct FOO;

fn main() {
    assert_ne!(Foo::CLASS_ID, FOO::CLASS_ID);
    assert_ne!(
        Foo::class_descriptor().class_id,
        FOO::class_descriptor().class_id
    );
}
