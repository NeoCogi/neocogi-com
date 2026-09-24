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
        completed: Option<ComPtr<__CompletionReceiverAbi>>,
    }
    impl IProducer for Producer {
        /// Allows tests to deliver after binding has finished.
        fn emit(&mut self) -> Result<(), NcStatus> {
            Completion::Cancelled.post_to(&mut self.completed)
        }

        /// Ordinary COM mutation installs the only destination, replacing the old one.
        fn __map_completed(&mut self, destination: neocogi_com::ComRef<'_, __CompletionReceiverAbi>) -> Result<(), NcStatus> {
            self.completed = Some(destination.add_ref());
            // Emit during setup to exercise the actual producer/receiver ABI end to end.
            Completion::Cancelled.post_to(&mut self.completed)
        }
    }

    /// Mapping replaces destinations; setup and producer destruction cannot retract queued facts.
    #[test]
    fn generated_output_installs_a_direct_receiver() {
        let producer: ComPtr<IProducerAbi> = Producer::from_value(Producer { completed: None }).query().unwrap();
        let (first_tx, first) = std::sync::mpsc::channel();
        neocogi_com::events::bind(producer.completed(), first_tx.clone(), |_| 1).unwrap();
        let (second_tx, second) = std::sync::mpsc::channel();
        neocogi_com::events::bind(producer.completed(), second_tx.clone(), |_| 2).unwrap();
        drop(producer);
        assert_eq!(first.try_recv().ok(), Some(1));
        assert_eq!(second.try_recv().ok(), Some(2));
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
    }
}

#[cfg(test)]
mod named_output_tests {
    use super::*;
    use neocogi_com::{com_object, ComPtr, ComRef, NcStatus};
    use neocogi_com::events::{NamedOutputs, bind_output};

    /// An independent producer that emits only when explicitly requested.
    #[com_object(IProducer)]
    struct Source {
        /// Exactly one concrete destination; the named table is not on this path.
        destination: Option<ComPtr<__CompletionReceiverAbi>>,
    }
    impl IProducer for Source {
        /// Replaces the destination after generated binding has validated its type.
        fn __map_completed(&mut self, receiver: ComRef<'_, __CompletionReceiverAbi>) -> Result<(), NcStatus> {
            self.destination = Some(receiver.add_ref());
            Ok(())
        }
        /// Publishes directly to the installed generated receiver.
        fn emit(&mut self) -> Result<(), NcStatus> {
            Completion::Cancelled.post_to(&mut self.destination)
        }
    }

    /// Applications see only names and payload contracts, never the private producer.
    #[test]
    fn opaque_named_binding_preserves_type_checks_replacement_and_lifetimes() {
        let mut source: ComPtr<IProducerAbi> = Source::from_value(Source { destination: None }).query().unwrap();
        let mut names = NamedOutputs::new();
        assert!(names.export("bad", source.clone(), "missing").is_err());
        names.export("finished", source.clone(), "completed").unwrap();
        assert!(names.export("finished", source.clone(), "completed").is_err());
        let opaque = names.into_com().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        bind_output(opaque.as_com_ref(), "finished", tx.clone(), |_: Completion| 1_u32).unwrap();
        assert_eq!(
            bind_output(opaque.as_com_ref(), "missing", tx.clone(), |_: Completion| 8),
            Err(neocogi_com::NC_E_NOT_FOUND)
        );
        assert!(bind_output(opaque.as_com_ref(), "finished", tx.clone(), |_: NumberChanged| 9).is_err());
        source.emit().unwrap();
        assert_eq!(rx.try_recv().unwrap(), 1, "failed binding must leave the old destination intact");
        bind_output(opaque.as_com_ref(), "finished", tx, |_: Completion| 2).unwrap();
        drop(opaque);
        // Binding-table destruction cannot break a receiver already installed on a live source.
        source.emit().unwrap();
        assert_eq!(rx.try_recv().unwrap(), 2);
        drop(rx);
        source.emit().unwrap();
    }
}
#[cfg(test)]
mod observer_tests {
    use super::*;
    use neocogi_com::{com_object, ComPtr, ComRef, NcStatus};

    /// Producer retains exactly the optional destination supplied by its factory.
    #[com_object(Editor)]
    struct EditorValue {
        /// One destination handles both outputs for the producer's whole lifetime.
        observer: Option<ComPtr<EditorObserverAbi>>,
    }
    impl Editor for EditorValue {
        /// Two facts from one update preserve their order across the COM boundary.
        fn emit(&mut self) -> Result<(), NcStatus> {
            EditorEvent::Changed { text: "hello".into(), cursor: 5 }.post_to_observer(&mut self.observer)?;
            EditorEvent::Completed.post_to_observer(&mut self.observer)
        }
    }

