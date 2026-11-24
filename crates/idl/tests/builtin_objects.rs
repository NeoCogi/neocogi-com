//! Core object capabilities retain explicit ownership and exact signatures.
use std::path::Path;
use neocogi_idl::{parse_module, resolve_modules};

/// Parses and resolves a standalone contract without synthetic core imports.
fn resolves(source: &str) -> bool {
    let module = parse_module(Path::new("test/builtins.ncidl"), source).unwrap();
    resolve_modules(vec![module]).is_ok()
}

/// Core identity, bytes, and GUIDs work only in their intended ABI positions.
#[test]
fn builtins_accept_explicit_object_ownership() {
    assert!(resolves(
        "interface IFactory { fn load(&self, bytes: &INcBytes, owner: &IUnknown, kind: Guid) -> Result<ComPtr<IUnknown>>; fn save(&self) -> Result<ComPtr<INcBytes>>; }"
    ));
    assert!(resolves("struct Owned { value: ComPtr<INcBytes> }"));
    assert!(!resolves("interface IUnknown {}"));
    assert!(!resolves("interface INcBytes {}"));
}

/// Built-in object names are bound intrinsics, so report their missing borrow
/// directly at the use.
#[test]
fn bare_builtin_object_inputs_point_to_the_missing_borrow() {
    for name in ["IUnknown", "INcBytes", "INcString"] {
        let source = format!("interface IWrong {{ fn take(&self, value: {name}) -> Result<()>; }}");
        let module = parse_module(Path::new("test/builtins.ncidl"), &source).unwrap();
        let neocogi_idl::ModuleGraphError::Resolution(error) = resolve_modules(vec![module]).unwrap_err() else {
            panic!("expected a type resolution error");
        };
        let [diagnostic] = error.diagnostics.as_slice() else {
            panic!("expected one diagnostic for {name}");
        };
        assert_eq!(diagnostic.code, "EIDL060");
        assert!(diagnostic.message.contains("must borrow interface values with &"));
        let [label] = diagnostic.labels.as_slice() else {
            panic!("a built-in interface has no authored declaration");
        };
        let start = label.span.start().column - 1;
        let end = label.span.end().column - 1;
        assert_eq!(&source[start..end], name);
        assert!(resolves(&format!("interface IValid {{ fn take(&self, value: &{name}) -> Result<()>; }}")));
    }
}

/// Ownership is explicit at every field depth and at the successful result slot.
#[test]
fn pointer_constructor_requires_a_closed_object_contract() {
    let prefix = "interface IToken {} struct Value { count: u32 } ";
    for body in [
        "struct Snapshot { token: IToken }",
        "struct Snapshot { tokens: Vec<IToken> }",
        "struct Snapshot { token: Option<IToken> }",
        "struct Snapshot { token: ComPtr<Value> }",
        "struct Snapshot { token: ComPtr<u32> }",
        "struct Snapshot { token: ComPtr<ComPtr<IToken>> }",
        "interface IFactory { fn create(&self) -> Result<IToken>; }",
        "interface IFactory { fn label(&self) -> Result<String>; }",
        "interface IFactory { fn take(&self, token: ComPtr<IToken>) -> Result<()>; }",
    ] {
        assert!(!resolves(&format!("{prefix}{body}")), "must reject {body}");
    }
    assert!(resolves(&format!(
        "{prefix} struct Snapshot {{ tokens: Vec<ComPtr<IToken>>, current: Option<ComPtr<IToken>> }}"
    )));
    assert!(resolves("interface IFactory { fn label(&self) -> Result<ComPtr<INcString>>; }"));
    for name in ["ComPtr", "INcString"] {
        assert!(!resolves(&format!("struct {name} {{ count: u32 }}")));
    }
}
