//! Structured, source-aware diagnostics shared by every IDL compiler phase.

use std::{
    cmp::Ordering,
    fmt,
    path::{Component, Path},
    sync::Arc,
};

use proc_macro2::Span;

/// Immutable IDL source retained so diagnostics can render exact snippets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile(Arc<SourceContents>);

/// Immutable source identity and text shared by its diagnostic labels.
#[derive(Debug, Eq, PartialEq)]
struct SourceContents {
    /// Exact logical module owning this file, independent of its filesystem root.
    module: String,
    /// Portable path relative to the configured IDL root.
    path: String,
    /// Complete UTF-8 source owned by the shared record.
    text: Box<str>,
}

impl SourceFile {
    /// Creates a diagnostic source with its owner, relative path and text.
    /// Rejects absolute, empty, non-UTF-8 or traversal paths before constructing
    /// a provenance key. Separators are normalized using native components.
    pub fn new(module: &crate::ModuleName, path: &Path, text: &str) -> Result<Self, crate::ModulePathError> {
        if path.is_absolute() {
            return Err(crate::ModulePathError::Absolute);
        }
        if !path.components().all(|part| matches!(part, Component::Normal(_))) {
            return Err(crate::ModulePathError::NonCanonical);
        }
        Ok(Self::from_path_key(module, crate::path::relative_path_text(path)?, text))
    }

    /// Installs the already admitted path key used to order parser inputs.
    pub(crate) fn from_path_key(module: &crate::ModuleName, path: String, text: &str) -> Self {
        Self(Arc::new(SourceContents {
            module: module.to_string(),
            path,
            text: text.into(),
        }))
    }

    /// Returns the exact logical module owning this source file.
    pub fn module(&self) -> &str {
        &self.0.module
    }

    /// Returns the normalized source path used in diagnostics.
    pub fn path(&self) -> &str {
        &self.0.path
    }

    /// Returns the exact source text supplied to the parser.
    pub fn text(&self) -> &str {
        &self.0.text
    }

    /// Returns one one-based source line without its trailing newline.
    fn line(&self, number: usize) -> Option<&str> {
        self.text().lines().nth(number.checked_sub(1)?)
    }
}

/// One-based line and column within an IDL source file.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourcePosition {
    /// One-based line number.
    pub line: usize,
    /// One-based Unicode scalar column number, matching the parser.
    pub column: usize,
}

/// Exact, owned source range attached to an IDL syntax-model element.
/// Parser-local spans are reduced to these coordinates before a worker returns.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SourceSpan {
    /// Logical source owner, attached with the part path before semantic resolution.
    module: String,
    /// Normalized source path, attached after a syntax part is parsed.
    path: String,
    /// Inclusive beginning of the range.
    start: SourcePosition,
    /// Exclusive end of the range.
    end: SourcePosition,
}

impl SourceSpan {
    /// Exhaustive owned-payload view for generated-copy accounting. Adding span
    /// metadata must update this list instead of silently bypassing the budget.
    pub(crate) fn owned_strings(&self) -> [&str; 2] {
        let Self { module, path, start: _, end: _ } = self;
        [module, path]
    }

    /// Allocation-free vacancy used only while moving a named type through a visitor.
    pub(crate) fn placeholder() -> Self {
        Self {
            module: String::new(),
            path: String::new(),
            start: SourcePosition { line: 0, column: 0 },
            end: SourcePosition { line: 0, column: 0 },
        }
    }

    /// Converts a parser span into stable one-based source coordinates.
    pub(crate) fn from_parser(span: Span) -> Self {
        let start = span.start();
        let end = span.end();
        Self {
            module: String::new(),
            path: String::new(),
            start: SourcePosition {
                line: start.line,
                column: start.column + 1,
            },
            end: SourcePosition { line: end.line, column: end.column + 1 },
        }
    }

    /// Attaches the normalized path of the part that owns this range.
    pub(crate) fn attach(&mut self, source: &SourceFile) {
        source.module().clone_into(&mut self.module);
        source.path().clone_into(&mut self.path);
    }

