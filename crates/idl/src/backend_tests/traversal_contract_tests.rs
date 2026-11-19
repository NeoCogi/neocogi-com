//! Independent bindings freeze traversal order and prove callable compatibility.
use std::{fs, path::Path, process::Command};
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
