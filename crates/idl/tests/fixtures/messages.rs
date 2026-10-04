#[cfg(test)]
mod message_tests {
    use super::*;
    use neocogi_com::{com_object, ComValue, ComPtr, NcValueRef, NcStatus};

    #[com_object(interfaces(IMessageInput))]
    /// Explicit message arguments retain their concrete contents after the caller returns.
    struct MessageInput {
        /// The callee owns this value after decoding its borrowed wire representation.
        value: Option<Completion>,
    }
    impl IMessageInput for MessageInput {
        /// Validates the exact tag and retains an interface payload before returning.
        fn accept(&mut self, value: NcValueRef<'_, Completion>) -> Result<(), NcStatus> {
            self.value = Some(value.to_owned()?);
            Ok(())
        }
    }

    /// Borrowed message arguments carry owned capabilities without weakening copied-value rules.
    #[test]
    fn ordinary_method_accepts_an_explicit_borrowed_message() {
        let input = MessageInput::into_local(MessageInput { value: None });
        let mut interface = input.query::<IMessageInputAbi>().unwrap();
        {
            let value = Completion::Accepted {
                token: Token::from_value(Token).query().unwrap(),
            };
            interface.accept(NcValueRef::new(&value).unwrap()).unwrap();
        }
        match input.try_borrow().unwrap().value.as_ref().unwrap() {
            Completion::Accepted { token } => assert_eq!(token.value(), Ok(42)),
            Completion::Cancelled => panic!("retained the wrong alternative"),
        }
    }

    #[com_object(interfaces(IToken))]
    /// Object lifetime is observable after all producer-side references are dropped.
    struct Token;
    impl IToken for Token {
        /// Reads the retained capability after producer ownership has ended.
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(42)
        }
    }

    #[com_object(IProducer)]
    /// A producer owns its destination directly and implements an ordinary setter.
    struct Producer {
        /// One optional concrete receiver is all the producer-side delivery storage.
        completed: IProducerEventPort,
    }
    impl IProducer for Producer {
        /// Allows tests to deliver after binding has finished.
        fn emit(&mut self) -> Result<(), NcStatus> {
            self.completed.post(Completion::Cancelled)
        }
    }

    /// Constructor-owned delivery acquires values before the call ends; destruction cannot retract them.
    #[test]
    fn generated_output_owns_a_direct_receiver() {
        let (queue, first) = neocogi_com::events::channel();
        let mut producer: ComPtr<IProducerAbi> = Producer::from_value_cyclic::<IProducerAbi>(move |origin| Producer {
            completed: neocogi_com::events::EventPort::with_queue(Some(queue), origin),
        })
        .unwrap()
        .query()
        .unwrap();
        producer.emit().unwrap();
        drop(producer);
        let message = first.try_recv().unwrap();
        assert!(message.downcast::<ICancelledEventAbi>().unwrap().is_some());
        assert!(message.origin.service::<IProducerAbi>().unwrap().unwrap().upgrade().unwrap().is_none());
    }

    #[com_object(IValidate)]
    /// Validation must complete before recording delivery.
    struct Validate {
        accepted: usize,
    }
    impl IValidate for Validate {
        fn tree(&mut self, value: NcValueRef<'_, Tree>) -> Result<(), NcStatus> {
            value.to_owned()?;
            self.accepted += 1;
            Ok(())
        }
        fn shapes(&mut self, value: NcValueRef<'_, Shapes>) -> Result<(), NcStatus> {
            value.to_owned()?;
            self.accepted += 1;
            Ok(())
        }
        fn batch(&mut self, value: NcValueRef<'_, NestedBatch>) -> Result<(), NcStatus> {
            value.to_owned()?;
            self.accepted += 1;
            Ok(())
        }
    }

    /// Generated recursive sequences own typed backing at every depth.
    #[test]
    fn recursive_tree_and_optional_array_fields_round_trip() {
        let tree = Tree {
            enabled: true,
            children: vec![Tree { enabled: false, children: vec![] }],
        };
        let decoded = NcValueRef::new(&tree).unwrap().to_owned().unwrap();
        drop(tree);
        assert!(!decoded.children[0].enabled);
        let shapes = Shapes {
            optional: OptionalNumber::Some(NumberChanged { value: 1.0, checked: true }).into(),
            flags: [false, true, false],
        };
        let decoded = NcValueRef::new(&shapes).unwrap().to_owned().unwrap();
        assert!(matches!(OptionalNumber::try_from(decoded.optional).unwrap(), OptionalNumber::Some(value) if value.checked));
        assert_eq!(decoded.flags, [false, true, false]);
    }

    /// Malformed active fields fail through generated IDL method thunks.
    #[test]
    fn recursive_aggregate_sequences_reject_invalid_boolean_and_optional_tags() {
        let local = Validate::into_local(Validate { accepted: 0 });
        let mut receiver = local.query::<IValidateAbi>().unwrap();
        let source = NestedBatch {
            values: vec![NumberChanged { value: 1.0, checked: true }],
        };
        let guard = NcValueRef::new(&source).unwrap();
        let mut element = source.values[0].encode_value(&mut neocogi_com::ValueBudget::default()).unwrap().0;
        element.checked = 2;
        let mut wire = unsafe { guard.as_ptr().read() };
        wire.values.pointer = &element;
        assert_eq!(
            receiver.batch(unsafe { NcValueRef::from_raw(&wire) }.unwrap()),
            Err(neocogi_com::NC_E_INVALID_ARG)
        );
        let shapes = Shapes {
            optional: OptionalNumber::None.into(),
            flags: [true; 3],
        };
        let guard = NcValueRef::new(&shapes).unwrap();
        let mut wire = unsafe { guard.as_ptr().read() };
        wire.optional.tag = 3;
        assert_eq!(
            receiver.shapes(unsafe { NcValueRef::from_raw(&wire) }.unwrap()),
            Err(neocogi_com::NC_E_INVALID_TAG)
        );
        assert_eq!(local.try_borrow().unwrap().accepted, 0);
    }

    /// Malformed indirect cycles terminate within a shared depth/work budget.
    #[test]
    fn raw_recursive_cycle_and_oversized_text_never_reach_application() {
        let local = Validate::into_local(Validate { accepted: 0 });
        let mut receiver = local.query::<IValidateAbi>().unwrap();
        let value = Tree { enabled: true, children: vec![] };
        let guard = NcValueRef::new(&value).unwrap();
        let mut raw = unsafe { guard.as_ptr().read() };
        raw.children.len = 1;
        raw.children.pointer = &raw;
        assert_eq!(
            receiver.tree(unsafe { NcValueRef::from_raw(&raw) }.unwrap()),
            Err(neocogi_com::NC_E_OUT_OF_RANGE)
        );
        assert_eq!(local.try_borrow().unwrap().accepted, 0);
        let text = TextChanged {
            text: "x".repeat(neocogi_com::MAX_VALUE_BYTES + 1),
            cursor: 0,
        };
        assert!(matches!(NcValueRef::new(&text), Err(neocogi_com::NC_E_OUT_OF_RANGE)));
        let pair = TextPair {
            left: "a".repeat(8),
            right: "b".repeat(8),
        };
        assert!(matches!(
            pair.encode_value(&mut neocogi_com::ValueBudget::new(15, 100, 64)),
            Err(neocogi_com::NC_E_OUT_OF_RANGE)
        ));
    }

    /// Strings acquire owned storage and reject malformed UTF-8 before acquisition.
    #[test]
    fn nested_string_validation_and_owned_enum_capability_survive_the_call() {
        let source = TextEnvelope {
            changed: TextChanged { text: "hello".into(), cursor: 2 },
        };
        let guard = NcValueRef::new(&source).unwrap();
        let mut raw = unsafe { guard.as_ptr().read() };
        let invalid = [0xffu8];
        raw.changed.text.pointer = invalid.as_ptr();
        raw.changed.text.len = 1;
        assert!(matches!(
            unsafe { NcValueRef::<TextEnvelope>::from_raw(&raw) }.unwrap().to_owned(),
            Err(neocogi_com::NC_E_INVALID_ARG)
        ));
        let decoded = guard.to_owned().unwrap();
        drop(guard);
        drop(source);
        assert_eq!(decoded.changed.text, "hello");
        let source = Completion::Accepted {
            token: Token::from_value(Token).query().unwrap(),
        };
        let decoded = NcValueRef::new(&source).unwrap().to_owned().unwrap();
        drop(source);
        match decoded {
            Completion::Accepted { token } => assert_eq!(token.value(), Ok(42)),
            Completion::Cancelled => panic!("wrong tag"),
        }
    }
    /// Inspection exposes a finite named graph for actual recursively generated values.
    #[test]
    fn resolved_recursive_metadata_is_complete_and_does_not_expand_cycles() {
        use neocogi_com::design::{IdlShape, IdlValueType};
        let values = IValidateAbi::IDL_INTERFACE.values;
        assert_eq!(values.iter().filter(|value| value.qualified_name == "example.messages.Tree").count(), 1);
        let tree = values.iter().find(|value| value.qualified_name == "example.messages.Tree").unwrap();
        let IdlShape::Struct(fields) = tree.shape else { panic!("record expected") };
        assert_eq!(fields[0].ty, IdlValueType::Scalar("bool"));
        assert_eq!(fields[1].ty, IdlValueType::Sequence(&IdlValueType::Named("example.messages.Tree")));
        assert!(values.iter().any(|value| value.qualified_name == "example.messages.NumberChanged"));
    }
}

