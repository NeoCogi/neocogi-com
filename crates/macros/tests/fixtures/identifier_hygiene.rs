//! Executable proof: accepted names remain data through codecs and ABI calls.
mod shadow;
mod values;

#[cfg(test)]
mod capability_dispatch {
    //! Ordinary handle syntax must enter each legal generated provider method.
    use super::values::*;
    use neocogi_com::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    /// Shared provider with externally observable ABI-entry instrumentation.
    #[com_object(ICapability)]
    struct Capability {
        calls: Arc<AtomicUsize>,
    }
    /// Exclusive provider uses the same legal callable contract.
    #[com_object(ICapability, ownership = exclusive)]
    struct ExclusiveCapability {
        calls: Arc<AtomicUsize>,
    }

    /// Equal semantic checks apply to both ownership policies without adapters.
    macro_rules! implement {
        ($provider:ty) => {
            impl ICapability for $provider {
                fn weak_capability(&self) -> Result<WeakComPtr<ICapabilityAbi>, NcStatus> {
                    self.calls.fetch_add(1, Ordering::Relaxed);
                    Err(NC_E_CLOSED)
                }
                fn clone_value(&self) -> Result<u32, NcStatus> {
                    self.calls.fetch_add(1, Ordering::Relaxed);
                    Ok(42)
                }
                fn raw_value(&self) -> Result<u32, NcStatus> {
                    self.calls.fetch_add(1, Ordering::Relaxed);
                    Ok(64)
                }
                fn helper_names(&self, clone: u32, downgrade: u32, query: u32) -> Result<u32, NcStatus> {
                    self.calls.fetch_add(1, Ordering::Relaxed);
                    Ok(clone + downgrade + query)
                }
            }
        };
    }
    implement!(Capability);
    implement!(ExclusiveCapability);

    /// The exact weak-return type retains provider failure; raw/clone-like names
    /// return distinct sentinels. Private helper-name arguments remain legal.
    #[test]
    fn legal_names_dispatch_through_owned_borrowed_and_exclusive_handles() {
        macro_rules! check {
            ($receiver:expr) => {{
                let receiver = $receiver;
                assert_eq!(receiver.weak_capability().unwrap_err(), NC_E_CLOSED);
                assert_eq!(receiver.clone_value().unwrap(), 42);
                assert_eq!(receiver.raw_value().unwrap(), 64);
                assert_eq!(receiver.helper_names(1, 20, 300).unwrap(), 321);
            }};
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let shared = Capability::from_value(Capability { calls: calls.clone() }).query::<ICapabilityAbi>().unwrap();
        check!(&shared);
        check!(shared.as_com_ref());
        let exclusive = ExclusiveCapability::into_box(ExclusiveCapability { calls: calls.clone() })
            .query::<ICapabilityAbi>()
            .unwrap();
        check!(&exclusive);
        let weak = exclusive.downgrade();
        check!(&weak);
        assert_eq!(calls.load(Ordering::Relaxed), 16);
    }
}

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

    #[com_object(super::shadow::IShadow)]
    /// A local Option remains an application type in the provider signature.
    struct Shadow;
    impl super::shadow::IShadow for Shadow {
        fn echo(&self, value: u32, other: super::shadow::Option) -> Result<u32, NcStatus> {
            assert!(other.enabled);
            Ok(value)
        }
        fn flags(&self, value: super::shadow::Flags) -> Result<u8, NcStatus> {
            Ok(value.raw())
        }
        fn typed(&self, input: super::shadow::T, other: super::shadow::M) -> Result<super::shadow::T, NcStatus> {
            assert_eq!(other.value, 91);
            Ok(input)
        }
    }

    #[test]
    fn authored_option_is_not_a_generated_support_type() {
        let value = super::shadow::Envelope {
            value: super::shadow::Option { enabled: true },
        };
        assert!(NcValueRef::new(&value).unwrap().to_owned().unwrap().value.enabled);
        let shadow = Shadow::from_value(Shadow).query::<super::shadow::IShadowAbi>().unwrap();
        assert_eq!(shadow.echo(73, value.value), Ok(73));
        let flags = super::shadow::Flags::ENABLED | super::shadow::Flags::VISIBLE;
        assert_eq!(flags, super::shadow::Flags::KNOWN_BITS);
        assert_eq!(shadow.flags(flags).unwrap(), 3);
        assert_eq!(shadow.flags(super::shadow::Flags::from_raw(4)), Err(NC_E_INVALID_TAG));
        assert_eq!(shadow.typed(super::shadow::T { value: 73 }, super::shadow::M { value: 91 }).unwrap().value, 73);
        assert_eq!(NcValueRef::new(&flags).unwrap().to_owned().unwrap(), flags);
        assert_eq!(super::shadow::State::Ready.canonical_name(), Some("ready"));
        assert_eq!(super::shadow::State::from_raw(255).canonical_name(), None);
    }
    #[com_object(IArguments)]
    /// Exercises output completion after the generated status/result locals exist.
    struct Arguments;
    impl IArguments for Arguments {
        fn buffers(
            &self,
            mut status: NcOutputWriter<'_, Record>,
            mut result: NcOutputWriter<'_, Record>,
            this: NcCopyFromBufferOffset<'_, Record>,
            object: u32,
        ) -> Result<u32, NcStatus> {
            check(this.as_slice()[0]);
            assert_eq!((status.offset(), result.offset(), this.offset()), (3, 5, 7));
            status.push(this.as_slice()[0])?;
            result.push(this.as_slice()[0])?;
            Ok(object)
        }
        fn scalars(&self, this: u32, object: u32, result: u32, status: u32) -> Result<u32, NcStatus> {
            Ok(this + 10 * object + 100 * result + 1000 * status)
        }
    }

