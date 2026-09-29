use neocogi_com::com_interface;

/// Omitting the lifetime must not silently produce a call-scoped declaration.
#[com_interface(auto_guid = "nc.tests.ui.ValueRef.MissingLifetime:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IMissingLifetime {
    /// The old parser accepted a lone type after discarding non-type arguments.
    fn receive(&self, value: neocogi_com::NcValueRef<u32>) -> Result<(), neocogi_com::NcStatus>;
}

/// No generic arguments must report the wrapper's expected shape.
#[com_interface(auto_guid = "nc.tests.ui.ValueRef.NoArguments:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait INoArguments {
    /// A bare wrapper is not a plain copied ABI type.
    fn receive(&self, value: neocogi_com::NcValueRef) -> Result<(), neocogi_com::NcStatus>;
}

/// Extra lifetime arguments must not be dropped while collecting types.
#[com_interface(auto_guid = "nc.tests.ui.ValueRef.ExtraLifetime:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IExtraLifetime {
    /// This still has exactly one type, but is not the permitted wrapper shape.
    fn receive(&self, value: neocogi_com::NcValueRef<'_, 'static, u32>) -> Result<(), neocogi_com::NcStatus>;
}

/// The value argument must be a type rather than a const argument.
#[com_interface(auto_guid = "nc.tests.ui.ValueRef.ConstElement:v1", assigned_signature = b"99999999999999999999999999999999")]
pub trait IConstElement {
    /// A number cannot select a ComValue implementation.
    fn receive(&self, value: neocogi_com::NcValueRef<'_, 3>) -> Result<(), neocogi_com::NcStatus>;
}

/// Each declaration is independently rejected during macro expansion.
fn main() {}
