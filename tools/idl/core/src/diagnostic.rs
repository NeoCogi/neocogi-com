//! Structured, source-aware diagnostics shared by every IDL compiler phase.

use std::{cmp::Ordering, collections::BTreeMap, fmt, path::Path, sync::Arc};

use proc_macro2::Span;

/// Immutable IDL source retained so diagnostics can render exact snippets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    /// Portable path relative to the configured IDL root.
    path: String,
    /// Complete UTF-8 source shared cheaply by every diagnostic label.
    text: Arc<str>,
}

impl SourceFile {
    /// Creates one diagnostic source from its relative path and exact contents.
    pub fn new(path: &Path, text: &str) -> Self {
        Self {
            path: path.to_string_lossy().replace('\\', "/"),
            text: Arc::from(text),
        }
    }

    /// Returns the normalized source path used in diagnostics.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the exact source text supplied to the parser.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns one one-based source line without its trailing newline.
    fn line(&self, number: usize) -> Option<&str> {
        self.text.lines().nth(number.checked_sub(1)?)
    }
}

/// One-based line and column within an IDL source file.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourcePosition {
    /// One-based line number.
    pub line: usize,
    /// One-based Unicode-scalar column number.
    pub column: usize,
}

/// Exact source range attached to an IDL syntax-model element.
#[derive(Clone)]
pub struct SourceSpan {
    /// Normalized source path, attached after a syntax part is parsed.
    path: String,
    /// Inclusive beginning of the range.
    start: SourcePosition,
    /// Exclusive end of the range.
    end: SourcePosition,
    /// Original parser span retained only while producing parser diagnostics.
    raw: Span,
}

impl SourceSpan {
    /// Converts a parser span into stable one-based source coordinates.
    pub(crate) fn from_parser(span: Span) -> Self {
        let start = span.start();
        let end = span.end();
        Self {
            path: String::new(),
            start: SourcePosition {
                line: start.line,
                column: start.column + 1,
            },
            end: SourcePosition { line: end.line, column: end.column + 1 },
            raw: span,
        }
    }

    /// Attaches the normalized path of the part that owns this range.
    pub(crate) fn attach(&mut self, source: &SourceFile) {
        self.path.clone_from(&source.path);
    }

    /// Returns the parser span for composing a multi-label syntax error.
    pub(crate) fn parser_span(&self) -> Span {
        self.raw
    }

    /// Returns the normalized path of the owning IDL source part.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the inclusive beginning of this range.
    pub fn start(&self) -> SourcePosition {
        self.start
    }

    /// Returns the exclusive end of this range.
    pub fn end(&self) -> SourcePosition {
        self.end
    }
}

impl fmt::Debug for SourceSpan {
    /// Omits the process-local parser span while showing stable coordinates.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceSpan")
            .field("path", &self.path)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

impl PartialEq for SourceSpan {
    /// Compares only portable source identity and coordinates.
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.start == other.start && self.end == other.end
    }
}

impl Eq for SourceSpan {}

impl Ord for SourceSpan {
    /// Orders ranges independently of filesystem discovery order.
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.path, self.start, self.end).cmp(&(&other.path, other.start, other.end))
    }
}

impl PartialOrd for SourceSpan {
    /// Delegates to the total portable source ordering.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Visual role of one source label in a compiler diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelStyle {
    /// Location at which the compiler detected the error.
    Primary,
    /// Related definition or earlier conflicting location.
    Secondary,
}

/// One annotated source range in a structured compiler diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticLabel {
    /// Exact source range being annotated.
    pub span: SourceSpan,
    /// Short explanation of the range's role in the error.
    pub message: String,
    /// Primary or supporting visual role.
    pub style: LabelStyle,
}

