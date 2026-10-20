//! Parsers and normalized option records for neocogi-com attribute macros.
//!
//! Each public macro accepts a small purpose-built grammar. This module parses
//! those tokens once, validates mutually exclusive or all-or-nothing options,
//! and derives the generated ABI/vtable paths needed by later expansion stages.

use quote::ToTokens;
use neocogi_com_core::NcComInterfaceCompanion as Companion;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, LitByteStr, LitInt, LitStr, Path, Result, Token, Type, parenthesized};

use crate::util::{interface_companion_ident, path_with_last_ident};

/// Parsed options for concrete `#[com_object]` storage generation.
pub(crate) struct ObjectArgs {
    /// Explicit interface list after deriving ABI and vtable helper paths.
    pub(crate) interfaces: Vec<ObjectInterface>,
    /// Constructor and call-lifetime policy fixed for every instance of this class.
    pub(crate) ownership: ObjectOwnership,
    /// Whether generated object storage may be shared across threads.
    pub(crate) thread_safe: bool,
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
    /// Optional generated ABI path for a factory-created interface contract.
    pub(crate) primary_interface: Option<Path>,
}

/// Provider-side lifetime policy; interface ABI and threading remain independent.
#[derive(Clone, Copy)]
pub(crate) enum ObjectOwnership {
    /// Strong references keep the implementation alive throughout each call.
    Shared,
    /// One owner controls retirement; weak calls need temporary admission.
    Exclusive,
}

/// Threading spellings accepted by generated interface attributes.
#[derive(Clone, Copy)]
pub(crate) enum InterfaceThreading {
    /// References and calls remain on one thread.
    Local,
    /// References and calls may cross threads.
    Shared,
}

/// Generated contract and statically typed provider paths for one object interface.
pub(crate) struct ObjectInterface {
    /// Path to the macro-generated interface marker.
    pub(crate) abi_path: Path,
    /// Path to the generated vtable provider and canonical binding table.
    pub(crate) vtable_for_path: Path,
}

/// Parsed options for `#[com_enum]`.
pub(crate) struct EnumArgs {
    /// Integer representation used for stable variant tags.
    pub(crate) tag: Type,
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
    /// Whether to generate format-neutral typed decoding for the Rust enum.
    pub(crate) deserialize: bool,
    /// Generator-computed declaration signature required by expansion.
    pub(crate) assigned_signature: Option<LitByteStr>,
}

/// Parsed options for `#[com_struct]`.
pub(crate) struct StructArgs {
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
    /// Requested C, packed, or explicitly aligned representation.
    pub(crate) repr: StructReprArgs,
    /// Check external field layouts through the compiler instead of COM traits.
    pub(crate) c_struct: bool,
    /// Whether to generate format-neutral typed decoding for the record.
    pub(crate) deserialize: bool,
    /// Whether omitted named fields retain their `Default` values.
    pub(crate) deserialize_default: bool,
    /// Generator-computed declaration signature required by expansion.
    pub(crate) assigned_signature: Option<LitByteStr>,
}

/// Supported physical layout modifiers for generated COM structs.
pub(crate) enum StructReprArgs {
    /// Ordinary `repr(C)` layout.
    C,
    /// Packed layout, optionally with an explicit packing alignment.
    Packed(Option<LitInt>),
    /// C layout with an additional explicit alignment.
    Align(LitInt),
}

/// Parsed identity and crate-path options for `#[com_interface]`.
pub(crate) struct InterfaceArgs {
    /// Explicit, derived, or constant-path interface GUID.
    pub(crate) guid: InterfaceGuid,
    /// Normative qualified name stored in descriptor metadata.
    pub(crate) qualified_name: LitStr,
    /// Generator-computed declaration signature required by expansion.
    pub(crate) assigned_signature: Option<LitByteStr>,
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
    /// Minimum capability promised by this callable contract.
    pub(crate) threading: InterfaceThreading,
}

