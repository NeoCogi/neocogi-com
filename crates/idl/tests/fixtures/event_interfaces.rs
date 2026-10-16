#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com::*;
    use neocogi_com::events::{EventMessage, EventOrigin, IEvent, IEventAbi, IEventReceiver, IEventReceiverAbi};
    use std::{cell::Cell, rc::Rc, sync::mpsc};

    #[com_object(Widget, ownership = exclusive)]
    /// Concrete widget lifetime is independent of queued snapshots.
    struct WidgetObject {
        drops: Rc<Cell<u32>>,
    }
    impl Widget for WidgetObject {
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(9)
        }
    }
    impl Drop for WidgetObject {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    #[com_object(Service)]
    /// Shared service can expire while its queued weak identity remains available.
    struct ServiceObject;
    impl Service for ServiceObject {
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(12)
        }
    }

    #[com_object(IChanged, ownership = exclusive)]
    /// Immutable event allocation exposes a concrete IDL getter.
    struct Changed {
        value: u32,
        drops: Rc<Cell<u32>>,
    }
    impl IEvent for Changed {}
    impl IChanged for Changed {
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(self.value)
        }
    }
    impl Drop for Changed {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    #[com_object(interfaces(neocogi_com::events::IEventReceiver))]
    /// This one receiver accepts every event interface and both origin kinds.
    struct Receiver {
        send: mpsc::Sender<EventMessage>,
    }
    impl IEventReceiver for Receiver {
        fn post(&mut self, origin: NcValueRef<'_, EventOrigin>, event: ComBox<IEventAbi>) -> Result<(), (NcStatus, ComBox<IEventAbi>)> {
            self.send
                .send(EventMessage::capture(origin, event)?)
                .map_err(|error| (NC_E_CLOSED, error.0.event))
        }
    }

    #[com_object(IOrigins)]
    /// Exercises recursive weak fields through a real borrowed input and owned result.
    struct OriginsObject {
        values: Origins,
    }
    impl IOrigins for OriginsObject {
        fn capture(&mut self, origins: NcValueRef<'_, Origins>) -> Result<(), NcStatus> {
            self.values = origins.to_owned()?;
            Ok(())
        }
        fn value(&self) -> Result<WeakComBox<WidgetAbi>, NcStatus> {
            Ok(self.values.one.clone())
        }
    }

    /// The producer dies immediately; immutable events live until the last queued owner drops.
    #[test]
    fn queued_events_survive_both_origin_kinds_and_failed_downcasts_return_none() {
        let widget_drops = Rc::new(Cell::new(0));
        let event_drops = Rc::new(Cell::new(0));
        let widget = WidgetObject::into_local_box(WidgetObject { drops: widget_drops.clone() });
        let weak = widget.query::<WidgetAbi>().unwrap();
        let service = ServiceObject::into_local(ServiceObject);
        let service_weak = service.query::<ServiceAbi>().unwrap().downgrade().unwrap();
        let (send, receive) = mpsc::channel();
        let mut queue = Receiver::from_value(Receiver { send }).query::<IEventReceiverAbi>().unwrap();
        let origins = [
            EventOrigin::Widget(weak.clone().into_unknown()),
            EventOrigin::Service(service_weak.clone().into_unknown()),
        ];
        for origin in &origins {
            let event = Changed::into_box(Changed { value: 42, drops: event_drops.clone() })
                .query::<IEventAbi>()
                .unwrap();
            queue.post(NcValueRef::new(origin).unwrap(), event).unwrap();
        }
        drop(widget);
        drop(service);
        assert_eq!(widget_drops.get(), 1);
        assert_eq!(event_drops.get(), 0);
        assert!(!weak.is_alive());
        assert!(service_weak.upgrade().unwrap().is_none());
        let first = receive.try_recv().unwrap();
        assert!(!first.origin.widget::<WidgetAbi>().unwrap().unwrap().is_alive());
        assert!(first.origin.service::<ServiceAbi>().unwrap().is_none());
        assert!(first.origin.widget::<ServiceAbi>().unwrap().is_none());
        assert!(first.downcast::<ISubmittedAbi>().unwrap().is_none());
        assert_eq!(first.downcast::<IChangedAbi>().unwrap().unwrap().value(), Ok(42));
        let second = receive.try_recv().unwrap();
        assert!(second.origin.service::<ServiceAbi>().unwrap().unwrap().upgrade().unwrap().is_none());
        assert!(second.origin.widget::<WidgetAbi>().unwrap().is_none());
        assert_eq!(second.downcast::<IChangedAbi>().unwrap().unwrap().value(), Ok(42));
        drop(first);
        assert_eq!(event_drops.get(), 1);
        let projected = second.downcast::<IChangedAbi>().unwrap().unwrap();
        drop(second);
        assert_eq!(event_drops.get(), 2);
        assert_eq!(projected.value(), Err(NC_E_DROPPED));
    }

    /// Nested exclusive observers retain their exact interface and never own the widget.
    #[test]
    fn weak_box_fields_round_trip_after_removal() {
        let drops = Rc::new(Cell::new(0));
        let widget = WidgetObject::into_local_box(WidgetObject { drops: drops.clone() });
        let weak = widget.query::<WidgetAbi>().unwrap();
        let values = Origins {
            one: weak.clone(),
            many: vec![weak.clone()],
            fixed: [weak.clone(), weak.clone()],
        };
        let mut store = OriginsObject::from_value(OriginsObject { values: values.clone() })
            .query::<IOriginsAbi>()
            .unwrap();
        store.capture(NcValueRef::new(&values).unwrap()).unwrap();
        assert_eq!(store.value().unwrap().value(), Ok(9));
        drop(widget);
        assert_eq!(drops.get(), 1);
        store.capture(NcValueRef::new(&values).unwrap()).unwrap();
        assert!(!store.value().unwrap().is_alive());
        let result = NcValueRef::new(&values).unwrap().to_owned().unwrap();
        assert!(!result.one.is_alive());
        assert!(result.many[0].same_object(&weak));
        assert!(result.fixed[1].same_object(&weak));
        let neocogi_com::design::IdlShape::Struct(fields) = Origins::IDL_TYPE.shape else {
            panic!("record")
        };
        assert_eq!(fields[0].ty, neocogi_com::design::IdlValueType::WeakBoxObject("proof.events.Widget"));
    }

    /// Valid raw backing can still name the wrong ownership mode, type, or null shape.
    #[test]
    fn malformed_weak_fields_reject_before_acquiring_an_observer() {
        let shared = ServiceObject::from_value(ServiceObject).query::<IUnknown>().unwrap();
        // SAFETY: shared owns a readable common header/table; its ownership kind is intentionally wrong.
        let result = unsafe { WeakComBox::<IUnknown>::decode_value(shared.as_raw(), &mut ValueBudget::default()) };
        assert_eq!(result.unwrap_err(), NC_E_INVALID_ARG);
        // SAFETY: null is a valid initialized wire shape for rejection.
        assert_eq!(
            unsafe { WeakComBox::<IUnknown>::decode_value(ComRaw::null(), &mut ValueBudget::default()) }.unwrap_err(),
            NC_E_POINTER
        );
        let drops = Rc::new(Cell::new(0));
        let widget = WidgetObject::into_local_box(WidgetObject { drops: drops.clone() });
        let weak = widget.query::<WidgetAbi>().unwrap();
        let wrong = ComRaw::<ServiceAbi> {
            object: weak.as_raw().object,
            vtable: weak.as_raw().vtable.cast(),
        };
        // SAFETY: the supplied table has a readable common prefix and deliberately different identity.
        assert_eq!(
            unsafe { WeakComBox::<ServiceAbi>::decode_value(wrong, &mut ValueBudget::default()) }.unwrap_err(),
            NC_E_NOINTERFACE
        );
        // SAFETY: initialized backing is valid but the decoder has no remaining value budget.
        assert_eq!(
            unsafe { WeakComBox::<WidgetAbi>::decode_value(weak.as_raw(), &mut ValueBudget::new(0, 0, 0)) }.unwrap_err(),
            NC_E_OUT_OF_RANGE
        );
        drop(widget);
        assert_eq!(drops.get(), 1);
    }

    /// A provider error cannot be mistaken for an unsupported event type.
    #[test]
    fn downcast_preserves_real_com_errors() {
        struct WrongRevision;
        // SAFETY: this marker has the same vtable shape; its different signature
        // deliberately requests an unsupported revision without invoking methods.
        unsafe impl ComInterface for WrongRevision {
            const INTERFACE_ID: NcInterfaceId = NcInterfaceId {
                iid: IChangedAbi::INTERFACE_ID.iid,
                abi_signature: ISubmittedAbi::INTERFACE_ID.abi_signature,
            };
            type VTable = <IChangedAbi as ComInterface>::VTable;
            type Threading = ThreadLocal;
        }
        let event = Changed::into_box(Changed { value: 42, drops: Rc::new(Cell::new(0)) })
            .query::<IEventAbi>()
            .unwrap();
        assert_eq!(event.query_weak::<WrongRevision>().unwrap_err(), NC_E_ABI_VERSION);
        assert_eq!(event.query_weak::<ISubmittedAbi>().unwrap_err(), NC_E_NOINTERFACE);
        let service = ServiceObject::into_local(ServiceObject);
        let message = EventMessage {
            origin: EventOrigin::Service(service.query::<ServiceAbi>().unwrap().downgrade().unwrap().into_unknown()),
            event,
        };
        assert_eq!(message.downcast::<WrongRevision>().unwrap_err(), NC_E_ABI_VERSION);
    }

    /// A rejected transfer preserves the exact event owner and its readable snapshot.
    #[test]
    fn invalid_origin_and_disconnected_receiver_return_original_owner() {
        let drops = Rc::new(Cell::new(0));
        let event = Changed::into_box(Changed { value: 77, drops: drops.clone() }).query::<IEventAbi>().unwrap();
        let observed = event.query_weak::<IChangedAbi>().unwrap();
        let identity = event.as_raw().object;
        let service = ServiceObject::into_local(ServiceObject);
        let origin = EventOrigin::Service(service.query::<ServiceAbi>().unwrap().downgrade().unwrap().into_unknown());
        let (send, receive) = mpsc::channel();
        let mut queue = Receiver::from_value(Receiver { send }).query::<IEventReceiverAbi>().unwrap();
        let encoded = NcValueRef::new(&origin).unwrap();
        // SAFETY: clone initialized wire storage while its borrowed backing remains live.
        let mut raw = unsafe { encoded.as_ptr().read() };
        raw.tag = i64::MAX;
        // SAFETY: only the tag is invalid; every backing pointer remains readable.
        let (status, event) = queue.post(unsafe { NcValueRef::from_raw(&raw) }.unwrap(), event).unwrap_err();
        assert_eq!(status, NC_E_INVALID_TAG);
        assert_eq!(event.as_raw().object, identity);
        assert!(receive.try_recv().is_err());
        assert_eq!(drops.get(), 0);
        drop(receive);
        let (status, returned) = queue.post(NcValueRef::new(&origin).unwrap(), event).unwrap_err();
        assert_eq!(status, NC_E_CLOSED);
        assert_eq!(returned.as_raw().object, identity);
        assert_eq!(observed.value(), Ok(77));
        assert_eq!(drops.get(), 0);
        drop(returned);
        assert_eq!(drops.get(), 1);
        assert_eq!(observed.value(), Err(NC_E_DROPPED));
    }
}
