//! Parsers and normalized option records for neocogi-com attribute macros.
//!
//! Each public macro accepts a small purpose-built grammar. This module parses
//! those tokens once, validates mutually exclusive or all-or-nothing options,
//! and derives the generated ABI/vtable paths needed by later expansion stages.

use quote::{ToTokens, format_ident};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, LitInt, LitStr, Path, Result, Token, Type, parenthesized};

use crate::util::{interface_field, path_with_last_ident};

/// Parsed options for concrete `#[com_object]` storage generation.
pub(crate) struct ClassArgs {
    /// Explicit interface list after deriving ABI and vtable helper paths.
    pub(crate) interfaces: Vec<ClassInterface>,
    /// Whether generated object storage may be shared across threads.
    pub(crate) thread_safe: bool,
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
    /// Optional generated ABI path for a registered callable class contract.
    pub(crate) class_contract: Option<Path>,
    /// Optional generated ABI path for a factory-created object description.
    pub(crate) object_description: Option<Path>,
}

/// Parsed identity and capability fields for an IDL-generated object shape.
pub(crate) struct ObjectDescriptionArgs {
    /// Stable UUID assigned by IDL semantic resolution.
    pub(crate) id: LitStr,
    /// Exact dotted declaration name.
    pub(crate) name: LitStr,
    /// Positive IDL declaration version.
    pub(crate) version: LitInt,
    /// Local or shared threading capability.
    pub(crate) threading: DescriptionThreading,
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
}

/// Threading spellings accepted by generated description attributes.
#[derive(Clone, Copy)]
pub(crate) enum DescriptionThreading {
    /// References and calls remain on one thread.
    Local,
    /// References and calls may cross threads.
    Shared,
}

/// Parsed identity for a callable class contract declared on a Rust trait.
///
/// The trait supplies the method signatures while these fields supply the
/// registry identity shared by hosts and implementations.
pub(crate) struct ClassContractArgs {
    /// Normative class name used to derive its stable registry identity.
    pub(crate) auto_guid: LitStr,
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
}

/// Generated paths and storage-field name for one class interface.
pub(crate) struct ClassInterface {
    /// Path to the macro-generated C-shaped interface header.
    pub(crate) abi_path: Path,
    /// Path to the generated ancestry-header storage for one implementor.
    pub(crate) interfaces_for_path: Path,
    /// Collision-checked field identifier used in class storage.
    pub(crate) field_ident: Ident,
}

/// Parsed options for `#[com_enum]`.
pub(crate) struct EnumArgs {
    /// Integer representation used for stable variant tags.
    pub(crate) tag: Type,
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
    /// Whether to generate format-neutral typed decoding for the Rust enum.
    pub(crate) deserialize: bool,
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
    /// Whether fields decode from an ordered sequence instead of a named record.
    pub(crate) deserialize_sequence: bool,
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
    /// Optional path overriding automatic `neocogi-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
}

/// Source form used to obtain a stable plugin, class, or interface GUID.
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
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected tag = integer_type, deserialize, or neocogi_com = path",
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
        let mut deserialize_sequence = false;

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
            } else if keyword == "sequence" {
                if deserialize_sequence {
                    return Err(syn::Error::new(keyword.span(), "duplicate sequence"));
                }
                deserialize_sequence = true;
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
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected c_struct, deserialize, default, sequence, packed, packed(N), align(N), or neocogi_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        if (deserialize_default || deserialize_sequence) && !deserialize {
            return Err(syn::Error::new(input.span(), "default and sequence require deserialize"));
        }
        if deserialize_default && deserialize_sequence {
            return Err(syn::Error::new(input.span(), "default is only supported for named-record decoding"));
        }

        Ok(Self {
            runtime_crate,
            repr: repr.unwrap_or(StructReprArgs::C),
            c_struct,
            deserialize,
            deserialize_default,
            deserialize_sequence,
        })
    }
}

