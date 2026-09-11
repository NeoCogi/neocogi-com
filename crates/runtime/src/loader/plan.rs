//! Deterministic planning for explicit hard plugin dependencies.
//!
//! Given already inspected artifacts, the planner validates one-to-one plugin
//! ID/name mappings, reports missing dependencies, and performs a stable
//! topological sort. Input order breaks ties so repeated planning produces the
//! same dependency-first output without filesystem discovery policy.

use core::fmt;
use std::collections::HashMap;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{NcAbiValidationMode, NcGuid};

use super::PluginInspection;

/// A collection-level error that prevents construction of a complete plugin
/// activation order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginDependencyPlanError {
    /// Two selected plugins use different ABI signature representations.
    ValidationModeMismatch {
        /// Deterministic reference plugin with the lowest GUID.
        expected_plugin_id: NcGuid,
        /// ABI validation mode declared by the reference plugin.
        expected: NcAbiValidationMode,
        /// First differently configured plugin in GUID order.
        actual_plugin_id: NcGuid,
        /// ABI validation mode declared by the differing plugin.
        actual: NcAbiValidationMode,
    },
    /// Two inspected identities disagree about the GUID/name mapping.
    IdentityMappingConflict {
        /// GUID of the first conflicting identity.
        first_id: NcGuid,
        /// Qualified name of the first conflicting identity.
        first_name: String,
        /// GUID of the second conflicting identity.
        second_id: NcGuid,
        /// Qualified name of the second conflicting identity.
        second_name: String,
    },
    /// Two inspected artifacts claim the same stable plugin identity.
    DuplicatePluginId {
        /// Plugin GUID claimed by more than one artifact.
        plugin_id: NcGuid,
    },
    /// A declared hard dependency is absent from the inspected artifact set.
    MissingDependency {
        /// Plugin whose dependency cannot be satisfied.
        plugin_id: NcGuid,
        /// Required plugin GUID absent from the input set.
        dependency_id: NcGuid,
    },
    /// The remaining plugin IDs form one or more dependency cycles.
    DependencyCycle {
        /// Sorted plugin GUIDs participating in, or blocked by, the cycles.
        plugin_ids: Vec<NcGuid>,
    },
}