/// One stable source-aware IDL compiler diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Stable machine-readable error code.
    pub code: String,
    /// Concise description of the violated language rule.
    pub message: String,
    /// Source annotations, with exactly one primary label when source exists.
    pub labels: Vec<DiagnosticLabel>,
    /// Explanatory notes and actionable help shown after source annotations.
    pub notes: Vec<String>,
    /// Source contents indexed by normalized path for deterministic rendering.
    sources: BTreeMap<String, SourceFile>,
}

impl Diagnostic {
    /// Creates one diagnostic without a source location.
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            sources: BTreeMap::new(),
        }
    }

    /// Adds one source file used by this diagnostic's labels.
    pub(crate) fn with_source(mut self, source: &SourceFile) -> Self {
        self.sources.insert(source.path.clone(), source.clone());
        self
    }

    /// Adds all sources from one parsed module.
    pub(crate) fn with_sources(mut self, sources: &[SourceFile]) -> Self {
        for source in sources {
            self.sources.insert(source.path.clone(), source.clone());
        }
        self
    }

    /// Adds the primary source range at which the error was detected.
    pub(crate) fn primary(mut self, span: &SourceSpan, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel {
            span: span.clone(),
            message: message.into(),
            style: LabelStyle::Primary,
        });
        self
    }

    /// Adds a related definition or earlier conflicting source range.
    pub(crate) fn secondary(mut self, span: &SourceSpan, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel {
            span: span.clone(),
            message: message.into(),
            style: LabelStyle::Secondary,
        });
        self
    }

    /// Adds an explanatory note or concrete correction hint.
    pub(crate) fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Builds a parser diagnostic from every span carried by one Syn error.
    pub(crate) fn from_syn(source: &SourceFile, error: syn::Error) -> Self {
        let mut errors = error.into_iter();
        let Some(primary) = errors.next() else {
            return Self::new("EIDL000", "unknown IDL syntax error").with_source(source);
        };
        let message = primary.to_string();
        let mut diagnostic = Self::new("EIDL001", &message)
            .with_source(source)
            .primary(&span_in_source(primary.span(), source), message);
        for related in errors {
            let message = related.to_string();
            diagnostic = diagnostic.secondary(&span_in_source(related.span(), source), message);
        }
        diagnostic
    }
}

impl fmt::Display for Diagnostic {
    /// Renders stable compiler-style locations, snippets, labels, and notes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "error[{}]: {}", self.code, self.message)?;
        let mut labels = self.labels.iter().collect::<Vec<_>>();
        labels.sort_by(|left, right| {
            let style = match (left.style, right.style) {
                (LabelStyle::Primary, LabelStyle::Secondary) => Ordering::Less,
                (LabelStyle::Secondary, LabelStyle::Primary) => Ordering::Greater,
                _ => Ordering::Equal,
            };
            style.then_with(|| left.span.cmp(&right.span))
        });
        for label in labels {
            let position = label.span.start;
            writeln!(formatter, " --> {}:{}:{}", label.span.path, position.line, position.column)?;
            if let Some(line) = self.sources.get(&label.span.path).and_then(|source| source.line(position.line)) {
                let gutter = position.line.to_string().len();
                writeln!(formatter, "{:gutter$} |", "", gutter = gutter)?;
                writeln!(formatter, "{} | {line}", position.line)?;
                let width = if label.span.end.line == position.line {
                    label.span.end.column.saturating_sub(position.column).max(1)
                } else {
                    1
                };
                let marker = match label.style {
                    LabelStyle::Primary => '^',
                    LabelStyle::Secondary => '-',
                };
                writeln!(
                    formatter,
                    "{:gutter$} | {}{} {}",
                    "",
                    " ".repeat(position.column.saturating_sub(1)),
                    marker.to_string().repeat(width),
                    label.message,
                    gutter = gutter
                )?;
            }
        }
        for note in &self.notes {
            writeln!(formatter, " = help: {note}")?;
        }
        Ok(())
    }
}

/// Converts a parser span and known source into an attached portable range.
fn span_in_source(span: Span, source: &SourceFile) -> SourceSpan {
    let mut result = SourceSpan::from_parser(span);
    result.attach(source);
    result
}
