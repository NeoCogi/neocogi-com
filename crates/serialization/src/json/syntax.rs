//! Small strict UTF-8 JSON parser and compact writer. Syntax is admitted before
//! constructing nodes; no generic serialization framework or external parser is
//! part of the runtime. Numeric conversion uses Rust's correctly rounded floats.
use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};
use core::fmt::Write;
use neocogi_com::{NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
use neocogi_com::serialization::{Error, Limits};
use super::grammar::{Node, Number, SyntaxBudget, Output};

/// Parses exactly one bounded document and retains a precise source diagnostic.
pub(super) fn decode(bytes: &[u8], limits: Limits) -> Result<Node, Error> {
    if bytes.len() > limits.bytes {
        return Err(location(bytes, 0, NC_E_OUT_OF_RANGE, "document byte limit"));
    }
    let input = core::str::from_utf8(bytes).map_err(|e| location(bytes, e.valid_up_to(), NC_E_INVALID_ARG, "invalid UTF-8"))?;
    let mut parser = Parser {
        input,
        position: 0,
        budget: SyntaxBudget::new(limits),
    };
    let value = parser.value(1)?;
    parser.whitespace();
    if parser.position != input.len() {
        return Err(parser.error(NC_E_INVALID_ARG, "trailing data after JSON value"));
    }
    Ok(value)
}
/// Calculates coordinates only on failure; successful input scanning maintains
/// a byte cursor without a per-character line/column bookkeeping cost.
fn location(bytes: &[u8], offset: usize, status: NcStatus, message: &str) -> Error {
    let prefix = &bytes[..offset.min(bytes.len())];
    let line = prefix.iter().filter(|&&byte| byte == b'\n').count() + 1;
    let start = prefix.iter().rposition(|&byte| byte == b'\n').map_or(0, |index| index + 1);
    let column = core::str::from_utf8(&prefix[start..]).map_or(prefix.len() - start, |text| text.chars().count()) + 1;
    let mut error = Error::new(status, message);
    error.byte_offset = Some(offset as u64);
    error.line = u32::try_from(line).ok();
    error.column = u32::try_from(column).ok();
    error
}
/// One borrowed input and monotonic cursor; all returned syntax owns its strings.
struct Parser<'a> {
    /// Validated UTF-8, retained only during factory parsing.
    input: &'a str,
    /// Next unread source byte; token/string boundaries remain UTF-8 boundaries.
    position: usize,
    /// Global node, nesting and immediate-container limits.
    budget: SyntaxBudget,
}
impl Parser<'_> {
    /// Associates the current byte cursor with a stable explanatory diagnostic.
    fn error(&self, status: NcStatus, message: &str) -> Error {
        location(self.input.as_bytes(), self.position, status, message)
    }
    /// Looks ahead without consuming an absent byte or reading outside the input.
    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }
    /// JSON whitespace consists of exactly these four ASCII characters.
    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }
    /// Admits a node before parsing or allocating its representation.
    fn value(&mut self, depth: usize) -> Result<Node, Error> {
        self.whitespace();
        self.budget
            .enter(depth)
            .map_err(|status| self.error(status, "JSON nesting or node limit exceeded"))?;
        match self.peek() {
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Node::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Node::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Node::Bool(false))
            }
            Some(b'"') => self.string().map(Node::String),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.error(NC_E_INVALID_ARG, "expected JSON value")),
        }
    }
    /// Consumes one fixed JSON literal; delimiters are checked by its enclosing scope.
    fn literal(&mut self, expected: &[u8]) -> Result<(), Error> {
        if !self.input.as_bytes()[self.position..].starts_with(expected) {
            return Err(self.error(NC_E_INVALID_ARG, "invalid JSON literal"));
        }
        self.position += expected.len();
        Ok(())
    }
    /// Rejects an extra collection element before parsing any of its subtree.
    fn element(&self, count: usize) -> Result<(), Error> {
        if count >= self.budget.elements {
            Err(self.error(NC_E_OUT_OF_RANGE, "JSON container limit exceeded"))
        } else {
            Ok(())
        }
    }
    /// Builds one ordered array, rejecting missing elements and trailing commas.
    fn array(&mut self, depth: usize) -> Result<Node, Error> {
        self.position += 1;
        self.whitespace();
        let mut values = Vec::new();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(Node::Array(values));
        }
        loop {
            self.element(values.len())?;
            values.push(self.value(depth + 1)?);
            self.whitespace();
            match self.peek() {
                Some(b']') => {
                    self.position += 1;
                    return Ok(Node::Array(values));
                }
                Some(b',') => {
                    self.position += 1;
                    self.whitespace();
                }
                _ => return Err(self.error(NC_E_INVALID_ARG, "expected ',' or ']' after array element")),
            }
        }
    }
    /// Builds a unique-key object with admission before acquiring another key/value.
    fn object(&mut self, depth: usize) -> Result<Node, Error> {
        self.position += 1;
        self.whitespace();
        let mut fields = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(Node::Object(fields));
        }
        loop {
            self.element(fields.len())?;
            let name = self.string()?;
            if fields.contains_key(&name) {
                return Err(self.error(NC_E_INVALID_ARG, "duplicate JSON object key"));
            }
            self.whitespace();
            if self.peek() != Some(b':') {
                return Err(self.error(NC_E_INVALID_ARG, "expected ':' after object key"));
            }
            self.position += 1;
            let value = self.value(depth + 1)?;
            fields.insert(name, value);
            self.whitespace();
            match self.peek() {
                Some(b'}') => {
                    self.position += 1;
                    return Ok(Node::Object(fields));
                }
                Some(b',') => {
                    self.position += 1;
                    self.whitespace();
                }
                _ => return Err(self.error(NC_E_INVALID_ARG, "expected ',' or '}' after object member")),
            }
        }
    }
    /// Copies unescaped strings in one operation, allocating an escape buffer only
    /// when required. ASCII quote/backslash boundaries cannot split UTF-8 scalars.
    fn string(&mut self) -> Result<String, Error> {
        if self.peek() != Some(b'"') {
            return Err(self.error(NC_E_INVALID_ARG, "expected quoted JSON string"));
        }
        self.position += 1;
        let mut start = self.position;
        let mut escaped: Option<String> = None;
        loop {
            match self.peek() {
                Some(b'"') => {
                    let tail = &self.input[start..self.position];
                    self.position += 1;
                    return Ok(match escaped {
                        Some(mut text) => {
                            text.push_str(tail);
                            text
                        }
                        None => tail.to_owned(),
                    });
                }
                Some(b'\\') => {
                    let text = escaped.get_or_insert_with(String::new);
                    text.push_str(&self.input[start..self.position]);
                    self.position += 1;
                    let character = match self.peek() {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'u') => {
                            self.position += 1;
                            let high = self.hex()?;
                            let scalar = if (0xd800..=0xdbff).contains(&high) {
                                if !self.input.as_bytes()[self.position..].starts_with(b"\\u") {
                                    return Err(self.error(NC_E_INVALID_ARG, "unpaired high Unicode surrogate"));
                                }
                                self.position += 2;
                                let low = self.hex()?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return Err(self.error(NC_E_INVALID_ARG, "expected low Unicode surrogate"));
                                }
                                0x10000 + ((u32::from(high) - 0xd800) << 10) + u32::from(low) - 0xdc00
                            } else if (0xdc00..=0xdfff).contains(&high) {
                                return Err(self.error(NC_E_INVALID_ARG, "unpaired low Unicode surrogate"));
                            } else {
                                u32::from(high)
                            };
                            text.push(char::from_u32(scalar).ok_or_else(|| self.error(NC_E_INVALID_ARG, "invalid Unicode scalar"))?);
                            start = self.position;
                            continue;
                        }
                        _ => return Err(self.error(NC_E_INVALID_ARG, "invalid JSON string escape")),
                    };
                    self.position += 1;
                    text.push(character);
                    start = self.position;
                }
                Some(0..=0x1f) => return Err(self.error(NC_E_INVALID_ARG, "unescaped control character in JSON string")),
                Some(_) => self.position += 1,
                None => return Err(self.error(NC_E_INVALID_ARG, "unterminated JSON string")),
            }
        }
    }
    /// Acquires exactly four hexadecimal digits; invalid/short escapes never
    /// advance past an unread byte or create an invalid Rust character.
    fn hex(&mut self) -> Result<u16, Error> {
        let mut value = 0;
        for _ in 0..4 {
            let digit = match self.peek() {
                Some(v @ b'0'..=b'9') => v - b'0',
                Some(v @ b'a'..=b'f') => v - b'a' + 10,
                Some(v @ b'A'..=b'F') => v - b'A' + 10,
                _ => return Err(self.error(NC_E_INVALID_ARG, "expected four hexadecimal Unicode digits")),
            };
            self.position += 1;
            value = value * 16 + u16::from(digit);
        }
        Ok(value)
    }
    /// Checks JSON's number grammar before using standard integer/float conversion.
    /// Integer endpoints stay exact rather than being routed through f64.
    fn number(&mut self) -> Result<Node, Error> {
        let start = self.position;
        let negative = self.peek() == Some(b'-');
        if negative {
            self.position += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.position += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(self.error(NC_E_INVALID_ARG, "leading zero in JSON number"));
                }
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(self.error(NC_E_INVALID_ARG, "expected integer digits")),
        }
        let mut floating = false;
        if self.peek() == Some(b'.') {
            floating = true;
            self.position += 1;
            self.digits("expected digits after decimal point")?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            floating = true;
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            self.digits("expected exponent digits")?;
        }
        let token = &self.input[start..self.position];
        let number = if floating {
            let value = token.parse::<f64>().map_err(|_| self.error(NC_E_INVALID_ARG, "invalid JSON number"))?;
            if !value.is_finite() {
                return Err(self.error(NC_E_OUT_OF_RANGE, "JSON float is out of range"));
            }
            // Retain direct-width results while this validated token is borrowed.
            // f32 overflow is a typed-acquisition error, not a document error.
            let narrow = token.parse::<f32>().map_err(|_| self.error(NC_E_INVALID_ARG, "invalid JSON number"))?;
            Number::Float {
                wide: value,
                narrow: narrow.is_finite().then_some(narrow),
            }
        } else {
            let digits = token.strip_prefix('-').unwrap_or(token);
            let magnitude = digits
                .parse::<u64>()
                .map_err(|_| self.error(NC_E_OUT_OF_RANGE, "JSON integer is out of range"))?;
            if negative {
                if magnitude > (1u64 << 63) {
                    return Err(self.error(NC_E_OUT_OF_RANGE, "JSON signed integer is out of range"));
                }
                Number::Signed(if magnitude == (1u64 << 63) { i64::MIN } else { -(magnitude as i64) })
            } else {
                Number::Unsigned(magnitude)
            }
        };
        Ok(Node::Number(number))
    }
    /// Requires a nonempty run rather than accepting a partial fraction/exponent.
    fn digits(&mut self, message: &str) -> Result<(), Error> {
        let start = self.position;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.position += 1;
        }
        if self.position == start {
            Err(self.error(NC_E_INVALID_ARG, message))
        } else {
            Ok(())
        }
    }
}

