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
        "#[version(1)] interface IFactory { fn load(&self, bytes: &INcBytes, owner: &IUnknown, kind: Guid) -> Result<ComPtr<IUnknown>>; fn save(&self) -> Result<ComPtr<INcBytes>>; }"
    ));
    assert!(!resolves("#[version(1)] interface IWrong { fn take(&self, value: IUnknown) -> Result<()>; }"));
    assert!(resolves("#[version(1)] struct Owned { value: ComPtr<INcBytes> }"));
    assert!(!resolves("#[version(1)] interface IUnknown {}"));
    assert!(!resolves("#[version(1)] interface INcBytes {}"));
}

/// Ownership is explicit at every field depth and at the successful result slot.
#[test]
fn pointer_constructor_requires_a_closed_object_contract() {
    let prefix = "#[version(1)] interface IToken {} #[version(1)] struct Value { count: u32 } ";
    for body in [
        "#[version(1)] struct Snapshot { token: IToken }",
        "#[version(1)] struct Snapshot { tokens: &[IToken] }",
        "#[version(1)] struct Snapshot { token: Option<IToken> }",
        "#[version(1)] struct Snapshot { token: ComPtr<Value> }",
        "#[version(1)] struct Snapshot { token: ComPtr<u32> }",
        "#[version(1)] struct Snapshot { token: ComPtr<ComPtr<IToken>> }",
        "#[version(1)] interface IFactory { fn create(&self) -> Result<IToken>; }",
        "#[version(1)] interface IFactory { fn label(&self) -> Result<String>; }",
        "#[version(1)] interface IFactory { fn take(&self, token: ComPtr<IToken>) -> Result<()>; }",
    ] {
        assert!(!resolves(&format!("{prefix}{body}")), "must reject {body}");
    }
    assert!(resolves(&format!(
        "{prefix} #[version(1)] struct Snapshot {{ tokens: &[ComPtr<IToken>], current: Option<ComPtr<IToken>> }}"
    )));
    assert!(resolves("#[version(1)] interface IFactory { fn label(&self) -> Result<ComPtr<INcString>>; }"));
    for name in ["ComPtr", "INcString"] {
        assert!(!resolves(&format!("#[version(1)] struct {name} {{ count: u32 }}")));
    }
}
