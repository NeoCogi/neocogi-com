use neocogi_com::{ComPtr, ComRef, NcStatus, com_interface};

#[com_interface(auto_guid = "nc.tests.ui.CanonicalClientBase:v1", assigned_signature = b"00000000000000000000000000000047")]
/// Base contract used to verify inherited lookup on the generated client view.
trait ICanonicalClientBase {
    /// Reads one plain value through the shared safe receiver.
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.ui.CanonicalClientDerived:v1", assigned_signature = b"00000000000000000000000000000048")]
/// Derived contract used through both owning and borrowed handles.
trait ICanonicalClientDerived: ICanonicalClientBase {
    /// Writes one plain value through the mutable safe receiver.
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus>;

    /// Returns an owned base interface after receiving a checked borrow.
    fn echo(&self, source: ComRef<'_, ICanonicalClientBaseAbi>) -> Result<ComPtr<ICanonicalClientBaseAbi>, NcStatus>;
}

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
