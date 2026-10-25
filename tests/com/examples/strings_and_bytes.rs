use neocogi_com::{ComPtr, ComRef, INcBytesAbi, INcBytesComPtrExt, INcStringComPtrExt, IUnknown, NcBytes, NcStatus, NcString, com_object};

const NC_E_EXAMPLE_INVALID_ARG_CODE: i32 = -101;
const NC_E_EXAMPLE_INVALID_ARG: NcStatus = NcStatus::from_code(NC_E_EXAMPLE_INVALID_ARG_CODE);

// Contract identity and wire codecs are compiled from examples/schemas/nc/examples/strings_and_bytes.ncidl.
#[allow(dead_code, unused_imports)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/example_strings_and_bytes.rs"));
}
use bindings::*;

/// A local stream whose generated mutable receiver admits one cursor update at a time.
#[com_object(interfaces(IByteStream))]
pub struct MemoryByteStream {
    /// Immutable source copied into independently owned byte chunks.
    bytes: Vec<u8>,
    /// Next unread byte, always at most bytes.len(); equality is terminal EOF.
    offset: usize,
}

impl MemoryByteStream {
    /// Owns the complete source and returns its checked stream projection.
    fn create_stream(bytes: Vec<u8>) -> ComPtr<IByteStreamAbi> {
        let unknown: ComPtr<IUnknown> = MemoryByteStream::from_value(Self { bytes, offset: 0 });
        unknown.query::<IByteStreamAbi>().expect("MemoryByteStream exposes IByteStream")
    }
}

impl IByteStream for MemoryByteStream {
    /// Copies an admitted prefix and advances only while the writer gate is held.
    fn next_chunk(&mut self, max_len: u64) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        // Zero remains invalid at EOF and must never advance the cursor.
        if max_len == 0 {
            return Err(NC_E_EXAMPLE_INVALID_ARG);
        }

        let start = self.offset;
        if start >= self.bytes.len() {
            return Ok(NcBytes::from_vec(Vec::new()));
        }

        let requested = if max_len > usize::MAX as u64 { usize::MAX } else { max_len as usize };
        let end = start + requested.min(self.bytes.len() - start);
        self.offset = end;

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

/// Runs the real checked calls, including silent boundaries reused by archive qualification.
fn main() -> Result<(), NcStatus> {
    let display_name = NcString::from_utf8_str("neocogi-com");
    println!("display name: {}", display_name.to_string()?);
    let mut prefix = [0; 2];
    display_name.copy_to(0, &mut prefix)?;
    assert_eq!(&prefix, b"ne");

    let counter_unknown: ComPtr<IUnknown> = ByteCounter::from_value(ByteCounter);
    let counter = counter_unknown.query::<IByteConsumerAbi>()?;
    let mut stream = MemoryByteStream::create_stream(b"chunked stream over COM".to_vec());
    let mut peer = stream.clone();
    assert!(matches!(stream.next_chunk(0), Err(status) if status == NC_E_EXAMPLE_INVALID_ARG));
    let mut expected = [b"chunked ".as_slice(), b"stream o", b"ver COM"].into_iter();

    loop {
        let chunk = stream.next_chunk(8)?;
        let len = counter.consume(chunk.as_com_ref())?;
        if len == 0 {
            break;
        }

        let bytes = chunk.to_vec()?;
        assert_eq!(bytes.as_slice(), expected.next().expect("exactly three chunks"));
        let text = String::from_utf8(bytes).expect("example data is UTF-8");
        println!("chunk len={len}: {text:?}");
    }
    assert!(expected.next().is_none());
    assert!(stream.next_chunk(8)?.to_vec()?.is_empty());
    assert!(peer.next_chunk(8)?.to_vec()?.is_empty());
    assert!(matches!(stream.next_chunk(0), Err(status) if status == NC_E_EXAMPLE_INVALID_ARG));

    let mut empty = MemoryByteStream::create_stream(Vec::new());
    assert!(empty.next_chunk(8)?.to_vec()?.is_empty());
    let mut short = MemoryByteStream::create_stream(b"tail".to_vec());
    let remainder = short.next_chunk(u64::MAX)?;
    assert!(short.next_chunk(1)?.to_vec()?.is_empty());
    drop(short);
    assert_eq!(remainder.to_vec()?, b"tail");

    Ok(())
}
