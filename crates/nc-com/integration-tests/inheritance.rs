//! Single-base layout, identity and ownership checks across generated interfaces.

use nc_com::{
    com_object, com_interface, ComInterface, ComPtr, ComRef, IUnknown, NcStatus, NC_E_NOINTERFACE,
};
use std::{cell::Cell, rc::Rc};

/// A separate module exercises fully qualified base companion paths.
mod contract {
    use super::*;

    #[com_interface(auto_guid = "nc.tests.inheritance.Widget:v1")]
    /// Common widget contract, including a recursive return to its derived type.
    pub trait IWidget {
        /// Observes the shared concrete state.
        fn value(&self) -> Result<u32, NcStatus>;
        /// Returns another button without making ABI hashes recursively expand.
        fn button(&self) -> Result<ComPtr<super::IButtonAbi>, NcStatus>;
    }
}

#[com_interface(auto_guid = "nc.tests.inheritance.Button:v1")]
/// Adds one slot after the complete widget prefix.
pub trait IButton: contract::IWidget {
    /// Changes state observed through all projections.
    fn set_value(&self, value: u32) -> Result<(), NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.inheritance.Toggle:v1")]
/// A second level verifies recursive vtables and inherited client lookup.
pub trait IToggle: IButton {
    /// Copies a base value through a checked borrowed argument.
    fn copy_value(&self, source: ComRef<'_, contract::IWidgetAbi>) -> Result<(), NcStatus>;
}

#[com_object(
    class_id(auto_guid = "nc.tests.inheritance.Toggle"),
    name = "nc.tests.inheritance.Toggle",
    version = "0.1.0",
    constructor(args = none, default),
    interfaces(IToggle, contract::IWidget)
)]
#[derive(Default)]
/// Explicitly repeating an inherited base exercises metadata deduplication.
struct Toggle {
    /// Authoritative state shared by every COM projection.
    value: Cell<u32>,
    /// Observation retained after the one allocation is finally released.
    drops: Rc<Cell<u32>>,
}

impl Drop for Toggle {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

impl contract::IWidget for Toggle {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value.get())
    }
    fn button(&self) -> Result<ComPtr<IButtonAbi>, NcStatus> {
        Toggle::from_value(Toggle {
            value: Cell::new(self.value.get()),
            drops: Rc::new(Cell::new(0)),
        })
        .query()
    }
}

impl IButton for Toggle {
    fn set_value(&self, value: u32) -> Result<(), NcStatus> {
        self.value.set(value);
        Ok(())
    }
}

impl IToggle for Toggle {
    fn copy_value(&self, source: ComRef<'_, contract::IWidgetAbi>) -> Result<(), NcStatus> {
        self.value.set(source.value()?);
        Ok(())
    }
}

#[test]
fn inherited_methods_and_queries_share_one_allocation() {
    let drops = Rc::new(Cell::new(0));
    let unknown = Toggle::from_value(Toggle {
        value: Cell::new(7),
        drops: drops.clone(),
    });
    let toggle = unknown.query::<IToggleAbi>().unwrap();
    let button = toggle.query::<IButtonAbi>().unwrap();
    let widget = toggle.query::<contract::IWidgetAbi>().unwrap();
    assert_eq!(toggle.value(), Ok(7));
    toggle.as_com_ref().set_value(19).unwrap();
    assert_eq!(widget.value(), Ok(19));
    assert_eq!(button.value(), Ok(19));
    toggle.copy_value(widget.as_com_ref()).unwrap();
    assert_eq!(toggle.button().unwrap().value(), Ok(19));
    assert_eq!(
        widget.query::<IUnknown>().unwrap().as_mut_ptr(),
        unknown.as_mut_ptr()
    );
    assert_eq!(
        button.query::<IUnknown>().unwrap().as_mut_ptr(),
        unknown.as_mut_ptr()
    );
    assert_eq!(
        widget.as_mut_ptr(),
        button.query::<contract::IWidgetAbi>().unwrap().as_mut_ptr()
    );
    assert_eq!(Toggle::COM_INTERFACE_COUNT, 4);
    // Independent physical base projections advertise their own exact identity.
    unsafe {
        nc_com::raw::validate_interface::<IToggleAbi>(toggle.as_mut_ptr()).unwrap();
        nc_com::raw::validate_interface::<contract::IWidgetAbi>(widget.as_mut_ptr()).unwrap();
        assert_eq!(
            nc_com::raw::validate_interface::<contract::IWidgetAbi>(toggle.as_mut_ptr().cast()),
            Err(NC_E_NOINTERFACE)
        );
    }
    drop((unknown, toggle, button));
    assert_eq!(drops.get(), 0);
    drop(widget);
    assert_eq!(drops.get(), 1);
}

