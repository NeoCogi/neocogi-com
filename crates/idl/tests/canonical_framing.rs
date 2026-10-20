//! Formerly equal byte streams must distinguish genuinely different generated wires.
use std::{fs, path::Path, process::Command};
use neocogi_idl::{ResolvedModules, parse_module, resolve_modules};

/// Independently resolves one accepted ordering of phantom template arguments.
/// Output and unit arguments are unused by Phantom except for its last field.
fn graph(arguments: &str) -> ResolvedModules {
    let source = format!(
        r#"
        #[version(1)] struct Phantom<A,B,C,D,E> {{ data:E }}
        #[version(1)] struct Shell<T> {{ inner:Phantom<{arguments}> }}
        #[version(1)] interface Exchange<T> {{ fn accept(&self,value:&Shell<T>)->Result<u32>; }}
    "#
    );
    resolve_modules(vec![
        parse_module(Path::new("proof/generic.ncidl"), &source).unwrap(),
        parse_module(
            Path::new("proof/consumer.ncidl"),
            r#"
            extern proof.generic;
            #[version(1)] interface Root:proof::generic::Exchange<u32> {}
        "#,
        )
        .unwrap(),
    ])
    .unwrap()
}

/// Returns a generated projection by its retained template key, never its UUID suffix.
fn projection(graph: &ResolvedModules, template: &str) -> String {
    graph
        .module("proof.consumer")
        .unwrap()
        .source
        .declarations
        .iter()
        .find(|declaration| {
            declaration
                .specialization
                .as_ref()
                .is_some_and(|origin| origin.template.segments.last().unwrap() == template)
        })
        .unwrap()
        .name
        .clone()
}

/// Both differing peers still compile and make their own valid ABI calls. Their
/// equal nominal IID now has distinct signatures, so cross-query is rejected.
#[test]
fn parameter_and_output_expressions_cannot_merge_with_following_siblings() {
    let variants = [
        ("small", "T, &mut [()], (), (), ()"),
        ("large", "&mut [()], (), (), (), T"),
        ("control", "&mut [u8], (), (), (), T"),
    ];
    let graphs = variants.iter().map(|(_, arguments)| graph(arguments)).collect::<Vec<_>>();
    for left in 0..graphs.len() {
        for right in left + 1..graphs.len() {
            assert_ne!(
                graphs[left].module("proof.generic").unwrap().abi_fingerprint,
                graphs[right].module("proof.generic").unwrap().abi_fingerprint
            );
            let l = graphs[left]
                .module("proof.consumer")
                .unwrap()
                .declaration(&projection(&graphs[left], "Exchange"))
                .unwrap();
            let r = graphs[right]
                .module("proof.consumer")
                .unwrap()
                .declaration(&projection(&graphs[right], "Exchange"))
                .unwrap();
            assert_eq!(l.id, r.id);
            assert_ne!(l.abi_signature, r.abi_signature);
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            "[package]\nname='canonical-framing-proof'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nneocogi-com={{path={:?}}}\n",
            root.join("crates/com/api")
        ),
    )
    .unwrap();
    for ((side, _), graph) in variants.iter().zip(&graphs) {
        let mut code = neocogi_idl::generate_rust(graph, "proof.consumer").unwrap();
        for template in ["Exchange", "Shell", "Phantom"] {
            let name = projection(graph, template);
            code.push_str(&format!("\npub use {name} as {template};\n"));
            if template == "Exchange" {
                code.push_str(&format!("pub use {name}Abi as ExchangeAbi; pub use {name}VtblFor as ExchangeVtblFor;\n"));
            }
        }
        fs::write(project.path().join(format!("src/{side}.rs")), code).unwrap();
    }
    fs::write(
        project.path().join("src/lib.rs"),
        r#"
        pub mod small; pub mod large; pub mod control;
        use neocogi_com::{ComInterface,ComValue,NcValueRef,NcStatus,NC_E_ABI_VERSION};
        #[neocogi_com::com_object(small::Exchange)] struct Small;
        impl small::Exchange for Small {
            fn accept(&self,value:NcValueRef<'_,small::Shell>)->Result<u32,NcStatus> {
                let _:()=value.to_owned()?.inner.data; Ok(17)
            }
        }
        #[neocogi_com::com_object(large::Exchange)] struct Large;
        impl large::Exchange for Large {
            fn accept(&self,value:NcValueRef<'_,large::Shell>)->Result<u32,NcStatus> { Ok(value.to_owned()?.inner.data) }
        }
        #[test] fn generated_wire_and_negotiation_proof() {
            assert_eq!(small::ExchangeAbi::INTERFACE_ID.iid,large::ExchangeAbi::INTERFACE_ID.iid);
            assert_ne!(small::ExchangeAbi::INTERFACE_ID,large::ExchangeAbi::INTERFACE_ID);
            assert_eq!(core::mem::size_of::<<small::Shell as ComValue>::Wire>(),1);
            assert_eq!(core::mem::size_of::<<large::Shell as ComValue>::Wire>(),4);
            let small=Small::from_value(Small).query::<small::ExchangeAbi>().unwrap();
            let large=Large::from_value(Large).query::<large::ExchangeAbi>().unwrap();
            assert_eq!(small.accept(NcValueRef::new(&small::Shell {inner:small::Phantom{data:()}}).unwrap()),Ok(17));
            assert_eq!(large.accept(NcValueRef::new(&large::Shell {inner:large::Phantom{data:0x0012_3400}}).unwrap()),Ok(0x0012_3400));
            assert!(matches!(small.query::<large::ExchangeAbi>(),Err(NC_E_ABI_VERSION)));
            assert!(matches!(large.query::<small::ExchangeAbi>(),Err(NC_E_ABI_VERSION)));
            assert_ne!(large::ExchangeAbi::INTERFACE_ID,control::ExchangeAbi::INTERFACE_ID);
        }
    "#,
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| root.join("target").into_os_string()),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("generated_wire_and_negotiation_proof ... ok"));
}
