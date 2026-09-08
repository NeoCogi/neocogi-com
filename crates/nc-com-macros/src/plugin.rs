//! Parsing and expansion of complete static plugin descriptors and entrypoints.
//!
//! `plugin!` combines stable plugin identity, dependency IDs, and generated
//! class descriptors into immutable tables. It emits the standard unmangled
//! descriptor symbol plus an ordinary Rust accessor over the same static root.

use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, LitStr, Path, Result, Token, bracketed, parenthesized};

use crate::args::{InterfaceGuid, parse_guid_arg};
use crate::util::{checked_u32_len, runtime_crate_path, stable_guid, validate_version_literal};

/// Parsed inputs to the `plugin!` declaration macro.
pub(crate) struct PluginArgs {
    /// Explicit, derived, or constant-path plugin identity.
    plugin_id: InterfaceGuid,
    /// Normative plugin qualified name.
    name: LitStr,
    /// Plugin version text.
    version: LitStr,
    /// Ordered hard dependency ID paths.
    dependencies: Vec<Path>,
    /// Ordered paths to macro-generated class descriptors.
    classes: Vec<Path>,
    /// Optional path overriding automatic `nc-com` crate discovery.
    runtime_crate: Option<Path>,
}

impl Parse for PluginArgs {
    /// Parses required plugin fields and rejects duplicate dependency paths.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut plugin_id = None;
        let mut name = None;
        let mut version = None;
        let mut dependencies = None;
        let mut classes = None;
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
                if name.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate name = ..."));
                }
                input.parse::<Token![=]>()?;
                name = Some(input.parse()?);
            } else if keyword == "version" {
                if version.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate version = ..."));
                }
                input.parse::<Token![=]>()?;
                version = Some(input.parse()?);
            } else if keyword == "dependencies" {
                if dependencies.is_some() {
                    return Err(syn::Error::new(
                        keyword.span(),
                        "duplicate dependencies = [...]",
                    ));
                }
                input.parse::<Token![=]>()?;
                let content;
                bracketed!(content in input);
                dependencies = Some(
                    content
                        .parse_terminated(Path::parse_mod_style, Token![,])?
                        .into_iter()
                        .collect::<Vec<_>>(),
                );
            } else if keyword == "classes" {
                if classes.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate classes = [...]"));
                }
                input.parse::<Token![=]>()?;
                let content;
                bracketed!(content in input);
                classes = Some(
                    content
                        .parse_terminated(Path::parse_mod_style, Token![,])?
                        .into_iter()
                        .collect::<Vec<_>>(),
                );
            } else if keyword == "nc_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate nc_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected plugin_id(...), name = ..., version = ..., dependencies = [...], classes = [...], or nc_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let dependencies = dependencies.unwrap_or_default();
        for (index, dependency) in dependencies.iter().enumerate() {
            let dependency_tokens = dependency.to_token_stream().to_string();
            if dependencies[..index]
                .iter()
                .any(|other| other.to_token_stream().to_string() == dependency_tokens)
            {
                return Err(syn::Error::new(
                    dependency.span(),
                    "duplicate plugin dependency path",
                ));
            }
        }

        Ok(Self {
            plugin_id: plugin_id
                .ok_or_else(|| syn::Error::new(input.span(), "expected plugin_id(...)"))?,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "expected name = ..."))?,
            version: version
                .ok_or_else(|| syn::Error::new(input.span(), "expected version = ..."))?,
            dependencies,
            classes: classes
                .ok_or_else(|| syn::Error::new(input.span(), "expected classes = [...]"))?,
            runtime_crate,
        })
    }
}

/// Emits static dependency/class tables, plugin root, and descriptor entrypoint.
pub(crate) fn expand_plugin(args: PluginArgs) -> proc_macro2::TokenStream {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let plugin_id = match stable_guid(args.plugin_id, &args.name, &nc) {
        Ok(plugin_id) => plugin_id,
        Err(err) => return err.to_compile_error(),
    };
    if let Err(err) = validate_version_literal(&args.version) {
        return err.to_compile_error();
    }
    let dependency_count_u32 = match checked_u32_len(
        args.dependencies.len(),
        args.name.span(),
        "plugin dependency count",
    ) {
        Ok(count) => count,
        Err(err) => return err.to_compile_error(),
    };
    let class_count_u32 =
        match checked_u32_len(args.classes.len(), args.name.span(), "plugin class count") {
            Ok(count) => count,
            Err(err) => return err.to_compile_error(),
        };
    let name = args.name;
    let version = args.version;
    let dependency_count = args.dependencies.len();
    let dependencies = args.dependencies.iter().map(|dependency| {
        quote! {
            #nc::NcPluginDependencyDescriptor {
                plugin_id: #dependency,
            }
        }
    });
    let class_count = args.classes.len();
    let class_descriptors = args.classes.iter().map(|class| {
        quote! { <#class>::CLASS_DESCRIPTOR }
    });

    quote! {
        // The plugin macro owns the descriptor statics for this compilation
        // unit. Class descriptors are copied by value from `#[com_object]`
        // output so hosts see one contiguous, immutable class table.
        static NC_COM_PLUGIN_DEPENDENCIES: [#nc::NcPluginDependencyDescriptor; #dependency_count] = [
            #(#dependencies,)*
        ];

        static NC_COM_PLUGIN_CLASSES: [#nc::NcClassDescriptor; #class_count] = [
            #(#class_descriptors,)*
        ];

        static NC_COM_PLUGIN_CLASS_TABLE: #nc::NcClassTable = #nc::NcClassTable {
            class_count: #class_count_u32,
            classes: NC_COM_PLUGIN_CLASSES.as_ptr(),
        };

        static NC_COM_PLUGIN_DESCRIPTOR: #nc::NcPluginDescriptor = #nc::NcPluginDescriptor {
            core_abi_version: #nc::NC_COM_CORE_ABI_VERSION,
            plugin_id: #plugin_id,
            qualified_name: #nc::NcStaticUtf8::from_static(#name),
            plugin_version: #nc::NcStaticUtf8::from_static(#version),
            dependency_count: #dependency_count_u32,
            dependencies: NC_COM_PLUGIN_DEPENDENCIES.as_ptr(),
            class_table: &NC_COM_PLUGIN_CLASS_TABLE,
        };

        #[doc = "Returns the static plugin descriptor generated by `nc_com::plugin!`."]
        pub fn plugin_descriptor() -> &'static #nc::NcPluginDescriptor {
            &NC_COM_PLUGIN_DESCRIPTOR
        }

    }
}

#[cfg(test)]
mod tests {
    use super::PluginArgs;

    #[test]
    fn duplicate_dependency_paths_are_rejected() {
        let error = syn::parse_str::<PluginArgs>(
            r#"
                plugin_id(auto_guid = "nc.tests.plugin:v1"),
                name = "nc.tests.plugin:v1",
                version = "1.0.0",
                dependencies = [BASE_PLUGIN_ID, BASE_PLUGIN_ID],
                classes = [],
            "#,
        )
        .err()
        .expect("duplicate dependency must fail");
        assert_eq!(error.to_string(), "duplicate plugin dependency path");
    }
}
