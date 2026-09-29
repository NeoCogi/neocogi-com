use neocogi_com::com_enum;

/// Macro syntax cannot evaluate this expression; generated const checks must.
const TAG: i64 = 7;
/// A second constant resolves to the same discriminant.
const ALIAS: i64 = 3 + 4;

#[com_enum(tag = i64, assigned_signature = b"77777777777777777777777777777777")]
enum DuplicateTag {
    #[com_tag(TAG)]
    First(u32),
    #[com_tag(ALIAS)]
    Second(bool),
}

fn main() {}
