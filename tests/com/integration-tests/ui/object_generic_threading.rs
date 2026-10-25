//! A generic shared provider must prove Send and Sync for its concrete specialization.
use neocogi_com::com_object;
use std::rc::Rc;

#[com_object(interfaces(), thread_safe)]
struct Generic<T> {
    state: T,
}

fn main() {
    let _owner = Generic::from_value(Generic { state: Rc::new(1u32) });
}
