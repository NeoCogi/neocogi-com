#![allow(improper_ctypes_definitions)]

use neocogi_com::com_struct;

#[derive(Clone, Copy)]
pub struct RustLayout {
    pub x: f32,
    pub y: f32,
}

#[com_struct(c_struct, assigned_signature = b"00000000000000000000000000000048")]
pub struct Contract {
    pub value: RustLayout,
}

fn main() {}
