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

int32_t test_defer_order();
int32_t test_defer_scope();
int32_t test_defer_return(int32_t val);
int32_t main();

int32_t test_defer_order() {
    printf("%s\n", "=== Defer Execution Order ===");
    printf("%s\n", "Step 1: First statement");
    printf("%s\n", "Step 2: First defer");
    printf("%s\n", "Step 3: Last defer");
    return 1;
    printf("%s\n", "Step 2: First defer");
    printf("%s\n", "Step 3: Last defer");
}

int32_t test_defer_scope() {
    printf("%s\n", "=== Defer Scope Test ===");
    int32_t x = 0;
    {
        printf("%s\n", "Inside inner block");
        x = (x + 10);
        x = (x + 1);
    }
    printf("After inner block: x=%d\n", x);
    x = (x + 100);
    return x;
    x = (x + 100);
}

int32_t test_defer_return(int32_t val) {
    if ((val < 0)) {
        printf("%s\n", "Early return path");
        printf("%s\n", "Defer executed");
        return -(1);
    }
    printf("%s\n", "Normal return path");
    printf("%s\n", "Defer executed");
    return val;
    printf("%s\n", "Defer executed");
}

int32_t main() {
    test_defer_order();
    printf("%s\n", ");\n\n    let result: i32 = test_defer_scope();\n    print(");
    printf("%s\n", "=== Defer with Early Return ===");
    test_defer_return(5);
    printf("%s\n", ");\n\n    print(\"=== Defer with Normal Return ===\");\n    test_defer_return(-2);\n    print(");
    printf("%s\n", "Phase 7 defer validation complete!");
    return result;
}

