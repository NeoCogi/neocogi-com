//! Typed, versionable TOML configuration; unknown fields are rejected as mistakes.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component as PathComponent, Path, PathBuf},
};
use serde::{Deserialize, Serialize};
use crate::Result;

/// Either one application's assembly or a named catalog at workspace/file scope.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Configuration {
    /// Named assemblies allow multiple executables to share one configuration file.
    Catalog(Catalog),
    /// A package may describe its single assembly directly in its metadata table.
    Single(Box<Assembly>),
}

/// Workspace/file configuration containing independently selectable assemblies.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Catalog {
    /// Assembly selected when the command line does not provide a name.
    pub default_assembly: Option<String>,
    /// Named application definitions; names also identify managed output directories.
    pub assemblies: BTreeMap<String, Assembly>,
}

/// Complete build inputs for one application and its dynamic components.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Assembly {
    /// Local application target; package defaults to the owning Cargo package.
    #[serde(default)]
    pub application: Application,
    /// Runtime shared library, always built before plugins and the executable.
    pub runtime: Component,
    /// Libraries whose exported descriptors determine runtime plugin load order.
    #[serde(default)]
    pub plugins: Vec<Component>,
    /// Authored files/directories copied relative to the staged executable.
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// Explicit feature and resource choices such as Vulkan or embedded assets.
    #[serde(default)]
    pub variants: BTreeMap<String, Variant>,
}

/// Cargo target and features for the executable, never for the tool itself.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Application {
    /// Workspace package name; inferred for configuration owned by a package.
    pub package: Option<String>,
    /// Binary target name; inferred only if the package has exactly one binary.
    pub bin: Option<String>,
    /// Explicit application features, independently selected from plugin features.
    pub features: Vec<String>,
    /// Whether Cargo's default features are enabled for this application.
    pub default_features: bool,
}

impl Default for Application {
    /// Matches Cargo's default-feature behavior while leaving target inference open.
    fn default() -> Self {
        Self {
            package: None,
            bin: None,
            features: Vec::new(),
            default_features: true,
        }
    }
}

/// A workspace package shorthand or an explicit local/registry source declaration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum Component {
    /// Name of a package in the application workspace, with default features enabled.
    Name(String),
    /// Source, feature, and default-feature choices for an independent library build.
    Detailed(ComponentSpec),
}

/// Source declaration converted into a Cargo build selection or generated dependency.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ComponentSpec {
    /// Cargo package name; custom library target names are discovered from artifacts.
    pub package: String,
    /// Registry version requirement; omission selects a local workspace member.
    pub version: Option<String>,
    /// Local crate directory relative to the configuration owner; takes precedence over version.
    pub path: Option<PathBuf>,
    /// Named Cargo registry; omission means crates.io for a versioned source.
    pub registry: Option<String>,
    /// Features used only for this component's build.
    #[serde(default)]
    pub features: Vec<String>,
    /// Whether to activate the component's default features.
    #[serde(default = "enabled")]
    pub default_features: bool,
}

/// Supplies Cargo's default behavior for omitted boolean configuration fields.
fn enabled() -> bool {
    true
}

impl Component {
    /// Expands shorthand to a fully typed source selection before applying variants.
    pub fn spec(&self) -> ComponentSpec {
        match self {
            Self::Name(package) => ComponentSpec {
                package: package.clone(),
                version: None,
                path: None,
                registry: None,
                features: Vec::new(),
                default_features: true,
            },
            Self::Detailed(spec) => spec.clone(),
        }
    }
}

/// A file or directory copied into a relative path within the final assembly.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Asset {
    /// Input path relative to the configuration's directory.
    pub from: PathBuf,
    /// Output path relative to the executable; parent traversal is forbidden.
    pub to: PathBuf,
}

/// Ordered, explicit modifications to an assembly's base feature/resource choices.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Variant {
    /// Replaces application features, useful for selecting exactly one renderer.
    pub application_features: Option<Vec<String>>,
    /// Adds independent capabilities, such as embedded assets or extension support.
    pub add_application_features: Vec<String>,
    /// Replaces features on the named runtime/plugin; unknown component names fail.
    pub component_features: BTreeMap<String, Vec<String>>,
    /// Additional plugin libraries needed by this variant.
    pub additional_plugins: Vec<Component>,
    /// Replaces staged resources; an empty list selects no external resources.
    pub assets: Option<Vec<Asset>>,
}

impl Configuration {
    /// Selects an unambiguous assembly, with explicit CLI names taking precedence.
    pub fn select(self, requested: Option<&str>) -> Result<(String, Assembly)> {
        match self {
            Self::Single(assembly) => {
                let name = requested.unwrap_or("default");
                validate_name(name)?;
                Ok((name.into(), *assembly))
            }
            Self::Catalog(mut catalog) => {
                let name = requested
                    .map(str::to_owned)
                    .or(catalog.default_assembly)
                    .or_else(|| (catalog.assemblies.len() == 1).then(|| catalog.assemblies.keys().next().unwrap().clone()))
                    .ok_or("choose --assembly NAME; configuration has no unique default")?;
                validate_name(&name)?;
                let assembly = catalog.assemblies.remove(&name).ok_or_else(|| format!("unknown assembly {name:?}"))?;
                Ok((name, assembly))
            }
        }
    }
}

impl Assembly {
    /// Applies variants in CLI order, then validates destinations and component identity.
    pub fn configure(&mut self, names: &[String]) -> Result<()> {
        for name in names {
            validate_name(name)?;
            let variant = self.variants.get(name).ok_or_else(|| format!("unknown variant {name:?}"))?.clone();
            if let Some(features) = variant.application_features {
                self.application.features = features;
            }
            self.application.features.extend(variant.add_application_features);
            self.plugins.extend(variant.additional_plugins);
            // Expand shorthand only when an override needs to alter its feature list.
            for (package, features) in variant.component_features {
                let component = std::iter::once(&mut self.runtime)
                    .chain(self.plugins.iter_mut())
                    .find(|component| component.spec().package == package)
                    .ok_or_else(|| format!("variant {name:?} refers to unknown component {package:?}"))?;
                let mut spec = component.spec();
                spec.features = features;
                *component = Component::Detailed(spec);
            }
            if let Some(assets) = variant.assets {
                self.assets = assets;
            }
        }
        let mut packages = BTreeSet::new();
        for component in std::iter::once(&self.runtime).chain(&self.plugins) {
            let spec = component.spec();
            validate_name(&spec.package)?;
            if !packages.insert(spec.package.clone()) {
                return Err(format!("duplicate runtime/plugin package {:?}", spec.package).into());
            }
            if spec.registry.is_some() && (spec.version.is_none() || spec.path.is_some()) {
                return Err("registry requires a version and cannot be combined with path".into());
            }
        }
        for asset in &self.assets {
            validate_destination(&asset.to)?;
        }
        Ok(())
    }
}

/// Restricts identifiers used as directory components, preventing path injection.
pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')) {
        return Err(format!("invalid identifier {name:?}: use letters, digits, '-' or '_'").into());
    }
    Ok(())
}

/// Allows only contained, nonempty asset destinations, including on Windows hosts.
pub(crate) fn validate_destination(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.components().any(|part| !matches!(part, PathComponent::Normal(_))) {
        return Err(format!("asset destination must be a contained relative path: {}", path.display()).into());
    }
    Ok(())
}
