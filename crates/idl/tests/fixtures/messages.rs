#[cfg(test)]
mod message_tests {
    use super::*;
    use neocogi_com::{com_object, ComMessage, ComPtr, NcMessage, NcStatus};

    #[com_object(interfaces(IMessageInput))]
    /// Explicit message arguments retain their concrete contents after the caller returns.
    struct MessageInput {
        /// The callee owns this value after decoding its borrowed wire representation.
        value: Option<Completion>,
    }
    impl IMessageInput for MessageInput {
        /// Validates the exact tag and retains an interface payload before returning.
        fn accept(&mut self, value: NcMessage<'_, Completion>) -> Result<(), NcStatus> {
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
            interface.accept(NcMessage::new(&value)).unwrap();
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
            self.completed.post(&Completion::Cancelled)
        }

    }
    impl IProducerStream for Producer {
        /// Only the first consumer receives the event emitted during attachment.
        fn attach(&mut self, destination: neocogi_com::ComRef<'_, IProducerSinkAbi>) -> Result<(), NcStatus> {
            self.completed.attach(destination)?;
            self.completed.post(&Completion::Cancelled)
        }
    }

    /// Mapping replaces destinations; setup and producer destruction cannot retract queued facts.
    #[test]
    fn generated_output_installs_a_direct_receiver() {
        let producer: ComPtr<IProducerAbi> = Producer::from_value(Producer { completed: Default::default() }).query().unwrap();
        let (first_tx, first) = std::sync::mpsc::channel();
        neocogi_com::events::bind_source(producer.as_com_ref(), first_tx.clone(), |_| 1).unwrap();
        let (second_tx, second) = std::sync::mpsc::channel();
        assert_eq!(neocogi_com::events::bind_source(producer.as_com_ref(), second_tx.clone(), |_| 2), Err(neocogi_com::NC_E_ALREADY_BOUND));
        drop(producer);
        assert_eq!(first.try_recv().ok(), Some(1));
        assert!(second.try_recv().is_err());
    }

    /// Producer release cannot destroy consumer-owned object or text snapshots.
    #[test]
    fn typed_delivery_retains_objects_and_copies_strings() {
        let mut source = None;
        let (events_tx, events) = std::sync::mpsc::channel();
        neocogi_com::events::bind(
            |consume| {
                source = Some(Completion::__receiver(consume)?);
                Ok(())
            },
            events_tx.clone(),
            |message| message,
        )
        .unwrap();
        let token: ComPtr<ITokenAbi> = Token::from_value(Token).query().unwrap();
        Completion::Accepted { token }.post_to(&mut source).unwrap();
        drop(source);
        match events.try_recv().unwrap() {
            Completion::Accepted { token } => assert_eq!(token.value(), Ok(42)),
            Completion::Cancelled => panic!("wrong case"),
        }
        let mut source = None;
        let (events_tx, events) = std::sync::mpsc::channel();
        neocogi_com::events::bind(
            |consume| {
                source = Some(TextChanged::__receiver(consume)?);
                Ok(())
            },
            events_tx.clone(),
            |message| message,
        )
        .unwrap();
        TextChanged { text: "héllo".into(), cursor: 2 }.post_to(&mut source).unwrap();
        drop(source);
        assert_eq!(events.try_recv().unwrap().text, "héllo");
    }

    /// Unit facts retain their checked copied ABI and slices become independent consumer storage.
    #[test]
    fn unit_messages_and_owned_slices_round_trip() {
        assert_eq!(Notice::try_from(NoticeAbi::from(Notice::Ready)).unwrap(), Notice::Ready);
        let mut destination = None;
        let (events_tx, events) = std::sync::mpsc::channel();
        neocogi_com::events::bind(
            |consume| {
                destination = Some(Notice::__receiver(consume)?);
                Ok(())
            },
            events_tx.clone(),
            |message| message,
        )
        .unwrap();
        Notice::Done.post_to(&mut destination).unwrap();
        assert_eq!(events.try_recv().ok(), Some(Notice::Done));
        let mut destination = None;
        let (events_tx, events) = std::sync::mpsc::channel();
        neocogi_com::events::bind(
            |consume| {
                destination = Some(Batch::__receiver(consume)?);
                Ok(())
            },
            events_tx.clone(),
            |message| message.values,
        )
        .unwrap();
        Batch { values: vec![1, 2, 3] }.post_to(&mut destination).unwrap();
        drop(destination);
        assert_eq!(events.try_recv().ok(), Some(vec![1, 2, 3]));
        let mut bool_destination = None;
        let (bool_tx, bool_events) = std::sync::mpsc::channel();
        neocogi_com::events::bind(
            |consume| {
                bool_destination = Some(BoolBatch::__receiver(consume)?);
                Ok(())
            },
            bool_tx,
            |message| message.flags,
        )
        .unwrap();
        BoolBatch { flags: vec![false, true, false] }.post_to(&mut bool_destination).unwrap();
        assert_eq!(bool_events.try_recv().unwrap(), vec![false, true, false]);
        let value = Batch { values: vec![1] };
        let mut wire = value.encode();
        wire.payload.case_0.values.len = u64::MAX;
        // The decoder must reject the impossible range before reading any pointed-to element.
        assert!(unsafe { NcMessage::<Batch>::from_raw(&wire) }.unwrap().to_owned().is_err());
    }

