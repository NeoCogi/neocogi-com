//! Linked-binary proof that using COM does not implicitly retain inspection schemas.
use neocogi_com_sdk::{com_object, ComInterface, NcStatus};
use std::hint::black_box;

#[allow(dead_code, unused_imports)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/contracts.rs"));
}
use bindings::*;
use bindings::{
    MetadataRetentionSentinelBase as IBase, MetadataRetentionSentinelBaseAbi as IBaseAbi, MetadataRetentionSentinelBaseVtblFor as IBaseVtblFor,
    MetadataRetentionSentinelLeft as ILeft, MetadataRetentionSentinelLeftVtblFor as ILeftVtblFor, MetadataRetentionSentinelRight as IRight,
    MetadataRetentionSentinelRightVtblFor as IRightVtblFor, MetadataRetentionSentinelTop as ITop, MetadataRetentionSentinelTopAbi as ITopAbi,
    MetadataRetentionSentinelTopVtblFor as ITopVtblFor,
};

#[com_object(interfaces(ITop))]
/// Concrete behavior reached through generated typed vtables.
struct Probe;
impl IBase for Probe {
    /// Returns an observable value so the executable exercises real COM dispatch.
    fn metadata_retention_sentinel_method(&self) -> Result<u32, NcStatus> {
        Ok(black_box(42))
    }
}
impl ILeft for Probe {}
impl IRight for Probe {}
impl ITop for Probe {}

/// Exercises construction, identity negotiation, method dispatch and final release.
fn main() {
    let object = Probe::from_value(Probe);
    let top = black_box(object).query::<ITopAbi>().unwrap();
    let base = top.query::<IBaseAbi>().unwrap();
    println!("value={}", base.metadata_retention_sentinel_method().unwrap());
    // Checking only compact identity information must not retain schema strings.
    black_box(IBaseAbi::INTERFACE_ID);
    #[cfg(feature = "inspect")]
    {
        print_inspection();
        check_loan_metadata();
    }
}

/// Explicit inspection must retain every unique contract and its method descriptions.
#[cfg(feature = "inspect")]
fn print_inspection() {
    // Hiding the table from constant propagation exercises actual runtime reads.
    for descriptor in black_box(&ITopAbi::INTERFACE_CHAIN) {
        // SAFETY: descriptors and their method/text views are generated static data.
        unsafe {
            println!("{}", descriptor.qualified_name.as_str().unwrap());
            for index in 0..descriptor.method_count as usize {
                let method = &*descriptor.methods.add(index);
                println!("{}", method.name.as_str().unwrap());
            }
        }
    }
}

