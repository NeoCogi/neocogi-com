#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef int32_t NcStatus;

typedef struct NcGuid NcGuid;
typedef struct NcAbiSignature NcAbiSignature;
typedef struct NcInterfaceId NcInterfaceId;
typedef struct NcStaticUtf8 NcStaticUtf8;
typedef struct NcMethodDescriptor NcMethodDescriptor;
typedef struct NcInterfaceDescriptor NcInterfaceDescriptor;
typedef struct IUnknown IUnknown;
typedef struct IUnknownVTable IUnknownVTable;
typedef struct INativeValueAbi INativeValueAbi;
typedef struct INativeValueVTable INativeValueVTable;
typedef struct INativeCalculatorAbi INativeCalculatorAbi;
typedef struct INativeCalculatorVTable INativeCalculatorVTable;

struct NcGuid {
    uint32_t data1;
    uint16_t data2;
    uint16_t data3;
    uint8_t data4[8];
};

struct NcAbiSignature {
    /* Null is absence; otherwise one complete immutable SHA-256 digest. */
    const uint8_t (*digest)[32];
};
_Static_assert(sizeof(NcAbiSignature) == sizeof(void *), "signature view must be one pointer");

struct NcInterfaceId {
    NcGuid iid;
    NcAbiSignature abi_signature;
};

struct NcStaticUtf8 {
    const uint8_t *data;
    uint32_t len;
};

struct NcInterfaceDescriptor {
    NcGuid interface_id;
    NcStaticUtf8 qualified_name;
    NcAbiSignature abi_signature;
    NcInterfaceId base;
    uint32_t method_count;
    const NcMethodDescriptor *methods;
};

typedef struct NcUtf8View { const uint8_t *data; size_t len; } NcUtf8View;
typedef struct NcFactoryInfoV1 { NcInterfaceId interface; uint32_t threading; } NcFactoryInfoV1;
typedef struct NcPluginInfoV1 {
    NcGuid plugin_id;
    NcUtf8View qualified_name, version, library_path;
    NcFactoryInfoV1 factory;
    const NcGuid *dependencies;
    size_t dependency_count;
} NcPluginInfoV1;

/* Current runtime-family-1 layout; the first three callback signatures are
 * retained as opaque pointers here because this test invokes metadata only. */
typedef struct NcRuntimeApiV1 {
    size_t size;
    uint32_t core_abi_version;
    int32_t (*register_plugin_descriptor)(const void *);
    void (*load_plugin_library_utf8)(void);
    void (*acquire_factory_plugin)(void);
    NcStatus (*plugin_info)(NcGuid, NcPluginInfoV1 *);
    size_t (*plugin_count)(void);
    NcStatus (*plugin_infos)(NcPluginInfoV1 *, size_t, size_t *);
    void (*inspect_plugin_library_utf8)(void);
} NcRuntimeApiV1;

typedef struct NcDebugApiV1Prefix {
    size_t size;
    void (*create_messenger)(void);
} NcDebugApiV1Prefix;

/* Host access groups immutable tables; it owns no C registry allocation. */
typedef struct NcPersistenceApiV1 NcPersistenceApiV1;
typedef struct NcHostApiV1 {
    size_t size;
    uint32_t runtime_abi_version, core_abi_version;
    const NcRuntimeApiV1 *runtime;
    const NcDebugApiV1Prefix *debug;
    const NcPersistenceApiV1 *persistence;
} NcHostApiV1;
/* The output capability is defined below; both parameters are part of ABI-v1. */
typedef NcStatus (*NcAcquirePluginFactory)(const NcHostApiV1 *, IUnknown *);
_Static_assert(offsetof(NcHostApiV1, runtime_abi_version) == sizeof(size_t), "host version prefix");
_Static_assert(sizeof(NcHostApiV1) == (sizeof(void *) == 8 ? 40 : 24), "host C layout");

/* A raw borrowed/owned capability; ownership follows the enclosing operation. */
struct IUnknown {
    void *object;
    const IUnknownVTable *vtable;
};

/* Common lifetime entry points receive a one-pointer token. C need not
 * interpret the atomic header to call the shared functions supplied by Rust. */
