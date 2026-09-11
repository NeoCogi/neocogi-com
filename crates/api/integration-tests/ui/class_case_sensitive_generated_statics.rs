#![allow(clippy::upper_case_acronyms)]

use neocogi_com::{NcStatus, com_class, com_object};

/// Lower-cased callable class identity fixture.
#[com_class(auto_guid = "nc.tests.ui.Foo:class:v1")]
trait Foo {
    /// Creates the lower-cased unit fixture.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;
}

/// Private storage for the lower-cased class identity.
#[com_object(constructible(Foo))]
struct FooObject;

impl Foo for FooObject {
    /// Creates the lower-cased unit object.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }
}

/// Upper-cased callable class identity fixture.
#[com_class(auto_guid = "nc.tests.ui.FOO:class:v1")]
trait FOO {
    /// Creates the upper-cased unit fixture.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;
}

/// Private storage for the upper-cased class identity.
#[com_object(constructible(FOO))]
struct FOOObject;

impl FOO for FOOObject {
    /// Creates the upper-cased unit object.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }
}

fn main() {
    // Case is part of the qualified name and must remain part of both IDs.
    assert_ne!(FooObject::CLASS_ID, FOOObject::CLASS_ID);
    assert_ne!(
        FooObject::class_descriptor().class_id,
        FOOObject::class_descriptor().class_id
    );
}
