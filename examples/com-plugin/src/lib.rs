//! One plugin implementation built as either a shared library or a linked Rust library.

use neocogi_com_serialization::ncb as binary;
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};
extern crate self as neocogi_example_com_plugin;
/// Independently generated bindings from the common example IDL.
pub mod contracts;
use neocogi_com_sdk::{
    ComBox, ComMut, ComPtr, ComRef, ComStringExt, INcStringAbi, NcString, NcValueRef, IUnknown, NcBytes, INcBytesAbi, NcGuid, NcStatus, NC_E_BUSY,
    NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE, com_object,
};
#[path = "../../com-host/support/extent_state.rs"]
mod extent_state;
use extent_state::{ExtentState, PERSISTENT_KIND};

use crate::contracts::{
    ExampleFactory, ExampleObject, ExampleObjectAbi, Extent, ExtentReport, ICompositionFactory, IEditableWidget, IEditableWidgetAbi, IMeasured,
    IMountedComposition, IMountedCompositionAbi,
};

/// Private mutable object constructed only through `ExampleFactory`.
#[com_object(interfaces(crate::contracts::ExampleObject, neocogi_com_sdk::persistence::IPersistable))]
struct DynamicExampleObject {
    /// Dimensions owned entirely by this COM object allocation.
    extent: Cell<Extent>,
    /// Optional concrete destination, excluded from the persistent state.
    changed: OnceCell<crate::contracts::notifications::ExtentDelivery>,
    /// Caption owned by the provider, excluded from persistent dimensions.
    caption: RefCell<Option<ComPtr<INcStringAbi>>>,
}

impl IMeasured for DynamicExampleObject {
    /// Returns the current dimensions by ABI-safe value copy.
    fn extent(&self) -> Result<Extent, NcStatus> {
        Ok(self.extent.get())
    }
    /// Writes only fresh copied records into caller storage, without provider scratch.
    fn reports(&self, mut output: neocogi_com_sdk::NcOutputWriter<'_, ExtentReport>) -> Result<u32, NcStatus> {
        if output.capacity() >= 2 {
            output.extend_from_slice(&[ExtentReport::Current { extent: self.extent.get(), usable: true }, ExtentReport::Empty])?;
        }
        Ok(2)
    }
}

impl ExampleObject for DynamicExampleObject {
    /// Keeps the caption field borrowed while the caller uses its lender.
    fn caption<R>(&self, lend: impl for<'loan> FnOnce(Result<Option<ComRef<'loan, INcStringAbi>>, NcStatus>) -> R) -> R {
        match self.caption.try_borrow() {
            Ok(field) => lend(Ok(field.as_ref().map(ComPtr::as_com_ref))),
            Err(_) => lend(Err(NC_E_BUSY)),
        }
    }

    /// Copies a caption through the read-only interface into private storage.
    fn set_caption(&self, caption: Option<ComRef<'_, INcStringAbi>>) -> Result<(), NcStatus> {
        let caption = caption.map(|text| text.to_string()).transpose()?.map(|text| NcString::from_utf8_str(&text));
        let old = std::mem::replace(&mut *self.caption.try_borrow_mut().map_err(|_| NC_E_BUSY)?, caption);
        drop(old);
        Ok(())
    }

    /// Commits dimensions before invoking the observer.
    fn resize(&self, extent: Extent) -> Result<(), NcStatus> {
        self.extent.set(extent);
        self.changed.get().expect("factory connected delivery before publication").changed(extent)
    }
}

/// Shared local root of the two typed construction interfaces.
#[com_object(interfaces(crate::contracts::ExampleFactory, crate::contracts::ICompositionFactory))]
struct DynamicExampleFactory {
    /// The factory and each widget retain this count independently.
    destroyed_widgets: Rc<Cell<u32>>,
}

