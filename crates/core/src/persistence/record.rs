//! Portable initialization values, independent of runtime allocation ownership.

use alloc::vec::Vec;
use crate::{binary, ComDeserialize, ComSerialize};

/// One owned initialization tree with the original NCB-v1 field ordering.
///
/// This is a local Rust value, never an argument passed by value across COM.
/// Sharing the schema lets authoring tools encode documents without a runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRecord {
    /// Recipe UUID in canonical byte order, independent of object identity.
    pub type_id: [u8; 16],
    /// Provider-owned persistent schema revision.
    pub schema_version: u32,
    /// Explicit initialization bytes, excluding process-local resources.
    pub state: Vec<u8>,
    /// Owned children in the order expected by the parent's factory.
    pub children: Vec<ObjectRecord>,
}

impl ObjectRecord {
    /// Count shared by the ordered encoder, decoder and virtual leaf measurement.
    const FIELD_COUNT: usize = 4;
    /// Measures a leaf persistence envelope before allocating its state bytes.
    /// UUID/revision contents have fixed widths, so zero placeholders give the
    /// exact shape for any recipe. State remains a sequence of byte values; it
    /// consumes the same element/value budgets as an actual Vec<u8>. This helper
    /// covers only records with no persistent children, not arbitrary trees.
    pub fn encoded_leaf_len(state_bytes: usize, limits: binary::Limits) -> Result<usize, binary::Error> {
        binary::encoded_len_with_limits(&LeafSize { state_bytes }, limits)
    }

    /// Encodes the single authoritative field order for owned records and leaf
    /// measurement. Abstracting only state serialization avoids a second envelope
    /// schema or hardcoded framing/value overhead in document admission checks.
    fn serialize_fields<T: ComSerialize + ?Sized>(
        serializer: &mut binary::Writer<'_>,
        type_id: &[u8; 16],
        schema_version: u32,
        state: &T,
        children: &[Self],
    ) -> Result<(), binary::Error> {
        serializer.begin_sequence(Self::FIELD_COUNT)?;
        type_id.com_serialize(serializer)?;
        schema_version.com_serialize(serializer)?;
        state.com_serialize(serializer)?;
        children.com_serialize(serializer)?;
        serializer.end_sequence()
    }
}

impl ComSerialize for ObjectRecord {
    /// Preserves the positional schema and exact field order used before relocation.
    fn com_serialize(&self, serializer: &mut binary::Writer<'_>) -> Result<(), binary::Error> {
        Self::serialize_fields(serializer, &self.type_id, self.schema_version, &self.state, &self.children)
    }
}

/// Counting-only leaf record; no placeholder byte allocation accompanies its length.
struct LeafSize {
    /// Number of state bytes which will become individual sequence values.
    state_bytes: usize,
}
impl ComSerialize for LeafSize {
    /// Uses the same envelope fields as a real owned record with empty children.
    fn com_serialize(&self, serializer: &mut binary::Writer<'_>) -> Result<(), binary::Error> {
        ObjectRecord::serialize_fields(serializer, &[0; 16], 0, &ByteCount(self.state_bytes), &[])
    }
}

/// A virtual sequence used only for measurement; byte contents cannot affect NCB size.
struct ByteCount(usize);
impl ComSerialize for ByteCount {
    /// Collection admission happens before visiting any elements, exactly as for
    /// Vec<u8>. Each zero still consumes one scalar value: this is not a blob.
    fn com_serialize(&self, serializer: &mut binary::Writer<'_>) -> Result<(), binary::Error> {
        serializer.begin_sequence(self.0)?;
        for _ in 0..self.0 {
            serializer.serialize_u8(0)?;
        }
        serializer.end_sequence()
    }
}

impl ComDeserialize for ObjectRecord {
    /// Acquires each required field once, in the same order used by serialize_fields.
    fn com_deserialize(reader: &mut binary::Reader<'_>) -> Result<Self, binary::Error> {
        reader.record(Self::FIELD_COUNT, |fields| {
            Ok(Self {
                type_id: fields.next()?.ok_or_else(|| fields.error("missing field"))?,
                schema_version: fields.next()?.ok_or_else(|| fields.error("missing field"))?,
                state: fields.next()?.ok_or_else(|| fields.error("missing field"))?,
                children: fields.next()?.ok_or_else(|| fields.error("missing field"))?,
            })
        })
    }
}

/// Local policy for both snapshot traversal and factory reconstruction.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Binary encoding/decoding budget, including state bytes and value counts.
    pub binary: binary::Limits,
    /// Maximum object count across the complete tree.
    pub objects: usize,
    /// Maximum objects along one root-to-leaf path.
    pub depth: usize,
}

impl Default for Limits {
    /// Retains the previous persistence limits for ordinary initialization trees.
    fn default() -> Self {
        Self {
            binary: binary::Limits::default(),
            objects: 4096,
            depth: 48,
        }
    }
}
