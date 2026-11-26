//! An immediate join does not shorten the static requirement of thread::spawn.

fn detach(child: &mut u32) {
    std::thread::spawn(move || *child += 1).join().unwrap(); // ERROR E0521
}