typedef struct NcWeakControl NcWeakControl;
typedef struct NcWeakRaw { NcWeakControl *control; } NcWeakRaw;
_Static_assert(sizeof(NcWeakRaw) == sizeof(void *), "weak token must be one pointer");

struct IUnknownVTable {
    /* Exact selected contract; receivers are direct object addresses in the current core ABI. */
    const NcInterfaceId *interface_id;
    NcStatus (*query_interface)(
        void *self,
        const NcInterfaceId *interface_id,
        IUnknown *out
    );
    uint32_t (*add_ref)(void *self);
    uint32_t (*release)(void *self);
    /* Optional provider-owned weak lifetime token; this fixture does not request it. */
    NcStatus (*downgrade)(void *self, NcWeakRaw *out_weak);
};

/* A raw borrowed/owned capability; ownership follows the enclosing operation. */
struct INativeValueAbi {
    void *object;
    const INativeValueVTable *vtable;
};

struct INativeValueVTable {
    IUnknownVTable unknown;
    NcStatus (*value)(void *self, int32_t *out);
};

/* A raw borrowed/owned capability; ownership follows the enclosing operation. */
struct INativeCalculatorAbi {
    void *object;
    const INativeCalculatorVTable *vtable;
};

typedef struct NativeAggregate {
    uint8_t tag;
    uint32_t count;
    uint64_t value;
} NativeAggregate;

struct INativeCalculatorVTable {
    /* The complete INativeValue contract is the physical prefix. */
    INativeValueVTable base;
    NcStatus (*add)(void *self, int32_t a, int32_t b, int32_t *out);
    NcStatus (*reset)(void *self);
    NcStatus (*make_value)(void *self, int32_t value, INativeValueAbi *out);
    NcStatus (*read_value)(void *self, INativeValueAbi value, int32_t *out);
    NcStatus (*sum_buffer)(
        void *self,
        uint64_t offset,
        uint64_t len,
        const uint8_t *input,
        uint64_t *out
    );
    NcStatus (*fill_buffer)(
        void *self,
        uint64_t offset,
        uint64_t len,
        uint8_t *output,
        uint64_t *written
    );
    NcStatus (*transform)(
        void *self,
        NativeAggregate input,
        NativeAggregate *out
    );
};

/* Borrow the caller's owner and prove QueryInterface compares all digest bytes,
 * independently of their address. No stack identity/digest may escape this call. */
int32_t nc_c_exercise_signature(
    IUnknown object,
    const NcInterfaceId *expected,
    NcStatus mismatch
) {
    /* Distinct initialized C storage exercises the nullable pointer-to-array ABI. */
    uint8_t digest[32];
    memcpy(digest, *expected->abi_signature.digest, sizeof(digest));
    NcInterfaceId requested = *expected;
    requested.abi_signature.digest = (const uint8_t (*)[32])&digest;
    IUnknown acquired = {0};
    if (object.vtable->query_interface(object.object, &requested, &acquired) != 0)
        return 0;
    if (!acquired.object || !acquired.vtable)
        return 0;
    /* Successful negotiation owns one reference even with caller-local bytes. */
    if (acquired.vtable->release(acquired.object) != 1)
        return 0;

    for (size_t index = 0; index < sizeof(digest); ++index) {
        digest[index] ^= 1;
        acquired = object; /* Borrowed sentinel: rejection must clear both words. */
        NcStatus status = object.vtable->query_interface(object.object, &requested, &acquired);
        if (status != mismatch || acquired.object || acquired.vtable)
            return 0;
        digest[index] ^= 1;
    }

    /* Null is an absent digest, not permission to bypass exact negotiation. */
    requested.abi_signature.digest = NULL;
    acquired = object;
    NcStatus status = object.vtable->query_interface(object.object, &requested, &acquired);
    return status == mismatch && !acquired.object && !acquired.vtable;
}

typedef struct NativeCResults {
    int32_t sum;
    int32_t made_value;
    int32_t borrowed_value;
    int32_t reset_status;
    int32_t null_reset_status;
    int32_t null_method_status;
    uint32_t add_ref_count;
    uint32_t release_extra_count;
    uint32_t release_query_count;
    uint32_t release_value_count;
    uint64_t buffer_sum;
    uint8_t filled_buffer[4];
    int32_t null_copy_from_status;
    int32_t null_copy_to_status;
    int32_t zero_copy_from_status;
    int32_t zero_copy_to_status;
    NativeAggregate aggregate;
} NativeCResults;

