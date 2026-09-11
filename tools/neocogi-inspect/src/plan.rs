//! Serialized and human-readable dependency activation plans.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use neocogi_com_core::NcGuid;
use nc_com_shared_runtime::PluginInspection;
#[cfg(test)]
use nc_com_shared_runtime::PluginActivationInspection;
use serde::Serialize;

pub(crate) const PLAN_SCHEMA: &str = "neocogi-com.load-plan";

#[derive(Serialize)]
struct JsonPlan<'a> {
    schema: &'static str,
    /// Common COM validation mode, or `null` for an empty plugin plan.
    com_validation: Option<&'static str>,
    plugins: Vec<JsonPlanPlugin<'a>>,
    load_order: Vec<JsonLoadEntry<'a>>,
}

#[derive(Serialize)]
struct JsonPlanPlugin<'a> {
    id: String,
    name: &'a str,
    version: &'a str,
    path: String,
    dependencies: Vec<String>,
}

#[derive(Serialize)]
struct JsonLoadEntry<'a> {
    id: String,
    name: &'a str,
    path: String,
}

pub(crate) fn render_plan_json(artifacts: &[(PathBuf, PluginInspection)], order: &[NcGuid]) -> Result<String, serde_json::Error> {
    let by_id = artifacts_by_id(artifacts);
    let mut plugins: Vec<_> = artifacts.iter().collect();
    plugins.sort_by_key(|(_, plugin)| plugin.plugin_id.to_string());
    let plan = JsonPlan {
        schema: PLAN_SCHEMA,
        com_validation: common_validation_mode(artifacts),
        plugins: plugins
            .into_iter()
            .map(|(path, plugin)| JsonPlanPlugin {
                id: plugin.plugin_id.to_string(),
                name: &plugin.qualified_name,
                version: &plugin.plugin_version,
                path: path.display().to_string(),
                dependencies: plugin.dependencies.iter().map(ToString::to_string).collect(),
            })
            .collect(),
        load_order: order
            .iter()
            .map(|plugin_id| {
                let (path, plugin) = by_id[plugin_id];
                JsonLoadEntry {
                    id: plugin_id.to_string(),
                    name: &plugin.qualified_name,
                    path: path.display().to_string(),
                }
            })
            .collect(),
    };
    let mut rendered = serde_json::to_string_pretty(&plan)?;
    rendered.push('\n');
    Ok(rendered)
}

pub(crate) fn render_plan_text(artifacts: &[(PathBuf, PluginInspection)], order: &[NcGuid]) -> String {
    let by_id = artifacts_by_id(artifacts);
    let mut out = String::new();
    writeln!(out, "schema: {PLAN_SCHEMA}").unwrap();
    writeln!(out, "com_validation: {}", common_validation_mode(artifacts).unwrap_or("(no plugins)")).unwrap();
    writeln!(out, "load_order:").unwrap();
    for (position, plugin_id) in order.iter().enumerate() {
        let (path, plugin) = by_id[plugin_id];
        writeln!(out, "  {}. {} ({}) [{}]", position + 1, plugin.qualified_name, plugin_id, path.display()).unwrap();
    }
    out
}

/// Returns the common validated mode established by the shared planner.
fn common_validation_mode(artifacts: &[(PathBuf, PluginInspection)]) -> Option<&'static str> {
    artifacts.first().and_then(|(_, plugin)| plugin.validation_mode.as_str())
}

fn artifacts_by_id(artifacts: &[(PathBuf, PluginInspection)]) -> HashMap<NcGuid, (&Path, &PluginInspection)> {
    artifacts.iter().map(|(path, plugin)| (plugin.plugin_id, (path.as_path(), plugin))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com_core::{NcAbiValidationMode, NcGuid};

    /// Constructs one minimal inspected plugin for plan rendering tests.
    fn plugin() -> PluginInspection {
        PluginInspection {
            core_abi_version: 7,
            validation_mode: NcAbiValidationMode::SHA_256,
            plugin_id: NcGuid::from_fields(1, 0, 0, [0; 8]),
            qualified_name: "nc.tests.plan".to_string(),
            plugin_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            activation: PluginActivationInspection::LegacyClassTable { classes: Vec::new() },
        }
    }

    #[test]
    fn renders_common_validation_mode_in_json_and_text_plans() {
        let plugin = plugin();
        let order = [plugin.plugin_id];
        let artifacts = [(PathBuf::from("plugin.so"), plugin)];

        assert!(render_plan_json(&artifacts, &order).unwrap().contains("\"com_validation\": \"sha-256\""));
        assert!(render_plan_text(&artifacts, &order).contains("com_validation: sha-256\n"));
    }

    #[test]
    fn renders_null_mode_for_an_empty_plan() {
        assert!(render_plan_json(&[], &[]).unwrap().contains("\"com_validation\": null"));
        assert!(render_plan_text(&[], &[]).contains("com_validation: (no plugins)\n"));
    }
}
