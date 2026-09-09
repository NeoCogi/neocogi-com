use neocogi_com::com_enum;

#[com_enum(tag = u32)]
pub enum InvalidTag {
    #[com_tag]
    Value,
}

fn main() {}