#[test]
fn inherited_vtable_is_a_physical_prefix_and_hashes_the_base() {
    assert_eq!(std::mem::offset_of!(IButtonVTable, base), 0);
    assert_eq!(
        std::mem::offset_of!(IButtonVTable, set_value),
        std::mem::size_of::<contract::IWidgetVTable>()
    );
    assert_eq!(
        std::mem::offset_of!(IToggleVTable, copy_value),
        std::mem::size_of::<IButtonVTable>()
    );
    assert_eq!(
        IButtonAbi::INTERFACE_DESCRIPTOR.base,
        contract::IWidgetAbi::INTERFACE_ID
    );
    assert_eq!(
        IToggleAbi::INTERFACE_DESCRIPTOR.base,
        IButtonAbi::INTERFACE_ID
    );
    assert_eq!(IToggleAbi::INTERFACE_CHAIN.len(), 3);
}

// The exported table includes all inherited contracts even though only the leaf
// needs to be named by an ordinary class declaration.
nc_com::plugin! {
    plugin_id(auto_guid = "nc.tests.inheritance.Plugin:v1"),
    name = "nc.tests.inheritance.Plugin:v1",
    version = "0.1.0",
    classes = [Toggle],
}

#[test]
fn descriptor_validation_requires_matching_acyclic_base_contracts() {
    use nc_com::{NcClassInterfaceDescriptor, NcClassTable, NcDescriptorValidationKind, NC_OK};
    // SAFETY: The generated plugin and every nested table are immutable statics.
    assert_eq!(
        unsafe { nc_com::validate_plugin_descriptor_with_issues(plugin_descriptor(), |_| {}) },
        NC_OK
    );
    let mut contracts: Vec<_> = Toggle::COM_INTERFACES
        .iter()
        .map(|entry| {
            // SAFETY: Generated class metadata contains readable static descriptors.
            unsafe { *entry.interface_descriptor }
        })
        .collect();
    // The table is IUnknown, Toggle, Button, Widget. Exercise a missing base,
    // a mismatched full base identity, and a cycle without following raw links.
    for case in 0..3 {
        let original = contracts[2].base;
        contracts[2].base = match case {
            0 => nc_com::NcInterfaceId::new(
                nc_com::guid_from_qualified_name("nc.tests.inheritance.Missing:v1"),
                original.abi_hash,
            ),
            1 => nc_com::NcInterfaceId::new(original.iid, IButtonAbi::ABI_HASH),
            _ => IToggleAbi::INTERFACE_ID,
        };
        let entries: Vec<_> = contracts
            .iter()
            .map(|descriptor| NcClassInterfaceDescriptor { interface_descriptor: descriptor })
            .collect();
        let mut class = Toggle::CLASS_DESCRIPTOR;
        class.interfaces = entries.as_ptr();
        let table = NcClassTable { class_count: 1, classes: &class };
        let mut plugin = *plugin_descriptor();
        plugin.class_table = &table;
        let mut found = false;
        // SAFETY: All local descriptor arrays remain readable through validation.
        let status = unsafe {
            nc_com::validate_plugin_descriptor_with_issues(&plugin, |issue| {
                found |= issue.kind == NcDescriptorValidationKind::InvalidInterfaceBase;
            })
        };
        assert!(status.is_err());
        assert!(found);
        contracts[2].base = original;
    }
}
