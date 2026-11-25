//! Caller-owned recipe registration and two-pass object reconstruction.

use std::collections::BTreeMap;
use neocogi_com_sdk::{ComPtr, IUnknown, NcCopyFromBufferOffset, NcGuid, NcStatus, NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE};
use neocogi_com_sdk::persistence::{IPersistenceFactoryAbi, Limits, SavedObject};

/// An explicit set of context-bound factories selected by the application.
#[derive(Default)]
pub(crate) struct FactoryRegistry {
    /// Recipe identity maps to one provider; replacement is never implicit.
    factories: BTreeMap<[u8; 16], ComPtr<IPersistenceFactoryAbi>>,
}
impl FactoryRegistry {
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
    pub(crate) fn restore_object_tree_with_limits(&self, bytes: &[u8], limits: Limits) -> Result<ComPtr<IUnknown>, NcStatus> {
        let codec = super::ncb_format(limits.binary)?;
        let source = codec.format.deserializer(bytes).map_err(|_| NC_E_INVALID_ARG)?;
        let record = SavedObject::decode_for_restore(source, limits)?;
        // The exclusive reader has finished and dropped. The format and its
        // provider remain live through both factory passes.
        // Complete preflight must succeed before the first constructor runs.
        self.validate(&record)?;
        self.construct(&record)
    }
    /// Preflight traverses the complete tree, including children of known parents.
    fn validate(&self, record: &SavedObject) -> Result<(), NcStatus> {
        // Providers inspect immutable initialization state; they must not publish
        // resources during validation. Their constructor validates again on direct use.
        let factory = self.factories.get(&record.type_id).ok_or(NC_E_NOT_FOUND)?;
        factory.validate(
            record.schema_version,
            NcCopyFromBufferOffset::new(0, &record.state),
            u32::try_from(record.children.len()).map_err(|_| NC_E_OUT_OF_RANGE)?,
        )?;
        for child in &record.children {
            self.validate(child)?;
        }
        Ok(())
    }
    /// Children are ordinary owning COM references until the parent adopts them.
    fn construct(&self, record: &SavedObject) -> Result<ComPtr<IUnknown>, NcStatus> {
        // Collection drops already-built siblings on failure. The COM list then
        // owns all descendants until a successful parent adopts its chosen children.
        let mut children = Vec::new();
        for child in &record.children {
            let child = self.construct(child)?;
            children.try_reserve(1).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            children.push(child);
        }
        let children = super::children::create(children)?;
        self.factories.get(&record.type_id).ok_or(NC_E_NOT_FOUND)?.restore(
            record.schema_version,
            NcCopyFromBufferOffset::new(0, &record.state),
            children.as_com_ref(),
        )
    }
}
