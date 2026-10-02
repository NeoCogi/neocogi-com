#[neocogi_com::com_enum(tag = u8, assigned_signature = b"00000000000000000000000000000045")]
enum Invalid {
    Empty = 0,
    Value(u32),
}

fn main() {}