    /// Construction itself is a COM call, including the nullable observer input.
    #[com_object(Editors)]
    struct EditorFactory;
    impl Editors for EditorFactory {
        /// Retains the borrowed observer before publishing the initialized object.
        fn create(&self, observer: Option<ComRef<'_, EditorObserverAbi>>) -> Result<ComPtr<EditorAbi>, NcStatus> {
            EditorValue::from_value(EditorValue {
                observer: observer.map(|value| value.add_ref()),
            })
            .query()
        }
    }

    /// Constructor retention, ordering, absence and capture release use actual thunks.
    #[test]
    fn one_observer_owns_both_outputs_until_the_producer_drops() {
        let factory: ComPtr<EditorsAbi> = EditorFactory::from_value(EditorFactory).query().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let observer = EditorEvent::observer(move |event| {
            let _ = sender.send(event);
            Ok(())
        })
        .unwrap();
        let mut editor = factory.create(Some(observer.as_com_ref())).unwrap();
        drop(observer);
        editor.emit().unwrap();
        assert!(matches!(receiver.try_recv().unwrap(), EditorEvent::Changed { text, cursor: 5 } if text == "hello"));
        assert!(matches!(receiver.try_recv().unwrap(), EditorEvent::Completed));
        drop(editor);
        assert!(matches!(receiver.try_recv(), Err(std::sync::mpsc::TryRecvError::Disconnected)));
        // No dummy observer is needed when the destination is absent.
        factory.create(None).unwrap().emit().unwrap();
    }

    /// A malformed wire message never invokes the generated local callback.
    #[test]
    fn observer_checks_payload_before_mapping() {
        use neocogi_com::ComMessage;
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut observer = EditorEvent::observer(move |event| {
            sender.send(event).unwrap();
            Ok(())
        })
        .unwrap();
        let value = EditorEvent::Changed { text: String::new(), cursor: 0 };
        let mut wire = value.encode();
        let invalid_utf8 = [0xff];
        wire.payload.case_0.text = neocogi_com::NcMessageSlice::new(&invalid_utf8);
        // SAFETY: The wire and its backing bytes remain live for the call. The
        // deliberately invalid text tests the boundary's checked UTF-8 decoder.
        let input = unsafe { neocogi_com::NcMessage::from_raw(&wire) }.unwrap();
        assert_eq!(observer.event(input), Err(neocogi_com::NC_E_INVALID_ARG));
        assert!(receiver.try_recv().is_err());
    }

    /// Public export binding changes only the component destination, while the
    /// widget keeps the one observer supplied by its constructor.
    #[test]
    fn named_exports_forward_without_rebinding_the_widget() {
        let factory: ComPtr<EditorsAbi> = EditorFactory::from_value(EditorFactory).query().unwrap();
        let (observer, exports) = EditorEvent::exported_observer().unwrap();
        let mut editor = factory.create(Some(observer.as_com_ref())).unwrap();
        drop(observer);
        let (sender, receiver) = std::sync::mpsc::channel();
        neocogi_com::events::bind_output(exports.as_com_ref(), "event", sender.clone(), |value: EditorEvent| match value { EditorEvent::Changed { text, .. } => text, EditorEvent::Completed => "completed".into() }).unwrap();
        assert!(neocogi_com::events::bind_output(exports.as_com_ref(), "event", sender.clone(), |_: Completion| String::new()).is_err());
        assert!(neocogi_com::events::bind_output(exports.as_com_ref(), "absent", sender.clone(), |_: EditorEvent| String::new()).is_err());
        editor.emit().unwrap();
        assert_eq!(receiver.try_recv().unwrap(), "hello");
        // A failed bind left the destination intact for both event variants.
        assert_eq!(receiver.try_recv().unwrap(), "completed");
        neocogi_com::events::bind_output(exports.as_com_ref(), "event", sender, |value: EditorEvent| match value { EditorEvent::Changed { text, .. } => text.to_uppercase(), EditorEvent::Completed => "COMPLETED".into() }).unwrap();
        editor.emit().unwrap();
        assert_eq!(receiver.try_recv().unwrap(), "HELLO");
        assert_eq!(receiver.try_recv().unwrap(), "COMPLETED");
        drop(exports);
        editor.emit().unwrap();
        assert_eq!(receiver.try_recv().unwrap(), "HELLO");
        assert_eq!(receiver.try_recv().unwrap(), "COMPLETED");
    }
}
