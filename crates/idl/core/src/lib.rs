//! Parser and canonical source model for NeoCogi IDL contract modules.
//!
//! A module name comes from its normalized source file or directory path below
//! an explicitly selected IDL root. The parser accepts the deliberately small
//! Rust-like language documented in `docs/IDL_MODULES.md`; it does not accept
//! arbitrary Rust syntax and never treats Rust's native representation as an
//! ABI.

#![deny(missing_docs)]

mod diagnostic;
mod model;
mod parser;
mod path;
mod resolve;
mod sha256;

pub use model::{
    AbiType, BitEnum, BitFlag, Callable, Declaration, DeclarationId, DeclarationKind, Enum, EnumNames, EnumRepr, EnumVariant, PortBinding, ExternalModule,
    Field, Import, Interface, InterfaceRole, Method, NamedType, Parameter, Receiver, Struct, Threading, VariantFields,
};
pub use diagnostic::{Diagnostic, DiagnosticLabel, LabelStyle, SourceFile, SourcePosition, SourceSpan};
pub use parser::{ModuleSource, ParseError, parse_module, parse_module_parts};
pub use path::{ModuleName, ModulePathError};
pub use resolve::{DeclarationIdentity, ResolutionError, ResolvedDeclaration, ResolvedDependency, ResolvedModule, ResolvedModules, resolve_modules};

/// One parsed and structurally validated IDL module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    /// Logical dotted module name derived from the source path.
    pub name: ModuleName,
    /// Every source part in normalized path order.
    pub sources: Vec<SourceFile>,
    /// Direct external dependencies in canonical module-name order.
    pub externals: Vec<ExternalModule>,
    /// Explicit imports in canonical module and declaration-name order.
    pub imports: Vec<Import>,
    /// ABI declarations in canonical top-level name order.
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
