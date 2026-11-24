//! Persistence implementation owned by the runtime in every linkage mode.
//!
//! Registries are explicitly allocated by callers, not installed in global
//! plugin state. Each traversal uses ordinary local mutable budgets and retains
//! COM ownership until it either returns a complete tree or cleans up failure.

mod children;
mod factory_registry;
mod save_object_tree;

pub(crate) use factory_registry::FactoryRegistry;
pub(crate) use save_object_tree::save_object_tree_with_limits;

use neocogi_com_sdk::{ComPtr, NcCopyFromBufferOffset, NcStatus, guid_from_qualified_name};
use neocogi_com_sdk::serialization::{Format, ISerializationPluginAbi, Limits};

/// Strong capabilities selected once for one synchronous persistence operation.
/// The format and its root stay alive through traversal and recipe callbacks;
/// their code/metadata are process-pinned by this host's catalog registration.
struct NcbFormat {
    /// Retained format factory. Each serialization or deserialization uses its own session.
    format: Format,
    /// Keep the acquiring provider alive until after the format is released.
    _provider: ComPtr<ISerializationPluginAbi>,
}

/// Selects named NCB through this implementation's own admitted service tables.
/// Catalog lookup releases its lock before bootstrap or format callbacks run.
/// Missing providers and format-selection failures propagate unchanged; no
/// default selection, implicit registration, loading or embedded fallback exists.
fn ncb_format(limits: Limits) -> Result<NcbFormat, NcStatus> {
    let provider = crate::initialize_runtime().acquire_plugin_root::<ISerializationPluginAbi>(guid_from_qualified_name("nc.serialization"))?;
    // Persistence accepts usize policy ceilings, while the existing format ABI
    // admits u32 container/depth ceilings. NCB-v1 container counts are u32;
    // saturate permissive ceilings rather than rejecting an otherwise small
    // document merely because its caller allowed more than the ABI can express.
    let limits = Limits {
        elements: limits.elements.min(u32::MAX as usize),
        depth: limits.depth.min(u32::MAX as usize),
        ..limits
    };
    let format = Format::new(provider.format(NcCopyFromBufferOffset::new(0, b"ncb"))?, limits);
    Ok(NcbFormat { format, _provider: provider })
}

#[cfg(all(test, not(miri)))]
mod tests;
