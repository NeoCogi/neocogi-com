use nc_com::{
    ComClass, ComClassContract, ComInterface, ComPtr, ComRef, ComStruct, IUnknown,
    NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET, NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET,
    NC_ABI_PASSING_VOID, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE,
    NC_ABI_TYPE_U8, NC_ABI_TYPE_VOID, NC_COM_CORE_ABI_VERSION, NC_E_ABI_VERSION, NC_E_INVALID_ARG,
    NC_E_INVALID_TAG, NC_E_NOINTERFACE, NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_OK, NcAbiHash,
    NcAbiHashPart, NcAbiPassing, NcAbiThreadingMode, NcAbiType, NcClassDescriptor, NcClassTable,
    NcConstructorFn, NcCopyFromBufferOffset, NcCopyToBufferOffset, NcGuid, NcInterfaceId,
    NcPluginDependencyDescriptor, NcPluginDescriptor, NcStaticUtf8, NcStatus, Runtime, ThreadLocal,
    ThreadSafe, ThreadSafeComObject, ThreadingModel, com_class, com_enum, com_int_enum, com_object,
    com_interface, com_struct, format_method_signature, plugin,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::fmt::Write;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};

static TRACKED_DROPS: AtomicUsize = AtomicUsize::new(0);
static VOID_FLUSHES: AtomicUsize = AtomicUsize::new(0);
const NC_E_TEST_FAIL_CODE: i32 = -77;
const NC_E_TEST_VOID_FAIL_CODE: i32 = -88;
const NC_E_TEST_FAIL: NcStatus = NcStatus::from_code(NC_E_TEST_FAIL_CODE);
const NC_E_TEST_VOID_FAIL: NcStatus = NcStatus::from_code(NC_E_TEST_VOID_FAIL_CODE);

