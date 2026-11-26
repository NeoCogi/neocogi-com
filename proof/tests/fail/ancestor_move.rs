//! A receiver-bound child loan prevents destruction of the owning ancestor.

fn discard_ancestor() {
    let panel = Box::new(Some(Box::new(42)));
    let child = panel.as_deref().unwrap();
    drop(panel); // ERROR E0505
    assert_eq!(*child, 42);
}