typedef struct NativeAbiLayouts {
    size_t interface_descriptor_size;
    size_t interface_qualified_name_offset;
    size_t interface_method_count_offset;
    size_t interface_methods_offset;
    size_t runtime_core_abi_version_offset;
    size_t runtime_plugin_register_offset;
    size_t runtime_plugin_count_offset;
    size_t runtime_plugin_info_offset;
    size_t runtime_size, runtime_plugin_infos_offset;
    size_t plugin_info_size, plugin_factory_offset, plugin_dependencies_offset, plugin_dependency_count_offset;
    size_t debug_first_function_offset;
    size_t host_size, host_runtime_version_offset, host_core_version_offset;
    size_t host_runtime_offset, host_debug_offset, host_persistence_offset;
    size_t aggregate_size;
    size_t aggregate_align;
    size_t aggregate_count_offset;
    size_t aggregate_value_offset;
} NativeAbiLayouts;

/* The ABI pair has no hidden allocation, header pointer, or ownership counter. */
_Static_assert(sizeof(IUnknown) == 2 * sizeof(void *), "raw capability must contain two pointer words");
_Static_assert(offsetof(IUnknown, object) == 0, "object is the first raw capability word");
_Static_assert(offsetof(IUnknown, vtable) == sizeof(void *), "vtable is the second raw capability word");

_Static_assert(offsetof(NcInterfaceDescriptor, qualified_name) == 16,
    "NcInterfaceDescriptor.qualified_name ABI offset changed");
_Static_assert(offsetof(NativeAggregate, count) == 4,
    "NativeAggregate.count ABI offset changed");
_Static_assert(offsetof(NativeAggregate, value) == 8,
    "NativeAggregate.value ABI offset changed");
_Static_assert(sizeof(NativeAggregate) == 16,
    "NativeAggregate ABI size changed");

_Static_assert(offsetof(NcRuntimeApiV1, size) == 0,
    "NcRuntimeApiV1 must begin with size");
_Static_assert(offsetof(NcRuntimeApiV1, core_abi_version) == sizeof(size_t),
    "NcRuntimeApiV1.core_abi_version prefix changed");
_Static_assert(offsetof(NcDebugApiV1Prefix, create_messenger) == sizeof(size_t),
    "NcDebugApiV1 prefix changed");

#if UINTPTR_MAX == UINT64_MAX
_Static_assert(sizeof(NcInterfaceDescriptor) == 80,
    "64-bit NcInterfaceDescriptor ABI size changed");
_Static_assert(offsetof(NcInterfaceDescriptor, method_count) == 64,
    "64-bit NcInterfaceDescriptor.method_count ABI offset changed");
_Static_assert(offsetof(NcInterfaceDescriptor, methods) == 72,
    "64-bit NcInterfaceDescriptor.methods ABI offset changed");
_Static_assert(offsetof(NcRuntimeApiV1, register_plugin_descriptor) == 16,
    "64-bit NcRuntimeApiV1 registration prefix changed");
#elif UINTPTR_MAX == UINT32_MAX
_Static_assert(sizeof(NcInterfaceDescriptor) == 56,
    "32-bit NcInterfaceDescriptor ABI size changed");
_Static_assert(offsetof(NcInterfaceDescriptor, method_count) == 48,
    "32-bit NcInterfaceDescriptor.method_count ABI offset changed");
_Static_assert(offsetof(NcInterfaceDescriptor, methods) == 52,
    "32-bit NcInterfaceDescriptor.methods ABI offset changed");
_Static_assert(offsetof(NcRuntimeApiV1, register_plugin_descriptor) == 8,
    "32-bit NcRuntimeApiV1 registration prefix changed");
#else
#error "native C ABI fixture supports only 32-bit and 64-bit targets"
#endif