/// Source form used to obtain a stable plugin, object, or interface GUID.
#[derive(Clone)]
pub(crate) enum InterfaceGuid {
    /// Canonical UUID string parsed during macro expansion.
    Explicit(LitStr),
    /// UUID-v5 derivation from the exact qualified-name literal.
    AutoGuid {
        /// Qualified-name bytes used as the derivation input.
        value: LitStr,
    },
    /// Existing constant or expression path supplied by the user.
    Path(Path),
    /// IDL-assigned UUID whose version is represented separately from its name.
    Assigned(LitStr),
}
impl Parse for EnumArgs {
    /// Parses the required tag type and optional runtime crate override.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut tag = None;
        let mut runtime_crate = None;
        let mut deserialize = false;
        let mut assigned_signature = None;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "tag" {
                if tag.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate tag = ..."));
                }
                input.parse::<Token![=]>()?;
                tag = Some(input.parse()?);
            } else if keyword == "neocogi_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate neocogi_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else if keyword == "deserialize" {
                if deserialize {
                    return Err(syn::Error::new(keyword.span(), "duplicate deserialize"));
                }
                deserialize = true;
            } else if keyword == "assigned_signature" {
                parse_assigned_signature(input, keyword, &mut assigned_signature)?;
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected tag = integer_type, deserialize, assigned_signature = b\"32 bytes\", or neocogi_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            tag: tag.ok_or_else(|| syn::Error::new(input.span(), "expected tag = integer_type"))?,
            runtime_crate,
            deserialize,
            assigned_signature,
        })
    }
}

impl Parse for StructArgs {
    /// Parses one optional layout modifier and runtime crate override.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut runtime_crate = None;
        let mut repr = None;
        let mut c_struct = false;
        let mut deserialize = false;
        let mut deserialize_default = false;
        let mut assigned_signature = None;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "neocogi_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate neocogi_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else if keyword == "c_struct" {
                if c_struct {
                    return Err(syn::Error::new(keyword.span(), "duplicate c_struct"));
                }
                c_struct = true;
            } else if keyword == "deserialize" {
                if deserialize {
                    return Err(syn::Error::new(keyword.span(), "duplicate deserialize"));
                }
                deserialize = true;
            } else if keyword == "default" {
                if deserialize_default {
                    return Err(syn::Error::new(keyword.span(), "duplicate default"));
                }
                deserialize_default = true;
            } else if keyword == "packed" {
                if repr.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate COM struct layout option"));
                }

                let packed = if input.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    let value = content.parse::<LitInt>()?;
                    if !content.is_empty() {
                        return Err(syn::Error::new(content.span(), "expected packed(N)"));
                    }
                    Some(value)
                } else {
                    None
                };
                repr = Some(StructReprArgs::Packed(packed));
            } else if keyword == "align" {
                if repr.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate COM struct layout option"));
                }

                let content;
                parenthesized!(content in input);
                let value = content.parse::<LitInt>()?;
                if !content.is_empty() {
                    return Err(syn::Error::new(content.span(), "expected align(N)"));
                }
                repr = Some(StructReprArgs::Align(value));
            } else if keyword == "assigned_signature" {
                parse_assigned_signature(input, keyword, &mut assigned_signature)?;
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected c_struct, deserialize, default, packed, packed(N), align(N), assigned_signature = b\"32 bytes\", or neocogi_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        if deserialize_default && !deserialize {
            return Err(syn::Error::new(input.span(), "default requires deserialize"));
        }

        Ok(Self {
            runtime_crate,
            repr: repr.unwrap_or(StructReprArgs::C),
            c_struct,
            deserialize,
            deserialize_default,
            assigned_signature,
        })
    }
}

