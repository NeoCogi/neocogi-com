use neocogi_com::com_enum;

#[com_enum(tag = u32)]
enum DuplicateTag {
    First,
    #[com_tag(1)]
    Second,
}

fn main() {}
