//! Independent NCB writer for the operations a saved tree actually emits.

use super::*;
use neocogi_com_sdk::{NC_E_CLOSED, NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_OUT_OF_RANGE, NC_OK, NcOutputWriter};

/// Selects production delegation or a precise fault in the independent writer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WriterMode {
    Production,
    Independent,
    CreateFailure,
    FinishFailure,
    ShortPrefix,
    WrongCount,
    OversizedLength,
    CopyFailure,
}

thread_local! {
    pub(super) static MODE: Cell<WriterMode> = const { Cell::new(WriterMode::Production) };
    /// Detects an automatic failure report on a write returning a status.
    static FAIL_REPORTS: Cell<usize> = const { Cell::new(0) };
}

/// The compiled writer handles sequences, u8 and u32. No record methods are used.
#[neocogi_com_sdk::com_object(interfaces(ISerializer, ISerializationOutput), ownership = exclusive)]
pub(super) struct Writer {
    mode: WriterMode,
    limits: neocogi_com_sdk::serialization::Limits,
    output: Vec<u8>,
    /// Number of values that still fit in the complete document.
    values_left: usize,
    /// Remaining elements for each open sequence.
    counts: Vec<u32>,
    phase: Phase,
}

/// The exclusive session accepts one root and publishes bytes after finish.
enum Phase {
    Empty,
    Writing,
    Failed(NcStatus),
    Finished,
}

impl Writer {
    pub(super) fn create(mode: WriterMode, limits: neocogi_com_sdk::serialization::Limits) -> Result<ComBox<ISerializerAbi>, NcStatus> {
        if mode == WriterMode::CreateFailure {
            return Err(NC_E_CLOSED);
        }
        if limits.bytes < 4 {
            return Err(NC_E_OUT_OF_RANGE);
        }
        Self::into_box(Self {
            mode,
            limits,
            output: b"NCB\x01".to_vec(),
            values_left: limits.values,
            counts: Vec::new(),
            phase: Phase::Empty,
        })
        .query()
        .map_err(|(status, _)| status)
    }

    /// Records the first operation error and keeps it terminal.
    fn write(&mut self, run: impl FnOnce(&mut Self) -> Result<(), NcStatus>) -> Result<(), NcStatus> {
        match self.phase {
            Phase::Failed(status) => return Err(status),
            Phase::Finished => return Err(NC_E_INVALID_ARG),
            Phase::Empty | Phase::Writing => {}
        }
        let result = run(self);
        if let Err(status) = result {
            self.phase = Phase::Failed(status);
        }
        result
    }

