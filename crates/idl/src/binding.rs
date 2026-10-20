//! Semantic references established once before substitution or ABI analysis.
//! Source spelling remains diagnostic data; only these keys select declarations.

/// Exact declaration or materialized Rust projection in the canonical module graph.
/// Module order is fixed before binding; declaration names survive specialization's
/// sorting. Neither compiler-local indexes nor Rust projection paths enter ABI bytes.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct DeclarationKey {
    /// Index in the graph's immutable canonical module order.
    pub(crate) module: usize,
    /// Exact source or generated declaration identifier within that module.
    pub(crate) name: String,
}

/// Intrinsic capabilities have no synthetic declaration or external dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Intrinsic {
    /// Borrowed UTF-8 view used only in supported method positions.
    Str,
    /// Root COM object capability.
    Unknown,
    /// Immutable byte capability, distinct from owned Vec data.
    Bytes,
    /// Immutable UTF-8 COM capability, distinct from owned String data.
    String,
}

/// Meaning of one authored named reference throughout the compiler pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Binding {
    /// Parser output only; successful binding removes this state before expansion.
    Unbound,
    /// Lexically bound parameter; its spelling never determines identity afterward.
    Parameter {
        /// Zero-based position in the enclosing template's parameter list.
        position: u32,
    },
    /// A language-owned capability or borrowed text intrinsic.
    Intrinsic(Intrinsic),
    /// Exact ordinary declaration, including an already materialized closed type.
    Declaration(DeclarationKey),
    /// Template application whose canonical origin and consumer projection differ.
    Application {
        /// Lexically bound template; arguments retain their own bound meanings.
        template: DeclarationKey,
        /// Filled by specialization even for an unused nested type argument.
        /// Open templates have no concrete projection and are never emitted.
        projection: Option<DeclarationKey>,
    },
}

impl Binding {
    /// Selects the original declaration for identity and template substitution.
    pub(crate) fn declaration(&self) -> Option<&DeclarationKey> {
        match self {
            Self::Declaration(key) | Self::Application { template: key, .. } => Some(key),
            Self::Unbound | Self::Parameter { .. } | Self::Intrinsic(_) => None,
        }
    }

    /// Selects the actual supplying Rust declaration without a global-name search.
    pub(crate) fn projection(&self) -> Option<&DeclarationKey> {
        match self {
            Self::Declaration(key) => Some(key),
            Self::Application { projection, .. } => projection.as_ref(),
            Self::Unbound | Self::Parameter { .. } | Self::Intrinsic(_) => None,
        }
    }
}

/// Recognizes intrinsic syntax at the binding boundary, after parameter shadowing.
pub(crate) fn intrinsic(segments: &[String]) -> Option<Intrinsic> {
    match segments {
        [name] => match name.as_str() {
            "str" => Some(Intrinsic::Str),
            "IUnknown" => Some(Intrinsic::Unknown),
            "INcBytes" => Some(Intrinsic::Bytes),
            "INcString" => Some(Intrinsic::String),
            _ => None,
        },
        _ => None,
    }
}
