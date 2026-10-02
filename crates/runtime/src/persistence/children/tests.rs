//! Counter exhaustion and final destruction through real persistence-list interfaces.

use super::*;
use core::sync::atomic::Ordering;

#[cfg(all(unix, not(miri)))]
#[path = "../../../../test-support/retain_abort.rs"]
mod retain_abort;

/// Both list owner-producing operations inherit the helper's unconditional C failure.
#[cfg(all(unix, not(miri)))]
#[test]
fn retain_exhaustion_aborts_in_real_children_entrypoints() {
    for case in ["clone", "query"] {
        retain_abort::assert_aborts(
            "persistence::children::tests::retain_exhaustion_aborts_in_real_children_entrypoints",
            case,
            || {
                let list = create(Vec::new()).unwrap();
                // SAFETY: a newly allocated list is privately owned by this subprocess.
                // Only its count is injected; no object layout or pointer is fabricated.
                unsafe { (*list.as_raw().object.cast::<Children>()).references.store(u32::MAX, Ordering::Relaxed) };
                match case {
                    "clone" => drop(list.clone()),
                    "query" => drop(list.query::<IUnknown>().unwrap()),
                    _ => unreachable!("closed set of list operations"),
                }
            },
        );
    }
}

/// Releasing the last list projection releases its child owners exactly once.
#[test]
fn final_list_release_drops_descendants_after_all_projections() {
    let child = create(Vec::new()).unwrap();
    let parent = create(vec![child.query::<IUnknown>().unwrap()]).unwrap();
    let clone = parent.clone();
    let projection = parent.query::<IUnknown>().unwrap();
    // SAFETY: the external child owner keeps this counter alive through parent Drop.
    let child_count = unsafe { &(*child.as_raw().object.cast::<Children>()).references };
    assert_eq!(child_count.load(Ordering::Relaxed), 2);
    drop(parent);
    drop(clone);
    assert_eq!(child_count.load(Ordering::Relaxed), 2);
    drop(projection);
    assert_eq!(child_count.load(Ordering::Relaxed), 1);
    drop(child);
}

/// A query at MAX-1 must record the new projection before publishing it.
#[test]
fn children_query_counts_the_last_owner() {
    let list = create(Vec::new()).unwrap();
    // SAFETY: list owns initialized private storage; no other thread can access it.
    let count = unsafe { &(*list.as_raw().object.cast::<Children>()).references };
    count.store(u32::MAX - 1, Ordering::Relaxed);
    let projection = list.query::<IUnknown>().unwrap();
    assert_eq!(count.load(Ordering::Relaxed), u32::MAX);
    drop(projection);
    assert_eq!(count.load(Ordering::Relaxed), u32::MAX - 1);
    // Remove injected owners before the real owner follows ordinary final Drop.
    count.store(1, Ordering::Relaxed);
    drop(list);
}
