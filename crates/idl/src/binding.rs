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

/// Finite semantic meaning of a closed argument, independent of its UUID digest.
/// Nominal leaves stop at their bound declaration, so recursive contracts do not
/// unfold here. Applications retain the original template, never a consumer's
/// generated projection. One bound template also fixes its complete contract
/// (kind, version, fields/cases, persistence and method policy).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SemanticOrigin {
    /// The intrinsic checked unit value.
    Unit,
    /// An intrinsic scalar; its spelling is the language's canonical scalar name.
    Scalar(String),
    /// Owned UTF-8 data, distinct from the string COM capability.
    Text,
    /// A bound built-in capability or borrowed text view.
    Intrinsic(Intrinsic),
    /// An authored declaration, without unfolding its fields.
    Nominal(DeclarationKey),
    /// A template and its ordered finite argument meanings.
    Application(DeclarationKey, Vec<SemanticOrigin>),
    /// A language constructor, optional array extent, and its child meaning.
    Constructed(Constructor, Option<u32>, Box<SemanticOrigin>),
}

/// Distinguishes constructors even when their legacy UUIDs coincide with a
/// nominal explicit ID. These tags are internal proofs, not new ABI bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Constructor {
    /// Strong shared COM owner.
    ComPtr,
    /// Shared weak observer.
    WeakComPtr,
    /// Exclusive COM owner.
    ComBox,
    /// Exclusive weak observer.
    WeakComBox,
    /// Synchronous borrow.
    Borrowed,
    /// Owned sequence.
    Vector,
    /// Unsized sequence referent.
    UnsizedSlice,
    /// Readable buffer.
    Slice,
    /// Write-only buffer.
    OutputSlice,
    /// Inline fixed-size array.
    Array,
}

/// Reads already-bound meaning without another lexical lookup or UUID reduction.
pub(crate) fn semantic_origin(ty: &crate::AbiType) -> Result<SemanticOrigin, crate::ResolutionError> {
    use crate::AbiType as T;
    use SemanticOrigin as O;
    let constructor = |kind, len, child| Ok(O::Constructed(kind, len, Box::new(semantic_origin(child)?)));
    match ty {
        T::Unit => Ok(O::Unit),
        T::Scalar(name) => Ok(O::Scalar(name.clone())),
        T::String => Ok(O::Text),
        T::Named(name) => match &name.binding {
            Binding::Intrinsic(value) => Ok(O::Intrinsic(*value)),
            Binding::Declaration(key) if name.arguments.is_empty() => Ok(O::Nominal(key.clone())),
            Binding::Application { template, .. } => Ok(O::Application(
                template.clone(),
                name.arguments.iter().map(semantic_origin).collect::<Result<_, _>>()?,
            )),
            _ => Err(crate::ResolutionError {
                diagnostics: vec![
                    crate::Diagnostic::new("EIDL060", "closed semantic origin requires a bound type").primary(&name.span, "unbound or open argument"),
                ],
            }),
        },
        T::ComPtr(child) => constructor(Constructor::ComPtr, None, child),
        T::WeakComPtr(child) => constructor(Constructor::WeakComPtr, None, child),
        T::ComBox(child) => constructor(Constructor::ComBox, None, child),
        T::WeakComBox(child) => constructor(Constructor::WeakComBox, None, child),
        T::Borrowed(child) => constructor(Constructor::Borrowed, None, child),
        T::Vector(child) => constructor(Constructor::Vector, None, child),
        T::UnsizedSlice(child) => constructor(Constructor::UnsizedSlice, None, child),
        T::Slice(child) => constructor(Constructor::Slice, None, child),
        T::OutputSlice(child) => constructor(Constructor::OutputSlice, None, child),
        T::Array { element, len } => constructor(Constructor::Array, Some(*len), element),
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