/// Both compiled descriptor formats must describe each authored loan.
#[cfg(feature = "inspect")]
fn check_loan_metadata() {
    use neocogi_com_sdk::idl_metadata::{MethodType, ReturnProjection, ValueType};
    use neocogi_com_sdk::{
        NcAbiPassingDescriptor, NcAbiReceiver, NcAbiTypeDescriptor, NcInterfaceDescriptor, NC_ABI_PASSING_COM_MUT, NC_ABI_PASSING_COM_MUT_OUT,
        NC_ABI_PASSING_COM_REF, NC_ABI_PASSING_COM_REF_OUT, NC_ABI_PASSING_OPTIONAL_COM_MUT, NC_ABI_PASSING_OPTIONAL_COM_MUT_OUT,
        NC_ABI_PASSING_OPTIONAL_COM_REF, NC_ABI_PASSING_OPTIONAL_COM_REF_OUT, NC_ABI_PASSING_SCOPED_COM_MUT_OUT, NC_ABI_PASSING_SCOPED_COM_REF_OUT,
        NC_ABI_THREADING_THREAD_SYNC,
    };

    let sync_passing = |tag| NcAbiPassingDescriptor {
        tag,
        threading: NC_ABI_THREADING_THREAD_SYNC,
    };
    let identity = NcAbiTypeDescriptor::from_interface(ILoanAbi::INTERFACE_ID);
    let loan = ValueType::Object("test.projection.ILoan");
    // Native descriptors keep own slots. Rust inspection lists the inherited
    // specialization first, then the methods declared on the derived interface.
    let expected = [
        (
            "loan_optional_mut",
            NcAbiReceiver::Mutable,
            ReturnProjection::Fallible,
            Some((NC_ABI_PASSING_OPTIONAL_COM_MUT, MethodType::OptionalMutableBorrowedInterface(loan))),
            NC_ABI_PASSING_OPTIONAL_COM_MUT_OUT,
            MethodType::OptionalMutableBorrowedInterfaceResult(loan),
        ),
        (
            "loan_read",
            NcAbiReceiver::Shared,
            ReturnProjection::Direct,
            None,
            NC_ABI_PASSING_COM_REF_OUT,
            MethodType::BorrowedInterfaceResult(loan),
        ),
        (
            "loan_find",
            NcAbiReceiver::Shared,
            ReturnProjection::Direct,
            Some((NC_ABI_PASSING_COM_REF, MethodType::Borrowed(loan))),
            NC_ABI_PASSING_OPTIONAL_COM_REF_OUT,
            MethodType::OptionalBorrowedInterfaceResult(loan),
        ),
        (
            "loan_write",
            NcAbiReceiver::Mutable,
            ReturnProjection::Direct,
            Some((NC_ABI_PASSING_COM_MUT, MethodType::MutableBorrowedInterface(loan))),
            NC_ABI_PASSING_COM_MUT_OUT,
            MethodType::MutableBorrowedInterfaceResult(loan),
        ),
        (
            "loan_visit",
            NcAbiReceiver::Shared,
            ReturnProjection::Direct,
            Some((NC_ABI_PASSING_OPTIONAL_COM_REF, MethodType::OptionalBorrowedInterface(loan))),
            NC_ABI_PASSING_SCOPED_COM_REF_OUT,
            MethodType::ScopedBorrowedInterface(loan),
        ),
        (
            "loan_visit_mut",
            NcAbiReceiver::Shared,
            ReturnProjection::Fallible,
            None,
            NC_ABI_PASSING_SCOPED_COM_MUT_OUT,
            MethodType::ScopedMutableBorrowedInterface(loan),
        ),
    ];
    let [derived, base] = black_box(&ILoanMethodsAbi::INTERFACE_CHAIN);
    let own_methods = |descriptor: &'static NcInterfaceDescriptor, count: usize| {
        assert_eq!(descriptor.method_count as usize, count);
        assert!(!descriptor.methods.is_null());
        // SAFETY: The generated static descriptor supplies the own-method array.
        unsafe { std::slice::from_raw_parts(descriptor.methods, count) }
    };
    let inherited = own_methods(base, 1);
    let own = own_methods(derived, expected.len() - inherited.len());
    let rust = black_box(&ILoanMethodsAbi::IDL_INTERFACE);
    println!("{}", rust.qualified_name);
    assert_eq!(rust.methods.len(), expected.len());

    for ((native, rust), (name, receiver, projection, input, native_result, rust_result)) in inherited.iter().chain(own).zip(rust.methods).zip(expected) {
        // SAFETY: Names are backed by the generated static descriptors.
        assert_eq!(unsafe { native.name.as_str() }.unwrap(), name);
        assert_eq!(native.receiver, receiver.as_raw(), "{name}");
        assert_eq!(native.returns.passing, sync_passing(native_result), "{name}");
        assert_eq!(native.returns.ty, identity, "{name}");
        assert_eq!(rust.name, name);
        assert_eq!(rust.receiver, receiver, "{name}");
        assert_eq!(rust.return_projection, projection, "{name}");
        assert_eq!(rust.returns, rust_result, "{name}");
        let count = usize::from(input.is_some());
        assert_eq!(native.param_count as usize, count, "{name}");
        assert_eq!(rust.parameters.len(), count, "{name}");
        if let Some((native_input, rust_input)) = input {
            assert!(!native.params.is_null(), "{name}");
            // SAFETY: This generated descriptor has one static parameter.
            let parameter = unsafe { &*native.params };
            // SAFETY: The parameter name has permanent generated backing.
            assert_eq!(unsafe { parameter.name.as_str() }.unwrap(), "arg", "{name}");
            assert_eq!(parameter.passing, sync_passing(native_input), "{name}");
            assert_eq!(parameter.ty, identity, "{name}");
            assert_eq!(rust.parameters[0].name, "arg", "{name}");
            assert_eq!(rust.parameters[0].ty, rust_input, "{name}");
        }
    }
}
