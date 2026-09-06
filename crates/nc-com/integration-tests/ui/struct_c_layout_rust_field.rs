#![allow(improper_ctypes_definitions)]

use nc_com::com_struct;

#[derive(Clone, Copy)]
pub struct RustLayout {
    pub x: f32,
    pub y: f32,
}

#[com_struct(c_struct)]
pub struct Contract {
    pub value: RustLayout,
}

fn main() {}
