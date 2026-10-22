//! UI event publication is deliberately absent from the IDL language.
use std::path::Path;
use neocogi_idl::parse_module;

/// Neither a concrete nor generic callable declaration can reintroduce UI attributes.
#[test]
fn event_attributes_are_rejected_as_application_policy() {
    for declaration in ["interface Source {}", "interface Source<T> {}", "struct Source { value: u32 }"] {
        for attribute in ["#[events()]", "#[events(Fact)]", "#[events(Fact, Fact)]"] {
            let error = parse_module(Path::new("proof/policy.ncidl"), &format!("#[version(1)] {attribute} {declaration}")).unwrap_err();
            assert!(error.to_string().contains("unsupported attribute"), "{error}");
        }
    }
}
