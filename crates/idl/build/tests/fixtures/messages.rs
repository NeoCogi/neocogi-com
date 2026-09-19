
#[cfg(test)]
mod message_tests {
    use super::*;
    use neocogi_com::{com_object, ComMessage, ComPtr, NcMessage, NcStatus};
    use neocogi_com_events::{Events, Port};

    #[com_object(interfaces(IToken))]
    /// Object lifetime is observable after all producer-side references are dropped.
    struct Token;
    impl IToken for Token {
        /// Reads the retained capability after producer ownership has ended.
        fn value(&self) -> Result<u32, NcStatus> { Ok(42) }
    }

    #[com_object(IProducer, ports(completed))]
    /// A real object binds its output field without any handwritten subscription method.
    struct Producer {
        /// The only producer-side storage is its typed port.
        completed: Port<Completion>,
    }
    impl IProducer for Producer {}

    /// Field binding returns only connection access and preserves its producer lifetime rule.
    #[test]
    fn generated_output_binds_the_real_port() {
        let producer: ComPtr<IProducerAbi> = Producer::from_value(Producer { completed: Port::new() }).query().unwrap();
        let mut events = Events::new();
        let first = events.map(producer.completed(), |message| message).unwrap();
        let mut second = Events::new();
        assert_eq!(second.map(producer.completed(), |message| message).unwrap_err(), neocogi_com_events::EVENT_E_BUSY);
        first.cancel().unwrap();
        let replacement = second.map(producer.completed(), |message| message).unwrap();
        first.cancel().unwrap();
        assert!(replacement.active().unwrap());
        let retained = producer.completed();
        drop(producer);
        assert!(!replacement.active().unwrap());
        assert_eq!(events.map(retained, |message| message).unwrap_err(), neocogi_com_events::EVENT_E_CLOSED);
    }

    /// Producer release cannot destroy consumer-owned object or text snapshots.
    #[test]
    fn typed_delivery_retains_objects_and_copies_strings() {
        let source = Port::<Completion>::new();
        let mut events = Events::new();
        events.map(source.as_ref(), |message| message).unwrap();
        let token: ComPtr<ITokenAbi> = Token::from_value(Token).query().unwrap();
        source.emit(Completion::Accepted { token }).unwrap();
        drop(source);
        match events.pop().unwrap() {
            Completion::Accepted { token } => assert_eq!(token.value(), Ok(42)),
            Completion::Cancelled => panic!("wrong case"),
        }
        let source = Port::<TextChanged>::new();
        let mut events = Events::new();
        events.map(source.as_ref(), |message| message).unwrap();
        source.emit(TextChanged { text: "héllo".into(), cursor: 2 }).unwrap();
        drop(source);
        assert_eq!(events.pop().unwrap().text, "héllo");
    }

    /// Invalid wire values fail before the mapping closure or queue is reached.
    #[test]
    fn malformed_payloads_never_reach_the_mapper() {
        let mut events = Events::new();
        let mut receiver = __map_number_changed(events.sender(), |message| message, None).unwrap();
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
        let mut receiver = __map_text_changed(text.sender(), |message| message, None).unwrap();
        let bad = [0xff];
        let message = TextChanged { text: String::new(), cursor: 0 };
        let mut wire = message.encode();
        wire.payload.case_0.text = neocogi_com::NcMessageSlice::new(&bad);
        assert_eq!(receiver.post(unsafe { NcMessage::from_raw(&wire) }.unwrap()), Err(neocogi_com::NC_E_INVALID_ARG));
        assert!(text.pop().is_none());
    }
}
