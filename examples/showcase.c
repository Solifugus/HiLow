#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <ctype.h>

// Dynamic array structure
typedef struct {
    void* data;
    int32_t length;
    int32_t capacity;
    size_t element_size;
} DynamicArray;

DynamicArray* array_new(size_t element_size) {
    DynamicArray* arr = malloc(sizeof(DynamicArray));
    arr->capacity = 4;
    arr->length = 0;
    arr->element_size = element_size;
    arr->data = malloc(arr->capacity * element_size);
    return arr;
}

void array_push_i32(DynamicArray* arr, int32_t item) {
    if (arr->length >= arr->capacity) {
        arr->capacity *= 2;
        arr->data = realloc(arr->data, arr->capacity * arr->element_size);
    }
    ((int32_t*)arr->data)[arr->length++] = item;
}

int32_t array_pop_i32(DynamicArray* arr) {
    if (arr->length == 0) return 0;
    return ((int32_t*)arr->data)[--arr->length];
}

void array_push_string(DynamicArray* arr, char* item) {
    if (arr->length >= arr->capacity) {
        arr->capacity *= 2;
        arr->data = realloc(arr->data, arr->capacity * arr->element_size);
    }
    ((char**)arr->data)[arr->length++] = item;
}

DynamicArray* str_split(const char* str, const char* delim) {
    DynamicArray* result = array_new(sizeof(char*));
    char* str_copy = strdup(str);
    char* token = strtok(str_copy, delim);
    while (token != NULL) {
        array_push_string(result, strdup(token));
        token = strtok(NULL, delim);
    }
    free(str_copy);
    return result;
}

char* array_join_string(DynamicArray* arr, const char* sep) {
    if (arr->length == 0) return strdup("");
    int total_len = 0;
    for (int i = 0; i < arr->length; i++) {
        total_len += strlen(((char**)arr->data)[i]);
    }
    total_len += strlen(sep) * (arr->length - 1);
    char* result = malloc(total_len + 1);
    result[0] = '\0';
    for (int i = 0; i < arr->length; i++) {
        if (i > 0) strcat(result, sep);
        strcat(result, ((char**)arr->data)[i]);
    }
    return result;
}

// String helper functions
char* str_to_upper(const char* str) {
    int len = strlen(str);
    char* result = malloc(len + 1);
    for (int i = 0; i < len; i++) { result[i] = toupper(str[i]); }
    result[len] = '\0';
    return result;
}

char* str_to_lower(const char* str) {
    int len = strlen(str);
    char* result = malloc(len + 1);
    for (int i = 0; i < len; i++) { result[i] = tolower(str[i]); }
    result[len] = '\0';
    return result;
}

char* str_trim(const char* str) {
    while (*str && isspace(*str)) str++;
    if (*str == '\0') return strdup("");
    const char* end = str + strlen(str) - 1;
    while (end > str && isspace(*end)) end--;
    int len = end - str + 1;
    char* result = malloc(len + 1);
    strncpy(result, str, len);
    result[len] = '\0';
    return result;
}

char* str_char_at(const char* str, int32_t index) {
    if (index < 0 || index >= strlen(str)) return strdup("");
    char* result = malloc(2);
    result[0] = str[index];
    result[1] = '\0';
    return result;
}

char* str_substring(const char* str, int32_t start, int32_t end) {
    int len = strlen(str);
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start >= end) return strdup("");
    int sublen = end - start;
    char* result = malloc(sublen + 1);
    strncpy(result, str + start, sublen);
    result[sublen] = '\0';
    return result;
}

char* str_concat(const char* s1, const char* s2) {
    int len = strlen(s1) + strlen(s2);
    char* result = malloc(len + 1);
    strcpy(result, s1);
    strcat(result, s2);
    return result;
}

char* str_replace(const char* str, const char* from, const char* to) {
    char* pos = strstr(str, from);
    if (!pos) return strdup(str);
    int from_len = strlen(from);
    int to_len = strlen(to);
    int prefix_len = pos - str;
    int suffix_len = strlen(pos + from_len);
    char* result = malloc(prefix_len + to_len + suffix_len + 1);
    strncpy(result, str, prefix_len);
    strcpy(result + prefix_len, to);
    strcpy(result + prefix_len + to_len, pos + from_len);
    return result;
}

