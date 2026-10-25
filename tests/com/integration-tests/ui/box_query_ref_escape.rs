//! A borrowed projection cannot survive movement of the owner's sole token.
use neocogi_com::{ComBox, IUnknown};
fn inspect(owner: ComBox<IUnknown>) {
    let projection = owner.query_ref::<IUnknown>().unwrap();
    drop(owner);
    let _raw = projection.as_raw();
}
fn main() {}
