#[neocogi_com::com_enum(tag = u8, assigned_signature = b"00000000000000000000000000000045")]
enum Invalid {
    #[com_tag(7)]
    Empty,
    Value(u32),
}

fn main() {}
