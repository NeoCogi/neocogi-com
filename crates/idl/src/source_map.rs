//! Portable provenance for the formatted Rust file emitted by the IDL compiler.
//!
//! This is NeoCogi's readable range-map schema, not the JavaScript Source Map v3
//! encoding. Coordinates are one-based Unicode scalar columns, matching the IDL
//! parser; ranges include their start and exclude their end.

use serde::{Deserialize, Serialize};

/// One position in the exact generated Rust or authored IDL text.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourceMapPosition {
    /// One-based physical line; tabs do not change the line number.
    pub line: usize,
    /// One-based Unicode scalar column; a tab counts as one character.
    pub column: usize,
}

/// Half-open text range, independent of filesystem offsets and platform newlines.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMapRange {
    /// First character belonging to the range.
    pub start: SourceMapPosition,
    /// First character after the range.
    pub end: SourceMapPosition,
}
impl SourceMapRange {
    /// Tests a location without treating the following item as part of this range.
    pub fn contains(&self, position: SourceMapPosition) -> bool {
        self.start <= position && position < self.end
    }
}

/// Authored identifier that caused a Rust definition or helper to be emitted.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMapOrigin {
    /// Normalized path relative to the map's IDL source root.
    pub path: String,
    /// Exact parser range of the originating identifier or import statement.
    /// Derived helpers inherit their nearest authored declaration/member origin.
    pub range: SourceMapRange,
}

/// Generated syntax categories exposed to provenance viewers and lookup tools.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMapKind {
    /// Complete output fragment, including helpers derived from one declaration.
    Declaration,
    /// Concrete Rust record or generated message wire representation.
    Struct,
    /// Rust enum or the enum-shaped body of a COM value macro.
    Enum,
    /// Generated message payload union.
    Union,
    /// Interface, factory, observer, or generated extension trait.
    Trait,
    /// Implementation block whose methods may refine the parent origin.
    Implementation,
    /// Free Rust function or helper.
    Function,
    /// Trait or implementation method, including its generated body.
    Method,
    /// Named or positional record/variant field.
    Field,
    /// Enum alternative or flag in a COM value macro.
    Variant,
    /// Constant derived from its owning declaration.
    Constant,
    /// Type alias derived from its owning declaration.
    Type,
    /// Explicit re-export of an IDL import.
    Import,
}

/// One generated range and its closest known authored origin.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMapping {
    /// Generated syntax category; declaration entries also cover helper gaps.
    pub kind: SourceMapKind,
    /// Rust name with its enclosing generated definition where applicable.
    pub name: String,
    /// Range in the final, pretty-printed Rust file beside the map.
    pub generated: SourceMapRange,
    /// IDL location responsible for this generated range.
    pub original: SourceMapOrigin,
}

/// Versioned provenance sidecar emitted beside one generated `.rs` file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RustSourceMap {
    /// NeoCogi range-map schema version; currently one.
    pub schema: u32,
    /// Exact logical IDL module being generated.
    pub module: String,
    /// Name of the generated Rust file, relative to the map's directory.
    pub generated_file: String,
    /// IDL root relative to the map directory when Cargo supplies filesystem context.
    /// Pure in-memory generation leaves this unset for its caller to resolve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    /// Deterministic ranges in generation order; nested entries refine parent origins.
    pub mappings: Vec<SourceMapping>,
}
impl RustSourceMap {
    /// Resolves a generated location to its innermost mapped definition.
    /// Equal ranges prefer the later, more specific syntax entry over a fragment.
    /// Module metadata with no authored declaration deliberately returns `None`.
    pub fn lookup(&self, position: SourceMapPosition) -> Option<&SourceMapping> {
        let mut selected: Option<&SourceMapping> = None;
        for candidate in &self.mappings {
            if !candidate.generated.contains(position) {
                continue;
            }
            match selected {
                Some(previous) if candidate.generated.start < previous.generated.start || candidate.generated.end > previous.generated.end => {}
                _ => selected = Some(candidate),
            }
        }
        selected
    }
}

/// Rust text and its provenance are produced together so formatting cannot stale a map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedRust {
    /// Complete Rust bindings, including readable IDL-origin documentation.
    pub source: String,
    /// Ranges referring to exactly this source string, before Rust macro expansion.
    pub source_map: RustSourceMap,
}
