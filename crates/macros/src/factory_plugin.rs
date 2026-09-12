//! Parsing and expansion of factory-only plugin descriptions.
//!
//! The generated metadata references exact contract module revisions and one
//! typed Rust factory implementation. Factory identity and threading are read
//! from the implementation's generated IDL description instead of repeated in
//! the plugin declaration.

use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, LitStr, Path, Result, Token, bracketed, parenthesized};

use crate::args::{InterfaceGuid, parse_guid_arg};
use crate::util::{checked_u32_len, runtime_crate_path, stable_guid, validate_version_literal};

/// Normalized inputs to one `factory_plugin!` declaration.
pub(crate) struct FactoryPluginArgs {
    /// Optional checked plugin ID; omission derives it from `name`.
    plugin_id: Option<InterfaceGuid>,
    /// Normative dotted plugin name.
    name: LitStr,
    /// Static plugin release version.
    version: LitStr,
    /// Hard plugin dependencies that must already be registered.
    dependencies: Vec<Path>,
    /// Exact generated contract module references used by this plugin.
    contracts: Vec<Path>,
    /// Rust implementation type for the plugin's root IDL factory.
    factory: Path,
    /// Optional path overriding automatic `neocogi-com` discovery.
    runtime_crate: Option<Path>,
}

impl Parse for FactoryPluginArgs {
    /// Parses named fields and rejects ambiguous or repeated declarations.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut plugin_id = None;
        let mut name = None;
        let mut version = None;
        let mut dependencies = None;
        let mut contracts = None;
        let mut factory = None;
        let mut runtime_crate = None;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "plugin_id" {
                if plugin_id.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate plugin_id(...)"));
                }
                let content;
                parenthesized!(content in input);
                plugin_id = Some(parse_guid_arg(&content, "plugin_id")?);
            } else if keyword == "name" {
                parse_unique_value(input, keyword, &mut name, "name")?;
            } else if keyword == "version" {
                parse_unique_value(input, keyword, &mut version, "version")?;
            } else if keyword == "dependencies" {
                if dependencies.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate dependencies = [...]"));
                }
                input.parse::<Token![=]>()?;
                dependencies = Some(parse_path_list(input)?);
            } else if keyword == "contracts" {
                if contracts.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate contracts = [...]"));
                }
                input.parse::<Token![=]>()?;
                contracts = Some(parse_path_list(input)?);
            } else if keyword == "factory" {
                parse_unique_value(input, keyword, &mut factory, "factory")?;
            } else if keyword == "neocogi_com" {
                parse_unique_value(input, keyword, &mut runtime_crate, "neocogi_com")?;
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected plugin_id(...), name = ..., version = ..., dependencies = [...], contracts = [...], factory = ..., or neocogi_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let contracts = contracts.ok_or_else(|| syn::Error::new(input.span(), "expected contracts = [...]"))?;
        if contracts.is_empty() {
            return Err(syn::Error::new(input.span(), "factory plugins must reference at least one contract module"));
        }
        let dependencies = dependencies.unwrap_or_default();
        reject_duplicate_paths(&dependencies, "plugin dependency")?;
        reject_duplicate_paths(&contracts, "contract module")?;

        Ok(Self {
            plugin_id,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "expected name = ..."))?,
            version: version.ok_or_else(|| syn::Error::new(input.span(), "expected version = ..."))?,
            dependencies,
            contracts,
            factory: factory.ok_or_else(|| syn::Error::new(input.span(), "expected factory = ..."))?,
            runtime_crate,
        })
    }
}

/// Parses one `keyword = value` field exactly once.
fn parse_unique_value<T: Parse>(input: ParseStream<'_>, keyword: Ident, slot: &mut Option<T>, name: &str) -> Result<()> {
    if slot.is_some() {
        return Err(syn::Error::new(keyword.span(), format!("duplicate {name} = ...")));
    }
    input.parse::<Token![=]>()?;
    *slot = Some(input.parse()?);
    Ok(())
}

/// Parses one bracketed list of ordinary Rust module paths.
fn parse_path_list(input: ParseStream<'_>) -> Result<Vec<Path>> {
    let content;
    bracketed!(content in input);
    Ok(content.parse_terminated(Path::parse_mod_style, Token![,])?.into_iter().collect())
}

/// Rejects duplicate path spellings before they become ambiguous static rows.
fn reject_duplicate_paths(paths: &[Path], subject: &str) -> Result<()> {
    for (index, path) in paths.iter().enumerate() {
        let key = path.to_token_stream().to_string();
        if paths[..index].iter().any(|previous| previous.to_token_stream().to_string() == key) {
            return Err(syn::Error::new(path.span(), format!("duplicate {subject} path")));
        }
    }
    Ok(())
}

