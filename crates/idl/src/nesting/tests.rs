//! Source admission is exercised in bounded children so old aborts cannot kill CI.
use std::path::Path;
#[cfg(not(miri))]
use std::process::Command;
use crate::{ModuleSource, ParseError, parse_module, parse_module_parts};

/// Builds source text, not compiler models that could bypass source admission.
fn vector(depth: usize) -> String {
    format!("struct Deep {{ value: {}u8{} }}", "Vec<".repeat(depth), ">".repeat(depth))
}
/// Requires an owned diagnostic after the syntax worker has returned.
fn source_error(source: &str) -> Box<crate::Diagnostic> {
    let ParseError::Source(error) = parse_module(Path::new("proof/limits.ncidl"), source).unwrap_err() else {
        panic!("expected a source admission error")
    };
    assert_eq!(error.labels[0].span.path(), "proof/limits.ncidl");
    error
}

/// Limits count normalized type height, independently of each sugar spelling.
#[test]
fn normalized_type_height_preserves_boundaries_and_sugar() {
    parse_module(Path::new("proof/limits.ncidl"), &vector(super::TYPE_HEIGHT - 1)).unwrap();
    assert_eq!(source_error(&vector(super::TYPE_HEIGHT)).code, "EIDL068");
    let element = format!("{}u8{}", "Vec<".repeat(super::TYPE_HEIGHT - 2), ">".repeat(super::TYPE_HEIGHT - 2));
    for wrapper in [
        format!("&[{element}]"),
        format!("&Slice<{element}>"),
        format!("&mut [{element}]"),
        format!("&mut Slice::<{element}>"),
    ] {
        let text = format!("interface Probe {{ fn put(&self, value: {wrapper}) -> Result<()>; }}");
        parse_module(Path::new("proof/limits.ncidl"), &text).unwrap();
    }
}

/// Every accepted recursive spelling reaches the same normalized boundary;
/// semantic suitability of a constructor's child belongs to later resolution.
#[test]
fn all_recursive_type_spellings_share_the_inline_height_boundary() {
    for (prefix, suffix) in [
        ("Vec<", ">"),
        ("ComPtr<", ">"),
        ("WeakComPtr<", ">"),
        ("ComBox<", ">"),
        ("WeakComBox<", ">"),
        ("Slice<", ">"),
        ("Array<", ", 1>"),
        ("[", "; 1]"),
        ("[", "]"),
        ("& ", ""),
        ("&[", "]"),
        ("&mut [", "]"),
        ("Named::<", ">"),
    ] {
        let text = |count| format!("struct Deep {{ value: {}u8{} }}", prefix.repeat(count), suffix.repeat(count));
        parse_module(Path::new("proof/limits.ncidl"), &text(super::TYPE_HEIGHT - 1)).unwrap_or_else(|error| panic!("{prefix}: {error}"));
        assert_eq!(source_error(&text(super::TYPE_HEIGHT)).code, "EIDL068", "{prefix}");
    }
}

/// Part ordering, source ownership and Unicode coordinates survive early rejection.
#[test]
fn group_limit_reports_the_actual_part_and_delimiter() {
    let text = format!(
        "// café\n{}u8{}",
        "(".repeat(super::TOKEN_GROUP_DEPTH + 1),
        ")".repeat(super::TOKEN_GROUP_DEPTH + 1)
    );
    let bad = ModuleSource {
        path: Path::new("proof/parts/z_invalid.ncidl"),
        text: &text,
    };
    let good = ModuleSource {
        path: Path::new("proof/parts/a_valid.ncidl"),
        text: "struct Good { value: u8 }",
    };
    for parts in [[good, bad], [bad, good]] {
        let ParseError::Source(error) = parse_module_parts(Path::new("proof/parts"), &parts).unwrap_err() else {
            panic!("source error required")
        };
        assert_eq!(error.code, "EIDL068");
        assert_eq!(error.labels[0].span.path(), "proof/parts/z_invalid.ncidl");
        assert_eq!(error.labels[0].span.module(), "proof.parts");
        assert_eq!(
            error.labels[0].span.start(),
            crate::SourcePosition {
                line: 2,
                column: super::TOKEN_GROUP_DEPTH + 1
            }
        );
    }
}

/// Literal/comment contents are handled by the existing lexer, never delimiter counting.
#[test]
fn comments_and_literal_text_do_not_spend_nesting() {
    let fake = "(<{[!&Vec<".repeat(1024);
    let text = format!(
        "{}hidden{} #[doc = r###\"{fake}\"###] struct Good {{ value: u8 }}",
        "/*".repeat(1024),
        "*/".repeat(1024)
    );
    let module = parse_module(Path::new("proof/limits.ncidl"), &text).unwrap();
    assert_eq!(module.declaration("Good").unwrap().docs, [fake]);
}

/// Each old recursive Syn route must now reject before building its general AST.
#[cfg(not(miri))]
#[test]
fn excessive_and_unsupported_syntax_returns_errors_on_small_callers() {
    const CHILD: &str = "NC_IDL_SYNTAX_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "nesting::tests::excessive_and_unsupported_syntax_returns_errors_on_small_callers",
                "--nocapture",
            ])
            .env(CHILD, "1");
        let output = crate::bounded_child::run(command);
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    #[cfg(target_os = "linux")]
    crate::bounded_child::cap();
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let groups = 16_384;
            for source in [
                vector(256),
                vector(1024),
                format!("struct Deep {{ value: {}u8 }}", "&".repeat(2048)),
                format!("struct Deep {{ value: {}u8 }}", "*const ".repeat(2048)),
                format!("struct Deep {{ value: {}u8 }}", "fn() -> ".repeat(2048)),
                format!("#[doc = {}true] struct Deep {{ value: u8 }}", "!".repeat(4096)),
                format!("#[unknown = {}true] struct Deep {{ value: u8 }}", "!".repeat(4096)),
                format!("enum Code: i32 {{ Bad = {}1 }}", "1 + ".repeat(4096)),
                format!("interface Bad {{ fn read(&self) -> Result<()> {{ {}true; }} }}", "!".repeat(4096)),
                format!("interface Bad {{ fn read(&self, {}value: u8) -> Result<()>; }}", "value @ ".repeat(4096)),
                format!("{}u8{}", "(".repeat(groups), ")".repeat(groups)),
                format!("{}u8", "(".repeat(groups)),
            ] {
                source_error(&source);
            }
            // Wide shallow input is not a nesting failure, and model cleanup occurs
            // on this small caller after the worker returns.
            let fields = (0..4096).map(|index| format!("field_{index}: u8,")).collect::<String>();
            let module = parse_module(Path::new("proof/limits.ncidl"), &format!("struct Wide {{ {fields} }}")).unwrap();
            drop(module);
            parse_module(Path::new("proof/limits.ncidl"), &vector(super::TYPE_HEIGHT - 1)).unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}
