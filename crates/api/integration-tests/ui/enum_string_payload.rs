use neocogi_com::com_enum;

#[com_enum(tag = u32)]
pub enum BadPayload {
    Text(String),
}

fn main() {}
