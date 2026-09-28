//! The generated source is compiled as a downstream application, not its producer.
#[allow(dead_code)]
mod absolute;
#[allow(dead_code)]
mod plain;

#[cfg(test)]
mod tests {
    use super::*;
    use path_contracts::r#type::{CounterChanged, CounterEvent};

    /// Application behavior owns its mutable state independently of COM metadata.
    #[derive(Default)]
    struct Handler {
        /// Last count observed through the generated required event handler.
        count: u32,
    }
    impl plain::Handlers for Handler {
        /// The compiler checks that generated binding metadata names the actual event.
        fn on_counter_0(&mut self, event: CounterEvent) -> Result<(), neocogi_com::NcStatus> {
            self.count = match event {
                CounterEvent::Changed(value) => value.value,
                CounterEvent::Reset => 0,
            };
            Ok(())
        }
    }
    impl absolute::Handlers for Handler {
        /// Optional leading `::` must preserve exactly the same application type.
        fn on_counter_0(&mut self, event: CounterEvent) -> Result<(), neocogi_com::NcStatus> {
            plain::Handlers::on_counter_0(self, event)
        }
    }

    /// Compiled form events retain the concrete generated payload and mutable dispatch.
    #[test]
    fn raw_and_absolute_bindings_dispatch_the_same_generated_type() {
        let mut handler = Handler::default();
        plain::Event::Widget0(CounterEvent::Changed(CounterChanged { value: 42 }))
            .dispatch(&mut handler)
            .unwrap();
        assert_eq!(handler.count, 42);
        absolute::Event::Widget0(CounterEvent::Reset).dispatch(&mut handler).unwrap();
        assert_eq!(handler.count, 0);
    }
}
