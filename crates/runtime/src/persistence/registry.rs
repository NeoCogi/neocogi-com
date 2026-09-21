//! Caller-owned recipe registration and two-pass object reconstruction.

use std::collections::BTreeMap;
use neocogi_com_core::{binary, ComPtr, IUnknown, NcCopyFromBufferOffset, NcGuid, NcStatus, NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_OUT_OF_RANGE};
use neocogi_com_core::persistence::{IPersistenceFactoryAbi, Limits, ObjectRecord};

/// An explicit set of context-bound factories selected by the application.
#[derive(Default)]
pub(crate) struct Registry {
    /// Recipe identity maps to one provider; replacement is never implicit.
    factories: BTreeMap<[u8; 16], ComPtr<IPersistenceFactoryAbi>>,
}
impl Registry {
    /// Creates an empty loader with no dynamically discovered providers.
    pub(crate) fn new() -> Self {
        Self::default()
    }
    /// Registers one recipe exactly once; duplicate IDs preserve the prior entry.
    pub(crate) fn register(&mut self, id: NcGuid, factory: ComPtr<IPersistenceFactoryAbi>) -> Result<(), NcStatus> {
        // Entry matching prevents replacement and releases rejected factory owners.
        match self.factories.entry(id.to_uuid_bytes()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(factory);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => Err(NC_E_INVALID_ARG),
        }
    }
    /// Uses bounded decoding and preflight before invoking any constructors.
    pub(crate) fn restore_with_limits(&self, bytes: &[u8], limits: Limits) -> Result<ComPtr<IUnknown>, NcStatus> {
        let record: ObjectRecord = binary::from_slice_with_limits(bytes, limits.binary).map_err(|_| NC_E_INVALID_ARG)?;
        // Complete preflight must succeed before the first constructor runs.
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
        // Providers inspect immutable initialization state; they must not publish
        // resources during validation. Their constructor validates again on direct use.
        let factory = self.factories.get(&record.type_id).ok_or(NC_E_NOT_FOUND)?;
        factory.validate(
            record.schema_version,
            NcCopyFromBufferOffset::new(0, &record.state),
            u32::try_from(record.children.len()).map_err(|_| NC_E_OUT_OF_RANGE)?,
        )?;
        for child in &record.children {
            self.validate(child, limits, depth + 1, remaining)?;
        }
        Ok(())
    }
    /// Children are ordinary owning COM references until the parent adopts them.
    fn construct(&self, record: &ObjectRecord) -> Result<ComPtr<IUnknown>, NcStatus> {
        // Collection drops already-built siblings on failure. The COM list then
        // owns all descendants until a successful parent adopts its chosen children.
        let children = record.children.iter().map(|child| self.construct(child)).collect::<Result<Vec<_>, _>>()?;
        let children = super::children::create(children)?;
        self.factories.get(&record.type_id).ok_or(NC_E_NOT_FOUND)?.restore(
            record.schema_version,
            NcCopyFromBufferOffset::new(0, &record.state),
            children.as_com_ref(),
        )
    }
}