    /// Invalid wire values fail before the mapping closure or queue is reached.
    #[test]
    fn malformed_payloads_never_reach_the_mapper() {
        let (events_tx, events) = std::sync::mpsc::channel();
        let sender = events_tx.clone();
        let mut receiver = NumberChanged::__receiver(Box::new(move |message| sender.send(message).map_err(|_| neocogi_com::NC_E_INVALID_ARG))).unwrap();
        let message = NumberChanged { value: 12.5, checked: true };
        receiver.post(NcMessage::new(&message)).unwrap();
        assert_eq!(events.try_recv().unwrap(), message);
        let mut wire = message.encode();
        wire.tag = 99;
        assert_eq!(
            receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()),
            Err(neocogi_com::NC_E_INVALID_TAG)
        );
        wire = message.encode();
        wire.payload.case_0.checked = 3;
        assert_eq!(
            receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()),
            Err(neocogi_com::NC_E_INVALID_ARG)
        );
        assert!(events.try_recv().is_err());
        let (text_tx, text) = std::sync::mpsc::channel();
        let sender = text_tx.clone();
        let mut receiver = TextChanged::__receiver(Box::new(move |message| sender.send(message).map_err(|_| neocogi_com::NC_E_INVALID_ARG))).unwrap();
        let bad = [0xff];
        let message = TextChanged { text: String::new(), cursor: 0 };
        let mut wire = message.encode();
        wire.payload.case_0.text = neocogi_com::NcMessageSlice::new(&bad);
        assert_eq!(
            receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()),
            Err(neocogi_com::NC_E_INVALID_ARG)
        );
        assert!(text.try_recv().is_err());
        let (bool_tx, bool_events) = std::sync::mpsc::channel();
        let sender = bool_tx.clone();
        let mut receiver = BoolBatch::__receiver(Box::new(move |message| sender.send(message).map_err(|_| neocogi_com::NC_E_INVALID_ARG))).unwrap();
        let source = BoolBatch { flags: vec![true] };
        let mut wire = source.encode();
        let invalid_bools = [0_u8, 2_u8];
        wire.payload.case_0.flags = neocogi_com::NcMessageSlice::new(&invalid_bools);
        assert_eq!(
            receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()),
            Err(neocogi_com::NC_E_INVALID_ARG)
        );
        assert!(bool_events.try_recv().is_err());
    }

    /// Both owned fields and nested messages consume one shared decode budget.
    #[test]
    fn generated_decoders_enforce_cumulative_bytes_and_record_depth() {
        let pair = TextPair { left: "abc".into(), right: "def".into() };
        let mut bytes = neocogi_com::MessageDecodeBudget::new(5, 1);
        assert_eq!(
            unsafe { TextPair::decode_with_budget(pair.encode(), &mut bytes) },
            Err(neocogi_com::NC_E_OUT_OF_RANGE)
        );
        assert_eq!(NcMessage::new(&pair).to_owned(), Ok(pair));

        let nested = TextEnvelope { changed: TextChanged { text: "ok".into(), cursor: 2 } };
        let mut records = neocogi_com::MessageDecodeBudget::new(2, 1);
        assert!(matches!(
            unsafe { TextEnvelope::decode_with_budget(nested.encode(), &mut records) },
            Err(neocogi_com::NC_E_OUT_OF_RANGE)
        ));
        let mut enough = neocogi_com::MessageDecodeBudget::new(2, 2);
        assert!(matches!(
            unsafe { TextEnvelope::decode_with_budget(nested.encode(), &mut enough) },
            Ok(TextEnvelope { changed: TextChanged { text, cursor: 2 } }) if text == "ok"
        ));
    }

    /// A valid but oversized wire buffer is rejected by the generated entry point.
    #[test]
    fn generated_receiver_rejects_oversized_buffer_before_copying() {
        let (tx, events) = std::sync::mpsc::channel();
        let mut receiver = TextChanged::__receiver(Box::new(move |message| tx.send(message).map_err(|_| neocogi_com::NC_E_INVALID_ARG))).unwrap();
        let text = "x".repeat(neocogi_com::MAX_MESSAGE_DECODE_BYTES + 1);
        let message = TextChanged { text, cursor: 0 };
        assert_eq!(receiver.post(NcMessage::new(&message)), Err(neocogi_com::NC_E_OUT_OF_RANGE));
        assert!(events.try_recv().is_err());
    }
}

