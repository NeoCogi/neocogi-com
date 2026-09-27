//! Single-base layout, identity and ownership checks across generated interfaces.

use neocogi_com::{com_interface, com_object, ComInterface, ComPtr, ComRef, IUnknown, NcStatus, NC_E_NOINTERFACE};
use std::{cell::Cell, rc::Rc};

/// A separate module exercises fully qualified base companion paths.
mod contract {
    use super::*;

    #[com_interface(auto_guid = "nc.tests.inheritance.Widget:v1", assigned_signature = b"00000000000000000000000000000007")]
    /// Common widget contract, including a recursive return to its derived type.
    pub trait IWidget {
        /// Observes the shared concrete state.
        fn value(&self) -> Result<u32, NcStatus>;
        /// Returns another button without making ABI signatures recursively expand.
        fn button(&self) -> Result<ComPtr<super::IButtonAbi>, NcStatus>;
    }
}

#[com_interface(auto_guid = "nc.tests.inheritance.Audit:v1", assigned_signature = b"00000000000000000000000000000016")]
/// Required sibling projection inherited transitively by the toggle contract.
pub trait IAudit {
    /// Observes whether the allocation's authoritative value is nonzero.
    fn active(&self) -> Result<bool, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.inheritance.Button:v1", assigned_signature = b"00000000000000000000000000000008")]
/// Adds one slot after the complete widget prefix.
pub trait IButton: contract::IWidget + IAudit {
    /// Changes state observed through all projections.
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.inheritance.Toggle:v1", assigned_signature = b"00000000000000000000000000000009")]
/// A second level verifies recursive vtables and inherited client lookup.
pub trait IToggle: IButton {
    /// Copies a base value through a checked borrowed argument.
    fn copy_value(&mut self, source: ComRef<'_, contract::IWidgetAbi>) -> Result<(), NcStatus>;
}

#[com_object(interfaces(IToggle))]
#[derive(Default)]
/// Concrete storage exposing the complete inherited interface chain.
struct Toggle {
    /// Authoritative state shared by every COM projection.
    value: u32,
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
        Ok(self.value)
    }
    fn button(&self) -> Result<ComPtr<IButtonAbi>, NcStatus> {
        Toggle::from_value(Toggle {
            value: self.value,
            drops: Rc::new(Cell::new(0)),
        })
        .query()
    }
}

impl IAudit for Toggle {
    /// Reads the same implementation value through the sibling header.
    fn active(&self) -> Result<bool, NcStatus> {
        Ok(self.value != 0)
    }
}

impl IButton for Toggle {
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus> {
        self.value = value;
        Ok(())
    }
}

impl IToggle for Toggle {
    fn copy_value(&mut self, source: ComRef<'_, contract::IWidgetAbi>) -> Result<(), NcStatus> {
        self.value = source.value()?;
        Ok(())
    }
}

#[test]
fn inherited_methods_and_queries_share_one_allocation() {
    let drops = Rc::new(Cell::new(0));
    let unknown = Toggle::from_value(Toggle { value: 7, drops: drops.clone() });
    let mut toggle = unknown.query::<IToggleAbi>().unwrap();
    let button = toggle.query::<IButtonAbi>().unwrap();
    let widget = toggle.query::<contract::IWidgetAbi>().unwrap();
    assert_eq!(toggle.value(), Ok(7));
    toggle.set_value(19).unwrap();
    assert_eq!(widget.value(), Ok(19));
    assert_eq!(button.value(), Ok(19));
    let source = Toggle::from_value(Toggle { value: 23, drops: Rc::new(Cell::new(0)) })
        .query::<contract::IWidgetAbi>()
        .unwrap();
    toggle.copy_value(source.as_com_ref()).unwrap();
    assert_eq!(toggle.button().unwrap().value(), Ok(23));
    assert_eq!(widget.query::<IUnknown>().unwrap().as_mut_ptr(), unknown.as_mut_ptr());
    assert_eq!(button.query::<IUnknown>().unwrap().as_mut_ptr(), unknown.as_mut_ptr());
    assert_eq!(widget.as_mut_ptr(), button.query::<contract::IWidgetAbi>().unwrap().as_mut_ptr());
    assert_eq!(Toggle::COM_INTERFACE_COUNT, 5);
    let audit = toggle.query::<IAuditAbi>().unwrap();
    assert_eq!(audit.active(), Ok(true));
    assert_eq!(audit.query::<IUnknown>().unwrap().as_mut_ptr(), unknown.as_mut_ptr());
    drop(audit);
    // Independent physical base projections advertise their own exact identity.
    unsafe {
        neocogi_com::raw::validate_interface::<IToggleAbi>(toggle.as_mut_ptr()).unwrap();
        neocogi_com::raw::validate_interface::<contract::IWidgetAbi>(widget.as_mut_ptr()).unwrap();
        assert_eq!(
            neocogi_com::raw::validate_interface::<contract::IWidgetAbi>(toggle.as_mut_ptr().cast()),
            Err(NC_E_NOINTERFACE)
        );
    }
    drop((unknown, toggle, button));
    assert_eq!(drops.get(), 0);
    drop(widget);
    assert_eq!(drops.get(), 1);
}

#[test]
fn inherited_vtable_is_a_physical_prefix_and_signs_the_base() {
    assert_eq!(std::mem::offset_of!(IButtonVTable, base), 0);
    assert_eq!(std::mem::offset_of!(IButtonVTable, set_value), std::mem::size_of::<contract::IWidgetVTable>());
    assert_eq!(std::mem::offset_of!(IToggleVTable, copy_value), std::mem::size_of::<IButtonVTable>());
    assert_eq!(IButtonAbi::INTERFACE_DESCRIPTOR.base, contract::IWidgetAbi::INTERFACE_ID);
    assert_eq!(IToggleAbi::INTERFACE_DESCRIPTOR.base, IButtonAbi::INTERFACE_ID);
    assert_eq!(IToggleAbi::INTERFACE_CHAIN.len(), 4);
}

#[path = "fixtures/diamond.rs"]
mod diamond;

/// Each physical header must lead back to the same concrete allocation, including
/// inherited calls whose static method type differs from the actual projection.
#[test]
fn single_pointer_headers_recover_one_owner_from_distinct_offsets() {
    use neocogi_com::{ComAbiObject, __nc_object_from_interface};
    assert_eq!(core::mem::size_of::<IUnknown>(), core::mem::size_of::<usize>());
    assert_eq!(core::mem::size_of::<IToggleAbi>(), core::mem::size_of::<usize>());
    assert_eq!(core::mem::size_of::<ComPtr<IToggleAbi>>(), core::mem::size_of::<usize>());
    assert_eq!(core::mem::size_of::<ComRef<'_, IToggleAbi>>(), core::mem::size_of::<usize>());
    let owner = Toggle::from_value(Toggle { value: 41, drops: Rc::new(Cell::new(0)) });
    let toggle = owner.query::<IToggleAbi>().unwrap();
    let button = owner.query::<IButtonAbi>().unwrap();
    let widget = owner.query::<contract::IWidgetAbi>().unwrap();
    let audit = owner.query::<IAuditAbi>().unwrap();
    let projections = [
        owner.as_mut_ptr(),
        toggle.as_mut_ptr().cast(),
        button.as_mut_ptr().cast(),
        widget.as_mut_ptr().cast(),
        audit.as_mut_ptr().cast(),
    ];
    // SAFETY: canonical IUnknown is interfaces[0] in this known generated storage.
    let allocation = unsafe { owner.as_mut_ptr().cast::<u8>().sub(core::mem::offset_of!(ToggleComObject, interfaces)) };
    for (header, layout) in projections.into_iter().zip(Toggle::ABI_HEADERS) {
        // SAFETY: all acquired references retain the same live allocation, and
        // each vtable is the immutable table installed by the generated constructor.
        unsafe {
            let prefix = &*(*header).vtable;
            assert_eq!(prefix.object_offset, header.cast::<u8>().offset_from(allocation) as usize);
            assert_eq!(prefix.object_offset, layout.object_offset);
            assert_eq!(*prefix.interface_id, layout.identity);
            assert_eq!(__nc_object_from_interface(header), allocation.cast());
        }
    }
    // The same base slot executes through three different physical headers.
    assert_eq!(toggle.value(), Ok(41));
    assert_eq!(button.value(), Ok(41));
    assert_eq!(widget.value(), Ok(41));
}
