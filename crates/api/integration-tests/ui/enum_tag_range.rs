use neocogi_com::com_enum;

/// A valid i64 wire integer can still be outside the declared source tag range.
#[com_enum(tag = u8, assigned_signature = b"77777777777777777777777777777777")]
enum OutOfRange {
    #[com_tag(256)]
    Selected(u32),
}

fn main() {}
