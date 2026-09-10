# Contract Crate Workflow

This example demonstrates the recommended workflow for a long-lived Rust
host/plugin ABI:

- `abi`: a `no_std` contract crate that defines only COM interfaces, ABI-safe
  structs, stable IDs, and shared constants.
- `plugin`: a `cdylib` that implements the contract and exports
  `nc_get_plugin_descriptor`.
- `host`: a host-side test crate that consumes the same contract crate, checks
  golden interface IDs/signatures/method order/diagnostic text, loads the plugin
  through `Runtime::load_plugin_library`, constructs a class, and performs
  COM method calls.

Run it directly with:

```sh
NC_COM_CONTRACT_WORKFLOW_PLUGIN_TARGET_DIR="$PWD/target/workflow-example-contract-plugin" \
  cargo test --manifest-path workflow-examples/contract-crate/host/Cargo.toml \
    --offline \
    --target-dir target/workflow-example-contract-host
```

The host test builds the plugin crate before loading it. The
`NC_COM_CONTRACT_WORKFLOW_PLUGIN_TARGET_DIR` variable controls where that plugin
artifact is built, while `--target-dir` controls the host test build output.

## Host Failure Policy

This example treats plugin contract violations as fatal host errors. A
production host should panic with the loader or `NcStatus` diagnostic when a
required plugin fails descriptor validation, class/interface ABI validation,
construction, or required method calls. An in-process plugin is not a sandbox
boundary; once the component contract is violated, continuing in the same
process can preserve corrupted state for later attacker-controlled use. The host
crate's `*_or_panic` helpers show this policy explicitly, while the process-wide
panic strategy remains the component developer's responsibility.