int32_t nc_c_validate_abi_layouts(const NativeAbiLayouts *layouts) {
    if (layouts == 0) {
        return 0;
    }

    return
        layouts->interface_descriptor_size == sizeof(NcInterfaceDescriptor) &&
        layouts->interface_qualified_name_offset == offsetof(NcInterfaceDescriptor, qualified_name) &&
        layouts->interface_method_count_offset == offsetof(NcInterfaceDescriptor, method_count) &&
        layouts->interface_methods_offset == offsetof(NcInterfaceDescriptor, methods) &&
        layouts->runtime_core_abi_version_offset == offsetof(NcRuntimeApiV1, core_abi_version) &&
        layouts->runtime_plugin_register_offset == offsetof(NcRuntimeApiV1, register_plugin_descriptor) &&
        layouts->runtime_plugin_count_offset == offsetof(NcRuntimeApiV1, plugin_count) &&
        layouts->runtime_plugin_info_offset == offsetof(NcRuntimeApiV1, plugin_info) &&
        layouts->runtime_size == sizeof(NcRuntimeApiV1) &&
        layouts->runtime_plugin_infos_offset == offsetof(NcRuntimeApiV1, plugin_infos) &&
        layouts->plugin_info_size == sizeof(NcPluginInfoV1) &&
        layouts->plugin_factory_offset == offsetof(NcPluginInfoV1, factory) &&
        layouts->plugin_dependencies_offset == offsetof(NcPluginInfoV1, dependencies) &&
        layouts->plugin_dependency_count_offset == offsetof(NcPluginInfoV1, dependency_count) &&
        layouts->debug_first_function_offset == offsetof(NcDebugApiV1Prefix, create_messenger) &&
        layouts->host_size == sizeof(NcHostApiV1) &&
        layouts->host_runtime_version_offset == offsetof(NcHostApiV1, runtime_abi_version) &&
        layouts->host_core_version_offset == offsetof(NcHostApiV1, core_abi_version) &&
        layouts->host_runtime_offset == offsetof(NcHostApiV1, runtime) &&
        layouts->host_debug_offset == offsetof(NcHostApiV1, debug) &&
        layouts->host_persistence_offset == offsetof(NcHostApiV1, persistence) &&
        layouts->aggregate_size == sizeof(NativeAggregate) &&
        layouts->aggregate_align == _Alignof(NativeAggregate) &&
        layouts->aggregate_count_offset == offsetof(NativeAggregate, count) &&
        layouts->aggregate_value_offset == offsetof(NativeAggregate, value);
}

static int failed(NcStatus status) {
    return status < 0;
}

