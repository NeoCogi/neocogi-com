use nc_com::{NcStatus, com_class, com_object};

/// Callable non-unit class whose construction policy is part of its contract.
#[com_class(auto_guid = "nc.tests.ui.NonUnitTypedConstructor:v1")]
pub trait NonUnitTypedConstructor {
    /// Creates an object holding the caller's value.
    #[constructor]
    fn new(value: u32) -> Result<Self, NcStatus>;

    /// Returns the value accepted by the constructor.
    fn value(&self) -> Result<u32, NcStatus>;
}

/// Private non-unit storage initialized by the typed constructor.
#[com_object(NonUnitTypedConstructor)]
pub struct NonUnitTypedConstructorObject {
    /// Value copied from the checked constructor frame.
    value: u32,
}

impl NonUnitTypedConstructor for NonUnitTypedConstructorObject {
    /// Copies the checked argument into owned constructor-owned storage.
    fn new(value: u32) -> Result<Self, NcStatus> {
        Ok(Self { value })
    }

    /// Returns the stored constructor value.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

fn main() {}
