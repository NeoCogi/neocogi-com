//! Ordinary factory plugin runtime regressions own one embedded runtime.

// Only this final root retains the implementation; shared qualification imports it.
extern crate nc_com as _;
include!("support/factory_plugin_runtime_cases.rs");
