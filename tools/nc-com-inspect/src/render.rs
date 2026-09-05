//! Human-readable and serialized output rendering.

use std::fmt::Write as _;

use nc_com_core::NcAbiThreadingMode;
use nc_com_runtime::{ClassInspection, InterfaceInspection, MethodInspection, PluginInspection};

use crate::compat::CompatibilityReport;
use crate::schema::{InspectionArtifactV1, SCHEMA, SCHEMA_VERSION, format_hash};

pub(crate) fn render_json(inspection: &PluginInspection) -> Result<String, serde_json::Error> {
    let json = InspectionArtifactV1::from(inspection);
    let mut rendered = serde_json::to_string_pretty(&json)?;
    rendered.push('\n');
    Ok(rendered)
}

pub(crate) fn render_text(inspection: &PluginInspection) -> String {
    let mut out = String::new();
    writeln!(out, "schema: {SCHEMA}").unwrap();
    writeln!(out, "schema_version: {SCHEMA_VERSION}").unwrap();
    writeln!(
        out,
        "producer: nc-com-inspect {}",
        env!("CARGO_PKG_VERSION")
    )
    .unwrap();
    writeln!(out, "target: {}", env!("NC_COM_INSPECT_TARGET")).unwrap();
    writeln!(
        out,
        "  pointer_width: {}",
        core::mem::size_of::<usize>() * 8
    )
    .unwrap();
    writeln!(
        out,
        "  endianness: {}",
        if cfg!(target_endian = "little") {
            "little"
        } else {
            "big"
        }
    )
    .unwrap();
    writeln!(out, "plugin: {}", inspection.qualified_name).unwrap();
    writeln!(out, "  id: {}", inspection.plugin_id).unwrap();
    writeln!(out, "  version: {}", inspection.plugin_version).unwrap();
    writeln!(out, "  core_abi_version: {}", inspection.core_abi_version).unwrap();
    writeln!(out, "  dependencies:").unwrap();
    if inspection.dependencies.is_empty() {
        writeln!(out, "    (none)").unwrap();
    } else {
        for dependency in &inspection.dependencies {
            writeln!(out, "    - {dependency}").unwrap();
        }
    }

    writeln!(out, "classes:").unwrap();
    if inspection.classes.is_empty() {
        writeln!(out, "  (none)").unwrap();
    } else {
        for class in &inspection.classes {
            render_class_text(&mut out, class);
        }
    }
    out
}

pub(crate) fn render_diff(differences: &[String]) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "{} difference{}",
        differences.len(),
        if differences.len() == 1 { "" } else { "s" }
    )
    .unwrap();
    for difference in differences {
        writeln!(out, "{difference}").unwrap();
    }
    out
}

pub(crate) fn render_compatibility(report: &CompatibilityReport) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "compatibility: {}",
        if report.is_compatible() {
            "compatible"
        } else {
            "breaking"
        }
    )
    .unwrap();
    render_compatibility_group(&mut out, "breaking", &report.breaking);
    render_compatibility_group(
        &mut out,
        "compatible additions and relaxations",
        &report.additive,
    );
    render_compatibility_group(&mut out, "diagnostic", &report.diagnostic);
    if report.breaking.is_empty() && report.additive.is_empty() && report.diagnostic.is_empty() {
        writeln!(out, "no compatibility-relevant changes").unwrap();
    }
    out
}

fn render_compatibility_group(out: &mut String, label: &str, changes: &[String]) {
    if changes.is_empty() {
        return;
    }
    writeln!(out, "{label}:").unwrap();
    for change in changes {
        writeln!(out, "  - {change}").unwrap();
    }
}

