#ifndef HILOW_RUNTIME_H
#define HILOW_RUNTIME_H

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>  // for malloc
#include <string.h>  // for strcat
#include <stdio.h>   // for sprintf

// Print functions for primitive types
// Each function prints the value followed by a newline

void print_i32(int32_t value);
void print_i64(int64_t value);
void print_u32(uint32_t value);
void print_u64(uint64_t value);
void print_f32(float value);
void print_f64(double value);
void print_bool(bool value);
void print_str(const char *value);

// F-string format helpers
char* hl_format_binary(unsigned long long value);
char* hl_format_center(const char* value, int width);

#endif // HILOW_RUNTIME_H