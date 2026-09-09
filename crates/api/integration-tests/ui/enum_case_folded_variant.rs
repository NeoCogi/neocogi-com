use neocogi_com::com_enum;

#[com_enum(tag = u8)]
enum CaseFoldedVariant {
    Foo(u8),
    FOO(u16),
    _EMPTY(u32),
}

fn main() {}
