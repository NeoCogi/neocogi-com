//! Reproducible built-in persistence bindings without a core/build-tool cycle.
//!
//! Core cannot depend on its own IDL compiler or procedural macros. This test
//! runs the ordinary IDL and interface generators and checks both committed
//! halves. Set `NC_UPDATE_PERSISTENCE_BINDINGS=1` to deliberately regenerate them.

use crate as neocogi_idl;
use std::path::Path;

/// Checks that the built-in contract retains exactly the original IDL identity.
#[test]
fn persistence_bindings_match_idl() {
    let consumer = super::test_consumer::Consumer::new("persistence-binding-proof");
    let source = std::fs::read_to_string(consumer.core.join("schemas/neocogi/com/persistence/objects.ncidl")).unwrap();
    let module = neocogi_idl::parse_module_parts(
        Path::new("neocogi/com/persistence"),
        &[neocogi_idl::ModuleSource {
            path: Path::new("neocogi/com/persistence/objects.ncidl"),
            text: &source,
        }],
    )
    .unwrap();
    let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
    let generated = neocogi_idl::generate::generate_bootstrap(&graph, "neocogi.com.persistence").unwrap();
    // Each actual consumer owns its canonical lexical self-root. Keep generated
    // executable paths and authored literals unchanged; format syntax only.
    let abi = generated.abi;
    let provider = generated.provider;
    check_generated(&consumer.core.join("src/persistence/interfaces.rs"), &abi);
    check_generated(&consumer.facade.join("src/persistence/providers.rs"), &format!("use super::*; {}", provider));
}

/// Formats one generated half and either verifies it or explicitly refreshes it.
fn check_generated(path: &Path, tokens: &str) {
    let file = syn::parse_file(tokens).unwrap();
    let serialization = path.parent().and_then(Path::file_name).is_some_and(|name| name == "serialization");
    let (origin, test) = if serialization {
        ("serialization/formats.ncidl", "serialization_bindings_match_idl")
    } else {
        ("persistence/objects.ncidl", "persistence_bindings_match_idl")
    };
    let origin = format!("core/schemas/neocogi/com/{origin}");
    let expected = format!(
        "// Generated from {origin}.\n\
         // Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-idl {test}\n\n{}",
        prettyplease::unparse(&file),
    );
    if std::env::var_os("NC_UPDATE_PERSISTENCE_BINDINGS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &expected).unwrap();
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), expected, "stale bindings: {}", path.display());
}

/// Serialization protocols have one NCIDL owner and share owned/scoped wire lowering.
#[test]
fn serialization_bindings_match_idl() {
    let consumer = super::test_consumer::Consumer::new("serialization-binding-proof");
    let source = std::fs::read_to_string(consumer.core.join("schemas/neocogi/com/serialization/formats.ncidl")).unwrap();
    let module = crate::parse_module_parts(
        Path::new("neocogi/com/serialization"),
        &[crate::ModuleSource {
            path: Path::new("neocogi/com/serialization/formats.ncidl"),
            text: &source,
        }],
    )
    .unwrap();
    let graph = crate::resolve_modules(vec![module]).unwrap();
    let bindings = crate::generate::generate_bootstrap(&graph, "neocogi.com.serialization").unwrap();
    check_generated(&consumer.core.join("src/serialization/interfaces.rs"), &bindings.abi);
    check_generated(
        &consumer.facade.join("src/serialization/providers.rs"),
        &format!("use super::*; {}", bindings.provider),
    );
    let scoped = crate::generate::generate_scoped_serialization_adapters(&graph, "neocogi.com.serialization").unwrap();
    check_generated(&consumer.core.join("src/serialization/scoped.rs"), &format!("use super::*; {scoped}"));
}

/// Ordinary IDL users can borrow the built-in format interface through generated ABI calls.
#[test]
fn idl_methods_accept_builtin_serializers_from_the_shared_source_contract() {
    let consumer = super::test_consumer::Consumer::new("serialization-interface-consumer").with_serialization();
    let source = std::fs::read_to_string(consumer.core.join("schemas/neocogi/com/serialization/formats.ncidl")).unwrap();
    let format = crate::parse_module_parts(
        Path::new("neocogi/com/serialization"),
        &[crate::ModuleSource {
            path: Path::new("neocogi/com/serialization/formats.ncidl"),
            text: &source,
        }],
    )
    .unwrap();
    let user = crate::parse_module(
        Path::new("proof/serialization.ncidl"),
        r#"
        extern neocogi.com.serialization;
        use neocogi.com.serialization::ISerializer;
        interface Emitter { fn emit(&self, output: &ISerializer) -> Result<()>; }
    "#,
    )
    .unwrap();
    let graph = crate::resolve_modules(vec![format, user]).unwrap();
    let mut paths = crate::generate::RustModulePaths::default();
    paths.insert("neocogi.com.serialization", "neocogi_com::serialization").unwrap();
    let bindings = crate::generate::generate_rust_with_paths(&graph, "proof.serialization", &paths).unwrap();
    let source = format!(
        r#"
        {bindings}
        #[neocogi_com::com_object(Emitter)] struct Object;
        impl Emitter for Object {{
            fn emit(&self,output:neocogi_com::ComRef<'_,ISerializerAbi>)->Result<(),neocogi_com::NcStatus>{{
                neocogi_com::serialization::Serializer::new(output).serialize(&42u32)
            }}
        }}
        #[test] fn real_format_call(){{
            use neocogi_com_serialization::ncb::NcbSerializer;
            use neocogi_com::serialization::{{ISerializationOutputAbi,Limits}};
            let object=Object::from_value(Object).query::<EmitterAbi>().unwrap();
            let writer=NcbSerializer::create(Limits::default()).unwrap();
            object.emit(writer.as_com_ref()).unwrap();
            let output=writer.query_ref::<ISerializationOutputAbi>().unwrap();
            let mut bytes=neocogi_com::NcOutputBuffer::new();
            output.copy_to(bytes.output(0,output.byte_len().unwrap() as usize)).unwrap();
            assert_eq!(neocogi_com_serialization::ncb::from_slice::<u32>(bytes.as_slice()).unwrap(),42);
        }}
    "#
    );
    super::name_ownership_tests::run_consumers_with_files(consumer, [("builtin COM parameter".into(), source)], &[], &["real_format_call"]);
}
