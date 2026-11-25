//! Writes a live object tree through the SDK serializer.

use neocogi_com::raw;
use neocogi_com_sdk::persistence::{IPersistableAbi, Limits, SavedObject};
use neocogi_com_sdk::serialization::Serializer;
use neocogi_com_sdk::{ComBytesExt, ComPtr, ComRaw, ComSerialize, INcBytesAbi, IUnknown, NC_E_INVALID_ARG, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE, NcStatus};

/// Writes one complete document using the host's selected NCB writer.
///
/// # Safety
/// The raw pair satisfies the shared-root and stable-view requirements of
/// `NcPersistenceApi::save_object_tree`.
pub(crate) unsafe fn save_object_tree_with_limits(object: ComRaw<IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
    // SAFETY: The caller lends the live pair under the raw save contract.
    unsafe { raw::validate_interface(object) }?;
    let remaining_after_root = limits.objects.checked_sub(1).ok_or(NC_E_OUT_OF_RANGE)?;
    if limits.depth == 0 {
        return Err(NC_E_OUT_OF_RANGE);
    }
    // SAFETY: The exact root was validated and its shared lender remains live.
    let root = unsafe { ComPtr::clone_from_raw(object) }.ok_or(NC_E_INVALID_ARG)?;
    let codec = super::ncb_format(limits.binary)?;
    codec.format.serialize_status(&ObjectTree {
        root: &root,
        remaining_after_root,
        max_object_depth: limits.depth,
    })
}

/// A stable live view borrowed until the selected writer commits or fails.
struct ObjectTree<'a> {
    /// Exact shared root retained for the full write.
    root: &'a ComPtr<IUnknown>,
    /// Slots available for children after charging the root.
    remaining_after_root: usize,
    /// Maximum number of objects along an ownership path.
    max_object_depth: usize,
}

/// An open record whose child sequence is being written.
struct SaveFrame {
    /// Exact persistence projection retains the same allocation as the child owner.
    node: ComPtr<IPersistableAbi>,
    /// Declared children already charged to the object budget.
    child_count: u32,
    /// Next child to acquire in provider order.
    next_child: u32,
}

impl ComSerialize for ObjectTree<'_> {
    fn com_serialize(&self, writer: &mut Serializer<'_>) -> Result<(), NcStatus> {
        let mut frames = Vec::new();
        let mut remaining = self.remaining_after_root;
        let mut scratch = [0; 4096];
        let result = (|| {
            SavedObject::begin_write(writer)?;
            frames.try_reserve(1).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            frames.push(self.write_node(writer, &mut scratch, self.root, 1, &mut remaining)?);
            while let Some(parent) = frames.last() {
                if parent.next_child == parent.child_count {
                    SavedObject::end_write(writer)?;
                    frames.pop();
                    continue;
                }
                // The writer can refuse the child's mandatory prefix before
                // the object provider gets a callback. Reserve just one frame.
                SavedObject::begin_write(writer)?;
                frames.try_reserve(1).map_err(|_| NC_E_OUT_OF_MEMORY)?;
                let parent = frames.last_mut().expect("the open parent is still active");
                let child = parent.node.child(parent.next_child)?;
                parent.next_child += 1;
                if frames.iter().any(|frame| frame.node.as_com_ref().same_object(child.as_com_ref())) {
                    return Err(NC_E_INVALID_ARG);
                }
                let frame = self.write_node(writer, &mut scratch, &child, frames.len() + 1, &mut remaining)?;
                frames.push(frame);
            }
            Ok(())
        })();
        // On failure a child can still depend on one of its active parents.
        // Release the deepest owner first, including during provider reentry.
        while frames.pop().is_some() {}
        result
    }
}

impl ObjectTree<'_> {
    /// Fills the admitted prefix and charges all declared children before reading state.
    fn write_node(
        &self,
        writer: &mut Serializer<'_>,
        scratch: &mut [u8; 4096],
        object: &ComPtr<IUnknown>,
        depth: usize,
        remaining: &mut usize,
    ) -> Result<SaveFrame, NcStatus> {
        let node = object.query::<IPersistableAbi>()?;
        let child_count = node.child_count()?;
        let count = usize::try_from(child_count).map_err(|_| NC_E_OUT_OF_RANGE)?;
        if count != 0 && depth >= self.max_object_depth {
            return Err(NC_E_OUT_OF_RANGE);
        }
        *remaining = remaining.checked_sub(count).ok_or(NC_E_OUT_OF_RANGE)?;
        let type_id = node.type_id()?.to_uuid_bytes();
        let schema_version = node.schema_version()?;
        SavedObject::write_fields(
            writer,
            &type_id,
            schema_version,
            |writer| write_state(writer, &node.save_state()?, scratch),
            count,
        )?;
        Ok(SaveFrame { node, child_count, next_child: 0 })
    }
}

/// Writes one stable byte buffer in bounded chunks through the ordinary sequence API.
fn write_state(writer: &mut Serializer<'_>, bytes: &ComPtr<INcBytesAbi>, scratch: &mut [u8; 4096]) -> Result<(), NcStatus> {
    // Reject unrepresentable lengths without the serializer's diagnostic path.
    let len = u32::try_from(bytes.len()?).map_err(|_| NC_E_OUT_OF_RANGE)?;
    let len = usize::try_from(len).map_err(|_| NC_E_OUT_OF_RANGE)?;
    writer.begin_sequence(len)?;
    let mut offset = 0;
    while offset < len {
        let count = (len - offset).min(scratch.len());
        let chunk = &mut scratch[..count];
        bytes.copy_to(offset as u64, chunk)?;
        for &byte in chunk.iter() {
            writer.serialize_u8(byte)?;
        }
        offset += count;
    }
    writer.end_sequence()
}
