use neocogi_com::com_enum;

#[com_enum(tag = u8)]
enum OpaqueValue {
    Empty,
    Number(u64),
}

fn main() {
    let _ = OpaqueValueAbi {
        tag: OpaqueValueTag::Number,
        payload: OpaqueValuePayload { _empty: () },
    };
}
