use neocogi_com::{com_object, NcStatus};

extern crate self as fixture_ui_class_generic_borrowed;
neocogi_com_test_contracts::ui_class_generic_borrowed!();

#[com_object(interfaces(IRead))]
struct Reader<T: AsRef<str>> {
    value: T,
}
impl<T: AsRef<str>> IRead for Reader<T> {
    fn length(&self) -> Result<u32, NcStatus> {
        Ok(self.value.as_ref().len() as u32)
    }
}

fn main() {
    let value = String::from("stack-owned text");
    let _retained = Reader::from_value(Reader { value: value.as_str() });
}
