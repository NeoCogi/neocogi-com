//! Private JSON syntax, never an application payload ABI.
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use neocogi_com_sdk::{NcStatus, NC_E_OUT_OF_RANGE, NC_E_OUT_OF_MEMORY};
use neocogi_com_sdk::serialization::Limits;

/// Exact numeric categories retained until a typed COM scalar is requested.
#[derive(Debug)]
pub(super) enum Number {
    /// Signed integral syntax, including the complete i64 domain.
    Signed(i64),
    /// Unsigned integral syntax, including values above i64::MAX.
    Unsigned(u64),
    /// Requested-width results for fractional/exponent syntax. Parsing both
    /// widths directly avoids double rounding without retaining source bytes.
    Float {
        /// Finite binary64 value admitted by the document syntax policy.
        wide: f64,
        /// Direct binary32 conversion, absent on overflow. A wide-only value
        /// remains valid syntax until the caller actually requests f32.
        narrow: Option<f32>,
    },
}

impl Number {
    /// Store an already checked finite writer value without parsing its output.
    /// Writer nodes are emitted, never supplied to the parser-owned Reader.
    pub fn from_float(wide: f64) -> Self {
        let narrow = wide as f32;
        Self::Float {
            wide,
            narrow: narrow.is_finite().then_some(narrow),
        }
    }
}

/// Owned JSON syntax. No Rust application type, COM object or trait object
/// is stored here. Container metadata belongs to the documented text grammar.
#[derive(Debug)]
pub(super) enum Node {
    /// Unit or absent option, distinguished by the requested typed operation.
    Null,
    /// Boolean syntax is distinct from numeric and string syntax.
    Bool(bool),
    /// Numeric width is checked when the caller requests its concrete scalar.
    Number(Number),
    /// Valid UTF-8 owned by the document implementation.
    String(String),
    /// Ordered syntax, also used by the explicit map-entry grammar.
    Array(Vec<Node>),
    /// Unique string keys; duplicates are rejected while parsing or constructing.
    Object(BTreeMap<String, Node>),
}

/// Finite syntax admission before building a node. Tagged option/case/map syntax
/// has bounded overhead beyond logical COM values; semantic budgets stay exact.
pub(super) struct SyntaxBudget {
    /// Remaining syntax nodes, including tags and map-pair containers.
    remaining: usize,
    /// At most 128 syntax levels, independently of caller-supplied limits.
    depth: usize,
    /// At least two slots permit fixed metadata and key/value pairs.
    pub elements: usize,
}
impl SyntaxBudget {
    /// Derives caps sufficient for the documented grammar without unbounded parsing.
    pub fn new(limits: Limits) -> Self {
        Self {
            remaining: limits.values.saturating_mul(3).saturating_add(16),
            depth: limits.depth.saturating_mul(3).saturating_add(4).min(128),
            elements: limits.elements.max(2),
        }
    }
    /// Charges a node before allocating its container or acquiring its children.
    pub fn enter(&mut self, depth: usize) -> Result<(), NcStatus> {
        if depth > self.depth || self.remaining == 0 {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.remaining -= 1;
        Ok(())
    }
    /// Writers must satisfy the same syntax ceilings as the corresponding parser.
    pub fn validate(&mut self, node: &Node, depth: usize) -> Result<(), NcStatus> {
        self.enter(depth)?;
        match node {
            Node::Array(values) => {
                if values.len() > self.elements {
                    return Err(NC_E_OUT_OF_RANGE);
                }
                for value in values {
                    self.validate(value, depth + 1)?;
                }
            }
            Node::Object(fields) => {
                if fields.len() > self.elements {
                    return Err(NC_E_OUT_OF_RANGE);
                }
                for value in fields.values() {
                    self.validate(value, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Fallible bounded JSON byte sink; no failed finish publishes bytes.
pub(super) struct Output {
    /// Complete bytes accumulated only inside the format implementation.
    pub bytes: Vec<u8>,
    /// Exact encoded-document cap, including escaping and format framing.
    maximum: usize,
    /// Original admission failure retained through a formatter's generic error.
    pub error: Option<NcStatus>,
}
impl Output {
    /// Starts an empty uncommitted output document.
    pub fn new(maximum: usize) -> Self {
        Self { bytes: Vec::new(), maximum, error: None }
    }
    /// Admits the complete append before touching initialized output.
    pub fn append(&mut self, bytes: &[u8]) -> Result<(), NcStatus> {
        let result = (|| {
            if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
                return Err(NC_E_OUT_OF_RANGE);
            }
            self.bytes.try_reserve(bytes.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            self.bytes.extend_from_slice(bytes);
            Ok(())
        })();
        if let Err(status) = result {
            self.error = Some(status);
        }
        result
    }
}
impl core::fmt::Write for Output {
    /// JSON formatting uses the same complete-append admission.
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        self.append(text.as_bytes()).map_err(|_| core::fmt::Error)
    }
}
