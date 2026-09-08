use nc_com::{NcStatus, com_class, com_object};

/// Registered fixture used to reject implementation generics at the boundary.
#[com_class(auto_guid = "test.GenericDescriptor:class:v1")]
trait GenericDescriptor {
    /// Creates the fixture without constructor arguments.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;
}

/// Generic storage cannot be selected through a non-generic registry entry.
#[com_object(GenericDescriptor)]
struct Reader<T> {
    /// Arbitrary specialization that a dynamic host could not name.
    value: T,
}

fn main() {}
