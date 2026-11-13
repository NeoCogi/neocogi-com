//! Real array codecs/COM calls expose stack amplification and partial ownership.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use neocogi_com::*;
#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
extern crate self as fixture_array_construction;
test_contracts::array_construction!();
#[cfg(not(miri))]
#[path = "support/array_watchdog.rs"]
mod watchdog;

/// Shared dispatch observation outlives every individual generated call.
#[com_object(thread_safe, interfaces(IArrays))]
struct Arrays {
    /// Counts only calls admitted past complete input validation.
    calls: Arc<AtomicUsize>,
}
impl IArrays for Arrays {
    fn accept(&self, values: [(); 65536]) -> Result<u32, NcStatus> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(values.len() as u32)
    }
}

/// Concrete destruction observation outlives all producer/receiver owners.
#[com_object(IProbe)]
struct Probe {
    /// Survives the object so final ownership release is independently visible.
    drops: Arc<AtomicUsize>,
}
impl IProbe for Probe {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(7)
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
/// Samples the generated lifecycle count without retaining another owner.
fn strong_count(owner: &ComPtr<IProbeAbi>) -> u32 {
    let raw = owner.as_raw();
    // SAFETY: generated projection tables share the live IUnknown prefix.
    unsafe {
        let table = &*raw.vtable.cast::<IUnknownVTable>();
        let count = (table.add_ref)(raw.object);
        (table.release)(raw.object);
        count - 1
    }
}

#[test]
fn borrowed_array_acquisition_releases_every_failed_owner_and_retains_success() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Probe::from_value(Probe { drops: drops.clone() }).query::<IProbeAbi>().unwrap();
    let source: [Owned; 3] = core::array::from_fn(|_| Owned { owner: owner.clone(), flag: true });
    let acquired = {
        let guard = NcValueRef::new(&source).unwrap();
        for index in 0..3 {
            // SAFETY: copy initialized raw storage while the producer retains all pointees.
            let mut wire = unsafe { guard.as_ptr().read() };
            wire.elements[index].flag = 2;
            let before = strong_count(&owner);
            // SAFETY: live objects and initialized raw fields; invalid bool is content only.
            assert!(matches!(
                unsafe { <[Owned; 3]>::decode_value(&wire, &mut ValueBudget::default()) },
                Err(NC_E_INVALID_ARG)
            ));
            assert_eq!(strong_count(&owner), before);
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }
        guard.to_owned().unwrap()
    };
    drop(source);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    for value in &acquired {
        assert_eq!(value.owner.value().unwrap(), 7);
    }
    drop(acquired);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Aligned native unit arrays have no actual result bytes to justify large scratch.
#[cfg(not(miri))]
#[repr(align(16))]
#[derive(Clone, Copy)]
struct AlignedUnit;
#[cfg(not(miri))]
impl ComSerialize for AlignedUnit {
    fn com_serialize(&self, writer: &mut serialization::Serializer<'_>) -> Result<(), NcStatus> {
        writer.serialize_unit()
    }
}
#[cfg(not(miri))]
impl ComDeserialize for AlignedUnit {
    fn com_deserialize(reader: &mut serialization::Deserializer<'_>) -> Result<Self, NcStatus> {
        reader.deserialize_unit().map(|_| Self)
    }
}
#[cfg(not(miri))]
// SAFETY: unit validation/accounting, no pointers, independent native alignment.
unsafe impl ComValue for AlignedUnit {
    const FIXED_COST: Option<ValueCost> = Some(ValueCost::SCALAR);
    type Wire = u8;
    type Storage = ();
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(u8, ()), NcStatus> {
        ().encode_value(budget)
    }
    unsafe fn decode_value(wire: &u8, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        unsafe { <() as ComValue>::decode_value(wire, budget) }.map(|_| Self)
    }
    unsafe fn encode_prechecked(&self) -> Result<(u8, ()), NcStatus> {
        unsafe { ().encode_prechecked() }
    }
    unsafe fn decode_prechecked(wire: &u8) -> Result<Self, NcStatus> {
        unsafe { <() as ComValue>::decode_prechecked(wire) }.map(|_| Self)
    }
}
/// Independent zero-sized backing alignment must not pad every wire/storage pair.
#[cfg(not(miri))]
#[derive(Clone, Copy)]
struct StorageUnit;
#[cfg(not(miri))]
// SAFETY: valid unit byte, no pointees; Storage is concrete and move-independent.
unsafe impl ComValue for StorageUnit {
    const FIXED_COST: Option<ValueCost> = Some(ValueCost::SCALAR);
    type Wire = u8;
    type Storage = AlignedUnit;
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(u8, AlignedUnit), NcStatus> {
        ().encode_value(budget).map(|(wire, _)| (wire, AlignedUnit))
    }
    unsafe fn decode_value(wire: &u8, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        unsafe { <() as ComValue>::decode_value(wire, budget) }.map(|_| Self)
    }
    unsafe fn encode_prechecked(&self) -> Result<(u8, AlignedUnit), NcStatus> {
        unsafe { ().encode_prechecked() }.map(|(wire, _)| (wire, AlignedUnit))
    }
    unsafe fn decode_prechecked(wire: &u8) -> Result<Self, NcStatus> {
        unsafe { <() as ComValue>::decode_prechecked(wire) }.map(|_| Self)
    }
}

#[cfg(not(miri))]
/// Keep raw-test temporaries out of the caller while measuring the safe client.
#[inline(never)]
fn generated_safe_call(object: &ComPtr<IArraysAbi, ThreadSafe>) {
    assert_eq!(object.accept([(); 65536]).unwrap(), 65536);
}

#[cfg(not(miri))]
/// Raw calls reuse main-owned input; invalid content must never reach dispatch.
#[inline(never)]
fn generated_raw_calls(object: &ComPtr<IArraysAbi, ThreadSafe>, wire: &ValueArray<u8, 65536>) {
    let raw = object.as_raw();
    let mut output = core::mem::MaybeUninit::uninit();
    // SAFETY: exact initialized raw array and matching output slot.
    assert_eq!(unsafe { ((*raw.vtable).accept)(raw.object, *wire, output.as_mut_ptr()) }, NC_OK);
    assert_eq!(unsafe { output.assume_init() }, 65536);
    for index in [0, 32768, 65535] {
        let mut invalid = *wire;
        invalid.elements[index] = 1;
        // SAFETY: raw unit byte is invalid content in live initialized storage.
        assert_eq!(unsafe { ((*raw.vtable).accept)(raw.object, invalid, output.as_mut_ptr()) }, NC_E_INVALID_ARG);
    }
}

#[cfg(not(miri))]
#[test]
fn fixed_array_codecs_and_com_calls_fit_a_one_mib_caller() {
    let Some(mode) = std::env::var_os("NC_ARRAY_STACK_CHILD") else {
        for mode in ["formats", "encoding", "acquisition", "generated"] {
            watchdog::run("fixed_array_codecs_and_com_calls_fit_a_one_mib_caller", mode);
        }
        return;
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let object = Arrays::from_value(Arrays { calls: calls.clone() }).query::<IArraysAbi>().unwrap();
    let source = [AlignedUnit; 65536];
    let guard = NcValueRef::new(&source).unwrap();
    let wire = Box::new(ValueArray { elements: [0u8; 65536] });
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn_scoped(scope, || match mode.to_str().unwrap() {
                "formats" => {
                    use serialization::{Format, Limits};
                    let mut bytes = [0; 8];
                    neocogi_com_serialization::ncb::to_slice(&source, &mut bytes).unwrap();
                    assert!(neocogi_com_serialization::ncb::from_slice::<[AlignedUnit; 65536]>(&bytes).is_ok());
                    bytes[4..].copy_from_slice(&0u32.to_le_bytes());
                    assert!(neocogi_com_serialization::ncb::from_slice::<[AlignedUnit; 65536]>(&bytes).is_err());
                    let json = Format::new(neocogi_com_serialization::json::factory(), Limits::default());
                    let bytes = json.serialize(&source).unwrap();
                    assert!(json.deserialize::<[AlignedUnit; 65536]>(&bytes).is_ok());
                    assert!(json.deserialize::<[AlignedUnit; 65536]>(b"[]").is_err());
                }
                "encoding" => {
                    let values = [StorageUnit; 65536];
                    assert!(NcValueRef::new(&values).is_ok());
                }
                "acquisition" => {
                    assert!(guard.to_owned().is_ok());
                }
                "generated" => {
                    generated_safe_call(&object);
                    generated_raw_calls(&object, &wire);
                    assert_eq!(calls.load(Ordering::SeqCst), 2);
                }
                _ => panic!("unknown stack test mode"),
            })
            .unwrap()
            .join()
            .unwrap();
    });
}
