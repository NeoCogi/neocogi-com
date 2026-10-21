use neocogi_com::{ComPtr, ComRef, INcBytesAbi, INcBytesComPtrExt, INcStringComPtrExt, IUnknown, NcBytes, NcStatus, NcString, com_object};
use std::cell::Cell;

const NC_E_EXAMPLE_INVALID_ARG_CODE: i32 = -101;
const NC_E_EXAMPLE_INVALID_ARG: NcStatus = NcStatus::from_code(NC_E_EXAMPLE_INVALID_ARG_CODE);

// Contract identity and wire codecs are compiled from examples/idl/nc/examples/strings_and_bytes.ncidl.
#[allow(dead_code, unused_imports)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/example_strings_and_bytes.rs"));
}
use bindings::*;

#[com_object(interfaces(IByteStream))]
pub struct MemoryByteStream {
    bytes: Vec<u8>,
    offset: Cell<usize>,
}

impl MemoryByteStream {
    fn create_stream(bytes: Vec<u8>) -> ComPtr<IByteStreamAbi> {
        let unknown: ComPtr<IUnknown> = MemoryByteStream::from_value(Self { bytes, offset: Cell::new(0) });
        unknown.query::<IByteStreamAbi>().expect("MemoryByteStream exposes IByteStream")
    }
}

impl IByteStream for MemoryByteStream {
    fn next_chunk(&self, max_len: u64) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        if max_len == 0 {
            return Err(NC_E_EXAMPLE_INVALID_ARG);
        }

        let start = self.offset.get();
        if start >= self.bytes.len() {
            return Ok(NcBytes::from_vec(Vec::new()));
        }

        let requested = if max_len > usize::MAX as u64 { usize::MAX } else { max_len as usize };
        let end = start + requested.min(self.bytes.len() - start);
        self.offset.set(end);

        Ok(NcBytes::from_slice(&self.bytes[start..end]))
    }
}

#[com_object(interfaces(IByteConsumer))]
pub struct ByteCounter;

impl IByteConsumer for ByteCounter {
    fn consume(&self, bytes: ComRef<'_, INcBytesAbi>) -> Result<u64, NcStatus> {
        bytes.len()
    }
}

fn main() -> Result<(), NcStatus> {
    let display_name = NcString::from_utf8_str("neocogi-com");
    println!("display name: {}", display_name.to_string()?);
    let mut prefix = [0; 2];
    display_name.copy_to(0, &mut prefix)?;
    assert_eq!(&prefix, b"ne");

    let counter_unknown: ComPtr<IUnknown> = ByteCounter::from_value(ByteCounter);
    let counter = counter_unknown.query::<IByteConsumerAbi>()?;
    let stream = MemoryByteStream::create_stream(b"chunked stream over COM".to_vec());

    loop {
        let chunk = stream.next_chunk(8)?;
        let len = counter.consume(chunk.as_com_ref())?;
        if len == 0 {
            break;
        }

        let text = String::from_utf8(chunk.to_vec()?).expect("example data is UTF-8");
        println!("chunk len={len}: {text:?}");
    }

    Ok(())
}