impl Parse for InterfaceArgs {
    /// Parses interface identity, qualified name, and runtime crate override.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut guid = None;
        let mut qualified_name = None;
        let mut runtime_crate = None;

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
                } else {
                    return Err(syn::Error::new(
                        keyword.span(),
                        "expected UUID string, name = \"stable.name:v1\", auto_guid = \"stable.name:v1\", assigned_guid = \"UUID\", or neocogi_com = path",
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
            runtime_crate,
        })
    }
}

impl Parse for ClassContractArgs {
    /// Parses one derived class identity and an optional runtime crate override.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut auto_guid = None;
        let mut runtime_crate = None;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "auto_guid" {
                if auto_guid.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate auto_guid = ..."));
                }
                input.parse::<Token![=]>()?;
                auto_guid = Some(input.parse()?);
            } else if keyword == "neocogi_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate neocogi_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(keyword.span(), "expected auto_guid = \"stable.name:v1\" or neocogi_com = path"));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            auto_guid: auto_guid.ok_or_else(|| syn::Error::new(input.span(), "expected auto_guid = \"stable.name:v1\""))?,
            runtime_crate,
        })
    }
}

impl Parse for ObjectDescriptionArgs {
    /// Parses the complete generator-owned identity without implicit defaults.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut id: Option<LitStr> = None;
        let mut name: Option<LitStr> = None;
        let mut version: Option<LitInt> = None;
        let mut threading: Option<DescriptionThreading> = None;
        let mut runtime_crate: Option<Path> = None;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "id" {
                if id.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate id = ..."));
                }
                input.parse::<Token![=]>()?;
                id = Some(input.parse()?);
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
            } else if keyword == "threading" {
                if threading.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate threading = ..."));
                }
                input.parse::<Token![=]>()?;
                let value: Ident = input.parse()?;
                threading = Some(if value == "local" {
                    DescriptionThreading::Local
                } else if value == "shared" {
                    DescriptionThreading::Shared
                } else {
                    return Err(syn::Error::new(value.span(), "threading must be `local` or `shared`"));
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
                    "expected id = \"UUID\", name = \"module.Name\", version = N, threading = local|shared, or neocogi_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let version = version.ok_or_else(|| syn::Error::new(input.span(), "expected version = N"))?;
        let parsed_version = version.base10_parse::<u32>()?;
        if parsed_version == 0 {
            return Err(syn::Error::new(version.span(), "description version must be positive"));
        }
        Ok(Self {
            id: id.ok_or_else(|| syn::Error::new(input.span(), "expected id = \"UUID\""))?,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "expected name = \"module.Name\""))?,
            version,
            threading: threading.ok_or_else(|| syn::Error::new(input.span(), "expected threading = local or shared"))?,
            runtime_crate,
        })
    }
}

impl Parse for ClassArgs {
    /// Parses a callable class path or `interfaces(IFoo, module::IBar)` with optional threading.
    ///
    /// Interface paths are intentionally kept to Rust module-style paths for
    /// now. That covers `interfaces(dom::INode)` without accepting generic or
    /// associated-type paths that the generated names cannot represent.
    ///
    /// A bare class path supplies the class identity, constructor frame,
    /// primary projection, and required interface set. Contract-backed
    /// implementations therefore reject even an empty `interfaces(...)` list.
    /// Interface-only local objects do not produce registry descriptors.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut interfaces = None;
        let mut thread_safe = false;
        let mut runtime_crate = None;
        let mut contract = None;
        let mut description = None;