    /// Returns the logical module that authored this range, including template origins.
    pub fn module(&self) -> &str {
        &self.module
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

/// Visual role of one source label in a compiler diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelStyle {
    /// Location at which the compiler detected the diagnostic.
    Primary,
    /// Related definition or earlier conflicting location.
    Secondary,
}

/// One annotated source range in a structured compiler diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticLabel {
    /// Exact source range being annotated.
    pub span: SourceSpan,
    /// Short explanation of the range's role in the diagnostic.
    pub message: String,
    /// Primary or supporting visual role.
    pub style: LabelStyle,
    /// Exact source retained by this label, even if two inputs share a path.
    source: Option<SourceFile>,
}

impl DiagnosticLabel {
    /// Creates a label from a span, explanation and display style.
    pub fn new(span: SourceSpan, message: impl Into<String>, style: LabelStyle) -> Self {
        Self {
            span,
            message: message.into(),
            style,
            source: None,
        }
    }

    /// Returns its retained file when it still matches the label's public span.
    pub fn source(&self) -> Option<&SourceFile> {
        self.source
            .as_ref()
            .filter(|source| source.module() == self.span.module() && source.path() == self.span.path())
    }
}

/// One stable source-aware IDL compiler diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Whether the diagnostic rejects compilation or reports an accepted construct.
    pub severity: DiagnosticSeverity,
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Concise description of the violated rule or accepted deprecated syntax.
    pub message: String,
    /// Source annotations, with exactly one primary label when source exists.
    pub labels: Vec<DiagnosticLabel>,
    /// Explanatory notes and actionable help shown after source annotations.
    pub notes: Vec<String>,
}

/// Whether a compiler diagnostic rejects input or accompanies successful parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    /// A violated contract or syntax rule that prevents compilation.
    Error,
    /// Accepted source that should be migrated, without affecting ABI identity.
    Warning,
}

impl Diagnostic {
    /// Creates one diagnostic without a source location.
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            code: code.to_owned(),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Creates a nonfatal diagnostic retained separately from contract data.
    pub(crate) fn warning(code: &str, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            ..Self::new(code, message)
        }
    }

    /// Attaches the parsed part to labels before owned diagnostics leave the worker.
    pub(crate) fn attach_source(&mut self, source: &SourceFile) {
        for label in &mut self.labels {
            label.span.attach(source);
            label.source = Some(source.clone());
        }
    }

    /// Retains only labelled files. Directly attached provenance wins over an
    /// index lookup, including for different inputs claiming the same name/path.
    pub(crate) fn attach_label_sources<'a>(&mut self, mut find: impl FnMut(&DiagnosticLabel) -> Option<&'a SourceFile>) {
        for label in &mut self.labels {
            if label.source.is_none() {
                label.source = find(label).cloned();
            }
        }
    }

    /// Adds the primary source range at which the error was detected.
    pub(crate) fn primary(mut self, span: &SourceSpan, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel::new(span.clone(), message, LabelStyle::Primary));
        self
    }

    /// Adds a related definition or earlier conflicting source range.
    pub(crate) fn secondary(mut self, span: &SourceSpan, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel::new(span.clone(), message, LabelStyle::Secondary));
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
            return Self::new("EIDL000", "unknown IDL syntax error");
        };
        let encoded_message = primary.to_string();
        let (code, message) = split_encoded_code(&encoded_message);
        let mut diagnostic = Self::new(code, message).primary(&span_in_source(primary.span(), source), message);
        for related in errors {
            let message = related.to_string();
            diagnostic = diagnostic.secondary(&span_in_source(related.span(), source), message);
        }
        diagnostic.attach_label_sources(|_| Some(source));
        diagnostic
    }

    /// Compares diagnostics by primary location, code, and message.
    pub(crate) fn stable_cmp(&self, other: &Self) -> Ordering {
        let own_span = self.labels.iter().find(|label| label.style == LabelStyle::Primary).map(|label| &label.span);
        let other_span = other.labels.iter().find(|label| label.style == LabelStyle::Primary).map(|label| &label.span);
        own_span
            .cmp(&other_span)
            .then_with(|| self.code.cmp(&other.code))
            .then_with(|| self.message.cmp(&other.message))
    }
}

