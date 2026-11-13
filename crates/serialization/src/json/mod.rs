//! COM sessions for bounded JSON syntax. Only this module knows the private
//! syntax tree; application construction stays in the caller's typed value traits.
mod grammar;
mod syntax;

use alloc::{
    borrow::ToOwned,
    collections::{BTreeMap, VecDeque},
    string::String,
    vec::Vec,
    format, vec,
};
use neocogi_com::{ComPtr, ComBox, NcStatus, NcCopyFromBufferOffset, NcOutputWriter, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
use neocogi_com::serialization::*;
use grammar::{Node, Number, SyntaxBudget};
use neocogi_com::serialization::Error as Failure;

/// A session has one root and no recovery after its first failed operation.
enum Phase {
    /// A document may still be traversed.
    Active,
    /// Preserve the first status even if a caller attempts further operations.
    Failed(NcStatus),
    /// Output is immutable or the source was completely consumed.
    Finished,
}
impl Phase {
    /// Admits only active format operations; output access is checked separately.
    fn admit(&self) -> Result<(), NcStatus> {
        match self {
            Self::Active => Ok(()),
            Self::Failed(status) => Err(*status),
            Self::Finished => Err(NC_E_INVALID_ARG),
        }
    }
}
/// Logical value accounting is independent from text syntax/framing nodes.
struct Budget {
    /// Full caller policy, including exact document bytes.
    limits: Limits,
    /// Remaining scalar/container values across the complete traversal.
    remaining: usize,
}
impl Budget {
    /// Initializes the immutable policy and cumulative construction counter.
    fn new(limits: Limits) -> Self {
        Self { limits, remaining: limits.values }
    }
    /// Opens one scalar or container before accepting its representation.
    fn enter(&mut self, depth: usize) -> Result<(), NcStatus> {
        if depth > self.limits.depth || depth > 128 || self.remaining == 0 {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.remaining -= 1;
        Ok(())
    }
    /// Bounds a container and its minimum required child count before allocation.
    fn collection(&self, count: usize, per_entry: usize) -> Result<(), NcStatus> {
        if count > self.limits.elements || count.checked_mul(per_entry).is_none_or(|n| n > self.remaining) {
            return Err(NC_E_OUT_OF_RANGE);
        }
        Ok(())
    }
}
/// Matching end operations distinguish ordered sequences from key/value maps.
#[derive(Clone, Copy, PartialEq)]
enum Ordered {
    Sequence,
    Map,
}
/// One open writer container owns only provider-local syntax children.
enum WriteFrame {
    /// Each unique field must be selected before its value is accepted.
    Record {
        expected: usize,
        fields: BTreeMap<String, Node>,
        selected: Option<String>,
    },
    /// Maps contain exactly twice their pair count in alternating key/value order.
    Ordered { kind: Ordered, expected: usize, values: Vec<Node> },
    /// Present options have one child, absent options have none.
    Option { present: bool, child: Option<Node> },
    /// Named cases have at most one payload value.
    Variant { name: String, child: Option<Node> },
}
/// Mutable text encoder state; committed bytes are inaccessible until finish.
struct Writer {
    /// Legal document transitions and retained first error.
    phase: Phase,
    /// Exact logical value and per-container limits.
    budget: Budget,
    /// Open containers, each consuming one logical depth level.
    frames: Vec<WriteFrame>,
    /// The sole complete root, before encoding at finish.
    root: Option<Node>,
    /// Immutable bytes after a successful finish.
    output: Vec<u8>,
    /// Lower bound on payload bytes, limiting syntax accumulation before finish.
    payload: usize,
    /// Allocated only on the first failed provider operation.
    failure: Option<Failure>,
}
impl Writer {
    /// Starts a fresh independent document with no shared cursor state.
    fn new(limits: Limits) -> Self {
        Self {
            phase: Phase::Active,
            budget: Budget::new(limits),
            frames: Vec::new(),
            root: None,
            output: Vec::new(),
            payload: 0,
            failure: None,
        }
    }
    /// Provider-entry failures permanently close the document; ABI rejections
    /// happen before this function and therefore cannot mutate its state.
    fn run<T>(&mut self, operation: &'static str, call: impl FnOnce(&mut Self) -> Result<T, NcStatus>) -> Result<T, NcStatus> {
        self.phase.admit()?;
        call(self).inspect_err(|&status| {
            if self.failure.is_none() {
                self.failure = Some(Failure::new(status, operation));
            }
            self.phase = Phase::Failed(status);
        })
    }
    /// Checks a payload lower bound before copying text or accumulating children.
    fn charge(&mut self, bytes: usize) -> Result<(), NcStatus> {
        if bytes > self.budget.limits.bytes.saturating_sub(self.payload) {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.payload += bytes;
        Ok(())
    }
    /// Attaches one complete value to its selected parent, or the sole root.
    fn store(&mut self, node: Node) -> Result<(), NcStatus> {
        match self.frames.last_mut() {
            Some(WriteFrame::Record { expected, fields, selected }) if fields.len() < *expected => {
                let name = selected.take().ok_or(NC_E_INVALID_ARG)?;
                if fields.insert(name, node).is_some() {
                    return Err(NC_E_INVALID_ARG);
                }
            }
            Some(WriteFrame::Ordered { expected, values, .. }) if values.len() < *expected => values.push(node),
            Some(WriteFrame::Option { present: true, child } | WriteFrame::Variant { child, .. }) if child.is_none() => *child = Some(node),
            None if self.root.is_none() => self.root = Some(node),
            _ => return Err(NC_E_INVALID_ARG),
        }
        Ok(())
    }
    /// Scalar operations consume exactly one logical value and no persistent scope.
    fn scalar(&mut self, node: Node) -> Result<(), NcStatus> {
        self.budget.enter(self.frames.len() + 1)?;
        self.charge(1)?;
        self.store(node)
    }
    /// Begins one container under the same accounting as the matching decoder.
    fn open(&mut self, frame: WriteFrame) -> Result<(), NcStatus> {
        self.budget.enter(self.frames.len() + 1)?;
        self.frames.push(frame);
        Ok(())
    }
    /// Opens an ordered collection after admitting its declared cardinality.
    fn ordered(&mut self, kind: Ordered, count: u32) -> Result<(), NcStatus> {
        self.budget.enter(self.frames.len() + 1)?;
        let per_entry = if kind == Ordered::Map { 2 } else { 1 };
        self.budget.collection(count as usize, per_entry)?;
        let expected = (count as usize).checked_mul(per_entry).ok_or(NC_E_OUT_OF_RANGE)?;
        self.frames.push(WriteFrame::Ordered { kind, expected, values: Vec::new() });
        Ok(())
    }
    /// Completes a sequence or explicit arbitrary-key map, checking its end kind.
    fn end_ordered(&mut self, expected_kind: Ordered) -> Result<(), NcStatus> {
        let Some(WriteFrame::Ordered { kind, expected, values }) = self.frames.pop() else {
            return Err(NC_E_INVALID_ARG);
        };
        if kind != expected_kind || values.len() != expected {
            return Err(NC_E_INVALID_ARG);
        }
        let node = if kind == Ordered::Sequence {
            Node::Array(values)
        } else {
            let mut values = values.into_iter();
            let mut pairs = Vec::new();
            while let Some(key) = values.next() {
                pairs.push(Node::Array(vec![key, values.next().ok_or(NC_E_INVALID_ARG)?]));
            }
            object("$map", Node::Array(pairs))
        };
        self.store(node)
    }
}
/// Constructs fixed framing without admitting an application field implicitly.
fn object(name: &str, value: Node) -> Node {
    Node::Object(BTreeMap::from([(name.to_owned(), value)]))
}
/// Validates whole UTF-8 buffer semantics before any provider copies it.
fn text(value: NcCopyFromBufferOffset<'_, u8>) -> Result<&str, NcStatus> {
    if value.offset() != 0 {
        return Err(NC_E_INVALID_ARG);
    }
    core::str::from_utf8(value.as_slice()).map_err(|_| NC_E_INVALID_ARG)
}

/// One source value owns its current representation and any unconsumed children.
enum ReadValue {
    /// Typed operations have not yet selected this syntax value.
    Unread(Node),
    /// A scalar or explicitly closed container has been consumed.
    Done,
    /// A validated string waits for one fresh-output copy.
    String(String),
    /// Fields may be selected in any schema order; leftovers remain detectable.
    Record {
        fields: BTreeMap<String, Node>,
        selected: Option<(String, Node)>,
        missing: Option<String>,
    },
    /// Sequence entries or flattened map keys/values remain in order.
    Ordered { kind: Ordered, values: VecDeque<Node>, index: usize },
    /// Presence has been selected; a present child must still be acquired.
    Option(Option<Node>),
    /// A case is probed by name before its optional payload is acquired.
    Variant { name: String, child: Option<Node>, matched: bool },
}
/// Failure context distinguishes an existing value from a requested next child.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PathSite {
    /// Provider framing/conversion failed on the currently entered value.
    Current,
    /// Schema failure may name a selected/missing field, never an unentered index.
    SelectedField,
    /// Entering the next child failed before its path was pushed.
    NextChild,
}
/// Provider-owned input syntax and exclusive typed-construction cursor.
struct Reader {
    /// Legal transitions, including the first failed status.
    phase: Phase,
    /// Exact logical construction limits.
    budget: Budget,
    /// Root syntax is acquired exactly once.
    root: Option<Node>,
    /// Each entered typed value has one owned representation state.
    values: Vec<ReadValue>,
    /// Distinguishes an unstarted document from a completed empty stack.
    opened: bool,
    /// Logical components retained without formatting on successful traversal.
    paths: Vec<Part>,
    /// Immutable parser or typed-construction diagnostic, created only on failure.
    failure: Option<Failure>,
}
impl Reader {
    /// Takes ownership of a completely parsed document; no caller borrow survives.
    fn new(node: Result<Node, Failure>, limits: Limits) -> Self {
        let (phase, root, failure) = match node {
            Ok(node) => (Phase::Active, Some(node), None),
            Err(error) => (Phase::Failed(error.status), None, Some(error)),
        };
        Self {
            phase,
            budget: Budget::new(limits),
            root,
            values: Vec::new(),
            opened: false,
            paths: Vec::new(),
            failure,
        }
    }

    /// Makes every provider or schema error terminal while retaining its status.
    fn run<T>(&mut self, operation: &'static str, call: impl FnOnce(&mut Self) -> Result<T, NcStatus>) -> Result<T, NcStatus> {
        self.phase.admit()?;
        let actual = self.actual();
        call(self).inspect_err(|&status| {
            if self.failure.is_none() {
                let mut error = Failure::new(status, format!("{}; found {actual}", expectation(operation)));
                let site = match operation {
                    "enter_value" => PathSite::NextChild,
                    "fail" => PathSite::SelectedField,
                    _ => PathSite::Current,
                };
                error.path = self.path(site);
                self.failure = Some(error);
            }
            self.phase = Phase::Failed(status);
        })
    }
    /// Replaces the current state only inside a gate-admitted mutable call.
    fn take(&mut self) -> Result<ReadValue, NcStatus> {
        Ok(core::mem::replace(self.values.last_mut().ok_or(NC_E_INVALID_ARG)?, ReadValue::Done))
    }
    /// Selects one syntax node exactly once for its requested representation.
    fn unread(&mut self) -> Result<Node, NcStatus> {
        match self.take()? {
            ReadValue::Unread(node) => Ok(node),
            _ => Err(NC_E_INVALID_ARG),
        }
    }
    /// Acquires a child only from its selected parent or the unconsumed root.
    fn enter(&mut self) -> Result<(), NcStatus> {
        self.budget.enter(self.values.len() + 1)?;
        let (node, part) = match self.values.last_mut() {
            None if !self.opened => {
                self.opened = true;
                (self.root.take(), Part::Root)
            }
            Some(ReadValue::Record { selected, .. }) => {
                let (name, node) = selected.take().ok_or(NC_E_INVALID_ARG)?;
                (Some(node), Part::Field(name))
            }
            Some(ReadValue::Option(selected)) => (selected.take(), Part::Some),
            Some(ReadValue::Ordered { kind, values, index }) => {
                let part = if *kind == Ordered::Map { Part::Map(*index) } else { Part::Index(*index) };
                // A failed entry refers to the attempted child, not its successor.
                // Advance only after acquiring an actual node.
                let node = values.pop_front().ok_or(NC_E_INVALID_ARG)?;
                *index += 1;
                (Some(node), part)
            }
            Some(ReadValue::Variant { child, matched: true, .. }) => (child.take(), Part::Payload),
            _ => (None, Part::Root),
        };
        let node = node.ok_or(NC_E_INVALID_ARG)?;
        self.paths.push(part);
        self.values.push(ReadValue::Unread(node));
        Ok(())
    }
    /// Containers with explicit end methods must reach Done before scope exit.
    fn leave(&mut self) -> Result<(), NcStatus> {
        match self.values.last() {
            Some(ReadValue::Done | ReadValue::Option(None) | ReadValue::Variant { child: None, matched: true, .. }) => {
                self.values.pop();
                self.paths.pop();
                Ok(())
            }
            _ => Err(NC_E_INVALID_ARG),
        }
    }
    /// Checks the scalar's syntax category before converting its declared width.
    fn number(&mut self) -> Result<Number, NcStatus> {
        match self.unread()? {
            Node::Number(number) => Ok(number),
            _ => Err(NC_E_INVALID_ARG),
        }
    }
    /// Opens a counted collection and preflights its required logical children.
    fn ordered(&mut self, kind: Ordered) -> Result<u32, NcStatus> {
        let mut node = self.unread()?;
        if kind == Ordered::Map {
            let Node::Object(ref mut fields) = node else {
                return Err(NC_E_INVALID_ARG);
            };
            if fields.len() != 1 {
                return Err(NC_E_INVALID_ARG);
            }
            node = fields.remove("$map").ok_or(NC_E_INVALID_ARG)?;
        }
        let Node::Array(nodes) = node else {
            return Err(NC_E_INVALID_ARG);
        };
        let count = nodes.len();
        self.budget.collection(count, if kind == Ordered::Map { 2 } else { 1 })?;
        let mut values = VecDeque::new();
        if kind == Ordered::Map {
            for pair in nodes {
                let Node::Array(pair) = pair else {
                    return Err(NC_E_INVALID_ARG);
                };
                if pair.len() != 2 {
                    return Err(NC_E_INVALID_ARG);
                }
                values.extend(pair);
            }
        } else {
            values.extend(nodes);
        }
        *self.values.last_mut().ok_or(NC_E_INVALID_ARG)? = ReadValue::Ordered { kind, values, index: 0 };
        u32::try_from(count).map_err(|_| NC_E_OUT_OF_RANGE)
    }
    /// Requires an empty ordered container and its matching end method.
    fn end_ordered(&mut self, expected: Ordered) -> Result<(), NcStatus> {
        match self.take()? {
            ReadValue::Ordered { kind, values, .. } if kind == expected && values.is_empty() => Ok(()),
            _ => Err(NC_E_INVALID_ARG),
        }
    }
}

/// One exclusive serializer; its ordinary COM gate protects all writer state.
#[neocogi_com::com_object(interfaces(ISerializer, ISerializationOutput, ISerializationDiagnostics), ownership = exclusive)]
struct JsonSerializer {
    /// Provider-owned syntax construction, budgets and committed output.
    writer: Writer,
}
/// One exclusive deserializer owns its complete parsed input.
#[neocogi_com::com_object(interfaces(IDeserializer, ISerializationDiagnostics), ownership = exclusive)]
struct JsonDeserializer {
    /// Parsed input, typed scopes and the immutable first failure.
    reader: Reader,
}
/// Shared immutable format selection; every method creates an independent session.
#[neocogi_com::com_object(ISerializationFormat)]
struct JsonFormat;

/// Creates a normal shared COM factory without a host or global format registry.
pub fn factory() -> ComPtr<ISerializationFormatAbi> {
    JsonFormat::from_value(JsonFormat)
        .query()
        .unwrap_or_else(|_| unreachable!("text format projection"))
}
/// Converts the fixed-width ABI policy before constructing local state.
fn limits(bytes: u64, elements: u32, depth: u32, values: u64) -> Result<Limits, NcStatus> {
    Ok(Limits {
        bytes: usize::try_from(bytes).map_err(|_| NC_E_OUT_OF_RANGE)?,
        elements: elements as usize,
        depth: depth as usize,
        values: usize::try_from(values).map_err(|_| NC_E_OUT_OF_RANGE)?,
    })
}
impl ISerializationFormat for JsonFormat {
    fn create_serializer(&self, bytes: u64, elements: u32, depth: u32, values: u64) -> Result<ComBox<ISerializerAbi>, NcStatus> {
        let writer = Writer::new(limits(bytes, elements, depth, values)?);
        JsonSerializer::into_box(JsonSerializer { writer }).query().map_err(|(status, _)| status)
    }
    fn create_deserializer(
        &self,
        input: NcCopyFromBufferOffset<'_, u8>,
        bytes: u64,
        elements: u32,
        depth: u32,
        values: u64,
    ) -> Result<ComBox<IDeserializerAbi>, NcStatus> {
        if input.offset() != 0 {
            return Err(NC_E_INVALID_ARG);
        }
        let limits = limits(bytes, elements, depth, values)?;
        let node = syntax::decode(input.as_slice(), limits);
        JsonDeserializer::into_box(JsonDeserializer { reader: Reader::new(node, limits) })
            .query()
            .map_err(|(status, _)| status)
    }
}
impl ISerializationOutput for JsonSerializer {
    fn byte_len(&self) -> Result<u64, NcStatus> {
        if !matches!(self.writer.phase, Phase::Finished) {
            return Err(NC_E_INVALID_ARG);
        }
        Ok(self.writer.output.len() as u64)
    }
    fn copy_to(&self, mut output: NcOutputWriter<'_, u8>) -> Result<u64, NcStatus> {
        self.byte_len()?;
        if output.offset() != 0 {
            return Err(NC_E_INVALID_ARG);
        }
        output.extend_from_slice(&self.writer.output)?;
        Ok(self.writer.output.len() as u64)
    }
}

impl ISerializer for JsonSerializer {
    /// Accepts one declared bool through the common logical budget.
    fn serialize_bool(&mut self, value: bool) -> Result<(), NcStatus> {
        self.writer.run("serialize_bool", |w| w.scalar(Node::Bool(value)))
    }
    /// Accepts one declared u8 through the common logical budget.
    fn serialize_u8(&mut self, value: u8) -> Result<(), NcStatus> {
        self.writer.run("serialize_u8", |w| w.scalar(Node::Number(Number::Unsigned(value as u64))))
    }
    /// Accepts one declared u16 through the common logical budget.
    fn serialize_u16(&mut self, value: u16) -> Result<(), NcStatus> {
        self.writer.run("serialize_u16", |w| w.scalar(Node::Number(Number::Unsigned(value as u64))))
    }
    /// Accepts one declared u32 through the common logical budget.
    fn serialize_u32(&mut self, value: u32) -> Result<(), NcStatus> {
        self.writer.run("serialize_u32", |w| w.scalar(Node::Number(Number::Unsigned(value as u64))))
    }
    /// Accepts one declared u64 through the common logical budget.
    fn serialize_u64(&mut self, value: u64) -> Result<(), NcStatus> {
        self.writer.run("serialize_u64", |w| w.scalar(Node::Number(Number::Unsigned(value))))
    }
    /// Accepts one declared i8 through the common logical budget.
    fn serialize_i8(&mut self, value: i8) -> Result<(), NcStatus> {
        self.writer.run("serialize_i8", |w| w.scalar(Node::Number(Number::Signed(value as i64))))
    }
    /// Accepts one declared i16 through the common logical budget.
    fn serialize_i16(&mut self, value: i16) -> Result<(), NcStatus> {
        self.writer.run("serialize_i16", |w| w.scalar(Node::Number(Number::Signed(value as i64))))
    }
    /// Accepts one declared i32 through the common logical budget.
    fn serialize_i32(&mut self, value: i32) -> Result<(), NcStatus> {
        self.writer.run("serialize_i32", |w| w.scalar(Node::Number(Number::Signed(value as i64))))
    }
    /// Accepts one declared i64 through the common logical budget.
    fn serialize_i64(&mut self, value: i64) -> Result<(), NcStatus> {
        self.writer.run("serialize_i64", |w| w.scalar(Node::Number(Number::Signed(value))))
    }
    /// Accepts one declared f32 through the common logical budget.
    fn serialize_f32(&mut self, value: f32) -> Result<(), NcStatus> {
        self.writer.run("serialize_f32", |w| {
            if !value.is_finite() {
                return Err(NC_E_INVALID_ARG);
            }
            w.scalar(Node::Number(Number::from_float(value as f64)))
        })
    }
    /// Accepts one declared f64 through the common logical budget.
    fn serialize_f64(&mut self, value: f64) -> Result<(), NcStatus> {
        self.writer.run("serialize_f64", |w| {
            if !value.is_finite() {
                return Err(NC_E_INVALID_ARG);
            }
            w.scalar(Node::Number(Number::from_float(value)))
        })
    }
    fn serialize_unit(&mut self) -> Result<(), NcStatus> {
        self.writer.run("serialize_unit", |w| w.scalar(Node::Null))
    }
    fn serialize_string(&mut self, value: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.writer.run("serialize_string", |w| {
            let value = text(value)?;
            w.charge(value.len())?;
            w.scalar(Node::String(value.to_owned()))
        })
    }
    fn begin_record(&mut self, count: u32) -> Result<(), NcStatus> {
        self.writer.run("begin_record", |w| {
            w.budget.enter(w.frames.len() + 1)?;
            w.budget.collection(count as usize, 1)?;
            w.frames.push(WriteFrame::Record {
                expected: count as usize,
                fields: BTreeMap::new(),
                selected: None,
            });
            Ok(())
        })
    }
    fn field(&mut self, name: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.writer.run("field", |w| {
            let name = text(name)?;
            w.charge(name.len())?;
            match w.frames.last_mut() {
                Some(WriteFrame::Record { expected, fields, selected }) if selected.is_none() && fields.len() < *expected && !fields.contains_key(name) => {
                    *selected = Some(name.to_owned());
                    Ok(())
                }
                _ => Err(NC_E_INVALID_ARG),
            }
        })
    }
    fn end_record(&mut self) -> Result<(), NcStatus> {
        self.writer.run("end_record", |w| match w.frames.pop() {
            Some(WriteFrame::Record { expected, fields, selected: None }) if fields.len() == expected => w.store(Node::Object(fields)),
            _ => Err(NC_E_INVALID_ARG),
        })
    }
    fn begin_sequence(&mut self, count: u32) -> Result<(), NcStatus> {
        self.writer.run("begin_sequence", |w| w.ordered(Ordered::Sequence, count))
    }
    fn end_sequence(&mut self) -> Result<(), NcStatus> {
        self.writer.run("end_sequence", |w| w.end_ordered(Ordered::Sequence))
    }
    fn begin_map(&mut self, count: u32) -> Result<(), NcStatus> {
        self.writer.run("begin_map", |w| w.ordered(Ordered::Map, count))
    }
    fn end_map(&mut self) -> Result<(), NcStatus> {
        self.writer.run("end_map", |w| w.end_ordered(Ordered::Map))
    }
    fn begin_option(&mut self, present: bool) -> Result<(), NcStatus> {
        self.writer.run("begin_option", |w| w.open(WriteFrame::Option { present, child: None }))
    }
    fn end_option(&mut self) -> Result<(), NcStatus> {
        self.writer.run("end_option", |w| match w.frames.pop() {
            Some(WriteFrame::Option { present: false, child: None }) => w.store(Node::Null),
            Some(WriteFrame::Option { present: true, child: Some(child) }) => w.store(object("$some", child)),
            _ => Err(NC_E_INVALID_ARG),
        })
    }
    fn begin_variant(&mut self, _index: u32, name: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.writer.run("begin_variant", |w| {
            let name = text(name)?;
            w.charge(name.len())?;
            w.open(WriteFrame::Variant { name: name.to_owned(), child: None })
        })
    }
    fn end_variant(&mut self) -> Result<(), NcStatus> {
        self.writer.run("end_variant", |w| {
            let Some(WriteFrame::Variant { name, child }) = w.frames.pop() else {
                return Err(NC_E_INVALID_ARG);
            };
            let mut fields = BTreeMap::from([("$case".to_owned(), Node::String(name))]);
            if let Some(child) = child {
                fields.insert("$value".to_owned(), child);
            }
            w.store(Node::Object(fields))
        })
    }
    fn fail(&mut self, message: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.writer.run("fail", |w| {
            w.failure = Some(Failure::new(NC_E_INVALID_ARG, text(message)?));
            Err(NC_E_INVALID_ARG)
        })
    }
    fn finish(&mut self) -> Result<(), NcStatus> {
        self.writer.run("finish", |w| {
            if !w.frames.is_empty() {
                return Err(NC_E_INVALID_ARG);
            }
            let node = w.root.take().ok_or(NC_E_INVALID_ARG)?;
            SyntaxBudget::new(w.budget.limits).validate(&node, 1)?;
            w.output = syntax::encode(&node, w.budget.limits.bytes)?;
            w.phase = Phase::Finished;
            Ok(())
        })
    }
}
impl IDeserializer for JsonDeserializer {
    fn enter_value(&mut self) -> Result<(), NcStatus> {
        self.reader.run("enter_value", Reader::enter)
    }
    fn leave_value(&mut self) -> Result<(), NcStatus> {
        self.reader.run("leave_value", Reader::leave)
    }
    fn deserialize_unit(&mut self) -> Result<(), NcStatus> {
        self.reader.run("deserialize_unit", |r| match r.unread()? {
            Node::Null => Ok(()),
            _ => Err(NC_E_INVALID_ARG),
        })
    }
    fn deserialize_bool(&mut self) -> Result<bool, NcStatus> {
        self.reader.run("deserialize_bool", |r| match r.unread()? {
            Node::Bool(value) => Ok(value),
            _ => Err(NC_E_INVALID_ARG),
        })
    }
    /// Checks exact integer syntax and the requested u8 range.
    fn deserialize_u8(&mut self) -> Result<u8, NcStatus> {
        self.reader.run("deserialize_u8", |r| match r.number()? {
            Number::Signed(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Unsigned(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Float { .. } => Err(NC_E_INVALID_ARG),
        })
    }
    /// Checks exact integer syntax and the requested u16 range.
    fn deserialize_u16(&mut self) -> Result<u16, NcStatus> {
        self.reader.run("deserialize_u16", |r| match r.number()? {
            Number::Signed(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Unsigned(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Float { .. } => Err(NC_E_INVALID_ARG),
        })
    }
    /// Checks exact integer syntax and the requested u32 range.
    fn deserialize_u32(&mut self) -> Result<u32, NcStatus> {
        self.reader.run("deserialize_u32", |r| match r.number()? {
            Number::Signed(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Unsigned(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Float { .. } => Err(NC_E_INVALID_ARG),
        })
    }
    /// Checks exact integer syntax and the requested u64 range.
    fn deserialize_u64(&mut self) -> Result<u64, NcStatus> {
        self.reader.run("deserialize_u64", |r| match r.number()? {
            Number::Signed(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Unsigned(value) => Ok(value),
            Number::Float { .. } => Err(NC_E_INVALID_ARG),
        })
    }
    /// Checks exact integer syntax and the requested i8 range.
    fn deserialize_i8(&mut self) -> Result<i8, NcStatus> {
        self.reader.run("deserialize_i8", |r| match r.number()? {
            Number::Signed(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Unsigned(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Float { .. } => Err(NC_E_INVALID_ARG),
        })
    }
    /// Checks exact integer syntax and the requested i16 range.
    fn deserialize_i16(&mut self) -> Result<i16, NcStatus> {
        self.reader.run("deserialize_i16", |r| match r.number()? {
            Number::Signed(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Unsigned(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Float { .. } => Err(NC_E_INVALID_ARG),
        })
    }
    /// Checks exact integer syntax and the requested i32 range.
    fn deserialize_i32(&mut self) -> Result<i32, NcStatus> {
        self.reader.run("deserialize_i32", |r| match r.number()? {
            Number::Signed(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Unsigned(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Float { .. } => Err(NC_E_INVALID_ARG),
        })
    }
    /// Checks exact integer syntax and the requested i64 range.
    fn deserialize_i64(&mut self) -> Result<i64, NcStatus> {
        self.reader.run("deserialize_i64", |r| match r.number()? {
            Number::Signed(value) => Ok(value),
            Number::Unsigned(value) => value.try_into().map_err(|_| NC_E_OUT_OF_RANGE),
            Number::Float { .. } => Err(NC_E_INVALID_ARG),
        })
    }
    /// Acquire the requested width directly. Only parsed negative integer syntax
    /// produces Signed(0), so its otherwise-lost lexical sign remains available.
    fn deserialize_f64(&mut self) -> Result<f64, NcStatus> {
        self.reader.run("deserialize_f64", |r| {
            Ok(match r.number()? {
                Number::Signed(0) => -0.0,
                Number::Signed(value) => value as f64,
                Number::Unsigned(value) => value as f64,
                Number::Float { wide, .. } => wide,
            })
        })
    }
    /// Integer casts and the parser's cached decimal conversion both round once.
    /// An overflowing narrow result fails here, preserving finite-f64 admission.
    fn deserialize_f32(&mut self) -> Result<f32, NcStatus> {
        self.reader.run("deserialize_f32", |r| match r.number()? {
            Number::Signed(0) => Ok(-0.0),
            Number::Signed(value) => Ok(value as f32),
            Number::Unsigned(value) => Ok(value as f32),
            Number::Float { narrow, .. } => narrow.ok_or(NC_E_OUT_OF_RANGE),
        })
    }
    fn string_len(&mut self) -> Result<u32, NcStatus> {
        self.reader.run("string_len", |r| {
            let Node::String(value) = r.unread()? else {
                return Err(NC_E_INVALID_ARG);
            };
            let len = u32::try_from(value.len()).map_err(|_| NC_E_OUT_OF_RANGE)?;
            *r.values.last_mut().ok_or(NC_E_INVALID_ARG)? = ReadValue::String(value);
            Ok(len)
        })
    }
    fn copy_string(&mut self, mut output: NcOutputWriter<'_, u8>) -> Result<u32, NcStatus> {
        self.reader.run("copy_string", |r| {
            if output.offset() != 0 {
                return Err(NC_E_INVALID_ARG);
            }
            let ReadValue::String(value) = r.take()? else {
                return Err(NC_E_INVALID_ARG);
            };
            output.extend_from_slice(value.as_bytes())?;
            Ok(value.len() as u32)
        })
    }
    fn option_present(&mut self) -> Result<bool, NcStatus> {
        self.reader.run("option_present", |r| {
            let child = match r.unread()? {
                Node::Null => None,
                Node::Object(mut fields) if fields.len() == 1 => Some(fields.remove("$some").ok_or(NC_E_INVALID_ARG)?),
                _ => return Err(NC_E_INVALID_ARG),
            };
            let present = child.is_some();
            *r.values.last_mut().ok_or(NC_E_INVALID_ARG)? = ReadValue::Option(child);
            Ok(present)
        })
    }
    fn begin_record(&mut self, maximum: u32) -> Result<u32, NcStatus> {
        self.reader.run("begin_record", |r| {
            let Node::Object(fields) = r.unread()? else {
                return Err(NC_E_INVALID_ARG);
            };
            let len = fields.len();
            if len > maximum as usize {
                return Err(NC_E_INVALID_ARG);
            }
            r.budget.collection(len, 1)?;
            *r.values.last_mut().ok_or(NC_E_INVALID_ARG)? = ReadValue::Record { fields, selected: None, missing: None };
            u32::try_from(len).map_err(|_| NC_E_OUT_OF_RANGE)
        })
    }
    fn field(&mut self, name: NcCopyFromBufferOffset<'_, u8>) -> Result<bool, NcStatus> {
        self.reader.run("field", |r| {
            let name = text(name)?;
            match r.values.last_mut() {
                Some(ReadValue::Record { fields, selected, missing }) if selected.is_none() => {
                    *selected = fields.remove_entry(name);
                    *missing = if selected.is_none() { Some(name.to_owned()) } else { None };
                    Ok(selected.is_some())
                }
                _ => Err(NC_E_INVALID_ARG),
            }
        })
    }
    fn end_record(&mut self) -> Result<(), NcStatus> {
        self.reader.run("end_record", |r| match r.take()? {
            ReadValue::Record { fields, selected: None, .. } if fields.is_empty() => Ok(()),
            _ => Err(NC_E_INVALID_ARG),
        })
    }
    fn begin_sequence(&mut self) -> Result<u32, NcStatus> {
        self.reader.run("begin_sequence", |r| r.ordered(Ordered::Sequence))
    }
    fn end_sequence(&mut self) -> Result<(), NcStatus> {
        self.reader.run("end_sequence", |r| r.end_ordered(Ordered::Sequence))
    }
    fn begin_map(&mut self) -> Result<u32, NcStatus> {
        self.reader.run("begin_map", |r| r.ordered(Ordered::Map))
    }
    fn end_map(&mut self) -> Result<(), NcStatus> {
        self.reader.run("end_map", |r| r.end_ordered(Ordered::Map))
    }
    fn variant_is(&mut self, _index: u32, name: NcCopyFromBufferOffset<'_, u8>) -> Result<bool, NcStatus> {
        self.reader.run("variant_is", |r| {
            let name = text(name)?;
            if matches!(r.values.last(), Some(ReadValue::Unread(_))) {
                let Node::Object(mut fields) = r.unread()? else {
                    return Err(NC_E_INVALID_ARG);
                };
                let Some(Node::String(case)) = fields.remove("$case") else {
                    return Err(NC_E_INVALID_ARG);
                };
                let child = fields.remove("$value");
                if !fields.is_empty() {
                    return Err(NC_E_INVALID_ARG);
                }
                *r.values.last_mut().ok_or(NC_E_INVALID_ARG)? = ReadValue::Variant { name: case, child, matched: false };
            }
            match r.values.last_mut() {
                Some(ReadValue::Variant { name: case, matched, .. }) if !*matched => {
                    *matched = case == name;
                    Ok(*matched)
                }
                _ => Err(NC_E_INVALID_ARG),
            }
        })
    }
    fn fail(&mut self, message: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.reader.run("fail", |r| {
            let mut error = Failure::new(NC_E_INVALID_ARG, text(message)?);
            error.path = r.path(PathSite::SelectedField);
            r.failure = Some(error);
            Err(NC_E_INVALID_ARG)
        })
    }
    fn finish(&mut self) -> Result<(), NcStatus> {
        self.reader.run("finish", |r| {
            if !r.opened || r.root.is_some() || !r.values.is_empty() {
                return Err(NC_E_INVALID_ARG);
            }
            r.phase = Phase::Finished;
            Ok(())
        })
    }
}

/// Path components borrow no foreign memory. Field names move out of the parsed
/// map; indices require no string allocation until a diagnostic is requested.
enum Part {
    /// Root document marker.
    Root,
    /// Authored named field, owned by this active scope.
    Field(String),
    /// Sequence element ordinal.
    Index(usize),
    /// Alternating key/value ordinal in an arbitrary-key map.
    Map(usize),
    /// Present optional payload.
    Some,
    /// Selected enum payload.
    Payload,
}
impl Part {
    /// Formats a component only while constructing the first failure diagnostic.
    fn append(&self, path: &mut String) {
        use core::fmt::Write;
        match self {
            Self::Root => {}
            Self::Field(name) => {
                let _ = write!(path, ".{name}");
            }
            Self::Index(index) => {
                let _ = write!(path, "[{index}]");
            }
            Self::Map(index) => {
                let _ = write!(path, "[$map][{}].{}", index / 2, if index % 2 == 0 { "key" } else { "value" });
            }
            Self::Some => path.push_str(".$some"),
            Self::Payload => path.push_str(".$value"),
        }
    }
}
impl Reader {
    /// Reports the current representation without inspecting application values.
    fn actual(&self) -> &'static str {
        match self.values.last() {
            Some(ReadValue::Unread(Node::Null)) => "null",
            Some(ReadValue::Unread(Node::Bool(_))) => "boolean",
            Some(ReadValue::Unread(Node::Number(_))) => "number",
            Some(ReadValue::Unread(Node::String(_))) => "string",
            Some(ReadValue::Unread(Node::Array(_))) => "array",
            Some(ReadValue::Unread(Node::Object(_))) => "object",
            _ => "incomplete or consumed value",
        }
    }
    /// Builds a logical path only on failure; successful scalar calls do no formatting.
    fn path(&self, site: PathSite) -> String {
        let mut path = String::from("$");
        for part in &self.paths {
            part.append(&mut path);
        }
        if site != PathSite::Current {
            match self.values.last() {
                Some(ReadValue::Record { selected: Some((name, _)), .. }) => Part::Field(name.clone()).append(&mut path),
                Some(ReadValue::Record { missing: Some(name), .. }) => Part::Field(name.clone()).append(&mut path),
                Some(ReadValue::Ordered { kind, index, .. }) if site == PathSite::NextChild => {
                    if *kind == Ordered::Map {
                        Part::Map(*index).append(&mut path);
                    } else {
                        Part::Index(*index).append(&mut path);
                    }
                }
                Some(ReadValue::Option(_)) if site == PathSite::NextChild => Part::Some.append(&mut path),
                Some(ReadValue::Variant { matched: true, .. }) if site == PathSite::NextChild => Part::Payload.append(&mut path),
                _ => {}
            }
        }
        path
    }
}
/// Borrows caller-visible diagnostic data from one failed owning text session.
fn details(error: Option<&Failure>) -> Result<crate::diagnostics::Details<'_>, NcStatus> {
    let error = error.ok_or(NC_E_INVALID_ARG)?;
    Ok(crate::diagnostics::Details {
        status: error.status,
        message: &error.message,
        path: &error.path,
        offset: error.byte_offset.unwrap_or(u64::MAX),
        line: error.line.unwrap_or(0),
        column: error.column.unwrap_or(0),
    })
}
impl JsonSerializer {
    /// Successful/active sessions expose no fabricated failure.
    fn details(&self) -> Result<crate::diagnostics::Details<'_>, NcStatus> {
        details(self.writer.failure.as_ref())
    }
}
impl JsonDeserializer {
    /// Syntax errors and later schema errors share the same immutable projection.
    fn details(&self) -> Result<crate::diagnostics::Details<'_>, NcStatus> {
        details(self.reader.failure.as_ref())
    }
}
diagnostic_projection!(JsonSerializer);
diagnostic_projection!(JsonDeserializer);

/// Human-readable expectations are constructed only on the error path.
fn expectation(operation: &str) -> &str {
    match operation {
        "deserialize_bool" => "expected boolean",
        "deserialize_unit" => "expected unit/null",
        "deserialize_u8" => "expected u8 integer in range",
        "deserialize_u16" => "expected u16 integer in range",
        "deserialize_u32" => "expected u32 integer in range",
        "deserialize_u64" => "expected u64 integer in range",
        "deserialize_i8" => "expected i8 integer in range",
        "deserialize_i16" => "expected i16 integer in range",
        "deserialize_i32" => "expected i32 integer in range",
        "deserialize_i64" => "expected i64 integer in range",
        "deserialize_f32" => "expected finite f32",
        "deserialize_f64" => "expected finite f64",
        "string_len" => "expected UTF-8 string",
        "begin_record" => "expected record with declared fields",
        "end_record" => "unknown or unconsumed record fields",
        "begin_sequence" => "expected sequence",
        "begin_map" => "expected map entries",
        "leave_value" => "incomplete value or unclosed container",
        "finish" => "incomplete document",
        _ => operation,
    }
}