NcStatus nc_c_exercise_calculator(
    IUnknown unknown,
    const NcInterfaceId *calculator_interface_id,
    NativeCResults *results,
    NcStatus pointer_status
) {
    if ((unknown.object == 0 || unknown.vtable == 0) || calculator_interface_id == 0 || results == 0) {
        return pointer_status;
    }

    memset(results, 0, sizeof(*results));

    IUnknown queried = {0};
    NcStatus status =
        unknown.vtable->query_interface(
            unknown.object,
            calculator_interface_id,
            &queried
        );
    if (failed(status)) {
        return status;
    }
    if (queried.object == 0 || queried.vtable == 0 || queried.object != unknown.object) {
        return pointer_status;
    }

    INativeCalculatorAbi calculator = {queried.object, (const INativeCalculatorVTable *)queried.vtable};
    void *calculator_unknown = calculator.object;
    int32_t inherited = 0;
    if (calculator.vtable->base.value(calculator.object, &inherited) != 0 || inherited != 23) {
        status = -99;
        goto release_calculator;
    }

    results->add_ref_count =
        calculator.vtable->base.unknown.add_ref(calculator_unknown);
    results->release_extra_count =
        calculator.vtable->base.unknown.release(calculator_unknown);

    status = calculator.vtable->add(calculator.object, 2, 3, &results->sum);
    if (failed(status)) {
        goto release_calculator;
    }

    results->null_method_status = calculator.vtable->add(calculator.object, 1, 1, 0);
    if (results->null_method_status != pointer_status) {
        status = results->null_method_status;
        goto release_calculator;
    }

    results->reset_status = calculator.vtable->reset(calculator.object);
    if (failed(results->reset_status)) {
        status = results->reset_status;
        goto release_calculator;
    }

    results->null_reset_status = calculator.vtable->reset(0);
    if (results->null_reset_status != pointer_status) {
        status = results->null_reset_status;
        goto release_calculator;
    }

    INativeValueAbi value = {0};
    status = calculator.vtable->make_value(calculator.object, 42, &value);
    if (failed(status)) {
        goto release_calculator;
    }
    if (value.object == 0 || value.vtable == 0) {
        status = pointer_status;
        goto release_calculator;
    }

    status = value.vtable->value(value.object, &results->made_value);
    if (failed(status)) {
        goto release_value;
    }

    status = calculator.vtable->read_value(calculator.object, value, &results->borrowed_value);
    if (failed(status)) {
        goto release_value;
    }

    /* Both half-null shapes must fail before reading any typed method slot. */
    INativeValueAbi malformed = {value.object, 0};
    if (calculator.vtable->read_value(calculator.object, malformed, &results->borrowed_value) != pointer_status) {
        status = -98;
        goto release_value;
    }
    malformed.object = 0;
    malformed.vtable = value.vtable;
    if (calculator.vtable->read_value(calculator.object, malformed, &results->borrowed_value) != pointer_status) {
        status = -98;
        goto release_value;
    }

    const uint8_t input[] = {1, 2, 3, 4};
    status = calculator.vtable->sum_buffer(
        calculator.object,
        15,
        sizeof(input),
        input,
        &results->buffer_sum
    );
    if (failed(status)) {
        goto release_value;
    }

    uint64_t written = UINT64_MAX;
    status = calculator.vtable->fill_buffer(
        calculator.object,
        7,
        sizeof(results->filled_buffer),
        results->filled_buffer,
        &written
    );
    if (failed(status) || written != sizeof(results->filled_buffer)) {
        goto release_value;
    }

    results->null_copy_from_status = calculator.vtable->sum_buffer(
        calculator.object,
        0,
        1,
        0,
        &results->buffer_sum
    );
    if (results->null_copy_from_status != pointer_status) {
        status = results->null_copy_from_status;
        goto release_value;
    }

    results->null_copy_to_status = calculator.vtable->fill_buffer(
        calculator.object,
        0,
        1,
        0,
        &written
    );
    if (results->null_copy_to_status != pointer_status) {
        status = results->null_copy_to_status;
        goto release_value;
    }

    uint64_t zero_sum = UINT64_MAX;
    results->zero_copy_from_status = calculator.vtable->sum_buffer(
        calculator.object,
        0,
        0,
        0,
        &zero_sum
    );
    if (failed(results->zero_copy_from_status) || zero_sum != 0) {
        status = failed(results->zero_copy_from_status)
            ? results->zero_copy_from_status
            : pointer_status;
        goto release_value;
    }

    results->zero_copy_to_status = calculator.vtable->fill_buffer(
        calculator.object,
        0,
        0,
        0,
        &written
    );
    if (failed(results->zero_copy_to_status) || written != 0) {
        status = results->zero_copy_to_status;
        goto release_value;
    }

    const NativeAggregate aggregate = {3, 5, 11};
    status = calculator.vtable->transform(
        calculator.object,
        aggregate,
        &results->aggregate
    );

release_value:
    results->release_value_count =
        value.vtable->unknown.release(value.object);

release_calculator:
    results->release_query_count =
        calculator.vtable->base.unknown.release(calculator_unknown);
    return status;
}

/* Consumes one sole strong owner and checks the full weak protocol from plain C. */
int nc_c_exercise_weak(IUnknown owned, const NcInterfaceId *requested, NcStatus dropped_status,
    void (*retain)(NcWeakControl *), void (*release)(NcWeakControl *),
    NcStatus (*upgrade)(NcWeakControl *, const NcInterfaceId *, IUnknown *)) {
    NcWeakRaw weak = {0};
    if (!owned.vtable->downgrade || owned.vtable->downgrade(owned.object, &weak) != 0) return 0;
    if ((void *)weak.control != owned.object) return 0;
    retain(weak.control);
    IUnknown promoted = {0};
    if (upgrade(weak.control, requested, &promoted) != 0) return 0;
    if (promoted.object != owned.object) return 0;
    promoted.vtable->release(promoted.object);
    owned.vtable->release(owned.object);
    /* Failure must clear even a nonempty output; owned's address is no longer dereferenced. */
    promoted = owned;
    NcStatus status = upgrade(weak.control, requested, &promoted);
    int result = status == dropped_status && promoted.object == NULL && promoted.vtable == NULL;
    release(weak.control);
    release(weak.control);
    return result;
}

