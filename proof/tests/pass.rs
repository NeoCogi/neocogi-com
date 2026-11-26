//! Rust reference behavior for the proposed boxed ownership and loan contract.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Mutex,
    thread,
};

struct Panel {
    child: Option<Box<u32>>,
}

impl Panel {
    fn child(&self) -> Option<&u32> {
        self.child.as_deref()
    }

    fn child_mut(&mut self) -> Option<&mut u32> {
        self.child.as_deref_mut()
    }

    fn child_read_from_mutable_source(&mut self) -> Option<&u32> {
        self.child.as_deref()
    }
}

fn receiver_bound_child() {
    let mut panel = Box::new(Panel { child: Some(Box::new(3)) });
    let child = panel.child_mut().unwrap();
    thread::scope(|scope| {
        // The scope waits even when its join handle is discarded.
        let worker_loan = &mut *child;
        scope.spawn(move || *worker_loan += 1);
    });
    *child += 1;
    assert_eq!(panel.child(), Some(&5));

    let child = panel.child_read_from_mutable_source().unwrap();
    thread::scope(|scope| {
        let first = scope.spawn(move || *child);
        let second = scope.spawn(move || *child);
        assert_eq!(first.join().unwrap(), 5);
        assert_eq!(second.join().unwrap(), 5);
    });
    assert_eq!(panel.child(), Some(&5));
}

struct GuardedPanel {
    // This provider stays on its original thread when it lends its child.
    _thread_local: Rc<()>,
    child: RefCell<Box<u32>>,
}

impl GuardedPanel {
    fn with_child<R>(&self, visit: impl for<'loan> FnOnce(&'loan mut u32) -> R) -> R {
        let mut child = self.child.borrow_mut();
        visit(&mut child)
    }
}

fn visitor_holds_field_guard() {
    let panel = GuardedPanel {
        _thread_local: Rc::new(()),
        child: RefCell::new(Box::new(4)),
    };
    panel.with_child(|child| {
        // Reentry through a separate shared reference to the parent still
        // obeys the field's own replacement rules.
        assert!(panel.child.try_borrow_mut().is_err());
        thread::scope(|scope| {
            scope.spawn(move || *child += 1);
        });
    });
    let mut child = panel.child.try_borrow_mut().unwrap();
    assert_eq!(**child, 5);
    *child = Box::new(6);
}

fn transfer_requires_send() {
    let mut owner = Box::new(Cell::new(42));
    let exclusive = &mut *owner;
    thread::scope(|scope| {
        scope.spawn(move || exclusive.set(43));
    });
    assert_eq!(owner.get(), 43);

    // Cell is Send and not Sync. Moving the whole owner allows detached work.
    assert_eq!(thread::spawn(move || owner.into_inner()).join().unwrap(), 43);
}

fn sharing_requires_sync() {
    let mutex = Mutex::new(6);
    let owner = Box::new(mutex.lock().unwrap());
    // MutexGuard is Sync and not Send. The guard remains local for destruction.
    let shared = &*owner;
    thread::scope(|scope| {
        assert_eq!(scope.spawn(move || **shared).join().unwrap(), 6);
    });
    drop(owner);
    assert_eq!(*mutex.try_lock().unwrap(), 6);
}

fn main() {
    receiver_bound_child();
    visitor_holds_field_guard();
    transfer_requires_send();
    sharing_requires_sync();
}
