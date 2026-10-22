//! Parsers and normalized option records for neocogi-com attribute macros.
//!
//! Each public macro accepts a small purpose-built grammar. This module parses
//! those tokens once, validates mutually exclusive or all-or-nothing options,
//! and derives the generated ABI/vtable paths needed by later expansion stages.

use quote::ToTokens;
use neocogi_com_core::NcComInterfaceCompanion as Companion;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, LitStr, Path, Result, Token, parenthesized};

use crate::util::{interface_companion_ident, path_with_last_ident};

/// Parsed options for concrete `#[com_object]` storage generation.
pub struct ObjectArgs {
    /// Explicit interface list after deriving ABI and vtable helper paths.
    pub interfaces: Vec<ObjectInterface>,
    /// Constructor and call-lifetime policy fixed for every instance of this class.
    pub ownership: ObjectOwnership,
    /// Whether generated object storage may be shared across threads.
    pub thread_safe: bool,
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub runtime_crate: Option<Path>,
    /// Optional generated ABI path for a factory-created interface contract.
    pub primary_interface: Option<Path>,
}

/// Provider-side lifetime policy; interface ABI and threading remain independent.
#[derive(Clone, Copy)]
pub enum ObjectOwnership {
    /// Strong references keep the implementation alive throughout each call.
    Shared,
    /// One owner controls retirement; weak calls need temporary admission.
    Exclusive,
}

/// Threading requirement already resolved by the IDL compiler.
#[derive(Clone, Copy)]
pub enum InterfaceThreading {
    /// References and calls remain on one thread.
    Local,
    /// References and calls may cross threads.
    Shared,
}

/// Generated contract and statically typed provider paths for one object interface.
pub struct ObjectInterface {
    /// Path to the macro-generated interface marker.
    pub abi_path: Path,
    /// Path to the generated vtable provider and canonical binding table.
    pub vtable_for_path: Path,
}

/// Plugin-only identity spelling; callable identity is resolved IDL data.
#[derive(Clone)]
pub enum PluginGuid {
    /// Canonical UUID string parsed during macro expansion.
    Explicit(LitStr),
    /// UUID-v5 derivation from the exact qualified-name literal.
    AutoGuid {
        /// Qualified-name bytes used as the derivation input.
        value: LitStr,
    },
    /// Existing constant or expression path supplied by the user.
    Path(Path),
}
impl Parse for ObjectArgs {
    /// Parses a primary interface path or `interfaces(IFoo, module::IBar)` with optional threading.
    ///
    /// Interface paths are intentionally kept to Rust module-style paths for
    /// now. That covers `interfaces(dom::INode)` without accepting generic or
    /// associated-type paths that the generated names cannot represent.
    ///
    /// A bare interface path supplies the primary projection, threading promise,
    /// and required siblings. Its implementation cannot redeclare those facts
    /// with an additional list or threading flag. The explicit-list form is for
    /// objects exposing several independent projections. Ownership is independent
    /// of those interfaces and defaults to shared; an implementation can select exclusive.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut interfaces = None;
        let mut thread_safe = false;
        let mut runtime_crate = None;
        let mut primary = None;
        let mut ownership = None;

