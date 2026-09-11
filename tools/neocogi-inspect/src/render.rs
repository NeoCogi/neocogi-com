//! Human-readable and serialized output rendering.

use std::fmt::Write as _;

use neocogi_com_core::NcAbiThreadingMode;
use nc_com_shared_runtime::{ContractModuleInspection, FactoryInspection, PluginInspection};

use crate::compat::CompatibilityReport;
use crate::schema::{InspectionArtifact, SCHEMA, SCHEMA_VERSION};

pub(crate) fn render_json(inspection: &PluginInspection) -> Result<String, serde_json::Error> {
    let json = InspectionArtifact::from(inspection);
    let mut rendered = serde_json::to_string_pretty(&json)?;
    rendered.push('\n');
    Ok(rendered)
}

pub(crate) fn render_text(inspection: &PluginInspection) -> String {
    let mut out = String::new();
    writeln!(out, "schema: {SCHEMA}").unwrap();
    writeln!(out, "schema_version: {SCHEMA_VERSION}").unwrap();
    writeln!(out, "producer: neocogi-inspect {}", env!("CARGO_PKG_VERSION")).unwrap();
    writeln!(out, "target: {}", env!("NC_COM_INSPECT_TARGET")).unwrap();
    writeln!(out, "  pointer_width: {}", core::mem::size_of::<usize>() * 8).unwrap();
    writeln!(out, "  endianness: {}", if cfg!(target_endian = "little") { "little" } else { "big" }).unwrap();
    writeln!(out, "plugin: {}", inspection.qualified_name).unwrap();
    writeln!(out, "  id: {}", inspection.plugin_id).unwrap();
    writeln!(out, "  version: {}", inspection.plugin_version).unwrap();
    writeln!(out, "  core_abi_version: {}", inspection.core_abi_version).unwrap();
    writeln!(out, "  com_validation: {}", inspection.validation_mode).unwrap();
    writeln!(out, "  dependencies:").unwrap();
    if inspection.dependencies.is_empty() {
        writeln!(out, "    (none)").unwrap();
    } else {
        for dependency in &inspection.dependencies {
            writeln!(out, "    - {dependency}").unwrap();
        }
    }

    writeln!(out, "activation: {}", inspection.activation_model()).unwrap();
    render_factory_text(&mut out, &inspection.factory);
    out
}

/// Renders one factory root and the canonical graph reached through its bindings.
fn render_factory_text(out: &mut String, factory: &FactoryInspection) {
    writeln!(out, "root_factory:").unwrap();
    writeln!(out, "  id: {}", factory.root_factory_id).unwrap();
    writeln!(out, "  threading: {}", threading_name(factory.root_factory_threading)).unwrap();
    writeln!(out, "direct_contracts:").unwrap();
    for contract in &factory.direct_contracts {
        writeln!(out, "  - {} @ {}", contract.module_id, fingerprint_text(contract.abi_fingerprint.bytes)).unwrap();
    }
    writeln!(out, "contract_modules:").unwrap();
    for module in &factory.modules {
        render_contract_module_text(out, module);
    }
}

/// Renders one canonical contract module without exposing native getter pointers.
fn render_contract_module_text(out: &mut String, module: &ContractModuleInspection) {
    writeln!(out, "  - {}", module.module_name).unwrap();
    writeln!(out, "    id: {}", module.module_id).unwrap();
    writeln!(out, "    abi_fingerprint: {}", fingerprint_text(module.abi_fingerprint.bytes)).unwrap();
    writeln!(out, "    dependencies:").unwrap();
    if module.dependencies.is_empty() {
        writeln!(out, "      (none)").unwrap();
    } else {
        for dependency in &module.dependencies {
            writeln!(out, "      - {} @ {}", dependency.module_id, fingerprint_text(dependency.abi_fingerprint.bytes)).unwrap();
        }
    }
    writeln!(out, "    declarations:").unwrap();
    for declaration in &module.declarations {
        writeln!(
            out,
            "      - {} {} v{}",
            declaration.kind.as_str(),
            declaration.qualified_name,
            declaration.version
        )
        .unwrap();
        writeln!(out, "        id: {}", declaration.declaration_id).unwrap();
        if let Some(threading) = declaration.threading {
            writeln!(out, "        threading: {}", threading_name(threading)).unwrap();
        }
    }
    writeln!(out, "    source:").unwrap();
    for line in module.source.lines() {
        writeln!(out, "      {line}").unwrap();
    }
}

/// Returns the IDL spelling of a validated threading capability.
fn threading_name(threading: NcAbiThreadingMode) -> &'static str {
    match threading {
        NcAbiThreadingMode::ThreadLocal => "thread_local",
        NcAbiThreadingMode::ThreadSafe => "thread_safe",
    }
}

/// Formats a contract fingerprint as fixed-width lowercase hexadecimal text.
fn fingerprint_text(bytes: [u8; 32]) -> String {
    let mut text = String::with_capacity(64);
    for byte in bytes {
        write!(text, "{byte:02x}").expect("writing into String cannot fail");
    }
    text
}

pub(crate) fn render_diff(differences: &[String]) -> String {
    let mut out = String::new();
    writeln!(out, "{} difference{}", differences.len(), if differences.len() == 1 { "" } else { "s" }).unwrap();
    for difference in differences {
        writeln!(out, "{difference}").unwrap();
    }
    out
}

pub(crate) fn render_compatibility(report: &CompatibilityReport) -> String {
    let mut out = String::new();
    writeln!(out, "compatibility: {}", if report.is_compatible() { "compatible" } else { "breaking" }).unwrap();
    render_compatibility_group(&mut out, "breaking", &report.breaking);
    render_compatibility_group(&mut out, "compatible additions and relaxations", &report.additive);
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

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com_core::NcGuid;
    use nc_com_shared_runtime::FactoryInspection;

    /// Constructs a minimal inspected factory plugin for rendering tests.
    fn inspection_with_thread_safe_factory() -> PluginInspection {
        PluginInspection {
            core_abi_version: 1,
            validation_mode: neocogi_com_core::NcAbiValidationMode::MANGLING,
            plugin_id: NcGuid::from_fields(1, 2, 3, [4; 8]),
            qualified_name: "nc.tests.inspect".to_string(),
            plugin_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            factory: FactoryInspection {
                root_factory_id: NcGuid::from_fields(5, 6, 7, [8; 8]),
                root_factory_threading: NcAbiThreadingMode::ThreadSafe,
                direct_contracts: Vec::new(),
                modules: Vec::new(),
            },
        }
    }

    #[test]
    fn renders_factory_threading_in_text_and_json() {
        let inspection = inspection_with_thread_safe_factory();

        assert!(render_text(&inspection).contains("  threading: thread_safe\n"));
        assert!(render_text(&inspection).contains("  com_validation: mangling\n"));
        assert!(render_json(&inspection).unwrap().contains("\"root_factory_threading\": \"thread_safe\""));
        assert!(render_json(&inspection).unwrap().contains("\"com_validation\": \"mangling\""));
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
        assert!(rendered.contains("compatible additions and relaxations:\n  - added new interface\n"));
        assert!(rendered.contains("diagnostic:\n  - version changed\n"));
    }
}
