use neocogi_com::com_object;

/// Shared service constructors cannot publish exclusive instances.
#[com_object(interfaces(), ownership = shared)]
struct Service;

/// Exclusive classes cannot publish retainable shared instances.
#[com_object(interfaces(), ownership = exclusive)]
struct Widget;

fn main() {
    let _ = Service::into_box(Service);
    let _ = Widget::from_value(Widget);
}