    #[test]
    fn positional_arguments_keep_outputs_and_metadata() {
        let object = Arguments::from_value(Arguments).query::<IArgumentsAbi>().unwrap();
        assert_eq!(object.scalars(1, 2, 3, 4).unwrap(), 4321);
        let mut status = neocogi_com::NcOutputBuffer::new();
        let mut result = neocogi_com::NcOutputBuffer::new();
        assert_eq!(
            object
                .buffers(status.output(3, 1), result.output(5, 1), NcCopyFromBufferOffset::new(7, &[record()]), 89)
                .unwrap(),
            89
        );
        check(status[0]);
        check(result[0]);
        // SAFETY: generated descriptors and names are immutable static storage.
        let text = unsafe { format_method_signature(&IArgumentsAbi::METHOD_DESCRIPTORS[1]) }.unwrap();
        assert_eq!(
            text,
            "scalars(this: value<u32>, object: value<u32>, result: value<u32>, status: value<u32>) -> out<u32> status"
        );
    }
}

#[cfg(test)]
mod private_arguments {
    use neocogi_com::*;

    #[com_interface(auto_guid = "nc.tests.hygiene.Parameters:v1", assigned_signature = b"00000000000000000000000000000902")]
    /// Parameters deliberately use both old and new generator-private spellings.
    trait IParameters {
        /// Buffer helper roles cannot be captured by later source parameters.
        fn buffers(
            &self,
            data: NcCopyFromBufferOffset<'_, bool>,
            __nc_data_values: u32,
            __nc_data_offset: u32,
            __nc_data_len: u32,
            __nc_data_ptr: u32,
            __nc_arg_0: u32,
            __nc_arg_0_values: u32,
            __nc_arg_0_offset: u32,
            __nc_arg_0_len: u32,
            __nc_arg_0_ptr: u32,
        ) -> Result<u32, NcStatus>;
        /// A failed owner transfer must restore the exact original object.
        fn owner(&self, status: ComBox<ITokenAbi>, result: bool) -> Result<u32, (NcStatus, ComBox<ITokenAbi>)>;
    }
    #[com_interface(auto_guid = "nc.tests.hygiene.Token:v1", assigned_signature = b"00000000000000000000000000000903")]
    /// Observes a transferred owner's continued lifetime.
    trait IToken {
        fn value(&self) -> Result<u32, NcStatus>;
    }
    #[com_object(interfaces(IToken), ownership = exclusive)]
    /// Single owner returned on rejection and destroyed on successful consumption.
    struct Token;
    impl IToken for Token {
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(73)
        }
    }
    #[com_object(IParameters)]
    /// Stateless provider so capture cannot be hidden by application bookkeeping.
    struct Parameters;
    impl IParameters for Parameters {
        fn buffers(
            &self,
            data: NcCopyFromBufferOffset<'_, bool>,
            __nc_data_values: u32,
            __nc_data_offset: u32,
            __nc_data_len: u32,
            __nc_data_ptr: u32,
            __nc_arg_0: u32,
            __nc_arg_0_values: u32,
            __nc_arg_0_offset: u32,
            __nc_arg_0_len: u32,
            __nc_arg_0_ptr: u32,
        ) -> Result<u32, NcStatus> {
            assert_eq!(data.offset(), 9);
            assert_eq!(data.as_slice(), &[true, false]);
            assert_eq!((__nc_data_values, __nc_data_offset, __nc_data_len, __nc_data_ptr), (1, 2, 3, 4));
            assert_eq!(
                (__nc_arg_0, __nc_arg_0_values, __nc_arg_0_offset, __nc_arg_0_len, __nc_arg_0_ptr),
                (5, 6, 7, 8, 9)
            );
            Ok(73)
        }
        fn owner(&self, status: ComBox<ITokenAbi>, result: bool) -> Result<u32, (NcStatus, ComBox<ITokenAbi>)> {
            match result {
                true => Ok(status.value().unwrap()),
                false => Err((NC_E_INVALID_ARG, status)),
            }
        }
    }
    #[test]
    fn buffer_helpers_and_owning_rejection_are_independent_of_parameter_names() {
        let provider = Parameters::from_value(Parameters).query::<IParametersAbi>().unwrap();
        assert_eq!(
            provider.buffers(NcCopyFromBufferOffset::new(9, &[true, false]), 1, 2, 3, 4, 5, 6, 7, 8, 9),
            Ok(73)
        );
        let token = Token::into_box(Token).query::<ITokenAbi>().unwrap();
        let weak = token.downgrade();
        let (error, token) = provider.owner(token, false).unwrap_err();
        assert_eq!(error, NC_E_INVALID_ARG);
        assert!(weak.same_object(&token.downgrade()));
        assert_eq!(weak.value(), Ok(73));
        assert_eq!(provider.owner(token, true).unwrap(), 73);
        assert_eq!(weak.value(), Err(NC_E_DROPPED));
    }
}

/// User item names must not capture generated core support types or variants.
#[cfg(test)]
#[allow(dead_code)]
mod support_shadow {
    pub type Result = u32;
    pub type Option = bool;
    pub struct Some;
    pub struct None;
    pub struct Ok;
    pub struct Err;

    #[neocogi_com::com_interface(auto_guid = "nc.tests.hygiene.Shadow:v1", assigned_signature = b"00000000000000000000000000000904")]
    pub trait IShadow {
        fn echo(&self, value: Result, other: Option) -> ::core::result::Result<u32, neocogi_com::NcStatus>;
    }
}
