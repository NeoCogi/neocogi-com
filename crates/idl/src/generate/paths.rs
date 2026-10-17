//! Validated Rust import locations, kept outside semantic contract identities.

use std::collections::{BTreeMap, btree_map::Entry};

use super::GenerateError;
use crate::ResolvedModules;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

/// Explicit Rust modules that expose generated declarations for logical IDL modules.
///
/// An owner may publish contracts inside an existing facade. Changing that Rust
/// location must not change the logical module name or its transitive signature.
/// Setup uses exclusive access; generation only borrows the completed mapping.
#[derive(Clone, Debug, Default)]
pub struct RustModulePaths {
    /// Exact logical names mapped to validated absolute, nongeneric Rust paths.
    pub(super) modules: BTreeMap<String, syn::Path>,
    /// Consumer-accessible dependency paths selected by Cargo. These also drive
    /// the provider's hidden re-exports; canonical owner paths above still own
    /// public inspection metadata. Pure generation leaves this map empty.
    pub(super) dependencies: BTreeMap<String, syn::Path>,
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

    /// Selects access through one real direct Cargo dependency.
    ///
    /// An indirect owner is re-exported by the selected provider under its
    /// logical module UUID. Flattening that namespace at every provider keeps
    /// paths bounded to one hop even when source graphs travel through many
    /// crates. No type is copied and these paths never enter ABI identity.
    pub(crate) fn route_dependency(&mut self, graph: &ResolvedModules, module: &str, provider: &str) -> Result<(), GenerateError> {
        let owner = self.path(provider)?;
        let path = if module == provider {
            owner
        } else {
            let alias = dependency_alias(graph, module)?;
            syn::parse_quote!(#owner::__nc_dependencies::#alias)
        };
        self.dependencies.insert(module.to_owned(), path);
        Ok(())
    }

    /// Resolves a callable/imported Rust type from this consumer's dependencies.
    /// Ordinary pure generators continue using their explicitly supplied owners.
    pub(super) fn dependency_path(&self, module: &str) -> Result<syn::Path, GenerateError> {
        match self.dependencies.get(module) {
            Some(path) => Ok(path.clone()),
            None => self.path(module),
        }
    }

    /// Names this module's dependency namespace when it exports any routes.
    /// The Cargo include entry explicitly re-exports it so facade glob imports
    /// cannot replace it with another module's namespace.
    pub(crate) fn dependency_namespace(&self) -> Option<Ident> {
        (!self.dependencies.is_empty()).then(|| format_ident!("__nc_dependencies"))
    }

    /// Derives the provider namespace from the same paths used to emit its types.
    /// All transitive owners are exported, including templates not closed here,
    /// so a later consumer can instantiate them without naming indirect crates.
    pub(super) fn dependency_exports(&self, graph: &ResolvedModules) -> Result<TokenStream, GenerateError> {
        let Some(namespace) = self.dependency_namespace() else {
            return Ok(TokenStream::new());
        };
        let exports = self.dependencies.iter().map(|(module, path)| {
            let alias = dependency_alias(graph, module)?;
            let documentation = format!(
                "IDL module `{module}`. The alias is `module_` plus its logical module UUID as hex, not an ABI signature or reversible module-name encoding."
            );
            Ok(quote! { #[doc = #documentation] pub use #path as #alias; })
        }).collect::<Result<Vec<_>, GenerateError>>()?;
        Ok(quote! {
            /// Compiler-owned typed routes for transitive contract dependencies.
            /// Re-exports preserve the original types; no runtime storage exists.
            #[doc(hidden)]
            pub mod #namespace { #(#exports)* }
        })
    }

    /// Resolves an explicit location or the established conventional crate name.
    pub(super) fn path(&self, module: &str) -> Result<syn::Path, GenerateError> {
        match self.modules.get(module) {
            Some(path) => Ok(path.clone()),
            None => parse_rust_path(&format!("::{}_contracts", module.replace('.', "_"))),
        }
    }
}

/// Names an existing logical module identity inside the generated dependency
/// namespace. UUID aliases avoid separator/case collisions and long nested paths;
/// generated documentation retains the dotted source name for interpretation.
fn dependency_alias(graph: &ResolvedModules, module: &str) -> Result<Ident, GenerateError> {
    let owner = graph.module(module).ok_or_else(|| GenerateError::new(format!("dependency route names unknown IDL module {module:?}")))?;
    Ok(format_ident!("module_{}", owner.module_id.to_string().replace('-', "")))
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
