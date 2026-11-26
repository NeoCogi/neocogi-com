//! Owned syntax model shared by generation, build integration, and tools.

use neocogi_com::NcGuid;

use crate::SourceSpan;

/// One explicit dependency on another logical IDL module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalModule {
    /// Complete `extern` declaration used for dependency diagnostics.
    pub span: SourceSpan,
    /// Canonical dotted name of the required module.
    pub module: String,
}

/// One imported declaration and its exact authored source origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Import {
    /// Imported identifier used for diagnostics and generated re-export provenance.
    pub span: SourceSpan,
    /// Dotted name of the module that owns the imported declarations.
    pub module: String,
    /// Exact declaration name imported from the module.
    pub name: String,
}

/// One source declaration together with its stable identity inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    /// Declared identifier used for duplicate and identity diagnostics.
    pub span: SourceSpan,
    /// Documentation lines without Rust comment markers.
    pub docs: Vec<String>,
    /// Identifier exactly as declared in source.
    pub name: String,
    /// Authored UUID override; absence derives identity from the qualified name.
    /// Closed specializations store their template/argument-derived UUID here.
    pub explicit_id: Option<NcGuid>,
    /// Ordered parameters of a struct, enum, or interface template; empty after specialization.
    pub parameters: Vec<String>,
    /// Canonical origin of an instantiated declaration template, when applicable.
    pub specialization: Option<Specialization>,
    /// Declaration-specific ABI contents.
    pub kind: DeclarationKind,
}

/// Supported top-level declaration categories.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclarationKind {
    /// A copied record with named fields.
    Struct(Struct),
    /// A tagged or integer enumeration.
    Enum(Enum),
    /// A transparent integer set of named bit flags.
    BitEnum(BitEnum),
    /// A reusable COM interface contract.
    Interface(Interface),
}

/// An ABI bit set whose declared constants describe every accepted bit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitEnum {
    /// Fixed unsigned integer representation selected after the declaration name.
    pub repr: EnumRepr,
    /// Named flags and aliases in normative declaration order.
    pub flags: Vec<BitFlag>,
}

/// One named constant in a bit-enum declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitFlag {
    /// Declared flag identifier used for duplicate-value diagnostics.
    pub span: SourceSpan,
    /// Documentation lines without comment markers.
    pub docs: Vec<String>,
    /// SCREAMING_SNAKE_CASE flag identifier exactly as declared.
    pub name: String,
    /// Explicit nonnegative raw bit pattern.
    pub value: u64,
}

/// Persistent record shape; ordinary values acquire both codecs automatically.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PersistencePolicy {
    /// Named fields are all required; codec availability follows the complete value graph.
    #[default]
    Automatic,
    /// Named-record decoding starts from a host-authored Default implementation.
    Default,
}
impl PersistencePolicy {
    /// Explicit trailing defaults require codecs for every reachable field.
    pub fn required(self) -> bool {
        !matches!(self, Self::Automatic)
    }
}

/// An ABI record declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Struct {
    /// Named fields in normative ABI order.
    pub fields: Vec<Field>,
    /// Symmetric persistent format policy, separate from live COM transport.
    pub persistence: PersistencePolicy,
}

/// One named field in a struct or named enum payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    /// Declared field identifier and type used for layout diagnostics.
    pub span: SourceSpan,
    /// Documentation lines without comment markers.
    pub docs: Vec<String>,
    /// Field identifier exactly as declared.
    pub name: String,
    /// Parsed ABI type expression.
    pub ty: IdlType,
}

/// Fixed integer representation selected by an enum's optional type suffix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnumRepr {
    /// Unsigned eight-bit discriminant.
    U8,
    /// Unsigned sixteen-bit discriminant.
    U16,
    /// Unsigned thirty-two-bit discriminant and default when no suffix appears.
    U32,
    /// Signed eight-bit discriminant.
    I8,
    /// Signed sixteen-bit discriminant.
    I16,
    /// Signed thirty-two-bit discriminant.
    I32,
}

/// An ABI enum declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Enum {
    /// Fixed discriminant representation selected by the suffix or default.
    pub repr: EnumRepr,
    /// Whether unit variants form the exact zero-based contiguous range.
    pub dense: bool,
    /// Optional canonical text generated for each dense unit variant.
    pub names: Option<EnumNames>,
    /// Variants in normative ABI order.
    pub variants: Vec<EnumVariant>,
}

