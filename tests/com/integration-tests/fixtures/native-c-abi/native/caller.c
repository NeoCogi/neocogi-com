#include <stdlib.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef int32_t NcStatus;

typedef struct NcGuid NcGuid;
typedef struct NcAbiDigest NcAbiDigest;
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

struct NcAbiDigest {
    /* Null is absence; otherwise one complete immutable SHA-256 digest. */
    const uint8_t (*digest)[32];
};
_Static_assert(sizeof(NcAbiDigest) == sizeof(void *), "signature view must be one pointer");

struct NcInterfaceId {
    NcGuid iid;
    NcAbiDigest abi_signature;
};

struct NcStaticUtf8 {
    const uint8_t *data;
    uint32_t len;
};

struct NcInterfaceDescriptor {
    NcGuid interface_id;
    NcStaticUtf8 qualified_name;
    NcAbiDigest abi_signature;
    NcInterfaceId base;
    uint32_t method_count;
    const NcMethodDescriptor *methods;
};

typedef struct NcUtf8View { const uint8_t *data; size_t len; } NcUtf8View;
typedef struct NcFactoryInfo { NcInterfaceId interface; uint32_t threading; } NcFactoryInfo;
typedef struct NcPluginInfo {
    NcGuid plugin_id;
    NcUtf8View qualified_name, version, library_path;
    NcFactoryInfo factory;
    const NcGuid *dependencies;
    size_t dependency_count;
} NcPluginInfo;

/* Host plugin operation table. The fixture leaves callbacks
 * opaque where it only compares their slots. */
typedef struct NcPluginApi {
    size_t size;
    int32_t (*register_plugin_descriptor)(const void *);
    void (*load_plugin_library_utf8)(void);
    void (*acquire_factory_plugin)(void);
    NcStatus (*plugin_info)(NcGuid, NcPluginInfo *);
    size_t (*plugin_count)(void);
    NcStatus (*plugin_infos)(NcPluginInfo *, size_t, size_t *);
    void (*inspect_plugin_library_utf8)(void);
} NcPluginApi;

typedef struct NcDebugApiPrefix {
    size_t size;
    void (*create_messenger)(void);
} NcDebugApiPrefix;

/* Host access groups immutable tables; it owns no C registry allocation. */
typedef struct NcPersistenceApi NcPersistenceApi;
typedef struct NcHostApi {
    size_t size;
    uint32_t abi_magic, abi_version;
    const NcPluginApi *runtime;
    const NcDebugApiPrefix *debug;
    const NcPersistenceApi *persistence;
} NcHostApi;
/* The output capability is defined below; both parameters are part of the native ABI. */
typedef NcStatus (*NcAcquirePluginFactory)(const NcHostApi *, IUnknown *);

typedef struct NcPluginDescriptor {
    uint32_t abi_magic, abi_version;
    NcGuid plugin_id;
    NcStaticUtf8 qualified_name, plugin_version;
    uint32_t dependency_count;
    const struct NcPluginDependencyDescriptor *dependencies;
    NcInterfaceId root_factory;
    uint32_t root_factory_threading;
    NcAcquirePluginFactory acquire;
} NcPluginDescriptor;

_Static_assert(offsetof(NcPluginDescriptor, abi_magic) == 0, "descriptor marker prefix");
_Static_assert(offsetof(NcPluginDescriptor, abi_version) == 4, "descriptor version");
_Static_assert(offsetof(NcHostApi, abi_magic) == sizeof(size_t), "host marker prefix");
_Static_assert(offsetof(NcHostApi, abi_version) == sizeof(size_t) + sizeof(uint32_t), "host version");
_Static_assert(sizeof(NcHostApi) == (sizeof(void *) == 8 ? 40 : 24), "host C layout");

/* A raw borrowed/owned capability; ownership follows the enclosing operation. */
struct IUnknown {
    void *object;
    const IUnknownVTable *vtable;
};

/* Common lifetime entry points receive a one-pointer token. C need not
 * interpret the atomic header to call the shared functions supplied by Rust. */
struct NcSharedLifetimeState;
typedef struct NcSharedWeakRaw { struct NcSharedLifetimeState *control; } NcSharedWeakRaw;
_Static_assert(sizeof(NcSharedWeakRaw) == sizeof(void *), "weak token must be one pointer");
_Static_assert(offsetof(NcSharedWeakRaw, control) == 0, "shared weak token starts at its header pointer");

