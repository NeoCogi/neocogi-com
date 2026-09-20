//! Concrete COM mappings into standard Rust channels.
//! Only generated typed receivers cross plugin boundaries.
//! Mapping is immediate; consumer behavior is deferred to runtime dispatch.

#![deny(missing_docs)]

mod named;
pub use named::{NamedOutputs, bind_output};
mod mapping;
pub use mapping::map;
mod component;
pub use component::component_dispatcher;
mod scoped;
pub use scoped::with_dispatcher;

pub use neocogi_com_events_contracts::{INamedOutputs, INamedOutputsAbi, IEventDispatcher, IEventDispatcherAbi, EVENT_E_LIMIT};

/// A scoped dispatcher was retained beyond its borrowed callback lifetime.
pub const EVENT_E_CLOSED: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1000);
/// Shared component access would reenter an active mutable operation.
pub const EVENT_E_BUSY: neocogi_com::NcStatus = neocogi_com::NcStatus::from_code(-1001);
