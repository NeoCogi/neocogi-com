//! Borrowed structural spelling for retained closed applications. This renderer
//! never changes lexical owners or Rust projections, nor constructs a display AST.

use std::fmt::{self, Write};
use crate::{AbiType, NamedType, ResolvedModules, Specialization};

/// Diagnostic names have a finite payload; generated documentation retains the
/// complete already-admitted spelling. Truncation never splits a UTF-8 scalar.
struct Text {
    /// Output owned by this rendering call only.
    value: String,
    /// Optional byte ceiling including the explicit truncation marker.
    limit: Option<usize>,
}
impl Write for Text {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if let Some(limit) = self.limit {
            let remaining = limit.saturating_sub(self.value.len());
            if text.len() > remaining {
                let mut end = remaining;
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                self.value.push_str(&text[..end]);
                return Err(fmt::Error);
            }
        }
        self.value.push_str(text);
        Ok(())
    }
}

/// Renders a borrowed template plus its retained ordered arguments. Qualified
/// documentation uses bound owners; diagnostics preserve authored source paths.
pub(crate) fn application(value: &Specialization, qualified: Option<&ResolvedModules>, diagnostic: bool) -> String {
    let mut text = Text {
        value: String::new(),
        limit: diagnostic.then_some(4093),
    };
    if named(&mut text, &value.template, &value.arguments, qualified).is_err() {
        text.value.push_str("...");
    }
    text.value
}

/// Writes segments without allocating a temporary joined path.
fn segments(text: &mut Text, values: &[String]) -> fmt::Result {
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            text.write_str("::")?;
        }
        text.write_str(value)?;
    }
    Ok(())
}

/// Only display qualification is derived here; canonical and supplying binding
/// keys remain unchanged and nominal declarations are never unfolded.
fn named(text: &mut Text, name: &NamedType, arguments: &[AbiType], graph: Option<&ResolvedModules>) -> fmt::Result {
    if let Some((graph, key)) = graph.zip(name.binding.declaration()) {
        segments(text, graph.modules()[key.module].source.name.segments())?;
        text.write_str("::")?;
        text.write_str(&key.name)?;
    } else {
        segments(text, &name.segments)?;
    }
    if !arguments.is_empty() {
        text.write_str("<")?;
        for (index, argument) in arguments.iter().enumerate() {
            if index != 0 {
                text.write_str(", ")?;
            }
            ty(text, argument, graph)?;
        }
        text.write_str(">")?;
    }
    Ok(())
}

/// Exhaustive structural spelling over finite admitted expressions only.
fn ty(text: &mut Text, value: &AbiType, graph: Option<&ResolvedModules>) -> fmt::Result {
    let (prefix, child, suffix) = match value {
        AbiType::Unit => return text.write_str("()"),
        AbiType::Scalar(name) => return text.write_str(name),
        AbiType::String => return text.write_str("String"),
        AbiType::Named(name) => return named(text, name, &name.arguments, graph),
        AbiType::ComPtr(child) => ("ComPtr<", child, ">"),
        AbiType::WeakComPtr(child) => ("WeakComPtr<", child, ">"),
        AbiType::ComBox(child) => ("ComBox<", child, ">"),
        AbiType::WeakComBox(child) => ("WeakComBox<", child, ">"),
        AbiType::Borrowed(child) => ("&", child, ""),
        AbiType::Vector(child) => ("Vec<", child, ">"),
        AbiType::UnsizedSlice(child) | AbiType::Slice(child) => ("&[", child, "]"),
        AbiType::OutputSlice(child) => ("&mut [", child, "]"),
        AbiType::Array { element, len } => {
            text.write_str("[")?;
            ty(text, element, graph)?;
            return write!(text, "; {len}]");
        }
    };
    // Unsized referents have no borrow prefix in authored spelling.
    if matches!(value, AbiType::UnsizedSlice(_)) {
        text.write_str("[")?;
    } else {
        text.write_str(prefix)?;
    }
    ty(text, child, graph)?;
    text.write_str(suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A truncated UTF-8 label stays bounded and explicitly marked; no type AST
    /// allocation or complete intermediate string is needed before truncation.
    #[test]
    fn diagnostic_application_truncates_on_utf8_boundary() {
        let mut text = Text { value: String::new(), limit: Some(4093) };
        assert!(text.write_str(&"界".repeat(2000)).is_err());
        text.value.push_str("...");
        assert!(text.value.len() <= 4096);
        assert!(text.value.ends_with("..."));
    }
}

#[cfg(test)]
mod spelling_tests {
    use super::*;

    /// Every constructor keeps the authoritative owned-model spelling when
    /// rendered through a borrowed application. No semantic AST is rewritten.
    #[test]
    fn borrowed_application_constructor_spellings_match_model() {
        for argument in [
            "()",
            "u8",
            "String",
            "IUnknown",
            "ComPtr<IUnknown>",
            "WeakComPtr<IUnknown>",
            "ComBox<IUnknown>",
            "WeakComBox<IUnknown>",
            "&u8",
            "Vec<u8>",
            "[u8]",
            "&[u8]",
            "&mut [u8]",
            "[u8; 4]",
        ] {
            let source = format!("#[version(1)] struct Marker<T> {{ value: u8 }} #[version(1)] struct Root {{ value: Marker<{argument}> }}");
            let module = crate::parse_module(std::path::Path::new("proof/spelling.ncidl"), &source).unwrap();
            let crate::DeclarationKind::Struct(record) = &module.declaration("Root").unwrap().kind else {
                panic!("record")
            };
            let AbiType::Named(mut name) = record.fields[0].ty.clone() else {
                panic!("application")
            };
            let expected = name.rust_path();
            let arguments = std::mem::take(&mut name.arguments);
            assert_eq!(application(&Specialization { template: name, arguments }, None, false), expected, "{argument}");
        }
    }
}
