//! Validated Rust import locations, kept outside semantic contract identities.

use std::collections::{BTreeMap, btree_map::Entry};

use super::GenerateError;

/// Explicit Rust modules that expose generated declarations for logical IDL modules.
///
/// An owner may publish contracts inside an existing facade. Changing that Rust
/// location must not change the logical module name or its transitive signature.
/// Setup uses exclusive access; generation only borrows the completed mapping.
#[derive(Clone, Debug, Default)]
pub struct RustModulePaths {
    /// Exact logical names mapped to validated absolute, nongeneric Rust paths.
    pub(super) modules: BTreeMap<String, syn::Path>,
}

impl RustModulePaths {
    /// Associates a logical module with a crate-rooted Rust location.
    ///
    /// Accepts paths such as `neocogi_com::events` and rejects relative keywords,
    /// generic arguments, and conflicting registrations. An identical repeated
    /// registration is harmless when dependency graphs share one contract owner.
    /// COM's `validate_rust_path` defines the shared grammar and byte limit; valid
    /// raw identifiers and an optional leading `::` use that same policy.
    pub fn insert(&mut self, module: &str, rust_path: &str) -> Result<(), GenerateError> {
        neocogi_com_core::validate_qualified_name(module).map_err(|error| GenerateError::new(format!("invalid IDL module {module:?}: {error}")))?;
        let path = parse_rust_path(rust_path)?;
        match self.modules.entry(module.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(path);
            }
            Entry::Occupied(entry) if entry.get() == &path => {}
            Entry::Occupied(_) => return Err(GenerateError::new(format!("conflicting Rust locations for IDL module {module:?}"))),
        }
        Ok(())
    }

    /// Resolves an explicit location or the established conventional crate name.
    pub(super) fn path(&self, module: &str) -> Result<syn::Path, GenerateError> {
        match self.modules.get(module) {
            Some(path) => Ok(path.clone()),
            None => parse_rust_path(&format!("::{}_contracts", module.replace('.', "_"))),
        }
    }
}

/// Applies COM's binding-path contract before constructing the backend's syntax tree.
/// `syn` is a token-construction dependency here, not a separate metadata policy.
fn parse_rust_path(value: &str) -> Result<syn::Path, GenerateError> {
    neocogi_com_core::validate_rust_path(value).map_err(|error| GenerateError::new(format!("invalid Rust contract path {value:?}: {error}")))?;
    let mut path: syn::Path =
        syn::parse_str(value).map_err(|error| GenerateError::new(format!("cannot parse validated Rust contract path {value:?}: {error}")))?;
    // Equivalent optional-root spellings must not conflict during registration.
    path.leading_colon = Some(Default::default());
    Ok(path)
}
