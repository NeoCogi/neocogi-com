use neocogi_com::com_object;

#[com_object(interfaces())]
struct Model { value: u32 }

fn main() {
    let owner = Model::into_local(Model { value: 1 });
    let model = owner.try_borrow().unwrap();
    std::thread::scope(|scope| {
        scope.spawn(move || println!("{}", model.value));
    });
}
