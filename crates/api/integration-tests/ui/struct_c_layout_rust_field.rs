#![allow(improper_ctypes_definitions)]

use neocogi_com::com_struct;

#[derive(Clone, Copy)]
pub struct RustLayout {
    pub x: f32,
    pub y: f32,
}

// Explicit conversion is valid even for an application-only Rust layout. The
// enclosing c_struct must still reject that layout in its separate C-layout proof.
unsafe impl neocogi_com::ComValue for RustLayout {
    type Wire = [f32; 2];
    type Storage = ();
    fn encode_value(&self, _: &mut neocogi_com::ValueBudget) -> Result<(Self::Wire, ()), neocogi_com::NcStatus> {
        Ok(([self.x, self.y], ()))
    }
    unsafe fn decode_value(wire: Self::Wire, _: &mut neocogi_com::ValueBudget) -> Result<Self, neocogi_com::NcStatus> {
        Ok(Self { x: wire[0], y: wire[1] })
    }
}
unsafe impl neocogi_com::ComInlineValue for RustLayout {}

#[com_struct(c_struct, assigned_signature = b"00000000000000000000000000000048")]
pub struct Contract {
    pub value: RustLayout,
}

fn main() {}