        while !input.is_empty() {
            let starts_qualified_path = input.peek(Token![::]) || input.peek(Token![crate]) || input.peek(Token![self]) || input.peek(Token![super]);
            let starts_named_path = if input.peek(Ident) {
                let lookahead = input.fork();
                let candidate: Ident = lookahead.parse()?;
                lookahead.peek(Token![::]) || !matches!(candidate.to_string().as_str(), "interfaces" | "thread_safe" | "neocogi_com" | "ownership")
            } else {
                false
            };
            if starts_qualified_path || starts_named_path {
                if primary.is_some() {
                    return Err(syn::Error::new(input.span(), "duplicate primary interface path"));
                }
                primary = Some(Path::parse_mod_style(input)?);
                if input.is_empty() {
                    break;
                }
                input.parse::<Token![,]>()?;
                continue;
            }

            let keyword: Ident = input.parse()?;
            if keyword == "interfaces" {
                if interfaces.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate interfaces(...)"));
                }

                let content;
                parenthesized!(content in input);
                let paths = content.parse_terminated(Path::parse_mod_style, Token![,])?;
                validate_object_interface_paths(paths.iter())?;
                interfaces = Some(paths.into_iter().map(ObjectInterface::from_path).collect::<Result<Vec<_>>>()?);
            } else if keyword == "thread_safe" {
                if thread_safe {
                    return Err(syn::Error::new(keyword.span(), "duplicate thread_safe"));
                }
                thread_safe = true;
            } else if keyword == "ownership" {
                if ownership.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate ownership = ..."));
                }
                input.parse::<Token![=]>()?;
                let policy: Ident = input.parse()?;
                ownership = Some(match policy.to_string().as_str() {
                    "shared" => ObjectOwnership::Shared,
                    "exclusive" => ObjectOwnership::Exclusive,
                    _ => return Err(syn::Error::new(policy.span(), "expected shared or exclusive ownership")),
                });
            } else if keyword == "neocogi_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate neocogi_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected a primary interface path, interfaces(...), ownership = shared/exclusive, thread_safe, or neocogi_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        // Preserve whether the author wrote the option before normalizing its
        // contents. An empty list is still redundant on a callable contract.
        let interfaces_were_declared = interfaces.is_some();
        let mut interfaces = interfaces.unwrap_or_default();
        if let Some(selected) = primary.clone() {
            if thread_safe {
                return Err(syn::Error::new(
                    input.span(),
                    "the primary interface owns its threading promise; remove thread_safe",
                ));
            }
            let primary = ObjectInterface::from_path(selected)?;
            if interfaces_were_declared {
                return Err(syn::Error::new(
                    input.span(),
                    "the primary interface declares its required projections as supertraits; remove interfaces(...) from its implementation",
                ));
            }
            interfaces.insert(0, primary);
        }
        let primary_interface = primary.clone().map(|path| interface_abi_path(&path)).transpose()?;

        Ok(Self {
            interfaces,
            thread_safe,
            runtime_crate,
            primary_interface,
            ownership: ownership.unwrap_or(ObjectOwnership::Shared),
        })
    }
}

impl ObjectInterface {
    /// Derives generated ABI and vtable-provider names from a trait path.
    fn from_path(path: Path) -> Result<Self> {
        let abi_path = interface_abi_path(&path)?;
        let vtable_for_path = interface_vtable_for_path(&path)?;

        Ok(Self { abi_path, vtable_for_path })
    }
}

/// Rejects explicit `IUnknown` and repeated authored interface paths.
fn validate_object_interface_paths<'a>(paths: impl Iterator<Item = &'a Path>) -> Result<()> {
    let mut seen_paths = Vec::new();

    for path in paths {
        let Some(last) = path.segments.last() else {
            return Err(syn::Error::new(path.span(), "expected interface path"));
        };
        if last.ident == "IUnknown" {
            return Err(syn::Error::new_spanned(
                path,
                "IUnknown is implicit for every COM object; remove it from interfaces(...)",
            ));
        }

        let path_key = path.to_token_stream().to_string();
        if seen_paths.contains(&path_key) {
            return Err(syn::Error::new_spanned(path, "duplicate COM interface in interfaces(...)"));
        }
        seen_paths.push(path_key);
    }

    Ok(())
}

/// Replaces an interface trait's final path segment with its generated ABI type.
fn interface_abi_path(path: &Path) -> Result<Path> {
    let Some(last) = path.segments.last() else {
        return Err(syn::Error::new(path.span(), "expected interface path"));
    };
    let trait_ident = &last.ident;
    let abi_ident = interface_companion_ident(trait_ident, Companion::Abi);
    Ok(path_with_last_ident(path, abi_ident))
}

/// Replaces the final trait segment with its statically typed vtable provider.
fn interface_vtable_for_path(path: &Path) -> Result<Path> {
    let Some(last) = path.segments.last() else {
        return Err(syn::Error::new(path.span(), "expected interface path"));
    };
    let trait_ident = &last.ident;
    let vtable_for_ident = interface_companion_ident(trait_ident, Companion::Provider);
    Ok(path_with_last_ident(path, vtable_for_ident))
}

/// Parses a GUID literal, `auto_guid = ...`, or existing constant path.
pub fn parse_guid_arg(input: ParseStream<'_>, name: &str) -> Result<PluginGuid> {
    let guid = if input.peek(LitStr) {
        PluginGuid::Explicit(input.parse()?)
    } else {
        let fork = input.fork();
        let keyword: Ident = fork.parse()?;
        if keyword == "auto_guid" && fork.peek(Token![=]) {
            input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;
            PluginGuid::AutoGuid { value: input.parse()? }
        } else {
            PluginGuid::Path(Path::parse_mod_style(input)?)
        }
    };

    if !input.is_empty() {
        input.parse::<Token![,]>()?;
        if !input.is_empty() {
            return Err(syn::Error::new(input.span(), format!("unexpected extra tokens in {name}(...)")));
        }
    }

    Ok(guid)
}