/// Writes one compact document into bounded uncommitted output.
pub(super) fn encode(node: &Node, maximum: usize) -> Result<Vec<u8>, NcStatus> {
    let mut output = Output::new(maximum);
    emit(node, &mut output).map_err(|_| output.error.unwrap_or(NC_E_INVALID_ARG))?;
    Ok(output.bytes)
}
/// Writes contiguous UTF-8 runs in bulk, escaping only JSON's required bytes.
pub(super) fn quoted(value: &str, output: &mut impl Write) -> core::fmt::Result {
    output.write_char('"')?;
    let mut start = 0;
    for (index, byte) in value.bytes().enumerate() {
        if byte < 0x20 || matches!(byte, b'"' | b'\\') {
            output.write_str(&value[start..index])?;
            match byte {
                b'"' => output.write_str("\\\"")?,
                b'\\' => output.write_str("\\\\")?,
                b'\n' => output.write_str("\\n")?,
                b'\r' => output.write_str("\\r")?,
                b'\t' => output.write_str("\\t")?,
                8 => output.write_str("\\b")?,
                12 => output.write_str("\\f")?,
                _ => write!(output, "\\u{byte:04x}")?,
            }
            start = index + 1;
        }
    }
    output.write_str(&value[start..])?;
    output.write_char('"')
}
/// Emits only the previously admitted finite syntax tree, never application layout.
fn emit(node: &Node, output: &mut Output) -> core::fmt::Result {
    match node {
        Node::Null => output.write_str("null"),
        Node::Bool(value) => write!(output, "{value}"),
        Node::Number(Number::Signed(value)) => write!(output, "{value}"),
        Node::Number(Number::Unsigned(value)) => write!(output, "{value}"),
        // LowerExp is functional numeric formatting, independent of Debug
        // stripping in min-size builds. Its exponent also preserves float syntax
        // and negative zero. Provider entry already rejected NaN/infinity.
        Node::Number(Number::Float { wide, .. }) => write!(output, "{wide:e}"),
        Node::String(value) => quoted(value, output),
        Node::Array(values) => {
            output.write_char('[')?;
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.write_char(',')?;
                }
                emit(value, output)?;
            }
            output.write_char(']')
        }
        Node::Object(fields) => {
            output.write_char('{')?;
            for (index, (name, value)) in fields.iter().enumerate() {
                if index != 0 {
                    output.write_char(',')?;
                }
                quoted(name, output)?;
                output.write_char(':')?;
                emit(value, output)?;
            }
            output.write_char('}')
        }
    }
}
