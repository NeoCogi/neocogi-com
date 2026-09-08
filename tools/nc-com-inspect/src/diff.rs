//! Deterministic exact comparison of validated inspection artifacts.

use std::collections::{BTreeMap, BTreeSet};

use crate::schema::{ArtifactClass, ArtifactInterface, ArtifactInterfaceId, ArtifactMethod, ArtifactPlugin, InspectionArtifactV3};

pub(crate) fn diff_inspections(old: &InspectionArtifactV3, new: &InspectionArtifactV3) -> Vec<String> {
    // Artifact validation runs before this function, so identity-keyed maps
    // cannot silently normalize duplicate entries.
    let mut differences = Vec::new();
    push_changed_str(&mut differences, "schema", &old.schema, &new.schema);
    push_changed_display(
        &mut differences,
        "schema_version",
        old.schema_version,
        new.schema_version,
    );
    push_changed_str(
        &mut differences,
        "producer.name",
        &old.producer.name,
        &new.producer.name,
    );
    push_changed_str(
        &mut differences,
        "producer.version",
        &old.producer.version,
        &new.producer.version,
    );
    push_changed_str(
        &mut differences,
        "target.triple",
        &old.target.triple,
        &new.target.triple,
    );
    push_changed_display(
        &mut differences,
        "target.pointer_width",
        old.target.pointer_width,
        new.target.pointer_width,
    );
    push_changed_debug(
        &mut differences,
        "target.endianness",
        &old.target.endianness,
        &new.target.endianness,
    );
    push_changed_display(
        &mut differences,
        "core_abi_version",
        old.core_abi_version,
        new.core_abi_version,
    );
    diff_plugin(&mut differences, &old.plugin, &new.plugin);
    diff_classes(&mut differences, &old.classes, &new.classes);
    differences
}

fn diff_plugin(differences: &mut Vec<String>, old: &ArtifactPlugin, new: &ArtifactPlugin) {
    push_changed_str(differences, "plugin.id", &old.id, &new.id);
    push_changed_str(differences, "plugin.name", &old.name, &new.name);
    push_changed_str(differences, "plugin.version", &old.version, &new.version);
    let old_dependencies: BTreeSet<_> = old.dependencies.iter().collect();
    let new_dependencies: BTreeSet<_> = new.dependencies.iter().collect();
    for dependency in old_dependencies.difference(&new_dependencies) {
        differences.push(format!("removed: plugin dependency {dependency}"));
    }
    for dependency in new_dependencies.difference(&old_dependencies) {
        differences.push(format!("added: plugin dependency {dependency}"));
    }
}

fn diff_classes(differences: &mut Vec<String>, old: &[ArtifactClass], new: &[ArtifactClass]) {
    let old = map_by(old, |class| class.id.clone());
    let new = map_by(new, |class| class.id.clone());
    for id in sorted_keys(&old, &new) {
        match (old.get(&id), new.get(&id)) {
            (Some(old), Some(new)) => diff_class(differences, &id, old, new),
            (Some(old), None) => differences.push(format!("removed: class {id} ({})", old.name)),
            (None, Some(new)) => differences.push(format!("added: class {id} ({})", new.name)),
            (None, None) => unreachable!(),
        }
    }
}

fn diff_class(differences: &mut Vec<String>, id: &str, old: &ArtifactClass, new: &ArtifactClass) {
    let path = format!("class {id}");
    push_changed_str(differences, &format!("{path}.name"), &old.name, &new.name);
    push_changed_str(differences, &format!("{path}.version"), &old.version, &new.version);
    push_changed_debug(differences, &format!("{path}.threading"), &old.threading, &new.threading);
    push_changed_str(
        differences,
        &format!("{path}.constructor_abi_hash"),
        &old.constructor_abi_hash,
        &new.constructor_abi_hash,
    );
    diff_interfaces(differences, &format!("{path}.interface"), &old.interfaces, &new.interfaces);
}

fn diff_interfaces(
    differences: &mut Vec<String>,
    path: &str,
    old: &[ArtifactInterface],
    new: &[ArtifactInterface],
) {
    let old = map_by(old, |interface| interface.id.clone());
    let new = map_by(new, |interface| interface.id.clone());
    for id in sorted_keys(&old, &new) {
        match (old.get(&id), new.get(&id)) {
            (Some(old), Some(new)) => {
                let interface_path = format!("{path} {id}");
                diff_interface_id(differences, &format!("{interface_path}.base"), &old.base, &new.base);
                push_changed_str(
                    differences,
                    &format!("{interface_path}.name"),
                    &old.name,
                    &new.name,
                );
                push_changed_str(
                    differences,
                    &format!("{interface_path}.abi_hash"),
                    &old.abi_hash,
                    &new.abi_hash,
                );
                diff_methods(
                    differences,
                    &format!("{interface_path}.method"),
                    &old.methods,
                    &new.methods,
                );
            }
            (Some(_), None) => differences.push(format!("removed: {path} {id}")),
            (None, Some(_)) => differences.push(format!("added: {path} {id}")),
            (None, None) => unreachable!(),
        }
    }
}

