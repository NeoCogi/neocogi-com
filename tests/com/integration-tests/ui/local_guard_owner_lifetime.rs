use neocogi_com_sdk::com_object;

#[com_object(interfaces())]
struct Model {
    value: u32,
}

fn main() {
    let owner = Model::into_local(Model { value: 1 });
    let model = owner.get();
    drop(owner);
    println!("{}", model.value);
}
