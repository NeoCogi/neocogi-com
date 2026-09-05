//! Safe Rust view of the runtime type registry.
//!
//! This context enumerates class IDs and complete interface ABI identities.
//! Provenance and containment queries are deliberately kept in `Inspection`.

use std::vec::Vec;

use crate::{NcGuid, NcInterfaceId, NcStatus};

use super::{Runtime, read_vec};

/// Read-only catalog of registered classes and interface ABI identities.
#[derive(Clone, Copy, Debug)]
pub struct TypeRegistry<'runtime> {
    /// Runtime facade used for generation and table-size checks on every call.
    /// No raw C table pointer is cached by this context.
    runtime: &'runtime Runtime,
}

impl<'runtime> TypeRegistry<'runtime> {
    /// Creates a type-registry context borrowing the runtime facade.
    pub(super) const fn new(runtime: &'runtime Runtime) -> Self {
        Self { runtime }
    }

    /// Returns all registered class IDs.
    pub fn class_ids(self) -> Result<Vec<NcGuid>, NcStatus> {
        let api = self.runtime.type_registry_api();
        read_vec(
            || unsafe { (api.class_count)() },
            |out, capacity, out_len| unsafe { (api.class_ids)(out, capacity, out_len) },
        )
    }

    /// Returns all distinct registered interface ABI identities.
    pub fn interface_ids(self) -> Result<Vec<NcInterfaceId>, NcStatus> {
        let api = self.runtime.type_registry_api();
        read_vec(
            || unsafe { (api.interface_count)() },
            |out, capacity, out_len| unsafe { (api.interface_ids)(out, capacity, out_len) },
        )
    }
}
