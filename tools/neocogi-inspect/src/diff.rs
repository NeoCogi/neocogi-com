//! Deterministic exact comparison of validated inspection artifacts.

use std::collections::BTreeSet;

use crate::schema::{ArtifactFactory, ArtifactPlugin, InspectionArtifact, value_text};

/// Returns stable field-level changes after both documents pass validation.
pub(crate) fn diff_inspections(old: &InspectionArtifact, new: &InspectionArtifact) -> Vec<String> {
    // Artifact validation runs before this function, so identity-keyed maps
    // cannot silently normalize duplicate entries.
    let mut differences = Vec::new();
    push_changed(&mut differences, "schema", &old.schema, &new.schema);
    push_changed(&mut differences, "schema_version", old.schema_version, new.schema_version);
    push_changed(&mut differences, "producer.name", &old.producer.name, &new.producer.name);
    push_changed(&mut differences, "producer.version", &old.producer.version, &new.producer.version);
    push_changed(&mut differences, "target.triple", &old.target.triple, &new.target.triple);
    push_changed(&mut differences, "target.pointer_width", old.target.pointer_width, new.target.pointer_width);
    push_changed(&mut differences, "target.endianness", &old.target.endianness, &new.target.endianness);
    push_changed(&mut differences, "core_abi_version", old.core_abi_version, new.core_abi_version);
    push_changed(
        &mut differences,
        "description_abi_version",
        old.description_abi_version,
        new.description_abi_version,
    );
    diff_plugin(&mut differences, &old.plugin, &new.plugin);
    diff_factory(&mut differences, &old.factory, &new.factory);
    differences
}

/// Compares a typed factory root and its exact canonical contract revisions.
fn diff_factory(differences: &mut Vec<String>, old: &ArtifactFactory, new: &ArtifactFactory) {
    push_changed(differences, "factory.root_factory_id", &old.root_factory_id, &new.root_factory_id);
    push_changed(
        differences,
        "factory.root_factory_threading",
        &old.root_factory_threading,
        &new.root_factory_threading,
    );
    push_changed(
        differences,
        "factory.root_factory_signature",
        &old.root_factory_signature,
        &new.root_factory_signature,
    );
}

/// Compares plugin identity and order-independent provider requirements.
fn diff_plugin(differences: &mut Vec<String>, old: &ArtifactPlugin, new: &ArtifactPlugin) {
    push_changed(differences, "plugin.id", &old.id, &new.id);
    push_changed(differences, "plugin.name", &old.name, &new.name);
    push_changed(differences, "plugin.version", &old.version, &new.version);
    let old_dependencies: BTreeSet<_> = old.dependencies.iter().collect();
    let new_dependencies: BTreeSet<_> = new.dependencies.iter().collect();
    for dependency in old_dependencies.difference(&new_dependencies) {
        differences.push(format!("removed: plugin dependency {dependency}"));
    }
    for dependency in new_dependencies.difference(&old_dependencies) {
        differences.push(format!("added: plugin dependency {dependency}"));
    }
}

/// Compare one schema value and retain both canonical, escaped representations.
/// Equality and report ordering stay independent from how text is rendered.
fn push_changed<T: Eq + serde::Serialize>(differences: &mut Vec<String>, path: &str, old: T, new: T) {
    if old != new {
        differences.push(format!("changed: {path}: {} -> {}", value_text(&old), value_text(&new)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::sample_artifact;
    use neocogi_com::guid_from_qualified_name;

    #[test]
    fn identical_artifacts_have_no_differences() {
        assert!(diff_inspections(&sample_artifact(), &sample_artifact()).is_empty());
    }

    /// Confirms exact factory comparison names the changed contract fingerprint.
    #[test]
    fn reports_factory_contract_fingerprint_changes() {
        let old = sample_artifact();
        let mut new = old.clone();
        new.factory.root_factory_signature = "22".repeat(32);

        let differences = diff_inspections(&old, &new);

        assert_eq!(
            differences,
            vec![format!(
                "changed: factory.root_factory_signature: \"{}\" -> \"{}\"",
                old.factory.root_factory_signature, new.factory.root_factory_signature
            )]
        );
    }

    #[test]
    fn reports_dependency_changes_independent_of_order() {
        let dependency_a = guid_from_qualified_name("example.DependencyA").to_string();
        let dependency_b = guid_from_qualified_name("example.DependencyB").to_string();
        let dependency_c = guid_from_qualified_name("example.DependencyC").to_string();
        let mut old = sample_artifact();
        old.plugin.dependencies = vec![dependency_b.clone(), dependency_a.clone()];
        let mut new = sample_artifact();
        new.plugin.dependencies = vec![dependency_a, dependency_c.clone()];

        assert_eq!(
            diff_inspections(&old, &new),
            vec![
                format!("removed: plugin dependency {dependency_b}"),
                format!("added: plugin dependency {dependency_c}"),
            ]
        );
    }
}