fn render_class_text(out: &mut String, class: &ClassInspection) {
    writeln!(out, "  - {}", class.qualified_name).unwrap();
    writeln!(out, "    id: {}", class.class_id).unwrap();
    writeln!(out, "    version: {}", class.class_version).unwrap();
    writeln!(
        out,
        "    threading: {}",
        match class.threading {
            NcAbiThreadingMode::ThreadLocal => "thread_local",
            NcAbiThreadingMode::ThreadSafe => "thread_safe",
        }
    )
    .unwrap();
    writeln!(out, "    constructor_args:").unwrap();
    writeln!(out, "      iid: {}", class.constructor_args.iid).unwrap();
    writeln!(
        out,
        "      abi_hash: {}",
        format_hash(class.constructor_args.abi_hash)
    )
    .unwrap();
    writeln!(out, "    interfaces:").unwrap();
    if class.interfaces.is_empty() {
        writeln!(out, "      (none)").unwrap();
    } else {
        for interface in &class.interfaces {
            render_interface_text(out, &interface.interface);
        }
    }
}

fn render_interface_text(out: &mut String, interface: &InterfaceInspection) {
    writeln!(out, "      - {}", interface.qualified_name).unwrap();
    writeln!(out, "        id: {}", interface.interface_id).unwrap();
    writeln!(out, "        abi_hash: {}", format_hash(interface.abi_hash)).unwrap();
    writeln!(out, "        methods:").unwrap();
    if interface.methods.is_empty() {
        writeln!(out, "          (none)").unwrap();
    } else {
        for method in &interface.methods {
            render_method_text(out, method);
        }
    }
}

fn render_method_text(out: &mut String, method: &MethodInspection) {
    writeln!(out, "          - [{}] {}", method.index, method.name).unwrap();
    writeln!(out, "            typed_signature: {}", method.signature).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_com_core::{NcAbiHash, NcGuid, NcInterfaceId};
    use nc_com_runtime::{ClassInspection, ClassInterfaceInspection, InterfaceInspection};

    fn inspection_with_thread_safe_class() -> PluginInspection {
        PluginInspection {
            core_abi_version: 1,
            plugin_id: NcGuid::from_fields(1, 2, 3, [4; 8]),
            qualified_name: "nc.tests.inspect".to_string(),
            plugin_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            classes: vec![ClassInspection {
                class_id: NcGuid::from_fields(5, 6, 7, [8; 8]),
                qualified_name: "nc.tests.inspect.ThreadSafeClass".to_string(),
                class_version: "1.0.0".to_string(),
                constructor_args: NcInterfaceId::NONE,
                threading: NcAbiThreadingMode::ThreadSafe,
                interfaces: vec![ClassInterfaceInspection {
                    interface_id: NcGuid::from_fields(9, 10, 11, [12; 8]),
                    interface: InterfaceInspection {
                        interface_id: NcGuid::from_fields(9, 10, 11, [12; 8]),
                        qualified_name: "nc.tests.inspect.Interface:v1".to_string(),
                        abi_hash: NcAbiHash::from_bytes([0x44; 32]),
                        methods: Vec::new(),
                    },
                }],
            }],
        }
    }

    #[test]
    fn renders_class_threading_in_text_and_json() {
        let inspection = inspection_with_thread_safe_class();

        assert!(render_text(&inspection).contains("    threading: thread_safe\n"));
        assert!(render_text(&inspection).contains("      - nc.tests.inspect.Interface:v1\n"));
        assert!(
            render_json(&inspection)
                .unwrap()
                .contains("\"threading\": \"thread_safe\"")
        );
        assert!(
            render_json(&inspection)
                .unwrap()
                .contains("\"name\": \"nc.tests.inspect.Interface:v1\"")
        );
    }

    #[test]
    fn renders_compatibility_categories_and_result() {
        let report = CompatibilityReport {
            breaking: vec!["removed old interface".to_string()],
            additive: vec!["added new interface".to_string()],
            diagnostic: vec!["version changed".to_string()],
        };

        let rendered = render_compatibility(&report);
        assert!(rendered.starts_with("compatibility: breaking\n"));
        assert!(rendered.contains("breaking:\n  - removed old interface\n"));
        assert!(
            rendered.contains("compatible additions and relaxations:\n  - added new interface\n")
        );
        assert!(rendered.contains("diagnostic:\n  - version changed\n"));
    }
}
