//! Parsers and normalized option records for nc-com attribute macros.
//!
//! Each public macro accepts a small purpose-built grammar. This module parses
//! those tokens once, validates mutually exclusive or all-or-nothing options,
//! and derives the generated ABI/vtable paths needed by later expansion stages.

use quote::{ToTokens, format_ident};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, LitInt, LitStr, Path, Result, Token, Type, parenthesized};

use crate::util::{interface_field, path_with_last_ident};

/// Parsed options for `#[com_class]`.
pub(crate) struct ClassArgs {
    /// Explicit interface list after deriving ABI and vtable helper paths.
    pub(crate) interfaces: Vec<ClassInterface>,
    /// Whether generated object storage may be shared across threads.
    pub(crate) thread_safe: bool,
    /// Optional path overriding automatic `nc-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
    /// Optional all-or-nothing class descriptor and constructor metadata.
    pub(crate) descriptor: Option<ClassDescriptorArgs>,
}

/// Parsed identity for a callable class contract declared on a Rust trait.
///
/// The trait supplies the method signatures while these fields supply the
/// registry identity shared by hosts and implementations.
pub(crate) struct ClassContractArgs {
    /// Explicit, derived, or path-based stable class identity.
    pub(crate) class_id: InterfaceGuid,
    /// Normative class qualified name.
    pub(crate) name: LitStr,
    /// Class contract version text.
    pub(crate) version: LitStr,
    /// Optional path overriding automatic `nc-com` crate discovery.
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

/// Optional descriptor metadata attached to a generated COM class.
pub(crate) enum ClassDescriptorArgs {
    /// An implementation-owned identity written directly on its struct.
    Explicit {
        /// Explicit, derived, or path-based stable class identity.
        class_id: InterfaceGuid,
        /// Normative class qualified name.
        name: LitStr,
        /// Class implementation version text.
        version: LitStr,
        /// Constructor argument and factory configuration.
        constructor: ClassConstructorArgs,
    },
    /// An implementation of one shared callable class contract.
    Contract {
        /// Generated ABI path carrying the shared class identity and methods.
        abi_path: Path,
        /// Constructor implementation policy for this concrete object.
        constructor: ClassConstructorArgs,
    },
}

/// Parsed constructor policy for a descriptor-bearing class.
pub(crate) struct ClassConstructorArgs {
    /// Required argument interface, or no arguments.
    pub(crate) arguments: ConstructorArguments,
    /// Optional user factory invoked by the generated constructor thunk.
    pub(crate) function: Option<Path>,
    /// Whether a no-argument class should be built with `Default`.
    pub(crate) use_default: bool,
}

/// Constructor argument interface selection.
pub(crate) enum ConstructorArguments {
    /// The constructor rejects non-null argument objects.
    None,
    /// The constructor requires an object supporting the generated ABI interface.
    Interface {
        /// Path to the required generated interface header.
        abi_path: Path,
    },
}

/// Parsed options for `#[com_enum]`.
pub(crate) struct EnumArgs {
    /// Integer representation used for stable variant tags.
    pub(crate) tag: Type,
    /// Optional path overriding automatic `nc-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
}

/// Parsed options for `#[com_struct]`.
pub(crate) struct StructArgs {
    /// Optional path overriding automatic `nc-com` crate discovery.
    pub(crate) runtime_crate: Option<Path>,
    /// Requested C, packed, or explicitly aligned representation.
    pub(crate) repr: StructReprArgs,
    /// Check external field layouts through the compiler instead of COM traits.
    pub(crate) c_struct: bool,
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
    /// Optional path overriding automatic `nc-com` crate discovery.
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
}
impl Parse for EnumArgs {
    /// Parses the required tag type and optional runtime crate override.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut tag = None;
        let mut runtime_crate = None;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "tag" {
                if tag.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate tag = ..."));
                }
                input.parse::<Token![=]>()?;
                tag = Some(input.parse()?);
            } else if keyword == "nc_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate nc_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected tag = integer_type or nc_com = path",
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
        })
    }
}

