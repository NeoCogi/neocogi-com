//! Directional ABI compatibility checks over validated inspection artifacts.
//!
//! Interface compatibility is determined exclusively by the `(IID, ABI signature)`
//! identity. Method names and rendered signatures remain diagnostics and never
//! participate in the compatibility decision.

use std::collections::{BTreeMap, BTreeSet};

use crate::schema::{ArtifactClass, ArtifactInterface, ArtifactThreading, InspectionArtifact};

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct CompatibilityReport {
    pub(crate) breaking: Vec<String>,
    pub(crate) additive: Vec<String>,
    pub(crate) diagnostic: Vec<String>,
}

impl CompatibilityReport {
    pub(crate) fn is_compatible(&self) -> bool {
        self.breaking.is_empty()
    }
}

pub(crate) fn check_compatibility(old: &InspectionArtifact, new: &InspectionArtifact) -> CompatibilityReport {
    let mut report = CompatibilityReport::default();

    if old.plugin.id != new.plugin.id {
        report.breaking.push(format!("plugin identity changed: {} -> {}", old.plugin.id, new.plugin.id));
        return report;
    }
    if old.core_abi_version != new.core_abi_version {
        report
            .breaking
            .push(format!("core ABI version changed: {} -> {}", old.core_abi_version, new.core_abi_version));
        return report;
    }
    if old.com_validation != new.com_validation {
        report.breaking.push(format!(
            "COM validation mode changed: {} -> {}; signatures encoded by different representations are not directly comparable",
            old.com_validation, new.com_validation
        ));
        return report;
    }
    if old.target != new.target {
        report.breaking.push(format!("target ABI changed: {:?} -> {:?}", old.target, new.target));
        return report;
    }

    push_diagnostic_change(&mut report, "producer version", &old.producer.version, &new.producer.version);
    push_diagnostic_change(&mut report, "plugin version", &old.plugin.version, &new.plugin.version);
    compare_dependencies(&mut report, old, new);
    compare_classes(&mut report, &old.classes, &new.classes);
    report
}

fn compare_dependencies(report: &mut CompatibilityReport, old: &InspectionArtifact, new: &InspectionArtifact) {
    let old: BTreeSet<_> = old.plugin.dependencies.iter().collect();
    let new: BTreeSet<_> = new.plugin.dependencies.iter().collect();
    for dependency in new.difference(&old) {
        report
            .breaking
            .push(format!("added hard plugin dependency {dependency}; existing activation plans may fail"));
    }
    for dependency in old.difference(&new) {
        report.additive.push(format!("removed hard plugin dependency {dependency}"));
    }
}

fn compare_classes(report: &mut CompatibilityReport, old: &[ArtifactClass], new: &[ArtifactClass]) {
    let old = map_by(old, |class| class.id.as_str());
    let new = map_by(new, |class| class.id.as_str());

    for (id, old_class) in &old {
        let Some(new_class) = new.get(id) else {
            report.breaking.push(format!("removed class {id} ({})", old_class.name));
            continue;
        };
        compare_class(report, id, old_class, new_class);
    }
    for (id, new_class) in &new {
        if !old.contains_key(id) {
            report.additive.push(format!("added class {id} ({})", new_class.name));
        }
    }
}

fn compare_class(report: &mut CompatibilityReport, id: &str, old: &ArtifactClass, new: &ArtifactClass) {
    if old.constructor_abi_signature != new.constructor_abi_signature {
        report.breaking.push(format!(
            "class {id} constructor ABI changed: {} -> {}",
            old.constructor_abi_signature, new.constructor_abi_signature
        ));
    }
    match (old.threading, new.threading) {
        (ArtifactThreading::ThreadSafe, ArtifactThreading::ThreadLocal) => {
            report.breaking.push(format!("class {id} threading weakened: thread_safe -> thread_local"))
        }
        (ArtifactThreading::ThreadLocal, ArtifactThreading::ThreadSafe) => {
            report.additive.push(format!("class {id} threading strengthened: thread_local -> thread_safe"))
        }
        _ => {}
    }
    push_diagnostic_change(report, &format!("class {id} version"), &old.version, &new.version);
    compare_interfaces(report, id, &old.interfaces, &new.interfaces);
}

fn compare_interfaces(report: &mut CompatibilityReport, class_id: &str, old: &[ArtifactInterface], new: &[ArtifactInterface]) {
    let old = map_by(old, |interface| interface.id.as_str());
    let new = map_by(new, |interface| interface.id.as_str());

    for (iid, old_interface) in &old {
        let Some(new_interface) = new.get(iid) else {
            report
                .breaking
                .push(format!("class {class_id} removed interface {iid} ({})", old_interface.name));
            continue;
        };
        if old_interface.abi_signature != new_interface.abi_signature {
            report.breaking.push(format!(
                "class {class_id} changed interface {iid} ABI signature: {} -> {}",
                old_interface.abi_signature, new_interface.abi_signature
            ));
        } else if old_interface.methods != new_interface.methods || old_interface.base != new_interface.base {
            report.diagnostic.push(format!(
                "class {class_id} interface {iid} diagnostic method metadata changed with an unchanged ABI signature"
            ));
        }
    }
    for (iid, new_interface) in &new {
        if !old.contains_key(iid) {
            report.additive.push(format!("class {class_id} added interface {iid} ({})", new_interface.name));
        }
    }
}

