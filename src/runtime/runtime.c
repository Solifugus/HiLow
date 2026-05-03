#include <stdio.h>
#include "runtime.h"

void print_i32(int32_t value) {
    printf("%d\n", value);
}

void print_i64(int64_t value) {
    printf("%ld\n", value);
}

void print_u32(uint32_t value) {
    printf("%u\n", value);
}

void print_u64(uint64_t value) {
    printf("%lu\n", value);
}

void print_f32(float value) {
    printf("%g\n", value);
}

void print_f64(double value) {
    printf("%g\n", value);
}

void print_bool(bool value) {
    printf("%s\n", value ? "true" : "false");
}