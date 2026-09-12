use neocogi_com::{ComPtr, IUnknown, NC_E_INVALID_ARG, NC_OK, NcStatus, com_interface, com_object};

#[com_interface(
    auto_guid = "nc.tests.status_contract.InvalidStatus:v1",
    assigned_signature = b"00000000000000000000000000000003"
)]
pub trait IInvalidStatus {
    fn plain(&self, status: NcStatus) -> Result<u32, NcStatus>;
    fn object(&self, status: NcStatus) -> Result<ComPtr<IUnknown>, NcStatus>;
    fn void(&self, status: NcStatus) -> Result<(), NcStatus>;
}

#[com_object(interfaces(IInvalidStatus))]
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

fn invalid_error_statuses() -> [NcStatus; 2] {
    [NC_OK, NcStatus::from_code(1)]
}

#[test]
fn generated_thunks_normalize_non_negative_result_errors_for_every_return_shape() {
    let invalid = InvalidStatusObject::create().query::<IInvalidStatusAbi>().unwrap();
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

#[neocogi_com::com_enum(tag = u8, assigned_signature = b"00000000000000000000000000000004")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Legal Rust variants whose lowercase spellings are reserved field identifiers.
enum KeywordPayload {
    /// Exercises a control-flow keyword.
    Move(u32),
    /// Exercises a type-definition keyword.
    Type(u8),
    /// Exercises a keyword which cannot simply become a raw identifier.
    Crate(u64),
    /// Previously collided with the generator's empty-union sentinel.
    _EMPTY(u16),
}

#[test]
fn keyword_shaped_payload_variants_round_trip_without_identifier_collisions() {
    for value in [
        KeywordPayload::Move(7),
        KeywordPayload::Type(8),
        KeywordPayload::Crate(9),
        KeywordPayload::_EMPTY(10),
    ] {
        let abi = KeywordPayloadAbi::from(value);
        assert_eq!(KeywordPayload::try_from(abi), Ok(value));
    }
}
