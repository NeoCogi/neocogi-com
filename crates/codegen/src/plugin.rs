//! Parsing and expansion of plugin descriptors.
//!
//! A typed Rust acquisition return supplies the exact generated interface
//! identity, signature and threading. The macro emits immutable metadata, a
//! fixed acquisition adapter and the optional DLL discovery export.

use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{GenericArgument, Ident, ItemFn, LitStr, Path, PathArguments, Result, ReturnType, Token, Type, bracketed, parenthesized};

use crate::args::{PluginGuid, parse_guid_arg};
use crate::util::{PrivateNames, checked_u32_len, runtime_crate_path, stable_guid, validate_version_literal};

/// Normalized inputs to one `plugin!` declaration.
pub struct PluginArgs {
    /// Optional checked plugin ID; omission derives it from `name`.
    plugin_id: Option<PluginGuid>,
    /// Normative dotted plugin name.
    name: LitStr,
    /// Static plugin release version.
    version: Option<LitStr>,
    /// Hard plugin dependencies that must already be registered.
    dependencies: Vec<Path>,
    /// Optional path overriding automatic `neocogi-com` discovery.
    runtime_crate: Option<Path>,
}

impl Parse for PluginArgs {
    /// Parses named fields and rejects ambiguous or repeated declarations.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut plugin_id = None;
        let mut name = None;
        let mut version = None;
        let mut dependencies = None;
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
            } else if keyword == "neocogi_com" {
                parse_unique_value(input, keyword, &mut runtime_crate, "neocogi_com")?;
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected plugin_id(...), name = ..., version = ..., dependencies = [...], or neocogi_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let dependencies = dependencies.unwrap_or_default();
        reject_duplicate_paths(&dependencies, "plugin dependency")?;

        Ok(Self {
            plugin_id,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "expected name = ..."))?,
            version,
            dependencies,
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

/// Emits immutable plugin metadata backed by one exact typed factory interface.
pub fn expand_plugin(args: PluginArgs, input: ItemFn) -> Result<proc_macro2::TokenStream> {
    let nc = runtime_crate_path(args.runtime_crate.as_ref());
    let signature = &input.sig;
    if !signature.inputs.is_empty()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.constness.is_some()
    {
        return Err(syn::Error::new_spanned(
            signature,
            "plugin acquisition must be a safe, non-generic, zero-argument Rust function",
        ));
    }
    let ReturnType::Type(_, output) = &signature.output else {
        return Err(syn::Error::new_spanned(
            signature,
            "plugin acquisition must return Result<ComPtr<InterfaceAbi>, NcStatus>",
        ));
    };
    let owned = first_type_argument(output, "Result")?;
    let interface = first_type_argument(owned, "ComPtr")?;
    if let Some(version) = &args.version {
        validate_version_literal(version)?;
    }
    let identity = args.plugin_id.unwrap_or_else(|| PluginGuid::AutoGuid { value: args.name.clone() });
    let plugin_id = stable_guid(identity, &args.name, &nc)?;
    let dependency_count = checked_u32_len(args.dependencies.len(), args.name.span(), "plugin dependency count")?;
    let dependency_len = args.dependencies.len();
    let dependencies = args.dependencies.iter().map(|dependency| {
        quote! {
            #nc::NcPluginDependencyDescriptor {
                plugin_id: #dependency,
            }
        }
    });
    let acquire = &signature.ident;
    let name = &args.name;
    let version = args
        .version
        .as_ref()
        // This reference is expanded in the provider's macro namespace. A local
        // env! must not replace the compiler's package-version lookup.
        .map_or_else(|| quote! { ::core::env!("CARGO_PKG_VERSION") }, |version| quote! { #version });

    // Seed every authored reference before adding private function-scoped items.
    // The GUID expression can itself contain a caller-provided constant path.
    let paths = &args.dependencies;
    let mut names = PrivateNames::new(quote! { #input #nc #interface #plugin_id #version #(#paths)* });
    let dependencies_name = names.fresh("NC_COM_PLUGIN_DEPENDENCIES");
    let descriptor_name = names.fresh("NC_COM_PLUGIN_DESCRIPTOR");
    let adapter = names.fresh("__nc_acquire_plugin_root");
    let output = names.binding("__nc_output");

    let binding_imports = names.binding_imports();
    Ok(quote! {
        #input

        /// Returns immutable assembly metadata without constructing a root.
        /// Private items have function scope; their names are fresh against every
        /// authored acquisition/dependency reference, including printed source.
        pub const fn plugin_descriptor() -> &'static #nc::NcPluginDescriptor {
        #binding_imports
        /// Hard plugin dependencies required before this interface can be acquired.
        static #dependencies_name: [#nc::NcPluginDependencyDescriptor; #dependency_len] = [
            #(#dependencies,)*
        ];

        /// Immutable plugin metadata consumed by the shared runtime.
        static #descriptor_name: #nc::NcPluginDescriptor = #nc::NcPluginDescriptor {
            description_abi_version: #nc::NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
            core_abi_version: #nc::NC_COM_CORE_ABI_VERSION,
            plugin_id: #plugin_id,
            qualified_name: #nc::NcStaticUtf8::from_static(#name),
            plugin_version: #nc::NcStaticUtf8::from_static(#version),
            dependency_count: #dependency_count,
            dependencies: #dependencies_name.as_ptr(),
            root_factory: <#interface as #nc::ComInterface>::INTERFACE_ID,
            root_factory_threading: <<#interface as #nc::ComInterface>::Threading as #nc::ThreadingModel>::ABI_THREADING_MODE.as_raw(),
            acquire: ::core::option::Option::Some(#adapter),
        };

        /// Transfers one typed root through the fixed acquisition ABI.
        ///
        /// # Safety
        /// `out` must be null or writable for one raw COM pair. The helper clears
        /// usable storage before calling provider code and transfers only success.
        unsafe extern "C" fn #adapter(#output: *mut #nc::ComRaw<#nc::IUnknown>) -> #nc::NcStatus {
            // SAFETY: forwarded output obligations; the typed function fixes the
            // exact interface and threading used by the descriptor above.
            unsafe { #nc::write_plugin_root::<#interface>(#output, #acquire) }
        }

        &#descriptor_name
        }

        // The gate belongs to the provider crate. Static-only local uses need
        // not declare the feature; suppress the intentional gate only here.
        #[allow(unexpected_cfgs)]
        const _: () = {
        #binding_imports
        /// Lends immutable metadata while this dynamic plugin remains mapped.
        ///
        /// # Safety
        /// `out` must be null or writable for one descriptor pointer. The caller
        /// must retain the library while using any reachable metadata or code.
        #[cfg(feature = "dynamic-plugin")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn nc_get_plugin_descriptor_v1(#output: *mut *const #nc::NcPluginDescriptor) -> #nc::NcStatus {
            // SAFETY: the descriptor is immutable static data owned by this library.
            unsafe { #nc::return_plugin_descriptor(#output, plugin_descriptor()) }
        }
        };
    })
}

/// Requires explicit Result/ComPtr syntax so metadata is derived from the typed return.
/// Rust checks the complete function signature, status type and threading afterward.
fn first_type_argument<'a>(ty: &'a Type, expected: &str) -> Result<&'a Type> {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.last() {
            if segment.ident == expected {
                if let PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    if let Some(GenericArgument::Type(argument)) = arguments.args.first() {
                        return Ok(argument);
                    }
                }
            }
        }
    }
    Err(syn::Error::new_spanned(
        ty,
        format!("plugin acquisition requires an explicit {expected}<...> return type"),
    ))
}