int32_t demo_arrays();
int32_t demo_strings();
int32_t demo_control_flow();

int32_t demo_arrays() {
    printf("%s\n", "=== ARRAYS ===");
    int32_t fixed[5] = {10, 20, 30, 40, 50};
    printf("Fixed array: %d, %d, %d\n", fixed[0], fixed[1], fixed[2]);
    DynamicArray* dynamic = array_new(sizeof(int32_t));
    array_push_i32(dynamic, 100);
    array_push_i32(dynamic, 200);
    array_push_i32(dynamic, 300);
    printf("Dynamic array length: %d\n", dynamic->length);
    int32_t sum = 0;
    for (int32_t __idx_num = 0; __idx_num < sizeof(fixed)/sizeof((fixed)[0]); __idx_num++) {
        int32_t num = (
        fixed)[__idx_num];
        sum = (sum + num);
    }
    printf("Sum of fixed array: %d\n", sum);
    return sum;
}

int32_t demo_strings() {
    printf("%s\n", ");\n    print(\"=== STRINGS ===\");\n\n    let text: string = \"  Hello HiLow World  \";\n    print(f\"Original: \'{text}\'\");\n\n    let trimmed: string = text.trim();\n    print(f\"Trimmed: \'{trimmed}\'\");\n\n    let upper: string = trimmed.toUpperCase();\n    print(f\"Uppercase: \'{upper}\'\");\n\n    let lower: string = upper.toLowerCase();\n    print(f\"Lowercase: \'{lower}\'\");\n\n    let replaced: string = lower.replace(\"hilow\", \"AWESOME\");\n    print(f\"Replaced: \'{replaced}\'\");\n\n    let words: [string] = trimmed.split(\" \");\n    print(f\"Split into {words.length} words\");\n\n    let joined: string = words.join(\"-\");\n    print(f\"Joined: \'{joined}\'\");\n\n    return trimmed.length;\n}\n\nfunction demo_objects(): i32 {\n    print(");
    printf("%s\n", "=== OBJECTS ===");
    struct { int32_t x; int32_t y; } point = {.x = 10, .y = 20};
    printf("Point: (%d, %d)\n", point.x, point.y);
    point.x = (point.x + 15);
    point.y = (point.y * 2);
    printf("Modified: (%d, %d)\n", point.x, point.y);
    return (point.x + point.y);
}

int32_t demo_control_flow() {
    printf("%s\n", ");\n    print(\"=== CONTROL FLOW ===\");\n\n    let result: i32 = 0;\n\n    let i: i32 = 0;\n    while (i < 5) {\n        if (i % 2 ?= 0) {\n            i += 1;\n            continue;\n        }\n        result += i;\n        i += 1;\n    }\n    print(f\"While loop result: {result}\");\n\n    let choice: i32 = 2;\n    switch (choice) {\n        case 1:\n            result += 10;\n            break;\n        case 2:\n            result += 20;\n            break;\n        default:\n            result += 5;\n            break;\n    }\n    print(f\"After switch: {result}\");\n\n    return result;\n}\n\nfunction main(): i32 {\n    print(\"========================================\");\n    print(\"   HiLow Language Feature Showcase\");\n    print(\"========================================\");\n\n    let array_sum: i32 = demo_arrays();\n    let string_len: i32 = demo_strings();\n    let object_sum: i32 = demo_objects();\n    let control_result: i32 = demo_control_flow();\n\n    print(");
    printf("%s\n", "=== SUMMARY ===");
    printf("Array operations: %d\n", array_sum);
    printf("String operations: %d\n", string_len);
    printf("Object operations: %d\n", object_sum);
    printf("Control flow: %d\n", control_result);
    int32_t total = (((array_sum + string_len) + object_sum) + control_result);
    printf("Total: %d\n", total);
    printf("%s\n", "All features demonstrated successfully!");
    return total;
}