struct IUnknownVTable {
    /* Exact selected contract; receivers are direct object addresses in the native ABI. */
    const NcInterfaceId *interface_id;
    NcStatus (*query_interface)(
        void *self,
        const NcInterfaceId *interface_id,
        IUnknown *out
    );
    uint32_t (*add_ref)(void *self);
    uint32_t (*release)(void *self);
    /* Optional provider-owned weak lifetime token; this fixture does not request it. */
    NcStatus (*downgrade)(void *self, NcSharedWeakRaw *out_weak);
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
    size_t runtime_plugin_register_offset;
    size_t runtime_plugin_count_offset;
    size_t runtime_plugin_info_offset;
    size_t runtime_size, runtime_plugin_infos_offset;
    size_t plugin_info_size, plugin_factory_offset, plugin_dependencies_offset, plugin_dependency_count_offset;
    size_t debug_first_function_offset;
    size_t descriptor_size, descriptor_plugin_id_offset, descriptor_acquire_offset;
    size_t host_size, host_abi_magic_offset, host_abi_version_offset;
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

_Static_assert(offsetof(NcPluginApi, size) == 0,
    "NcPluginApi must begin with size");
_Static_assert(offsetof(NcPluginApi, register_plugin_descriptor) == sizeof(size_t),
    "NcPluginApi first callback offset changed");
_Static_assert(offsetof(NcDebugApiPrefix, create_messenger) == sizeof(size_t),
    "NcDebugApi prefix changed");

#if UINTPTR_MAX == UINT64_MAX
_Static_assert(sizeof(NcInterfaceDescriptor) == 80,
    "64-bit NcInterfaceDescriptor ABI size changed");
_Static_assert(offsetof(NcInterfaceDescriptor, method_count) == 64,
    "64-bit NcInterfaceDescriptor.method_count ABI offset changed");
_Static_assert(offsetof(NcInterfaceDescriptor, methods) == 72,
    "64-bit NcInterfaceDescriptor.methods ABI offset changed");
#elif UINTPTR_MAX == UINT32_MAX
_Static_assert(sizeof(NcInterfaceDescriptor) == 56,
    "32-bit NcInterfaceDescriptor ABI size changed");
_Static_assert(offsetof(NcInterfaceDescriptor, method_count) == 48,
    "32-bit NcInterfaceDescriptor.method_count ABI offset changed");
_Static_assert(offsetof(NcInterfaceDescriptor, methods) == 52,
    "32-bit NcInterfaceDescriptor.methods ABI offset changed");
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
        layouts->runtime_plugin_register_offset == offsetof(NcPluginApi, register_plugin_descriptor) &&
        layouts->runtime_plugin_count_offset == offsetof(NcPluginApi, plugin_count) &&
        layouts->runtime_plugin_info_offset == offsetof(NcPluginApi, plugin_info) &&
        layouts->runtime_size == sizeof(NcPluginApi) &&
        layouts->runtime_plugin_infos_offset == offsetof(NcPluginApi, plugin_infos) &&
        layouts->plugin_info_size == sizeof(NcPluginInfo) &&
        layouts->plugin_factory_offset == offsetof(NcPluginInfo, factory) &&
        layouts->plugin_dependencies_offset == offsetof(NcPluginInfo, dependencies) &&
        layouts->plugin_dependency_count_offset == offsetof(NcPluginInfo, dependency_count) &&
        layouts->debug_first_function_offset == offsetof(NcDebugApiPrefix, create_messenger) &&
        layouts->descriptor_size == sizeof(NcPluginDescriptor) &&
        layouts->descriptor_plugin_id_offset == offsetof(NcPluginDescriptor, plugin_id) &&
        layouts->descriptor_acquire_offset == offsetof(NcPluginDescriptor, acquire) &&
        layouts->host_size == sizeof(NcHostApi) &&
        layouts->host_abi_magic_offset == offsetof(NcHostApi, abi_magic) &&
        layouts->host_abi_version_offset == offsetof(NcHostApi, abi_version) &&
        layouts->host_runtime_offset == offsetof(NcHostApi, runtime) &&
        layouts->host_debug_offset == offsetof(NcHostApi, debug) &&
        layouts->host_persistence_offset == offsetof(NcHostApi, persistence) &&
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
    void (*retain)(struct NcSharedLifetimeState *), void (*release)(struct NcSharedLifetimeState *),
    NcStatus (*upgrade)(struct NcSharedLifetimeState *, const NcInterfaceId *, IUnknown *)) {
    NcSharedWeakRaw weak = {0};
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

/* Both copied result projections use this out slot and decode only on success. */
NcStatus nc_c_copied_choice(void *object, uint32_t mode, NativeChoiceWire *out) {
    (void)object;
    if (mode == 2) return -14; /* NC_E_DROPPED, without touching output. */
    NativeChoiceWire value = {0};
    value.tag = mode == 1 ? INT64_MAX : 0;
    value.payload.flag.enabled = 1;
    *out = value;
    return 0;
}

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

/* Use C's independently declared layouts to call the generated acquisition adapter. */
int32_t nc_c_exercise_host_bootstrap(const NcHostApi *host, const NcPluginDescriptor *plugin,
                                    NcStatus expected, NcStatus incompatible) {
    if (!host || host->size != sizeof(*host) || host->abi_magic != NC_COM_ABI_MAGIC ||
        host->abi_version != NC_COM_ABI_VERSION || !host->runtime || !host->debug || !host->persistence) return 0;
    if (!plugin || plugin->abi_magic != NC_COM_ABI_MAGIC ||
        plugin->abi_version != NC_COM_ABI_VERSION || !plugin->acquire) return 0;

    IUnknown out = {0};
    NcStatus status = plugin->acquire(host, &out);
    if (status != expected || out.object != NULL || out.vtable != NULL || host->runtime->plugin_count() != 17)
        return 0;

    /* Only the old prefix is readable. Rejection must precede any child access. */
    const struct { size_t size; uint32_t family, generation; } old_host = {
        sizeof(NcHostApi), 4, 23
    };
    status = plugin->acquire((const NcHostApi *)&old_host, &out);
    if (status != incompatible || out.object != NULL || out.vtable != NULL) return 0;
    const struct { size_t size; uint32_t marker, version; } marked_v4_host = {
        sizeof(NcHostApi), UINT32_C(0x4e434f4d), 4
    };
    status = plugin->acquire((const NcHostApi *)&marked_v4_host, &out);
    if (status != incompatible || out.object || out.vtable) return 0;
    const struct { size_t size; uint32_t marker, version; } marked_v5_host = {
        sizeof(NcHostApi), UINT32_C(0x4e434f4d), 5
    };
    status = plugin->acquire((const NcHostApi *)&marked_v5_host, &out);
    return status == incompatible && out.object == NULL && out.vtable == NULL;
}

/* Independent policy-specific provider storage. These fields are initialized
 * before publication and are accessed atomically only by the supplied Rust
 * lifetime operations; this fixture makes no concurrent plain C accesses. */
typedef struct NcObjectTypeInfo {
    void (*destroy_value)(void *);
    void (*deallocate)(void *);
    const IUnknownVTable *const *interfaces;
    size_t interface_count;
} NcObjectTypeInfo;
typedef struct NcSharedLifetimeState {
    const NcObjectTypeInfo *type_info;
    uintptr_t strong_count;
    uintptr_t weak_count;
} NcSharedLifetimeState;
typedef struct NcBoxedLifetimeState {
    const NcObjectTypeInfo *type_info;
} NcBoxedLifetimeState;
_Static_assert(sizeof(NcObjectTypeInfo) == 4 * sizeof(void *), "object type information size");
_Static_assert(offsetof(NcObjectTypeInfo, destroy_value) == 0, "object cleanup callback offset");
_Static_assert(sizeof(NcSharedLifetimeState) == 3 * sizeof(void *), "shared lifetime size");
_Static_assert(sizeof(NcBoxedLifetimeState) == sizeof(void *), "boxed lifetime size");
_Static_assert(offsetof(NcSharedLifetimeState, type_info) == 0, "shared metadata prefix");
_Static_assert(offsetof(NcBoxedLifetimeState, type_info) == 0, "boxed metadata prefix");
_Static_assert(offsetof(NcSharedLifetimeState, strong_count) == sizeof(void *), "strong count offset");
_Static_assert(offsetof(NcSharedLifetimeState, weak_count) == 2 * sizeof(void *), "shared weak offset");

typedef struct NcLifecycleCounters { unsigned destroyed, freed; } NcLifecycleCounters;
typedef struct NcSharedCAllocation {
    NcSharedLifetimeState lifetime;
    NcLifecycleCounters *counters;
} NcSharedCAllocation;
typedef struct NcBoxedCAllocation {
    NcBoxedLifetimeState lifetime;
    NcLifecycleCounters *counters;
} NcBoxedCAllocation;

/* Destruction and allocation release remain distinct, provider-local callbacks. */
static void nc_c_shared_destroy(void *object) {
    ((NcSharedCAllocation *)object)->counters->destroyed += 1;
}
static void nc_c_shared_free(void *object) {
    NcSharedCAllocation *storage = (NcSharedCAllocation *)object;
    storage->counters->freed += 1;
    free(storage);
}
static void nc_c_boxed_destroy(void *object) {
    ((NcBoxedCAllocation *)object)->counters->destroyed += 1;
}
static void nc_c_boxed_free(void *object) {
    NcBoxedCAllocation *storage = (NcBoxedCAllocation *)object;
    storage->counters->freed += 1;
    free(storage);
}

/* Each callback family accepts exactly its declared state. No metadata kind,
 * opposite-policy token or universal weak operation participates in the protocol. */
int nc_c_exercise_split_lifetimes(
    const IUnknownVTable *shared_root, const IUnknownVTable *exclusive_root,
    NcStatus dropped, NcStatus invalid,
    void (*shared_retain)(NcSharedLifetimeState *),
    void (*shared_release)(NcSharedLifetimeState *),
    NcStatus (*upgrade)(NcSharedLifetimeState *, const NcInterfaceId *, IUnknown *)) {
    if (shared_root->release == exclusive_root->release ||
        shared_root->add_ref == exclusive_root->add_ref) return 0;
    const IUnknownVTable *shared_tables[1] = {shared_root};
    const IUnknownVTable *exclusive_tables[1] = {exclusive_root};
    const NcObjectTypeInfo shared_info = {nc_c_shared_destroy, nc_c_shared_free, shared_tables, 1};
    const NcObjectTypeInfo boxed_info = {nc_c_boxed_destroy, nc_c_boxed_free, exclusive_tables, 1};
    NcLifecycleCounters shared_counts = {0, 0}, boxed_counts = {0, 0};
    NcSharedCAllocation *shared = (NcSharedCAllocation *)malloc(sizeof(*shared));
    NcBoxedCAllocation *boxed = (NcBoxedCAllocation *)malloc(sizeof(*boxed));
    if (!shared || !boxed) { free(shared); free(boxed); return 0; }
    shared->lifetime.type_info = &shared_info;
    shared->lifetime.strong_count = 1;
    shared->lifetime.weak_count = 1;
    shared->counters = &shared_counts;
    boxed->lifetime.type_info = &boxed_info;
    boxed->counters = &boxed_counts;

    shared_retain(&shared->lifetime);
    if (shared_root->add_ref(shared) != 2 || shared->lifetime.strong_count != 2) return 0;
    if (shared_root->release(shared) != 1) return 0;
    IUnknown output = {0};
    if (upgrade(&shared->lifetime, shared_root->interface_id, &output) != 0 ||
        output.object != shared || output.vtable != shared_root) return 0;
    if (output.vtable->release(output.object) != 1) return 0;
    if (shared_root->release(shared) != 0 || shared_counts.destroyed != 1 || shared_counts.freed != 0) return 0;
    if (shared->lifetime.strong_count != 0 || shared->lifetime.weak_count != 1) return 0;
    if (upgrade(&shared->lifetime, shared_root->interface_id, &output) != dropped ||
        output.object || output.vtable) return 0;
    shared_release(&shared->lifetime);
    if (shared_counts.freed != 1) return 0;

    if (exclusive_root->add_ref(boxed) != 0) return 0;
    output.object = boxed;
    output.vtable = exclusive_root;
    if (exclusive_root->query_interface(boxed, exclusive_root->interface_id, &output) != invalid ||
        output.object || output.vtable) return 0;
    NcSharedWeakRaw rejected = {0};
    if (!exclusive_root->downgrade || exclusive_root->downgrade(boxed, &rejected) != invalid ||
        rejected.control) return 0;
    if (exclusive_root->release(boxed) != 0) return 0;
    return boxed_counts.freed == 1 && shared_counts.destroyed == 1 && boxed_counts.destroyed == 1;
}

/* Represent each return and loan slot independently of generated Rust types. */
typedef void (*NativeValueVisit)(void *context, INativeValueAbi loan);
typedef struct NativeCapability NativeCapability;
typedef struct NativeBoxedActionsTable NativeBoxedActionsTable;
typedef struct NativeBoxedActions {
    void *object;
    const NativeBoxedActionsTable *vtable;
} NativeBoxedActions;
typedef void (*NativeBoxedVisit)(void *context, NativeBoxedActions loan);
struct NativeBoxedActionsTable {
    IUnknownVTable unknown;
    NcStatus (*find)(void *, INativeValueAbi *);
    NcStatus (*invalidate)(void *);
    NcStatus (*child)(void *, NativeBoxedActions *);
    NcStatus (*child_mut)(void *, NativeBoxedActions *);
    NcStatus (*visit_child)(void *, void *, NativeBoxedVisit);
};
typedef struct NativeCapabilityTable {
    IUnknownVTable unknown;
    NcStatus (*find)(void *, uint32_t, INativeValueAbi *);
    NcStatus (*load)(void *, uint32_t, INativeValueAbi *);
    NcStatus (*observed)(void *, uint32_t, void *, NativeValueVisit);
    NcStatus (*read)(void *, INativeValueAbi, int32_t *);
    NcStatus (*invalidate)(void *);
    NcStatus (*invalidate_borrowed)(void *, NativeBoxedActions);
    NcStatus (*stable)(void *, uint32_t, INativeValueAbi *);
    NcStatus (*direct_loan)(void *, uint32_t, INativeValueAbi *);
} NativeCapabilityTable;
struct NativeCapability {
    void *object;
    const NativeCapabilityTable *vtable;
};
/* The Rust harness supplies a portable thread-local address probe. */
typedef const void *(*NativeThreadProbe)(void);
typedef struct NativeVisitState {
    NativeThreadProbe thread;
    const void *initiating_thread;
    unsigned calls;
    int32_t value;
} NativeVisitState;

static int record_native_visit(NativeVisitState *state) {
    if (state->thread() != state->initiating_thread) return 0;
    state->calls++;
    return 1;
}

/* Delivery never retains the loan. Both null words are observable as absence. */
static void visit_native_value(void *context, INativeValueAbi loan) {
    NativeVisitState *state = context;
    if (!record_native_visit(state)) return;
    state->value = -1;
    if (loan.object != NULL && loan.vtable != NULL) {
        if (loan.vtable->value(loan.object, &state->value) != 0) {
            state->value = -2;
        }
    }
}

/* Use the exclusive result only inside the visit. The pair owns nothing. */
static void visit_native_boxed_actions(void *context, NativeBoxedActions loan) {
    NativeVisitState *state = context;
    if (!record_native_visit(state)) return;
    if (loan.object && loan.vtable) {
        state->value = loan.vtable->invalidate(loan.object);
    } else {
        state->value = loan.object || loan.vtable ? -2 : -1;
    }
}

int32_t nc_c_exercise_callable_slots(NativeCapability receiver, NcStatus pointer, NcStatus dropped, NativeThreadProbe thread) {
    const NativeCapabilityTable *table = receiver.vtable;
    void *object = receiver.object;
    INativeValueAbi output = {0};
    INativeValueAbi absent = {0};
    INativeValueAbi half = {object, NULL};
    NativeVisitState visitor = {thread, thread(), 0, 0};
    int32_t value;
    if (table->load(object, 0, &output) != 0 || !output.object || !output.vtable) return 1;
    INativeValueAbi other = output;
    if (table->find(object, 1, &other) != 0 || other.object || other.vtable) return 2;
    other = output;
    if (table->load(object, 2, &other) != dropped ||
        other.object != output.object || other.vtable != output.vtable) return 3;
    if (table->read(object, absent, &value) != 0 || value != -1) return 4;
    if (table->read(object, half, &value) != pointer) return 5;
    if (table->read(object, output, &value) != 0 || value != 42) return 6;
    if (table->observed(object, 0, &visitor, visit_native_value) != 0 ||
        visitor.calls != 1 || visitor.value != 42) return 7;
    if (table->observed(object, 1, &visitor, visit_native_value) != 0 ||
        visitor.calls != 2 || visitor.value != -1) return 8;
    if (table->observed(object, 2, &visitor, visit_native_value) != dropped || visitor.calls != 2) return 9;
    if (table->observed(object, 0, NULL, NULL) != pointer) return 10;
    if (table->invalidate(object) != 0) return 11;
    if (output.vtable->value(output.object, &value) != 0 || value != 42) return 12;
    INativeValueAbi loan = {0};
    if (table->stable(object, 0, &loan) != 0 ||
        loan.object != output.object || loan.vtable != output.vtable) return 13;
    /* The source remains live after the slot returns. This pair owns nothing. */
    if (loan.vtable->value(loan.object, &value) != 0 || value != 42) return 14;
    other = loan;
    if (table->stable(object, 2, &other) != dropped ||
        other.object != loan.object || other.vtable != loan.vtable) return 15;
    if (table->stable(object, 1, &other) != 0 || other.object || other.vtable) return 16;
    output.vtable->unknown.release(output.object);
    return 0;
}

/* Exercise independent C dispatch through two receiver-bound out pairs and a
 * distinct callback slot. Each exclusive loan ends before the next root call. */
int32_t nc_c_exercise_boxed_loan_slots(NativeCapability receiver, NativeBoxedActions root,
                                     NcStatus pointer, NcStatus dropped, NativeThreadProbe thread) {
    const NativeBoxedActionsTable *table = root.vtable;
    NativeBoxedActions read = {0};
    INativeValueAbi value = {0};
    if (table->child(root.object, &read) != 0 || !read.object || !read.vtable) return 1;
    if (read.vtable->find(read.object, &value) != 0 || !value.object || !value.vtable) return 2;
    int32_t number;
    NcStatus status = value.vtable->value(value.object, &number);
    value.vtable->unknown.release(value.object);
    if (status != 0 || number != 42) return 3;

    NativeBoxedActions edit = {0};
    if (table->child_mut(root.object, &edit) != 0 || !edit.object || !edit.vtable) return 4;
    /* The shared child loan has ended. Transfer this reborrow for the call. */
    if (receiver.vtable->invalidate_borrowed(receiver.object, edit) != 0) return 5;
    if (edit.vtable->find(edit.object, &value) != 0 || value.object || value.vtable) return 6;

    NativeVisitState visitor = {thread, thread(), 0, 0};
    if (table->visit_child(root.object, &visitor, visit_native_boxed_actions) != 0 ||
        visitor.calls != 1 || visitor.value != 0) return 7;
    if (table->child_mut(root.object, NULL) != pointer) return 8;

    /* The child is a leaf. Its failed slot never writes an out pair. */
    if (table->child(root.object, &read) != 0) return 9;
    NativeBoxedActions unchanged = root;
    if (read.vtable->child(read.object, &unchanged) != dropped ||
        unchanged.object != root.object || unchanged.vtable != root.vtable) return 10;
    if (table->child_mut(root.object, &edit) != 0) return 11;
    if (edit.vtable->visit_child(edit.object, &visitor, visit_native_boxed_actions) != 0 ||
        visitor.calls != 2 || visitor.value != -1) return 12;
    return 0;
}

/* An independently written C provider can violate a required out-pair slot by
 * delivering the shape only admitted by an optional slot. */
NcStatus nc_c_absent_required_boxed_result(void *object, NativeBoxedActions *out) {
    (void)object;
    *out = (NativeBoxedActions){0};
    return 0;
}

/* Replace only the result slot in a real shared receiver's exact vtable. The
 * modes distinguish absent, malformed and failed calls from a C provider. */
NcStatus nc_c_adverse_optional_result(void *object, uint32_t mode, INativeValueAbi *out) {
    if (mode == 2) return -14; /* NC_E_DROPPED, without touching output. */
    INativeValueAbi result = {mode == 1 ? object : NULL, NULL};
    *out = result;
    return 0;
}

/* Fails missing or duplicate delivery and rejects a half-null delivered pair. */
NcStatus nc_c_adverse_scoped_result(void *object, uint32_t mode, void *context, NativeValueVisit visit) {
    INativeValueAbi absent = {0};
    INativeValueAbi half = {object, NULL};
    switch (mode) {
        case 1: visit(context, half); return 0;
        case 2: return -14;
        case 3: visit(context, absent); visit(context, absent); return 0;
        case 4: visit(context, absent); return -14;
        case 5: return 0;
        default: visit(context, absent); return 0;
    }
}
