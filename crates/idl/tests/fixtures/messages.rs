
#[cfg(test)]
mod message_tests {
    use super::*;
    use neocogi_com::{com_object, ComMessage, ComPtr, NcMessage, NcStatus};

    #[com_object(interfaces(IToken))]
    /// Object lifetime is observable after all producer-side references are dropped.
    struct Token;
    impl IToken for Token {
        /// Reads the retained capability after producer ownership has ended.
        fn value(&self) -> Result<u32, NcStatus> { Ok(42) }
    }

    #[com_object(IProducer)]
    /// A producer owns its destination directly and implements an ordinary setter.
    struct Producer {
        /// One optional concrete receiver is all the producer-side delivery storage.
        completed: Option<ComPtr<__CompletionReceiverAbi>>,
    }
    impl IProducer for Producer {
        /// Allows tests to deliver after binding has finished.
        fn emit(&mut self) -> Result<(), NcStatus> { Completion::Cancelled.post_to(&mut self.completed) }

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
        neocogi_com_events::map(producer.completed(), first_tx.clone(), |_| 1).unwrap();
        let (second_tx, second) = std::sync::mpsc::channel();
        neocogi_com_events::map(producer.completed(), second_tx.clone(), |_| 2).unwrap();
        drop(producer);
        assert_eq!(first.try_recv().ok(), Some(1));
        assert_eq!(second.try_recv().ok(), Some(2));
    }

    /// Producer release cannot destroy consumer-owned object or text snapshots.
    #[test]
    fn typed_delivery_retains_objects_and_copies_strings() {
        let mut source = None;
        let (events_tx, events) = std::sync::mpsc::channel();
        neocogi_com_events::map(|consume| { source = Some(Completion::__receiver(consume)?); Ok(()) }, events_tx.clone(), |message| message).unwrap();
        let token: ComPtr<ITokenAbi> = Token::from_value(Token).query().unwrap();
        Completion::Accepted { token }.post_to(&mut source).unwrap();
        drop(source);
        match events.try_recv().unwrap() {
            Completion::Accepted { token } => assert_eq!(token.value(), Ok(42)),
            Completion::Cancelled => panic!("wrong case"),
        }
        let mut source = None;
        let (events_tx, events) = std::sync::mpsc::channel();
        neocogi_com_events::map(|consume| { source = Some(TextChanged::__receiver(consume)?); Ok(()) }, events_tx.clone(), |message| message).unwrap();
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
        neocogi_com_events::map(|consume| { destination = Some(Notice::__receiver(consume)?); Ok(()) }, events_tx.clone(), |message| message).unwrap();
        Notice::Done.post_to(&mut destination).unwrap();
        assert_eq!(events.try_recv().ok(), Some(Notice::Done));
        let mut destination = None;
        let (events_tx, events) = std::sync::mpsc::channel();
        neocogi_com_events::map(|consume| { destination = Some(Batch::__receiver(consume)?); Ok(()) }, events_tx.clone(), |message| message.values).unwrap();
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
        assert_eq!(receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()), Err(neocogi_com::NC_E_INVALID_TAG));
        wire = message.encode();
        wire.payload.case_0.checked = 3;
        assert_eq!(receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()), Err(neocogi_com::NC_E_INVALID_ARG));
        assert!(events.try_recv().is_err());
        let (text_tx, text) = std::sync::mpsc::channel();
        let sender = text_tx.clone();
        let mut receiver = TextChanged::__receiver(Box::new(move |message| sender.send(message).map_err(|_| neocogi_com::NC_E_INVALID_ARG))).unwrap();
        let bad = [0xff];
        let message = TextChanged { text: String::new(), cursor: 0 };
        let mut wire = message.encode();
        wire.payload.case_0.text = neocogi_com::NcMessageSlice::new(&bad);
        assert_eq!(receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()), Err(neocogi_com::NC_E_INVALID_ARG));
        assert!(text.try_recv().is_err());
    }
}

#[cfg(test)]
mod named_output_tests {
    use super::*;
    use neocogi_com::{com_object, ComPtr, ComRef, NcStatus};
    use neocogi_com_events::{NamedOutputs, bind_output};

    /// An independent producer that emits only when explicitly requested.
    #[com_object(IProducer)]
    struct Source {
        /// Exactly one concrete destination; the named table is not on this path.
        destination: Option<ComPtr<__CompletionReceiverAbi>>,
    }
    impl IProducer for Source {
        /// Replaces the destination after generated binding has validated its type.
        fn __map_completed(&mut self, receiver: ComRef<'_, __CompletionReceiverAbi>) -> Result<(), NcStatus> {
            self.destination = Some(receiver.add_ref()); Ok(())
        }
        /// Publishes directly to the installed generated receiver.
        fn emit(&mut self) -> Result<(), NcStatus> { Completion::Cancelled.post_to(&mut self.destination) }
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
        assert_eq!(bind_output(opaque.as_com_ref(), "missing", tx.clone(), |_: Completion| 8), Err(neocogi_com::NC_E_NOT_FOUND));
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