#[neocogi_com::com_object(interfaces(ICancelledEvent), ownership = exclusive)]
struct CancelledSnapshot;
impl neocogi_com::events::IEvent for CancelledSnapshot {}
impl ICancelledEvent for CancelledSnapshot {}
#[neocogi_com::com_object(interfaces(IAcceptedEvent), ownership = exclusive)]
struct AcceptedSnapshot(neocogi_com::ComPtr<ITokenAbi>);
impl neocogi_com::events::IEvent for AcceptedSnapshot {}
impl IAcceptedEvent for AcceptedSnapshot {
    fn token(&self) -> Result<neocogi_com::ComPtr<ITokenAbi>, neocogi_com::NcStatus> { Ok(self.0.clone()) }
}
impl neocogi_com::events::EventPayload for Completion {
    fn into_event(self) -> Result<neocogi_com::ComBox<neocogi_com::events::IEventAbi>, neocogi_com::NcStatus> {
        match self {
            Self::Cancelled => CancelledSnapshot::into_box(CancelledSnapshot).query().map_err(|(status, _)| status),
            Self::Accepted { token } => AcceptedSnapshot::into_box(AcceptedSnapshot(token)).query().map_err(|(status, _)| status),
        }
    }
    fn from_event(event: &neocogi_com::ComBox<neocogi_com::events::IEventAbi>) -> Result<Option<Self>, neocogi_com::NcStatus> {
        if let Some(value) = neocogi_com::events::downcast_event::<IAcceptedEventAbi>(event)? {
            return Ok(Some(Self::Accepted { token: value.token()? }));
        }
        Ok(neocogi_com::events::downcast_event::<ICancelledEventAbi>(event)?.map(|_| Self::Cancelled))
    }
}
