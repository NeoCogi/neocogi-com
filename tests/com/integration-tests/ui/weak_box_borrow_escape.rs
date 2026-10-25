use neocogi_com::com_object;

#[com_object(interfaces(), ownership = exclusive)]
struct Model { value: u32 }

fn main() {
    let owner = Model::into_local_box(Model { value: 1 });
    let mut weak = owner.downgrade();
    let escaped = weak.with_mut(|value| &mut value.value).unwrap();
    drop(owner);
    *escaped = 2;
}
