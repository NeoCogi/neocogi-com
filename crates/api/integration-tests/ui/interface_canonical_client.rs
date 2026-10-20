use neocogi_com::{ComPtr, ComRef, NcStatus};

extern crate self as fixture_ui_interface_canonical_client;
neocogi_com_test_contracts::ui_interface_canonical_client!();

/// Type-checks every canonical call shape without importing an extension trait.
fn call_owned_and_borrowed(
    mut owned: ComPtr<ICanonicalClientDerivedAbi>,
    mut borrowed: ComRef<'_, ICanonicalClientDerivedAbi>,
    source: ComRef<'_, ICanonicalClientBaseAbi>,
) -> Result<ComPtr<ICanonicalClientBaseAbi>, NcStatus> {
    // `ComPtr` lends the derived client, whose Deref chain supplies the base
    // method before the local mutable method is selected.
    let owned_value = owned.value()?;
    owned.set_value(owned_value)?;

    // `ComRef` lends the same client hierarchy without acquiring ownership or
    // requiring a second generated calling trait.
    let borrowed_value = borrowed.value()?;
    borrowed.set_value(borrowed_value)?;
    // The final call covers checked borrowed arguments and owned COM returns.
    borrowed.echo(source)
}

/// Compile-test entry point; this fixture is validated by type checking.
fn main() {}
