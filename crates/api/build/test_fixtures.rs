//! Compile actual fixture sources through the same bound graph and typed backend.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use neocogi_idl::{Module, parse_module, resolve_modules};

/// Discovers fixture source files deterministically; no handwritten Rust schema is read.
fn sources(root: &Path, directory: &Path, modules: &mut BTreeMap<String, Module>) {
    if !directory.exists() {
        return;
    }
    let mut entries = fs::read_dir(directory).unwrap().map(|entry| entry.unwrap().path()).collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            // Shared multipart fixtures have their own consumer builds.
            continue;
        } else if path.extension().is_some_and(|extension| extension == "ncidl") {
            println!("cargo:rerun-if-changed={}", path.display());
            let module = parse_module(path.strip_prefix(root).unwrap(), &fs::read_to_string(&path).unwrap()).unwrap_or_else(|error| panic!("{error}"));
            assert!(modules.insert(module.name.to_string(), module).is_none(), "duplicate fixture owner");
        }
    }
}

/// Selects only genuine source dependencies, keeping unrelated fixture IIDs separate.
fn dependencies(name: &str, modules: &BTreeMap<String, Module>, selected: &mut BTreeSet<String>) {
    if neocogi_idl::is_bundled_module(name) || !selected.insert(name.into()) {
        return;
    }
    let module = modules.get(name).unwrap_or_else(|| panic!("fixture dependency {name} has no source"));
    for external in &module.externals {
        dependencies(&external.module, modules, selected);
    }
}

/// Generates complete Rust behind no-argument fixture macros; it creates no runtime state.
pub fn generate(root: &Path) {
    println!("cargo:rerun-if-changed={}", root.display());
    let mut modules = BTreeMap::new();
    sources(root, &root.join("nc/tests"), &mut modules);
    let mut output = String::new();
    for (name, module) in &modules {
        let mut selected = BTreeSet::new();
        dependencies(name, &modules, &mut selected);
        let graph = resolve_modules(selected.iter().map(|name| modules[name].clone()).collect()).unwrap_or_else(|error| panic!("{error}"));
        let mut paths = neocogi_idl::generate::RustModulePaths::default();
        for dependency in graph.modules() {
            let logical = dependency.source.name.to_string();
            if neocogi_idl::is_bundled_module(&logical) {
                continue;
            }
            let segments = dependency.source.name.segments();
            assert_eq!(&segments[..2], ["nc", "tests"]);
            let mut rust = format!("fixture_{}", segments[2]);
            for scope in &segments[3..] {
                rust.push_str("::");
                rust.push_str(scope);
            }
            paths.insert(&logical, &rust).unwrap();
        }
        let rust = neocogi_idl::generate::generate_rust_with_paths(&graph, name, &paths).unwrap();
        let fixture = module.name.segments()[2..].join("__");
        // Independent revisions share one test's local namespace through
        // explicit type exports; each retains its own module constants/alloc scope.
        let declarations = &graph.module(name).unwrap().source.declarations;
        let mut exports = Vec::new();
        for declaration in declarations {
            exports.push(declaration.name.clone());
            if matches!(declaration.kind, neocogi_idl::DeclarationKind::Interface(_)) {
                for companion in neocogi_com_core::NcComInterfaceCompanion::ALL {
                    exports.push(format!("{}{}", declaration.name, companion.suffix()));
                }
            }
        }
        let exports = exports.join(",");
        let scope = format!("__nc_fixture_{fixture}");
        output.push_str(&format!(
            "/// Complete typed bindings from fixture `{name}`.\nmacro_rules! {fixture} {{ () => {{ #[allow(dead_code)] mod {scope} {{ {rust} }} pub use self::{scope}::{{{exports}}}; }}; }}\npub(crate) use {fixture};\n"
        ));
    }
    let output_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    fs::write(output_dir.join("test_contracts.rs"), output).unwrap();
    let notifications = root
        .join("../../../examples/schemas/neocogi/idl_example/notifications.rs")
        .canonicalize()
        .unwrap();
    fs::write(
        output_dir.join("example_helpers.rs"),
        format!(
            "/// Domain-specific notification delivery.\n#[path = {:?}] pub mod notifications;\n",
            notifications
        ),
    )
    .unwrap();
}
