//! Parser and canonical source model for NeoCogi IDL contract modules.
//!
//! A module name comes from its normalized source path below an explicitly
//! selected IDL root. The parser accepts the deliberately small Rust-like
//! language documented in `docs/IDL_MODULES.md`; it does not accept arbitrary
//! Rust syntax and never treats Rust's native representation as an ABI.

#![deny(missing_docs)]

mod model;
mod parser;
mod path;

pub use model::{
    AbiType, Class, Declaration, DeclarationId, DeclarationKind, Enum, EnumRepr, EnumVariant, Factory, Field, Import, Interface, Method, NamedType, Parameter,
    Receiver, Struct, Threading, VariantFields,
};
pub use parser::{ParseError, parse_module};
pub use path::{ModuleName, ModulePathError};

/// One parsed and structurally validated IDL module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    /// Logical dotted module name derived from the source path.
    pub name: ModuleName,
    /// Explicit imports in source order.
    pub imports: Vec<Import>,
    /// ABI declarations in source order.
    pub declarations: Vec<Declaration>,
}

impl Module {
    /// Returns one declaration by its exact source identifier.
    pub fn declaration(&self, name: &str) -> Option<&Declaration> {
        // Declaration names are unique after parser validation, so the first
        // match is necessarily the only match.
        self.declarations.iter().find(|declaration| declaration.name == name)
    }
}
