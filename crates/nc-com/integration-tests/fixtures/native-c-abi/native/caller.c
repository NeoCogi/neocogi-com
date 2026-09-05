#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef int32_t NcStatus;

typedef struct NcGuid NcGuid;
typedef struct NcAbiHash NcAbiHash;
typedef struct NcInterfaceId NcInterfaceId;
typedef struct NcStaticUtf8 NcStaticUtf8;
typedef struct NcMethodDescriptor NcMethodDescriptor;
typedef struct NcInterfaceDescriptor NcInterfaceDescriptor;
typedef struct NcClassInterfaceDescriptor NcClassInterfaceDescriptor;
typedef struct NcClassDescriptor NcClassDescriptor;
typedef struct NcClassTable NcClassTable;
typedef struct NcPluginDependencyDescriptor NcPluginDependencyDescriptor;
typedef struct NcPluginDescriptor NcPluginDescriptor;
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

struct NcAbiHash {
    uint8_t bytes[32];
};

struct NcInterfaceId {
    NcGuid iid;
    NcAbiHash abi_hash;
};

struct NcStaticUtf8 {
    const uint8_t *data;
    uint32_t len;
};

struct NcInterfaceDescriptor {
    NcGuid interface_id;
    NcStaticUtf8 qualified_name;
    NcAbiHash abi_hash;
    uint32_t method_count;
    const NcMethodDescriptor *methods;
};

typedef NcStatus (*NcConstructorFn)(
    const NcClassDescriptor *class_descriptor,
    IUnknown *arguments,
    IUnknown **out
);

struct NcClassInterfaceDescriptor {
    const NcInterfaceDescriptor *interface_descriptor;
};

struct NcClassDescriptor {
    NcGuid class_id;
    NcStaticUtf8 qualified_name;
    NcStaticUtf8 class_version;
    NcInterfaceId constructor_args;
    uint32_t threading;
    NcConstructorFn construct;
    uint32_t interface_count;
    const NcClassInterfaceDescriptor *interfaces;
};

struct NcClassTable {
    uint32_t class_count;
    const NcClassDescriptor *classes;
};

struct NcPluginDependencyDescriptor {
    NcGuid plugin_id;
};

struct NcPluginDescriptor {
    uint32_t core_abi_version;
    NcGuid plugin_id;
    NcStaticUtf8 qualified_name;
    NcStaticUtf8 plugin_version;
    uint32_t dependency_count;
    const NcPluginDependencyDescriptor *dependencies;
    const NcClassTable *class_table;
};

typedef struct NcRuntimeApiV1Prefix {
    size_t size;
    uint32_t core_abi_version;
    NcStatus (*register_plugin_descriptor)(const NcPluginDescriptor *plugin);
} NcRuntimeApiV1Prefix;

typedef struct NcPluginRegistryV1Prefix {
    size_t size;
    size_t (*plugin_count)(void);
} NcPluginRegistryV1Prefix;

typedef struct NcTypeRegistryV1Prefix {
    size_t size;
    size_t (*class_count)(void);
} NcTypeRegistryV1Prefix;

typedef struct NcInspectionApiV1Prefix {
    size_t size;
    void (*plugin_info)(void);
} NcInspectionApiV1Prefix;

typedef struct NcDebugApiV1Prefix {
    size_t size;
    void (*create_messenger)(void);
} NcDebugApiV1Prefix;

struct IUnknown {
    const IUnknownVTable *vtable;
    void *object;
};

struct IUnknownVTable {
    NcStatus (*query_interface)(
        IUnknown *self,
        const NcInterfaceId *interface_id,
        void **out
    );
    uint32_t (*add_ref)(IUnknown *self);
    uint32_t (*release)(IUnknown *self);
};

struct INativeValueAbi {
    const INativeValueVTable *vtable;
    void *object;
};

struct INativeValueVTable {
    IUnknownVTable unknown;
    NcStatus (*value)(INativeValueAbi *self, int32_t *out);
};

struct INativeCalculatorAbi {
    const INativeCalculatorVTable *vtable;
    void *object;
};

typedef struct NativeAggregate {
    uint8_t tag;
    uint32_t count;
    uint64_t value;
} NativeAggregate;

