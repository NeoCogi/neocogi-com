//! A worker scope cannot outlive the visit that issued a loan.

fn with_child<R>(visit: impl for<'loan> FnOnce(&'loan mut u32) -> R) -> R {
    visit(&mut 42)
}

fn start_in_outer_scope() {
    std::thread::scope(|outer| {
        with_child(|child| {
            outer.spawn(move || *child += 1).join().unwrap(); // ERROR E0521
        });
    });
}