fn push_diagnostic_change(report: &mut CompatibilityReport, label: &str, old: &str, new: &str) {
    if old != new {
        report.diagnostic.push(format!("{label} changed: {old:?} -> {new:?}"));
    }
}

fn map_by<'a, T>(values: &'a [T], key: impl Fn(&'a T) -> &'a str) -> BTreeMap<&'a str, &'a T> {
    values.iter().map(|value| (key(value), value)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ArtifactMethod, sample_artifact};
    use neocogi_com_core::guid_from_qualified_name;

    #[test]
    fn identical_artifacts_are_compatible() {
        assert_eq!(check_compatibility(&sample_artifact(), &sample_artifact()), CompatibilityReport::default());
    }

    #[test]
    fn new_classes_and_interfaces_are_additive() {
        let old = sample_artifact();
        let mut new = old.clone();
        let mut class = new.classes[0].clone();
        class.name = "example.Bar".to_string();
        class.id = guid_from_qualified_name(&class.name).to_string();
        new.classes.push(class);

        let interface_name = "example.Added:v1";
        let base = new.classes[0].interfaces[1].base.clone();
        new.classes[0].interfaces.push(ArtifactInterface {
            id: guid_from_qualified_name(interface_name).to_string(),
            name: interface_name.to_string(),
            abi_signature: "ncabi1;added-interface".to_string(),
            base,
            methods: Vec::new(),
        });

        let report = check_compatibility(&old, &new);
        assert!(report.is_compatible());
        assert_eq!(report.additive.len(), 2);
    }

    #[test]
    fn different_validation_modes_are_not_comparable() {
        let old = sample_artifact();
        let mut new = sample_artifact();
        new.com_validation = crate::schema::ArtifactValidationMode::Sha256;

        let report = check_compatibility(&old, &new);

        assert!(!report.is_compatible());
        assert!(report.breaking[0].contains("not directly comparable"));
    }

    #[test]
    fn removed_or_mutated_interfaces_are_breaking() {
        let old = sample_artifact();

        let mut removed_class = old.clone();
        removed_class.classes.clear();
        let report = check_compatibility(&old, &removed_class);
        assert!(!report.is_compatible());
        assert!(report.breaking[0].contains("removed class"));

        let mut removed = old.clone();
        removed.classes[0].interfaces.pop();
        assert!(!check_compatibility(&old, &removed).is_compatible());

        let mut mutated = old.clone();
        mutated.classes[0].interfaces[1].abi_signature = "ncabi1;mutated-interface".to_string();
        let report = check_compatibility(&old, &mutated);
        assert!(!report.is_compatible());
        assert!(report.breaking[0].contains("ABI signature"));
    }

    #[test]
    fn constructor_and_threading_downgrades_are_breaking() {
        let old = sample_artifact();
        let mut new = old.clone();
        new.classes[0].constructor_abi_signature = "ncabi1;constructor".to_string();
        new.classes[0].threading = ArtifactThreading::ThreadSafe;
        let strengthened = check_compatibility(&old, &new);
        assert_eq!(strengthened.breaking.len(), 1);
        assert_eq!(strengthened.additive.len(), 1);

        let mut old_safe = old.clone();
        old_safe.classes[0].threading = ArtifactThreading::ThreadSafe;
        let report = check_compatibility(&old_safe, &old);
        assert!(!report.is_compatible());
        assert!(report.breaking[0].contains("threading weakened"));
    }

    #[test]
    fn added_dependencies_break_activation_and_removed_dependencies_relax_it() {
        let dependency = guid_from_qualified_name("example.Dependency").to_string();
        let old = sample_artifact();
        let mut new = old.clone();
        new.plugin.dependencies.push(dependency.clone());
        assert!(!check_compatibility(&old, &new).is_compatible());

        let report = check_compatibility(&new, &old);
        assert!(report.is_compatible());
        assert_eq!(report.additive, vec![format!("removed hard plugin dependency {dependency}")]);
    }

    #[test]
    fn versions_and_rendered_methods_are_diagnostic_only() {
        let old = sample_artifact();
        let mut new = old.clone();
        new.producer.version = "0.2.0".to_string();
        new.plugin.version = "2.0.0".to_string();
        new.classes[0].version = "2.0.0".to_string();
        new.classes[0].interfaces[1].methods[0] = ArtifactMethod {
            index: 0,
            name: "different diagnostic".to_string(),
            signature: "different diagnostic".to_string(),
        };

        let report = check_compatibility(&old, &new);
        assert!(report.is_compatible());
        assert_eq!(report.diagnostic.len(), 4);
    }

    #[test]
    fn different_plugin_core_or_target_is_breaking() {
        let old = sample_artifact();

        let mut new = old.clone();
        new.plugin.name = "example.other".to_string();
        new.plugin.id = guid_from_qualified_name(&new.plugin.name).to_string();
        assert!(!check_compatibility(&old, &new).is_compatible());

        let mut new = old.clone();
        new.core_abi_version += 1;
        assert!(!check_compatibility(&old, &new).is_compatible());

        let mut new = old.clone();
        new.target.pointer_width = 32;
        assert!(!check_compatibility(&old, &new).is_compatible());
    }
}
