use neocogi_com_sdk::NcValueRef;

fn main() {
    let guard;
    {
        let source = vec![13_u32; 8];
        guard = NcValueRef::new(&source).unwrap();
    }
    let _owned = guard.to_owned();
}