impl Parse for StructArgs {
    /// Parses one optional layout modifier and runtime crate override.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut runtime_crate = None;
        let mut repr = None;
        let mut c_struct = false;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "nc_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate nc_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else if keyword == "c_struct" {
                if c_struct {
                    return Err(syn::Error::new(keyword.span(), "duplicate c_struct"));
                }
                c_struct = true;
            } else if keyword == "packed" {
                if repr.is_some() {
                    return Err(syn::Error::new(
                        keyword.span(),
                        "duplicate COM struct layout option",
                    ));
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
                    return Err(syn::Error::new(
                        keyword.span(),
                        "duplicate COM struct layout option",
                    ));
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
                    "expected c_struct, packed, packed(N), align(N), or nc_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            runtime_crate,
            repr: repr.unwrap_or(StructReprArgs::C),
            c_struct,
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
                        return Err(syn::Error::new(
                            value.span(),
                            "auto_guid already supplies the qualified name",
                        ));
                    }
                    qualified_name = Some(value.clone());
                    guid = Some(InterfaceGuid::AutoGuid { value });
                } else if keyword == "name" {
                    if qualified_name.is_some() {
                        return Err(syn::Error::new(
                            keyword.span(),
                            "duplicate interface qualified name",
                        ));
                    }
                    input.parse::<Token![=]>()?;
                    qualified_name = Some(input.parse()?);
                } else if keyword == "nc_com" {
                    if runtime_crate.is_some() {
                        return Err(syn::Error::new(keyword.span(), "duplicate nc_com = ..."));
                    }
                    input.parse::<Token![=]>()?;
                    runtime_crate = Some(input.parse()?);
                } else {
                    return Err(syn::Error::new(
                        keyword.span(),
                        "expected UUID string, name = \"stable.name:v1\", auto_guid = \"stable.name:v1\", or nc_com = path",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            guid: guid.ok_or_else(|| {
                syn::Error::new(
                    input.span(),
                    "expected UUID string or auto_guid = \"stable.name:v1\"",
                )
            })?,
            qualified_name: qualified_name.ok_or_else(|| {
                syn::Error::new(
                    input.span(),
                    "explicit interface UUIDs require name = \"stable.name:v1\"",
                )
            })?,
            runtime_crate,
        })
    }
}

impl Parse for ClassContractArgs {
    /// Parses the stable identity of a no-argument callable class contract.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut class_id = None;
        let mut name = None;
        let mut version = None;
        let mut runtime_crate = None;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "class_id" {
                if class_id.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate class_id(...)"));
                }
                let content;
                parenthesized!(content in input);
                class_id = Some(parse_guid_arg(&content, "class_id")?);
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
            } else if keyword == "nc_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate nc_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected class_id(...), name = ..., version = ..., or nc_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            class_id: class_id
                .ok_or_else(|| syn::Error::new(input.span(), "expected class_id(...)"))?,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "expected name = ..."))?,
            version: version
                .ok_or_else(|| syn::Error::new(input.span(), "expected version = ..."))?,
            runtime_crate,
        })
    }
}

impl Parse for ClassArgs {
    /// Parses `interfaces(IFoo, module::IBar)` with optional `thread_safe`.
    ///
    /// Interface paths are intentionally kept to Rust module-style paths for
    /// now. That covers `interfaces(dom::INode)` without accepting generic or
    /// associated-type paths that the generated names cannot represent.
    ///
    /// Descriptor metadata is optional and all-or-nothing. Existing classes can
    /// keep using `#[com_class(interfaces(...))]`; classes that opt into
    /// `class_id(...)`, `name`, `version`, and `constructor(...)` get generated
    /// class metadata for `plugin!` to assemble.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut interfaces = None;
        let mut thread_safe = false;
        let mut runtime_crate = None;
        let mut class_id = None;
        let mut name = None;
        let mut version = None;
        let mut constructor = None;
        let mut contract = None;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "interfaces" {
                if interfaces.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate interfaces(...)"));
                }

