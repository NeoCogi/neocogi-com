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

/// One callable contract, including factory roots and event sinks.
/// The first required interface is the physical vtable base; later entries
/// are sibling projections guaranteed by QueryInterface on the same object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interface {
    /// Base followed by required sibling interfaces, in declared query order.
    pub bases: Vec<NamedType>,
    /// Ordinary methods and event subscription ports in normative vtable order.
    pub callables: Vec<Callable>,
    /// Construction or delivery policy layered over the common interface ABI.
    pub role: InterfaceRole,
    /// Minimum threading capability promised by every implementation.
    pub threading: Threading,
}

/// Semantic restrictions and generated conveniences for one interface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterfaceRole {
    /// A callable projection acquired from another object or factory.
    Ordinary,
    /// A construction/provider interface eligible for plugin bootstrap.
    Factory,
    /// An immutable delivery interface with generated owned event snapshots.
    Event,
}

impl Interface {
    /// Visits declared methods without synthesizing subscription-port calls.
    pub fn methods(&self) -> impl Iterator<Item = &Method> {
        self.callables.iter().filter_map(|callable| match callable {
            Callable::Method(method) => Some(method),
            Callable::EventPort(_) => None,
        })
    }
}

/// Threading capability promised by an interface.
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

/// One ordered callable declaration on a source interface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Callable {
    /// An ordinary author-declared COM method.
    Method(Method),
    /// A first-class event subscription port with generated sink and lease types.
    EventPort(EventPort),
}

impl Callable {
    /// Returns the ABI method name used for duplicate and vtable diagnostics.
    pub fn name(&self) -> &str {
        match self {
            Self::Method(method) => &method.name,
            Self::EventPort(port) => &port.name,
        }
    }

    /// Returns the original declaration span for diagnostics.
    pub fn span(&self) -> &SourceSpan {
        match self {
            Self::Method(method) => &method.span,
            Self::EventPort(port) => &port.span,
        }
    }
}

/// Canonical COM module that defines the generated event lease ABI.
/// Canonical NCIDL module whose event transport ABI is intrinsic to COM.
pub const COM_EVENTS_MODULE_NAME: &str = "neocogi.com.events";

/// Canonical lease declaration used by every lowered event port.
pub(crate) const COM_EVENT_LEASE_NAME: &str = "IEventSubscription";

/// One subscribable event port whose sink is a first-class NCIDL event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventPort {
    /// Port method identifier and source of subscription diagnostics.
    pub span: SourceSpan,
    /// Documentation lines for the generated source method.
    pub docs: Vec<String>,
    /// Exact method name, commonly `subscribe` or `subscribe_changed`.
    pub name: String,
    /// Shared or exclusive source receiver promised to consumers.
    pub receiver: Receiver,
    /// Concrete event sink declaration; arbitrary traits are rejected.
    pub sink: NamedType,
}

impl EventPort {
    /// Expands the declarative port into its exact generated COM signature.
    /// The shared lease must be imported from the canonical COM events module;
    /// resolution rejects an unbound lease or a same-named non-COM declaration.
    pub fn signature(&self) -> Method {
        Method {
            span: self.span.clone(),
            docs: self.docs.clone(),
            name: self.name.clone(),
            receiver: self.receiver,
            parameters: vec![Parameter {
                span: self.span.clone(),
                name: "sink".to_owned(),
                ty: AbiType::Borrowed(Box::new(AbiType::Named(self.sink.clone()))),
            }],
            output: AbiType::Named(NamedType {
                span: self.span.clone(),
                segments: vec![COM_EVENT_LEASE_NAME.to_owned()],
            }),
        }
    }
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
