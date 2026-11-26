//! A read-only output from a mutable receiver still borrows the source exclusively.

struct Panel(Box<u32>);

impl Panel {
    fn child(&mut self) -> &u32 {
        &self.0
    }

    fn inspect(&self) -> u32 {
        *self.0
    }
}

fn read_parent(mut panel: Panel) {
    let child = panel.child();
    let _ = panel.inspect(); // ERROR E0502
    assert_eq!(*child, 42);
}
