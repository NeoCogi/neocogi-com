//! Runtime-owned index of registered classes and their relationships.
//!
//! One append-only state stores ordered class/interface identities and reverse
//! relationship maps for plugin-to-class, class-to-interface, and
//! interface-to-implementor queries. The enclosing runtime catalog lock keeps
//! every projection coherent.
//!
//! Ordered vectors preserve deterministic ABI enumeration while hash indexes
//! make identity and reverse-relationship lookups independent of catalog size.
//! Every vector and index is published in one catalog write transaction.

use std::collections::{HashMap, HashSet};
use std::vec::Vec;

use neocogi_com_core::{NcClassDescriptor, NcGuid, NcInterfaceId};

#[cfg(test)]
use neocogi_com_core::NcStatus;

use crate::loader::LoadedModuleId;

/// Append-only catalog of classes and interface relationships.
pub(crate) struct TypeRegistry {
    /// All ordered records and indexes published together.
    state: TypeRegistryState,
}

/// Append-only class, interface, and relationship data.
struct TypeRegistryState {
    /// Full class records in registration order.
    classes: Vec<RegisteredClass>,
    /// Class IDs in public enumeration order.
    class_ids: Vec<NcGuid>,
    /// Class ID-to-record index.
    class_by_id: HashMap<NcGuid, usize>,
    /// Distinct interface identities in first-seen order.
    interface_ids: Vec<NcInterfaceId>,
    /// Membership index backing ordered interface insertion.
    known_interfaces: HashSet<NcInterfaceId>,
    /// Ordered reverse edges from plugin ID to registered classes.
    class_ids_by_plugin: HashMap<NcGuid, Vec<NcGuid>>,
    /// Ordered reverse edges from interface identity to implementing classes.
    implementors_by_interface: HashMap<NcInterfaceId, Vec<NcGuid>>,
}

impl TypeRegistry {
    /// Creates an empty class/interface catalog and all supporting indexes.
    pub(crate) fn new() -> Self {
        Self {
            state: TypeRegistryState {
                classes: Vec::new(),
                class_ids: Vec::new(),
                class_by_id: HashMap::new(),
                interface_ids: Vec::new(),
                known_interfaces: HashSet::new(),
                class_ids_by_plugin: HashMap::new(),
                implementors_by_interface: HashMap::new(),
            },
        }
    }

    #[cfg(test)]
    /// Copies class IDs for direct registry tests.
    pub(crate) fn registered_class_ids(&self) -> Vec<NcGuid> {
        self.with_registered_class_ids(<[NcGuid]>::to_vec)
    }

    #[cfg(test)]
    /// Copies interface identities for direct registry tests.
    pub(crate) fn registered_interface_ids(&self) -> Result<Vec<NcInterfaceId>, NcStatus> {
        Ok(self.with_registered_interface_ids(<[NcInterfaceId]>::to_vec))
    }

    /// Borrows class IDs under the enclosing catalog read guard.
    pub(crate) fn with_registered_class_ids<R>(&self, read: impl FnOnce(&[NcGuid]) -> R) -> R {
        read(&self.state.class_ids)
    }

    /// Borrows distinct interface identities under the catalog read guard.
    pub(crate) fn with_registered_interface_ids<R>(&self, read: impl FnOnce(&[NcInterfaceId]) -> R) -> R {
        read(&self.state.interface_ids)
    }

    /// Borrows class IDs associated with a plugin, or an empty slice if absent.
    pub(crate) fn with_plugin_class_ids<R>(&self, plugin_id: NcGuid, read: impl FnOnce(&[NcGuid]) -> R) -> R {
        read(self.state.class_ids_by_plugin.get(&plugin_id).map_or(&[], Vec::as_slice))
    }

    /// Borrows interface identities associated with a class, or an empty slice.
    pub(crate) fn with_class_interface_ids<R>(&self, class_id: NcGuid, read: impl FnOnce(&[NcInterfaceId]) -> R) -> R {
        let interfaces = self
            .state
            .class_by_id
            .get(&class_id)
            .map(|index| self.state.classes[*index].interface_ids.as_slice())
            .unwrap_or(&[]);
        read(interfaces)
    }

    /// Borrows implementing class IDs for an interface, or an empty slice.
    pub(crate) fn with_interface_implementor_class_ids<R>(&self, interface_id: NcInterfaceId, read: impl FnOnce(&[NcGuid]) -> R) -> R {
        read(self.state.implementors_by_interface.get(&interface_id).map_or(&[], Vec::as_slice))
    }

    /// Looks up a copyable class descriptor/provenance projection.
    pub(crate) fn class(&self, class_id: NcGuid) -> Option<RegisteredClassView> {
        let class = &self.state.classes[*self.state.class_by_id.get(&class_id)?];
        Some(RegisteredClassView {
            plugin_id: class.plugin_id,
            module_id: class.module_id,
            class_id: class.class_id,
            class: class.class,
        })
    }

    /// Copies one class's interface identities for use after releasing the lock.
    pub(crate) fn class_interface_ids(&self, class_id: NcGuid) -> Option<Vec<NcInterfaceId>> {
        let class = &self.state.classes[*self.state.class_by_id.get(&class_id)?];
        Some(class.interface_ids.clone())
    }

    #[cfg(test)]
    /// Copies class IDs for a plugin in direct registry tests.
    pub(crate) fn plugin_class_ids(&self, plugin_id: NcGuid) -> Vec<NcGuid> {
        self.with_plugin_class_ids(plugin_id, <[NcGuid]>::to_vec)
    }

    #[cfg(test)]
    /// Copies implementing class IDs in direct registry tests.
    pub(crate) fn interface_implementor_class_ids(&self, interface_id: NcInterfaceId) -> Vec<NcGuid> {
        self.with_interface_implementor_class_ids(interface_id, <[NcGuid]>::to_vec)
    }

