use neocogi_com::com_enum;

#[com_enum(tag = u32, assigned_signature = b"77777777777777777777777777777777")]
pub enum InvalidTag {
    #[com_tag]
    Value,
}

fn main() {}
