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
        "#[version(1)] interface IFactory { fn load(&self, bytes: &INcBytes, owner: &IUnknown, kind: Guid) -> Result<IUnknown>; fn save(&self) -> Result<INcBytes>; }"
    ));
    assert!(!resolves("#[version(1)] interface IWrong { fn take(&self, value: IUnknown) -> Result<()>; }"));
    assert!(!resolves("#[version(1)] struct Wrong { value: INcBytes }"));
    assert!(!resolves("#[version(1)] interface IUnknown {}"));
    assert!(!resolves("#[version(1)] interface INcBytes {}"));
}
