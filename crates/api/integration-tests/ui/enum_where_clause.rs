use neocogi_com::com_enum;

#[com_enum(tag = u32)]
pub enum BadWhere
where
    u32: Copy,
{
    Value(u32),
}

fn main() {}
