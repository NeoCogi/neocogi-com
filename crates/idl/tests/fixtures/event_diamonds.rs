//! Real generated metadata and getters share canonical plain/generic ancestry.
extern crate self as event_diamonds_proof;
mod contracts;
mod left;
mod right;
mod source;

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com::{ComInterface, NcStatus, NC_E_DROPPED};
    use neocogi_com::events::{IEvent, IEventAbi};
    use neocogi_ui_design::{DesignValueType, DesignWidgetManifest};

    #[neocogi_com::com_object(interfaces(source::Fact), ownership = exclusive)]
    /// One immutable captured value serves both local projections of its ancestor.
    struct FactEvent;
    impl IEvent for FactEvent {}
    impl left::DetailSnapshot for FactEvent {
        /// Reads the captured value through the left module's closed projection.
        fn value(&self) -> Result<contracts::Detail, NcStatus> {
            Ok(contracts::Detail { value: 42 })
        }
    }
    impl right::DetailSnapshot for FactEvent {
        /// Implements the same canonical getter contract through the right projection.
        fn value(&self) -> Result<contracts::Detail, NcStatus> {
            Ok(contracts::Detail { value: 42 })
        }
    }
    impl left::Left for FactEvent {
        /// Distinct local getters retain their base-first order in metadata.
        fn left(&self) -> Result<bool, NcStatus> {
            Ok(true)
        }
    }
    impl right::Right for FactEvent {
        /// A sibling getter contributes once after the shared ancestor.
        fn right(&self) -> Result<bool, NcStatus> {
            Ok(false)
        }
    }
    impl source::Fact for FactEvent {
        /// The derived getter follows both bases.
        fn revision(&self) -> Result<u32, NcStatus> {
            Ok(7)
        }
    }

    #[neocogi_com::com_object(interfaces(source::ScalarFact), ownership = exclusive)]
    /// Scalar specialization is distinct from the record-valued shared ancestor.
    struct ScalarEvent;
    impl IEvent for ScalarEvent {}
    impl left::ScalarSnapshot for ScalarEvent {
        /// Exposes the scalar through one independently generated projection.
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(9)
        }
    }
    impl right::ScalarSnapshot for ScalarEvent {
        /// The other projection negotiates the same scalar interface contract.
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(9)
        }
    }
    impl left::ScalarLeft for ScalarEvent {}
    impl right::ScalarRight for ScalarEvent {}
    impl source::ScalarFact for ScalarEvent {}

    #[neocogi_com::com_object(interfaces(source::PlainFact), ownership = exclusive)]
    /// Ordinary diamonds also retain a single shared ancestor getter.
    struct PlainEvent;
    impl IEvent for PlainEvent {}
    impl contracts::Plain for PlainEvent {
        /// The one non-generic ancestor has one implementation.
        fn value(&self) -> Result<contracts::Detail, NcStatus> {
            Ok(contracts::Detail { value: 21 })
        }
    }
    impl left::PlainLeft for PlainEvent {
        /// Local plain-diamond branch getter.
        fn left(&self) -> Result<bool, NcStatus> {
            Ok(true)
        }
    }
    impl right::PlainRight for PlainEvent {
        /// Required sibling getter stays queryable on the same allocation.
        fn right(&self) -> Result<bool, NcStatus> {
            Ok(false)
        }
    }
    impl source::PlainFact for PlainEvent {
        /// The derived plain getter remains independent of the shared base.
        fn revision(&self) -> Result<u32, NcStatus> {
            Ok(3)
        }
    }

    /// Includes one field per semantic getter and preserves authored event/base order.
    #[test]
    fn catalog_admits_and_round_trips_all_three_diamonds() {
        for metadata in [&source::FactAbi::IDL_INTERFACE, &source::PlainFactAbi::IDL_INTERFACE] {
            assert_eq!(metadata.methods.iter().map(|method| method.name).collect::<Vec<_>>(),
                ["value", "left", "right", "revision"]);
            assert!(metadata.methods.iter().all(|method| method.receiver == neocogi_com::NcAbiReceiver::Shared
                && method.parameters.is_empty()));
            assert_eq!(metadata.methods[0].returns, neocogi_com::design::IdlMethodType::Value(
                neocogi_com::design::IdlValueType::Named("proof.contracts.Detail")));
        }
        assert_eq!(source::ScalarFactAbi::IDL_INTERFACE.methods.len(), 1);
        let descriptor = &source::SourceAbi::IDL_INTERFACE;
        let events: Vec<_> = descriptor.events.iter().map(|event| event.qualified_name).collect();
        assert_eq!(events, ["proof.source.ScalarFact", "proof.source.Fact", "proof.source.PlainFact"]);
        for (event, expected) in descriptor.events.iter().zip([
            &["value"][..],
            &["value", "left", "right", "revision"][..],
            &["value", "left", "right", "revision"][..],
        ]) {
            assert_eq!(event.fields.iter().map(|field| field.name).collect::<Vec<_>>(), expected);
        }
        let manifest = DesignWidgetManifest::from_idl("proof.source", vec![], descriptor).unwrap();
        assert_eq!(manifest.interface.events[0].fields[0].ty, DesignValueType::Scalar("u32".into()));
        for event in &manifest.interface.events[1..] {
            assert_eq!(event.fields[0].ty, DesignValueType::Named("proof.contracts.Detail".into()));
        }
        assert!(manifest.interface.values.iter().any(|value| value.qualified_name == "proof.contracts.Detail"));
        assert_eq!(DesignWidgetManifest::decode(&manifest.encode().unwrap()).unwrap(), manifest);
    }

    /// Event ownership and cached projections are identical across generic branches.
    #[test]
    fn generic_getters_negotiate_one_ancestor_and_expire_with_the_owner() {
        assert_eq!(left::DetailSnapshotAbi::INTERFACE_ID, right::DetailSnapshotAbi::INTERFACE_ID);
        assert_eq!(left::ScalarSnapshotAbi::INTERFACE_ID, right::ScalarSnapshotAbi::INTERFACE_ID);
        assert_ne!(left::DetailSnapshotAbi::INTERFACE_ID, left::ScalarSnapshotAbi::INTERFACE_ID);
        let event = FactEvent::into_box(FactEvent).query::<IEventAbi>().unwrap();
        let left = event.query_weak::<left::DetailSnapshotAbi>().unwrap();
        let right = event.query_weak::<right::DetailSnapshotAbi>().unwrap();
        assert_eq!(left.as_raw().object, right.as_raw().object);
        assert_eq!(left.value().unwrap().value, 42);
        assert_eq!(right.value().unwrap().value, 42);
        assert!(event.query_weak::<left::LeftAbi>().unwrap().left().unwrap());
        assert!(!event.query_weak::<right::RightAbi>().unwrap().right().unwrap());
        assert_eq!(event.query_weak::<source::FactAbi>().unwrap().revision(), Ok(7));
        drop(event);
        assert_eq!(left.value().unwrap_err(), NC_E_DROPPED);
        assert_eq!(right.value().unwrap_err(), NC_E_DROPPED);
        let scalar = ScalarEvent::into_box(ScalarEvent).query::<IEventAbi>().unwrap();
        assert_eq!(scalar.query_weak::<left::ScalarSnapshotAbi>().unwrap().value(), Ok(9));
        assert_eq!(scalar.query_weak::<right::ScalarSnapshotAbi>().unwrap().value(), Ok(9));
    }

    /// Ordinary shared ancestors and sibling getters remain callable through COM.
    #[test]
    fn plain_diamond_calls_share_the_same_exclusive_allocation() {
        let event = PlainEvent::into_box(PlainEvent).query::<IEventAbi>().unwrap();
        let base = event.query_weak::<contracts::PlainAbi>().unwrap();
        let left = event.query_weak::<left::PlainLeftAbi>().unwrap();
        let right = event.query_weak::<right::PlainRightAbi>().unwrap();
        assert_eq!(base.as_raw().object, left.as_raw().object);
        assert_eq!(base.as_raw().object, right.as_raw().object);
        assert_eq!(base.value().unwrap().value, 21);
        assert!(left.left().unwrap());
        assert!(!right.right().unwrap());
        assert_eq!(event.query_weak::<source::PlainFactAbi>().unwrap().revision(), Ok(3));
    }
}
