//! Directional ABI compatibility checks over validated inspection artifacts.
//!
//! The root factory signature covers its complete transitive contract revision.
//! Plugin version text remains diagnostic; provider dependencies affect deployment.

use std::collections::BTreeSet;

use crate::schema::{ArtifactFactory, ArtifactThreading, InspectionArtifact, value_text};

/// Directional changes relevant to replacing one deployed plugin.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct CompatibilityReport {
    /// Changes that can invalidate an existing host or load plan.
    pub(crate) breaking: Vec<String>,
    /// Requirements relaxed or capabilities strengthened by the new plugin.
    pub(crate) additive: Vec<String>,
    /// Informational changes outside the typed compatibility boundary.
    pub(crate) diagnostic: Vec<String>,
}

impl CompatibilityReport {
    /// Reports whether an old consumer remains compatible with the new snapshot.
    pub(crate) fn is_compatible(&self) -> bool {
        self.breaking.is_empty()
    }
}

/// Compares validated snapshots, stopping early when native identity differs.
pub(crate) fn check_compatibility(old: &InspectionArtifact, new: &InspectionArtifact) -> CompatibilityReport {
    let mut report = CompatibilityReport::default();

    if old.plugin.id != new.plugin.id {
        report.breaking.push(format!("plugin identity changed: {} -> {}", old.plugin.id, new.plugin.id));
        return report;
    }
    if old.description_abi_version != new.description_abi_version {
        report.breaking.push(format!(
            "descriptor ABI version changed: {} -> {}",
            old.description_abi_version, new.description_abi_version
        ));
        return report;
    }
    if old.core_abi_version != new.core_abi_version {
        report
            .breaking
            .push(format!("core ABI version changed: {} -> {}", old.core_abi_version, new.core_abi_version));
        return report;
    }
    if old.target != new.target {
        report
            .breaking
            .push(format!("target ABI changed: {} -> {}", value_text(&old.target), value_text(&new.target)));
        return report;
    }

    push_diagnostic_change(&mut report, "producer version", &old.producer.version, &new.producer.version);
    push_diagnostic_change(&mut report, "plugin version", &old.plugin.version, &new.plugin.version);
    compare_dependencies(&mut report, old, new);
    compare_factory(&mut report, &old.factory, &new.factory);
    report
}

/// Compares one typed root and its exact native contract requirements.
fn compare_factory(report: &mut CompatibilityReport, old: &ArtifactFactory, new: &ArtifactFactory) {
    if old.root_factory_id != new.root_factory_id {
        report
            .breaking
            .push(format!("root factory identity changed: {} -> {}", old.root_factory_id, new.root_factory_id));
        return;
    }
    if old.root_factory_signature != new.root_factory_signature {
        report.breaking.push(format!(
            "root factory ABI signature changed: {} -> {}",
            old.root_factory_signature, new.root_factory_signature
        ));
        push_diagnostic_change(report, "root factory threading", &old.root_factory_threading, &new.root_factory_threading);
        return;
    }
    compare_threading(report, "root factory", old.root_factory_threading, new.root_factory_threading);
}

/// A new hard dependency can make a previously valid load plan incomplete.
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

/// Classifies a capability strengthening as additive and weakening as breaking.
fn compare_threading(report: &mut CompatibilityReport, subject: &str, old: ArtifactThreading, new: ArtifactThreading) {
    match (old, new) {
        (ArtifactThreading::ThreadSafe, ArtifactThreading::ThreadLocal) => {
            report.breaking.push(format!("{subject} threading weakened: thread_safe -> thread_local"))
        }
        (ArtifactThreading::ThreadLocal, ArtifactThreading::ThreadSafe) => {
            report.additive.push(format!("{subject} threading strengthened: thread_local -> thread_safe"))
        }
        _ => {}
    }
}

/// Records changed diagnostic values with their canonical JSON spelling.
fn push_diagnostic_change<T: serde::Serialize + PartialEq + ?Sized>(report: &mut CompatibilityReport, label: &str, old: &T, new: &T) {
    if old != new {
        report.diagnostic.push(format!("{label} changed: {} -> {}", value_text(old), value_text(new)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::sample_artifact;
    use neocogi_com_core::guid_from_qualified_name;

    #[test]
    fn identical_artifacts_are_compatible() {
        assert_eq!(check_compatibility(&sample_artifact(), &sample_artifact()), CompatibilityReport::default());
    }

    /// Confirms an exact contract revision change is a factory ABI break.
    #[test]
    fn factory_contract_fingerprint_changes_are_breaking() {
        let old = sample_artifact();
        let mut new = old.clone();
        new.factory.root_factory_signature = "22".repeat(32);

        let report = check_compatibility(&old, &new);

        assert!(!report.is_compatible());
        assert!(report.breaking.iter().any(|change| change.contains("ABI signature changed")));
    }

    #[test]
    fn factory_threading_changes_are_directional() {
        let old = sample_artifact();
        let mut new = old.clone();
        new.factory.root_factory_threading = ArtifactThreading::ThreadSafe;
        let strengthened = check_compatibility(&old, &new);
        assert!(strengthened.is_compatible());
        assert_eq!(strengthened.additive.len(), 1);

        let mut old_safe = old.clone();
        old_safe.factory.root_factory_threading = ArtifactThreading::ThreadSafe;
        let report = check_compatibility(&old_safe, &old);
        assert!(!report.is_compatible());
        assert!(report.breaking[0].contains("threading weakened"));
    }

    /// A new exact contract cannot relax the previous root's required capability.
    #[test]
    fn threading_changes_with_a_new_digest_are_diagnostic_context() {
        let mut old = sample_artifact();
        old.plugin.dependencies.push(guid_from_qualified_name("example.Dependency").to_string());
        let mut new = sample_artifact();
        new.factory.root_factory_signature = "22".repeat(32);
        new.factory.root_factory_threading = ArtifactThreading::ThreadSafe;

        let report = check_compatibility(&old, &new);
        assert!(!report.is_compatible());
        assert_eq!(report.breaking.len(), 1);
        assert!(report.breaking[0].contains("ABI signature changed"));
        assert_eq!(report.additive.len(), 1);
        assert!(report.additive[0].contains("removed hard plugin dependency"));
        assert_eq!(report.diagnostic.len(), 1);
        assert!(report.diagnostic[0].contains("root factory threading changed"));

        let reverse = check_compatibility(&new, &old);
        assert!(!reverse.is_compatible());
        assert_eq!(reverse.breaking.len(), 2);
        assert!(reverse.breaking.iter().all(|change| !change.contains("threading")));
        assert_eq!(reverse.diagnostic.len(), 1);
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
    fn versions_are_diagnostic_only() {
        let old = sample_artifact();
        let mut new = old.clone();
        new.producer.version = "0.2.0".to_string();
        new.plugin.version = "2.0.0".to_string();

        let report = check_compatibility(&old, &new);
        assert!(report.is_compatible());
        assert_eq!(report.diagnostic.len(), 2);
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
