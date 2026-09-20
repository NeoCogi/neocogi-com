//! Generates the optional persistence capability contracts.

/// Keeps interface identity and ABI validation in the existing IDL build path.
fn main() {
    neocogi_idl_build::Module::directory("idl", "neocogi/com/persistence")
        .build()
        .expect("failed to build neocogi.com.persistence");
}