impl fmt::Display for PluginDependencyPlanError {
    /// Formats identity, missing-edge, duplicate, or cycle detail for tooling.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationModeMismatch {
                expected_plugin_id,
                expected,
                actual_plugin_id,
                actual,
            } => write!(
                f,
                "mixed COM validation modes: plugin {expected_plugin_id} uses {expected}, but plugin {actual_plugin_id} uses {actual}"
            ),
            Self::IdentityMappingConflict {
                first_id,
                first_name,
                second_id,
                second_name,
            } => write!(f, "identity mapping conflict: {first_id} is `{first_name}`, but {second_id} is `{second_name}`"),
            Self::DuplicatePluginId { plugin_id } => {
                write!(f, "multiple artifacts claim plugin {plugin_id}")
            }
            Self::MissingDependency { plugin_id, dependency_id } => write!(f, "plugin {plugin_id} requires missing plugin {dependency_id}"),
            Self::DependencyCycle { plugin_ids } => {
                write!(f, "plugin dependency cycle among")?;
                for plugin_id in plugin_ids {
                    write!(f, " {plugin_id}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for PluginDependencyPlanError {}

/// Computes a deterministic dependency-first activation order.
///
/// This function never loads plugins. All hard dependencies must be present in
/// `plugins`; mixed validation modes, duplicate identities, missing dependencies,
/// and cycles are rejected. It also rejects a GUID associated with multiple
/// qualified names or one qualified name associated with multiple GUIDs anywhere
/// in the selected plugin/class/interface metadata. Independent nodes are ordered
/// by GUID field value so filesystem enumeration order cannot affect the result.
pub fn plan_plugin_load_order(plugins: &[PluginInspection]) -> Result<Vec<NcGuid>, PluginDependencyPlanError> {
    validate_validation_modes(plugins)?;
    validate_identity_mappings(plugins)?;
    let mut by_id = HashMap::with_capacity(plugins.len());
    for (index, plugin) in plugins.iter().enumerate() {
        if by_id.insert(plugin.plugin_id, index).is_some() {
            return Err(PluginDependencyPlanError::DuplicatePluginId { plugin_id: plugin.plugin_id });
        }
    }

    let mut indegree = vec![0usize; plugins.len()];
    let mut dependents = vec![Vec::new(); plugins.len()];
    let mut indices: Vec<_> = (0..plugins.len()).collect();
    indices.sort_by_key(|&index| guid_key(plugins[index].plugin_id));
    for index in indices {
        let plugin = &plugins[index];
        for dependency_id in &plugin.dependencies {
            let Some(&dependency_index) = by_id.get(dependency_id) else {
                return Err(PluginDependencyPlanError::MissingDependency {
                    plugin_id: plugin.plugin_id,
                    dependency_id: *dependency_id,
                });
            };
            indegree[index] += 1;
            dependents[dependency_index].push(index);
        }
    }

    let mut ready: Vec<_> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, &degree)| (degree == 0).then_some(index))
        .collect();
    let mut ordered = Vec::with_capacity(plugins.len());
    while !ready.is_empty() {
        ready.sort_by_key(|&index| guid_key(plugins[index].plugin_id));
        let index = ready.remove(0);
        ordered.push(plugins[index].plugin_id);
        for &dependent in &dependents[index] {
            indegree[dependent] -= 1;
            if indegree[dependent] == 0 {
                ready.push(dependent);
            }
        }
    }

    if ordered.len() != plugins.len() {
        let mut plugin_ids: Vec<_> = indegree
            .iter()
            .enumerate()
            .filter_map(|(index, &degree)| (degree != 0).then_some(plugins[index].plugin_id))
            .collect();
        plugin_ids.sort_by_key(|plugin_id| guid_key(*plugin_id));
        return Err(PluginDependencyPlanError::DependencyCycle { plugin_ids });
    }

    Ok(ordered)
}

/// Requires one ABI validation representation across an assembly.
///
/// The lowest plugin GUID is the reference and candidates are checked in GUID
/// order, keeping diagnostics stable when filesystem enumeration order changes.
fn validate_validation_modes(plugins: &[PluginInspection]) -> Result<(), PluginDependencyPlanError> {
    let Some(expected_plugin) = plugins.iter().min_by_key(|plugin| guid_key(plugin.plugin_id)) else {
        return Ok(());
    };
    let mut candidates: Vec<_> = plugins.iter().collect();
    candidates.sort_by_key(|plugin| guid_key(plugin.plugin_id));
    for plugin in candidates {
        if plugin.validation_mode != expected_plugin.validation_mode {
            return Err(PluginDependencyPlanError::ValidationModeMismatch {
                expected_plugin_id: expected_plugin.plugin_id,
                expected: expected_plugin.validation_mode,
                actual_plugin_id: plugin.plugin_id,
                actual: plugin.validation_mode,
            });
        }
    }
    Ok(())
}

/// Enforces a one-to-one mapping between every reachable GUID and qualified name.
fn validate_identity_mappings(plugins: &[PluginInspection]) -> Result<(), PluginDependencyPlanError> {
    let mut identities = Vec::new();
    for plugin in plugins {
        identities.push((plugin.plugin_id, plugin.qualified_name.as_str()));
        for class in &plugin.classes {
            identities.push((class.class_id, class.qualified_name.as_str()));
            for interface in &class.interfaces {
                identities.push((interface.interface_id, interface.interface.qualified_name.as_str()));
            }
        }
    }

    for (index, &(first_id, first_name)) in identities.iter().enumerate() {
        for &(second_id, second_name) in &identities[index + 1..] {
            if (first_id == second_id) != (first_name == second_name) {
                return Err(PluginDependencyPlanError::IdentityMappingConflict {
                    first_id,
                    first_name: first_name.to_string(),
                    second_id,
                    second_name: second_name.to_string(),
                });
            }
        }
    }
    Ok(())
}

/// Converts a GUID into an orderable field tuple for deterministic tie-breaking.
fn guid_key(guid: NcGuid) -> (u32, u16, u16, [u8; 8]) {
    (guid.data1, guid.data2, guid.data3, guid.data4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin(id: u32, dependencies: &[u32]) -> PluginInspection {
        PluginInspection {
            core_abi_version: 1,
            validation_mode: NcAbiValidationMode::SHA_256,
            plugin_id: guid(id),
            qualified_name: format!("nc.tests.plugin{id}"),
            plugin_version: "1.0.0".to_string(),
            dependencies: dependencies.iter().copied().map(guid).collect(),
            classes: Vec::new(),
        }
    }

    fn guid(value: u32) -> NcGuid {
        NcGuid::from_fields(value, 0, 0, [0; 8])
    }

    #[test]
    fn orders_dependencies_before_dependents_deterministically() {
        let plugins = [plugin(3, &[2]), plugin(2, &[1]), plugin(4, &[]), plugin(1, &[])];
        assert_eq!(plan_plugin_load_order(&plugins).unwrap(), vec![guid(1), guid(2), guid(3), guid(4)]);
    }

    #[test]
    fn rejects_missing_dependencies() {
        assert_eq!(
            plan_plugin_load_order(&[plugin(1, &[2])]),
            Err(PluginDependencyPlanError::MissingDependency {
                plugin_id: guid(1),
                dependency_id: guid(2),
            })
        );
    }

    #[test]
    fn rejects_cycles() {
        assert_eq!(
            plan_plugin_load_order(&[plugin(2, &[1]), plugin(1, &[2])]),
            Err(PluginDependencyPlanError::DependencyCycle { plugin_ids: vec![guid(1), guid(2)] })
        );
    }

    #[test]
    fn rejects_guid_name_mapping_conflicts() {
        let first = plugin(1, &[]);
        let mut second = plugin(1, &[]);
        second.qualified_name = "nc.tests.different".to_string();

        assert_eq!(
            plan_plugin_load_order(&[first, second]),
            Err(PluginDependencyPlanError::IdentityMappingConflict {
                first_id: guid(1),
                first_name: "nc.tests.plugin1".to_string(),
                second_id: guid(1),
                second_name: "nc.tests.different".to_string(),
            })
        );
    }

    #[test]
    fn rejects_mixed_validation_modes_deterministically() {
        let mut first = plugin(2, &[]);
        first.validation_mode = NcAbiValidationMode::FNV_1A_128;
        let second = plugin(1, &[]);

        assert_eq!(
            plan_plugin_load_order(&[first, second]),
            Err(PluginDependencyPlanError::ValidationModeMismatch {
                expected_plugin_id: guid(1),
                expected: NcAbiValidationMode::SHA_256,
                actual_plugin_id: guid(2),
                actual: NcAbiValidationMode::FNV_1A_128,
            })
        );
    }
}
