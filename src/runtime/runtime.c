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

void print_str(const char *value) {
    printf("%s\n", value);
}

char* hl_format_binary(unsigned long long value) {
    // Allocate enough space for 64 bits + null terminator
    char* result = malloc(65);
    result[64] = '\0';

    // Handle zero case
    if (value == 0) {
        result[0] = '0';
        result[1] = '\0';
        return result;
    }

    int pos = 63;
    while (value > 0 && pos >= 0) {
        result[pos] = (value & 1) ? '1' : '0';
        value >>= 1;
        pos--;
    }

    // Move the result to the beginning
    int start = pos + 1;
    int len = 64 - start;
    for (int i = 0; i < len; i++) {
        result[i] = result[start + i];
    }
    result[len] = '\0';

    return result;
}

char* hl_format_center(const char* value, int width) {
    int len = strlen(value);
    if (len >= width) {
        // If value is already wider than or equal to the desired width, return as-is
        char* result = malloc(len + 1);
        strcpy(result, value);
        return result;
    }

    int padding = width - len;
    int left_padding = padding / 2;
    int right_padding = padding - left_padding;

    char* result = malloc(width + 1);
    memset(result, ' ', width);
    result[width] = '\0';

    // Copy the value into the center
    memcpy(result + left_padding, value, len);

    return result;
}