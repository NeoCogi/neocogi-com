//! Owned syntax model shared by generation, build integration, and tools.

use neocogi_com_core::NcGuid;

use crate::SourceSpan;

/// One explicit dependency on another logical IDL module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalModule {
    /// Complete `extern` declaration used for dependency diagnostics.
    pub span: SourceSpan,
    /// Canonical dotted name of the required module.
    pub module: String,
}

/// One explicit import from another logical IDL module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Import {
    /// Complete import statement used for conflict diagnostics.
    pub span: SourceSpan,
    /// Dotted name of the module that owns the imported declarations.
    pub module: String,
    /// Exact declaration names imported from the module.
    pub names: Vec<String>,
}

/// Version and optional explicitly assigned identity for one declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclarationId {
    /// Positive author-controlled declaration version.
    pub version: u32,
    /// Canonical UUID bytes when `#[id]` overrides derived identity.
    pub explicit: Option<NcGuid>,
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
    /// Version and optional explicit UUID parsed from attributes.
    pub id: DeclarationId,
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
    /// A public object contract created only by a factory.
    Class(Class),
    /// A typed provider and construction contract.
    Factory(Factory),
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

/// An ABI record declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Struct {
    /// Named fields in normative ABI order.
    pub fields: Vec<Field>,
    /// Whether generated bindings implement format-neutral typed decoding.
    pub deserialize: bool,
    /// Whether omitted named fields retain values from `Default::default`.
    pub deserialize_default: bool,
    /// Whether the record may also decode from an ordered sequence.
    pub deserialize_sequence: bool,
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
    pub ty: AbiType,
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
        Vec<AbiType>,
    ),
    /// A payload with named fields.
    Named(
        /// Named ABI fields in normative layout order.
        Vec<Field>,
    ),
}

/// A reusable COM interface declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interface {
    /// Base interface paths in declared query order.
    pub bases: Vec<NamedType>,
    /// Callable methods in normative vtable order.
    pub methods: Vec<Method>,
}

/// A public object contract with no runtime constructor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Class {
    /// Interfaces every implementation must expose through `QueryInterface`.
    pub required_interfaces: Vec<NamedType>,
    /// Callable methods on the class's primary projection.
    pub methods: Vec<Method>,
    /// Minimum threading capability promised to consumers.
    pub threading: Threading,
}

/// A typed construction or provider interface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Factory {
    /// Additional interfaces every factory implementation must expose.
    pub required_interfaces: Vec<NamedType>,
    /// Typed provider methods in normative vtable order.
    pub methods: Vec<Method>,
    /// Minimum threading capability promised to consumers.
    pub threading: Threading,
}

/// Threading capability attached to a class or factory contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Threading {
    /// Calls and references remain on their creating thread.
    Local,
    /// Calls and references may cross threads concurrently.
    Shared,
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
    /// Successful value carried by the required `Result<T>` return.
    pub output: AbiType,
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
    pub ty: AbiType,
}

/// One path naming an IDL declaration or built-in type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedType {
    /// Exact path expression used for name-resolution diagnostics.
    pub span: SourceSpan,
    /// Path segments in source order without rewriting.
    pub segments: Vec<String>,
}

impl NamedType {
    /// Returns the source-like `::`-separated spelling of this path.
    pub fn rust_path(&self) -> String {
        // Source rendering is diagnostic only; identity resolution uses the
        // original segment vector to avoid splitting normalized text again.
        self.segments.join("::")
    }
}

/// ABI-safe type expressions understood by the initial IDL language.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbiType {
    /// The zero-sized successful result written as `()`.
    Unit,
    /// A built-in scalar such as `u32`, `i64`, `f32`, or `bool`.
    Scalar(
        /// Exact built-in scalar spelling.
        String,
    ),
    /// An owned immutable UTF-8 COM string.
    String,
    /// A declaration referenced by a local, imported, or qualified path.
    Named(NamedType),
    /// A value borrowed only for the duration of one ABI call.
    Borrowed(
        /// Borrowed referent type.
        Box<AbiType>,
    ),
    /// A borrowed contiguous sequence.
    Slice(
        /// ABI-safe sequence element type.
        Box<AbiType>,
    ),
    /// A borrowed writable sequence filled synchronously by the callee.
    OutputSlice(
        /// ABI-safe sequence element type.
        Box<AbiType>,
    ),
    /// A fixed-size inline array.
    Array {
        /// ABI-safe array element type.
        element: Box<AbiType>,
        /// Compile-time array element count.
        len: u32,
    },
    /// An explicitly optional value with generated tag-and-payload layout.
    Option(
        /// Value present when the generated tag selects `Some`.
        Box<AbiType>,
    ),
}
