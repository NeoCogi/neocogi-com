//! Portable initialization values, independent of runtime allocation ownership.

use alloc::vec::Vec;
use crate::{serialization, ComBox, ComDeserialize, ComSerialize, NC_E_INVALID_ARG, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE, NcStatus};

/// Recipe, initialization bytes, and child records needed to rebuild one object.
///
/// Authoring tools can construct this value and serialize it without a runtime.
/// Serialization preserves the original NCB-v1 field order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedObject {
    /// Recipe UUID in canonical byte order, independent of object identity.
    pub type_id: [u8; 16],
    /// Provider-owned persistent schema revision.
    pub schema_version: u32,
    /// Explicit initialization bytes, excluding process-local resources.
    pub state: Vec<u8>,
    /// Owned children in the order expected by the parent's factory.
    pub children: Vec<SavedObject>,
}

impl SavedObject {
    /// Count shared by the ordered encoder, decoder and virtual leaf measurement.
    const FIELD_COUNT: usize = 4;
    /// Creates a virtual leaf value for format-specific measurement before allocating state.
    /// UUID/revision contents have fixed widths, so zero placeholders give the
    /// exact shape for any recipe. State remains a sequence of byte values; it
    /// consumes the same element/value budgets as an actual `Vec<u8>`. This helper
    /// covers only records with no persistent children, not arbitrary trees.
    pub fn leaf_size(state_bytes: usize) -> LeafSize {
        LeafSize { state_bytes }
    }

    /// Encodes the single authoritative field order for owned records and leaf
    /// measurement. Abstracting only state serialization avoids a second envelope
    /// schema or hardcoded framing/value overhead in document admission checks.
    fn serialize_fields<T: ComSerialize + ?Sized>(
        serializer: &mut crate::serialization::Serializer<'_>,
        type_id: &[u8; 16],
        schema_version: u32,
        state: &T,
        children: &[Self],
    ) -> Result<(), crate::NcStatus> {
        serializer.begin_record(Self::FIELD_COUNT)?;
        serializer.field("type_id")?;
        type_id.com_serialize(serializer)?;
        serializer.field("schema_version")?;
        schema_version.com_serialize(serializer)?;
        serializer.field("state")?;
        state.com_serialize(serializer)?;
        serializer.field("children")?;
        children.com_serialize(serializer)?;
        serializer.end_record()
    }
}

impl ComSerialize for SavedObject {
    /// Preserves the positional schema and exact field order used before relocation.
    fn com_serialize(&self, serializer: &mut crate::serialization::Serializer<'_>) -> Result<(), crate::NcStatus> {
        Self::serialize_fields(serializer, &self.type_id, self.schema_version, &self.state, &self.children)
    }
}

/// Counting-only leaf record; no placeholder byte allocation accompanies its length.
pub struct LeafSize {
    /// Number of state bytes which will become individual sequence values.
    state_bytes: usize,
}
impl ComSerialize for LeafSize {
    /// Uses the same envelope fields as a real owned record with empty children.
    fn com_serialize(&self, serializer: &mut crate::serialization::Serializer<'_>) -> Result<(), crate::NcStatus> {
        SavedObject::serialize_fields(serializer, &[0; 16], 0, &ByteCount(self.state_bytes), &[])
    }
}

/// A virtual sequence used only for measurement; byte contents cannot affect NCB size.
struct ByteCount(usize);
impl ComSerialize for ByteCount {
    /// Collection admission happens before visiting any elements, exactly as for
    /// Vec<u8>. Each zero still consumes one scalar value: this is not a blob.
    fn com_serialize(&self, serializer: &mut crate::serialization::Serializer<'_>) -> Result<(), crate::NcStatus> {
        serializer.begin_sequence(self.0)?;
        for _ in 0..self.0 {
            serializer.serialize_u8(0)?;
        }
        serializer.end_sequence()
    }
}

