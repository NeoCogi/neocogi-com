//! Executable proof: accepted names remain data through codecs and ABI calls.
mod shadow;
mod values;

#[cfg(test)]
mod tests {
    use super::values::*;
    use neocogi_com::*;

    #[com_object(IValues)]
    /// Echoes copied values and inspects an owned value received through COM.
    struct Echo;
    impl IValues for Echo {
        fn record(&self, value: Record) -> Result<Record, NcStatus> {
            Ok(value)
        }
        fn choice(&self, value: Choice) -> Result<Choice, NcStatus> {
            Ok(value)
        }
        fn owned(&self, value: NcValueRef<'_, Owned>) -> Result<u32, NcStatus> {
            let value = value.to_owned()?;
            assert_eq!(value.budget, "budget is application data");
            assert_eq!(value.storage[0].raw_0, 73);
            Ok(value.raw_0 as u32)
        }
    }

    /// Distinct sentinels expose capture even when the replacement has the same type.
    fn record() -> Record {
        Record {
            first: true,
            raw_0: 73,
            budget: 101,
            storage: 102,
            wire: 103,
            result: 104,
        }
    }

    /// Compare fields explicitly without requiring optional generated derives.
    fn check(value: Record) {
        assert!(value.first);
        assert_eq!((value.raw_0, value.budget, value.storage, value.wire, value.result), (73, 101, 102, 103, 104));
    }

    #[test]
    fn copied_fields_and_enum_alternatives_round_trip() {
        let echo = Echo::from_value(Echo).query::<IValuesAbi>().unwrap();
        check(echo.record(record()).unwrap());
        for source in [Choice::Named { first: true, raw_0: 73, budget: 109 }, Choice::Tuple(record()), Choice::Empty] {
            match echo.choice(source).unwrap() {
                Choice::Named { first, raw_0, budget } => {
                    assert!(first);
                    assert_eq!((raw_0, budget), (73, 109));
                }
                Choice::Tuple(value) => check(value),
                Choice::Empty => {}
            }
        }
        // SAFETY: the default budgets admit this fixed, allocation-free record.
        // Raw data came from the matching encoder and stays initialized and live.
        let (wire, _) = unsafe { record().encode_prechecked() }.unwrap();
        check(unsafe { Record::decode_prechecked(wire) }.unwrap());
        let (wire, _) = record().encode_value(&mut ValueBudget::default()).unwrap();
        check(unsafe { Record::decode_value(wire, &mut ValueBudget::default()) }.unwrap());
    }

    #[test]
    fn owned_fields_keep_data_and_shared_budget() {
        let value = Owned {
            first: true,
            raw_0: 73,
            budget: "budget is application data".into(),
            storage: vec![record()],
        };
        let echo = Echo::from_value(Echo).query::<IValuesAbi>().unwrap();
        assert_eq!(echo.owned(NcValueRef::new(&value).unwrap()).unwrap(), 73);
        let decoded = NcValueRef::new(&value).unwrap().to_owned().unwrap();
        assert!(decoded.first);
        assert_eq!(decoded.raw_0, 73);
        assert_eq!(decoded.budget, value.budget);
        check(decoded.storage[0]);
        assert!(matches!(value.encode_value(&mut ValueBudget::new(0, 100, 64)), Err(NC_E_OUT_OF_RANGE)));
        for value in [
            OwnedChoice::Named {
                first: true,
                raw_0: 73,
                budget: "owned enum".into(),
            },
            OwnedChoice::Empty,
        ] {
            match NcValueRef::new(&value).unwrap().to_owned().unwrap() {
                OwnedChoice::Named { first, raw_0, budget } => {
                    assert!(first);
                    assert_eq!(raw_0, 73);
                    assert_eq!(budget, "owned enum");
                }
                OwnedChoice::Empty => {}
            }
        }
    }

    #[com_struct(assigned_signature = b"00000000000000000000000000000901")]
    /// Handwritten Rust permits names outside IDL's snake-case spelling policy.
    struct PrivateSpelling {
        /// Forces conversion of restricted bits.
        first: bool,
        /// Must not capture the first private input binding.
        __nc_field_0: u8,
        /// Must not capture the first private output binding.
        __nc_wire_field_0: u8,
        /// Must not capture the first private storage binding.
        __nc_storage_field_0: u8,
    }

    #[test]
    fn private_spelling_does_not_reserve_source_fields() {
        let source = PrivateSpelling {
            first: true,
            __nc_field_0: 71,
            __nc_wire_field_0: 72,
            __nc_storage_field_0: 73,
        };
        let value = NcValueRef::new(&source).unwrap().to_owned().unwrap();
        assert!(value.first);
        assert_eq!((value.__nc_field_0, value.__nc_wire_field_0, value.__nc_storage_field_0), (71, 72, 73));
    }

    #[test]
    fn authored_option_is_not_a_generated_support_type() {
        let value = super::shadow::Envelope {
            value: super::shadow::Option { enabled: true },
        };
        assert!(NcValueRef::new(&value).unwrap().to_owned().unwrap().value.enabled);
    }
}
