//! Sending an exclusive loan to a Cell does not imply its shared loans can move.

use std::cell::Cell;

fn share() {
    let owner = Box::new(Cell::new(42));
    let shared = &*owner;
    std::thread::scope(|scope| {
        scope.spawn(move || shared.get()); // ERROR E0277
    });
}
