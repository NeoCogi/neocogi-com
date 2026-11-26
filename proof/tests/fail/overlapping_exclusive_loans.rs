//! Two workers in one scope cannot hold separate exclusive loans to one target.

fn edit_twice(value: &mut u32) {
    std::thread::scope(|scope| {
        scope.spawn(|| *value += 1);
        scope.spawn(|| *value += 1); // ERROR E0499
    });
}
