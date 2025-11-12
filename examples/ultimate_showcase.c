#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <ctype.h>
#include <math.h>

// Unknown type structure
typedef struct {
    char* reason;
    char** options;
    int32_t option_count;
} Unknown;

Unknown* create_unknown(const char* reason) {
    Unknown* u = malloc(sizeof(Unknown));
    u->reason = strdup(reason);
    u->options = NULL;
    u->option_count = 0;
    return u;
}

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

void array_reverse_i32(DynamicArray* arr) {
    int32_t* data = (int32_t*)arr->data;
    for (int i = 0; i < arr->length / 2; i++) {
        int32_t temp = data[i];
        data[i] = data[arr->length - 1 - i];
        data[arr->length - 1 - i] = temp;
    }
}

DynamicArray* array_map_i32(DynamicArray* arr, int32_t(*func)(int32_t, int32_t)) {
    DynamicArray* result = array_new(sizeof(int32_t));
    for (int i = 0; i < arr->length; i++) {
        int32_t val = ((int32_t*)arr->data)[i];
        int32_t mapped = func(val, 0);
        array_push_i32(result, mapped);
    }
    return result;
}

DynamicArray* array_filter_i32(DynamicArray* arr, int32_t(*func)(int32_t, int32_t)) {
    DynamicArray* result = array_new(sizeof(int32_t));
    for (int i = 0; i < arr->length; i++) {
        int32_t val = ((int32_t*)arr->data)[i];
        if (func(val, 0)) {
            array_push_i32(result, val);
        }
    }
    return result;
}

int32_t array_reduce_i32(DynamicArray* arr, int32_t(*func)(int32_t, int32_t), int32_t initial) {
    int32_t result = initial;
    for (int i = 0; i < arr->length; i++) {
        int32_t val = ((int32_t*)arr->data)[i];
        result = func(result, val);
    }
    return result;
}

void array_forEach_i32(DynamicArray* arr, int32_t(*func)(int32_t, int32_t)) {
    for (int i = 0; i < arr->length; i++) {
        int32_t val = ((int32_t*)arr->data)[i];
        func(val, 0);
    }
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

int32_t __lambda_0(int32_t x, int32_t dummy) {
    return (x * x);
}

int32_t sum(int32_t a, int32_t b);
int32_t is_positive(int32_t x, int32_t dummy);
int32_t main();

int32_t sum(int32_t a, int32_t b) {
    return (a + b);
}

int32_t is_positive(int32_t x, int32_t dummy) {
    if ((x > 0)) {
        return 1;
    }
    return 0;
}

int32_t main() {
    printf("%s\n", "========================================");
    printf("%s\n", "   HiLow Ultimate Feature Showcase");
    printf("%s\n", "========================================");
    printf("%s\n", ");\n    print(\"1. F-STRINGS & QUOTE RECURSION\");\n    let name: string = \"HiLow\";\n    let version: i32 = 1;\n    print(f\"Language: {name}, Version: {version}\");\n    print(\"Quote recursion: nested quotes work\");\n\n    print(");
    printf("%s\n", "2. STRING METHODS");
    char* text = "  hello world  ";
    char* clean = str_to_upper(str_trim(text));
    printf("Transformed: '%s'\n", clean);
    DynamicArray* words = str_split(clean, " ");
    char* back = array_join_string(words, "-");
    printf("Split/Join: '%s'\n", back);
    printf("%s\n", ");\n    print(\"3. DYNAMIC ARRAYS\");\n    let nums: [i32];\n    nums.push(10);\n    nums.push(20);\n    nums.push(30);\n    print(f\"Array length: {nums.length}\");\n\n    print(");
    printf("%s\n", "4. FUNCTIONAL PROGRAMMING");
    void* squares = __lambda_0;
    DynamicArray* mapped = array_map_i32(nums, squares);
    printf("%s\n", "Squared values:");
    int32_t i = 0;
    while ((i < mapped->length)) {
        printf("  %d\n", ((int32_t*)mapped->data)[i]);
        i = (i + 1);
    }
    printf("%s\n", ");\n    print(\"5. CLOSURES WITH CAPTURE\");\n    let multiplier: i32 = 3;\n    let scale: function = function(x: i32, dummy: i32): i32 {\n        return x * multiplier;\n    };\n    let scaled: i32 = scale(7);\n    print(f\"scale(7) with multiplier={multiplier}: {scaled}\");\n\n    print(");
    printf("%s\n", "6. DEFER STATEMENT");
    {
        printf("%s\n", "  Inside block");
        printf("%s\n", "  Deferred cleanup executed!");
    }
    printf("%s\n", ");\n    print(\"7. CONTROL FLOW\");\n    let choice: i32 = 2;\n    switch (choice) {\n        case 1:\n            print(\"  Case 1\");\n            break;\n        case 2:\n            print(\"  Case 2 selected\");\n            break;\n        default:\n            print(\"  Default\");\n            break;\n    }\n\n    print(");
    printf("%s\n", "8. OBJECTS");
    struct { int32_t x; int32_t y; } point = {.x = 5, .y = 10};
    point.x = (point.x + 15);
    printf("Point: (%d, %d)\n", point.x, point.y);
    printf("%s\n", ");\n    print(\"9. MATH FUNCTIONS\");\n    let calc: i32 = max(abs(-10), min(5, 20));\n    print(f\"max(abs(-10), min(5,20)) = {calc}\");\n\n    print(");
    printf("%s\n", "10. NOTHING TYPE");
    int32_t ptr = NULL;
    if ((ptr == NULL)) {
        printf("%s\n", "nothing works correctly!");
    }
    printf("%s\n", "========================================");
    printf("%s\n", "   All HiLow Features Demonstrated!");
    printf("%s\n", "========================================");
    return 0;
}