impl Enum {
    /// Classifies authored shape before specialization or native Rust layout.
    /// Even an empty tuple/record payload is a tagged-sum case, never an integer constant.
    pub fn has_payload(&self) -> bool {
        self.variants.iter().any(|variant| !matches!(variant.fields, VariantFields::Unit))
    }
}

/// Canonical text policy requested for a dense unit enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnumNames {
    /// Converts declared variant identifiers to lowercase snake case.
    SnakeCase,
}

/// One enum variant and its optional payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariant {
    /// Declared variant identifier used for discriminant diagnostics.
    pub span: SourceSpan,
    /// Documentation lines without comment markers.
    pub docs: Vec<String>,
    /// Variant identifier exactly as declared.
    pub name: String,
    /// Optional explicit integer discriminant.
    pub discriminant: Option<i64>,
    /// Unit, tuple, or named variant payload.
    pub fields: VariantFields,
}

/// Payload shape for one enum variant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VariantFields {
    /// A variant with no payload.
    Unit,
    /// A positional payload in normative element order.
    Tuple(
        /// ABI types stored in the tuple payload.
        Vec<IdlType>,
    ),
    /// A payload with named fields.
    Named(
        /// Named ABI fields in normative layout order.
        Vec<Field>,
    ),
}

/// One callable contract, including factory roots.
/// The first required interface is the physical vtable base; later entries
/// are sibling projections guaranteed by QueryInterface on the same object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interface {
    /// Base followed by required sibling interfaces, in declared query order.
    pub bases: Vec<NamedType>,
    /// Ordinary methods in normative vtable order.
    pub methods: Vec<Method>,
    /// Minimum threading capability promised by every implementation.
    pub threading: Threading,
}

impl Interface {
    /// Reports whether a method declared here needs a whole-value writer.
    pub(crate) fn declares_mutable_receiver(&self) -> bool {
        self.methods.iter().any(|method| method.receiver == Receiver::Mutable)
    }
}

/// Threading capability promised by an interface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Threading {
    /// The interface grants neither cross-thread transfer nor shared access.
    Local,
    /// The interface grants both cross-thread transfer and shared access.
    Shared,
    /// The interface grants cross-thread transfer of exclusive access.
    Send,
    /// The interface grants cross-thread use of shared access.
    Sync,
}

impl Threading {
    /// An interface must preserve each independent guarantee of a base.
    pub(crate) const fn supports(self, required: Self) -> bool {
        match required {
            Self::Local => true,
            Self::Shared => matches!(self, Self::Shared),
            Self::Send => matches!(self, Self::Send | Self::Shared),
            Self::Sync => matches!(self, Self::Sync | Self::Shared),
        }
    }
}

/// One COM method declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Method {
    /// Declared method signature used for callable diagnostics.
    pub span: SourceSpan,
    /// Documentation lines without comment markers.
    pub docs: Vec<String>,
    /// Method identifier exactly as declared.
    pub name: String,
    /// Shared or exclusive receiver contract.
    pub receiver: Receiver,
    /// Explicit parameters after the receiver.
    pub parameters: Vec<Parameter>,
    /// Successful value carried by the method's return.
    pub output: IdlType,
    /// How the generated Rust method exposes the status.
    pub return_projection: ReturnProjection,
    /// An explicitly scoped visitor, as opposed to a receiver-bound result.
    pub(crate) scoped: bool,
    /// Native passing selected once for an admitted closed method result.
    pub(crate) return_passing: Option<ReturnPassing>,
}

/// Rust source projection of the native status and successful payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnProjection {
    /// Both provider and caller express failure through `Result`.
    Fallible,
    /// The authored Rust type projects successful completion directly.
    Direct,
}

/// Receiver capability for one COM method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Receiver {
    /// A non-exclusive `&self` call.
    Shared,
    /// An exclusive `&mut self` call.
    Mutable,
}

/// One named method parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    /// Declared parameter and type used for ownership diagnostics.
    pub span: SourceSpan,
    /// Parameter identifier exactly as declared.
    pub name: String,
    /// Parsed ABI type expression.
    pub ty: IdlType,
    /// Native passing selected once for this admitted closed input position.
    pub(crate) passing: Option<ParameterPassing>,
}

