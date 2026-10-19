//! A message getter view borrows its event owner.
use neocogi_com::{IUnknown, events::EventMessage};
fn inspect(message: EventMessage) {
    let projection = message.query::<IUnknown>().unwrap().unwrap();
    drop(message);
    let _raw = projection.as_raw();
}
fn main() {}