#[cfg(test)]
mod tests {
    use super::PluginArgs;

    /// Repeated provider dependencies cannot produce ambiguous registration rows.
    #[test]
    fn duplicate_dependencies_are_rejected() {
        let error = syn::parse_str::<PluginArgs>(
            r#"name = "nc.tests.plugin", version = "1.0.0",
                dependencies = [provider::ID, provider::ID],"#,
        )
        .err()
        .expect("duplicate dependency must fail");
        assert_eq!(error.to_string(), "duplicate plugin dependency path");
    }
}

/// Admission checks run before emitting descriptor or ABI adapter code.
#[cfg(test)]
mod signature_tests {
    /// Every unsupported signature is rejected at its authored function.
    #[test]
    fn rejects_non_bootstrap_function_shapes() {
        for source in [
            "fn root(value: u32) -> Result<ComPtr<I>, NcStatus> { todo!() }",
            "unsafe fn root() -> Result<ComPtr<I>, NcStatus> { todo!() }",
            "async fn root() -> Result<ComPtr<I>, NcStatus> { todo!() }",
            "extern \"C\" fn root() -> Result<ComPtr<I>, NcStatus> { todo!() }",
            "fn root<T>() -> Result<ComPtr<I>, NcStatus> { todo!() }",
            "fn root() -> Result<ComBox<I>, NcStatus> { todo!() }",
            "fn root() -> AliasedResult { todo!() }",
            "fn root() {}",
        ] {
            let args = syn::parse_str("name = \"nc.test.invalid\"").unwrap();
            let function = syn::parse_str(source).unwrap();
            assert!(super::expand_plugin(args, function).is_err(), "{source}");
        }
    }
}
