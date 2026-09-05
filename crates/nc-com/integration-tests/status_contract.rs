use nc_com::{
    ComPtr, IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK,
    NcClassDescriptor, NcGuid, NcInterfaceId, NcStaticUtf8, NcStatus, ThreadLocal, com_class,
    com_interface, construct_class_checked,
};

#[com_interface(auto_guid = "nc.tests.status_contract.InvalidStatus:v1")]
pub trait IInvalidStatus {
    fn plain(&self, status: NcStatus) -> Result<u32, NcStatus>;
    fn object(&self, status: NcStatus) -> Result<ComPtr<IUnknown>, NcStatus>;
    fn void(&self, status: NcStatus) -> Result<(), NcStatus>;
}

#[com_class(interfaces(IInvalidStatus))]
pub struct InvalidStatusObject;

impl IInvalidStatus for InvalidStatusObject {
    fn plain(&self, status: NcStatus) -> Result<u32, NcStatus> {
        Err(status)
    }

    fn object(&self, status: NcStatus) -> Result<ComPtr<IUnknown>, NcStatus> {
        Err(status)
    }

    fn void(&self, status: NcStatus) -> Result<(), NcStatus> {
        Err(status)
    }
}

const CONSTRUCTOR_CLASS_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.tests.status_contract.Constructor");

unsafe extern "C" fn unused_constructor(
    _class: *const NcClassDescriptor,
    _arguments: *mut IUnknown,
    out: *mut *mut IUnknown,
) -> NcStatus {
    if out.is_null() {
        NC_E_POINTER
    } else {
        NC_E_INVALID_ARG
    }
}

static CONSTRUCTOR_DESCRIPTOR: NcClassDescriptor = NcClassDescriptor {
    class_id: CONSTRUCTOR_CLASS_ID,
    qualified_name: NcStaticUtf8::from_static("nc.tests.status_contract.Constructor"),
    class_version: NcStaticUtf8::from_static("1.0.0"),
    constructor_args: NcInterfaceId::NONE,
    threading: NC_ABI_THREADING_THREAD_LOCAL,
    construct: Some(unused_constructor),
    interface_count: 0,
    interfaces: core::ptr::null(),
};

fn invalid_error_statuses() -> [NcStatus; 2] {
    [NC_OK, NcStatus::from_code(1)]
}

#[test]
fn generated_thunks_normalize_non_negative_result_errors_for_every_return_shape() {
    let invalid = InvalidStatusObject::create()
        .query::<IInvalidStatusAbi>()
        .unwrap();
    let raw = invalid.as_mut_ptr();

    for invalid_error in invalid_error_statuses() {
        let mut plain_out = 0xfeed_beef;
        let status = unsafe { ((*(*raw).vtable).plain)(raw, invalid_error, &mut plain_out) };
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(plain_out, 0xfeed_beef);
        assert_eq!(invalid.plain(invalid_error), Err(NC_E_INVALID_ARG));

        let sentinel = core::ptr::NonNull::<IUnknown>::dangling().as_ptr();
        let mut object_out = sentinel;
        let status = unsafe { ((*(*raw).vtable).object)(raw, invalid_error, &mut object_out) };
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(object_out, sentinel);
        match invalid.object(invalid_error) {
            Ok(_) => panic!("non-negative Result::Err unexpectedly produced an object"),
            Err(status) => assert_eq!(status, NC_E_INVALID_ARG),
        }

        let status = unsafe { ((*(*raw).vtable).void)(raw, invalid_error) };
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(invalid.void(invalid_error), Err(NC_E_INVALID_ARG));
    }
}

#[test]
fn checked_constructor_normalizes_non_negative_result_errors_and_clears_output() {
    for invalid_error in invalid_error_statuses() {
        let mut out = core::ptr::NonNull::<IUnknown>::dangling().as_ptr();
        let status = unsafe {
            construct_class_checked::<ThreadLocal, _>(
                &CONSTRUCTOR_DESCRIPTOR,
                core::ptr::null_mut(),
                &mut out,
                CONSTRUCTOR_CLASS_ID,
                NcInterfaceId::NONE,
                |_arguments| Err(invalid_error),
            )
        };

        assert_eq!(status, NC_E_INVALID_ARG);
        assert!(out.is_null());
    }
}

#[test]
fn checked_constructor_success_publishes_a_non_null_owned_output() {
    let mut out = core::ptr::null_mut();
    let status = unsafe {
        construct_class_checked::<ThreadLocal, _>(
            &CONSTRUCTOR_DESCRIPTOR,
            core::ptr::null_mut(),
            &mut out,
            CONSTRUCTOR_CLASS_ID,
            NcInterfaceId::NONE,
            |_arguments| Ok(InvalidStatusObject::create()),
        )
    };

    assert_eq!(status, NC_OK);
    assert!(!out.is_null());

    // SAFETY: Successful construction transferred one owned reference to us.
    let owned = unsafe { ComPtr::<IUnknown>::from_raw_owned(out) };
    assert!(owned.is_some());
}
