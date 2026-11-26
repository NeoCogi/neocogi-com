use neocogi_com_sdk::com_object;

#[com_object(interfaces())]
struct Model;

fn main() {
    let owner = Model::into_local(Model);
    let root = owner.query::<neocogi_com_sdk::IUnknown>().unwrap();
    let model = root.as_com_ref();
    std::thread::scope(|scope| {
        scope.spawn(move || {
            let _ = model.as_raw();
        });
    });
}
