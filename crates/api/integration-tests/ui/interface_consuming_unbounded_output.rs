//! An unused consuming declaration must still prove allocation-free completion.
use neocogi_com::*;

#[derive(Clone, Copy)]
/// Same logical boolean representation, but intentionally no maximum-cost proof.
pub struct Unbounded(bool);
// SAFETY: the wire and logical identity are exactly the checked bool codec's.
unsafe impl AbiType for Unbounded { const ABI_TYPE: NcAbiType = <bool as AbiType>::ABI_TYPE; }
// SAFETY: the logical boolean owns no pointers and is durable copied data.
unsafe impl AbiPlainValue for Unbounded {}
// SAFETY: owned values are self-contained and restricted bits are checked by bool.
unsafe impl ComValue for Unbounded {
    type Wire = u8;
    type Storage = ();
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(u8, ()), NcStatus> { self.0.encode_value(budget) }
    unsafe fn decode_value(wire: u8, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        // SAFETY: this decoder delegates the exact same initialized raw byte.
        unsafe { bool::decode_value(wire, budget) }.map(Self)
    }
}
// SAFETY: a single raw byte carries no backing pointers or ownership obligations.
unsafe impl ComInlineValue for Unbounded {}

#[com_interface(auto_guid = "nc.tests.unbounded.Copied:v1", assigned_signature = b"00000000000000000000000000000821")]
pub trait ICopied {
    fn take(&self, owner: ComBox<IUnknown>) -> Result<Unbounded, (NcStatus, ComBox<IUnknown>)>;
}
#[com_interface(auto_guid = "nc.tests.unbounded.Buffer:v1", assigned_signature = b"00000000000000000000000000000822")]
pub trait IBuffer {
    fn take(&self, owner: ComBox<IUnknown>, output: NcOutputWriter<'_, Unbounded>) -> Result<(), (NcStatus, ComBox<IUnknown>)>;
}
// Non-consuming methods need no declaration-time maximum; ordinary codec errors
// remain recoverable. These declarations must not add diagnostics to the fixture.
#[com_interface(auto_guid = "nc.tests.unbounded.Ordinary:v1", assigned_signature = b"00000000000000000000000000000823")]
pub trait IOrdinary {
    fn get(&self, output: NcOutputWriter<'_, Unbounded>) -> Result<Unbounded, NcStatus>;
}
fn main() {}