#[cfg(test)]
mod direct_stream_tests {
    use super::*;
    use neocogi_com::{com_object, ComPtr, ComRef, NcStatus};
    use neocogi_com::events::bind_source;

    /// An independent producer that emits only when explicitly requested.
    #[com_object(IProducer)]
    struct Source {
        /// Exactly one concrete destination shared by every projection.
        destination: IProducerEventPort,
    }
    impl IProducerStream for Source {
        /// Checks the single lifetime slot before retaining the typed receiver.
        fn attach(&mut self, receiver: ComRef<'_, IProducerSinkAbi>) -> Result<(), NcStatus> {
            self.destination.attach(receiver)
        }
    }
    impl IProducer for Source {
        /// Publishes directly to the installed generated receiver.
        fn emit(&mut self) -> Result<(), NcStatus> {
            self.destination.post(&Completion::Cancelled)
        }
    }

    /// Queried stream projections preserve concrete type checks and one lifetime consumer.
    #[test]
    fn direct_binding_preserves_type_checks_single_attachment_and_lifetimes() {
        let mut source: ComPtr<IProducerAbi> = Source::from_value(Source { destination: Default::default() }).query().unwrap();
        // Querying another event type fails; aliases of this stream share its slot.
        assert!(source.query::<EditorStreamAbi>().is_err());
        let stream = source.query::<IProducerStreamAbi>().unwrap();
        let opaque = source.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        bind_source(opaque.as_com_ref(), tx.clone(), |_: Completion| 1_u32).unwrap();
        assert_eq!(
            bind_source(opaque.as_com_ref(), tx.clone(), |_: Completion| 8),
            Err(neocogi_com::NC_E_ALREADY_BOUND)
        );
        assert!(source.query::<EditorSinkAbi>().is_err());
        source.emit().unwrap();
        assert_eq!(rx.try_recv().unwrap(), 1, "failed binding must leave the old destination intact");
        assert_eq!(neocogi_com::events::bind_stream(stream.as_com_ref(), tx, |_: Completion| 2), Err(neocogi_com::NC_E_ALREADY_BOUND));
        drop(opaque);
        // Dropping an extra source handle cannot detach a live source's receiver.
        source.emit().unwrap();
        assert_eq!(rx.try_recv().unwrap(), 1);
        drop(rx);
        source.emit().unwrap();
    }
}
#[cfg(test)]
mod class_sink_tests {
    use super::*;
    use neocogi_com::{com_object, ComMessage, ComPtr, NcStatus};
    use neocogi_com::events::MessageSinkInterface;

    /// Producer starts unbound and retains exactly the receiver attached by its parent.
    #[neocogi_com::com_class(Editor, event = events)]
    struct EditorValue {
        /// One destination handles both enum variants for the producer's whole lifetime.
        events: EditorEventPort,
    }
    impl Editor for EditorValue {
        /// Two facts from one update preserve their order across the COM boundary.
        fn emit(&mut self) -> Result<(), NcStatus> {
            self.events.post(&EditorEvent::Changed(TextChanged { text: "hello".into(), cursor: 5 }))?;
            self.events.post(&EditorEvent::Completed)
        }
    }

    /// Construction itself is a COM call and returns an unbound stream.
    #[com_object(Editors)]
    struct EditorFactory;
    impl Editors for EditorFactory {
        /// Returns an initialized object whose parent can attach its sole consumer.
        fn create(&self) -> Result<ComPtr<EditorAbi>, NcStatus> {
            EditorValue::from_value(EditorValue {
                events: Default::default(),
            })
            .query()
        }
    }

