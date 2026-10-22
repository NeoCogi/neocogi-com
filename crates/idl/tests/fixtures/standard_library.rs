//! Ordinary Option specializations exercise all ownership and raw validation paths.
pub mod contract;
#[cfg(test)]
mod tests {
    use super::contract::*;
    use neocogi_com::{com_object, ComPtr, ComValue, NcValueRef, NcStatus, NcCopyFromBufferOffset, NC_E_INVALID_TAG, NC_E_INVALID_ARG};

    /// An independently owned live capability used to check receiver retention.
    #[com_object(IToken)]
    struct Token;
    impl IToken for Token {
        /// Returns a value after the producer has released its handle.
        fn value(&self) -> Result<u32, NcStatus> { Ok(42) }
    }
    /// Application ownership is explicit and mutated only through exclusive methods.
    #[com_object(IValues)]
    struct Values {
        /// Independently retained optional capabilities, surviving the producer call.
        tokens: Vec<ComPtr<ITokenAbi>>,
        /// Complete text snapshots decoded from ordinary active variants.
        text: Vec<String>,
    }
    impl IValues for Values {
        /// Copied results use the same checked enum wire as copied arguments.
        fn number(&mut self, value: MaybeCount) -> Result<MaybeCount, NcStatus> { Ok(value) }
        /// Acquires text only from the selected payload, with ordinary value budgets.
        fn text(&mut self, value: NcValueRef<'_, MaybeText>) -> Result<(), NcStatus> {
            match value.to_owned()? { MaybeText::None => {}, MaybeText::Some(text) => self.text.push(text) }
            Ok(())
        }
        /// Retains the supplied object through the explicit owned payload codec.
        fn capability(&mut self, value: NcValueRef<'_, MaybeToken>) -> Result<u32, NcStatus> {
            match value.to_owned()? {
                MaybeToken::None => Ok(0),
                MaybeToken::Some(token) => { let result = token.value()?; self.tokens.push(token); Ok(result) },
            }
        }
        /// Input buffer elements have already passed the copied enum decoder.
        fn flags(&mut self, values: NcCopyFromBufferOffset<'_, MaybeFlag>) -> Result<u32, NcStatus> { Ok(values.len() as u32) }
    }

    /// Both absence and presence survive copied results, owned snapshots, and typed COM storage.
    #[test]
    fn ordinary_option_calls_preserve_values_and_receiver_ownership() {
        let local = Values::into_local(Values { tokens: vec![], text: vec![] });
        let mut interface: ComPtr<IValuesAbi> = local.query().unwrap();
        for input in [MaybeCount::None, MaybeCount::Some(7)] {
            assert_eq!(interface.number(input).unwrap(), input);
        }
        let text = MaybeText::Some("retained".into());
        interface.text(NcValueRef::new(&text).unwrap()).unwrap();
        drop(text);
        let token = MaybeToken::Some(Token::from_value(Token).query().unwrap());
        assert_eq!(interface.capability(NcValueRef::new(&token).unwrap()).unwrap(), 42);
        drop(token);
        assert_eq!(interface.capability(NcValueRef::new(&MaybeToken::None).unwrap()).unwrap(), 0);
        let flags = [MaybeFlag::None, MaybeFlag::Some(true)];
        assert_eq!(interface.flags(NcCopyFromBufferOffset::new(0, &flags)).unwrap(), 2);
        assert_eq!(local.try_borrow().unwrap().tokens[0].value().unwrap(), 42);
        assert_eq!(local.try_borrow().unwrap().text, ["retained"]);
    }

    /// No nullable-pointer shortcut or unchecked active enum payload survives lowering.
    #[test]
    fn invalid_optional_tags_are_rejected_by_generated_thunks() {
        let local = Values::into_local(Values { tokens: vec![], text: vec![] });
        let interface: ComPtr<IValuesAbi> = local.query().unwrap();
        let mut raw = MaybeCount::None.encode_value(&mut Default::default()).unwrap().0;
        raw.tag = 9;
        let view = interface.as_raw();
        let mut out = core::mem::MaybeUninit::uninit();
        // SAFETY: initialized raw tag storage and live object/table are valid;
        // the inactive None payload need not be inspected for this invalid tag.
        assert_eq!(unsafe { ((*view.vtable).number)(view.object, raw, out.as_mut_ptr()) }, NC_E_INVALID_TAG);
        let (mut raw, _storage) = MaybeFlag::None.encode_value(&mut Default::default()).unwrap();
        raw.tag = 9;
        let mut count = 0;
        // SAFETY: one initialized raw enum element is readable for this call.
        assert_eq!(unsafe { ((*view.vtable).flags)(view.object, 0, 1, &raw, &mut count) }, NC_E_INVALID_TAG);
        let (mut raw, _storage) = MaybeFlag::Some(true).encode_value(&mut Default::default()).unwrap();
        // A valid Some tag must still validate its recursively lowered boolean.
        raw.payload.case_1.field_0 = 2;
        assert_eq!(unsafe { ((*view.vtable).flags)(view.object, 0, 1, &raw, &mut count) }, NC_E_INVALID_ARG);
        // A unit variant has no active payload at all. The union may therefore
        // remain uninitialized; only the integer tag is initialized and read.
        let mut absent = core::mem::MaybeUninit::<<MaybeCount as ComValue>::Wire>::uninit();
        unsafe {
            core::ptr::addr_of_mut!((*absent.as_mut_ptr()).tag).write(0);
            // SAFETY: the tag is valid i64 storage and an uninspected union has
            // no field-validity invariant. The None decoder must not read it.
            let absent = absent.assume_init();
            assert_eq!(((*view.vtable).number)(view.object, absent, out.as_mut_ptr()), neocogi_com::NC_OK);
        }
    }

    /// COM authoring metadata reaches the ordinary enum and its payload references.
    #[test]
    fn inspection_exposes_option_as_an_enum_definition() {
        use neocogi_com::idl_metadata::{TypeShape, ValueType};
        let description = IValuesAbi::IDL_INTERFACE;
        let options = description.values.iter().filter(|value| value.qualified_name.starts_with("neocogi.lang.OptionInstance")).collect::<Vec<_>>();
        assert_eq!(options.len(), 4);
        for definition in options {
            let TypeShape::Enum(variants) = definition.shape else { panic!("ordinary enum metadata expected") };
            assert_eq!(variants[0].name, "None");
            assert!(variants[0].fields.is_empty());
            assert_eq!(variants[1].name, "Some");
            assert!(matches!(variants[1].fields[0].ty, ValueType::Scalar(_) | ValueType::Text | ValueType::Object(_)));
        }
    }
}
