//! Serialized and human-readable dependency activation plans.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use neocogi_com_core::NcGuid;
use nc_com::PluginInspection;
use serde::Serialize;

pub(crate) const PLAN_SCHEMA: &str = "neocogi-com.load-plan";

#[derive(Serialize)]
struct JsonPlan<'a> {
    schema: &'static str,
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
    writeln!(out, "load_order:").unwrap();
    for (position, plugin_id) in order.iter().enumerate() {
        let (path, plugin) = by_id[plugin_id];
        writeln!(out, "  {}. {} ({}) [{}]", position + 1, plugin.qualified_name, plugin_id, path.display()).unwrap();
    }
    out
}

fn artifacts_by_id(artifacts: &[(PathBuf, PluginInspection)]) -> HashMap<NcGuid, (&Path, &PluginInspection)> {
    artifacts.iter().map(|(path, plugin)| (plugin.plugin_id, (path.as_path(), plugin))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com_core::NcGuid;

    /// Constructs one minimal inspected plugin for plan rendering tests.
    fn plugin() -> PluginInspection {
        PluginInspection {
            description_abi_version: neocogi_com_core::NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
            core_abi_version: 7,
            plugin_id: NcGuid::from_fields(1, 0, 0, [0; 8]),
            qualified_name: "nc.tests.plan".to_string(),
            plugin_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            factory: nc_com::FactoryInspection {
                root_factory_id: NcGuid::from_fields(2, 0, 0, [0; 8]),
                root_factory_threading: neocogi_com_core::NcAbiThreadingMode::ThreadLocal,
                root_factory_signature: [1; 32],
            },
        }
    }

    #[test]
    fn renders_plugin_in_json_and_text_plans() {
        let plugin = plugin();
        let order = [plugin.plugin_id];
        let artifacts = [(PathBuf::from("plugin.so"), plugin)];

        assert!(render_plan_json(&artifacts, &order).unwrap().contains("nc.tests.plan"));
        assert!(render_plan_text(&artifacts, &order).contains("nc.tests.plan"));
    }

    #[test]
    fn renders_empty_plan_without_plugins() {
        assert!(render_plan_json(&[], &[]).unwrap().contains("\"plugins\": []"));
        assert!(render_plan_text(&[], &[]).contains("load_order:\n"));
    }
}
