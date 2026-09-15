//! Embeds the canonical NCIDL events description in the sole COM runtime image.

/// Resolves authored events NCIDL and generates immutable core-ABI statics.
fn main() {
    neocogi_idl_build::EmbeddedModule::directory("../event-contracts/idl", "neocogi/com/events")
        .build()
        .expect("failed to embed the COM events description");
}