struct INativeCalculatorVTable {
    IUnknownVTable unknown;
    NcStatus (*add)(INativeCalculatorAbi *self, int32_t a, int32_t b, int32_t *out);
    NcStatus (*reset)(INativeCalculatorAbi *self);
    NcStatus (*make_value)(INativeCalculatorAbi *self, int32_t value, INativeValueAbi **out);
    NcStatus (*read_value)(INativeCalculatorAbi *self, INativeValueAbi *value, int32_t *out);
    NcStatus (*sum_buffer)(
        INativeCalculatorAbi *self,
        uint64_t offset,
        uint64_t len,
        const uint8_t *input,
        uint64_t *out
    );
    NcStatus (*fill_buffer)(
        INativeCalculatorAbi *self,
        uint64_t offset,
        uint64_t len,
        uint8_t *output
    );
    NcStatus (*transform)(
        INativeCalculatorAbi *self,
        NativeAggregate input,
        NativeAggregate *out
    );
};

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
    size_t class_descriptor_size;
    size_t class_threading_offset;
    size_t class_construct_offset;
    size_t class_interfaces_offset;
    size_t plugin_descriptor_size;
    size_t plugin_qualified_name_offset;
    size_t plugin_dependencies_offset;
    size_t plugin_class_table_offset;
    size_t runtime_core_abi_version_offset;
    size_t runtime_register_offset;
    size_t plugin_registry_first_function_offset;
    size_t type_registry_first_function_offset;
    size_t inspection_first_function_offset;
    size_t debug_first_function_offset;
    size_t aggregate_size;
    size_t aggregate_align;
    size_t aggregate_count_offset;
    size_t aggregate_value_offset;
} NativeAbiLayouts;

_Static_assert(offsetof(NcInterfaceDescriptor, qualified_name) == 16,
    "NcInterfaceDescriptor.qualified_name ABI offset changed");
_Static_assert(offsetof(NativeAggregate, count) == 4,
    "NativeAggregate.count ABI offset changed");
_Static_assert(offsetof(NativeAggregate, value) == 8,
    "NativeAggregate.value ABI offset changed");
_Static_assert(sizeof(NativeAggregate) == 16,
    "NativeAggregate ABI size changed");

_Static_assert(offsetof(NcRuntimeApiV1Prefix, size) == 0,
    "NcRuntimeApiV1 must begin with size");
_Static_assert(offsetof(NcRuntimeApiV1Prefix, core_abi_version) == sizeof(size_t),
    "NcRuntimeApiV1.core_abi_version prefix changed");
_Static_assert(offsetof(NcPluginRegistryV1Prefix, plugin_count) == sizeof(size_t),
    "NcPluginRegistryV1 prefix changed");
_Static_assert(offsetof(NcTypeRegistryV1Prefix, class_count) == sizeof(size_t),
    "NcTypeRegistryV1 prefix changed");
_Static_assert(offsetof(NcInspectionApiV1Prefix, plugin_info) == sizeof(size_t),
    "NcInspectionApiV1 prefix changed");
_Static_assert(offsetof(NcDebugApiV1Prefix, create_messenger) == sizeof(size_t),
    "NcDebugApiV1 prefix changed");

#if UINTPTR_MAX == UINT64_MAX
_Static_assert(sizeof(NcInterfaceDescriptor) == 80,
    "64-bit NcInterfaceDescriptor ABI size changed");
_Static_assert(offsetof(NcInterfaceDescriptor, method_count) == 64,
    "64-bit NcInterfaceDescriptor.method_count ABI offset changed");
_Static_assert(offsetof(NcInterfaceDescriptor, methods) == 72,
    "64-bit NcInterfaceDescriptor.methods ABI offset changed");
_Static_assert(sizeof(NcClassDescriptor) == 128,
    "64-bit NcClassDescriptor ABI size changed");
_Static_assert(offsetof(NcClassDescriptor, threading) == 96,
    "64-bit NcClassDescriptor.threading ABI offset changed");
_Static_assert(offsetof(NcClassDescriptor, construct) == 104,
    "64-bit NcClassDescriptor.construct ABI offset changed");
