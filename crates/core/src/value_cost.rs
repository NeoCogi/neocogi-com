//! Compile-time value counts and nesting depths for allocation-free conversions.
//!
//! None means the codec must account dynamically. Costs never imply that raw
//! contents are valid: the same boolean/tag/field validators still run afterward.

/// Logical value count and maximum nesting depth for a conversion proof.
/// `aggregate`, `alternatives`, and `array` preserve exact input counts;
/// `maximum` instead constructs an upper bound covering any enum alternative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueCost {
    /// Exact count or upper bound, including the root and its active descendants.
    pub(crate) values: usize,
    /// Longest root-to-leaf path, including this subtree's root.
    pub(crate) depth: usize,
}
impl ValueCost {
    /// One primitive, unit value, or other allocation-free scalar conversion.
    pub const SCALAR: Self = Self { values: 1, depth: 1 };

    /// Combines fixed children into one record or selected enum alternative.
    /// A dynamic child or arithmetic overflow prevents fixed pre-accounting.
    pub const fn aggregate(children: &[Option<Self>]) -> Option<Self> {
        let mut result = Self::SCALAR;
        let mut index = 0;
        while index < children.len() {
            let child = match children[index] {
                Some(cost) => cost,
                None => return None,
            };
            result.values = match result.values.checked_add(child.values) {
                Some(value) => value,
                None => return None,
            };
            let depth = match child.depth.checked_add(1) {
                Some(value) => value,
                None => return None,
            };
            if depth > result.depth {
                result.depth = depth;
            }
            index += 1;
        }
        Some(result)
    }

    /// Counts an aggregate and its fixed fields, leaving dynamic fields to their codecs.
    ///
    /// This is a partial count, unlike `aggregate`: a field with no fixed cost is
    /// omitted here and must still receive the shared budget during conversion.
    /// The selected enum alternative can use this even when other alternatives
    /// have different field counts. Overflow means no representable budget fits.
    #[doc(hidden)]
    pub const fn known_fields(children: &[Option<Self>]) -> Option<Self> {
        let mut result = Self::SCALAR;
        let mut index = 0;
        while index < children.len() {
            if let Some(child) = children[index] {
                result.values = match result.values.checked_add(child.values) {
                    Some(value) => value,
                    None => return None,
                };
                let depth = match child.depth.checked_add(1) {
                    Some(value) => value,
                    None => return None,
                };
                if depth > result.depth {
                    result.depth = depth;
                }
            }
            index += 1;
        }
        Some(result)
    }

    /// Resolves an enum only when every alternative has the same exact cost.
    /// Different shapes retain tag-dependent accounting rather than an upper bound.
    pub const fn alternatives(cases: &[Option<Self>]) -> Option<Self> {
        if cases.is_empty() {
            return None;
        }
        let first = match cases[0] {
            Some(cost) => cost,
            None => return None,
        };
        let mut index = 1;
        while index < cases.len() {
            match cases[index] {
                Some(cost) if cost.values == first.values && cost.depth == first.depth => {}
                _ => return None,
            }
            index += 1;
        }
        Some(first)
    }

    /// Bounds any one enum alternative without counting inactive alternatives.
    ///
    /// Unlike `alternatives`, the result is an upper bound, not an exact count.
    /// A fresh conversion may skip bookkeeping only if this complete bound fits;
    /// otherwise it must account for the actual selected fields normally.
    pub const fn maximum(cases: &[Option<Self>]) -> Option<Self> {
        if cases.is_empty() {
            return None;
        }
        let mut result = Self::SCALAR;
        let mut index = 0;
        while index < cases.len() {
            let cost = match cases[index] {
                Some(cost) => cost,
                None => return None,
            };
            if cost.values > result.values {
                result.values = cost.values;
            }
            if cost.depth > result.depth {
                result.depth = cost.depth;
            }
            index += 1;
        }
        Some(result)
    }

    /// Scales a fixed element cost without adding a container root or depth level.
    /// Buffer conversion has no aggregate root; owned sequences charge theirs separately.
    #[cfg(feature = "alloc")]
    pub(crate) fn elements(self, len: usize) -> Option<Self> {
        Some(Self {
            values: self.values.checked_mul(len)?,
            depth: if len == 0 { 0 } else { self.depth },
        })
    }

    /// Accounts for one array root and its compile-time number of fixed children.
    pub const fn array(child: Option<Self>, len: usize) -> Option<Self> {
        if len == 0 {
            return Some(Self::SCALAR);
        }
        let child = match child {
            Some(cost) => cost,
            None => return None,
        };
        let descendants = match child.values.checked_mul(len) {
            Some(value) => value,
            None => return None,
        };
        let values = match descendants.checked_add(1) {
            Some(value) => value,
            None => return None,
        };
        let depth = match child.depth.checked_add(1) {
            Some(value) => value,
            None => return None,
        };
        Some(Self { values, depth })
    }
}
