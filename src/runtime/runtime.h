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

// Nothing type support (Phase 9a)
// Global nothing singleton - all nothing values are pointers to this
typedef struct HiLowNothing {
    int sentinel; // Just to make it a real struct
} HiLowNothing;

extern HiLowNothing the_nothing;
void print_nothing(void);

// Unknown type support (Phase 9b)
// Runtime representation of unknown values with reason and optional suggestions
typedef struct HiLowUnknown {
    int refcount;             // Reference count for memory management
    const char* reason;       // Why the operation failed (required)
    const char** options;     // Null-terminated array of suggested fixes (optional)
    int options_count;        // Number of options (0 if none)
} HiLowUnknown;

// Unknown constructor functions
HiLowUnknown* hl_unknown_new(const char* reason);
HiLowUnknown* hl_unknown_new_with_options(const char* reason, const char** options, int options_count);

// Unknown memory management
void hl_unknown_retain(HiLowUnknown* unknown);
void hl_unknown_release(HiLowUnknown* unknown);

// Unknown property access
const char* hl_unknown_get_reason(HiLowUnknown* unknown);
const char** hl_unknown_get_options(HiLowUnknown* unknown);
int hl_unknown_get_options_count(HiLowUnknown* unknown);

// Unknown print support
void print_unknown(HiLowUnknown* unknown);

// Unknown type checking
bool hl_is_unknown(void* value);

// F-string format helpers
char* hl_format_binary(unsigned long long value);
char* hl_format_center(const char* value, int width);

// Function value support (Phase 7c-β)
typedef struct HiLowFunction {
    int refcount;          // Reference count (Phase 8b)
    void* fn_ptr;          // pointer to the C function
    void* env;             // captured environment; NULL for non-closures
} HiLowFunction;

// Object support (Phase 7a)
// Tagged union for all HiLow values that can be stored as object properties
typedef enum {
    HL_VALUE_I32,
    HL_VALUE_I64,
    HL_VALUE_U32,
    HL_VALUE_U64,
    HL_VALUE_F32,
    HL_VALUE_F64,
    HL_VALUE_BOOL,
    HL_VALUE_STR,
    HL_VALUE_OBJECT,
    HL_VALUE_FUNCTION
} HiLowValueType;

typedef struct HiLowValue {
    HiLowValueType type;
    union {
        int32_t i32_val;
        int64_t i64_val;
        uint32_t u32_val;
        uint64_t u64_val;
        float f32_val;
        double f64_val;
        bool bool_val;
        char* str_val;
        struct HiLowObject* obj_val;
        HiLowFunction* fn_val;
    } value;
} HiLowValue;

// Forward declaration for weak reference tracking (Phase 8c)
struct HiLowObject;
struct WeakRef;

// Property in an object (key-value pair)
typedef struct Property {
    const char* key;
    HiLowValue value;
    bool is_weak;  // NEW: indicates if this property holds a weak reference
} Property;

// Object representation (heap-allocated with property table)
typedef struct HiLowObject {
    int refcount;              // Reference count (Phase 8b)
    Property* properties;
    size_t property_count;
    size_t property_capacity;
    struct WeakRef* weak_refs; // NEW: linked list of weak references to this object
} HiLowObject;

// Weak reference tracking (Phase 8c)
typedef struct WeakRef {
    HiLowObject** location;  // address of the pointer to invalidate
    struct WeakRef* next;
} WeakRef;

// Object operations
HiLowObject* hl_object_new(void);
void hl_object_set_i32(HiLowObject* obj, const char* key, int32_t value);
void hl_object_set_i64(HiLowObject* obj, const char* key, int64_t value);
void hl_object_set_u32(HiLowObject* obj, const char* key, uint32_t value);
void hl_object_set_u64(HiLowObject* obj, const char* key, uint64_t value);
void hl_object_set_f32(HiLowObject* obj, const char* key, float value);
void hl_object_set_f64(HiLowObject* obj, const char* key, double value);
void hl_object_set_bool(HiLowObject* obj, const char* key, bool value);
void hl_object_set_str(HiLowObject* obj, const char* key, const char* value);
void hl_object_set_object(HiLowObject* obj, const char* key, HiLowObject* value);
void hl_object_set_function(HiLowObject* obj, const char* key, HiLowFunction* value);

