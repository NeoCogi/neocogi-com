//! Selecting a new interface cannot grant cross-thread access to a local owner.
use neocogi_com::{ComBox, IUnknown};
fn require_send<T: Send>(_: T) {}
fn inspect(owner: ComBox<IUnknown>) {
    require_send(owner.query_ref::<IUnknown>().unwrap());
}
fn main() {}
