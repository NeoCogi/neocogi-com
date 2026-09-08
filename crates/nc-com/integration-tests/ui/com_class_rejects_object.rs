use nc_com::com_class;

/// Deliberately applies the contract-only macro to concrete object storage.
#[com_class(interfaces(ILegacyObject))]
struct LegacyObject;

fn main() {
    // Compilation must fail before this fixture can run.
}
