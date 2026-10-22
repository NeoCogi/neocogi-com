//! Domain-specific construction-bound observation, without a generic event system.
use crate::{ExampleObjectAbi, Extent, ExtentBinding, ExtentNotification, IExtentObserverAbi};
use neocogi_com::{ComPtr, ComRef, NcStatus, NcValueRef, WeakComPtr};

impl ExtentBinding {
    /// Retains the selected shared observer; exclusive borrows cannot grant shared ownership.
    pub fn from_observer(observer: Option<ComRef<'_, IExtentObserverAbi>>) -> Result<Self, NcStatus> {
        Ok(match observer {
            None => Self::Unobserved,
            Some(observer) => Self::Observed(observer.try_add_ref()?),
        })
    }
}

/// One factory-selected observer and its matching weak producer identity.
pub enum ExtentDelivery {
    /// Updates remain ordinary object mutations with no notification allocation.
    Unobserved,
    /// The observer may outlive the producer; its identity owns no producer value.
    Observed {
        /// Shared destination retained at construction.
        observer: ComPtr<IExtentObserverAbi>,
        /// Exact concrete source capability; not a generic root projection.
        source: WeakComPtr<ExampleObjectAbi>,
    },
}
impl ExtentDelivery {
    /// Connects delivery only while the factory still owns unpublished initialized storage.
    pub fn new(binding: ExtentBinding, source: WeakComPtr<ExampleObjectAbi>) -> Self {
        match binding {
            ExtentBinding::Unobserved => Self::Unobserved,
            ExtentBinding::Observed(observer) => Self::Observed { observer, source },
        }
    }
    /// Sends a copied domain fact after the caller commits the new dimensions.
    /// Receiver errors propagate without rolling back producer state.
    pub fn changed(&mut self, extent: Extent) -> Result<(), NcStatus> {
        match self {
            Self::Unobserved => Ok(()),
            Self::Observed { observer, source } => {
                let notification = ExtentNotification { source: source.clone(), extent };
                observer.changed(NcValueRef::new(&notification)?)
            }
        }
    }
}
