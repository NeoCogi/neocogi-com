//! Single-threaded allocation probe comparing direct and named scalar mappings.
//! Run with: cargo run -p neocogi-example-com-plugin --example event_allocations --features monolith

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use neocogi_com::{ComPtr, Runtime, guid_from_qualified_name};
use neocogi_com::events::mapped_sink;
use neocogi_com::NcValueRef;
use neocogi_idl_example_contracts::{ExampleObjectEventBinding, ExampleObjectSinkAbi};
use neocogi_idl_example_contracts::{ExampleFactoryAbi, ExampleObjectAbi, Extent, ExtentChanged};

/// Gates counting around the measured setup or synchronous delivery only.
static COUNTING: AtomicBool = AtomicBool::new(false);
/// Counts allocation/reallocation calls, not bytes or elapsed time.
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
/// Largest requested allocation in a measured operation.
static LARGEST_ALLOCATION: AtomicUsize = AtomicUsize::new(0);

/// Delegates every memory operation unchanged to the system allocator.
struct CountingAllocator;
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Records allocation calls without allocating or taking a lock itself.
fn allocated(bytes: usize) {
    if COUNTING.load(Ordering::Relaxed) {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        LARGEST_ALLOCATION.fetch_max(bytes, Ordering::Relaxed);
    }
}
// SAFETY: Every operation forwards the caller's original allocation contract to System.
unsafe impl GlobalAlloc for CountingAllocator {
    /// Measures a normal allocation while preserving its requested layout.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocated(layout.size());
        // SAFETY: GlobalAlloc callers provide the required valid layout.
        unsafe { System.alloc(layout) }
    }
    /// Measures a zeroed allocation without changing initialization guarantees.
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocated(layout.size());
        // SAFETY: The forwarded layout satisfies the allocator contract.
        unsafe { System.alloc_zeroed(layout) }
    }
    /// Reallocation may allocate even if a particular allocator grows in place.
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocated(size);
        // SAFETY: The pointer, old layout and new size come from a valid realloc call.
        unsafe { System.realloc(pointer, layout, size) }
    }
    /// Frees through the allocator that supplied the storage.
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The caller supplies the live pointer and its original layout.
        unsafe { System.dealloc(pointer, layout) }
    }
}

/// Measures one synchronous operation after all fixtures and channels exist.
fn count(operation: impl FnOnce()) -> usize {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    LARGEST_ALLOCATION.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Relaxed);
    operation();
    COUNTING.store(false, Ordering::Relaxed);
    ALLOCATIONS.load(Ordering::Relaxed)
}

/// Emits and drains scalar events; assertion formatting only runs on a failure.
fn deliver(object: &mut ComPtr<ExampleObjectAbi>, messages: &std::sync::mpsc::Receiver<u32>) {
    for width in 0..10_000 {
        object.resize(Extent { width, height: 1 }).unwrap();
        assert_eq!(messages.try_recv().unwrap(), width);
    }
}

/// Equal allocation counts expose any allocation accidentally added to named delivery.
fn main() {
    // A valid collection count followed by no elements must not reserve 64 MiB
    // for 65,536 records of 1,024 bytes each before detecting truncated input.
    count(|| {
        assert!(neocogi_com::binary::from_slice::<Vec<[u8; 1024]>>(b"NCB\x01\0\0\x01\0").is_err());
    });
    let largest = LARGEST_ALLOCATION.load(Ordering::Relaxed);
    assert!(largest <= 4096, "malformed input reserved {largest} bytes before validation");
    println!("largest eager allocation for truncated large-record collection: {largest} bytes");
    let runtime = Runtime::global();
    // SAFETY: The immutable descriptor and implementation are linked into this executable.
    unsafe { runtime.register_factory_plugin_description(nc_example_com::factory_plugin_description()) }.unwrap();
    let factory = runtime
        .activate_plugin_factory::<ExampleFactoryAbi>(guid_from_qualified_name("nc.examples.factory_plugin_component"))
        .unwrap();
    let (sender, messages) = std::sync::mpsc::channel();
    let mut sink = None;
    let binding = count(|| sink = Some(mapped_sink::<ExampleObjectSinkAbi, _>(sender, |value: ExtentChanged| value.width).unwrap()));
    let events = ExampleObjectEventBinding::Observed(sink.unwrap());
    let mut object = factory.create(Extent { width: 0, height: 0 }, NcValueRef::new(&events).unwrap()).unwrap();
    let delivery = count(|| deliver(&mut object, &messages));
    println!("one typed COM receiver binding: {binding} allocations");
    println!("10,000 scalar events including channel storage: {delivery} allocations");
}