_Static_assert(offsetof(NcClassDescriptor, interfaces) == 120,
    "64-bit NcClassDescriptor.interfaces ABI offset changed");
_Static_assert(sizeof(NcPluginDescriptor) == 80,
    "64-bit NcPluginDescriptor ABI size changed");
_Static_assert(offsetof(NcPluginDescriptor, qualified_name) == 24,
    "64-bit NcPluginDescriptor.qualified_name ABI offset changed");
_Static_assert(offsetof(NcPluginDescriptor, dependencies) == 64,
    "64-bit NcPluginDescriptor.dependencies ABI offset changed");
_Static_assert(offsetof(NcPluginDescriptor, class_table) == 72,
    "64-bit NcPluginDescriptor.class_table ABI offset changed");
_Static_assert(offsetof(NcRuntimeApiV1Prefix, register_plugin_descriptor) == 16,
    "64-bit NcRuntimeApiV1 registration prefix changed");
#elif UINTPTR_MAX == UINT32_MAX
_Static_assert(sizeof(NcInterfaceDescriptor) == 64,
    "32-bit NcInterfaceDescriptor ABI size changed");
_Static_assert(offsetof(NcInterfaceDescriptor, method_count) == 56,
    "32-bit NcInterfaceDescriptor.method_count ABI offset changed");
_Static_assert(offsetof(NcInterfaceDescriptor, methods) == 60,
    "32-bit NcInterfaceDescriptor.methods ABI offset changed");
_Static_assert(sizeof(NcClassDescriptor) == 96,
    "32-bit NcClassDescriptor ABI size changed");
_Static_assert(offsetof(NcClassDescriptor, threading) == 80,
    "32-bit NcClassDescriptor.threading ABI offset changed");
_Static_assert(offsetof(NcClassDescriptor, construct) == 84,
    "32-bit NcClassDescriptor.construct ABI offset changed");
_Static_assert(offsetof(NcClassDescriptor, interfaces) == 92,
    "32-bit NcClassDescriptor.interfaces ABI offset changed");
_Static_assert(sizeof(NcPluginDescriptor) == 48,
    "32-bit NcPluginDescriptor ABI size changed");
_Static_assert(offsetof(NcPluginDescriptor, qualified_name) == 20,
    "32-bit NcPluginDescriptor.qualified_name ABI offset changed");
_Static_assert(offsetof(NcPluginDescriptor, dependencies) == 40,
    "32-bit NcPluginDescriptor.dependencies ABI offset changed");
_Static_assert(offsetof(NcPluginDescriptor, class_table) == 44,
    "32-bit NcPluginDescriptor.class_table ABI offset changed");
_Static_assert(offsetof(NcRuntimeApiV1Prefix, register_plugin_descriptor) == 8,
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
        layouts->class_descriptor_size == sizeof(NcClassDescriptor) &&
        layouts->class_threading_offset == offsetof(NcClassDescriptor, threading) &&
        layouts->class_construct_offset == offsetof(NcClassDescriptor, construct) &&
        layouts->class_interfaces_offset == offsetof(NcClassDescriptor, interfaces) &&
        layouts->plugin_descriptor_size == sizeof(NcPluginDescriptor) &&
        layouts->plugin_qualified_name_offset == offsetof(NcPluginDescriptor, qualified_name) &&
        layouts->plugin_dependencies_offset == offsetof(NcPluginDescriptor, dependencies) &&
        layouts->plugin_class_table_offset == offsetof(NcPluginDescriptor, class_table) &&
        layouts->runtime_core_abi_version_offset == offsetof(NcRuntimeApiV1Prefix, core_abi_version) &&
        layouts->runtime_register_offset == offsetof(NcRuntimeApiV1Prefix, register_plugin_descriptor) &&
        layouts->plugin_registry_first_function_offset == offsetof(NcPluginRegistryV1Prefix, plugin_count) &&
        layouts->type_registry_first_function_offset == offsetof(NcTypeRegistryV1Prefix, class_count) &&
        layouts->inspection_first_function_offset == offsetof(NcInspectionApiV1Prefix, plugin_info) &&
        layouts->debug_first_function_offset == offsetof(NcDebugApiV1Prefix, create_messenger) &&
        layouts->aggregate_size == sizeof(NativeAggregate) &&
        layouts->aggregate_align == _Alignof(NativeAggregate) &&
        layouts->aggregate_count_offset == offsetof(NativeAggregate, count) &&
        layouts->aggregate_value_offset == offsetof(NativeAggregate, value);
}