    /// Sink retention, variant ordering, unbound posting and capture release use actual thunks.
    #[test]
    fn one_sink_owns_both_variants_until_the_producer_drops() {
        let factory: ComPtr<EditorsAbi> = EditorFactory::from_value(EditorFactory).query().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let sink = EditorSinkAbi::receiver(move |event| {
            let _ = sender.send(event);
            Ok(())
        })
        .unwrap();
        let mut editor = factory.create().unwrap();
        neocogi_com::events::attach_source(editor.as_com_ref(), sink.as_com_ref()).unwrap();
        drop(sink);
        editor.emit().unwrap();
        assert!(matches!(receiver.try_recv().unwrap(), EditorEvent::Changed(TextChanged { text, cursor: 5 }) if text == "hello"));
        assert!(matches!(receiver.try_recv().unwrap(), EditorEvent::Completed));
        drop(editor);
        assert!(matches!(receiver.try_recv(), Err(std::sync::mpsc::TryRecvError::Disconnected)));
        // An unbound preview emits without allocating a dummy sink.
        factory.create().unwrap().emit().unwrap();
    }

    /// A malformed wire message never invokes the generated local callback.
    #[test]
    fn sink_checks_payload_before_mapping() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut sink = EditorSinkAbi::receiver(move |event| {
            sender.send(event).unwrap();
            Ok(())
        })
        .unwrap();
        let value = EditorEvent::Changed(TextChanged { text: String::new(), cursor: 0 });
        let mut wire = value.encode();
        let invalid_utf8 = [0xff];
        wire.payload.case_0.field_0.payload.case_0.text = neocogi_com::NcMessageSlice::new(&invalid_utf8);
        // SAFETY: The wire and its backing bytes remain live for the call. The
        // deliberately invalid text tests the boundary's checked UTF-8 decoder.
        let input = unsafe { neocogi_com::NcMessage::from_raw(&wire) }.unwrap();
        assert_eq!(sink.post(input), Err(neocogi_com::NC_E_INVALID_ARG));
        assert!(receiver.try_recv().is_err());
    }

    /// A nested borrowed wire copies its text exactly once and rejects malformed
    /// nested UTF-8 before any caller can retain a partially decoded envelope.
    #[test]
    fn nested_struct_messages_decode_owned_fields_recursively() {
        let source = TextEnvelope { changed: TextChanged { text: "nested".into(), cursor: 6 } };
        let wire = source.encode();
        // SAFETY: The source message owns every borrowed byte until decoding ends.
        let acquired = unsafe { TextEnvelope::decode(wire) }.unwrap();
        drop(source);
        assert_eq!(acquired.changed.text, "nested");
        assert_eq!(acquired.changed.cursor, 6);
        let source = TextEnvelope { changed: TextChanged { text: String::new(), cursor: 0 } };
        let mut wire = source.encode();
        let invalid_utf8 = [0xff];
        wire.payload.case_0.changed.payload.case_0.text = neocogi_com::NcMessageSlice::new(&invalid_utf8);
        // SAFETY: The malformed borrowed bytes remain live for the checked decode.
        assert!(matches!(unsafe { TextEnvelope::decode(wire) }, Err(neocogi_com::NC_E_INVALID_ARG)));
    }

    /// Cloning a concrete handle never creates another attachment slot.
    #[test]
    fn cloned_handles_cannot_rebind_the_widget() {
        let factory: ComPtr<EditorsAbi> = EditorFactory::from_value(EditorFactory).query().unwrap();
        let mut editor = factory.create().unwrap();
        let exports = editor.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        neocogi_com::events::bind_source(exports.as_com_ref(), sender.clone(), |value: EditorEvent| match value { EditorEvent::Changed(TextChanged { text, .. }) => text, EditorEvent::Completed => "completed".into() }).unwrap();
        assert!(exports.query::<IProducerStreamAbi>().is_err());
        assert_eq!(neocogi_com::events::bind_source(exports.as_com_ref(), sender.clone(), |_: EditorEvent| String::new()), Err(neocogi_com::NC_E_ALREADY_BOUND));
        editor.emit().unwrap();
        assert_eq!(receiver.try_recv().unwrap(), "hello");
        // A failed bind left the destination intact for both event variants.
        assert_eq!(receiver.try_recv().unwrap(), "completed");
        assert_eq!(neocogi_com::events::bind_source(exports.as_com_ref(), sender, |value: EditorEvent| match value { EditorEvent::Changed(TextChanged { text, .. }) => text.to_uppercase(), EditorEvent::Completed => "COMPLETED".into() }), Err(neocogi_com::NC_E_ALREADY_BOUND));
        editor.emit().unwrap();
        assert_eq!(receiver.try_recv().unwrap(), "hello");
        assert_eq!(receiver.try_recv().unwrap(), "completed");
        drop(exports);
        editor.emit().unwrap();
        assert_eq!(receiver.try_recv().unwrap(), "hello");
        assert_eq!(receiver.try_recv().unwrap(), "completed");
    }
}