/* Independent wire declaration: native Rust enum size/layout is never observed. */
typedef struct {
    int64_t tag; /* Zero-based declaration ordinals occupy the common raw tag. */
    union {
        struct { uint8_t enabled; } flag; /* Valid bool bytes are exactly 0 and 1. */
        struct { uint8_t empty; } empty; /* Ignored for the unit alternative. */
    } payload;
} NativeChoiceWire;

/* Real generated Rust thunk, using the same C calling convention as plugin slots. */
typedef int32_t (*NativeChoiceEcho)(void *, NativeChoiceWire, NativeChoiceWire *);

/* Malformed data stays in unrestricted C storage until the thunk rejects it. */
int32_t nc_c_exercise_choice(void *object, NativeChoiceEcho echo, int32_t invalid_tag, int32_t invalid_bool) {
    NativeChoiceWire input = {0}, output = {0};
    input.tag = 1;
    input.payload.empty.empty = 255; /* Inactive bytes must not be validated/read. */
    if (echo(object, input, &output) != 0 || output.tag != 1) return 0;
    input.tag = 0;
    for (uint8_t enabled = 0; enabled < 2; ++enabled) {
        input.payload.flag.enabled = enabled;
        if (echo(object, input, &output) != 0 || output.tag != 0 ||
            output.payload.flag.enabled != enabled) return 0;
    }
    input.payload.flag.enabled = 2;
    if (echo(object, input, &output) != invalid_bool) return 0;
    input.tag = 999;
    if (echo(object, input, &output) != invalid_tag) return 0;
    return 1;
}

/* Generic output uses four arguments, plus the ordinary required-total return. */
typedef NcStatus (*NativeChoiceOutput)(void *, uint64_t, uint64_t,
    NativeChoiceWire *, uint64_t *, uint32_t *);

/* These initialized C bytes are intentionally invalid as native Rust bool/enums.
 * The new thunk must only write them, then publish the explicit initialized prefix. */
int32_t nc_c_exercise_choice_output(void *object, NativeChoiceOutput output,
    NcStatus pointer_status) {
    NativeChoiceWire slots[3];
    memset(slots, 0x55, sizeof(slots));
    NativeChoiceWire tail = slots[2];
    uint64_t written = UINT64_MAX;
    uint32_t required = UINT32_MAX;
    if (output(object, 0, 3, slots, &written, &required) != 0 ||
        required != 2 || written != 2 || slots[0].tag != 0 ||
        slots[0].payload.flag.enabled != 1 || slots[1].tag != 1 ||
        memcmp(&slots[2], &tail, sizeof(tail)) != 0) return 0;
    /* Zero capacity is a valid query; required total is not its written count. */
    if (output(object, 0, 0, NULL, &written, &required) != 0 ||
        written != 0 || required != 2) return 0;
    if (output(object, 0, 1, NULL, &written, &required) != pointer_status)
        return 0;
    if (output(object, 0, 0, NULL, NULL, &required) != pointer_status)
        return 0;
    return 1;
}

/* This checks a real typed bootstrap call, not merely function-pointer size. */
int32_t nc_c_exercise_host_bootstrap(const NcHostApiV1 *host, NcAcquirePluginFactory acquire, NcStatus expected) {
    if (!host || host->size != sizeof(*host) || host->runtime_abi_version != 1 ||
        host->core_abi_version != NC_COM_CORE_ABI_VERSION || !host->runtime || !host->debug || !host->persistence) return 0;
    IUnknown out = {0};
    NcStatus status = acquire(host, &out);
    return status == expected && out.object == NULL && out.vtable == NULL && host->runtime->plugin_count() == 17;
}