/// Emits immutable plugin metadata backed by typed IDL and module references.
pub(crate) fn expand_factory_plugin(args: FactoryPluginArgs) -> proc_macro2::TokenStream {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    if let Err(error) = validate_version_literal(&args.version) {
        return error.to_compile_error();
    }
    let plugin_id = match args.plugin_id {
        Some(plugin_id) => match stable_guid(plugin_id, &args.name, &nc) {
            Ok(plugin_id) => plugin_id,
            Err(error) => return error.to_compile_error(),
        },
        None => match stable_guid(InterfaceGuid::AutoGuid { value: args.name.clone() }, &args.name, &nc) {
            Ok(plugin_id) => plugin_id,
            Err(error) => return error.to_compile_error(),
        },
    };
    let dependency_count = match checked_u32_len(args.dependencies.len(), args.name.span(), "plugin dependency count") {
        Ok(count) => count,
        Err(error) => return error.to_compile_error(),
    };
    let contract_count = match checked_u32_len(args.contracts.len(), args.name.span(), "contract module count") {
        Ok(count) => count,
        Err(error) => return error.to_compile_error(),
    };
    let dependency_len = args.dependencies.len();
    let contract_len = args.contracts.len();
    let dependencies = args.dependencies.iter().map(|dependency| {
        quote! {
            #nc::NcPluginDependencyDescriptor {
                plugin_id: #dependency,
            }
        }
    });
    let contracts = &args.contracts;
    let factory = &args.factory;
    let name = &args.name;
    let version = &args.version;

    quote! {
        /// Hard plugin dependencies required before this factory can activate.
        static NC_COM_FACTORY_PLUGIN_DEPENDENCIES: [#nc::NcPluginDependencyDescriptor; #dependency_len] = [
            #(#dependencies,)*
        ];

        /// Exact contract module revisions linked into this plugin assembly.
        static NC_COM_FACTORY_PLUGIN_CONTRACTS: [#nc::NcContractModuleReference; #contract_len] = [
            #(#contracts,)*
        ];

        /// Immutable factory-only plugin metadata consumed by the shared runtime.
        static NC_COM_FACTORY_PLUGIN_DESCRIPTION: #nc::NcFactoryPluginDescription = #nc::NcFactoryPluginDescription {
            core_abi_version: #nc::NC_COM_CORE_ABI_VERSION,
            plugin_id: #plugin_id,
            qualified_name: #nc::NcStaticUtf8::from_static(#name),
            plugin_version: #nc::NcStaticUtf8::from_static(#version),
            dependency_count: #dependency_count,
            dependencies: NC_COM_FACTORY_PLUGIN_DEPENDENCIES.as_ptr(),
            contract_count: #contract_count,
            contracts: NC_COM_FACTORY_PLUGIN_CONTRACTS.as_ptr(),
            root_factory_id: <<#factory as #nc::ComFactoryImplementation>::Description as #nc::ComObjectDescription>::OBJECT_ID,
            root_factory_threading: <<<#factory as #nc::ComFactoryImplementation>::Description as #nc::ComObjectDescription>::Threading as #nc::ThreadingModel>::ABI_THREADING_MODE.as_raw(),
            activate: Some(#nc::activate_factory_implementation::<#factory>),
        };

        /// Returns the factory-only plugin description for static registration.
        pub fn factory_plugin_description() -> &'static #nc::NcFactoryPluginDescription {
            &NC_COM_FACTORY_PLUGIN_DESCRIPTION
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FactoryPluginArgs;

    /// Confirms an empty contract list is rejected at the declaration site.
    #[test]
    fn contract_module_list_must_not_be_empty() {
        let error = syn::parse_str::<FactoryPluginArgs>(
            r#"
                name = "nc.tests.factory_plugin",
                version = "1.0.0",
                contracts = [],
                factory = RootFactory,
            "#,
        )
        .err()
        .expect("empty contract list must fail");
        assert_eq!(error.to_string(), "factory plugins must reference at least one contract module");
    }

    /// Confirms repeated contract paths are rejected before code generation.
    #[test]
    fn duplicate_contract_module_paths_are_rejected() {
        let error = syn::parse_str::<FactoryPluginArgs>(
            r#"
                name = "nc.tests.factory_plugin",
                version = "1.0.0",
                contracts = [contract::MODULE, contract::MODULE],
                factory = RootFactory,
            "#,
        )
        .err()
        .expect("duplicate contract path must fail");
        assert_eq!(error.to_string(), "duplicate contract module path");
    }
}