impl Parameter {
    /// Returns the resolved passing rule for a closed input.
    ///
    /// Parsed declarations and retained open templates have no passing rule.
    pub const fn passing(&self) -> Option<ParameterPassing> {
        self.passing
    }

    /// Provides the type's runtime operand without interpreting its passing.
    pub(crate) fn payload(&self) -> &IdlType {
        self.ty.call_payload()
    }
}

impl Method {
    /// Reports whether the source declares visitor delivery with `#[scoped]`.
    pub const fn is_scoped(&self) -> bool {
        self.scoped
    }

    /// Returns the resolved success-passing rule independently of status projection.
    ///
    /// Parsed declarations and retained open templates have no passing rule.
    pub const fn return_passing(&self) -> Option<ReturnPassing> {
        self.return_passing
    }

    /// Provides the type's runtime operand without interpreting its passing.
    pub(crate) fn return_payload(&self) -> &IdlType {
        self.output.call_payload()
    }
}

/// Bound direction and access of a single method input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterPassing {
    /// A self-contained value supplied by copy.
    Copied,
    /// A checked, borrowed aggregate wire.
    BorrowedValue,
    /// A borrowed UTF-8 view with readable byte storage.
    BorrowedText,
    /// A readable element buffer.
    InputBuffer,
    /// A fresh writable element buffer.
    OutputBuffer,
    /// A read-only interface loan. Only optional inputs accept both-null.
    SharedLoan {
        /// Both-null denotes absence.
        optional: bool,
    },
    /// A whole-value exclusive loan for the call.
    MutableLoan {
        /// Both-null denotes absence.
        optional: bool,
    },
    /// An exclusive owner moved from a writable caller slot.
    ConsumingBox,
}

/// Successful native method completion, independent of Rust status projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnPassing {
    /// Native completion is status-only.
    Unit,
    /// A self-contained copied value out slot.
    Copied,
    /// One shared owner transferred in an ordinary out-pair.
    SharedOwner {
        /// Both-null denotes a successful absence.
        optional: bool,
    },
    /// One exclusive owner transferred in an ordinary out-pair.
    BoxOwner,
    /// One retained shared weak control token.
    SharedObserver,
    /// A borrowed out-pair lasting for the source receiver's borrow.
    BorrowedLoan {
        /// The result grants whole-value exclusive access.
        mutable: bool,
        /// Both-null denotes absence.
        optional: bool,
    },
    /// An optional loan delivered only within a synchronous continuation.
    ScopedLoan {
        /// The visitor receives whole-value exclusive access.
        mutable: bool,
    },
}

/// One path naming an IDL declaration or built-in type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedType {
    /// Exact path expression used for name-resolution diagnostics.
    pub span: SourceSpan,
    /// Path segments in source order without rewriting.
    pub segments: Vec<String>,
    /// Arguments to a declaration template, resolved before ordinary ABI validation.
    pub arguments: Vec<IdlType>,
    /// Semantic owner assigned once before specialization; source paths are not
    /// reinterpreted by hashing, ABI validation, metadata or Rust generation.
    pub(crate) binding: crate::binding::Binding,
}

impl NamedType {
    /// Moves a type without cloning its argument tree. The vacancy cannot escape
    /// a successful visitor or specialization operation.
    pub(crate) fn take(&mut self) -> Self {
        std::mem::replace(
            self,
            Self {
                span: SourceSpan::placeholder(),
                segments: Vec::new(),
                arguments: Vec::new(),
                binding: crate::binding::Binding::Unbound,
            },
        )
    }

    /// Whether binding selected the language's borrowed UTF-8 view intrinsic.
    pub(crate) fn is_str(&self) -> bool {
        matches!(self.binding, crate::binding::Binding::Intrinsic(crate::binding::Intrinsic::Str))
    }
    /// Returns the source-like `::`-separated spelling of this path.
    pub fn rust_path(&self) -> String {
        // Rendering is diagnostic only; semantic meaning is owned by binding.
        let path = self.segments.join("::");
        match self.arguments.is_empty() {
            true => path,
            false => format!("{}<{}>", path, self.arguments.iter().map(IdlType::source_name).collect::<Vec<_>>().join(", ")),
        }
    }
}

