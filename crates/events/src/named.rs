//! Named setup for opaque components, with direct typed delivery afterward.

use std::{collections::BTreeMap, sync::mpsc::Sender};
use neocogi_com::{ComEvent, ComOutputInterface, ComPtr, ComRef, IUnknown, NcCopyFromBufferOffset, NcStatus, NC_E_INVALID_ARG, NC_E_NOT_FOUND};
use neocogi_com_events_contracts::{INamedOutputs, INamedOutputsAbi};

/// A single setup function; it is never invoked during event delivery.
type Install = fn(ComRef<'_, IUnknown>, &str, ComRef<'_, IUnknown>) -> Result<(), NcStatus>;

/// One exported name maps to a private producer and one of its declared outputs.
struct Export {
    /// Keeps the producer alive while its component advertises the export.
    source: ComPtr<IUnknown>,
    /// Internal output name, validated when the export is registered.
    port: String,
    /// Statically generated typed installation, selected by the producer contract.
    install: Install,
}

/// Provider-owned export table; no event payload or application state lives here.
/// A component can own this as an ordinary field and delegate INamedOutputs.
#[derive(Default)]
pub struct NamedOutputs {
    /// Exact public names; duplicate registration cannot replace an existing export.
    exports: BTreeMap<String, Export>,
}
impl NamedOutputs {
    /// Starts a component with no public event exports.
    pub fn new() -> Self {
        Self::default()
    }
    /// Registers a private producer output under a public, layout-independent name.
    /// Inherited outputs are registered through their declaring base projection.
    pub fn export<T: ComOutputInterface>(&mut self, name: &str, source: ComPtr<T>, port: &str) -> Result<(), NcStatus> {
        if name.is_empty() {
            return Err(NC_E_INVALID_ARG);
        }
        T::output_receiver(port)?;
        let source = source.query::<IUnknown>()?;
        match self.exports.entry(name.into()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Export {
                    source,
                    port: port.into(),
                    install: install::<T>,
                });
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => Err(NC_E_INVALID_ARG),
        }
    }
    /// Performs the only public-name lookup; events bypass this table entirely.
    pub fn bind(&mut self, name: &str, receiver: ComRef<'_, IUnknown>) -> Result<(), NcStatus> {
        let export = self.exports.get(name).ok_or(NC_E_NOT_FOUND)?;
        (export.install)(export.source.as_com_ref(), &export.port, receiver)
    }
    /// Serves the generated COM operation while rejecting nonzero offsets and invalid text.
    pub fn bind_com(&mut self, name: NcCopyFromBufferOffset<'_, u8>, receiver: ComRef<'_, IUnknown>) -> Result<(), NcStatus> {
        if name.offset() != 0 {
            return Err(NC_E_INVALID_ARG);
        }
        let name = std::str::from_utf8(name.as_slice()).map_err(|_| NC_E_INVALID_ARG)?;
        self.bind(name, receiver)
    }
    /// Publishes a standalone binding capability when a provider needs no custom root.
    pub fn into_com(self) -> Result<ComPtr<INamedOutputsAbi>, NcStatus> {
        OutputObject::from_value(OutputObject { outputs: self }).query()
    }
}

/// Installs through exact source and receiver interfaces, never an unchecked cast.
fn install<T: ComOutputInterface>(source: ComRef<'_, IUnknown>, name: &str, receiver: ComRef<'_, IUnknown>) -> Result<(), NcStatus> {
    T::bind_output(source.add_ref().query::<T>()?.as_com_ref(), name, receiver)
}

/// Optional standalone owner of a completed export table.
#[neocogi_com::com_object(interfaces(neocogi_com_events_contracts::INamedOutputs))]
struct OutputObject {
    /// Ordinary exclusive state used only during binding calls.
    outputs: NamedOutputs,
}
impl INamedOutputs for OutputObject {
    /// Delegates to the same provider helper used by richer component roots.
    fn bind_output(&mut self, name: NcCopyFromBufferOffset<'_, u8>, receiver: ComRef<'_, IUnknown>) -> Result<(), NcStatus> {
        self.outputs.bind_com(name, receiver)
    }
}

/// Binds a typed event from an opaque component into an application channel.
///
/// E is known to the application; the source widget type and location are not.
/// The mapper/receiver allocate during setup. Delivery uses the existing typed
/// post and channel path, with no repeated name lookup or extra relay object.
pub fn bind_output<E: ComEvent, M: 'static>(
    outputs: ComRef<'_, INamedOutputsAbi>,
    name: &str,
    sender: Sender<M>,
    translate: impl FnMut(E) -> M + 'static,
) -> Result<(), NcStatus> {
    crate::map(
        |consume| {
            let receiver = E::make_receiver(consume)?;
            outputs
                .add_ref()
                .bind_output(NcCopyFromBufferOffset::new(0, name.as_bytes()), receiver.as_com_ref())
        },
        sender,
        translate,
    )
}
