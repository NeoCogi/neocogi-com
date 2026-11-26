//! A temporary field guard cannot back a receiver-bound return.

use std::cell::RefCell;

fn child(field: &RefCell<Box<u32>>) -> &u32 {
    &*field.borrow() // ERROR E0515
}
