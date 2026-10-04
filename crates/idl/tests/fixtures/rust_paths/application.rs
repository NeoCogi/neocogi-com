//! Compiles optional form fields using both spellings of the relocated contract path.
#[allow(dead_code)]
mod absolute;
#[allow(dead_code)]
mod plain;

#[cfg(test)]
mod tests {
    use super::*;
    use path_contracts::r#type::{CounterAbi, CounterChanged, CounterEvent, ICounterEventAbi};

    /// Both field projections resolve to the same real ABI type across Cargo packages.
    #[test]
    fn raw_and_absolute_bindings_share_types_and_getter_values() {
        let _: fn(&plain::Form) -> &neocogi_ui::Widget<CounterAbi> = |form| &form.widget_counter_0;
        let _: fn(&absolute::Form) -> &neocogi_ui::Widget<CounterAbi> = |form| &form.widget_counter_0;
        let snapshot = path_contracts::r#type::changed(CounterEvent::Changed(CounterChanged { value: 42 })).unwrap();
        let event = neocogi_com::events::downcast_event::<ICounterEventAbi>(&snapshot).unwrap().unwrap();
        assert!(matches!(event.value().unwrap(), CounterEvent::Changed(value) if value.value == 42));
    }
}
