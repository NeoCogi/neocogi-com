//! Opt-in COM persistence with explicit, caller-owned factory registration.
//!
//! Snapshots contain recipe IDs, schema revisions, initialization bytes and an
//! owned child tree. Registration is local to a loader, never a global class
//! registry. Providers keep runtime services outside the serialized document.

#![deny(missing_docs)]

use std::collections::BTreeMap;
use neocogi_com::{
    binary, ComPtr, ComRef, IUnknown, NcGuid, NcStatus, INcBytesComPtrExt, NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_OUT_OF_RANGE,
};
pub use neocogi_com_persistence_contracts::*;

/// Captures a complete tree with the default limits.
pub fn snapshot(object: ComRef<'_, IUnknown>) -> Result<Vec<u8>, NcStatus> {
    snapshot_with_limits(object, Limits::default())
}

/// Exports only opt-in state, detects ownership cycles, and bounds child traversal.
pub fn snapshot_with_limits(object: ComRef<'_, IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
    let mut remaining = limits.objects;
    let mut active = Vec::new();
    let mut bytes_left = limits.binary.bytes;
    let record = capture(object, limits, &mut remaining, &mut bytes_left, &mut active)?;
    binary::to_vec_with_limits(&record, limits.binary).map_err(|_| NC_E_INVALID_ARG)
}

/// Recursively captures semantic state while keeping identity checks local.
fn capture(
    object: ComRef<'_, IUnknown>,
    limits: Limits,
    remaining: &mut usize,
    bytes_left: &mut usize,
    active: &mut Vec<ComPtr<IUnknown>>,
) -> Result<ObjectRecord, NcStatus> {
    if *remaining == 0 || active.len() >= limits.depth {
        return Err(NC_E_OUT_OF_RANGE);
    }
    *remaining -= 1;
    let identity = object.add_ref().query::<IUnknown>()?;
    if active.iter().any(|parent| parent.as_mut_ptr() == identity.as_mut_ptr()) {
        return Err(NC_E_INVALID_ARG);
    }
    let value = identity.query::<IPersistableAbi>()?;
    active.push(identity);
    let count = value.child_count()?;
    if count as usize > *remaining || count as usize > limits.binary.elements {
        return Err(NC_E_OUT_OF_RANGE);
    }
    let state = value.save_state()?;
    // Inspect the remote byte count before copying consumer-owned storage.
    let state = state.to_vec_bounded((*bytes_left).min(limits.binary.elements))?;
    if state.len() > *bytes_left || state.len() > limits.binary.elements {
        return Err(NC_E_OUT_OF_RANGE);
    }
    *bytes_left -= state.len();
    let mut children = Vec::with_capacity(count as usize);
    for index in 0..count {
        children.push(capture(value.child(index)?.as_com_ref(), limits, remaining, bytes_left, active)?);
    }
    let result = ObjectRecord {
        type_id: value.type_id()?.to_uuid_bytes(),
        schema_version: value.schema_version()?,
        state,
        children,
    };
    active.pop();
    Ok(result)
}

/// An explicit set of context-bound factories selected by the application.
#[derive(Default)]
pub struct FactoryRegistry {
    /// Recipe identity maps to one provider; replacement is never implicit.
    factories: BTreeMap<[u8; 16], ComPtr<IPersistenceFactoryAbi>>,
}
impl FactoryRegistry {
    /// Creates an empty loader with no dynamically discovered providers.
    pub fn new() -> Self {
        Self::default()
    }
    /// Registers one recipe exactly once; duplicate IDs preserve the prior entry.
    pub fn register(&mut self, id: NcGuid, factory: ComPtr<IPersistenceFactoryAbi>) -> Result<(), NcStatus> {
        match self.factories.entry(id.to_uuid_bytes()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(factory);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => Err(NC_E_INVALID_ARG),
        }
    }
    /// Reconstructs a complete tree after validating every recipe and state.
    pub fn restore(&self, bytes: &[u8]) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.restore_with_limits(bytes, Limits::default())
    }
    /// Uses bounded decoding and preflight before invoking any constructors.
    pub fn restore_with_limits(&self, bytes: &[u8], limits: Limits) -> Result<ComPtr<IUnknown>, NcStatus> {
        let record: ObjectRecord = binary::from_slice_with_limits(bytes, limits.binary).map_err(|_| NC_E_INVALID_ARG)?;
        let mut remaining = limits.objects;
        self.validate(&record, limits, 0, &mut remaining)?;
        self.construct(&record)
    }
    /// Preflight traverses the complete tree, including children of known parents.
    fn validate(&self, record: &ObjectRecord, limits: Limits, depth: usize, remaining: &mut usize) -> Result<(), NcStatus> {
        if depth >= limits.depth || *remaining == 0 {
            return Err(NC_E_OUT_OF_RANGE);
        }
        *remaining -= 1;
        let factory = self.factories.get(&record.type_id).ok_or(NC_E_NOT_FOUND)?;
        factory.validate(
            record.schema_version,
            neocogi_com::NcCopyFromBufferOffset::new(0, &record.state),
            u32::try_from(record.children.len()).map_err(|_| NC_E_OUT_OF_RANGE)?,
        )?;
        for child in &record.children {
            self.validate(child, limits, depth + 1, remaining)?;
        }
        Ok(())
    }
    /// Children are ordinary owning COM references until the parent adopts them.
    fn construct(&self, record: &ObjectRecord) -> Result<ComPtr<IUnknown>, NcStatus> {
        let children = record.children.iter().map(|child| self.construct(child)).collect::<Result<Vec<_>, _>>()?;
        let children = Children::from_value(Children { values: children }).query::<IObjectChildrenAbi>()?;
        self.factories.get(&record.type_id).ok_or(NC_E_NOT_FOUND)?.restore(
            record.schema_version,
            neocogi_com::NcCopyFromBufferOffset::new(0, &record.state),
            children.as_com_ref(),
        )
    }
}

/// Read-only construction list; it never exposes a Rust Vec across COM.
#[neocogi_com::com_object(interfaces(IObjectChildren))]
struct Children {
    /// Strong references keep all children alive throughout parent construction.
    values: Vec<ComPtr<IUnknown>>,
}
impl IObjectChildren for Children {
    /// Bounds were validated before this list was created.
    fn count(&self) -> Result<u32, NcStatus> {
        u32::try_from(self.values.len()).map_err(|_| NC_E_OUT_OF_RANGE)
    }
    /// The parent receives ownership only for children it actually requests.
    fn get(&self, index: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.values.get(index as usize).cloned().ok_or(NC_E_OUT_OF_RANGE)
    }
}