    /// Counts one value and appends its primitive bytes or sequence count.
    fn value(&mut self, bytes: &[u8]) -> Result<(), NcStatus> {
        if self.counts.len() >= self.limits.depth || bytes.len() > self.limits.bytes - self.output.len() {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.values_left = self.values_left.checked_sub(1).ok_or(NC_E_OUT_OF_RANGE)?;
        if let Some(remaining) = self.counts.last_mut() {
            *remaining = remaining.checked_sub(1).ok_or(NC_E_INVALID_ARG)?;
        } else if matches!(self.phase, Phase::Empty) {
            self.phase = Phase::Writing;
        } else {
            return Err(NC_E_INVALID_ARG);
        }
        self.output.extend_from_slice(bytes);
        Ok(())
    }
}

/// The test writer only needs the operations in the positional object layout.
macro_rules! unsupported {
    ($($name:ident($($argument:ident: $ty:ty),*));* $(;)?) => {$(
        fn $name(&mut self, $($argument:$ty),*) -> Result<(), NcStatus> {
            $(let _ = $argument;)*
            self.write(|_| Err(NC_E_INVALID_ARG))
        }
    )*};
}

impl ISerializer for Writer {
    unsupported! {
        serialize_unit();
        serialize_bool(value: bool);
        serialize_u16(value: u16);
        serialize_u64(value: u64);
        serialize_i8(value: i8);
        serialize_i16(value: i16);
        serialize_i32(value: i32);
        serialize_i64(value: i64);
        serialize_f32(value: f32);
        serialize_f64(value: f64);
        serialize_string(value: NcCopyFromBufferOffset<'_, u8>);
        begin_option(present: bool);
        end_option();
        begin_record(fields: u32);
        field(name: NcCopyFromBufferOffset<'_, u8>);
        end_record();
        begin_map(pairs: u32);
        end_map();
        begin_variant(index: u32, name: NcCopyFromBufferOffset<'_, u8>);
        end_variant();
    }

    fn serialize_u8(&mut self, value: u8) -> Result<(), NcStatus> {
        self.write(|this| this.value(&[value]))
    }

    fn serialize_u32(&mut self, value: u32) -> Result<(), NcStatus> {
        self.write(|this| this.value(&value.to_le_bytes()))
    }

    fn begin_sequence(&mut self, count: u32) -> Result<(), NcStatus> {
        self.write(|this| {
            if count as usize > this.limits.elements || count as usize >= this.values_left || (count != 0 && this.counts.len() + 1 >= this.limits.depth) {
                return Err(NC_E_OUT_OF_RANGE);
            }
            this.value(&count.to_le_bytes())?;
            this.counts.push(count);
            Ok(())
        })
    }

    fn end_sequence(&mut self) -> Result<(), NcStatus> {
        self.write(|this| match this.counts.pop() {
            Some(0) => Ok(()),
            _ => Err(NC_E_INVALID_ARG),
        })
    }

    fn fail(&mut self, status: NcStatus, _text: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        FAIL_REPORTS.with(|calls| calls.set(calls.get() + 1));
        self.write(|_| Err(status.normalize_error()))
    }

    fn finish(&mut self) -> Result<(), NcStatus> {
        self.write(|this| {
            if this.mode == WriterMode::FinishFailure {
                return Err(NC_E_CLOSED);
            }
            if !matches!(this.phase, Phase::Writing) || !this.counts.is_empty() {
                return Err(NC_E_INVALID_ARG);
            }
            this.phase = Phase::Finished;
            Ok(())
        })
    }
}

impl ISerializationOutput for Writer {
    fn byte_len(&self) -> Result<u64, NcStatus> {
        if matches!(self.phase, Phase::Finished) {
            Ok(if self.mode == WriterMode::OversizedLength {
                u64::MAX
            } else {
                self.output.len() as u64
            })
        } else {
            Err(NC_E_INVALID_ARG)
        }
    }

    fn copy_to(&self, mut output: NcOutputWriter<'_, u8>) -> Result<u64, NcStatus> {
        let len = self.byte_len()?;
        match self.mode {
            WriterMode::ShortPrefix => output.extend_from_slice(&self.output[..self.output.len() - 1])?,
            WriterMode::CopyFailure => return Err(NC_E_CLOSED),
            _ => output.extend_from_slice(&self.output)?,
        }
        Ok(if self.mode == WriterMode::WrongCount { len + 1 } else { len })
    }
}

/// Each getter creates its child on demand. The writer holds the sole child owner.
#[neocogi_com_sdk::com_object(IPersistable)]
struct LazyNode {
    depth: usize,
    end_depth: usize,
    active: Rc<Cell<usize>>,
    fail_at_leaf: bool,
}

impl Drop for LazyNode {
    fn drop(&mut self) {
        assert_eq!(self.active.get(), self.depth, "descendants drop before ancestors");
        self.active.set(self.depth - 1);
    }
}

impl IPersistable for LazyNode {
    fn type_id(&self) -> Result<NcGuid, NcStatus> {
        Ok(KIND)
    }

    fn schema_version(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }

    fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        if self.fail_at_leaf && self.depth == self.end_depth {
            return Err(NC_E_NOT_FOUND);
        }
        Ok(NcBytes::from_vec(Vec::new()))
    }

    fn child_count(&self) -> Result<u32, NcStatus> {
        Ok(u32::from(self.depth < self.end_depth))
    }

    fn child(&self, index: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        assert_eq!(index, 0);
        assert!(self.depth < self.end_depth);
        assert_eq!(self.active.get(), self.depth);
        let depth = self.depth + 1;
        self.active.set(depth);
        Ok(Self::from_value(Self {
            depth,
            end_depth: self.end_depth,
            active: self.active.clone(),
            fail_at_leaf: self.fail_at_leaf,
        }))
    }
}

/// More than the stock 128 wire levels, with release order on success and failure.
#[test]
fn deep_writer_releases_ephemeral_children_and_skips_failure_reporting() {
    if isolated("stream_writer::deep_writer_releases_ephemeral_children_and_skips_failure_reporting") {
        return;
    }
    let runtime = crate::initialize_runtime();
    // SAFETY: This isolated process links the complete provider graph.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    MODE.with(|mode| mode.set(WriterMode::Independent));
    let depth = 256;
    let mut limits = Limits { depth, ..Limits::default() };
    limits.binary.depth = 2 * depth + 1;
    for fail_at_leaf in [false, true] {
        let active = Rc::new(Cell::new(1));
        let root = LazyNode::from_value(LazyNode {
            depth: 1,
            end_depth: depth,
            active: active.clone(),
            fail_at_leaf,
        });
        let result = save_object_tree_with_limits(&runtime, &root, limits);
        assert_eq!(active.get(), 1);
        assert_eq!(FAIL_REPORTS.with(Cell::get), 0);
        if fail_at_leaf {
            assert_eq!(result, Err(NC_E_NOT_FOUND));
        } else {
            assert_eq!(result.unwrap().len(), 4 + 36 * depth);
        }
        drop(root);
        assert_eq!(active.get(), 0);
    }
}

/// Observes the complete document, then reports a delivery failure.
/// # Safety
/// The input and receipt satisfy `receive_saved_document`'s requirements.
unsafe extern "C" fn decline_saved_document(data: *mut core::ffi::c_void, bytes: *const u8, len: usize) -> NcStatus {
    // SAFETY: This callback has the same borrowed document and receipt contract.
    let status = unsafe { receive_saved_document(data, bytes, len) };
    if status.is_err() { status } else { NC_E_CLOSED }
}

/// Failures in writer creation, finish and checked output copy skip the raw sink.
#[test]
fn writer_faults_do_not_deliver_partial_documents() {
    if isolated("stream_writer::writer_faults_do_not_deliver_partial_documents") {
        return;
    }
    let runtime = crate::initialize_runtime();
    // SAFETY: This isolated process links the complete provider graph.
    unsafe { runtime.register_plugin_descriptor(plugin_descriptor()) }.unwrap();
    let owner = Value::from_value(Value::new(1));
    // SAFETY: The runtime owns an immutable process-lived persistence table.
    let api = unsafe { &*crate::initialize_process().persistence };
    for (mode, status) in [
        (WriterMode::CreateFailure, NC_E_CLOSED),
        (WriterMode::FinishFailure, NC_E_CLOSED),
        (WriterMode::ShortPrefix, NC_E_INVALID_ARG),
        (WriterMode::WrongCount, NC_E_INVALID_ARG),
        (WriterMode::OversizedLength, NC_E_OUT_OF_RANGE),
        (WriterMode::CopyFailure, NC_E_CLOSED),
        (WriterMode::Independent, NC_OK),
    ] {
        MODE.with(|selection| selection.set(mode));
        let mut receipt = SaveReceipt::default();
        // SAFETY: The call borrows the stable root and the exclusive local receipt.
        let outcome = unsafe {
            (api.save_object_tree)(
                owner.as_raw(),
                Limits::default().into(),
                Some(receive_saved_document),
                core::ptr::from_mut(&mut receipt).cast(),
            )
        };
        assert_eq!(outcome, status, "{mode:?}");
        assert_eq!(receipt.calls, usize::from(status.is_ok()));
        if status.is_ok() {
            assert_eq!(receipt.bytes, bytes());
        }
    }
    let mut receipt = SaveReceipt::default();
    // SAFETY: The final normal writer lends a complete document to this live receipt.
    let rejected = unsafe {
        (api.save_object_tree)(
            owner.as_raw(),
            Limits::default().into(),
            Some(decline_saved_document),
            core::ptr::from_mut(&mut receipt).cast(),
        )
    };
    assert_eq!(rejected, NC_E_CLOSED);
    assert_eq!(receipt.calls, 1);
    assert_eq!(receipt.bytes, bytes());
}
