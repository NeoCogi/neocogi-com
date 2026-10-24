//! Local bindings and helpers compiled against this consumer's exact Rust types.
include!(env!("IDL_EXAMPLE_BINDINGS"));
/// Domain-specific notification delivery.
#[path = "../../schemas/neocogi/idl_example/notifications.rs"]
pub mod notifications;
