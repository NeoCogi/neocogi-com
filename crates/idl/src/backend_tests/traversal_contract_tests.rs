//! Independent bindings freeze traversal order and prove callable compatibility.
use std::{fmt::Write, fs, path::Path, process::Command};
use super::test_consumer::Consumer;

/// A diamond has different preorder identities and postorder methods. Metadata
/// follows reachable fields once, while independently generated views make calls.
#[test]
fn metadata_diamond_order_and_independent_com_calls() {
    let source = "struct Alpha { value: u32 } struct Zeta { nested: Alpha }
        interface Base { fn base(&self) -> Result<u32>; }
        interface Left: Base { fn left(&self) -> Result<u32>; }
        interface Right: Base { fn right(&self) -> Result<u32>; }
        interface Leaf: Left + Right { fn echo(&self, value: Zeta) -> Result<Zeta>; }";
    let emit = || {
        let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/order.ncidl"), source).unwrap()]).unwrap();
        crate::generate_rust_with_source_map(&graph, "proof.order").unwrap().source
    };
    let consumer = Consumer::new("traversal-order-peers");
    let root = consumer.directory.path();
    fs::write(root.join("src/producer.rs"), emit()).unwrap();
    fs::write(root.join("src/consumer.rs"), emit()).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"
        mod producer; mod consumer;
        #[neocogi_com_sdk::com_object(producer::Leaf)] struct Probe;
        impl producer::Base for Probe { fn base(&self) -> Result<u32,neocogi_com_sdk::NcStatus> { Ok(11) } }
        impl producer::Left for Probe { fn left(&self) -> Result<u32,neocogi_com_sdk::NcStatus> { Ok(22) } }
        impl producer::Right for Probe { fn right(&self) -> Result<u32,neocogi_com_sdk::NcStatus> { Ok(33) } }
        impl producer::Leaf for Probe {
            fn echo(&self, value: producer::Zeta) -> Result<producer::Zeta,neocogi_com_sdk::NcStatus> { Ok(value) }
        }
        #[test] fn metadata_and_calls() {
            use consumer::{BaseAbi,LeftAbi,RightAbi,LeafAbi};
            let metadata = LeafAbi::IDL_INTERFACE;
            assert_eq!(metadata.chain.iter().map(|i| i.iid).collect::<Vec<_>>(), [
                LeafAbi::IDL_INTERFACE.identity.iid, LeftAbi::IDL_INTERFACE.identity.iid,
                BaseAbi::IDL_INTERFACE.identity.iid, RightAbi::IDL_INTERFACE.identity.iid,
            ]);
            assert_eq!(metadata.methods.iter().map(|m|m.name).collect::<Vec<_>>(), ["base","left","right","echo"]);
            assert_eq!(metadata.values.iter().map(|v|v.qualified_name).collect::<Vec<_>>(), ["proof.order.Alpha","proof.order.Zeta"]);
            assert_eq!(producer::ABI_FINGERPRINT,consumer::ABI_FINGERPRINT);
            let owner=Probe::from_value(Probe);
            let leaf=owner.query::<LeafAbi>().unwrap();
            assert_eq!(leaf.base().unwrap(),11);
            assert_eq!(leaf.left().unwrap(),22);
            assert_eq!(owner.query::<RightAbi>().unwrap().right().unwrap(),33);
            assert_eq!(leaf.echo(consumer::Zeta { nested: consumer::Alpha { value: 47 } }).unwrap().nested.value,47);
        }
    "#,
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("metadata_and_calls ... ok"));
}

/// Wide IDL ancestry must compile through ordinary object construction on the floor.
#[test]
fn wide_interface_compiles_and_dispatches_through_compact_tables() {
    const PARENTS: usize = 384;
    let mut schema = String::new();
    let mut implementations = String::new();
    for parent in 0..PARENTS {
        let method = match parent {
            0 => "fn first(&self) -> Result<u32>;",
            383 => "fn last(&self) -> Result<u32>;",
            _ => "",
        };
        writeln!(schema, "interface I{parent} {{ {method} }}").unwrap();
        let implementation = match parent {
            0 => "fn first(&self) -> Result<u32, neocogi_com_sdk::NcStatus> { Ok(17) }",
            383 => "fn last(&self) -> Result<u32, neocogi_com_sdk::NcStatus> { Ok(42) }",
            _ => "",
        };
        writeln!(implementations, "impl I{parent} for Provider {{ {implementation} }}").unwrap();
    }
    let bases = (0..PARENTS).map(|parent| format!("I{parent}")).collect::<Vec<_>>().join(" + ");
    writeln!(schema, "interface Wide: {bases} {{}} interface Extra {{}}").unwrap();
    let graph = crate::resolve_modules(vec![crate::parse_module(Path::new("proof/wide.ncidl"), &schema).unwrap()]).unwrap();
    let bindings = crate::generate_rust(&graph, "proof.wide").unwrap();
    let provider = format!(
        r#"
        {bindings}
        #[neocogi_com_sdk::com_object(interfaces(Wide, Extra))] struct Provider;
        impl Wide for Provider {{}}
        impl Extra for Provider {{}}
        {implementations}

        #[test] fn dispatch() {{
            assert_eq!(Provider::COM_INTERFACE_COUNT, {count});
            assert_eq!(WideAbi::INTERFACE_CHAIN.len(), {wide});
            let owner = Provider::from_value(Provider);
            let wide = owner.query::<WideAbi>().unwrap();
            assert_eq!(wide.first(), Ok(17));
            assert_eq!(wide.query::<I383Abi>().unwrap().last(), Ok(42));
            assert_eq!(wide.query::<ExtraAbi>().unwrap().as_raw().object, wide.as_raw().object);
        }}
        "#,
        count = PARENTS + 3,
        wide = PARENTS + 1,
    );
    let consumer = Consumer::new("wide-interface-projection");
    super::name_ownership_tests::run_consumers_with_files(consumer, [("wide interface".into(), provider)], &[], &["dispatch"]);
}