int32_t hl_object_get_i32(HiLowObject* obj, const char* key);
int64_t hl_object_get_i64(HiLowObject* obj, const char* key);
uint32_t hl_object_get_u32(HiLowObject* obj, const char* key);
uint64_t hl_object_get_u64(HiLowObject* obj, const char* key);
float hl_object_get_f32(HiLowObject* obj, const char* key);
double hl_object_get_f64(HiLowObject* obj, const char* key);
bool hl_object_get_bool(HiLowObject* obj, const char* key);
char* hl_object_get_str(HiLowObject* obj, const char* key);
HiLowObject* hl_object_get_object(HiLowObject* obj, const char* key);
HiLowFunction* hl_object_get_function(HiLowObject* obj, const char* key);

// Function value operations (Phase 7c-β)
HiLowFunction* hl_function_new(void* fn_ptr);
HiLowFunction* hl_function_new_with_env(void* fn_ptr, void* env);

// Phase 7b-extension: Object is check
bool hl_object_is(HiLowObject* child, HiLowObject* parent);

// Phase 7c-ζ: For-in iteration helpers
size_t hl_object_property_count(HiLowObject* obj);
const char* hl_object_property_key_at(HiLowObject* obj, size_t index);
int hl_object_property_type_at(HiLowObject* obj, size_t index);
int32_t hl_object_property_value_i32_at(HiLowObject* obj, size_t index);
int64_t hl_object_property_value_i64_at(HiLowObject* obj, size_t index);
uint32_t hl_object_property_value_u32_at(HiLowObject* obj, size_t index);
uint64_t hl_object_property_value_u64_at(HiLowObject* obj, size_t index);
float hl_object_property_value_f32_at(HiLowObject* obj, size_t index);
double hl_object_property_value_f64_at(HiLowObject* obj, size_t index);
bool hl_object_property_value_bool_at(HiLowObject* obj, size_t index);
char* hl_object_property_value_str_at(HiLowObject* obj, size_t index);
HiLowObject* hl_object_property_value_object_at(HiLowObject* obj, size_t index);
HiLowFunction* hl_object_property_value_function_at(HiLowObject* obj, size_t index);

// Type constants for runtime dispatch (Phase 7c-ζ)
#define TYPE_I32 1
#define TYPE_I64 2
#define TYPE_U32 3
#define TYPE_U64 4
#define TYPE_F32 5
#define TYPE_F64 6
#define TYPE_BOOL 7
#define TYPE_STR 8
#define TYPE_OBJECT 9
#define TYPE_FUNCTION 10

// Debug allocator (Phase 8a)
extern int hl_alloc_count;
extern int hl_free_count;

// Free helpers for heap-allocated types (Phase 8a)
void hl_object_free(HiLowObject* obj);
void hl_function_free(HiLowFunction* fn);

// Refcounting operations (Phase 8b)
void hl_object_retain(HiLowObject* obj);
void hl_object_release(HiLowObject* obj);
void hl_function_retain(HiLowFunction* fn);
void hl_function_release(HiLowFunction* fn);

// Weak reference operations (Phase 8c)
void hl_object_weak_register(HiLowObject* target, HiLowObject** location);
void hl_object_weak_unregister(HiLowObject* target, HiLowObject** location);
HiLowObject** hl_object_property_addr(HiLowObject* obj, const char* key);

// Optional unwrap helpers for narrowed types (Phase 9b)
// These extract the underlying T value from a T? that is known to hold T (not unknown)
// Calling these on an unknown-state optional is undefined behavior
int32_t hl_optional_unwrap_i32(void* optional);
const char* hl_optional_unwrap_string(void* optional);

#endif // HILOW_RUNTIME_H