fn diff_methods(
    differences: &mut Vec<String>,
    path: &str,
    old: &[ArtifactMethod],
    new: &[ArtifactMethod],
) {
    let old = method_map(old);
    let new = method_map(new);
    let keys = old
        .keys()
        .chain(new.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    for index in keys {
        match (old.get(&index), new.get(&index)) {
            (Some(old), Some(new)) => {
                let method_path = format!("{path}[{index}]");
                push_changed_str(
                    differences,
                    &format!("{method_path}.name"),
                    &old.name,
                    &new.name,
                );
                push_changed_str(
                    differences,
                    &format!("{method_path}.signature"),
                    &old.signature,
                    &new.signature,
                );
            }
            (Some(old), None) => differences.push(format!("removed: {path}[{index}] {}", old.name)),
            (None, Some(new)) => differences.push(format!("added: {path}[{index}] {}", new.name)),
            (None, None) => unreachable!(),
        }
    }
}

fn diff_interface_id(
    differences: &mut Vec<String>,
    path: &str,
    old: &ArtifactInterfaceId,
    new: &ArtifactInterfaceId,
) {
    push_changed_str(differences, &format!("{path}.iid"), &old.iid, &new.iid);
    push_changed_str(
        differences,
        &format!("{path}.abi_hash"),
        &old.abi_hash,
        &new.abi_hash,
    );
}

fn map_by<T>(values: &[T], key: impl Fn(&T) -> String) -> BTreeMap<String, &T> {
    values.iter().map(|value| (key(value), value)).collect()
}

fn method_map(values: &[ArtifactMethod]) -> BTreeMap<u32, &ArtifactMethod> {
    values.iter().map(|value| (value.index, value)).collect()
}

fn sorted_keys<T>(old: &BTreeMap<String, T>, new: &BTreeMap<String, T>) -> BTreeSet<String> {
    old.keys().chain(new.keys()).cloned().collect()
}

fn push_changed_str(differences: &mut Vec<String>, path: &str, old: &str, new: &str) {
    if old != new {
        differences.push(format!("changed: {path}: {old:?} -> {new:?}"));
    }
}

fn push_changed_display<T>(differences: &mut Vec<String>, path: &str, old: T, new: T)
where
    T: Copy + Eq + std::fmt::Display,
{
    if old != new {
        differences.push(format!("changed: {path}: {old} -> {new}"));
    }
}

fn push_changed_debug<T>(differences: &mut Vec<String>, path: &str, old: &T, new: &T)
where
    T: Eq + std::fmt::Debug,
{
    if old != new {
        differences.push(format!("changed: {path}: {old:?} -> {new:?}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ArtifactClass, sample_artifact};
    use nc_com_core::guid_from_qualified_name;

    #[test]
    fn identical_artifacts_have_no_differences() {
        assert!(diff_inspections(&sample_artifact(), &sample_artifact()).is_empty());
    }

    #[test]
    fn reports_method_signature_changes_by_vtable_index() {
        let old = sample_artifact();
        let mut new = sample_artifact();
        new.classes[0].interfaces[1].methods[0].signature =
            "value() -> out<u64> status".to_string();

        let differences = diff_inspections(&old, &new);

        assert!(
            differences
                .iter()
                .any(|difference| difference.contains("method[0].signature"))
        );
    }

    #[test]
    fn reports_interface_qualified_name_changes() {
        let old = sample_artifact();
        let mut new = sample_artifact();
        new.classes[0].interfaces[1].name = "example.RenamedInterface:v1".to_string();

        assert!(
            diff_inspections(&old, &new)
                .iter()
                .any(|difference| difference.contains(".name"))
        );
    }

    #[test]
    fn reports_class_threading_changes() {
        let old = sample_artifact();
        let mut new = sample_artifact();
        new.classes[0].threading = crate::schema::ArtifactThreading::ThreadSafe;

        assert!(
            diff_inspections(&old, &new)
                .iter()
                .any(|difference| difference.contains(".threading"))
        );
    }

    #[test]
    fn reports_added_classes() {
        let old = sample_artifact();
        let mut new = sample_artifact();
        let mut class: ArtifactClass = new.classes[0].clone();
        class.name = "example.Bar".to_string();
        class.id = guid_from_qualified_name(&class.name).to_string();
        new.classes.push(class);

        assert!(
            diff_inspections(&old, &new)
                .iter()
                .any(|difference| difference.starts_with("added: class"))
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
