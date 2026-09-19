
#[cfg(test)]
mod message_tests {
    use super::*;
    use neocogi_com::{com_object, ComMessage, ComPtr, NcMessage, NcStatus};
    use neocogi_com_events::{Events};

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
        let mut first = Events::new();
        first.map(producer.completed(), |_| 1).unwrap();
        let mut second = Events::new();
        second.map(producer.completed(), |_| 2).unwrap();
        drop(producer);
        assert_eq!(first.pop(), Some(1));
        assert_eq!(second.pop(), Some(2));
    }

    /// Producer release cannot destroy consumer-owned object or text snapshots.
    #[test]
    fn typed_delivery_retains_objects_and_copies_strings() {
        let mut source = None;
        let mut events = Events::new();
        events.map(|consume| { source = Some(Completion::__receiver(consume)?); Ok(()) }, |message| message).unwrap();
        let token: ComPtr<ITokenAbi> = Token::from_value(Token).query().unwrap();
        Completion::Accepted { token }.post_to(&mut source).unwrap();
        drop(source);
        match events.pop().unwrap() {
            Completion::Accepted { token } => assert_eq!(token.value(), Ok(42)),
            Completion::Cancelled => panic!("wrong case"),
        }
        let mut source = None;
        let mut events = Events::new();
        events.map(|consume| { source = Some(TextChanged::__receiver(consume)?); Ok(()) }, |message| message).unwrap();
        TextChanged { text: "héllo".into(), cursor: 2 }.post_to(&mut source).unwrap();
        drop(source);
        assert_eq!(events.pop().unwrap().text, "héllo");
    }

    /// Unit facts retain their checked copied ABI and slices become independent consumer storage.
    #[test]
    fn unit_messages_and_owned_slices_round_trip() {
        assert_eq!(Notice::try_from(NoticeAbi::from(Notice::Ready)).unwrap(), Notice::Ready);
        let mut port = None;
        let mut events = Events::new();
        events.map(|consume| { port = Some(Notice::__receiver(consume)?); Ok(()) }, |message| message).unwrap();
        Notice::Done.post_to(&mut port).unwrap();
        assert_eq!(events.pop(), Some(Notice::Done));
        let mut port = None;
        let mut events = Events::new();
        events.map(|consume| { port = Some(Batch::__receiver(consume)?); Ok(()) }, |message| message.values).unwrap();
        Batch { values: vec![1, 2, 3] }.post_to(&mut port).unwrap();
        drop(port);
        assert_eq!(events.pop(), Some(vec![1, 2, 3]));
        let value = Batch { values: vec![1] };
        let mut wire = value.encode();
        wire.payload.case_0.values.len = u64::MAX;
        // The decoder must reject the impossible range before reading any pointed-to element.
        assert!(unsafe { NcMessage::<Batch>::from_raw(&wire) }.unwrap().to_owned().is_err());
    }

    /// Invalid wire values fail before the mapping closure or queue is reached.
    #[test]
    fn malformed_payloads_never_reach_the_mapper() {
        let mut events = Events::new();
        let sender = events.sender();
        let mut receiver = NumberChanged::__receiver(Box::new(move |message| sender.post(message))).unwrap();
        let message = NumberChanged { value: 12.5, checked: true };
        receiver.post(NcMessage::new(&message)).unwrap();
        assert_eq!(events.pop().unwrap(), message);
        let mut wire = message.encode();
        wire.tag = 99;
        assert_eq!(receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()), Err(neocogi_com::NC_E_INVALID_TAG));
        wire = message.encode();
        wire.payload.case_0.checked = 3;
        assert_eq!(receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()), Err(neocogi_com::NC_E_INVALID_ARG));
        assert!(events.is_empty());
        let mut text = Events::new();
        let sender = text.sender();
        let mut receiver = TextChanged::__receiver(Box::new(move |message| sender.post(message))).unwrap();
        let bad = [0xff];
        let message = TextChanged { text: String::new(), cursor: 0 };
        let mut wire = message.encode();
        wire.payload.case_0.text = neocogi_com::NcMessageSlice::new(&bad);
        assert_eq!(receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()), Err(neocogi_com::NC_E_INVALID_ARG));
        assert!(text.pop().is_none());
    }
}
