#include <stdint.h>

/* These C layouts are declared independently from the generated Rust wire. */
typedef struct { uint32_t elements[2]; } Array2;
typedef struct { uint32_t elements[3]; } Array3;
typedef struct { Array2 elements[2]; } Rows2;
typedef struct { Array2 elements[3]; } Rows3;
typedef struct { uint8_t tag; Rows3 values; uint32_t tail; } ArrayRecord;

typedef int32_t (*Rotate)(void *, Array3, Array3 *);
typedef int32_t (*Increment)(void *, ArrayRecord, ArrayRecord *);
typedef int32_t (*Transfer)(void *, uint64_t, uint64_t, const Rows2 *,
                            uint64_t, uint64_t, Rows2 *, uint64_t *);

/* The Rust owner remains live for each call. Each output has writable storage. */
int nc_c_exercise_fixed_arrays(void *object, Rotate rotate, Increment increment, Transfer transfer) {
    const Array3 direct = {{11, 22, 33}};
    Array3 result;
    if (rotate(object, direct, &result) != 0 ||
        result.elements[0] != 22 || result.elements[1] != 33 || result.elements[2] != 11) {
        return 1;
    }

    const ArrayRecord record = {.tag = 29, .values.elements = {{{1, 2}}, {{3, 4}}, {{5, 6}}}, .tail = 65537};
    ArrayRecord changed;
    if (increment(object, record, &changed) != 0 || changed.tag != 29 || changed.tail != 65538) {
        return 2;
    }
    for (uint32_t row = 0; row < 3; ++row) {
        for (uint32_t column = 0; column < 2; ++column) {
            if (changed.values.elements[row].elements[column] != record.values.elements[row].elements[column] + 1) {
                return 3;
            }
        }
    }

    const Rows2 input[2] = {
        {.elements = {{{10, 20}}, {{30, 40}}}},
        {.elements = {{{50, 60}}, {{70, 80}}}}
    };
    Rows2 output[2];
    uint64_t written = 0;
    if (transfer(object, 5, 2, input, 9, 2, output, &written) != 0 || written != 2) {
        return 4;
    }
    for (uint32_t index = 0; index < 2; ++index) {
        for (uint32_t row = 0; row < 2; ++row) {
            for (uint32_t column = 0; column < 2; ++column) {
                if (output[index].elements[row].elements[column] != input[1 - index].elements[row].elements[column]) {
                    return 5;
                }
            }
        }
    }
    return 0;
}
