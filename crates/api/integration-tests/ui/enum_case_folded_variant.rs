use neocogi_com::com_enum;

#[com_enum(tag = u8, assigned_signature = b"77777777777777777777777777777777")]
enum CaseFoldedVariant {
    Foo(u8),
    FOO(u16),
    _EMPTY(u32),
}

fn main() {}
