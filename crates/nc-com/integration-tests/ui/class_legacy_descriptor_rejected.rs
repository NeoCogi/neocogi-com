use nc_com::com_object;

/// Obsolete implementation-owned descriptor syntax must not remain accepted.
#[com_object(
    class_id(auto_guid = "nc.tests.ui.LegacyDescriptor:v1"),
    name = "nc.tests.ui.LegacyDescriptor:v1",
    version = "1.0.0",
    constructor(args = none),
    interfaces()
)]
struct LegacyDescriptor;

/// Compile-fail fixtures require an otherwise ordinary binary entrypoint.
fn main() {}