impl ExampleFactory for DynamicExampleFactory {
    /// Constructs one typed object without consulting a runtime class table.
    fn create(&self, extent: Extent, events: NcValueRef<'_, crate::contracts::ExtentBinding>) -> Result<ComPtr<ExampleObjectAbi>, NcStatus> {
        let binding = events.to_owned()?;
        let local = DynamicExampleObject::into_local(DynamicExampleObject {
            extent: Cell::new(extent),
            changed: OnceCell::new(),
            caption: RefCell::new(None),
        });
        // Obtain identity from initialized storage and connect delivery before
        // returning the first application-facing object capability.
        let origin = local.query::<ExampleObjectAbi>()?.downgrade()?;
        assert!(
            local
                .get()
                .changed
                .set(crate::contracts::notifications::ExtentDelivery::new(binding, origin))
                .is_ok(),
            "new object has no previous delivery"
        );
        local.query()
    }
}

impl ICompositionFactory for DynamicExampleFactory {
    /// Creates the child, moves it into the mount and transfers the mount.
    fn composition(&self) -> Result<ComBox<IMountedCompositionAbi>, NcStatus> {
        let child = EditableWidget::into_box(EditableWidget {
            extent: Extent { width: 8, height: 6 },
            destroyed_widgets: Rc::clone(&self.destroyed_widgets),
        })
        .query()
        .map_err(|(status, _owner)| status)?;
        MountedComposition::into_box(MountedComposition { child })
            .query()
            .map_err(|(status, _owner)| status)
    }

    fn destroyed_widgets(&self) -> u32 {
        self.destroyed_widgets.get()
    }
}

/// A uniquely owned mount stores one uniquely owned interface.
#[com_object(crate::contracts::IMountedComposition, ownership = exclusive)]
struct MountedComposition {
    /// Lends the child for the lifetime of either receiver borrow.
    child: ComBox<IEditableWidgetAbi>,
}

impl IMountedComposition for MountedComposition {
    fn widget(&self) -> ComRef<'_, IEditableWidgetAbi> {
        self.child.as_com_ref()
    }

    fn widget_mut(&mut self) -> ComMut<'_, IEditableWidgetAbi> {
        self.child.as_com_mut()
    }
}

/// Exclusive borrowing permits plain mutable dimensions.
#[com_object(crate::contracts::IEditableWidget, ownership = exclusive)]
struct EditableWidget {
    /// Dimensions managed by the widget's sole owner.
    extent: Extent,
    /// Shared with the factory so the host can observe provider-side drop.
    destroyed_widgets: Rc<Cell<u32>>,
}

impl IEditableWidget for EditableWidget {
    fn extent(&self) -> Extent {
        self.extent
    }

    fn set_extent(&mut self, extent: Extent) {
        self.extent = extent;
    }
}

impl Drop for EditableWidget {
    fn drop(&mut self) {
        self.destroyed_widgets.set(self.destroyed_widgets.get().wrapping_add(1));
    }
}

/// Acquires one owned root; object construction remains on its typed methods.
#[neocogi_com_sdk::plugin(name = "nc.examples.factory_plugin_component")]
fn acquire_root(_runtime: ::neocogi_com_sdk::RuntimeHandle) -> Result<neocogi_com_sdk::ComPtr<crate::contracts::ExampleFactoryAbi>, neocogi_com_sdk::NcStatus> {
    DynamicExampleFactory::from_value(DynamicExampleFactory { destroyed_widgets: Rc::new(Cell::new(0)) }).query()
}

// Only the final DSO exports this symbol; monoliths call the Rust getter.

impl neocogi_com_sdk::persistence::IPersistable for DynamicExampleObject {
    /// Returns a recipe identity independent from ABI revisions.
    fn type_id(&self) -> Result<NcGuid, NcStatus> {
        Ok(PERSISTENT_KIND)
    }
    /// The persistent record has one explicitly supported revision.
    fn schema_version(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }
    /// Snapshot captures current dimensions through the shared typed codec.
    fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
        let extent = self.extent.get();
        let state = ExtentState {
            width: extent.width,
            height: extent.height,
        };
        Ok(NcBytes::from_vec(binary::to_vec(&state).map_err(|_| NC_E_INVALID_ARG)?))
    }
    /// This ordinary object is a leaf in the persistent tree.
    fn child_count(&self) -> Result<u32, NcStatus> {
        Ok(0)
    }
    /// A leaf has no valid child indices.
    fn child(&self, _: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
        Err(NC_E_OUT_OF_RANGE)
    }
}
