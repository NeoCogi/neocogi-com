use neocogi_com::{ComPtr, IUnknown, NC_E_INVALID_ARG, NC_OK, NcStatus, com_object};

extern crate self as fixture_status_contract;
neocogi_com_test_contracts::status_contract!();

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
    let invalid = InvalidStatusObject::from_value(InvalidStatusObject).query::<IInvalidStatusAbi>().unwrap();
    let raw = invalid.as_raw();

    for invalid_error in invalid_error_statuses() {
        let mut plain_out = 0xfeed_beef;
        let status = unsafe { ((*raw.vtable).plain)(raw.object, invalid_error, &mut plain_out) };
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(plain_out, 0xfeed_beef);
        assert_eq!(invalid.plain(invalid_error), Err(NC_E_INVALID_ARG));

        let sentinel = neocogi_com::ComRaw::<IUnknown> {
            object: core::ptr::dangling_mut(),
            vtable: core::ptr::null(),
        };
        let mut object_out = sentinel;
        let status = unsafe { ((*raw.vtable).object)(raw.object, invalid_error, &mut object_out) };
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(object_out, sentinel);
        match invalid.object(invalid_error) {
            Ok(_) => panic!("non-negative Result::Err unexpectedly produced an object"),
            Err(status) => assert_eq!(status, NC_E_INVALID_ARG),
        }

        let status = unsafe { ((*raw.vtable).void)(raw.object, invalid_error) };
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(invalid.void(invalid_error), Err(NC_E_INVALID_ARG));
    }
}

#[test]
fn keyword_shaped_payload_variants_round_trip_without_identifier_collisions() {
    for value in [
        KeywordPayload::Move(7),
        KeywordPayload::Type(8),
        KeywordPayload::Crate(9),
        KeywordPayload::Empty(10),
    ] {
        let wire = neocogi_com::encode_inline(&value).unwrap();
        // SAFETY: checked inline encoding supplies valid self-contained storage.
        assert_eq!(unsafe { neocogi_com::decode_inline::<KeywordPayload>(wire) }, Ok(value));
    }
}
