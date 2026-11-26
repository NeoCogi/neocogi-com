//! A Sync-only target may be read on another thread while its owner stays local.

use std::sync::Mutex;

fn move_owner() {
    let mutex = Mutex::new(6);
    let owner = Box::new(mutex.lock().unwrap());
    std::thread::scope(|scope| {
        scope.spawn(move || drop(owner)); // ERROR E0277
    });
}