fn hash_hex(hash: NcAbiHash) -> String {
    let mut out = String::with_capacity(64);
    for byte in hash.bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

com_int_enum! {
    pub enum TestNodeKind: u16 {
        ELEMENT = 1,
        TEXT = 3,
        DOCUMENT = 9,
    }
}

#[com_enum(tag = u32)]
#[derive(Debug, PartialEq)]
pub enum TestMutation {
    #[com_tag(10)]
    None,
    ChildAdded(u64, u32),
    TextChanged {
        node: u64,
        len: u32,
    },
}

#[com_enum(tag = u8)]
#[derive(Debug, PartialEq)]
pub enum TestScalarPayload {
    #[com_tag(1)]
    Flag(bool),
    #[com_tag(2)]
    Byte(u8),
}

#[com_struct]
#[derive(Debug, PartialEq)]
pub struct TestRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[com_struct]
#[derive(Debug, PartialEq)]
pub struct TestDelta(pub i32, pub i32);

#[com_struct]
pub struct TestHeader {
    pub tag: u8,
    pub len: u32,
}

#[com_struct(packed)]
pub struct TestPackedHeader {
    pub tag: u8,
    pub len: u32,
}

#[com_struct(packed(2))]
pub struct TestPacked2Header {
    pub tag: u8,
    pub len: u32,
}

#[com_struct(align(16))]
pub struct TestAlignedHeader {
    pub tag: u8,
    pub len: u32,
}

#[com_interface(
    "bd15576f-7f3e-5eb1-8151-b6abb1e38e66",
    name = "nc.tests.com_usage.Calculator:v1"
)]
pub trait ICalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.Multiplier:v1")]
pub trait IMultiplier {
    fn multiply(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.Unsupported:v1")]
pub trait IUnsupported {
    fn noop(&self, value: i32) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.Tracked:v1")]
pub trait ITracked {
    fn value(&self) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.LifecycleTracked:v1")]
pub trait ILifecycleTracked {
    fn value(&self) -> Result<i32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.Failable:v1")]
pub trait IFailable {
    fn fail(&self) -> Result<u32, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.Geometry:v1")]
pub trait IGeometry {
    fn translate(&self, rect: TestRect, delta: TestDelta) -> Result<TestRect, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.VoidAction:v1")]
pub trait IVoidAction {
    fn flush(&self) -> Result<(), NcStatus>;
    fn fail_void(&self) -> Result<(), NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.ReferenceConsumer:v1")]
pub trait IReferenceConsumer {
    fn borrowed_value(&self, tracked: ComRef<'_, ITrackedAbi>) -> Result<i32, NcStatus>;
    fn make_tracked(&self, value: i32) -> Result<ComPtr<ITrackedAbi>, NcStatus>;
}

#[com_interface(auto_guid = "nc.tests.com_usage.BufferStore:v1")]
pub trait IBufferStore {
    fn write(&self, input: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus>;
    fn read(&self, output: NcCopyToBufferOffset<'_, u8>) -> Result<(), NcStatus>;
}

#[com_object(interfaces(ICalculator, IMultiplier))]
pub struct Calculator;

impl ICalculator for Calculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(a + b)
    }
}

impl IMultiplier for Calculator {
    fn multiply(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(a * b)
    }
}

/// Registered unit class used to exercise macro-generated plugin descriptors.
#[com_class(auto_guid = "nc.tests.com_usage.MacroPluginEcho:class:v1")]
pub trait MacroPluginEcho {
    /// Creates the stateless echo implementation.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;

    /// Returns the fixture's fixed answer.
    fn answer(&self) -> Result<u32, NcStatus>;
}

/// Private unit storage for the callable echo class.
#[com_object(MacroPluginEcho)]
pub struct MacroPluginEchoObject;

impl MacroPluginEcho for MacroPluginEchoObject {
    /// Creates the unit implementation without hidden constructor policy.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }

    /// Returns the fixture's fixed answer.
    fn answer(&self) -> Result<u32, NcStatus> {
        Ok(314)
    }
}

/// Registered non-unit class used to exercise no-argument typed construction.
#[com_class(auto_guid = "nc.tests.com_usage.MacroPluginDefault:class:v1")]
pub trait MacroPluginDefault {
    /// Creates the implementation with its documented default value.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;

    /// Returns the value initialized by the constructor.
    fn value(&self) -> Result<u32, NcStatus>;
}

/// Private state for the callable default-value fixture.
#[derive(Default)]
#[com_object(MacroPluginDefault)]
pub struct MacroPluginDefaultObject {
    /// Value returned through the class projection after construction.
    value: u32,
}

impl MacroPluginDefault for MacroPluginDefaultObject {
    /// Uses the ordinary Rust default as the explicit class implementation policy.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self::default())
    }

    /// Returns the constructor-initialized value.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

#[com_class(auto_guid = "nc.tests.com_usage.CallableCounter:v1")]
/// A registered class whose primary methods require no companion interface.
pub trait CallableCounter: ICalculator + IMultiplier {
    /// Creates a counter holding one caller-supplied value.
    #[constructor]
    fn new(value: u32, label: NcCopyFromBufferOffset<'_, u8>, calculator: ComRef<'_, ICalculatorAbi>) -> Result<Self, NcStatus>;

    /// Returns the fixed value supplied by this test implementation.
    fn answer(&self) -> Result<u32, NcStatus>;
}

#[com_object(CallableCounter)]
/// Private state object implementing the shared callable class contract.
pub struct CallableCounterObject {
    /// Fixed answer supplied by the typed constructor frame.
    value: u32,
}

impl CallableCounter for CallableCounterObject {
    /// Creates the stateless implementation value.
    fn new(value: u32, label: NcCopyFromBufferOffset<'_, u8>, calculator: ComRef<'_, ICalculatorAbi>) -> Result<Self, NcStatus> {
        Ok(Self {
            value: value + label.len() as u32 + calculator.add(0, 0)? as u32,
        })
    }

    /// Implements the class method directly on the private plugin object.
    fn answer(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
}

impl ICalculator for CallableCounterObject {
    /// Uses the constructor value to prove the primary base projection reaches the same state.
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(self.value as i32 + a + b)
    }
}

impl IMultiplier for CallableCounterObject {
    /// Returns a sibling-interface result derived from the same constructor state.
    fn multiply(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(self.value as i32 + a * b)
    }
}

plugin! {
    plugin_id(auto_guid = "nc.tests.com_usage.macro_plugin:v1"),
    name = "nc.tests.com_usage.macro_plugin:v1",
    version = "1.0.0",
    dependencies = [CALCULATOR_PLUGIN_ID],
    classes = [MacroPluginEchoObject, CallableCounterObject],
}

const CALCULATOR_PLUGIN_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.tests.com_usage.calculator_plugin");

const CALCULATOR_CLASS_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.tests.com_usage.Calculator");

static CALCULATOR_CLASSES: [NcClassDescriptor; 2] = [
    NcClassDescriptor {
        class_id: CALCULATOR_CLASS_ID,
        qualified_name: NcStaticUtf8::from_static("nc.tests.com_usage.Calculator"),
        class_version: NcStaticUtf8::from_static("1.0.0"),
        constructor_abi_hash: NcAbiHash::ZERO,
        threading: NC_ABI_THREADING_THREAD_LOCAL,
        construct: Some(construct_calculator_class as NcConstructorFn),
        interface_count: Calculator::COM_INTERFACES.len() as u32,
        interfaces: Calculator::COM_INTERFACES.as_ptr(),
    },
    ThreadSafeCalculatorObject::CLASS_DESCRIPTOR,
];

static CALCULATOR_CLASS_TABLE: NcClassTable = NcClassTable {
    class_count: CALCULATOR_CLASSES.len() as u32,
    classes: CALCULATOR_CLASSES.as_ptr(),
};

static CALCULATOR_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    plugin_id: CALCULATOR_PLUGIN_ID,
    qualified_name: NcStaticUtf8::from_static("nc.tests.com_usage.calculator_plugin"),
    plugin_version: NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: core::ptr::null(),
    class_table: &CALCULATOR_CLASS_TABLE,
};

const UNCHECKED_NO_ARGS_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.tests.com_usage.UncheckedNoArgs");
const UNCHECKED_ARGS_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.tests.com_usage.UncheckedArgs");
const UNCHECKED_WRONG_HASH_ARGS_CLASS_ID: NcGuid = nc_com::guid_from_qualified_name("nc.tests.com_usage.UncheckedWrongHashArgs");
const TRACKED_CONSTRUCTOR_ABI_HASH: NcAbiHash = NcAbiHash::from_abi_parts(&[NcAbiHashPart::Arg {
    index: 0,
    passing: NcAbiPassing::Value,
    ty: NcAbiType::U32,
}]);
const WRONG_TRACKED_CONSTRUCTOR_ABI_HASH: NcAbiHash = NcAbiHash::from_bytes([0x5a; 32]);

/// Concrete immutable frame used to exercise low-level runtime validation.
#[repr(C)]
struct TrackedConstructorFrame {
    /// Plain seed copied into the frame by the caller.
    value: u32,
}

static UNCHECKED_CUSTOM_CONSTRUCTOR_CALLS: AtomicUsize = AtomicUsize::new(0);

static UNCHECKED_CONSTRUCTOR_CLASSES: [NcClassDescriptor; 3] = [
    NcClassDescriptor {
        class_id: UNCHECKED_NO_ARGS_CLASS_ID,
        qualified_name: NcStaticUtf8::from_static("nc.tests.com_usage.UncheckedNoArgs"),
        class_version: NcStaticUtf8::from_static("1.0.0"),
        constructor_abi_hash: NcAbiHash::ZERO,
        threading: NC_ABI_THREADING_THREAD_LOCAL,
        construct: Some(unchecked_custom_constructor as NcConstructorFn),
        interface_count: Calculator::COM_INTERFACES.len() as u32,
        interfaces: Calculator::COM_INTERFACES.as_ptr(),
    },
    NcClassDescriptor {
        class_id: UNCHECKED_ARGS_CLASS_ID,
        qualified_name: NcStaticUtf8::from_static("nc.tests.com_usage.UncheckedArgs"),
        class_version: NcStaticUtf8::from_static("1.0.0"),
        constructor_abi_hash: TRACKED_CONSTRUCTOR_ABI_HASH,
        threading: NC_ABI_THREADING_THREAD_LOCAL,
        construct: Some(unchecked_custom_constructor as NcConstructorFn),
        interface_count: Calculator::COM_INTERFACES.len() as u32,
        interfaces: Calculator::COM_INTERFACES.as_ptr(),
    },
    NcClassDescriptor {
        class_id: UNCHECKED_WRONG_HASH_ARGS_CLASS_ID,
        qualified_name: NcStaticUtf8::from_static("nc.tests.com_usage.UncheckedWrongHashArgs"),
        class_version: NcStaticUtf8::from_static("1.0.0"),
        constructor_abi_hash: WRONG_TRACKED_CONSTRUCTOR_ABI_HASH,
        threading: NC_ABI_THREADING_THREAD_LOCAL,
        construct: Some(unchecked_custom_constructor as NcConstructorFn),
        interface_count: Calculator::COM_INTERFACES.len() as u32,
        interfaces: Calculator::COM_INTERFACES.as_ptr(),
    },
];

static UNCHECKED_CONSTRUCTOR_CLASS_TABLE: NcClassTable = NcClassTable {
    class_count: UNCHECKED_CONSTRUCTOR_CLASSES.len() as u32,
    classes: UNCHECKED_CONSTRUCTOR_CLASSES.as_ptr(),
};

static UNCHECKED_CONSTRUCTOR_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    plugin_id: nc_com::guid_from_qualified_name("nc.tests.com_usage.unchecked_constructor_plugin"),
    qualified_name: NcStaticUtf8::from_static("nc.tests.com_usage.unchecked_constructor_plugin"),
    plugin_version: NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: core::ptr::null(),
    class_table: &UNCHECKED_CONSTRUCTOR_CLASS_TABLE,
};

const FAILING_CONSTRUCTOR_CLASS_ID: NcGuid =
    nc_com::guid_from_qualified_name("nc.tests.com_usage.FailingConstructor");

static FAILING_CONSTRUCTOR_CLASSES: [NcClassDescriptor; 1] = [NcClassDescriptor {
    class_id: FAILING_CONSTRUCTOR_CLASS_ID,
    qualified_name: NcStaticUtf8::from_static("nc.tests.com_usage.FailingConstructor"),
    class_version: NcStaticUtf8::from_static("1.0.0"),
    constructor_abi_hash: NcAbiHash::ZERO,
    threading: NC_ABI_THREADING_THREAD_LOCAL,
    construct: Some(failing_custom_constructor as NcConstructorFn),
    interface_count: Calculator::COM_INTERFACES.len() as u32,
    interfaces: Calculator::COM_INTERFACES.as_ptr(),
}];

static FAILING_CONSTRUCTOR_CLASS_TABLE: NcClassTable = NcClassTable {
    class_count: FAILING_CONSTRUCTOR_CLASSES.len() as u32,
    classes: FAILING_CONSTRUCTOR_CLASSES.as_ptr(),
};

static FAILING_CONSTRUCTOR_PLUGIN: NcPluginDescriptor = NcPluginDescriptor {
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    plugin_id: nc_com::guid_from_qualified_name("nc.tests.com_usage.failing_constructor_plugin"),
    qualified_name: NcStaticUtf8::from_static("nc.tests.com_usage.failing_constructor_plugin"),
    plugin_version: NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: core::ptr::null(),
    class_table: &FAILING_CONSTRUCTOR_CLASS_TABLE,
};

unsafe extern "C" fn construct_calculator_class(class: *const NcClassDescriptor, arguments: *const c_void, out: *mut *mut IUnknown) -> NcStatus {
    unsafe {
        nc_com::construct_class_checked(class, arguments, out, CALCULATOR_CLASS_ID, NcAbiHash::ZERO, |_arguments| {
            let unknown: ComPtr<IUnknown> = Calculator::from_value(Calculator);
            Ok(unknown)
        })
    }
}

unsafe extern "C" fn unchecked_custom_constructor(_class: *const NcClassDescriptor, _arguments: *const c_void, out: *mut *mut IUnknown) -> NcStatus {
    UNCHECKED_CUSTOM_CONSTRUCTOR_CALLS.fetch_add(1, Ordering::SeqCst);
    if out.is_null() {
        return NC_E_POINTER;
    }
    let unknown: ComPtr<IUnknown> = Calculator::from_value(Calculator);
    unsafe {
        *out = unknown.into_raw();
    }
    NC_OK
}

unsafe extern "C" fn failing_custom_constructor(_class: *const NcClassDescriptor, _arguments: *const c_void, out: *mut *mut IUnknown) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe {
        *out = core::ptr::NonNull::<IUnknown>::dangling().as_ptr();
    }
    NC_E_TEST_FAIL
}

/// Registered calculator class whose objects may cross thread boundaries.
#[com_class(auto_guid = "nc.tests.com_usage.ThreadSafeCalculator")]
pub trait ThreadSafeCalculator: ICalculator {
    /// Creates the stateless calculator implementation.
    #[constructor]
    fn new() -> Result<Self, NcStatus>;
}

/// Private thread-safe unit storage for the registered calculator class.
#[com_object(thread_safe, ThreadSafeCalculator)]
pub struct ThreadSafeCalculatorObject;

impl ThreadSafeCalculator for ThreadSafeCalculatorObject {
    /// Creates the unit implementation without auxiliary state.
    fn new() -> Result<Self, NcStatus> {
        Ok(Self)
    }
}

impl ICalculator for ThreadSafeCalculatorObject {
    /// Adds two operands without accessing mutable state.
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(a + b)
    }
}

#[com_object(interfaces(ITracked))]
pub struct TrackedObject {
    value: i32,
}

impl ITracked for TrackedObject {
    fn value(&self) -> Result<i32, NcStatus> {
        Ok(self.value)
    }
}

impl Drop for TrackedObject {
    fn drop(&mut self) {
        TRACKED_DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

#[com_object(interfaces(ILifecycleTracked))]
pub struct LifecycleTrackedObject {
    value: i32,
    drops: Arc<AtomicUsize>,
}

impl ILifecycleTracked for LifecycleTrackedObject {
    fn value(&self) -> Result<i32, NcStatus> {
        Ok(self.value)
    }
}

impl Drop for LifecycleTrackedObject {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

fn lifecycle_tracked(value: i32) -> (ComPtr<IUnknown>, Arc<AtomicUsize>) {
    let drops = Arc::new(AtomicUsize::new(0));
    let unknown = LifecycleTrackedObject::from_value(LifecycleTrackedObject {
        value,
        drops: Arc::clone(&drops),
    });
    (unknown, drops)
}

#[com_object(thread_safe, interfaces(ILifecycleTracked, ITracked))]
pub struct ThreadSafeLifecycleTrackedObject {
    value: i32,
    drops: Arc<AtomicUsize>,
}

impl ILifecycleTracked for ThreadSafeLifecycleTrackedObject {
    fn value(&self) -> Result<i32, NcStatus> {
        Ok(self.value)
    }
}

impl ITracked for ThreadSafeLifecycleTrackedObject {
    fn value(&self) -> Result<i32, NcStatus> {
        Ok(self.value)
    }
}

impl Drop for ThreadSafeLifecycleTrackedObject {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

fn thread_safe_lifecycle_tracked(value: i32) -> (ComPtr<IUnknown, ThreadSafe>, Arc<AtomicUsize>) {
    let drops = Arc::new(AtomicUsize::new(0));
    let unknown = ThreadSafeLifecycleTrackedObject::from_value(ThreadSafeLifecycleTrackedObject {
        value,
        drops: Arc::clone(&drops),
    });
    (unknown, drops)
}

#[com_object(interfaces(IFailable))]
pub struct FailableObject;

impl IFailable for FailableObject {
    fn fail(&self) -> Result<u32, NcStatus> {
        Err(NC_E_TEST_FAIL)
    }
}

#[com_object(interfaces(IGeometry))]
pub struct Geometry;

impl IGeometry for Geometry {
    fn translate(&self, rect: TestRect, delta: TestDelta) -> Result<TestRect, NcStatus> {
        Ok(TestRect {
            x: rect.x + delta.0,
            y: rect.y + delta.1,
            width: rect.width,
            height: rect.height,
        })
    }
}

#[com_object(interfaces(IVoidAction))]
pub struct VoidAction;

impl IVoidAction for VoidAction {
    fn flush(&self) -> Result<(), NcStatus> {
        VOID_FLUSHES.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn fail_void(&self) -> Result<(), NcStatus> {
        Err(NC_E_TEST_VOID_FAIL)
    }
}

#[com_object(interfaces(IReferenceConsumer))]
pub struct ReferenceConsumer;

impl IReferenceConsumer for ReferenceConsumer {
    fn borrowed_value(&self, tracked: ComRef<'_, ITrackedAbi>) -> Result<i32, NcStatus> {
        tracked.value()
    }

    fn make_tracked(&self, value: i32) -> Result<ComPtr<ITrackedAbi>, NcStatus> {
        let unknown = TrackedObject::from_value(TrackedObject { value });
        unknown.query()
    }
}

#[com_object(interfaces(IBufferStore))]
pub struct BufferStore {
    bytes: RefCell<Vec<u8>>,
}

impl BufferStore {
    fn with_bytes(bytes: Vec<u8>) -> ComPtr<IUnknown> {
        Self::from_value(Self {
            bytes: RefCell::new(bytes),
        })
    }
}

impl IBufferStore for BufferStore {
    fn write(&self, input: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        let offset = usize::try_from(input.offset()).map_err(|_| NC_E_OUT_OF_RANGE)?;
        let end = offset.checked_add(input.len()).ok_or(NC_E_OUT_OF_RANGE)?;
        let mut bytes = self.bytes.borrow_mut();
        if bytes.len() < end {
            bytes.resize(end, 0);
        }
        bytes[offset..end].copy_from_slice(input.as_slice());
        Ok(())
    }

    fn read(&self, mut output: NcCopyToBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        let offset = usize::try_from(output.offset()).map_err(|_| NC_E_OUT_OF_RANGE)?;
        let end = offset.checked_add(output.len()).ok_or(NC_E_OUT_OF_RANGE)?;
        let bytes = self.bytes.borrow();
        let source = bytes.get(offset..end).ok_or(NC_E_OUT_OF_RANGE)?;
        output.as_mut_slice().copy_from_slice(source);
        Ok(())
    }
}

mod path_interfaces {
    pub mod dom {
        use nc_com::{NcStatus, com_interface};

        #[com_interface(auto_guid = "nc.tests.com_usage.path.Element:v1")]
        pub trait IPathElement {
            fn id(&self) -> Result<u64, NcStatus>;
        }
    }
}

#[com_object(interfaces(path_interfaces::dom::IPathElement))]
pub struct PathElement {
    id: u64,
}

impl path_interfaces::dom::IPathElement for PathElement {
    fn id(&self) -> Result<u64, NcStatus> {
        Ok(self.id)
    }
}

#[test]
fn calls_interface_methods_through_generated_vtables() {
    let unknown: ComPtr<IUnknown> = Calculator::create();

    let calculator = unknown.query::<ICalculatorAbi>().unwrap();
    assert_eq!(calculator.add(2, 3).unwrap(), 5);

    let multiplier = calculator.query::<IMultiplierAbi>().unwrap();
    assert_eq!(multiplier.multiply(4, 5).unwrap(), 20);
}

#[test]
fn thread_safe_classes_produce_send_sync_comptrs() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<ComPtr<IUnknown, ThreadSafe>>();
    assert_send_sync::<ComPtr<ICalculatorAbi, ThreadSafe>>();

    let unknown: ComPtr<IUnknown, ThreadSafe> = ThreadSafeCalculatorObject::create();
    let calculator: ComPtr<ICalculatorAbi, ThreadSafe> = unknown.query().unwrap();

    let handle = std::thread::spawn(move || calculator.add(21, 21).unwrap());
    assert_eq!(handle.join().unwrap(), 42);
}

#[test]
fn generated_class_threading_is_the_pointer_and_descriptor_source_of_truth() {
    fn assert_class_mode<C, M>()
    where
        C: ComClass<Threading = M>,
        M: ThreadingModel,
    {
    }

    fn assert_thread_safe_class<C: ThreadSafeComObject>() {}

    assert_class_mode::<Calculator, ThreadLocal>();
    assert_class_mode::<ThreadSafeCalculatorObject, ThreadSafe>();
    assert_thread_safe_class::<ThreadSafeCalculatorObject>();

    assert_eq!(
        MacroPluginEchoObject::CLASS_DESCRIPTOR.threading,
        <<MacroPluginEchoObject as ComClass>::Threading as ThreadingModel>::ABI_THREADING_MODE
            .as_raw()
    );
    assert_eq!(
        ThreadSafeCalculatorObject::CLASS_DESCRIPTOR.threading,
        <<ThreadSafeCalculatorObject as ComClass>::Threading as ThreadingModel>::ABI_THREADING_MODE
            .as_raw()
    );
}

#[test]
fn owned_thread_safe_pointer_weakening_transfers_exactly_one_reference() {
    let (unknown, drops) = thread_safe_lifecycle_tracked(707);
    let raw = unknown.as_ptr();
    let remaining_thread_safe_owner = unknown.clone();

    let local: ComPtr<IUnknown, ThreadLocal> = unknown.into_thread_local();
    assert_eq!(local.as_ptr(), raw);

    let tracked: ComPtr<ILifecycleTrackedAbi, ThreadLocal> = local.query().unwrap();
    assert_eq!(tracked.value(), Ok(707));

    drop(tracked);
    drop(local);
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    drop(remaining_thread_safe_owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn borrowed_thread_safe_pointer_weakening_preserves_lifetime_and_mode() {
    let (unknown, drops) = thread_safe_lifecycle_tracked(808);

    let from_owner: ComPtr<IUnknown, ThreadLocal> = unknown.as_thread_local_ref().add_ref();
    let from_borrow: ComPtr<IUnknown, ThreadLocal> =
        unknown.as_com_ref().into_thread_local().add_ref();

    drop(unknown);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(from_owner);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(from_borrow);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the linked runtime C ABI")]
fn runtime_is_shareable_as_process_wide_state() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Runtime>();

    let runtime = Runtime::global();
    assert_eq!(
        unsafe { runtime.register_plugin_descriptor(&CALCULATOR_PLUGIN) },
        NC_OK
    );

    let construction_runtime = runtime;
    let construction_thread = std::thread::spawn(move || {
        let calculator = construction_runtime.construct_class_as::<ICalculatorAbi>(CALCULATOR_CLASS_ID, ()).unwrap();
        calculator.add(20, 22).unwrap()
    });

    let validation_runtime = runtime;
    let validation_thread = std::thread::spawn(move || {
        validation_runtime
            .require_interface_type::<ICalculatorAbi>(CALCULATOR_CLASS_ID)
            .unwrap();
    });

    assert_eq!(construction_thread.join().unwrap(), 42);
    validation_thread.join().unwrap();
}

#[test]
fn com_class_accepts_interface_paths() {
    use path_interfaces::dom::{IPathElementAbi, IPathElementComPtrExt};

    let unknown = PathElement::from_value(PathElement { id: 99 });
    let element = unknown.query::<IPathElementAbi>().unwrap();

    assert_eq!(element.id().unwrap(), 99);
}

#[test]
fn com_ref_arguments_are_borrowed_interface_parameters() {
    let consumer = ReferenceConsumer::create()
        .query::<IReferenceConsumerAbi>()
        .unwrap();
    let tracked = TrackedObject::from_value(TrackedObject { value: 55 })
        .query::<ITrackedAbi>()
        .unwrap();

    assert_eq!(consumer.borrowed_value(tracked.as_com_ref()).unwrap(), 55);

    let thread_safe_tracked =
        ThreadSafeLifecycleTrackedObject::from_value(ThreadSafeLifecycleTrackedObject {
            value: 56,
            drops: Arc::new(AtomicUsize::new(0)),
        })
        .query::<ITrackedAbi>()
        .unwrap();
    assert_eq!(
        consumer
            .borrowed_value(thread_safe_tracked.as_thread_local_ref())
            .unwrap(),
        56
    );

    let raw = consumer.as_mut_ptr();
    let mut out = 0;
    let status = unsafe { ((*(*raw).vtable).borrowed_value)(raw, core::ptr::null_mut(), &mut out) };
    assert_eq!(status, NC_E_POINTER);
}

#[test]
fn com_ptr_returns_transfer_owned_interface_references() {
    let consumer = ReferenceConsumer::create()
        .query::<IReferenceConsumerAbi>()
        .unwrap();

    let tracked = consumer.make_tracked(64).unwrap();

    assert_eq!(tracked.value().unwrap(), 64);
}

#[test]
fn com_struct_records_cross_interface_methods_by_value() {
    let geometry = Geometry::create().query::<IGeometryAbi>().unwrap();
    let rect = TestRect {
        x: 10,
        y: 20,
        width: 30,
        height: 40,
    };

    let moved = geometry.translate(rect, TestDelta(-3, 7)).unwrap();

    assert_eq!(
        moved,
        TestRect {
            x: 7,
            y: 27,
            width: 30,
            height: 40,
        }
    );
}

#[test]
fn com_struct_layout_options_are_part_of_the_abi_type() {
    fn assert_header_layout<T: ComStruct + nc_com::AbiType>(
        size: usize,
        align: usize,
        len_offset: usize,
    ) {
        let layout = <T as ComStruct>::ABI_STRUCT_LAYOUT;
        assert_eq!(layout.size, size);
        assert_eq!(layout.align, align);
        assert_eq!(layout.fields.len(), 2);
        assert_eq!(layout.fields[0].index, 0);
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[0].ty, NcAbiType::U8);
        assert_eq!(layout.fields[1].index, 1);
        assert_eq!(layout.fields[1].offset, len_offset);
        assert_eq!(layout.fields[1].ty, NcAbiType::U32);
        assert_eq!(
            <T as nc_com::AbiType>::ABI_TYPE,
            NcAbiType::Aggregate(NcAbiHash::from_struct_layout(layout))
        );
    }

    assert_header_layout::<TestHeader>(core::mem::size_of::<TestHeader>(), 4, 4);
    assert_header_layout::<TestPackedHeader>(5, 1, 1);
    assert_header_layout::<TestPacked2Header>(6, 2, 2);
    assert_header_layout::<TestAlignedHeader>(16, 16, 4);

    let normal = <TestHeader as nc_com::AbiType>::ABI_TYPE;
    let packed = <TestPackedHeader as nc_com::AbiType>::ABI_TYPE;
    let packed2 = <TestPacked2Header as nc_com::AbiType>::ABI_TYPE;
    let aligned = <TestAlignedHeader as nc_com::AbiType>::ABI_TYPE;

    assert_ne!(normal, packed);
    assert_ne!(normal, packed2);
    assert_ne!(normal, aligned);
    assert_ne!(packed, packed2);
}

#[test]
fn generated_interface_and_vtable_keep_iunknown_prefix_layout() {
    assert_eq!(
        core::mem::size_of::<ICalculatorAbi>(),
        core::mem::size_of::<IUnknown>()
    );
    assert_eq!(
        core::mem::align_of::<ICalculatorAbi>(),
        core::mem::align_of::<IUnknown>()
    );
    assert_eq!(core::mem::offset_of!(ICalculatorAbi, vtable), 0);
    assert_eq!(
        core::mem::offset_of!(ICalculatorAbi, object),
        core::mem::offset_of!(IUnknown, object)
    );
    assert_eq!(core::mem::offset_of!(ICalculatorVTable, unknown), 0);

    assert!(ICalculatorAbi::iunknown(core::ptr::null_mut()).is_null());
    let raw = core::ptr::NonNull::<ICalculatorAbi>::dangling().as_ptr();
    assert_eq!(ICalculatorAbi::iunknown(raw), raw.cast::<IUnknown>());
}

#[test]
fn generated_metadata_describes_interfaces_and_classes() {
    assert_ne!(ICalculatorAbi::ABI_HASH, NcAbiHash::ZERO);
    assert_ne!(ICalculatorAbi::ABI_HASH, IMultiplierAbi::ABI_HASH);
    assert_eq!(
        ICalculatorAbi::INTERFACE_DESCRIPTOR.interface_id,
        ICalculatorAbi::IID
    );
    assert_eq!(ICalculatorAbi::INTERFACE_DESCRIPTOR.method_count, 1);
    assert!(!ICalculatorAbi::INTERFACE_DESCRIPTOR.methods.is_null());

    let methods = unsafe {
        std::slice::from_raw_parts(
            ICalculatorAbi::INTERFACE_DESCRIPTOR.methods,
            ICalculatorAbi::INTERFACE_DESCRIPTOR.method_count as usize,
        )
    };
    assert_eq!(unsafe { methods[0].name.as_str() }.unwrap(), "add");
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&methods[0]) }.unwrap(),
        "add(a: value<i32>, b: value<i32>) -> out<i32> status"
    );

    assert_eq!(Calculator::COM_INTERFACES.len(), 3);
    let iunknown_descriptor = unsafe { &*Calculator::COM_INTERFACES[0].interface_descriptor };
    let calculator_descriptor = unsafe { &*Calculator::COM_INTERFACES[1].interface_descriptor };
    let multiplier_descriptor = unsafe { &*Calculator::COM_INTERFACES[2].interface_descriptor };
    assert_eq!(iunknown_descriptor.interface_id, IUnknown::IID);
    assert_eq!(iunknown_descriptor.abi_hash, IUnknown::ABI_HASH);
    assert_eq!(iunknown_descriptor.method_count, 0);
    assert_eq!(calculator_descriptor.interface_id, ICalculatorAbi::IID);
    assert_eq!(calculator_descriptor.abi_hash, ICalculatorAbi::ABI_HASH);
    assert_eq!(multiplier_descriptor.interface_id, IMultiplierAbi::IID);
    assert_eq!(
        MacroPluginEchoObject::CLASS_DESCRIPTOR.threading,
        NC_ABI_THREADING_THREAD_LOCAL
    );
    assert_eq!(
        ThreadSafeCalculatorObject::CLASS_DESCRIPTOR.threading,
        NC_ABI_THREADING_THREAD_SAFE
    );
}

#[test]
fn generated_interface_hashes_include_referenced_interface_iids() {
    let expected = NcAbiHash::from_abi_parts(&[
        NcAbiHashPart::Interface {
            iid: IReferenceConsumerAbi::IID,
        },
        NcAbiHashPart::Method {
            index: 0,
            name: "borrowed_value",
        },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::ComRef { mode: NcAbiThreadingMode::ThreadLocal },
            ty: NcAbiType::Interface { iid: ITrackedAbi::IID },
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Out,
            ty: NcAbiType::I32,
        },
        NcAbiHashPart::Status,
        NcAbiHashPart::Method {
            index: 1,
            name: "make_tracked",
        },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::Value,
            ty: NcAbiType::I32,
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::ComPtrOut { mode: NcAbiThreadingMode::ThreadLocal },
            ty: NcAbiType::Interface { iid: ITrackedAbi::IID },
        },
        NcAbiHashPart::Status,
    ]);
    let with_different_referenced_iid = NcAbiHash::from_abi_parts(&[
        NcAbiHashPart::Interface { iid: IReferenceConsumerAbi::IID },
        NcAbiHashPart::Method { index: 0, name: "borrowed_value" },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::ComRef { mode: NcAbiThreadingMode::ThreadLocal },
            ty: NcAbiType::Interface { iid: ICalculatorAbi::IID },
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::Out,
            ty: NcAbiType::I32,
        },
        NcAbiHashPart::Status,
        NcAbiHashPart::Method {
            index: 1,
            name: "make_tracked",
        },
        NcAbiHashPart::Arg {
            index: 0,
            passing: NcAbiPassing::Value,
            ty: NcAbiType::I32,
        },
        NcAbiHashPart::Return {
            passing: NcAbiPassing::ComPtrOut { mode: NcAbiThreadingMode::ThreadLocal },
            ty: NcAbiType::Interface { iid: ICalculatorAbi::IID },
        },
        NcAbiHashPart::Status,
    ]);

    assert_eq!(IReferenceConsumerAbi::ABI_HASH, expected);
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&IReferenceConsumerAbi::METHOD_DESCRIPTORS[0]) }.unwrap(),
        format!(
            "borrowed_value(tracked: com_ref<thread_local, interface<{}, {}>>) -> out<i32> status",
            ITrackedAbi::IID,
            hash_hex(ITrackedAbi::ABI_HASH)
        )
    );
    assert_ne!(IReferenceConsumerAbi::ABI_HASH, with_different_referenced_iid);
}

#[test]
fn generated_buffer_parameters_are_safe_and_described_as_typed_abi() {
    let methods = IBufferStoreAbi::METHOD_DESCRIPTORS;
    assert_eq!(methods[0].param_count, 1);
    let write_params = unsafe { std::slice::from_raw_parts(methods[0].params, 1) };
    assert_eq!(unsafe { write_params[0].name.as_str() }.unwrap(), "input");
    assert_eq!(
        write_params[0].passing.tag,
        NC_ABI_PASSING_COPY_FROM_BUFFER_OFFSET
    );
    assert_eq!(
        write_params[0].passing.threading,
        NC_ABI_THREADING_THREAD_LOCAL
    );
    assert_eq!(write_params[0].ty.tag, NC_ABI_TYPE_U8);
    assert_eq!(methods[0].returns.passing.tag, NC_ABI_PASSING_VOID);
    assert_eq!(methods[0].returns.ty.tag, NC_ABI_TYPE_VOID);
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&methods[0]) }.unwrap(),
        "write(input: copy_from_buffer_offset<u8>) -> void status"
    );

    assert_eq!(methods[1].param_count, 1);
    let read_params = unsafe { std::slice::from_raw_parts(methods[1].params, 1) };
    assert_eq!(unsafe { read_params[0].name.as_str() }.unwrap(), "output");
    assert_eq!(
        read_params[0].passing.tag,
        NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET
    );
    assert_eq!(
        read_params[0].passing.threading,
        NC_ABI_THREADING_THREAD_LOCAL
    );
    assert_eq!(read_params[0].ty.tag, NC_ABI_TYPE_U8);
    assert_eq!(methods[1].returns.passing.tag, NC_ABI_PASSING_VOID);
    assert_eq!(methods[1].returns.ty.tag, NC_ABI_TYPE_VOID);
    assert_eq!(
        // SAFETY: Macro-generated method descriptors and their transitive
        // metadata are immutable static values.
        unsafe { format_method_signature(&methods[1]) }.unwrap(),
        "read(output: copy_to_buffer_offset<u8>) -> void status"
    );

    let unknown = BufferStore::with_bytes(vec![1, 2, 3, 4]);
    let store = unknown.query::<IBufferStoreAbi>().unwrap();

    store
        .write(NcCopyFromBufferOffset::new(2, &[9, 8, 7]))
        .unwrap();

    let mut out = [0; 5];
    store.read(NcCopyToBufferOffset::new(0, &mut out)).unwrap();
    assert_eq!(out, [1, 2, 9, 8, 7]);

    let raw = store.as_mut_ptr();
    let raw_input = [6, 5];
    let status =
        unsafe { ((*(*raw).vtable).write)(raw, 1, raw_input.len() as u64, raw_input.as_ptr()) };
    assert_eq!(status, NC_OK);

    let mut raw_output = [0; 2];
    let status = unsafe {
        ((*(*raw).vtable).read)(raw, 1, raw_output.len() as u64, raw_output.as_mut_ptr())
    };
    assert_eq!(status, NC_OK);
    assert_eq!(raw_output, raw_input);

    let status = unsafe { ((*(*raw).vtable).write)(raw, 0, 1, core::ptr::null()) };
    assert_eq!(status, NC_E_POINTER);
    let status = unsafe { ((*(*raw).vtable).read)(raw, 0, 1, core::ptr::null_mut()) };
    assert_eq!(status, NC_E_POINTER);

    let mut too_large = [0; 1];
    let status = store
        .read(NcCopyToBufferOffset::new(32, &mut too_large))
        .unwrap_err();
    assert_eq!(status, NC_E_OUT_OF_RANGE);
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the linked runtime C ABI")]
fn runtime_registers_plugin_and_constructs_objects() {
    let runtime = Runtime::global();

    assert_eq!(
        unsafe { runtime.register_plugin_descriptor(&CALCULATOR_PLUGIN) },
        NC_OK
    );
    assert!(
        runtime
            .plugin_registry()
            .plugin_ids()
            .unwrap()
            .contains(&CALCULATOR_PLUGIN.plugin_id)
    );
    assert!(
        runtime
            .type_registry()
            .class_ids()
            .unwrap()
            .contains(&CALCULATOR_CLASS_ID)
    );
    let registered_interfaces = runtime.type_registry().interface_ids().unwrap();
    assert!(registered_interfaces.contains(&ICalculatorAbi::INTERFACE_ID));
    assert_eq!(
        registered_interfaces
            .iter()
            .filter(|interface| **interface == IUnknown::INTERFACE_ID)
            .count(),
        1
    );
    assert_eq!(
        runtime
            .inspection()
            .plugin_class_ids(CALCULATOR_PLUGIN.plugin_id)
            .unwrap(),
        vec![CALCULATOR_CLASS_ID, ThreadSafeCalculatorAbi::CLASS_ID]
    );
    let class_interfaces = runtime
        .inspection()
        .class_interface_ids(CALCULATOR_CLASS_ID)
        .unwrap();
    assert_eq!(class_interfaces[0], IUnknown::INTERFACE_ID);
    assert_eq!(
        class_interfaces
            .iter()
            .filter(|interface| **interface == IUnknown::INTERFACE_ID)
            .count(),
        1
    );
    assert!(class_interfaces.contains(&ICalculatorAbi::INTERFACE_ID));
    assert!(
        runtime
            .inspection()
            .interface_implementor_class_ids(ICalculatorAbi::INTERFACE_ID)
            .unwrap()
            .contains(&CALCULATOR_CLASS_ID)
    );
    assert!(
        runtime
            .inspection()
            .interface_implementor_class_ids(IUnknown::INTERFACE_ID)
            .unwrap()
            .contains(&CALCULATOR_CLASS_ID)
    );
    assert_eq!(
        runtime
            .inspection()
            .module_id_for_class(CALCULATOR_CLASS_ID)
            .unwrap(),
        None
    );
    let plugin_info = runtime
        .inspection()
        .plugin_info(CALCULATOR_PLUGIN.plugin_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        plugin_info.qualified_name,
        "nc.tests.com_usage.calculator_plugin"
    );
    let class_info = runtime
        .inspection()
        .class_info(CALCULATOR_CLASS_ID)
        .unwrap()
        .unwrap();
    assert_eq!(class_info.plugin_id, CALCULATOR_PLUGIN.plugin_id);
    assert_eq!(class_info.module_id, None);
    assert_eq!(class_info.constructor_abi_hash, NcAbiHash::ZERO);
    assert_eq!(class_info.threading, NcAbiThreadingMode::ThreadLocal);
    let thread_safe_class_info = runtime
        .inspection()
        .class_info(ThreadSafeCalculatorAbi::CLASS_ID)
        .unwrap()
        .unwrap();
    assert_eq!(
        thread_safe_class_info.threading,
        NcAbiThreadingMode::ThreadSafe
    );
    runtime
        .require_interface_type::<ICalculatorAbi>(CALCULATOR_CLASS_ID)
        .unwrap();
    runtime
        .require_interface_type::<IUnknown>(CALCULATOR_CLASS_ID)
        .unwrap();

    assert_eq!(
        runtime.require_interface(
            CALCULATOR_CLASS_ID,
            NcInterfaceId::new(ICalculatorAbi::IID, NcAbiHash::ZERO),
        ),
        Err(NC_E_ABI_VERSION)
    );

    let calculator = runtime.construct_class_as::<ICalculatorAbi>(CALCULATOR_CLASS_ID, ()).unwrap();
    assert_eq!(calculator.add(19, 23).unwrap(), 42);

    assert_eq!(
        runtime
            .construct_class_as_with_model::<ICalculatorAbi, ThreadSafe>(CALCULATOR_CLASS_ID, (),)
            .unwrap_err(),
        NC_E_INVALID_ARG
    );

    let thread_safe_calculator = runtime
        .construct_class_as_with_model::<ICalculatorAbi, ThreadSafe>(
            ThreadSafeCalculatorAbi::CLASS_ID,
            (),
        )
        .unwrap();
    let thread_result = std::thread::spawn(move || thread_safe_calculator.add(20, 22).unwrap());
    assert_eq!(thread_result.join().unwrap(), 42);

    let downgraded_calculator = runtime
        .construct_class_as::<ICalculatorAbi>(ThreadSafeCalculatorAbi::CLASS_ID, ())
        .unwrap();
    assert_eq!(downgraded_calculator.add(40, 2).unwrap(), 42);

    let constructed_unknown = runtime.construct_class_as::<IUnknown>(CALCULATOR_CLASS_ID, ()).unwrap();
    assert!(constructed_unknown.query::<ICalculatorAbi>().is_ok());

    let local_unknown: ComPtr<IUnknown> = Calculator::create();
    let validated = runtime
        .query_validated::<ICalculatorAbi, _>(CALCULATOR_CLASS_ID, &local_unknown)
        .unwrap();
    assert_eq!(validated.add(2, 5).unwrap(), 7);

    let raw_unknown = Calculator::create().into_raw();
    let adopted = unsafe {
        // SAFETY: `into_raw` transfers exactly one owned IUnknown reference.
        runtime.adopt_unknown_as::<ICalculatorAbi>(CALCULATOR_CLASS_ID, raw_unknown)
    }
    .unwrap();
    assert_eq!(adopted.add(20, 22).unwrap(), 42);

    let unknown = runtime.construct_class(CALCULATOR_CLASS_ID, ()).unwrap();
    let err = runtime.query_validated::<IUnsupportedAbi, _>(CALCULATOR_CLASS_ID, &unknown).unwrap_err();
    assert_eq!(err, NC_E_NOINTERFACE);
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the linked runtime C ABI")]
fn runtime_enforces_constructor_argument_contract_before_custom_constructor() {
    UNCHECKED_CUSTOM_CONSTRUCTOR_CALLS.store(0, Ordering::SeqCst);
    let runtime = Runtime::global();

    assert_eq!(
        unsafe { runtime.register_plugin_descriptor(&UNCHECKED_CONSTRUCTOR_PLUGIN) },
        NC_OK
    );

    let frame = TrackedConstructorFrame { value: 99 };
    let frame_pointer = core::ptr::from_ref(&frame).cast();
    let err = unsafe { runtime.construct_class_raw(UNCHECKED_NO_ARGS_CLASS_ID, NcAbiHash::ZERO, frame_pointer) }.unwrap_err();
    assert_eq!(err, NC_E_INVALID_ARG);
    assert_eq!(UNCHECKED_CUSTOM_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 0);

    let err = unsafe { runtime.construct_class_raw(UNCHECKED_ARGS_CLASS_ID, TRACKED_CONSTRUCTOR_ABI_HASH, core::ptr::null()) }.unwrap_err();
    assert_eq!(err, NC_E_INVALID_ARG);
    assert_eq!(UNCHECKED_CUSTOM_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 0);

    let err = unsafe { runtime.construct_class_raw(UNCHECKED_ARGS_CLASS_ID, WRONG_TRACKED_CONSTRUCTOR_ABI_HASH, frame_pointer) }.unwrap_err();
    assert_eq!(err, NC_E_ABI_VERSION);
    assert_eq!(UNCHECKED_CUSTOM_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 0);

    let err = unsafe { runtime.construct_class_raw(UNCHECKED_WRONG_HASH_ARGS_CLASS_ID, TRACKED_CONSTRUCTOR_ABI_HASH, frame_pointer) }.unwrap_err();
    assert_eq!(err, NC_E_ABI_VERSION);
    assert_eq!(UNCHECKED_CUSTOM_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 0);

    let unknown = unsafe { runtime.construct_class_raw(UNCHECKED_ARGS_CLASS_ID, TRACKED_CONSTRUCTOR_ABI_HASH, frame_pointer) }.unwrap();
    assert_eq!(UNCHECKED_CUSTOM_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 1);

    let calculator = runtime
        .query_validated::<ICalculatorAbi, _>(UNCHECKED_ARGS_CLASS_ID, &unknown)
        .unwrap();
    assert_eq!(calculator.add(20, 22).unwrap(), 42);
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the linked runtime C ABI")]
fn runtime_propagates_custom_constructor_failure() {
    let runtime = Runtime::global();
    assert_eq!(
        unsafe { runtime.register_plugin_descriptor(&FAILING_CONSTRUCTOR_PLUGIN) },
        NC_OK
    );

    let error = runtime.construct_class(FAILING_CONSTRUCTOR_CLASS_ID, ()).unwrap_err();

    assert_eq!(error, NC_E_TEST_FAIL);
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the linked runtime C ABI")]
fn plugin_macro_descriptor_registers_and_constructs_class() {
    let runtime = Runtime::global();
    let descriptor = plugin_descriptor();

    assert_eq!(
        unsafe { descriptor.dependencies() }.unwrap(),
        &[NcPluginDependencyDescriptor {
            plugin_id: CALCULATOR_PLUGIN_ID,
        }]
    );
    assert_eq!(unsafe { runtime.register_plugin_descriptor(&CALCULATOR_PLUGIN) }, NC_OK);

    assert_eq!(
        MacroPluginEchoObject::class_descriptor().class_id,
        MacroPluginEchoAbi::CLASS_ID
    );
    assert_eq!(unsafe { runtime.register_plugin_descriptor(descriptor) }, NC_OK);
    assert_eq!(
        runtime.inspection().plugin_dependency_ids(descriptor.plugin_id).unwrap(),
        vec![CALCULATOR_PLUGIN_ID]
    );
    runtime
        .require_interface_type::<MacroPluginEchoAbi>(MacroPluginEchoAbi::CLASS_ID)
        .unwrap();
    runtime
        .require_interface_type::<ICalculatorAbi>(CallableCounterAbi::CLASS_ID)
        .unwrap();
    runtime
        .require_interface_type::<IMultiplierAbi>(CallableCounterAbi::CLASS_ID)
        .unwrap();

    let echo = runtime.construct::<MacroPluginEchoAbi>(()).unwrap();
    assert_eq!(echo.answer(), Ok(314));

    assert_eq!(
        CallableCounterObject::CLASS_ID,
        <CallableCounterAbi as ComClassContract>::CLASS_ID
    );
    assert_eq!(
        CallableCounterObject::CLASS_DESCRIPTOR.qualified_name,
        NcStaticUtf8::from_static(CallableCounterAbi::CLASS_QUALIFIED_NAME)
    );
    assert_eq!(
        CallableCounterObject::CLASS_DESCRIPTOR.class_version,
        NcStaticUtf8::from_static(env!("CARGO_PKG_VERSION"))
    );
    let label = *b"ok";
    let calculator = Calculator::create().query::<ICalculatorAbi>().unwrap();
    let arguments = CallableCounterAbi::arguments(40, NcCopyFromBufferOffset::new(0, &label), calculator.as_com_ref()).unwrap();
    let counter = runtime.construct::<CallableCounterAbi>(arguments).unwrap();
    assert_eq!(counter.answer(), Ok(42));
    let calculator_projection = counter.query::<ICalculatorAbi>().unwrap();
    let multiplier_projection = counter.query::<IMultiplierAbi>().unwrap();
    assert_eq!(calculator_projection.add(1, 2), Ok(45));
    assert_eq!(multiplier_projection.multiply(2, 3), Ok(48));
    assert_eq!(
        calculator_projection.query::<IUnknown>().unwrap().as_ptr(),
        multiplier_projection.query::<IUnknown>().unwrap().as_ptr()
    );
}

#[test]
fn typed_no_argument_constructor_constructs_non_unit_class() {
    let descriptor = MacroPluginDefaultObject::class_descriptor();
    let construct = descriptor.construct.unwrap();
    let mut out = core::ptr::null_mut();

    let status = unsafe { construct(descriptor, core::ptr::null_mut(), &mut out) };
    assert_eq!(status, NC_OK);
    assert!(!out.is_null());

    let unknown = unsafe { ComPtr::<IUnknown>::from_raw_owned(out) }.unwrap();
    let defaulted = unknown.query::<MacroPluginDefaultAbi>().unwrap();
    assert_eq!(defaulted.value(), Ok(0));
}

#[test]
fn generated_constructor_rejects_descriptor_threading_mismatch() {
    let mut descriptor = ThreadSafeCalculatorObject::CLASS_DESCRIPTOR;
    descriptor.threading = NC_ABI_THREADING_THREAD_LOCAL;
    let construct = descriptor.construct.unwrap();
    let mut out = core::ptr::dangling_mut();

    let status = unsafe { construct(&descriptor, core::ptr::null_mut(), &mut out) };

    assert_eq!(status, NC_E_INVALID_ARG);
    assert!(out.is_null());
}

#[test]
fn query_interface_preserves_com_identity() {
    let unknown = Calculator::create();
    let unknown_raw = unknown.as_ptr();

    let calculator = unknown.query::<ICalculatorAbi>().unwrap();
    let multiplier = calculator.query::<IMultiplierAbi>().unwrap();

    let calculator_unknown = calculator.query::<IUnknown>().unwrap();
    let multiplier_unknown = multiplier.query::<IUnknown>().unwrap();

    assert_eq!(calculator_unknown.as_ptr(), unknown_raw);
    assert_eq!(multiplier_unknown.as_ptr(), unknown_raw);
}

#[test]
fn query_interface_clears_out_on_lookup_failure() {
    let unknown = Calculator::create();
    let raw = unknown.as_mut_ptr();
    let mut sentinel = 0_u8;
    let mut out = (&mut sentinel as *mut u8).cast::<c_void>();
    let unsupported_id = IUnsupportedAbi::INTERFACE_ID;
    let status = unsafe { ((*(*raw).vtable).query_interface)(raw, &unsupported_id, &mut out) };
    assert_eq!(status, NC_E_NOINTERFACE);
    assert!(out.is_null());

    let mut out = (&mut sentinel as *mut u8).cast::<c_void>();
    let unsupported_calculator_id = NcInterfaceId::new(ICalculatorAbi::IID, NcAbiHash::ZERO);
    let status =
        unsafe { ((*(*raw).vtable).query_interface)(raw, &unsupported_calculator_id, &mut out) };
    assert_eq!(status, NC_E_ABI_VERSION);
    assert!(out.is_null());
}

#[test]
fn raw_interface_methods_reject_invalid_pointers() {
    let unknown = Calculator::create();
    let calculator = unknown.query::<ICalculatorAbi>().unwrap();
    let raw = calculator.as_mut_ptr();
    let mut out = 0;

    let status = unsafe { ((*(*raw).vtable).add)(core::ptr::null_mut(), 1, 2, &mut out) };
    assert_eq!(status, NC_E_POINTER);

    let status = unsafe { ((*(*raw).vtable).add)(raw, 1, 2, core::ptr::null_mut()) };
    assert_eq!(status, NC_E_POINTER);

    let mut header_with_null_object = ICalculatorAbi {
        vtable: unsafe { (*raw).vtable },
        object: core::ptr::null_mut(),
    };
    let status = unsafe { ((*(*raw).vtable).add)(&mut header_with_null_object, 1, 2, &mut out) };
    assert_eq!(status, NC_E_POINTER);

    let unknown_raw = unknown.as_mut_ptr();
    assert_eq!(
        unsafe { ((*(*unknown_raw).vtable).add_ref)(core::ptr::null_mut()) },
        0
    );
    assert_eq!(
        unsafe { ((*(*unknown_raw).vtable).release)(core::ptr::null_mut()) },
        0
    );
}

#[test]
fn failed_methods_do_not_overwrite_out_values() {
    let unknown = FailableObject::create();
    let failable = unknown.query::<IFailableAbi>().unwrap();
    let raw = failable.as_mut_ptr();
    let mut out = 123_u32;

    let status = unsafe { ((*(*raw).vtable).fail)(raw, &mut out) };
    assert_eq!(status, NC_E_TEST_FAIL);
    assert_eq!(out, 123);
    assert_eq!(failable.fail(), Err(NC_E_TEST_FAIL));
}

#[test]
fn void_success_methods_lower_to_status_only_abi() {
    VOID_FLUSHES.store(0, Ordering::SeqCst);

    let action = VoidAction::create().query::<IVoidActionAbi>().unwrap();

    assert_eq!(action.flush(), Ok(()));
    assert_eq!(VOID_FLUSHES.load(Ordering::SeqCst), 1);

    let raw = action.as_mut_ptr();
    let status = unsafe { ((*(*raw).vtable).flush)(raw) };
    assert_eq!(status, NC_OK);
    assert_eq!(VOID_FLUSHES.load(Ordering::SeqCst), 2);

    let status = unsafe { ((*(*raw).vtable).flush)(core::ptr::null_mut()) };
    assert_eq!(status, NC_E_POINTER);

    let mut header_with_null_object = IVoidActionAbi {
        vtable: unsafe { (*raw).vtable },
        object: core::ptr::null_mut(),
    };
    let status = unsafe { ((*(*raw).vtable).flush)(&mut header_with_null_object) };
    assert_eq!(status, NC_E_POINTER);

    let status = unsafe { ((*(*raw).vtable).fail_void)(raw) };
    assert_eq!(status, NC_E_TEST_VOID_FAIL);
    assert_eq!(action.fail_void(), Err(NC_E_TEST_VOID_FAIL));
}

#[test]
fn generated_reference_counting_drops_storage_once() {
    TRACKED_DROPS.store(0, Ordering::SeqCst);

    let unknown = TrackedObject::from_value(TrackedObject { value: 11 });
    let tracked = unknown.query::<ITrackedAbi>().unwrap();
    let tracked_clone = tracked.clone();
    let unknown_again = tracked.query::<IUnknown>().unwrap();

    assert_eq!(tracked.value().unwrap(), 11);
    assert_eq!(TRACKED_DROPS.load(Ordering::SeqCst), 0);

    drop(unknown);
    assert_eq!(TRACKED_DROPS.load(Ordering::SeqCst), 0);

    drop(tracked);
    assert_eq!(TRACKED_DROPS.load(Ordering::SeqCst), 0);

    drop(unknown_again);
    assert_eq!(TRACKED_DROPS.load(Ordering::SeqCst), 0);

    drop(tracked_clone);
    assert_eq!(TRACKED_DROPS.load(Ordering::SeqCst), 1);
}

#[test]
fn multiple_generated_objects_keep_independent_lifetimes() {
    let mut owners = Vec::new();
    let mut retained = Vec::new();

    for value in [7, 17, 11, 13, 19] {
        let (unknown, drops) = lifecycle_tracked(value);
        let tracked = unknown.query::<ILifecycleTrackedAbi>().unwrap();
        let queried_unknown = tracked.query::<IUnknown>().unwrap();
        owners.push(unknown);
        retained.push((tracked, queried_unknown, drops, value));
    }

    drop(owners);
    for (tracked, _, drops, value) in &retained {
        assert_eq!(tracked.value().unwrap(), *value);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    while let Some((tracked, queried_unknown, drops, value)) = retained.pop() {
        assert_eq!(tracked.value().unwrap(), value);
        drop(queried_unknown);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(tracked);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn raw_add_ref_release_stress_balances_reference_count() {
    let (unknown, drops) = lifecycle_tracked(101);
    let raw = unknown.as_mut_ptr();

    for expected in 2..=129 {
        assert_eq!(unsafe { nc_com::raw::add_ref(raw) }, expected);
    }
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    for expected in (1..=128).rev() {
        assert_eq!(unsafe { nc_com::raw::release(raw) }, expected);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    drop(unknown);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn query_interface_lifecycle_stress_keeps_storage_until_last_release() {
    let (unknown, drops) = lifecycle_tracked(202);
    let unknown_raw = unknown.as_ptr();
    let mut retained = Vec::new();

    for _ in 0..64 {
        let tracked = unknown.query::<ILifecycleTrackedAbi>().unwrap();
        assert_eq!(tracked.value().unwrap(), 202);

        let tracked_unknown = tracked.query::<IUnknown>().unwrap();
        assert_eq!(tracked_unknown.as_ptr(), unknown_raw);

        retained.push(tracked.clone());
        retained.push(tracked);
    }

    drop(unknown);
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    while retained.len() > 1 {
        drop(retained.pop().unwrap());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    drop(retained.pop().unwrap());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn thread_safe_refcounting_survives_concurrent_queries_and_clones() {
    let (unknown, drops) = thread_safe_lifecycle_tracked(505);
    let start = Arc::new(Barrier::new(9));
    let mut handles = Vec::new();
    let mut releases = Vec::new();

    for _ in 0..8 {
        let unknown = unknown.clone();
        let start = Arc::clone(&start);
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        releases.push(release_tx);
        handles.push(std::thread::spawn(move || {
            start.wait();
            for _ in 0..256 {
                let tracked = unknown.query::<ILifecycleTrackedAbi>().unwrap();
                assert_eq!(tracked.value().unwrap(), 505);

                let tracked_clone = tracked.clone();
                let tracked_unknown = tracked.query::<IUnknown>().unwrap();
                drop(tracked_unknown);
                drop(tracked_clone);
                drop(tracked);
            }
            // Keep the worker's owner alive until the main thread has checked
            // that releasing its own owner does not destroy the object.
            release_rx.recv().unwrap();
        }));
    }

    start.wait();
    drop(unknown);
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    for release in releases {
        release.send(()).unwrap();
    }
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn com_ref_add_ref_extends_lifetime_after_original_owner_drops() {
    let drops;
    let retained = {
        let (unknown, object_drops) = lifecycle_tracked(303);
        let tracked = unknown.query::<ILifecycleTrackedAbi>().unwrap();
        let retained = tracked.as_com_ref().add_ref();

        drop(tracked);
        drop(unknown);
        assert_eq!(object_drops.load(Ordering::SeqCst), 0);

        drops = object_drops;
        retained
    };

    assert_eq!(retained.value().unwrap(), 303);
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    drop(retained);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_query_interface_stress_does_not_leak_or_release_owner() {
    let (unknown, drops) = lifecycle_tracked(404);

    for _ in 0..128 {
        assert_eq!(
            unknown.query::<IUnsupportedAbi>().unwrap_err(),
            NC_E_NOINTERFACE
        );
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    drop(unknown);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn raw_comptr_ownership_apis_are_explicit() {
    let unknown: ComPtr<IUnknown> = Calculator::create();

    let const_ptr: *const IUnknown = unknown.as_ptr();
    let mut_ptr: *mut IUnknown = unknown.as_mut_ptr();
    assert_eq!(const_ptr, mut_ptr.cast_const());
    assert_eq!(unknown.as_non_null().as_ptr(), mut_ptr);

    let raw = unknown.into_raw();
    let cloned = unsafe { ComPtr::<IUnknown>::clone_from_raw(raw) }.unwrap();
    let remaining = unsafe { nc_com::raw::release(raw) };
    assert_eq!(remaining, 1);

    let calculator = cloned.query::<ICalculatorAbi>().unwrap();
    assert_eq!(calculator.add(8, 13).unwrap(), 21);
}

#[test]
fn from_raw_owned_wraps_transferred_references() {
    let unknown: ComPtr<IUnknown> = Calculator::create();
    let raw = unknown.into_raw();
    let wrapped = unsafe { ComPtr::<IUnknown>::from_raw_owned(raw) }.unwrap();

    assert!(wrapped.is::<ICalculatorAbi>());
}

#[test]
fn interface_queries_preserve_failure_status() {
    let unknown = Calculator::create();

    assert!(unknown.query::<ICalculatorAbi>().is_ok());
    assert!(matches!(
        unknown.query::<IUnsupportedAbi>(),
        Err(NC_E_NOINTERFACE)
    ));
}

#[test]
fn com_enum_preserves_known_and_unknown_values() {
    assert_eq!(TestNodeKind::ELEMENT.raw(), 1);
    assert!(TestNodeKind::TEXT.is_known());

    let future_kind = TestNodeKind::from_raw(99);
    assert_eq!(future_kind.raw(), 99);
    assert!(future_kind.is_unknown());
}

#[test]
fn abi_type_marker_accepts_supported_value_types() {
    fn assert_abi_type<T: nc_com::AbiType>() {}
    fn assert_abi_plain_value<T: nc_com::AbiPlainValue>() {}
    fn assert_com_struct<T: nc_com::ComStruct>() {}

    assert_abi_type::<u32>();
    assert_abi_type::<i64>();
    assert_abi_type::<f32>();
    assert_abi_type::<nc_com::NcGuid>();
    assert_abi_type::<NcStatus>();
    assert_abi_type::<TestNodeKind>();
    assert_abi_type::<TestMutationTag>();
    assert_abi_type::<TestMutationPayload>();
    assert_abi_type::<TestMutationAbi>();
    assert_abi_type::<TestRect>();
    assert_abi_type::<TestDelta>();
    assert_abi_type::<TestHeader>();
    assert_abi_type::<TestPackedHeader>();
    assert_abi_type::<TestPacked2Header>();
    assert_abi_type::<TestAlignedHeader>();

    assert_abi_plain_value::<u32>();
    assert_abi_plain_value::<NcStatus>();
    assert_abi_plain_value::<TestNodeKind>();
    assert_abi_plain_value::<TestMutationAbi>();
    assert_abi_plain_value::<TestRect>();
    assert_abi_plain_value::<TestDelta>();
    assert_abi_plain_value::<TestHeader>();
    assert_abi_plain_value::<TestPackedHeader>();
    assert_abi_plain_value::<TestPacked2Header>();
    assert_abi_plain_value::<TestAlignedHeader>();

    assert_com_struct::<TestRect>();
    assert_com_struct::<TestDelta>();
    assert_com_struct::<TestHeader>();
    assert_com_struct::<TestPackedHeader>();
    assert_com_struct::<TestPacked2Header>();
    assert_com_struct::<TestAlignedHeader>();
}

#[test]
fn tagged_com_enum_converts_unit_tuple_and_named_variants() {
    #[repr(C)]
    struct TestMutationAbiLayout {
        tag: TestMutationTag,
        payload: TestMutationPayload,
    }

    assert_eq!(
        core::mem::size_of::<TestMutationAbi>(),
        core::mem::size_of::<TestMutationAbiLayout>()
    );
    assert_eq!(
        core::mem::align_of::<TestMutationAbi>(),
        core::mem::align_of::<TestMutationAbiLayout>()
    );
    assert_eq!(TestMutationAbi::TAG_OFFSET, 0);
    assert_eq!(
        TestMutationAbi::PAYLOAD_OFFSET,
        core::mem::offset_of!(TestMutationAbiLayout, payload)
    );

    let none_abi = TestMutationAbi::from(TestMutation::None);
    assert_eq!(none_abi.tag().raw(), 10);
    assert_eq!(
        TestMutation::try_from(none_abi).unwrap(),
        TestMutation::None
    );

    let child_abi = TestMutationAbi::from(TestMutation::ChildAdded(7, 2));
    assert_eq!(child_abi.tag(), TestMutationTag::ChildAdded);
    assert_eq!(
        TestMutation::try_from(child_abi).unwrap(),
        TestMutation::ChildAdded(7, 2)
    );

    let text_abi = TestMutationAbi::from(TestMutation::TextChanged { node: 9, len: 12 });
    assert_eq!(text_abi.tag(), TestMutationTag::TextChanged);
    assert_eq!(
        TestMutation::try_from(text_abi).unwrap(),
        TestMutation::TextChanged { node: 9, len: 12 }
    );
}

#[test]
fn tagged_com_enum_raw_construction_is_an_explicit_unsafe_escape_hatch() {
    let payload = TestMutationPayload {
        __nc_childadded: core::mem::ManuallyDrop::new(TestMutationChildAddedPayload { _0: 17, _1: 4 }),
    };
    // SAFETY: `ChildAdded` selects the initialized `__nc_childadded` member, whose
    // integer fields contain valid, fully initialized values.
    let abi =
        unsafe { TestMutationAbi::from_raw_parts_unchecked(TestMutationTag::ChildAdded, payload) };

    assert_eq!(abi.tag(), TestMutationTag::ChildAdded);
    assert_eq!(
        TestMutation::try_from(abi),
        Ok(TestMutation::ChildAdded(17, 4))
    );
}

#[test]
fn tagged_com_enum_round_trips_valid_restricted_payload_values() {
    let flag = TestScalarPayloadAbi::from(TestScalarPayload::Flag(true));
    assert_eq!(flag.tag(), TestScalarPayloadTag::Flag);
    assert_eq!(
        TestScalarPayload::try_from(flag),
        Ok(TestScalarPayload::Flag(true))
    );

    let byte = TestScalarPayloadAbi::from(TestScalarPayload::Byte(2));
    assert_eq!(byte.tag(), TestScalarPayloadTag::Byte);
    assert_eq!(
        TestScalarPayload::try_from(byte),
        Ok(TestScalarPayload::Byte(2))
    );
}

#[test]
fn tagged_com_enum_rejects_unknown_tags() {
    let invalid = TestMutationAbi::from_unknown_tag(TestMutationTag::from_raw(99)).unwrap();

    assert_eq!(TestMutation::try_from(invalid), Err(NC_E_INVALID_TAG));
    assert!(matches!(
        TestMutationAbi::from_unknown_tag(TestMutationTag::ChildAdded),
        Err(status) if status == NC_E_INVALID_ARG
    ));
}

mod visibility {
    use nc_com::{ComPtr, IUnknown, NcStatus, com_interface, com_object};

    #[com_interface(auto_guid = "nc.tests.com_usage.visibility.CrateVisible:v1")]
    pub(crate) trait ICrateVisible {
        fn crate_value(&self) -> Result<i32, NcStatus>;
    }

    #[com_interface(auto_guid = "nc.tests.com_usage.visibility.Private:v1")]
    trait IPrivate {
        fn private_value(&self) -> Result<i32, NcStatus>;
    }

    #[com_interface(auto_guid = "nc.tests.com_usage.visibility.Public:v1")]
    pub trait IPublic {
        fn public_value(&self) -> Result<i32, NcStatus>;
    }

    #[com_object(interfaces(ICrateVisible, IPrivate, IPublic))]
    pub(crate) struct VisibilityObject;

    impl ICrateVisible for VisibilityObject {
        fn crate_value(&self) -> Result<i32, NcStatus> {
            Ok(1)
        }
    }

    impl IPrivate for VisibilityObject {
        fn private_value(&self) -> Result<i32, NcStatus> {
            Ok(2)
        }
    }

    impl IPublic for VisibilityObject {
        fn public_value(&self) -> Result<i32, NcStatus> {
            Ok(3)
        }
    }

    #[test]
    fn generated_items_follow_interface_visibility() {
        let unknown: ComPtr<IUnknown> = VisibilityObject::create();

        let crate_visible = unknown.query::<ICrateVisibleAbi>().unwrap();
        assert_eq!(crate_visible.crate_value().unwrap(), 1);

        let private = unknown.query::<IPrivateAbi>().unwrap();
        assert_eq!(private.private_value().unwrap(), 2);

        let public = unknown.query::<IPublicAbi>().unwrap();
        assert_eq!(public.public_value().unwrap(), 3);
    }
}
