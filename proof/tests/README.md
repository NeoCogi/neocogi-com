# Rust ownership and loan probes

Run the probes from the repository root.

```sh
python3 proof/tests/run.py
```

The runner uses installed Rust stable and Rust 1.85.0 toolchains. It reports
missing toolchains without installing them. On a system without rustup, it uses
the available `rustc`. Compilation and execution use a temporary directory.

`pass.rs` exercises receiver-bound boxed child loans, a visit under a field
guard, joined worker scopes, and separate transfer and sharing requirements.
Each `fail/` case marks the source line and diagnostic expected from rustc.
These probes use standard Rust boxes and references as the reference behavior
for the proposed COM contract. Generated bindings, native pointer provenance,
and C ABI behavior require separate evidence.