impl Parse for InterfaceArgs {
    /// Parses interface identity, qualified name, and runtime crate override.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut guid = None;
        let mut qualified_name = None;
        let mut runtime_crate = None;
        let mut assigned_signature = None;
        let mut threading = None;
        while !input.is_empty() {
            if input.peek(LitStr) {
                if guid.is_some() {
                    let value: LitStr = input.parse()?;
                    return Err(syn::Error::new(value.span(), "duplicate interface ID"));
                }
                guid = Some(InterfaceGuid::Explicit(input.parse()?));
            } else {
                let keyword: Ident = input.parse()?;
                if keyword == "auto_guid" {
                    if guid.is_some() {
                        return Err(syn::Error::new(keyword.span(), "duplicate interface ID"));
                    }
                    input.parse::<Token![=]>()?;
                    let value: LitStr = input.parse()?;
                    if qualified_name.is_some() {
                        return Err(syn::Error::new(value.span(), "auto_guid already supplies the qualified name"));
                    }
                    qualified_name = Some(value.clone());
                    guid = Some(InterfaceGuid::AutoGuid { value });
                } else if keyword == "assigned_guid" {
                    if guid.is_some() {
                        return Err(syn::Error::new(keyword.span(), "duplicate interface ID"));
                    }
                    input.parse::<Token![=]>()?;
                    guid = Some(InterfaceGuid::Assigned(input.parse()?));
                } else if keyword == "name" {
                    if qualified_name.is_some() {
                        return Err(syn::Error::new(keyword.span(), "duplicate interface qualified name"));
                    }
                    input.parse::<Token![=]>()?;
                    qualified_name = Some(input.parse()?);
                } else if keyword == "neocogi_com" {
                    if runtime_crate.is_some() {
                        return Err(syn::Error::new(keyword.span(), "duplicate neocogi_com = ..."));
                    }
                    input.parse::<Token![=]>()?;
                    runtime_crate = Some(input.parse()?);
                } else if keyword == "threading" {
                    if threading.is_some() {
                        return Err(syn::Error::new(keyword.span(), "duplicate threading = ..."));
                    }
                    input.parse::<Token![=]>()?;
                    let value: Ident = input.parse()?;
                    threading = Some(match value.to_string().as_str() {
                        "local" => InterfaceThreading::Local,
                        "shared" => InterfaceThreading::Shared,
                        _ => return Err(syn::Error::new(value.span(), "threading must be `local` or `shared`")),
                    });
                } else if keyword == "assigned_signature" {
                    parse_assigned_signature(input, keyword, &mut assigned_signature)?;
                } else {
                    return Err(syn::Error::new(
                        keyword.span(),
                        "expected UUID string, name = \"stable.name:v1\", auto_guid = \"stable.name:v1\", assigned_guid = \"UUID\", assigned_signature = b\"32 bytes\", or neocogi_com = path",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            guid: guid.ok_or_else(|| syn::Error::new(input.span(), "expected UUID string or auto_guid = \"stable.name:v1\""))?,
            qualified_name: qualified_name.ok_or_else(|| syn::Error::new(input.span(), "explicit interface UUIDs require name = \"stable.name:v1\""))?,
            assigned_signature,
            runtime_crate,
            threading: threading.unwrap_or(InterfaceThreading::Local),
        })
    }
}

/// Parses one generator-owned SHA-256 byte array exactly once.
fn parse_assigned_signature(input: ParseStream<'_>, keyword: Ident, slot: &mut Option<LitByteStr>) -> Result<()> {
    if slot.is_some() {
        return Err(syn::Error::new(keyword.span(), "duplicate assigned_signature = ..."));
    }
    input.parse::<Token![=]>()?;
    let signature: LitByteStr = input.parse()?;
    if signature.value().len() != 32 {
        return Err(syn::Error::new_spanned(signature, "assigned_signature must contain exactly 32 bytes"));
    }
    *slot = Some(signature);
    Ok(())
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
    /// of those interfaces and defaults to shared; widgets opt into exclusive.
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
                !matches!(candidate.to_string().as_str(), "interfaces" | "thread_safe" | "neocogi_com" | "ownership")
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
pub(crate) fn parse_guid_arg(input: ParseStream<'_>, name: &str) -> Result<InterfaceGuid> {
    let guid = if input.peek(LitStr) {
        InterfaceGuid::Explicit(input.parse()?)
    } else {
        let fork = input.fork();
        let keyword: Ident = fork.parse()?;
        if keyword == "auto_guid" && fork.peek(Token![=]) {
            input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;
            InterfaceGuid::AutoGuid { value: input.parse()? }
        } else {
            InterfaceGuid::Path(Path::parse_mod_style(input)?)
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
