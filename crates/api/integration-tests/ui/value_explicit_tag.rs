#[neocogi_com::com_value( assigned_signature = b"00000000000000000000000000000045")]
enum Invalid {
    Empty,
    #[com_tag(1)]
    Value(String),
}

fn main() {}