    /// Appends validated classes and updates all forward/reverse indexes.
    pub(crate) fn insert_classes(&mut self, classes: Vec<RegisteredClass>) {
        let state = &mut self.state;
        for class in classes {
            debug_assert!(!state.class_by_id.contains_key(&class.class_id));
            let class_index = state.classes.len();
            let class_id = class.class_id;
            let plugin_id = class.plugin_id;

            state.class_ids.push(class_id);
            state.class_by_id.insert(class_id, class_index);
            state.class_ids_by_plugin.entry(plugin_id).or_default().push(class_id);

            for interface_id in class.interface_ids.iter().copied() {
                if state.known_interfaces.insert(interface_id) {
                    state.interface_ids.push(interface_id);
                }
                state.implementors_by_interface.entry(interface_id).or_default().push(class_id);
            }
            state.classes.push(class);
        }
    }
}

#[derive(Clone, Copy)]
/// Copyable lookup projection over a registered class record.
pub(crate) struct RegisteredClassView {
    /// Plugin that owns the class.
    pub(crate) plugin_id: NcGuid,
    /// Dynamic module provenance, absent for static registration.
    pub(crate) module_id: Option<LoadedModuleId>,
    /// Stable class identity.
    pub(crate) class_id: NcGuid,
    /// Borrowed immutable class descriptor.
    pub(crate) class: *const NcClassDescriptor,
}

/// Runtime-owned indexes and borrowed descriptor pointer for one class.
pub(crate) struct RegisteredClass {
    // Plugin identity is copied so relationship queries do not need to walk the
    // plugin descriptor graph.
    pub(crate) plugin_id: NcGuid,
    // Static registrations have no dynamic module provenance.
    pub(crate) module_id: Option<LoadedModuleId>,
    // Stable lookup key copied into runtime-owned storage.
    pub(crate) class_id: NcGuid,
    // Borrowed immutable descriptor kept live by the caller or module registry.
    pub(crate) class: *const NcClassDescriptor,
    // Validated identities copied once during registration for indexed reads.
    pub(crate) interface_ids: Vec<NcInterfaceId>,
}

// SAFETY: Registered classes only store immutable descriptor pointers copied
// from validated plugin metadata. The exporting shared library must keep those
// descriptors live while registered, and the registry never mutates through
// the pointers.
unsafe impl Send for RegisteredClass {}
// SAFETY: Concurrent readers only inspect immutable descriptor metadata under
// the enclosing runtime catalog lock.
unsafe impl Sync for RegisteredClass {}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com_core::NcAbiSignature;
    use std::hint::black_box;
    use std::time::Instant;

    fn guid(index: u32) -> NcGuid {
        NcGuid::from_fields(index.wrapping_add(1), 0x5100, 0x4100, [0x80; 8])
    }

    fn interface(index: u32) -> NcInterfaceId {
        let bytes = std::format!("ncabi1;test-interface-{index:08x}").into_bytes().into_boxed_slice();
        NcInterfaceId::new(guid(index.wrapping_add(1_000_000)), NcAbiSignature::from_static(std::boxed::Box::leak(bytes)))
    }

    fn records(count: usize) -> Vec<RegisteredClass> {
        let shared = interface(u32::MAX - 1);
        (0..count)
            .map(|index| RegisteredClass {
                plugin_id: guid((index % 10) as u32 + 500_000),
                module_id: None,
                class_id: guid(index as u32),
                class: core::ptr::null(),
                interface_ids: vec![shared, interface(index as u32)],
            })
            .collect()
    }

    #[test]
    fn indexes_preserve_registration_and_relationship_order() {
        let mut registry = TypeRegistry::new();
        registry.insert_classes(records(12));

        assert_eq!(registry.registered_class_ids(), (0..12).map(guid).collect::<Vec<_>>());
        assert_eq!(registry.class(guid(7)).unwrap().class_id, guid(7));
        assert_eq!(registry.plugin_class_ids(guid(500_003)), vec![guid(3)],);
        assert_eq!(
            registry.interface_implementor_class_ids(interface(u32::MAX - 1)),
            (0..12).map(guid).collect::<Vec<_>>(),
        );

        let interfaces = registry.registered_interface_ids().unwrap();
        assert_eq!(interfaces[0], interface(u32::MAX - 1));
        assert_eq!(interfaces[1..], (0..12).map(interface).collect::<Vec<_>>());
    }

    #[test]
    #[ignore = "manual registry scaling benchmark; run scripts/bench-registry.sh"]
    fn registry_scaling_benchmark() {
        for count in [10_usize, 1_000, 10_000] {
            let mut registry = TypeRegistry::new();
            let start = Instant::now();
            registry.insert_classes(records(count));
            let insert = start.elapsed();

            let rounds = (100_000 / count).max(10);
            let start = Instant::now();
            for _ in 0..rounds {
                for index in 0..count {
                    black_box(registry.class(black_box(guid(index as u32))));
                }
            }
            let lookups = count * rounds;
            let lookup = start.elapsed();

            let shared = interface(u32::MAX - 1);
            let start = Instant::now();
            for _ in 0..rounds {
                registry.with_interface_implementor_class_ids(shared, |values| black_box(values.len()));
                registry.with_registered_interface_ids(|values| black_box(values.len()));
            }
            let relationships = start.elapsed();
            let relationship_reads = rounds * 2;

            println!(
                "classes={count:>5} insert={insert:?} lookup_total={lookup:?} lookup_ns={:.1} relationship_total={relationships:?} relationship_ns={:.1}",
                lookup.as_nanos() as f64 / lookups as f64,
                relationships.as_nanos() as f64 / relationship_reads as f64,
            );
        }
    }
}