impl SavedObject {
    /// Decodes a complete restore document under the caller's object policy.
    ///
    /// Consumes and releases the selected format's source, including on failure.
    /// Codec errors map to invalid argument. Local shape admission reports range,
    /// and refusal of a local child-storage reservation reports out of memory.
    /// The latter discards the source without requesting a provider diagnostic.
    #[doc(hidden)]
    pub fn decode_for_restore(mut source: ComBox<serialization::IDeserializerAbi>, limits: Limits) -> Result<Self, NcStatus> {
        let mut reader = serialization::Deserializer::for_restore(source.as_com_mut(), limits.binary);
        let mut budget = RestoreBudget {
            remaining: limits.objects,
            depth: limits.depth,
            rejected: None,
        };
        reader
            .deserialize_with(|reader| Self::read_record(reader, Some(&mut budget), 1))
            .map_err(|_| budget.rejected.unwrap_or(NC_E_INVALID_ARG))
    }

    /// Supplies the same field order and names to authoring tools and host restore.
    fn read_record(reader: &mut serialization::Deserializer<'_>, mut budget: Option<&mut RestoreBudget>, depth: usize) -> Result<Self, NcStatus> {
        reader.record(Self::FIELD_COUNT, |fields| {
            if let (1, Some(budget)) = (depth, budget.as_deref_mut()) {
                budget.charge(1, 0)?;
            }
            let type_id = fields.read_field("type_id")?.ok_or_else(|| fields.error("missing field"))?;
            let schema_version = fields.read_field("schema_version")?.ok_or_else(|| fields.error("missing field"))?;
            let state = fields.read_field("state")?.ok_or_else(|| fields.error("missing field"))?;
            let children = match budget {
                Some(budget) => fields.read_field_with("children", |reader| budget.read_children(reader, depth))?,
                None => fields.read_field("children")?,
            }
            .ok_or_else(|| fields.error("missing field"))?;
            Ok(Self { type_id, schema_version, state, children })
        })
    }
}

impl ComDeserialize for SavedObject {
    /// Authoring tools use the selected format's binary limits alone.
    fn com_deserialize(reader: &mut serialization::Deserializer<'_>) -> Result<Self, NcStatus> {
        Self::read_record(reader, None, 1)
    }
}

/// One restore operation's allowance for decoded and already declared object slots.
struct RestoreBudget {
    /// Unclaimed slots after accounting for every admitted child count.
    remaining: usize,
    /// Maximum number of object records on one root-to-leaf path.
    depth: usize,
    /// A local rejection whose status must survive the codec error translation.
    rejected: Option<NcStatus>,
}

impl RestoreBudget {
    /// Applies the budget after binary count admission, before entering the next level.
    fn charge(&mut self, count: usize, parent_depth: usize) -> Result<(), NcStatus> {
        if count != 0 && (count > self.remaining || parent_depth >= self.depth) {
            return Err(self.reject(NC_E_OUT_OF_RANGE));
        }
        self.remaining -= count;
        Ok(())
    }

    /// Preserves the host failure separately from any selected provider status.
    fn reject(&mut self, status: NcStatus) -> NcStatus {
        self.rejected = Some(status);
        status
    }

    /// Charges all declared siblings once, then allocates only for completed records.
    fn read_children(&mut self, reader: &mut serialization::Deserializer<'_>, depth: usize) -> Result<Vec<SavedObject>, NcStatus> {
        reader.sequence(|sequence| {
            let count = sequence.remaining();
            self.charge(count, depth)?;
            let mut children = Vec::new();
            while let Some(child) = sequence.read_next_with(|reader| SavedObject::read_record(reader, Some(self), depth + 1))? {
                if children.len() == children.capacity() {
                    let capacity = children.len().saturating_mul(2).max(1).min(count);
                    children
                        .try_reserve_exact(capacity - children.len())
                        .map_err(|_| self.reject(NC_E_OUT_OF_MEMORY))?;
                }
                children.push(child);
            }
            Ok(children)
        })
    }
}

/// Resource limits for saving and restoring object trees.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Binary encoding/decoding budget, including state bytes and value counts.
    pub binary: serialization::Limits,
    /// Maximum object count across the complete tree.
    pub objects: usize,
    /// Maximum objects along one root-to-leaf path.
    pub depth: usize,
}

impl Default for Limits {
    /// Retains the previous persistence limits for ordinary initialization trees.
    fn default() -> Self {
        Self {
            binary: serialization::Limits::default(),
            objects: 4096,
            depth: 48,
        }
    }
}