static int failed(NcStatus status) {
    return status < 0;
}

NcStatus nc_c_exercise_calculator(
    IUnknown *unknown,
    const NcInterfaceId *calculator_interface_id,
    NativeCResults *results,
    NcStatus pointer_status
) {
    if (unknown == 0 || calculator_interface_id == 0 || results == 0) {
        return pointer_status;
    }

    memset(results, 0, sizeof(*results));

    void *queried = 0;
    NcStatus status =
        unknown->vtable->query_interface(
            unknown,
            calculator_interface_id,
            &queried
        );
    if (failed(status)) {
        return status;
    }
    if (queried == 0) {
        return pointer_status;
    }

    INativeCalculatorAbi *calculator = (INativeCalculatorAbi *)queried;
    IUnknown *calculator_unknown = (IUnknown *)calculator;

    results->add_ref_count =
        calculator->vtable->unknown.add_ref(calculator_unknown);
    results->release_extra_count =
        calculator->vtable->unknown.release(calculator_unknown);

    status = calculator->vtable->add(calculator, 2, 3, &results->sum);
    if (failed(status)) {
        goto release_calculator;
    }

    results->null_method_status = calculator->vtable->add(calculator, 1, 1, 0);
    if (results->null_method_status != pointer_status) {
        status = results->null_method_status;
        goto release_calculator;
    }

    results->reset_status = calculator->vtable->reset(calculator);
    if (failed(results->reset_status)) {
        status = results->reset_status;
        goto release_calculator;
    }

    results->null_reset_status = calculator->vtable->reset(0);
    if (results->null_reset_status != pointer_status) {
        status = results->null_reset_status;
        goto release_calculator;
    }

    INativeValueAbi *value = 0;
    status = calculator->vtable->make_value(calculator, 42, &value);
    if (failed(status)) {
        goto release_calculator;
    }
    if (value == 0) {
        status = pointer_status;
        goto release_calculator;
    }

    status = value->vtable->value(value, &results->made_value);
    if (failed(status)) {
        goto release_value;
    }

    status = calculator->vtable->read_value(calculator, value, &results->borrowed_value);
    if (failed(status)) {
        goto release_value;
    }

    const uint8_t input[] = {1, 2, 3, 4};
    status = calculator->vtable->sum_buffer(
        calculator,
        15,
        sizeof(input),
        input,
        &results->buffer_sum
    );
    if (failed(status)) {
        goto release_value;
    }

    status = calculator->vtable->fill_buffer(
        calculator,
        7,
        sizeof(results->filled_buffer),
        results->filled_buffer
    );
    if (failed(status)) {
        goto release_value;
    }

    results->null_copy_from_status = calculator->vtable->sum_buffer(
        calculator,
        0,
        1,
        0,
        &results->buffer_sum
    );
    if (results->null_copy_from_status != pointer_status) {
        status = results->null_copy_from_status;
        goto release_value;
    }

    results->null_copy_to_status = calculator->vtable->fill_buffer(
        calculator,
        0,
        1,
        0
    );
    if (results->null_copy_to_status != pointer_status) {
        status = results->null_copy_to_status;
        goto release_value;
    }

    uint64_t zero_sum = UINT64_MAX;
    results->zero_copy_from_status = calculator->vtable->sum_buffer(
        calculator,
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

    results->zero_copy_to_status = calculator->vtable->fill_buffer(
        calculator,
        0,
        0,
        0
    );
    if (failed(results->zero_copy_to_status)) {
        status = results->zero_copy_to_status;
        goto release_value;
    }

    const NativeAggregate aggregate = {3, 5, 11};
    status = calculator->vtable->transform(
        calculator,
        aggregate,
        &results->aggregate
    );

release_value:
    results->release_value_count =
        value->vtable->unknown.release((IUnknown *)value);

release_calculator:
    results->release_query_count =
        calculator->vtable->unknown.release(calculator_unknown);
    return status;
}
