//! Portable initialization values, independent of runtime allocation ownership.

use alloc::vec::Vec;
use crate::{binary, ComDeserialize, ComDeserializer, ComRecord, ComRecordAccess, ComSerialize, ComSerializer};

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
    fn serialize_fields<S: ComSerializer, T: ComSerialize + ?Sized>(
        serializer: &mut S, type_id: &[u8; 16], schema_version: u32, state: &T, children: &[Self],
    ) -> Result<(), S::Error> {
        serializer.begin_record(Self::NAME, Self::FIELDS.len())?;
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

impl ComSerialize for ObjectRecord {
    /// Preserves the named schema and exact field order used before relocation.
    fn com_serialize<S: ComSerializer>(&self, serializer: &mut S) -> Result<(), S::Error> {
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
    fn com_serialize<S: ComSerializer>(&self, serializer: &mut S) -> Result<(), S::Error> {
        ObjectRecord::serialize_fields(serializer, &[0; 16], 0, &ByteCount(self.state_bytes), &[])
    }
}

/// A virtual sequence used only for measurement; byte contents cannot affect NCB size.
struct ByteCount(usize);
impl ComSerialize for ByteCount {
    /// Collection admission happens before visiting any elements, exactly as for
    /// Vec<u8>. Each zero still consumes one scalar value: this is not a blob.
    fn com_serialize<S: ComSerializer>(&self, serializer: &mut S) -> Result<(), S::Error> {
        serializer.begin_sequence(self.0)?;
        for _ in 0..self.0 {
            serializer.serialize_u8(0)?;
        }
        serializer.end_sequence()
    }
}

impl ComDeserialize for ObjectRecord {
    /// Delegates format policy and limits to the selected typed decoder.
    fn com_deserialize<'de, D: ComDeserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        decoder.deserialize_record::<Self>()
    }
}

impl ComRecord for ObjectRecord {
    const NAME: &'static str = "ObjectRecord";
    const FIELDS: &'static [&'static str] = &["type_id", "schema_version", "state", "children"];

    /// Rejects missing, duplicate and unknown fields without an erased value map.
    fn read_record<'de, A: ComRecordAccess<'de>>(record: &mut A) -> Result<Self, A::Error> {
        let mut type_id = None;
        let mut schema_version = None;
        let mut state = None;
        let mut children = None;
        // Each typed slot is either absent or initialized exactly once. Match
        // makes that transition explicit and preserves the former derive policy.
        while let Some(field) = record.next_field(Self::FIELDS)? {
            match field {
                0 => read_once(record, &mut type_id, "type_id")?,
                1 => read_once(record, &mut schema_version, "schema_version")?,
                2 => read_once(record, &mut state, "state")?,
                3 => read_once(record, &mut children, "children")?,
                _ => return Err(record.invalid_field_index(field)),
            }
        }
        Ok(Self {
            type_id: type_id.ok_or_else(|| record.missing_field("type_id"))?,
            schema_version: schema_version.ok_or_else(|| record.missing_field("schema_version"))?,
            state: state.ok_or_else(|| record.missing_field("state"))?,
            children: children.ok_or_else(|| record.missing_field("children"))?,
        })
    }
}

/// Initializes a statically typed field once, preserving decoder diagnostics.
fn read_once<'de, A: ComRecordAccess<'de>, T: ComDeserialize>(record: &mut A, slot: &mut Option<T>, name: &'static str) -> Result<(), A::Error> {
    match slot {
        Some(_) => Err(record.duplicate_field(name)),
        None => {
            *slot = Some(record.next_value()?);
            Ok(())
        }
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