/// Source-level IDL type before physical C ABI lowering.
///
/// Named paths, owned containers and borrowed buffer forms remain logical source
/// constructs here. The core `AbiType` trait describes a separate emitted value
/// ABI contract and retains its name. Renaming this enum changes no IDL spelling,
/// canonical encoding, interface identity or generated C representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdlType {
    /// The zero-sized successful result written as `()`.
    Unit,
    /// A built-in scalar such as `u32`, `i64`, `f32`, or `bool`.
    Scalar(
        /// Exact built-in scalar spelling.
        String,
    ),
    /// Owned UTF-8 text data; an owned COM string is explicitly `ComPtr<INcStringAbi>`.
    String,
    /// A declaration referenced by a local, imported, or qualified path.
    Named(NamedType),
    /// An owned COM reference to a closed interface contract.
    /// Ownership is explicit; the referenced implementation is never stored inline.
    ComPtr(Box<IdlType>),
    /// A shared weak reference, stored in a value or transferred as a method result.
    /// It retains control metadata only and must be promoted before interface calls.
    WeakComPtr(Box<IdlType>),
    /// One exclusive owner transferred by a factory or into a container.
    /// It cannot be copied into an ordinary struct or enum field.
    ComBox(Box<IdlType>),
    /// A read loan resolved according to its callable position.
    Borrowed(
        /// Borrowed referent type.
        Box<IdlType>,
    ),
    /// An authored mutable borrow that must close over an interface or output slice.
    BorrowedMut(
        /// Borrowed referent type.
        Box<IdlType>,
    ),
    /// An explicitly owned variable-length sequence; generated Rust stores a Vec.
    Vector(
        /// Recursively owned element type, never an implicit borrowed field.
        Box<IdlType>,
    ),
    /// An unsized sequence referent, valid through a call-scoped borrow only.
    UnsizedSlice(
        /// Element type before a borrow selects readable or writable passing.
        Box<IdlType>,
    ),
    /// A borrowed contiguous sequence.
    Slice(
        /// ABI-safe sequence element type.
        Box<IdlType>,
    ),
    /// A borrowed writable sequence filled synchronously by the callee.
    OutputSlice(
        /// ABI-safe sequence element type.
        Box<IdlType>,
    ),
    /// A fixed-size inline array.
    Array {
        /// ABI-safe array element type.
        element: Box<IdlType>,
        /// Compile-time array element count.
        len: u32,
    },
}

/// Origin of a closed declaration; generated Rust names never determine its COM identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Specialization {
    /// Bound source template declaration with no arguments; spelling is diagnostic.
    pub template: NamedType,
    /// Bound, substituted argument types in template parameter order.
    pub arguments: Vec<IdlType>,
}

impl IdlType {
    /// Peels the unary constructors that carry an operand in callable positions.
    fn call_payload(&self) -> &Self {
        match self {
            Self::ComPtr(child)
            | Self::WeakComPtr(child)
            | Self::ComBox(child)
            | Self::Borrowed(child)
            | Self::BorrowedMut(child)
            | Self::Slice(child)
            | Self::OutputSlice(child) => child,
            _ => self,
        }
    }

    /// Renders authored structural spelling for diagnostics, never for fingerprinting.
    pub fn source_name(&self) -> String {
        match self {
            Self::Unit => "()".into(),
            Self::Scalar(name) => name.clone(),
            Self::String => "String".into(),
            Self::Named(name) => name.rust_path(),
            Self::ComPtr(value) => format!("ComPtr<{}>", value.source_name()),
            Self::WeakComPtr(value) => format!("WeakComPtr<{}>", value.source_name()),
            Self::ComBox(value) => format!("ComBox<{}>", value.source_name()),
            Self::Borrowed(value) => format!("&{}", value.source_name()),
            Self::BorrowedMut(value) => format!("&mut {}", value.source_name()),
            Self::Vector(value) => format!("Vec<{}>", value.source_name()),
            Self::UnsizedSlice(value) => format!("[{}]", value.source_name()),
            Self::Slice(value) => format!("&[{}]", value.source_name()),
            Self::OutputSlice(value) => format!("&mut [{}]", value.source_name()),
            Self::Array { element, len } => format!("[{}; {}]", element.source_name(), len),
        }
    }
}