        while !input.is_empty() {
            let starts_qualified_path = input.peek(Token![::]) || input.peek(Token![crate]) || input.peek(Token![self]) || input.peek(Token![super]);
            let starts_named_path = if input.peek(Ident) {
                let lookahead = input.fork();
                let candidate: Ident = lookahead.parse()?;
                !matches!(
                    candidate.to_string().as_str(),
                    "interfaces" | "constructible" | "thread_safe" | "neocogi_com" | "class_id" | "name" | "version" | "constructor"
                )
            } else {
                false
            };
            if starts_qualified_path || starts_named_path {
                if description.is_some() {
                    return Err(syn::Error::new(input.span(), "duplicate object description path"));
                }
                description = Some(Path::parse_mod_style(input)?);
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
                validate_class_interface_paths(paths.iter())?;
                interfaces = Some(paths.into_iter().map(ClassInterface::from_path).collect::<Result<Vec<_>>>()?);
            } else if keyword == "constructible" {
                if contract.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate constructible(...) path"));
                }
                let content;
                parenthesized!(content in input);
                contract = Some(Path::parse_mod_style(&content)?);
                if !content.is_empty() {
                    return Err(content.error("constructible(...) accepts exactly one class path"));
                }
            } else if keyword == "thread_safe" {
                if thread_safe {
                    return Err(syn::Error::new(keyword.span(), "duplicate thread_safe"));
                }
                thread_safe = true;
            } else if keyword == "neocogi_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate neocogi_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected an object description path, constructible(...), interfaces(...), thread_safe, or neocogi_com = path",
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
        if contract.is_some() && description.is_some() {
            return Err(syn::Error::new(input.span(), "choose an object description or constructible(...), not both"));
        }
        let selected = description.clone().or_else(|| contract.clone());
        if let Some(selected) = selected {
            let primary = ClassInterface::from_path(selected)?;
            if interfaces_were_declared {
                return Err(syn::Error::new(
                    input.span(),
                    "a described object declares required interfaces as trait supertraits; remove interfaces(...) from its implementation",
                ));
            }
            interfaces.insert(0, primary);
        }
        let class_contract = contract.map(|path| interface_abi_path(&path)).transpose()?;
        let object_description = description.map(|path| interface_abi_path(&path)).transpose()?;

        Ok(Self {
            interfaces,
            thread_safe,
            runtime_crate,
            class_contract,
            object_description,
        })
    }
}

impl ClassInterface {
    /// Derives generated ABI, vtable, and storage names from a trait path.
    fn from_path(path: Path) -> Result<Self> {
        let abi_path = interface_abi_path(&path)?;
        let interfaces_for_path = interface_interfaces_for_path(&path)?;
        let field_ident = interface_field(&path);

        Ok(Self {
            abi_path,
            interfaces_for_path,
            field_ident,
        })
    }
}

/// Rejects explicit `IUnknown`, duplicate paths, and generated-name collisions.
fn validate_class_interface_paths<'a>(paths: impl Iterator<Item = &'a Path>) -> Result<()> {
    let mut seen_paths = Vec::new();
    let mut seen_names = Vec::new();

    for path in paths {
        let Some(last) = path.segments.last() else {
            return Err(syn::Error::new(path.span(), "expected interface path"));
        };
        if last.ident == "IUnknown" {
            return Err(syn::Error::new_spanned(
                path,
                "IUnknown is implicit for every COM class; remove it from interfaces(...)",
            ));
        }

        let path_key = path.to_token_stream().to_string();
        let name_key = last.ident.to_string();
        if seen_paths.contains(&path_key) {
            return Err(syn::Error::new_spanned(path, "duplicate COM interface in interfaces(...)"));
        }
        if seen_names.contains(&name_key) {
            return Err(syn::Error::new_spanned(path, "COM interfaces in one class must have distinct final path names"));
        }
        seen_paths.push(path_key);
        seen_names.push(name_key);
    }

    Ok(())
}

/// Replaces an interface trait's final path segment with its generated ABI type.
fn interface_abi_path(path: &Path) -> Result<Path> {
    let Some(last) = path.segments.last() else {
        return Err(syn::Error::new(path.span(), "expected interface path"));
    };
    let trait_ident = &last.ident;
    let abi_ident = format_ident!("{trait_ident}Abi", span = trait_ident.span());
    Ok(path_with_last_ident(path, abi_ident))
}

/// Replaces the final trait segment with its generated ancestry storage type.
fn interface_interfaces_for_path(path: &Path) -> Result<Path> {
    let Some(last) = path.segments.last() else {
        return Err(syn::Error::new(path.span(), "expected interface path"));
    };
    let trait_ident = &last.ident;
    let vtable_for_ident = format_ident!("{trait_ident}InterfacesFor", span = trait_ident.span());
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
