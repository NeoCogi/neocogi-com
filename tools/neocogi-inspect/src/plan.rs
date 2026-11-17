//! Serialized and human-readable dependency activation plans.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use neocogi_com_core::NcGuid;
use neocogi_com_host::PluginInspection;
use serde::Serialize;

/// Stable JSON/text load-plan family; native ABI versions remain independent.
pub(crate) const PLAN_SCHEMA: &str = "neocogi-com.load-plan";

/// Complete document serialized before the CLI publishes any output.
#[derive(Serialize)]
struct JsonPlan<'a> {
    /// Serialized document family.
    schema: &'static str,
    /// Inspected artifacts in canonical plugin-ID order.
    plugins: Vec<JsonPlanPlugin<'a>>,
    /// Validated provider-first activation order.
    load_order: Vec<JsonLoadEntry<'a>>,
}

/// One inspected artifact, borrowing its original native source pathname.
#[derive(Serialize)]
struct JsonPlanPlugin<'a> {
    /// Canonical plugin GUID.
    id: String,
    /// Qualified plugin name copied during inspection.
    name: &'a str,
    /// Diagnostic plugin version text.
    version: &'a str,
    /// Exact source spelling. Serde serializes Unicode as a string and rejects
    /// unrepresentable paths; diagnostic Display must not supply machine identity.
    path: &'a Path,
    /// Declared hard dependency GUIDs.
    dependencies: Vec<String>,
}

/// One ordered activation entry pointing to the same source artifact.
#[derive(Serialize)]
struct JsonLoadEntry<'a> {
    /// Canonical plugin GUID.
    id: String,
    /// Qualified plugin name copied during inspection.
    name: &'a str,
    /// Exact source spelling. Serde serializes Unicode as a string and rejects
    /// unrepresentable paths; diagnostic Display must not supply machine identity.
    path: &'a Path,
}

/// Renders a complete JSON document for the already validated artifact/order set.
/// Borrowed Path serialization preserves Unicode spelling or returns a data
/// error. Failure drops the private String before stdout/file publication;
/// inspection and dependency validation have already run in the caller.
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
                path: path.as_path(),
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
                    path,
                }
            })
            .collect(),
    };
    let mut rendered = serde_json::to_string_pretty(&plan)?;
    rendered.push('\n');
    Ok(rendered)
}

/// Renders diagnostic text with native-path Display; no machine-path encoding is promised.
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

/// Borrows original artifact paths for the planner-validated unique plugin IDs.
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
            factory: neocogi_com_host::FactoryInspection {
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

    /// JSON escapes text while retaining the exact Unicode path value in both arrays.
    #[test]
    fn json_source_paths_preserve_unicode_escapes_and_relative_spelling() {
        let plugin = plugin();
        let order = [plugin.plugin_id];
        let spelling = "../providers/é-e\u{301}-\"quoted\"-line\n.so";
        let artifacts = [(PathBuf::from(spelling), plugin)];
        let rendered = render_plan_json(&artifacts, &order).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["plugins"][0]["path"], spelling);
        assert_eq!(value["load_order"][0]["path"], spelling);
    }

    /// Provider-first order can differ from canonical plugin-ID order; paths
    /// must remain associated with their identities under either input order.
    #[test]
    fn json_paths_follow_each_identity_in_both_distinct_orders() {
        let mut provider = plugin();
        provider.plugin_id = NcGuid::from_fields(2, 0, 0, [0; 8]);
        provider.qualified_name = "nc.tests.provider".into();
        let mut dependent = plugin();
        dependent.dependencies = vec![provider.plugin_id];
        let order = [provider.plugin_id, dependent.plugin_id];
        let mut artifacts = [(PathBuf::from("provider-é.so"), provider), (PathBuf::from("dependent.so"), dependent)];
        let expected = render_plan_json(&artifacts, &order).unwrap();
        let value: serde_json::Value = serde_json::from_str(&expected).unwrap();
        assert_eq!(value["plugins"][0]["id"], order[1].to_string());
        assert_eq!(value["plugins"][0]["path"], "dependent.so");
        assert_eq!(value["plugins"][1]["path"], "provider-é.so");
        assert_eq!(value["load_order"][0]["id"], order[0].to_string());
        assert_eq!(value["load_order"][0]["path"], "provider-é.so");
        assert_eq!(value["load_order"][1]["path"], "dependent.so");
        artifacts.reverse();
        assert_eq!(render_plan_json(&artifacts, &order).unwrap(), expected);
    }

    /// Constructs a valid native OsString that cannot be represented by a JSON string.
    #[cfg(unix)]
    fn non_unicode_source() -> PathBuf {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(b"provider-\xff.so".to_vec()).into()
    }

    /// Windows native paths may contain unpaired UTF-16 units, outside Unicode.
    #[cfg(windows)]
    fn non_unicode_source() -> PathBuf {
        use std::os::windows::ffi::OsStringExt;
        std::ffi::OsString::from_wide(&[b'p' as u16, 0xd800, b'.' as u16, b'd' as u16, b'l' as u16, b'l' as u16]).into()
    }

    /// Every plugin path is serialized, even if no activation-order entry uses it.
    #[cfg(any(unix, windows))]
    #[test]
    fn json_rejects_non_unicode_sources_even_without_a_load_order_entry() {
        let plugin = plugin();
        let order = [plugin.plugin_id];
        let path = non_unicode_source();
        assert!(path.to_str().is_none());
        let artifacts = [(path, plugin)];
        for selected in [&order[..], &[][..]] {
            let error = render_plan_json(&artifacts, selected).unwrap_err();
            assert!(error.is_data());
            assert!(error.to_string().contains("path contains invalid UTF-8"));
        }
        assert!(render_plan_text(&artifacts, &order).contains("nc.tests.plan"));
    }
}
