use nc_com::{com_class, com_interface, NcStatus};

#[com_interface(auto_guid = "test.GenericBorrow:v1")]
trait IRead {
    fn length(&self) -> Result<u32, NcStatus>;
}

#[com_class(interfaces(IRead))]
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
