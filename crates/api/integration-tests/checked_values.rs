//! Restricted nested fields cross real generated COM slots only as checked wire values.
// The depth-limit regression deliberately instantiates a 65-level wire type.
#![recursion_limit = "256"]
use neocogi_com::*;

#[com_struct(assigned_signature = b"00000000000000000000000000000081")]
#[derive(Debug, PartialEq)]
/// A plain record containing a restricted scalar.
pub struct Flags {
    /// A byte on the wire and a bool only after validation.
    pub enabled: bool,
}
#[com_struct(assigned_signature = b"00000000000000000000000000000082")]
#[derive(Debug, PartialEq)]
/// Nested ordinary records must recursively lower their fields.
pub struct Settings {
    /// Validation cannot stop at this named type boundary.
    pub flags: Flags,
}
#[com_enum(tag = u8, assigned_signature = b"00000000000000000000000000000083")]
#[derive(Clone, Copy, Debug, PartialEq)]
/// Enum payload uses the same nested record converter.
pub enum Action {
    /// Selects a nested checked payload.
    Configure(Settings),
    /// Requires no initialized payload.
    Idle,
}
#[com_struct(assigned_signature = b"00000000000000000000000000000087")]
#[derive(Debug, PartialEq)]
/// Copied records and fixed arrays store native enums, not public tag/union wrappers.
pub struct ActionSet {
    /// Alternatives of unequal payload size still lower each active value.
    pub actions: [Action; 2],
}
#[com_interface(auto_guid = "nc.tests.values.Check:v1", assigned_signature = b"00000000000000000000000000000084")]
/// Real copied, enum, and buffer slots for adversarial raw callers.
pub trait ICheck {
    /// Returns the input after the thunk validates it.
    fn settings(&self, value: Settings) -> Result<Settings, NcStatus>;
    /// The native enum lowers to a checked wire record in the raw slot.
    fn action(&self, value: Action) -> Result<Action, NcStatus>;
    /// A copied record may recursively contain native enums in fixed arrays.
    fn action_set(&self, value: ActionSet) -> Result<ActionSet, NcStatus>;
    /// A transformation has a distinct readable input and fresh output destination.
    fn actions(&self, input: NcCopyFromBufferOffset<'_, Action>, value: NcOutputWriter<'_, Action>) -> Result<(), NcStatus>;
    /// Nested restricted fields are checked before reconstructing a typed slice.
    fn input(&self, value: NcCopyFromBufferOffset<'_, Settings>) -> Result<u32, NcStatus>;
    /// Output elements are produced without decoding old destination bytes.
    fn output(&self, value: NcOutputWriter<'_, Settings>) -> Result<(), NcStatus>;
}
#[com_object(ICheck)]
/// Stateless implementation; errors must originate before method dispatch.
struct Check;
impl ICheck for Check {
    fn settings(&self, value: Settings) -> Result<Settings, NcStatus> {
        Ok(value)
    }
    fn action(&self, value: Action) -> Result<Action, NcStatus> {
        Ok(value)
    }
    fn action_set(&self, value: ActionSet) -> Result<ActionSet, NcStatus> {
        Ok(value)
    }
    fn actions(&self, input: NcCopyFromBufferOffset<'_, Action>, mut value: NcOutputWriter<'_, Action>) -> Result<(), NcStatus> {
        // Transform only explicit checked inputs, never destination contents.
        for &action in input.as_slice() {
            value.push(match action {
                Action::Idle => Action::Configure(Settings { flags: Flags { enabled: true } }),
                Action::Configure(_) => Action::Idle,
            })?;
        }
        Ok(())
    }
    fn input(&self, value: NcCopyFromBufferOffset<'_, Settings>) -> Result<u32, NcStatus> {
        Ok(value.len() as u32)
    }
    fn output(&self, mut value: NcOutputWriter<'_, Settings>) -> Result<(), NcStatus> {
        for _ in 0..value.capacity() {
            value.push(Settings { flags: Flags { enabled: true } })?;
        }
        Ok(())
    }
}

#[test]
fn invalid_nested_bool_fails_copied_and_buffer_thunks() {
    let check = Check::from_value(Check).query::<ICheckAbi>().unwrap();
    let ptr = check.as_raw();
    let mut wire = encode_inline(&Settings { flags: Flags { enabled: true } }).unwrap();
    wire.flags.enabled = 2;
    let mut out = core::mem::MaybeUninit::uninit();
    // SAFETY: all raw storage is initialized, aligned and live; invalid bytes
    // deliberately test conversion before a restricted Rust value is created.
    unsafe {
        assert_eq!(((*ptr.vtable).settings)(ptr.object, wire, out.as_mut_ptr()), NC_E_INVALID_ARG);
        let mut count = 0;
        assert_eq!(((*ptr.vtable).input)(ptr.object, 0, 1, &wire, &mut count), NC_E_INVALID_ARG);
        // Old output bytes are not input, even when they contain invalid booleans.
        let mut destination = wire;
        let mut written = 99;
        assert_eq!(((*ptr.vtable).output)(ptr.object, 0, 1, &mut destination, &mut written), NC_OK);
        assert_eq!(written, 1);
        assert_eq!(destination.flags.enabled, 1);
    }
}

#[test]
fn enum_tag_is_checked_before_payload_and_nested_field_is_checked_afterward() {
    let check = Check::from_value(Check).query::<ICheckAbi>().unwrap();
    let ptr = check.as_raw();
    let mut bad_field = encode_inline(&Action::Configure(Settings { flags: Flags { enabled: true } })).unwrap();
    // Start with the encoded active alternative, then change only its raw bool byte.
    bad_field.payload.case_0.field_0.flags.enabled = 3;
    let mut bad_tag = encode_inline(&Action::Idle).unwrap();
    bad_tag.tag = 99;
    let mut out = core::mem::MaybeUninit::uninit();
    unsafe {
        assert_eq!(((*ptr.vtable).action)(ptr.object, bad_field, out.as_mut_ptr()), NC_E_INVALID_ARG);
        assert_eq!(((*ptr.vtable).action)(ptr.object, bad_tag, out.as_mut_ptr()), NC_E_INVALID_TAG);
    }
}

#[test]
fn valid_calls_acquire_output_buffers_and_nested_return_values() {
    let check = Check::from_value(Check).query::<ICheckAbi>().unwrap();
    let settings = Settings { flags: Flags { enabled: false } };
    assert_eq!(check.settings(settings).unwrap(), settings);
    let mut values = NcOutputBuffer::new();
    check.output(values.output(0, 1)).unwrap();
    assert!(values[0].flags.enabled);
    assert_eq!(check.input(NcCopyFromBufferOffset::new(0, &values)).unwrap(), 1);
}

/// Native enums round-trip directly, nested in copied records, and through writable buffers.
#[test]
fn native_enums_cross_copied_nested_array_and_buffer_slots() {
    let check = Check::from_value(Check).query::<ICheckAbi>().unwrap();
    let configured = Action::Configure(Settings { flags: Flags { enabled: true } });
    for action in [Action::Idle, configured] {
        assert_eq!(check.action(action).unwrap(), action);
    }
    let set = ActionSet { actions: [Action::Idle, configured] };
    assert_eq!(check.action_set(set).unwrap(), set);
    let mut actions = NcOutputBuffer::new();
    check.actions(NcCopyFromBufferOffset::new(0, &set.actions), actions.output(0, 2)).unwrap();
    assert_eq!(actions.as_slice(), [configured, Action::Idle]);
    // Metadata retains the same nominal identity while raw and native storage
    // remain separate even if a particular compiler happens to size them equally.
    assert!(matches!(Action::ABI_TYPE, NcAbiType::Aggregate(signature) if signature == Action::VALUE_SIGNATURE));
    assert!(matches!(Flags::ABI_TYPE, NcAbiType::Aggregate(signature) if signature == Flags::VALUE_SIGNATURE));
    assert!(matches!(Settings::ABI_TYPE, NcAbiType::Aggregate(signature) if signature == Settings::VALUE_SIGNATURE));
    assert!(matches!(ActionSet::ABI_TYPE, NcAbiType::Aggregate(signature) if signature == ActionSet::VALUE_SIGNATURE));
    assert!(!Action::BORROW_WIRE);
    assert!(!ActionSet::BORROW_WIRE);
}

/// A later malformed enum element must not publish an earlier valid element's mutation.
#[test]
fn enum_buffer_validation_is_atomic_before_native_dispatch() {
    let check = Check::from_value(Check).query::<ICheckAbi>().unwrap();
    let raw = check.as_raw();
    for (tag, enabled, expected) in [(99, 1, NC_E_INVALID_TAG), (0, 3, NC_E_INVALID_ARG)] {
        let mut wire = [
            encode_inline(&Action::Idle).unwrap(),
            encode_inline(&Action::Configure(Settings { flags: Flags { enabled: true } })).unwrap(),
        ];
        wire[1].tag = tag;
        wire[1].payload.case_0.field_0.flags.enabled = enabled;
        // SAFETY: both records are initialized wire storage, with a bad raw tag
        // or boolean byte. The callee must reject before materializing native enums.
        let mut output = [encode_inline(&Action::Idle).unwrap(); 2];
        let mut written = 99;
        assert_eq!(
            unsafe { ((*raw.vtable).actions)(raw.object, 0, 2, wire.as_ptr(), 0, 2, output.as_mut_ptr(), &mut written) },
            expected
        );
        assert_eq!(output[0].tag, 1);
        assert_eq!(output[1].tag, 1);
        assert_eq!(wire[0].tag, 1);
        assert_eq!(wire[1].tag, tag);
    }
}

#[com_struct(assigned_signature = b"00000000000000000000000000000085")]
#[derive(Debug, PartialEq)]
/// Numeric render-style records need no validation or allocation when borrowed.
pub struct Coordinates {
    /// Every floating-point bit pattern is a valid initialized Rust value.
    pub x: f32,
    /// The second field has the same application and wire offset.
    pub y: f32,
}

/// Identity proof retains zero-copy behavior without admitting restricted fields.
#[test]
fn unrestricted_buffers_borrow_in_place_and_restricted_buffers_are_separate() {
    let values = [Coordinates { x: 1.0, y: 2.0 }];
    let encoded = EncodedInput::new(NcCopyFromBufferOffset::new(7, &values)).unwrap();
    assert_eq!(encoded.as_ptr().cast::<Coordinates>(), values.as_ptr());
    assert_eq!(encoded.offset(), 7);
    let decoded = unsafe { DecodedInput::<Coordinates>::from_raw(encoded.as_ptr(), encoded.len()) }.unwrap();
    assert_eq!(decoded.as_slice().as_ptr(), values.as_ptr());
    let restricted = [Settings { flags: Flags { enabled: true } }];
    let encoded = EncodedInput::new(NcCopyFromBufferOffset::new(0, &restricted)).unwrap();
    assert_ne!(encoded.as_ptr().cast::<Settings>(), restricted.as_ptr());
    let decoded = unsafe { DecodedInput::<Settings>::from_raw(encoded.as_ptr(), encoded.len()) }.unwrap();
    assert_ne!(decoded.as_slice().as_ptr(), restricted.as_ptr());
}

/// Output identity storage stays allocation-free and exposes only the written prefix.
#[test]
fn writable_identity_buffers_retain_the_original_allocation() {
    let mut slots = [core::mem::MaybeUninit::<u32>::uninit(); 3];
    let pointer = slots.as_mut_ptr();
    let mut count = 0;
    let mut prepared = NcOutput::from_slots(9, &mut slots, &mut count).unwrap().prepare().unwrap();
    assert_eq!(prepared.as_mut_ptr(), pointer.cast());
    let mut written = 0;
    // SAFETY: the prepared output lends the same exclusive, properly sized wire slots.
    let mut writer = unsafe { NcOutputWriter::<u32>::from_raw(9, prepared.as_mut_ptr(), 3, &mut written) }.unwrap();
    writer.extend_from_slice(&[1, 8]).unwrap();
    drop(writer);
    unsafe {
        *prepared.written_ptr() = written;
        prepared.convert_output().unwrap();
    }
    prepared.publish();
    assert_eq!(count, 2);
    // SAFETY: only this successfully published prefix was initialized.
    assert_eq!(unsafe { slots[1].assume_init() }, 8);
}

/// Fixed-shape optimization must preserve the exact nested count/depth contract.
#[test]
fn copied_aggregate_budget_boundaries_remain_exact() {
    use neocogi_com::{ValueBudget, NC_E_OUT_OF_RANGE};
    let value = Settings { flags: Flags { enabled: true } };
    // Settings -> Flags -> bool is three logical values and three levels deep.
    let (wire, _) = value.encode_value(&mut ValueBudget::new(0, 3, 3)).unwrap();
    for (values, depth) in [(2, 3), (3, 2)] {
        assert!(matches!(value.encode_value(&mut ValueBudget::new(0, values, depth)), Err(NC_E_OUT_OF_RANGE)));
        // SAFETY: the successful encoding supplies a fully initialized raw record.
        assert_eq!(
            unsafe { Settings::decode_value(wire, &mut ValueBudget::new(0, values, depth)) },
            Err(NC_E_OUT_OF_RANGE)
        );
    }
    // Siblings share the cumulative count, but each completed subtree restores depth.
    let mut encode_budget = ValueBudget::new(0, 4, 3);
    value.encode_value(&mut encode_budget).unwrap();
    true.encode_value(&mut encode_budget).unwrap();
    assert!(matches!(true.encode_value(&mut encode_budget), Err(NC_E_OUT_OF_RANGE)));
    let mut decode_budget = ValueBudget::new(0, 4, 3);
    // SAFETY: wire came from the checked encoder and bool uses an unrestricted byte.
    unsafe {
        assert_eq!(Settings::decode_value(wire, &mut decode_budget), Ok(value));
        assert_eq!(bool::decode_value(1, &mut decode_budget), Ok(true));
        assert_eq!(bool::decode_value(1, &mut decode_budget), Err(NC_E_OUT_OF_RANGE));
    }
}

/// Array preflight must include its root and every nested record, then validate bits.
#[test]
fn fixed_arrays_charge_whole_subtrees_and_still_validate_each_element() {
    let value = [Settings { flags: Flags { enabled: true } }; 2];
    // One array plus two Settings/Flags/bool subtrees: seven values, depth four.
    let (mut wire, _) = value.encode_value(&mut ValueBudget::new(0, 7, 4)).unwrap();
    assert!(matches!(value.encode_value(&mut ValueBudget::new(0, 6, 4)), Err(NC_E_OUT_OF_RANGE)));
    assert!(matches!(value.encode_value(&mut ValueBudget::new(0, 7, 3)), Err(NC_E_OUT_OF_RANGE)));
    wire.elements[1].flags.enabled = 7;
    // SAFETY: the raw array is initialized; its invalid boolean must be rejected.
    assert_eq!(
        unsafe { <[Settings; 2]>::decode_value(wire, &mut ValueBudget::new(0, 7, 4)) },
        Err(NC_E_INVALID_ARG)
    );
}

/// Unequal enum alternatives must pay their selected shape, not the maximum shape.
#[test]
fn variant_dependent_shapes_retain_exact_dynamic_accounting() {
    let idle = Action::Idle;
    let configured = Action::Configure(Settings { flags: Flags { enabled: true } });
    assert_eq!(Action::FIXED_COST, None);
    let (idle_wire, _) = idle.encode_value(&mut ValueBudget::new(0, 1, 1)).unwrap();
    let (configured_wire, _) = configured.encode_value(&mut ValueBudget::new(0, 4, 4)).unwrap();
    assert!(matches!(configured.encode_value(&mut ValueBudget::new(0, 3, 4)), Err(NC_E_OUT_OF_RANGE)));
    // SAFETY: both wires came from checked conversion and contain no indirect data.
    unsafe {
        assert_eq!(Action::decode_value(idle_wire, &mut ValueBudget::new(0, 1, 1)), Ok(idle));
        assert_eq!(Action::decode_value(configured_wire, &mut ValueBudget::new(0, 4, 4)), Ok(configured));
        assert_eq!(Action::decode_value(configured_wire, &mut ValueBudget::new(0, 4, 3)), Err(NC_E_OUT_OF_RANGE));
    }
}

#[com_value(assigned_signature = b"00000000000000000000000000000086")]
#[derive(Debug, PartialEq)]
/// Mixes fixed records, a selected enum variant, and variable-length ownership.
pub struct MixedSettings {
    /// This fixed record is counted together with the enclosing root.
    pub settings: Settings,
    /// Text remains dynamically byte-counted and independently acquired.
    pub title: String,
    /// Only the selected alternative contributes logical values and depth.
    pub action: Action,
}

/// Counting known fields together must leave exact shared limits for dynamic fields.
#[test]
fn mixed_records_preserve_cumulative_bytes_values_and_depth() {
    let value = MixedSettings {
        settings: Settings { flags: Flags { enabled: true } },
        title: "ok".to_owned(),
        action: Action::Configure(Settings { flags: Flags { enabled: false } }),
    };
    // Root + Settings(3) + String(1) + Configure(4) = nine values, depth five.
    assert_eq!(MixedSettings::MAX_COST, None);
    let (wire, _storage) = value.encode_value(&mut ValueBudget::new(2, 9, 5)).unwrap();
    for (bytes, values, depth) in [(1, 9, 5), (2, 8, 5), (2, 9, 4)] {
        assert!(matches!(
            value.encode_value(&mut ValueBudget::new(bytes, values, depth)),
            Err(NC_E_OUT_OF_RANGE)
        ));
        // SAFETY: the source and typed guard retain every active wire field.
        assert_eq!(
            unsafe { MixedSettings::decode_value(wire, &mut ValueBudget::new(bytes, values, depth)) },
            Err(NC_E_OUT_OF_RANGE)
        );
    }
    // SAFETY: the retained source supplies the valid string backing.
    assert_eq!(unsafe { MixedSettings::decode_value(wire, &mut ValueBudget::new(2, 9, 5)) }, Ok(value));
}

/// Eight optional levels compose a small value whose largest shape exceeds depth 64.
type Optional8<T> = Option<Option<Option<Option<Option<Option<Option<Option<T>>>>>>>>;
/// A 64-option chain adds one more level for its scalar leaf.
type Optional64 = Optional8<Optional8<Optional8<Optional8<Optional8<Optional8<Optional8<Optional8<bool>>>>>>>>;

/// Builds eight present levels without allocation or an untyped payload.
fn present8<T>(value: T) -> Optional8<T> {
    Some(Some(Some(Some(Some(Some(Some(Some(value))))))))
}

/// An excessive upper bound must fall back to the actual shape, never reject it alone.
#[test]
fn inline_bound_falls_back_to_selected_depth() {
    let absent: Optional64 = None;
    let wire = encode_inline(&absent).unwrap();
    // SAFETY: absent storage has no initialized payload requirement.
    assert_eq!(unsafe { decode_inline::<Optional64>(wire) }, Ok(absent));
    let deepest = present8(present8(present8(present8(present8(present8(present8(present8(true))))))));
    assert!(matches!(encode_inline(&deepest), Err(NC_E_OUT_OF_RANGE)));
    // Produce valid depth-65 storage with an explicit larger budget, then ensure
    // ordinary inline acquisition still enforces its depth-64 default.
    let (wire, _storage) = deepest.encode_value(&mut ValueBudget::new(0, 65, 65)).unwrap();
    // SAFETY: all selected optional fields are initialized by the checked encoder.
    assert_eq!(unsafe { decode_inline::<Optional64>(wire) }, Err(NC_E_OUT_OF_RANGE));
}

/// Buffer bounds may exceed the budget while the actual small variants still fit.
#[test]
fn buffers_count_selected_variants_without_charging_larger_alternatives() {
    let values = vec![Action::Idle; MAX_VALUE_COUNT / 4 + 1];
    // Configure needs four values; Idle needs one. Multiplying the maximum would
    // reject this valid buffer, so the actual active alternatives must be counted.
    let mut wire = encode_value_buffer(&values).unwrap();
    // SAFETY: wire owns initialized, self-contained enum representations.
    let decoded = unsafe { decode_value_buffer::<Action>(wire.as_ptr(), wire.len() as u64) }.unwrap();
    assert_eq!(decoded.len(), values.len());
    assert!(decoded.iter().all(|value| *value == Action::Idle));
    wire.last_mut().unwrap().tag = 99;
    // SAFETY: unknown raw tags are valid wire integers and must be rejected.
    assert!(matches!(
        unsafe { decode_value_buffer::<Action>(wire.as_ptr(), wire.len() as u64) },
        Err(NC_E_INVALID_TAG)
    ));
}

/// Fixed buffer elements must count nested records, not just the number of elements.
#[test]
fn buffer_count_includes_each_nested_record() {
    let values = vec![Settings { flags: Flags { enabled: true } }; MAX_VALUE_COUNT / 3 + 1];
    assert!(matches!(encode_value_buffer(&values), Err(NC_E_OUT_OF_RANGE)));
    let element = encode_inline(&Settings { flags: Flags { enabled: true } }).unwrap();
    let wire = vec![element; values.len()];
    // SAFETY: all wire records are initialized, but their total work exceeds policy.
    assert!(matches!(
        unsafe { decode_value_buffer::<Settings>(wire.as_ptr(), wire.len() as u64) },
        Err(NC_E_OUT_OF_RANGE)
    ));
}

/// Array identity propagates only when every nested element has unrestricted bits.
#[test]
fn numeric_arrays_borrow_in_place_but_boolean_arrays_are_checked() {
    let values = [[Coordinates { x: 1.0, y: 2.0 }; 2]];
    let encoded = EncodedInput::new(NcCopyFromBufferOffset::new(0, &values)).unwrap();
    assert_eq!(encoded.as_ptr().cast::<[Coordinates; 2]>(), values.as_ptr());
    // SAFETY: the encoded guard borrows the original initialized numeric array.
    let decoded = unsafe { DecodedInput::<[Coordinates; 2]>::from_raw(encoded.as_ptr(), encoded.len()) }.unwrap();
    assert_eq!(decoded.as_slice().as_ptr(), values.as_ptr());
    assert!(!<[Settings; 2]>::BORROW_WIRE);
}
