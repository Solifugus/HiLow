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

// Time and duration type support (Phase 9c)
// Time precision levels for precision-aware comparison
typedef enum {
    HL_TIME_PREC_DAY,
    HL_TIME_PREC_HOUR,
    HL_TIME_PREC_MINUTE,
    HL_TIME_PREC_SECOND,
    HL_TIME_PREC_MILLI,
    HL_TIME_PREC_MICRO,
    HL_TIME_PREC_NANO,
} HiLowTimePrecision;

// Time type: nanoseconds since epoch + precision tag
typedef struct {
    int64_t nanos_since_epoch;
    HiLowTimePrecision precision;
} HiLowTime;

// Duration type: just nanoseconds (no precision tag)
typedef struct {
    int64_t nanos;
} HiLowDuration;

// Money type support (Phase 9d)
// Currency enum for supported currencies
typedef enum {
    HL_CURRENCY_USD,
    HL_CURRENCY_EUR,
    HL_CURRENCY_GBP,
    HL_CURRENCY_JPY,
    HL_CURRENCY_CAD,
    HL_CURRENCY_AUD,
    HL_CURRENCY_CHF,
    HL_CURRENCY_CNY,
} HiLowCurrency;

// Money type: amount in micro-units (4 decimal places) + currency
typedef struct {
    int64_t amount;           // amount in micro-units (e.g., $19.99 stored as 199900)
    HiLowCurrency currency;
} HiLowMoney;

// Optional type support (Phase 9b fix 3a)
// Proper wrapper struct for T? values to replace broken bit-packing approach
typedef enum {
    HL_OPT_I32,
    HL_OPT_STRING,
    HL_OPT_UNKNOWN,
    HL_OPT_TIME,
    HL_OPT_DURATION,
    HL_OPT_MONEY,
    // Add others as needed by tests
} HiLowOptionalKind;

typedef struct HiLowOptional {
    int refcount;
    HiLowOptionalKind kind;
    union {
        int32_t i32_val;
        const char* str_val;
        HiLowUnknown* unk_val;
        HiLowTime time_val;
        HiLowDuration duration_val;
        HiLowMoney money_val;
    } payload;
} HiLowOptional;

// Unknown type checking (updated for HiLowOptional)
bool hl_is_unknown(HiLowOptional* opt);

// Optional constructor functions
HiLowOptional* hl_optional_new_i32(int32_t v);
HiLowOptional* hl_optional_new_string(const char* s);
HiLowOptional* hl_optional_new_unknown(HiLowUnknown* u);
HiLowOptional* hl_optional_new_time(HiLowTime t);
HiLowOptional* hl_optional_new_duration(HiLowDuration d);
HiLowOptional* hl_optional_new_money(HiLowMoney m);

// Optional memory management
void hl_optional_retain(HiLowOptional* opt);
void hl_optional_release(HiLowOptional* opt);

// F-string format helpers
char* hl_format_binary(unsigned long long value);
char* hl_format_center(const char* value, int width);

// Function value support (Phase 7c-β)
typedef struct HiLowFunction {
    int refcount;          // Reference count (Phase 8b)
    void* fn_ptr;          // pointer to the C function
    void* env;             // captured environment; NULL for non-closures
} HiLowFunction;

// Watcher value support (Phase 10-δ-α)
typedef struct HiLowWatcher {
    int refcount;          // Reference count for memory management
    bool active;           // Whether the watcher is currently active
    bool ended;            // Whether the watcher has been permanently ended
} HiLowWatcher;

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
    HL_VALUE_FUNCTION,
    HL_VALUE_MONEY
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
        HiLowMoney money_val;
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

// Watcher value operations (Phase 10-δ-α)
HiLowWatcher* hl_watcher_new(void);
void hl_watcher_retain(HiLowWatcher* w);
void hl_watcher_release(HiLowWatcher* w);
void hl_watcher_pause(HiLowWatcher* w);
void hl_watcher_resume(HiLowWatcher* w);
void hl_watcher_end(HiLowWatcher* w);
bool hl_watcher_is_active(HiLowWatcher* w);

// Array support (Array Phase A) and watcher scaffolding (Array Phase B)
// Forward-declared; full definition populated in Phase 10-ε.
typedef struct HiLowArrayWatcher HiLowArrayWatcher;

// Element function pointer type (Phase C)
typedef void (*hl_elem_fn)(void*);

typedef struct HiLowArray {
    int refcount;
    size_t length;
    size_t capacity;
    size_t elem_size;
    void* data;
    HiLowArrayWatcher* watchers;   // Phase B scaffolding: head of subscription list (always NULL in Phase B)
    hl_elem_fn retain_fn;          // NULL for primitive arrays, hl_object_retain for object arrays
    hl_elem_fn release_fn;         // NULL for primitive arrays, hl_object_release for object arrays
} HiLowArray;

// Phase B scaffolding: the subscription node. Phase 10-ε fills in the calling
// convention for firing the watcher body with delta information. For now the
// struct exists so mutation operations can walk an (always-empty) list.
struct HiLowArrayWatcher {
    int modifier;                   // ADDED / REMOVED / CHANGED / DEEP / MOVED (enum values; define a small set)
    void* body_fn;                  // watcher body function pointer (unused until 10-ε)
    void** captured_vars;           // captured context (unused until 10-ε)
    void* watcher_state;            // HiLowWatcher* for active/ended gating (unused until 10-ε)
    HiLowArrayWatcher* next;
};

