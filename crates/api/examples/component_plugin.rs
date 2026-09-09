use neocogi_com::{NcGuid, NcStatus, com_class, com_object};

const COMPONENT_PLUGIN_ID: NcGuid = neocogi_com::guid_from_qualified_name("nc.examples.component");

/// Dynamically linked class contract shared verbatim with its hosts.
#[com_class(auto_guid = "nc.examples.component.Greeter")]
pub trait Greeter {
    /// Creates a greeter from one caller-supplied seed.
    #[constructor]
    fn new(seed: u32) -> Result<Self, NcStatus>;

    /// Returns the greeting code derived during construction.
    fn greeting_code(&self) -> Result<u32, NcStatus>;
}

/// Private plugin state behind the public callable class projection.
#[com_object(Greeter, thread_safe)]
pub struct GreeterObject {
    /// Value returned by `greeting_code` after construction.
    greeting_code: u32,
}

impl Greeter for GreeterObject {
    /// Creates the private implementation from the decoded constructor frame.
    fn new(seed: u32) -> Result<Self, NcStatus> {
        Ok(Self { greeting_code: seed + 100 })
    }

    /// Reads the immutable greeting code.
    fn greeting_code(&self) -> Result<u32, NcStatus> {
        Ok(self.greeting_code)
    }
}

neocogi_com::plugin! {
    plugin_id(COMPONENT_PLUGIN_ID),
    name = "nc.examples.component",
    version = "1.0.0",
    classes = [GreeterObject],
}

// This crate is the final shared library, so it owns the sole dynamic entrypoint.
neocogi_com::export_plugin!();
