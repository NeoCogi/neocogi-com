//! Real Windows loading, dependency search, UTF-16 paths and adapter failures.
#![cfg(windows)]

#[path = "support/windows_loader_policy.rs"]
mod policy;

/// One fixture build, then isolated sequential processes for each search layout.
#[test]
fn windows_native_paths_and_dependency_search_are_explicit() {
    policy::exercise();
}

/// The parent admits this exact test and checks case entry and completion.
#[test]
fn windows_loader_child() {
    policy::child();
}