                let content;
                parenthesized!(content in input);
                let paths = content.parse_terminated(Path::parse_mod_style, Token![,])?;
                validate_class_interface_paths(paths.iter())?;
                interfaces = Some(
                    paths
                        .into_iter()
                        .map(ClassInterface::from_path)
                        .collect::<Result<Vec<_>>>()?,
                );
            } else if keyword == "thread_safe" {
                if thread_safe {
                    return Err(syn::Error::new(keyword.span(), "duplicate thread_safe"));
                }
                thread_safe = true;
            } else if keyword == "nc_com" {
                if runtime_crate.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate nc_com = ..."));
                }
                input.parse::<Token![=]>()?;
                runtime_crate = Some(input.parse()?);
            } else if keyword == "class_id" {
                if class_id.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate class_id(...)"));
                }
                let content;
                parenthesized!(content in input);
                class_id = Some(parse_guid_arg(&content, "class_id")?);
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
            } else if keyword == "constructor" {
                if constructor.is_some() {
                    return Err(syn::Error::new(
                        keyword.span(),
                        "duplicate constructor(...)",
                    ));
                }
                let content;
                parenthesized!(content in input);
                constructor = Some(content.parse()?);
            } else if keyword == "contract" {
                if contract.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate contract(...)"));
                }
                let content;
                parenthesized!(content in input);
                let path = Path::parse_mod_style(&content)?;
                if !content.is_empty() {
                    return Err(syn::Error::new(
                        content.span(),
                        "expected one class contract path",
                    ));
                }
                contract = Some(path);
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected contract(...), interfaces(...), thread_safe, class_id(...), name = ..., version = ..., constructor(...), or nc_com = path",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let mut interfaces = interfaces.unwrap_or_default();
        let descriptor = match (contract, class_id, name, version, constructor) {
            (Some(contract), None, None, None, Some(constructor)) => {
                let primary = ClassInterface::from_path(contract)?;
                if interfaces
                    .iter()
                    .any(|interface| interface.field_ident == primary.field_ident)
                {
                    return Err(syn::Error::new(
                        primary.field_ident.span(),
                        "the primary class contract is implicit; remove it from interfaces(...)"
                    ));
                }
                let abi_path = primary.abi_path.clone();
                interfaces.insert(0, primary);
                Some(ClassDescriptorArgs::Contract {
                    abi_path,
                    constructor,
                })
            }
            (None, None, None, None, None) => None,
            (None, Some(class_id), Some(name), Some(version), Some(constructor)) => {
                Some(ClassDescriptorArgs::Explicit {
                    class_id,
                    name,
                    version,
                    constructor,
                })
            }
            (Some(_), _, _, _, _) => {
                return Err(syn::Error::new(
                    input.span(),
                    "contract(...) supplies class identity and requires constructor(...); remove class_id, name, and version",
                ));
            }
            _ => {
                return Err(syn::Error::new(
                    input.span(),
                    "class descriptors require class_id(...), name = ..., version = ..., and constructor(...)",
                ));
            }
        };

        Ok(Self {
            interfaces,
            thread_safe,
            runtime_crate,
            descriptor,
        })
    }
}

impl Parse for ClassConstructorArgs {
    /// Parses and cross-validates constructor argument and factory options.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut arguments = None;
        let mut function = None;
        let mut use_default = false;

        while !input.is_empty() {
            let keyword: Ident = input.parse()?;
            if keyword == "args" {
                if arguments.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate args = ..."));
                }
                input.parse::<Token![=]>()?;
                arguments = Some(ConstructorArguments::parse_value(input)?);
            } else if keyword == "function" {
                if function.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate function = ..."));
                }
                input.parse::<Token![=]>()?;
                function = Some(input.parse()?);
            } else if keyword == "default" {
                if use_default {
                    return Err(syn::Error::new(keyword.span(), "duplicate default"));
                }
                use_default = true;
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected args = none, args = IArgs, function = path, or default",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let arguments =
            arguments.ok_or_else(|| syn::Error::new(input.span(), "expected args = ..."))?;
        if function.is_some() && use_default {
            return Err(syn::Error::new(
                input.span(),
                "constructor default cannot be combined with function = path",
            ));
        }
        if use_default && !matches!(arguments, ConstructorArguments::None) {
            return Err(syn::Error::new(
                input.span(),
                "constructor default is only valid with args = none",
            ));
        }
        if function.is_none() && !matches!(arguments, ConstructorArguments::None) {
            return Err(syn::Error::new(
                input.span(),
                "constructor(args = Interface) also needs function = path",
            ));
        }

        Ok(Self {
            arguments,
            function,
            use_default,
        })
    }
}

impl ConstructorArguments {
    /// Parses either the `none` sentinel or an interface path.
    fn parse_value(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Ident) {
            let fork = input.fork();
            let ident: Ident = fork.parse()?;
            if ident == "none" {
                input.parse::<Ident>()?;
                return Ok(Self::None);
            }
        }

        let path = Path::parse_mod_style(input)?;
        Ok(Self::Interface {
            abi_path: interface_abi_path(&path)?,
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
            return Err(syn::Error::new_spanned(
                path,
                "duplicate COM interface in interfaces(...)",
            ));
        }
        if seen_names.contains(&name_key) {
            return Err(syn::Error::new_spanned(
                path,
                "COM interfaces in one class must have distinct final path names",
            ));
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
            InterfaceGuid::AutoGuid {
                value: input.parse()?,
            }
        } else {
            InterfaceGuid::Path(Path::parse_mod_style(input)?)
        }
    };

    if !input.is_empty() {
        input.parse::<Token![,]>()?;
        if !input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                format!("unexpected extra tokens in {name}(...)"),
            ));
        }
    }

    Ok(guid)
}
