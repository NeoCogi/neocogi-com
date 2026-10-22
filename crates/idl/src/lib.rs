//! NeoCogi IDL parsing, canonical resolution, Rust generation, and Cargo integration.
//!
//! A module name comes from its normalized source file or directory path below
//! an explicitly selected IDL root. The parser accepts the deliberately small
//! Rust-like language documented in `docs/contracts/IDL_MODULES.md`; it does not accept
//! arbitrary Rust syntax and never treats Rust's native representation as an
//! ABI.

#![deny(missing_docs)]

mod binding;
mod diagnostic;
mod metadata_names;
mod model;
mod parser;
mod path;
mod resolve;
mod sha256;
mod specialize;
mod standard;
mod syntax;
pub use standard::is_bundled_module;

pub use model::{
    PersistencePolicy, AbiType, BitEnum, BitFlag, Declaration, DeclarationId, DeclarationKind, Enum, EnumNames, EnumRepr, EnumVariant, ExternalModule, Field,
    Import, Interface, Method, NamedType, Parameter, Receiver, Struct, Threading, VariantFields, Specialization,
};
pub use diagnostic::{Diagnostic, DiagnosticLabel, LabelStyle, SourceFile, SourcePosition, SourceSpan};
pub use parser::{ModuleSource, ParseError, parse_module, parse_module_parts};
pub use path::{ModuleName, ModulePathError};
pub use resolve::{
    MODULE_ABI_ENCODING_DOMAIN, DeclarationIdentity, ResolutionError, ResolvedDeclaration, ResolvedDependency, ResolvedModule, ResolvedModules, resolve_modules,
};

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
    /// Generic value and interface definitions retained for canonical ABI fingerprints.
    pub templates: Vec<Declaration>,
}

impl Module {
    /// Returns one declaration by its exact source identifier.
    pub fn declaration(&self, name: &str) -> Option<&Declaration> {
        // Declaration names are unique after parser validation, so the first
        // match is necessarily the only match.
        self.declarations.iter().find(|declaration| declaration.name == name)
    }
}

/// Build-script integration and transitive contract source exchange.
pub mod cargo;
/// Pure Rust binding generation; deployment and runtime policy remain elsewhere.
pub mod generate;
pub use cargo::{BuildError, BuildOutput, ModuleSources};
pub use generate::{GenerateError, generate_rust, generate_rust_with_source_map};

/// Generated Rust provenance independent of ABI identity and runtime metadata.
pub mod source_map;
pub use source_map::{GeneratedRust, RustSourceMap, SourceMapFile, SourceMapKind, SourceMapOrigin, SourceMapPosition, SourceMapRange, SourceMapping};

mod values;
pub use values::{Persistence, ValueDefinition, ValueGraph, ValueType};

#[cfg(test)]
mod backend_tests;
