use contract_abi::IContractValue;
use nc_com::{NcStatus, com_object};

// A host-owned value object passed into the plugin. This demonstrates that COM
// objects can cross in both directions as long as both sides share the contract.
#[com_object(interfaces(contract_abi::IContractValue))]
pub struct HostValue {
    value: u32,
}

impl IContractValue for HostValue {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::HostValue;
    use contract_abi::{ContractPoint, ContractWidgetAbi, ContractWidgetComPtrExt, IContractValueAbi, IContractValueComPtrExt};
    use nc_com::NC_E_NOINTERFACE;
    use nc_com::{AbiType, ComInterface, ComPtr, LoadedModuleInfo, NcAbiHash, NcAbiType, Runtime, format_method_signature};
    use std::fmt::{Display, Write};
    use std::path::{Path, PathBuf};
    use std::process::Command;

    #[test]
    fn contract_crate_exports_stable_abi_metadata() {
        // These values are the compatibility contract. If any assertion changes,
        // callers need an intentional side-by-side replacement contract or a major break.
        assert_eq!(contract_abi::CONTRACT_PLUGIN_ID.to_string(), "036bcacf-fcce-5bf9-a177-0f387041ab6c");
        assert_eq!(ContractWidgetAbi::CLASS_ID.to_string(), "d3e19d71-73a9-5255-a7a9-69a28313d6ce");
        assert_eq!(IContractValueAbi::IID.to_string(), "c6d018c2-42df-5d19-83ce-73c797c09102");
        assert_eq!(
            IContractValueAbi::ABI_HASH.bytes,
            [0x92, 0x9b, 0x4a, 0x08, 0x3d, 0xaa, 0xf2, 0x05, 0xa4, 0x33, 0x76, 0xa4, 0x85, 0xbe, 0x69, 0x91, 0xef, 0xeb, 0xd0, 0x33, 0x3f, 0x23, 0x65, 0x14, 0xc6, 0x4c, 0x23, 0xd7, 0xa3, 0x9f, 0xaa, 0x1d,]
        );
        assert_eq!(ContractWidgetAbi::IID, ContractWidgetAbi::CLASS_ID);
        assert_eq!(
            ContractWidgetAbi::CONSTRUCTOR_ABI_HASH,
            NcAbiHash::from_abi_parts(&[nc_com::NcAbiHashPart::Arg {
                index: 0,
                passing: nc_com::NcAbiPassing::Value,
                ty: NcAbiType::U32,
            }])
        );

        // Method order is part of the vtable layout. The hash catches signature
        // drift, while these checks make accidental reordering obvious.
        let value_methods = IContractValueAbi::METHOD_DESCRIPTORS;
        assert_eq!(value_methods.len(), 1);
        assert_eq!(unsafe { value_methods[0].name.as_str() }.unwrap(), "value");
        assert_eq!(
            // SAFETY: Macro-generated method descriptors and their transitive
            // metadata are immutable static values.
            unsafe { format_method_signature(&value_methods[0]) }.unwrap(),
            "value() -> out<u32> status"
        );

        let widget_methods = ContractWidgetAbi::METHOD_DESCRIPTORS;
        assert_eq!(widget_methods.len(), 4);
        let contract_point = aggregate_type::<ContractPoint>();
        let contract_value = interface_type::<IContractValueAbi>();
        let expected_signatures = [
            (
                "add",
                "add(left: value<u32>, right: value<u32>) -> out<u32> status".to_string(),
            ),
            (
                "translate",
                format!(
                    "translate(point: value<{contract_point}>) -> out<{contract_point}> status"
                ),
            ),
            (
                "read_value",
                format!(
                    "read_value(value: com_ref<thread_local, {contract_value}>) -> out<u32> status"
                ),
            ),
            (
                "make_value",
                format!(
                    "make_value(value: value<u32>) -> com_ptr_out<thread_local, {contract_value}> status"
                ),
            ),
        ];
        for (index, (expected_name, expected_signature)) in expected_signatures.iter().enumerate() {
            assert_eq!(
                unsafe { widget_methods[index].name.as_str() }.unwrap(),
                *expected_name
            );
            assert_eq!(
                // SAFETY: Macro-generated method descriptors and their
                // transitive metadata are immutable static values.
                unsafe { format_method_signature(&widget_methods[index]) }.unwrap(),
                expected_signature.as_str()
            );
        }
    }

    fn aggregate_type<T: AbiType>() -> String {
        match T::ABI_TYPE {
            NcAbiType::Aggregate(hash) => format!("aggregate<{}>", hash_hex(hash)),
            _ => panic!("expected aggregate ABI type"),
        }
    }

