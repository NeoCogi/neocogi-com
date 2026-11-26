//! A visitor cannot place its call-scoped loan in its return value.

fn with_child<R>(visit: impl for<'loan> FnOnce(&'loan mut u32) -> R) -> R {
    visit(&mut 42)
}

fn escape() -> &'static mut u32 {
    with_child(|child| child) // ERROR lifetime may not live long enough
}