/// Extracts a parser-internal diagnostic code prefix without exposing it.
fn split_encoded_code(message: &str) -> (&str, &str) {
    let Some(encoded) = message.strip_prefix('[') else {
        return ("EIDL001", message);
    };
    let Some((code, message)) = encoded.split_once("] ") else {
        return ("EIDL001", message);
    };
    if code.len() > 4 && code.starts_with("EIDL") && code[4..].bytes().all(|byte| byte.is_ascii_digit()) {
        (code, message)
    } else {
        ("EIDL001", message)
    }
}

impl fmt::Display for Diagnostic {
    /// Renders stable compiler-style locations, snippets, labels, and notes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
        };
        writeln!(formatter, "{severity}[{}]: {}", self.code, self.message)?;
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
            if let Some(line) = label.source().and_then(|source| source.line(position.line)) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ModuleGraphError, ModuleSource, parse_module, parse_module_parts, resolve_modules};

    /// The retained label holds only its own file after all other errors are dropped.
    #[test]
    fn diagnostics_retain_only_labelled_inputs() {
        let paths = (0..48).map(|i| format!("proof/many/part{i:02}.ncidl")).collect::<Vec<_>>();
        let texts = (0..48).map(|i| format!("struct Value{i:02} {{ field: Missing }}")).collect::<Vec<_>>();
        let parts = paths
            .iter()
            .zip(&texts)
            .map(|(path, text)| ModuleSource { path: Path::new(path), text })
            .collect::<Vec<_>>();
        let module = parse_module_parts(Path::new("proof/many"), &parts).unwrap();
        let retained = module.sources.iter().map(|source| Arc::downgrade(&source.0)).collect::<Vec<_>>();
        let ModuleGraphError::Resolution(error) = resolve_modules(vec![module]).unwrap_err() else {
            panic!("expected semantic errors");
        };
        assert_eq!(error.diagnostics.len(), parts.len());
        for (index, diagnostic) in error.diagnostics.iter().enumerate() {
            assert_eq!(diagnostic.code, "EIDL070");
            assert_eq!(diagnostic.labels.len(), 1);
            let source = diagnostic.labels[0].source().unwrap();
            assert_eq!(source.path(), paths[index]);
            assert_eq!(source.text(), texts[index]);
            assert_eq!(retained[index].strong_count(), 1);
        }
        let first = error.diagnostics.into_iter().next().unwrap();
        assert!(first.to_string().contains(&texts[0]));
        assert_eq!(retained[0].strong_count(), 1);
        assert!(retained[1..].iter().all(|source| source.strong_count() == 0));
    }

    /// The two source versions can share both logical identity and relative path.
    #[test]
    fn duplicate_modules_render_both_exact_source_versions() {
        let path = Path::new("proof/duplicate.ncidl");
        let first = parse_module(path, "struct Alpha { value: u32 }").unwrap();
        let second = parse_module(path, "struct Beta { value: u64 }").unwrap();
        let mut output = None;
        for input in [vec![first.clone(), second.clone()], vec![second, first]] {
            let ModuleGraphError::Resolution(error) = resolve_modules(input).unwrap_err() else {
                panic!("expected duplicate module");
            };
            let diagnostic = &error.diagnostics[0];
            assert_eq!(diagnostic.code, "EIDL011");
            assert!(diagnostic.labels[0].source().unwrap().text().contains("struct Beta"));
            assert!(diagnostic.labels[1].source().unwrap().text().contains("struct Alpha"));
            let text = diagnostic.to_string();
            assert!(text.contains("struct Alpha") && text.contains("struct Beta"));
            if let Some(prior) = output.replace(text.clone()) {
                assert_eq!(prior, text);
            }
        }
    }
}