// Array watcher modifier constants (Phase B scaffolding)
#define HL_ARR_ADDED 1
#define HL_ARR_REMOVED 2
#define HL_ARR_CHANGED 3
#define HL_ARR_DEEP 4
#define HL_ARR_MOVED 5

HiLowArray* hl_array_new(size_t elem_size, size_t initial_capacity, hl_elem_fn retain_fn, hl_elem_fn release_fn);
void hl_array_retain(HiLowArray* arr);
void hl_array_release(HiLowArray* arr);
void hl_array_push(HiLowArray* arr, void* elem);   // copies elem_size bytes from elem into the buffer, growing if needed, now with firing loop
void* hl_array_get(HiLowArray* arr, size_t index); // returns pointer to element slot (caller casts and dereferences)
size_t hl_array_len(HiLowArray* arr);
void* hl_array_pop(HiLowArray* arr);                // removes and returns pointer to the last element slot; decrements length
void hl_array_set(HiLowArray* arr, size_t index, void* elem); // overwrites element at index with firing loop
void* hl_array_remove(HiLowArray* arr, size_t index); // removes and returns element at index, shifting trailing elements down
void hl_array_insert(HiLowArray* arr, size_t index, void* elem); // inserts element at index, shifting trailing elements up

// Array watcher registration (Phase 10-ε-α)
void hl_array_register_watcher(HiLowArray* arr, int modifier, void* body_fn, void* watcher_state);

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
int32_t hl_optional_unwrap_i32(HiLowOptional* opt);
int64_t hl_optional_unwrap_i64(HiLowOptional* opt);
uint32_t hl_optional_unwrap_u32(HiLowOptional* opt);
uint64_t hl_optional_unwrap_u64(HiLowOptional* opt);
float hl_optional_unwrap_f32(HiLowOptional* opt);
double hl_optional_unwrap_f64(HiLowOptional* opt);
bool hl_optional_unwrap_bool(HiLowOptional* opt);
const char* hl_optional_unwrap_string(HiLowOptional* opt);
HiLowUnknown* hl_optional_unwrap_unknown(HiLowOptional* opt);
HiLowTime hl_optional_unwrap_time(HiLowOptional* opt);
HiLowDuration hl_optional_unwrap_duration(HiLowOptional* opt);
HiLowMoney hl_optional_unwrap_money(HiLowOptional* opt);

// Print functions for optional types (Phase 9b)
// These check the tag and call either print_unknown or print_T
void print_optional_i32(HiLowOptional* opt);
void print_optional_i64(HiLowOptional* opt);
void print_optional_u32(HiLowOptional* opt);
void print_optional_u64(HiLowOptional* opt);
void print_optional_f32(HiLowOptional* opt);
void print_optional_f64(HiLowOptional* opt);
void print_optional_bool(HiLowOptional* opt);
void print_optional_string(HiLowOptional* opt);
void print_optional_money(HiLowOptional* opt);

// Time constructor functions
HiLowTime hl_time_now(void);
HiLowOptional* hl_time_parse(const char* iso_string);

// Time arithmetic functions
HiLowTime hl_time_add_duration(HiLowTime time, HiLowDuration duration);
HiLowTime hl_time_sub_duration(HiLowTime time, HiLowDuration duration);
HiLowDuration hl_time_sub_time(HiLowTime lhs, HiLowTime rhs);
HiLowDuration hl_duration_add(HiLowDuration lhs, HiLowDuration rhs);

// Time comparison functions (precision-aware)
bool hl_time_eq(HiLowTime lhs, HiLowTime rhs);
bool hl_time_ne(HiLowTime lhs, HiLowTime rhs);
bool hl_time_lt(HiLowTime lhs, HiLowTime rhs);
bool hl_time_le(HiLowTime lhs, HiLowTime rhs);
bool hl_time_gt(HiLowTime lhs, HiLowTime rhs);
bool hl_time_ge(HiLowTime lhs, HiLowTime rhs);

// Duration comparison functions
bool hl_duration_eq(HiLowDuration lhs, HiLowDuration rhs);
bool hl_duration_ne(HiLowDuration lhs, HiLowDuration rhs);
bool hl_duration_lt(HiLowDuration lhs, HiLowDuration rhs);
bool hl_duration_le(HiLowDuration lhs, HiLowDuration rhs);
bool hl_duration_gt(HiLowDuration lhs, HiLowDuration rhs);
bool hl_duration_ge(HiLowDuration lhs, HiLowDuration rhs);

// Print functions for time and duration
void print_time(HiLowTime time);
void print_duration(HiLowDuration duration);

// Money functions (Phase 9d)
void print_money(HiLowMoney money);

// Money arithmetic functions
HiLowMoney hl_money_add(HiLowMoney lhs, HiLowMoney rhs);
HiLowMoney hl_money_sub(HiLowMoney lhs, HiLowMoney rhs);
HiLowMoney hl_money_mul_scalar(HiLowMoney money, double scalar);
HiLowMoney hl_money_div_scalar(HiLowMoney money, double scalar);
double hl_money_div_money(HiLowMoney lhs, HiLowMoney rhs);

// Money comparison functions
bool hl_money_eq(HiLowMoney lhs, HiLowMoney rhs);
bool hl_money_ne(HiLowMoney lhs, HiLowMoney rhs);
bool hl_money_lt(HiLowMoney lhs, HiLowMoney rhs);
bool hl_money_le(HiLowMoney lhs, HiLowMoney rhs);
bool hl_money_gt(HiLowMoney lhs, HiLowMoney rhs);
bool hl_money_ge(HiLowMoney lhs, HiLowMoney rhs);

#endif // HILOW_RUNTIME_H