    fn interface_type<T: ComInterface>() -> String {
        format!("interface<{}, {}>", T::IID, hash_hex(T::ABI_HASH))
    }

    fn hash_hex(hash: NcAbiHash) -> String {
        let mut out = String::with_capacity(64);
        for byte in hash.bytes {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    #[test]
    fn host_and_plugin_share_a_contract_crate_for_com_abi() {
        // Build a separate cdylib, then load it through the same dynamic-library
        // path a real host would use.
        let library_path = build_contract_plugin_example();
        let runtime = Runtime::global();

        let loaded = load_required_contract_plugin_or_panic(runtime, &library_path);
        assert_eq!(loaded.qualified_name(), "nc.tests.contract.plugin");
        assert_eq!(loaded.plugin_version(), "1.0.0");

        let widget = construct_contract_widget_or_panic(runtime, 13);

        assert_eq!(
            required_call_or_panic("IContractWidget::add", widget.add(2, 5)),
            20
        );
        assert_eq!(
            required_call_or_panic(
                "IContractWidget::translate",
                widget.translate(ContractPoint { x: 7, y: 30 })
            ),
            ContractPoint { x: 20, y: 17 }
        );

        let host_value = HostValue::from_value(HostValue { value: 8 })
            .query::<IContractValueAbi>()
            .unwrap();
        assert_eq!(
            required_call_or_panic(
                "IContractWidget::read_value",
                widget.read_value(host_value.as_com_ref())
            ),
            21
        );

        // The plugin can also allocate and return a COM object owned by the host.
        let plugin_value =
            required_call_or_panic("IContractWidget::make_value", widget.make_value(29));
        assert_eq!(
            required_call_or_panic("IContractValue::value", plugin_value.value()),
            42
        );
    }

    #[test]
    #[should_panic(expected = "fatal plugin contract violation while IContractWidget::add")]
    fn required_component_method_error_panics() {
        let _: u32 = required_call_or_panic("IContractWidget::add", Err(NC_E_NOINTERFACE));
    }

    fn load_required_contract_plugin_or_panic(
        runtime: &Runtime,
        library_path: &Path,
    ) -> LoadedModuleInfo {
        // Loading validates the plugin descriptor shape and registers the
        // classes advertised by the plugin. For a required in-process component,
        // a loader error is fatal: there is no safe degraded mode that keeps
        // using this component boundary.
        let loaded = unsafe { runtime.load_plugin_library(library_path) };

        // The class ID says which factory to use; the interface ABI check says
        // whether this host knows how to call objects produced by that factory.
        runtime
            .require_interface_type::<ContractWidgetAbi>(ContractWidgetAbi::CLASS_ID)
            .unwrap_or_else(|status| panic_plugin_contract("validating required widget interface", status));

        loaded
    }

    fn construct_contract_widget_or_panic(runtime: &Runtime, seed: u32) -> ComPtr<ContractWidgetAbi> {
        // The shared class macro builds the exact borrowed constructor frame;
        // the host never declares or allocates a constructor COM object.
        let arguments = ContractWidgetAbi::arguments(seed).unwrap_or_else(|status| panic_plugin_contract("building widget arguments", status));
        runtime
            .construct::<ContractWidgetAbi>(arguments)
            .unwrap_or_else(|status| panic_plugin_contract("constructing required widget", status))
    }

    fn required_call_or_panic<T>(context: &str, result: Result<T, nc_com::NcStatus>) -> T {
        result.unwrap_or_else(|status| panic_plugin_contract(context, status))
    }

    fn panic_plugin_contract(context: &str, detail: impl Display) -> ! {
        panic!("fatal plugin contract violation while {context}: {detail}");
    }

    fn build_contract_plugin_example() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../plugin/Cargo.toml");
        // The integration wrapper sets this so plugin build output stays in the
        // repository target directory instead of the example source tree.
        let target_dir = std::env::var_os("NC_COM_CONTRACT_WORKFLOW_PLUGIN_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/plugin"));
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let output = Command::new(cargo)
            .arg("build")
            .arg("--offline")
            .arg("--manifest-path")
            .arg(&manifest)
            .arg("--target-dir")
            .arg(&target_dir)
            .output()
            .expect("failed to build contract workflow plugin example");

        assert!(
            output.status.success(),
            "contract workflow plugin example failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        target_dir.join("debug").join(format!(
            "{}nc_com_contract_workflow_plugin{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ))
    }
}
