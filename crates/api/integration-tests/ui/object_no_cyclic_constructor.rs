use neocogi_com::{com_object, IUnknown};

/// Shared objects accept initialized values, with no initializer closure API.
#[com_object(interfaces())]
struct Shared;

/// Exclusive objects use the same ordinary-construction contract.
#[com_object(interfaces(), ownership = exclusive)]
struct Exclusive;

fn main() {
    let _ = Shared::from_value_cyclic::<IUnknown>(|_| Shared);
    let _ = Shared::into_local_cyclic::<IUnknown>(|_| Shared);
    let _ = Exclusive::into_box_cyclic::<IUnknown>(|_| Exclusive);
    let _ = Exclusive::into_local_box_cyclic::<IUnknown>(|_| Exclusive);
}
