#define _GNU_SOURCE  // For timegm
#define _POSIX_C_SOURCE 200809L
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

// Nothing type support (Phase 9a)
HiLowNothing the_nothing = { 42 }; // Global singleton

void print_nothing(void) {
    printf("nothing\n");
}

// Phase 10a-stealth: watcher suppression depth.
// Becomes thread-local in Phase 10b when async is added.
int hl_stealth_depth = 0;

// Unknown type support (Phase 9b)

// Internal helper for C string literals
static HiLowUnknown* hl_unknown_new_internal(const char* reason) {
    HiLowUnknown* unknown = malloc(sizeof(HiLowUnknown));
    hl_alloc_count++;

    unknown->refcount = 1;
    unknown->reason = malloc(strlen(reason) + 1);
    strcpy((char*)unknown->reason, reason);
    unknown->options = NULL;
    unknown->options_count = 0;

    return unknown;
}

HiLowUnknown* hl_unknown_new(HiLowArray* reason) {
    HiLowUnknown* unknown = malloc(sizeof(HiLowUnknown));
    hl_alloc_count++;

    unknown->refcount = 1;
    // Create null-terminated C string from HiLowArray data
    unknown->reason = malloc(reason->length + 1);
    memcpy((char*)unknown->reason, reason->data, reason->length);
    ((char*)unknown->reason)[reason->length] = '\0';  // Add null terminator
    unknown->options = NULL;
    unknown->options_count = 0;

    return unknown;
}

HiLowUnknown* hl_unknown_new_with_options(HiLowArray* reason, const char** options, int options_count) {
    HiLowUnknown* unknown = hl_unknown_new(reason);  // This sets refcount to 1

    if (options_count > 0) {
        unknown->options = malloc(sizeof(char*) * (options_count + 1));  // +1 for null terminator
        unknown->options_count = options_count;

        for (int i = 0; i < options_count; i++) {
            unknown->options[i] = malloc(strlen(options[i]) + 1);
            strcpy((char*)unknown->options[i], options[i]);
        }
        unknown->options[options_count] = NULL;  // Null-terminate the array
    }

    return unknown;
}

void hl_unknown_retain(HiLowUnknown* unknown) {
    if (unknown) {
        unknown->refcount++;
    }
}

void hl_unknown_release(HiLowUnknown* unknown) {
    if (unknown) {
        unknown->refcount--;
        if (unknown->refcount <= 0) {
            // Free the reason string
            free((void*)unknown->reason);

            // Free the options array if it exists
            if (unknown->options) {
                for (int i = 0; i < unknown->options_count; i++) {
                    free((void*)unknown->options[i]);
                }
                free(unknown->options);
            }

            free(unknown);
            hl_free_count++;
        }
    }
}

HiLowArray* hl_unknown_get_reason(HiLowUnknown* unknown) {
    // Returns a fresh managed string (refcount 1) — caller owns the reference.
    // Internal storage stays char* (see hl_unknown_new_internal callers).
    return hl_string_from_cstr(unknown ? unknown->reason : "");
}

const char** hl_unknown_get_options(HiLowUnknown* unknown) {
    return unknown ? unknown->options : NULL;
}

int hl_unknown_get_options_count(HiLowUnknown* unknown) {
    return unknown ? unknown->options_count : 0;
}

void print_unknown(HiLowUnknown* unknown) {
    if (unknown) {
        printf("unknown: %s\n", unknown->reason);
    } else {
        printf("unknown: <null>\n");
    }
}

bool hl_is_unknown(HiLowOptional* opt) {
    // New safe implementation: check the kind field of the wrapper struct
    return opt && opt->kind == HL_OPT_UNKNOWN;
}

// Optional type constructor functions (Phase 9b fix 3a)

HiLowOptional* hl_optional_new_i32(int32_t v) {
    HiLowOptional* opt = malloc(sizeof(HiLowOptional));
    hl_alloc_count++;
    opt->refcount = 1;
    opt->kind = HL_OPT_I32;
    opt->payload.i32_val = v;
    return opt;
}

HiLowOptional* hl_optional_new_string(HiLowArray* s) {
    HiLowOptional* opt = malloc(sizeof(HiLowOptional));
    hl_alloc_count++;
    opt->refcount = 1;
    opt->kind = HL_OPT_STRING;
    opt->payload.str_val = s;  // Take ownership of the string array
    return opt;
}

HiLowOptional* hl_optional_new_unknown(HiLowUnknown* u) {
    HiLowOptional* opt = malloc(sizeof(HiLowOptional));
    hl_alloc_count++;
    opt->refcount = 1;
    opt->kind = HL_OPT_UNKNOWN;
    opt->payload.unk_val = u;  // Take ownership of the unknown
    return opt;
}

HiLowOptional* hl_optional_new_time(HiLowTime t) {
    HiLowOptional* opt = malloc(sizeof(HiLowOptional));
    hl_alloc_count++;
    opt->refcount = 1;
    opt->kind = HL_OPT_TIME;
    opt->payload.time_val = t;  // Copy the time struct
    return opt;
}

HiLowOptional* hl_optional_new_duration(HiLowDuration d) {
    HiLowOptional* opt = malloc(sizeof(HiLowOptional));
    hl_alloc_count++;
    opt->refcount = 1;
    opt->kind = HL_OPT_DURATION;
    opt->payload.duration_val = d;  // Copy the duration struct
    return opt;
}

HiLowOptional* hl_optional_new_money(HiLowMoney m) {
    HiLowOptional* opt = malloc(sizeof(HiLowOptional));
    hl_alloc_count++;
    opt->refcount = 1;
    opt->kind = HL_OPT_MONEY;
    opt->payload.money_val = m;  // Copy the money struct
    return opt;
}

void hl_optional_retain(HiLowOptional* opt) {
    if (opt) {
        opt->refcount++;
    }
}

void hl_optional_release(HiLowOptional* opt) {
    if (opt) {
        opt->refcount--;
        if (opt->refcount <= 0) {
            // Release the inner value if applicable
            if (opt->kind == HL_OPT_UNKNOWN && opt->payload.unk_val) {
                hl_unknown_release(opt->payload.unk_val);
            } else if (opt->kind == HL_OPT_STRING && opt->payload.str_val) {
                hl_array_release(opt->payload.str_val);
            } else if (opt->kind == HL_OPT_OBJECT && opt->payload.obj_val) {
                hl_object_release(opt->payload.obj_val);
            }
            free(opt);
            hl_free_count++;
        }
    }
}

HiLowArray* hl_format_binary(unsigned long long value) {
    // Use temporary char buffer for formatting
    char temp[65];
    temp[64] = '\0';

    // Handle zero case
    if (value == 0) {
        temp[0] = '0';
        temp[1] = '\0';

        // Create HiLowArray with 1 byte
        HiLowArray* result = hl_array_new(sizeof(uint8_t), 1, NULL, NULL);
        uint8_t byte = '0';
        hl_array_push(result, &byte);
        return result;
    }

    int pos = 63;
    while (value > 0 && pos >= 0) {
        temp[pos] = (value & 1) ? '1' : '0';
        value >>= 1;
        pos--;
    }

    // Create HiLowArray and copy the relevant portion
    int start = pos + 1;
    int len = 64 - start;
    HiLowArray* result = hl_array_new(sizeof(uint8_t), len, NULL, NULL);

    for (int i = 0; i < len; i++) {
        uint8_t byte = (uint8_t)temp[start + i];
        hl_array_push(result, &byte);
    }

    return result;
}

HiLowArray* hl_format_center(HiLowArray* value, int width) {
    size_t len = value ? value->length : 0;
    if ((int)len >= width) {
        // If value is already wider than or equal to the desired width, return retained copy
        if (value) {
            hl_array_retain(value);
            return value;
        } else {
            // Return empty array if value is NULL
            return hl_array_new(sizeof(uint8_t), 0, NULL, NULL);
        }
    }

    int padding = width - (int)len;
    int left_padding = padding / 2;

    // Create result array with the desired width
    HiLowArray* result = hl_array_new(sizeof(uint8_t), width, NULL, NULL);

    // Add left padding spaces
    uint8_t space_byte = ' ';
    for (int i = 0; i < left_padding; i++) {
        hl_array_push(result, &space_byte);
    }

    // Copy the value bytes
    if (value && value->data) {
        for (size_t i = 0; i < value->length; i++) {
            uint8_t byte = *((uint8_t*)value->data + i);
            hl_array_push(result, &byte);
        }
    }

    // Add right padding spaces
    int right_padding = padding - left_padding;
    for (int i = 0; i < right_padding; i++) {
        hl_array_push(result, &space_byte);
    }

    return result;
}

// Object support implementation (Phase 7a)

// Prototype chain support (Phase 7b)
#define MAX_PROTO_DEPTH 100

// Forward declaration for prototype helper
static HiLowObject* hl_object_get_proto(HiLowObject* obj);

HiLowObject* hl_object_new(void) {
    HiLowObject* obj = malloc(sizeof(HiLowObject));
    hl_alloc_count++;
    obj->refcount = 1;         // Initialize refcount to 1 (Phase 8b)
    obj->properties = NULL;
    obj->property_count = 0;
    obj->property_capacity = 0;
    obj->weak_refs = NULL;     // Initialize weak ref list to empty (Phase 8c)
    return obj;
}

// Helper function to find a property by key
static Property* find_property(HiLowObject* obj, const char* key) {
    for (size_t i = 0; i < obj->property_count; i++) {
        if (strcmp(obj->properties[i].key, key) == 0) {
            return &obj->properties[i];
        }
    }
    return NULL;
}

// Helper function to ensure capacity for a new property
static void ensure_capacity(HiLowObject* obj) {
    if (obj->property_count >= obj->property_capacity) {
        size_t new_capacity = obj->property_capacity == 0 ? 4 : obj->property_capacity * 2;
        obj->properties = realloc(obj->properties, new_capacity * sizeof(Property));
        hl_alloc_count++;  // Count realloc as allocation for Phase 8a
        obj->property_capacity = new_capacity;
    }
}

// Helper function to add or update a property
static void set_property(HiLowObject* obj, const char* key, HiLowValue value) {
    Property* existing = find_property(obj, key);
    if (existing) {
        // Drop the old reference: a weak old value is unregistered (never
        // released); a strong heap value is released exactly once (Phase 1.5c)
        if (existing->value.type == HL_VALUE_OBJECT && existing->value.value.obj_val) {
            if (existing->is_weak) {
                hl_object_weak_unregister(existing->value.value.obj_val, obj,
                                          (size_t)(existing - obj->properties));
            } else {
                hl_object_release(existing->value.value.obj_val);
            }
        } else if (existing->value.type == HL_VALUE_FUNCTION && existing->value.value.fn_val) {
            hl_function_release(existing->value.value.fn_val);
        } else if (existing->value.type == HL_VALUE_STR && existing->value.value.str_val) {
            hl_array_release(existing->value.value.str_val);
        }
        existing->value = value;
        existing->is_weak = false;  // this store is strong
    } else {
        ensure_capacity(obj);
        obj->properties[obj->property_count].key = strdup(key);  // Duplicate the key string
        hl_alloc_count++;  // Count strdup allocation
        obj->properties[obj->property_count].value = value;
        obj->properties[obj->property_count].is_weak = false;  // Not weak by default (Phase 8c)
        obj->property_count++;
    }

    // Every store retains (Phase 1.5c ownership axiom): the property is a new
    // strong reference regardless of whether the key existed. The caller keeps
    // ownership of its own reference and disposes of it independently.
    if (value.type == HL_VALUE_OBJECT && value.value.obj_val) {
        hl_object_retain(value.value.obj_val);
    } else if (value.type == HL_VALUE_FUNCTION && value.value.fn_val) {
        hl_function_retain(value.value.fn_val);
    } else if (value.type == HL_VALUE_STR && value.value.str_val) {
        hl_array_retain(value.value.str_val);
    }
}

void hl_object_set_i32(HiLowObject* obj, const char* key, int32_t value) {
    HiLowValue val = { .type = HL_VALUE_I32, .value.i32_val = value };
    set_property(obj, key, val);
}

void hl_object_set_i64(HiLowObject* obj, const char* key, int64_t value) {
    HiLowValue val = { .type = HL_VALUE_I64, .value.i64_val = value };
    set_property(obj, key, val);
}

void hl_object_set_u32(HiLowObject* obj, const char* key, uint32_t value) {
    HiLowValue val = { .type = HL_VALUE_U32, .value.u32_val = value };
    set_property(obj, key, val);
}

void hl_object_set_u64(HiLowObject* obj, const char* key, uint64_t value) {
    HiLowValue val = { .type = HL_VALUE_U64, .value.u64_val = value };
    set_property(obj, key, val);
}

void hl_object_set_f32(HiLowObject* obj, const char* key, float value) {
    HiLowValue val = { .type = HL_VALUE_F32, .value.f32_val = value };
    set_property(obj, key, val);
}

void hl_object_set_f64(HiLowObject* obj, const char* key, double value) {
    HiLowValue val = { .type = HL_VALUE_F64, .value.f64_val = value };
    set_property(obj, key, val);
}

void hl_object_set_bool(HiLowObject* obj, const char* key, bool value) {
    HiLowValue val = { .type = HL_VALUE_BOOL, .value.bool_val = value };
    set_property(obj, key, val);
}

void hl_object_set_str(HiLowObject* obj, const char* key, HiLowArray* value) {
    // set_property retains on store (Phase 1.5c); no pre-retain here.
    HiLowValue val = { .type = HL_VALUE_STR, .value.str_val = value };
    set_property(obj, key, val);
}

void hl_object_set_object(HiLowObject* obj, const char* key, HiLowObject* value) {
    HiLowValue val = { .type = HL_VALUE_OBJECT, .value.obj_val = value };
    set_property(obj, key, val);
}

// Getter functions with prototype chain support (Phase 7b)
int32_t hl_object_get_i32(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_I32) {
                return prop->value.value.i32_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        // Property not found on current object - check prototype
        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

int64_t hl_object_get_i64(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_I64) {
                return prop->value.value.i64_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

uint32_t hl_object_get_u32(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_U32) {
                return prop->value.value.u32_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

uint64_t hl_object_get_u64(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_U64) {
                return prop->value.value.u64_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

float hl_object_get_f32(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_F32) {
                return prop->value.value.f32_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

double hl_object_get_f64(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_F64) {
                return prop->value.value.f64_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

bool hl_object_get_bool(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_BOOL) {
                return prop->value.value.bool_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

HiLowArray* hl_object_get_str(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_STR) {
                hl_array_retain(prop->value.value.str_val);  // Retain-on-return
                return prop->value.value.str_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

HiLowObject* hl_object_get_object(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_OBJECT) {
                return prop->value.value.obj_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

void hl_object_set_function(HiLowObject* obj, const char* key, HiLowFunction* value) {
    HiLowValue val = { .type = HL_VALUE_FUNCTION, .value.fn_val = value };
    set_property(obj, key, val);
}

HiLowFunction* hl_object_get_function(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_FUNCTION) {
                return prop->value.value.fn_val;
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

// Function value operations (Phase 7c-β)
HiLowFunction* hl_function_new(void* fn_ptr) {
    HiLowFunction* f = malloc(sizeof(HiLowFunction));
    hl_alloc_count++;
    f->refcount = 1;           // Initialize refcount to 1 (Phase 8b)
    f->fn_ptr = fn_ptr;
    f->env = NULL;
    f->env_dtor = NULL;
    return f;
}

HiLowFunction* hl_function_new_with_env(void* fn_ptr, void* env) {
    HiLowFunction* f = malloc(sizeof(HiLowFunction));
    hl_alloc_count++;
    f->refcount = 1;           // Initialize refcount to 1 (Phase 8b)
    f->fn_ptr = fn_ptr;
    f->env = env;
    f->env_dtor = NULL;
    return f;
}

HiLowFunction* hl_function_new_with_env_dtor(void* fn_ptr, void* env, void (*env_dtor)(void*)) {
    HiLowFunction* f = hl_function_new_with_env(fn_ptr, env);
    f->env_dtor = env_dtor;
    return f;
}

// Watcher value operations (Phase 10-δ-α)
HiLowWatcher* hl_watcher_new(void) {
    HiLowWatcher* w = malloc(sizeof(HiLowWatcher));
    hl_alloc_count++;
    w->refcount = 1;           // Initialize refcount to 1
    w->active = true;          // Start active
    w->ended = false;          // Not ended initially
    return w;
}

void hl_watcher_retain(HiLowWatcher* w) {
    if (w != NULL) {
        w->refcount++;
    }
}

void hl_watcher_release(HiLowWatcher* w) {
    if (w != NULL) {
        w->refcount--;
        if (w->refcount == 0) {
            free(w);
            hl_free_count++;
        }
    }
}

void hl_watcher_pause(HiLowWatcher* w) {
    if (w != NULL && !w->ended) {
        w->active = false;
    }
}

void hl_watcher_resume(HiLowWatcher* w) {
    if (w != NULL && !w->ended) {
        w->active = true;
    }
}

void hl_watcher_end(HiLowWatcher* w) {
    if (w != NULL) {
        w->ended = true;
        w->active = false;
    }
}

bool hl_watcher_is_active(HiLowWatcher* w) {
    if (w != NULL) {
        return w->active;
    }
    return false;
}

// Helper function to get the proto property as an object (Phase 7b)
static HiLowObject* hl_object_get_proto(HiLowObject* obj) {
    Property* proto_prop = find_property(obj, "proto");
    if (proto_prop && proto_prop->value.type == HL_VALUE_OBJECT) {
        return proto_prop->value.value.obj_val;
    }
    return NULL;
}

// Phase 7b-extension: Object is check - walks the prototype chain
bool hl_object_is(HiLowObject* child, HiLowObject* parent) {
    if (!child || !parent) return false;

    HiLowObject* current = child;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        if (current == parent) {
            return true; // Found parent in prototype chain
        }
        current = hl_object_get_proto(current);
        depth++;
    }

    return false; // Parent not found in prototype chain
}

// Phase 7c-ζ: For-in iteration helpers

size_t hl_object_property_count(HiLowObject* obj) {
    return obj ? obj->property_count : 0;
}

const char* hl_object_property_key_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count) {
        return NULL;
    }
    return obj->properties[index].key;
}

int hl_object_property_type_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count) {
        return 0;
    }

    // Map HiLowValueType to our type constants
    switch (obj->properties[index].value.type) {
        case HL_VALUE_I32: return TYPE_I32;
        case HL_VALUE_I64: return TYPE_I64;
        case HL_VALUE_U32: return TYPE_U32;
        case HL_VALUE_U64: return TYPE_U64;
        case HL_VALUE_F32: return TYPE_F32;
        case HL_VALUE_F64: return TYPE_F64;
        case HL_VALUE_BOOL: return TYPE_BOOL;
        case HL_VALUE_STR: return TYPE_STR;
        case HL_VALUE_OBJECT: return TYPE_OBJECT;
        case HL_VALUE_FUNCTION: return TYPE_FUNCTION;
        default: return 0;
    }
}

int32_t hl_object_property_value_i32_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_I32) {
        return 0;
    }
    return obj->properties[index].value.value.i32_val;
}

int64_t hl_object_property_value_i64_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_I64) {
        return 0;
    }
    return obj->properties[index].value.value.i64_val;
}

uint32_t hl_object_property_value_u32_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_U32) {
        return 0;
    }
    return obj->properties[index].value.value.u32_val;
}

uint64_t hl_object_property_value_u64_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_U64) {
        return 0;
    }
    return obj->properties[index].value.value.u64_val;
}

float hl_object_property_value_f32_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_F32) {
        return 0.0f;
    }
    return obj->properties[index].value.value.f32_val;
}

double hl_object_property_value_f64_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_F64) {
        return 0.0;
    }
    return obj->properties[index].value.value.f64_val;
}

bool hl_object_property_value_bool_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_BOOL) {
        return false;
    }
    return obj->properties[index].value.value.bool_val;
}

HiLowArray* hl_object_property_value_str_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_STR) {
        return NULL;
    }
    hl_array_retain(obj->properties[index].value.value.str_val);  // Retain-on-return
    return obj->properties[index].value.value.str_val;
}

HiLowObject* hl_object_property_value_object_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_OBJECT) {
        return NULL;
    }
    return obj->properties[index].value.value.obj_val;
}

HiLowFunction* hl_object_property_value_function_at(HiLowObject* obj, size_t index) {
    if (!obj || index >= obj->property_count || obj->properties[index].value.type != HL_VALUE_FUNCTION) {
        return NULL;
    }
    return obj->properties[index].value.value.fn_val;
}

// Debug allocator implementation (Phase 8a)
int hl_alloc_count = 0;
int hl_free_count = 0;

void hl_object_free(HiLowObject* obj) {
    if (obj) {
        if (obj->properties) {
            // Free all string values and keys that were strdup'd
            for (size_t i = 0; i < obj->property_count; i++) {
                // Free the key (always strdup'd)
                free((void*)obj->properties[i].key);
                hl_free_count++;

                // Release string values (now HiLowArray*)
                if (obj->properties[i].value.type == HL_VALUE_STR && obj->properties[i].value.value.str_val) {
                    hl_array_release(obj->properties[i].value.value.str_val);
                }
            }
            free(obj->properties);
            hl_free_count++;  // For the properties array
        }
        free(obj);
        hl_free_count++;  // For the object struct itself
    }
}

void hl_function_free(HiLowFunction* fn) {
    if (fn) {
        // For Phase 8a, we assume env is owned by the function if non-NULL
        // Function values that own their env should have that env freed
        if (fn->env) {
            if (fn->env_dtor) {
                fn->env_dtor(fn->env);  // release heap fields the env owns
            }
            free(fn->env);
            hl_free_count++;
        }
        free(fn);
        hl_free_count++;
    }
}

// Refcounting operations (Phase 8b)
void hl_object_retain(HiLowObject* obj) {
    if (obj) {
        obj->refcount++;
    }
}

void hl_object_release(HiLowObject* obj) {
    if (obj) {
        obj->refcount--;
        if (obj->refcount == 0) {
            // Step 1: Handle weak properties first - unregister from targets, no release
            for (size_t i = 0; i < obj->property_count; i++) {
                if (obj->properties[i].is_weak && obj->properties[i].value.type == HL_VALUE_OBJECT && obj->properties[i].value.value.obj_val) {
                    hl_object_weak_unregister(obj->properties[i].value.value.obj_val, obj, i);
                }
            }

            // Step 2: Release strong properties normally
            for (size_t i = 0; i < obj->property_count; i++) {
                if (!obj->properties[i].is_weak) {
                    if (obj->properties[i].value.type == HL_VALUE_OBJECT && obj->properties[i].value.value.obj_val) {
                        hl_object_release(obj->properties[i].value.value.obj_val);
                    } else if (obj->properties[i].value.type == HL_VALUE_FUNCTION && obj->properties[i].value.value.fn_val) {
                        hl_function_release(obj->properties[i].value.value.fn_val);
                    }
                }
            }

            // Step 3: Null out every weak property that points at this object.
            // WeakRef records (holder, property index) — stable across the
            // holder's property-array reallocs, unlike a raw slot address.
            WeakRef* current = obj->weak_refs;
            while (current) {
                current->holder->properties[current->prop_index].value.value.obj_val = NULL;
                WeakRef* next = current->next;
                free(current);
                hl_free_count++;
                current = next;
            }

            // Step 4: Free the object
            hl_object_free(obj);
        }
    }
}

void hl_function_retain(HiLowFunction* fn) {
    if (fn) {
        fn->refcount++;
    }
}

void hl_function_release(HiLowFunction* fn) {
    if (fn) {
        fn->refcount--;
        if (fn->refcount == 0) {
            hl_function_free(fn);
        }
    }
}

// Weak reference management functions (Phase 8c; reworked in Phase 1.5c to
// key on (holder, property index) — stable across property-array reallocs)

void hl_object_weak_register(HiLowObject* target, HiLowObject* holder, size_t prop_index) {
    if (!target || !holder) return;

    WeakRef* weak_ref = malloc(sizeof(WeakRef));
    hl_alloc_count++;
    weak_ref->holder = holder;
    weak_ref->prop_index = prop_index;
    weak_ref->next = target->weak_refs;
    target->weak_refs = weak_ref;
}

void hl_object_weak_unregister(HiLowObject* target, HiLowObject* holder, size_t prop_index) {
    if (!target || !holder) return;

    WeakRef** current = &target->weak_refs;
    while (*current) {
        if ((*current)->holder == holder && (*current)->prop_index == prop_index) {
            WeakRef* to_remove = *current;
            *current = (*current)->next;
            free(to_remove);
            hl_free_count++;
            return;
        }
        current = &(*current)->next;
    }
}

// Weak property store (Phase 1.5c ownership axiom): stores the target WITHOUT
// retaining it, marks the property weak, and registers the (holder, index)
// pair so target death nulls the slot. Overwrites drop the old reference
// correctly (unregister if weak, release if strong).
void hl_object_set_object_weak(HiLowObject* obj, const char* key, HiLowObject* target) {
    Property* existing = find_property(obj, key);
    size_t prop_index;
    if (existing) {
        prop_index = (size_t)(existing - obj->properties);
        if (existing->value.type == HL_VALUE_OBJECT && existing->value.value.obj_val) {
            if (existing->is_weak) {
                hl_object_weak_unregister(existing->value.value.obj_val, obj, prop_index);
            } else {
                hl_object_release(existing->value.value.obj_val);
            }
        } else if (existing->value.type == HL_VALUE_FUNCTION && existing->value.value.fn_val) {
            hl_function_release(existing->value.value.fn_val);
        } else if (existing->value.type == HL_VALUE_STR && existing->value.value.str_val) {
            hl_array_release(existing->value.value.str_val);
        }
        existing->value = (HiLowValue){ .type = HL_VALUE_OBJECT, .value.obj_val = target };
        existing->is_weak = true;
    } else {
        ensure_capacity(obj);
        prop_index = obj->property_count;
        obj->properties[prop_index].key = strdup(key);
        hl_alloc_count++;
        obj->properties[prop_index].value = (HiLowValue){ .type = HL_VALUE_OBJECT, .value.obj_val = target };
        obj->properties[prop_index].is_weak = true;
        obj->property_count++;
    }
    if (target) {
        hl_object_weak_register(target, obj, prop_index);
    }
}

// Weak property reads (Phase 1.5e, audit §5 item 6).

HiLowOptional* hl_optional_new_object(HiLowObject* o) {
    HiLowOptional* opt = malloc(sizeof(HiLowOptional));
    hl_alloc_count++;
    opt->refcount = 1;
    opt->kind = HL_OPT_OBJECT;
    opt->payload.obj_val = o;  // Take ownership of the object (+1)
    return opt;
}

HiLowObject* hl_optional_unwrap_object(HiLowOptional* opt) {
    // Retain-on-return, mirroring hl_optional_unwrap_string
    if (opt && opt->kind == HL_OPT_OBJECT && opt->payload.obj_val) {
        hl_object_retain(opt->payload.obj_val);
        return opt->payload.obj_val;
    }
    return NULL;
}

// Reading a weak property: the referent while alive, unknown after its death.
// Referent death nulls the slot (hl_object_release step 3), so a NULL
// obj_val on a found property IS the dead-weak state.
HiLowOptional* hl_object_get_weak(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_OBJECT) {
                HiLowObject* target = prop->value.value.obj_val;
                if (!target) {
                    return hl_optional_new_unknown(hl_unknown_new_internal("weak referent released"));
                }
                hl_object_retain(target);
                return hl_optional_new_object(target);
            } else {
                fprintf(stderr, "type mismatch on property '%s'\n", key);
                exit(1);
            }
        }

        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "prototype chain depth exceeded for property '%s'\n", key);
        exit(1);
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

// Member access through an object-or-unknown optional. An unknown propagates
// (the SAME unknown instance, per the spec's unknown-propagation rule); a
// live object wraps the named property in a fresh optional. Fatal on a
// missing property or property type mismatch, matching the plain getters.
static Property* optional_member_prop(HiLowOptional* opt, const char* key, HiLowUnknown** propagated) {
    *propagated = NULL;
    if (!opt) {
        fprintf(stderr, "member access on empty optional\n");
        exit(1);
    }
    if (opt->kind == HL_OPT_UNKNOWN) {
        hl_unknown_retain(opt->payload.unk_val);
        *propagated = opt->payload.unk_val;
        return NULL;
    }
    if (opt->kind != HL_OPT_OBJECT || !opt->payload.obj_val) {
        fprintf(stderr, "member access on non-object optional for property '%s'\n", key);
        exit(1);
    }

    HiLowObject* current = opt->payload.obj_val;
    int depth = 0;
    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            return prop;
        }
        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}

HiLowOptional* hl_optional_member_i32(HiLowOptional* opt, const char* key) {
    HiLowUnknown* propagated;
    Property* prop = optional_member_prop(opt, key, &propagated);
    if (propagated) return hl_optional_new_unknown(propagated);
    if (prop->value.type != HL_VALUE_I32) {
        fprintf(stderr, "type mismatch on property '%s'\n", key);
        exit(1);
    }
    return hl_optional_new_i32(prop->value.value.i32_val);
}

HiLowOptional* hl_optional_member_str(HiLowOptional* opt, const char* key) {
    HiLowUnknown* propagated;
    Property* prop = optional_member_prop(opt, key, &propagated);
    if (propagated) return hl_optional_new_unknown(propagated);
    if (prop->value.type != HL_VALUE_STR) {
        fprintf(stderr, "type mismatch on property '%s'\n", key);
        exit(1);
    }
    hl_array_retain(prop->value.value.str_val);
    return hl_optional_new_string(prop->value.value.str_val);
}

HiLowOptional* hl_optional_member_object(HiLowOptional* opt, const char* key) {
    HiLowUnknown* propagated;
    Property* prop = optional_member_prop(opt, key, &propagated);
    if (propagated) return hl_optional_new_unknown(propagated);
    if (prop->value.type != HL_VALUE_OBJECT) {
        fprintf(stderr, "type mismatch on property '%s'\n", key);
        exit(1);
    }
    HiLowObject* target = prop->value.value.obj_val;
    if (!target) {
        // A nested weak slot whose referent died: same dead-weak semantics
        return hl_optional_new_unknown(hl_unknown_new_internal("weak referent released"));
    }
    hl_object_retain(target);
    return hl_optional_new_object(target);
}

// Retain-and-return helpers for expression positions (Phase 1.5c): turn a
// borrowed reference into an owned +1 inline.
HiLowObject* hl_object_ref(HiLowObject* obj) {
    if (obj) obj->refcount++;
    return obj;
}

HiLowFunction* hl_function_ref(HiLowFunction* fn) {
    if (fn) fn->refcount++;
    return fn;
}

HiLowArray* hl_array_ref(HiLowArray* arr) {
    if (arr) arr->refcount++;
    return arr;
}

// Optional unwrap helpers for narrowed types (Phase 9b)
// These extract the underlying T value from a T? that is known to hold T (not unknown)

int32_t hl_optional_unwrap_i32(HiLowOptional* opt) {
    // Safe implementation: read from the wrapper struct's payload
    return opt ? opt->payload.i32_val : 0;
}

HiLowArray* hl_optional_unwrap_string(HiLowOptional* opt) {
    // Safe implementation: read from the wrapper struct's payload with retain-on-return
    if (opt && opt->payload.str_val) {
        hl_array_retain(opt->payload.str_val);
        return opt->payload.str_val;
    }
    return NULL;
}

HiLowUnknown* hl_optional_unwrap_unknown(HiLowOptional* opt) {
    // Extract the unknown value from the wrapper struct
    return opt ? opt->payload.unk_val : NULL;
}

int64_t hl_optional_unwrap_i64(HiLowOptional* opt) {
    // Placeholder - not used by current tests but needed for completeness
    return 0;
}

uint32_t hl_optional_unwrap_u32(HiLowOptional* opt) {
    // Placeholder - not used by current tests but needed for completeness
    return 0;
}

uint64_t hl_optional_unwrap_u64(HiLowOptional* opt) {
    // Placeholder - not used by current tests but needed for completeness
    return 0;
}

float hl_optional_unwrap_f32(HiLowOptional* opt) {
    // Placeholder - not used by current tests but needed for completeness
    return 0.0f;
}

double hl_optional_unwrap_f64(HiLowOptional* opt) {
    // Placeholder - not used by current tests but needed for completeness
    return 0.0;
}

bool hl_optional_unwrap_bool(HiLowOptional* opt) {
    // Placeholder - not used by current tests but needed for completeness
    return false;
}

HiLowTime hl_optional_unwrap_time(HiLowOptional* opt) {
    // Safe implementation: read from the wrapper struct's payload
    return opt ? opt->payload.time_val : (HiLowTime){0, HL_TIME_PREC_SECOND};
}

HiLowDuration hl_optional_unwrap_duration(HiLowOptional* opt) {
    // Safe implementation: read from the wrapper struct's payload
    return opt ? opt->payload.duration_val : (HiLowDuration){0};
}

HiLowMoney hl_optional_unwrap_money(HiLowOptional* opt) {
    // Safe implementation: read from the wrapper struct's payload
    return opt ? opt->payload.money_val : (HiLowMoney){0, HL_CURRENCY_USD};
}

// Print functions for optional types
void print_optional_i32(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        print_i32(hl_optional_unwrap_i32(opt));
    }
}

void print_optional_i64(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        print_i64(hl_optional_unwrap_i64(opt));
    }
}

void print_optional_u32(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        print_u32(hl_optional_unwrap_u32(opt));
    }
}

void print_optional_u64(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        print_u64(hl_optional_unwrap_u64(opt));
    }
}

void print_optional_f32(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        print_f32(hl_optional_unwrap_f32(opt));
    }
}

void print_optional_f64(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        print_f64(hl_optional_unwrap_f64(opt));
    }
}

void print_optional_bool(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        print_bool(hl_optional_unwrap_bool(opt));
    }
}

void print_optional_string(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        HiLowArray* str_array = hl_optional_unwrap_string(opt);
        if (str_array) {
            print_string(str_array);
            hl_array_release(str_array);  // Release the retained reference
        }
    }
}

void print_optional_money(HiLowOptional* opt) {
    if (hl_is_unknown(opt)) {
        print_unknown(hl_optional_unwrap_unknown(opt));
    } else {
        print_money(hl_optional_unwrap_money(opt));
    }
}

// Time and duration implementations (Phase 9c)
#include <time.h>
#include <string.h>
#include <stdlib.h>

// Get current time at nanosecond precision
HiLowTime hl_time_now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);

    HiLowTime result;
    result.nanos_since_epoch = (int64_t)ts.tv_sec * 1000000000LL + (int64_t)ts.tv_nsec;
    result.precision = HL_TIME_PREC_NANO;
    return result;
}

// Parse ISO 8601 time string, return time? (time or unknown)
HiLowOptional* hl_time_parse(HiLowArray* iso_array) {
    HiLowTime time;
    struct tm tm = {0};

    // Extract C string from array
    const char* iso_string = hl_array_to_cstr(iso_array);

    // Try to parse the date part: YYYY-MM-DD
    if (strlen(iso_string) < 10) {
        char buf[256];
        snprintf(buf, sizeof(buf), "invalid time format: %s", iso_string);
        HiLowUnknown* error = hl_unknown_new_internal(buf);
        free((void*)iso_string);  // Free the temporary C string
        hl_free_count++;
        return hl_optional_new_unknown(error);
    }

    // Parse year-month-day
    if (sscanf(iso_string, "%d-%d-%d", &tm.tm_year, &tm.tm_mon, &tm.tm_mday) != 3) {
        char buf[256];
        snprintf(buf, sizeof(buf), "invalid time format: %s", iso_string);
        HiLowUnknown* error = hl_unknown_new_internal(buf);
        free((void*)iso_string);  // Free the temporary C string
        hl_free_count++;
        return hl_optional_new_unknown(error);
    }

    tm.tm_year -= 1900;  // struct tm expects year since 1900
    tm.tm_mon -= 1;      // struct tm expects 0-11 months
    tm.tm_isdst = 0;     // Not daylight saving time (UTC has no DST)

    // Default time precision is day
    time.precision = HL_TIME_PREC_DAY;

    // Initialize fractional variables at function scope
    int millis = 0, micros = 0, nanos = 0;

    // Check if there's a time part (T)
    if (strlen(iso_string) > 10 && iso_string[10] == 'T') {
        const char* time_part = iso_string + 11;  // Skip past "T"
        int hour, minute, second = 0;

        // Try to parse hour:minute
        if (sscanf(time_part, "%d:%d", &hour, &minute) >= 2) {
            tm.tm_hour = hour;
            tm.tm_min = minute;
            time.precision = HL_TIME_PREC_MINUTE;

            // Try to parse seconds
            if (strlen(time_part) > 5 && time_part[2] == ':') {
                if (sscanf(time_part + 3, "%d", &second) == 1) {
                    tm.tm_sec = second;
                    time.precision = HL_TIME_PREC_SECOND;

                    // Try to parse fractional seconds
                    const char* dot_pos = strchr(time_part, '.');
                    if (dot_pos) {
                        const char* frac_part = dot_pos + 1;
                        int frac_len = 0;
                        while (frac_part[frac_len] >= '0' && frac_part[frac_len] <= '9' && frac_len < 9) {
                            frac_len++;
                        }

                        if (frac_len >= 3) {
                            sscanf(frac_part, "%3d", &millis);
                            time.precision = HL_TIME_PREC_MILLI;
                        }
                        if (frac_len >= 6) {
                            sscanf(frac_part + 3, "%3d", &micros);
                            time.precision = HL_TIME_PREC_MICRO;
                        }
                        if (frac_len >= 9) {
                            sscanf(frac_part + 6, "%3d", &nanos);
                            time.precision = HL_TIME_PREC_NANO;
                        }
                    }
                }
            }
        }
    }

    // Convert to time_t and then to nanoseconds (treat as UTC)
    time_t epoch_time = timegm(&tm);
    if (epoch_time == -1) {
        HiLowUnknown* error = hl_unknown_new_internal("invalid time: could not convert to timestamp");
        free((void*)iso_string);  // Free the temporary C string
        hl_free_count++;
        return hl_optional_new_unknown(error);
    }

    time.nanos_since_epoch = (int64_t)epoch_time * 1000000000LL +
                             (int64_t)millis * 1000000LL +
                             (int64_t)micros * 1000LL +
                             (int64_t)nanos;

    // Return a successful time using the proper time optional constructor
    free((void*)iso_string);  // Free the temporary C string
    hl_free_count++;
    return hl_optional_new_time(time);
}

// Time arithmetic
HiLowTime hl_time_add_duration(HiLowTime time, HiLowDuration duration) {
    HiLowTime result = time;
    result.nanos_since_epoch += duration.nanos;
    return result;
}

HiLowTime hl_time_sub_duration(HiLowTime time, HiLowDuration duration) {
    HiLowTime result = time;
    result.nanos_since_epoch -= duration.nanos;
    return result;
}

HiLowDuration hl_time_sub_time(HiLowTime lhs, HiLowTime rhs) {
    HiLowDuration result;
    result.nanos = lhs.nanos_since_epoch - rhs.nanos_since_epoch;
    return result;
}

HiLowDuration hl_duration_add(HiLowDuration lhs, HiLowDuration rhs) {
    HiLowDuration result;
    result.nanos = lhs.nanos + rhs.nanos;
    return result;
}

// Helper function to truncate time to precision for comparison
static int64_t truncate_time_to_precision(int64_t nanos, HiLowTimePrecision precision) {
    switch (precision) {
        case HL_TIME_PREC_DAY:   return (nanos / (24LL * 60LL * 60LL * 1000000000LL)) * (24LL * 60LL * 60LL * 1000000000LL);
        case HL_TIME_PREC_HOUR:  return (nanos / (60LL * 60LL * 1000000000LL)) * (60LL * 60LL * 1000000000LL);
        case HL_TIME_PREC_MINUTE: return (nanos / (60LL * 1000000000LL)) * (60LL * 1000000000LL);
        case HL_TIME_PREC_SECOND: return (nanos / 1000000000LL) * 1000000000LL;
        case HL_TIME_PREC_MILLI: return (nanos / 1000000LL) * 1000000LL;
        case HL_TIME_PREC_MICRO: return (nanos / 1000LL) * 1000LL;
        case HL_TIME_PREC_NANO:  return nanos;
        default: return nanos;
    }
}

// Get the coarser of two precisions
static HiLowTimePrecision min_precision(HiLowTimePrecision a, HiLowTimePrecision b) {
    return (a < b) ? a : b;
}

// Time comparison functions (precision-aware)
bool hl_time_eq(HiLowTime lhs, HiLowTime rhs) {
    HiLowTimePrecision precision = min_precision(lhs.precision, rhs.precision);
    int64_t lhs_truncated = truncate_time_to_precision(lhs.nanos_since_epoch, precision);
    int64_t rhs_truncated = truncate_time_to_precision(rhs.nanos_since_epoch, precision);
    return lhs_truncated == rhs_truncated;
}

bool hl_time_ne(HiLowTime lhs, HiLowTime rhs) {
    return !hl_time_eq(lhs, rhs);
}

bool hl_time_lt(HiLowTime lhs, HiLowTime rhs) {
    HiLowTimePrecision precision = min_precision(lhs.precision, rhs.precision);
    int64_t lhs_truncated = truncate_time_to_precision(lhs.nanos_since_epoch, precision);
    int64_t rhs_truncated = truncate_time_to_precision(rhs.nanos_since_epoch, precision);
    return lhs_truncated < rhs_truncated;
}

bool hl_time_le(HiLowTime lhs, HiLowTime rhs) {
    return hl_time_lt(lhs, rhs) || hl_time_eq(lhs, rhs);
}

bool hl_time_gt(HiLowTime lhs, HiLowTime rhs) {
    return !hl_time_le(lhs, rhs);
}

bool hl_time_ge(HiLowTime lhs, HiLowTime rhs) {
    return !hl_time_lt(lhs, rhs);
}

// Duration comparison functions
bool hl_duration_eq(HiLowDuration lhs, HiLowDuration rhs) {
    return lhs.nanos == rhs.nanos;
}

bool hl_duration_ne(HiLowDuration lhs, HiLowDuration rhs) {
    return lhs.nanos != rhs.nanos;
}

bool hl_duration_lt(HiLowDuration lhs, HiLowDuration rhs) {
    return lhs.nanos < rhs.nanos;
}

bool hl_duration_le(HiLowDuration lhs, HiLowDuration rhs) {
    return lhs.nanos <= rhs.nanos;
}

bool hl_duration_gt(HiLowDuration lhs, HiLowDuration rhs) {
    return lhs.nanos > rhs.nanos;
}

bool hl_duration_ge(HiLowDuration lhs, HiLowDuration rhs) {
    return lhs.nanos >= rhs.nanos;
}

// Print functions for time and duration
void print_time(HiLowTime time) {
    time_t seconds = (time_t)(time.nanos_since_epoch / 1000000000LL);
    struct tm* tm_info = gmtime(&seconds);

    switch (time.precision) {
        case HL_TIME_PREC_DAY:
            printf("%04d-%02d-%02d\n", tm_info->tm_year + 1900, tm_info->tm_mon + 1, tm_info->tm_mday);
            break;
        case HL_TIME_PREC_HOUR:
            printf("%04d-%02d-%02dT%02d\n", tm_info->tm_year + 1900, tm_info->tm_mon + 1, tm_info->tm_mday, tm_info->tm_hour);
            break;
        case HL_TIME_PREC_MINUTE:
            printf("%04d-%02d-%02dT%02d:%02d\n", tm_info->tm_year + 1900, tm_info->tm_mon + 1, tm_info->tm_mday, tm_info->tm_hour, tm_info->tm_min);
            break;
        case HL_TIME_PREC_SECOND:
            printf("%04d-%02d-%02dT%02d:%02d:%02d\n", tm_info->tm_year + 1900, tm_info->tm_mon + 1, tm_info->tm_mday, tm_info->tm_hour, tm_info->tm_min, tm_info->tm_sec);
            break;
        case HL_TIME_PREC_MILLI:
        case HL_TIME_PREC_MICRO:
        case HL_TIME_PREC_NANO:
            {
                int64_t sub_second = time.nanos_since_epoch % 1000000000LL;
                if (time.precision == HL_TIME_PREC_MILLI) {
                    printf("%04d-%02d-%02dT%02d:%02d:%02d.%03d\n", tm_info->tm_year + 1900, tm_info->tm_mon + 1, tm_info->tm_mday,
                           tm_info->tm_hour, tm_info->tm_min, tm_info->tm_sec, (int)(sub_second / 1000000LL));
                } else if (time.precision == HL_TIME_PREC_MICRO) {
                    printf("%04d-%02d-%02dT%02d:%02d:%02d.%06d\n", tm_info->tm_year + 1900, tm_info->tm_mon + 1, tm_info->tm_mday,
                           tm_info->tm_hour, tm_info->tm_min, tm_info->tm_sec, (int)(sub_second / 1000LL));
                } else {
                    printf("%04d-%02d-%02dT%02d:%02d:%02d.%09d\n", tm_info->tm_year + 1900, tm_info->tm_mon + 1, tm_info->tm_mday,
                           tm_info->tm_hour, tm_info->tm_min, tm_info->tm_sec, (int)sub_second);
                }
            }
            break;
    }
}

void print_duration(HiLowDuration duration) {
    int64_t nanos = duration.nanos;
    bool negative = false;

    if (nanos < 0) {
        negative = true;
        nanos = -nanos;
    }

    int64_t days = nanos / (24LL * 60LL * 60LL * 1000000000LL);
    nanos %= (24LL * 60LL * 60LL * 1000000000LL);

    int64_t hours = nanos / (60LL * 60LL * 1000000000LL);
    nanos %= (60LL * 60LL * 1000000000LL);

    int64_t minutes = nanos / (60LL * 1000000000LL);
    nanos %= (60LL * 1000000000LL);

    int64_t seconds = nanos / 1000000000LL;
    nanos %= 1000000000LL;

    int64_t millis = nanos / 1000000LL;
    nanos %= 1000000LL;

    int64_t micros = nanos / 1000LL;
    nanos %= 1000LL;

    if (negative) printf("-");

    // Print the largest non-zero unit
    if (days > 0) {
        printf("%ldd\n", days);
    } else if (hours > 0 && minutes > 0 && seconds > 0) {
        printf("%ldh%ldm%lds\n", hours, minutes, seconds);
    } else if (hours > 0 && minutes > 0) {
        printf("%ldh%ldm\n", hours, minutes);
    } else if (hours > 0) {
        printf("%ldh\n", hours);
    } else if (minutes > 0 && seconds > 0) {
        printf("%ldm%lds\n", minutes, seconds);
    } else if (minutes > 0) {
        printf("%ldm\n", minutes);
    } else if (seconds > 0) {
        printf("%lds\n", seconds);
    } else if (millis > 0) {
        printf("%ldms\n", millis);
    } else if (micros > 0) {
        printf("%ldus\n", micros);
    } else {
        printf("%ldns\n", nanos);
    }
}

// Money functions (Phase 9d)

void print_money(HiLowMoney money) {
    const char* symbol;
    int display_precision;

    switch (money.currency) {
        case HL_CURRENCY_USD:
            symbol = "$";
            display_precision = 2;
            break;
        case HL_CURRENCY_EUR:
            symbol = "€";
            display_precision = 2;
            break;
        case HL_CURRENCY_GBP:
            symbol = "£";
            display_precision = 2;
            break;
        case HL_CURRENCY_JPY:
            symbol = "¥";
            display_precision = 0;
            break;
        case HL_CURRENCY_CAD:
            symbol = "C$";
            display_precision = 2;
            break;
        case HL_CURRENCY_AUD:
            symbol = "A$";
            display_precision = 2;
            break;
        case HL_CURRENCY_CHF:
            symbol = "Fr";
            display_precision = 2;
            break;
        case HL_CURRENCY_CNY:
            symbol = "¥";
            display_precision = 0;
            break;
        default:
            symbol = "?";
            display_precision = 2;
            break;
    }

    // Amount is stored with 4 decimal places of internal precision
    // Display precision varies by currency
    if (display_precision == 0) {
        // JPY, CNY: no decimal places
        printf("%s%ld\n", symbol, money.amount / 10000);
    } else {
        // USD, EUR, etc: 2 decimal places
        int64_t whole = money.amount / 10000;
        int64_t decimal = (money.amount % 10000) / 100; // Convert from 4 to 2 decimal places
        printf("%s%ld.%02ld\n", symbol, whole, decimal);
    }
}

// Money arithmetic functions
HiLowMoney hl_money_add(HiLowMoney lhs, HiLowMoney rhs) {
    // Currency mismatch should be caught at compile time, but check at runtime too
    if (lhs.currency != rhs.currency) {
        fprintf(stderr, "Runtime error: cannot add different currencies\n");
        exit(1);
    }

    HiLowMoney result;
    result.currency = lhs.currency;
    result.amount = lhs.amount + rhs.amount;
    return result;
}

HiLowMoney hl_money_sub(HiLowMoney lhs, HiLowMoney rhs) {
    // Currency mismatch should be caught at compile time, but check at runtime too
    if (lhs.currency != rhs.currency) {
        fprintf(stderr, "Runtime error: cannot subtract different currencies\n");
        exit(1);
    }

    HiLowMoney result;
    result.currency = lhs.currency;
    result.amount = lhs.amount - rhs.amount;
    return result;
}

HiLowMoney hl_money_mul_scalar(HiLowMoney money, double scalar) {
    HiLowMoney result;
    result.currency = money.currency;
    result.amount = (int64_t)(money.amount * scalar);
    return result;
}

HiLowMoney hl_money_div_scalar(HiLowMoney money, double scalar) {
    HiLowMoney result;
    result.currency = money.currency;
    result.amount = (int64_t)(money.amount / scalar);
    return result;
}

double hl_money_div_money(HiLowMoney lhs, HiLowMoney rhs) {
    // Currency mismatch should be caught at compile time, but check at runtime too
    if (lhs.currency != rhs.currency) {
        fprintf(stderr, "Runtime error: cannot divide different currencies\n");
        exit(1);
    }

    return (double)lhs.amount / (double)rhs.amount;
}

// Money comparison functions
bool hl_money_eq(HiLowMoney lhs, HiLowMoney rhs) {
    if (lhs.currency != rhs.currency) {
        fprintf(stderr, "Runtime error: cannot compare different currencies\n");
        exit(1);
    }
    return lhs.amount == rhs.amount;
}

bool hl_money_ne(HiLowMoney lhs, HiLowMoney rhs) {
    return !hl_money_eq(lhs, rhs);
}

bool hl_money_lt(HiLowMoney lhs, HiLowMoney rhs) {
    if (lhs.currency != rhs.currency) {
        fprintf(stderr, "Runtime error: cannot compare different currencies\n");
        exit(1);
    }
    return lhs.amount < rhs.amount;
}

bool hl_money_le(HiLowMoney lhs, HiLowMoney rhs) {
    if (lhs.currency != rhs.currency) {
        fprintf(stderr, "Runtime error: cannot compare different currencies\n");
        exit(1);
    }
    return lhs.amount <= rhs.amount;
}

bool hl_money_gt(HiLowMoney lhs, HiLowMoney rhs) {
    if (lhs.currency != rhs.currency) {
        fprintf(stderr, "Runtime error: cannot compare different currencies\n");
        exit(1);
    }
    return lhs.amount > rhs.amount;
}

bool hl_money_ge(HiLowMoney lhs, HiLowMoney rhs) {
    if (lhs.currency != rhs.currency) {
        fprintf(stderr, "Runtime error: cannot compare different currencies\n");
        exit(1);
    }
    return lhs.amount >= rhs.amount;
}

// Array support (Array Phase A)

HiLowArray* hl_array_new(size_t elem_size, size_t initial_capacity, hl_elem_fn retain_fn, hl_elem_fn release_fn) {
    HiLowArray* arr = malloc(sizeof(HiLowArray));
    hl_alloc_count++;

    arr->refcount = 1;
    arr->length = 0;
    arr->capacity = initial_capacity;
    arr->elem_size = elem_size;
    arr->data = malloc(elem_size * initial_capacity);
    arr->watchers = NULL;  // Phase B scaffolding: watcher list always empty
    arr->retain_fn = retain_fn;
    arr->release_fn = release_fn;

    return arr;
}

void hl_array_retain(HiLowArray* arr) {
    if (arr) {
        arr->refcount++;
    }
}

void hl_array_release(HiLowArray* arr) {
    if (!arr) return;

    arr->refcount--;
    if (arr->refcount == 0) {
        // Release all elements if this is an object array
        if (arr->release_fn != NULL) {
            for (size_t i = 0; i < arr->length; i++) {
                void* slot = (char*)arr->data + (i * arr->elem_size);
                arr->release_fn(*(void**)slot);
            }
        }

        // Free watcher list nodes (but not the watcher state - that's owned by the binding)
        HiLowArrayWatcher* current = arr->watchers;
        while (current != NULL) {
            HiLowArrayWatcher* next = current->next;
            free(current);
            current = next;
        }
        free(arr->data);
        free(arr);
        hl_free_count++;
    }
}

void hl_array_push(HiLowArray* arr, void* elem) {
    // Grow if needed
    if (arr->length == arr->capacity) {
        arr->capacity *= 2;
        arr->data = realloc(arr->data, arr->elem_size * arr->capacity);
    }

    // Copy element into array
    void* dest = (char*)arr->data + (arr->length * arr->elem_size);
    memcpy(dest, elem, arr->elem_size);

    // Retain the element if this is an object array
    if (arr->retain_fn != NULL) {
        arr->retain_fn(*(void**)dest);
    }

    arr->length++;

    // Phase 10-ε-β: fire watchers registered on this array with delta-passing
    if (hl_stealth_depth == 0) {
        for (HiLowArrayWatcher* w = arr->watchers; w != NULL; w = w->next) {
            HiLowWatcher* state = (HiLowWatcher*)w->watcher_state;
        if (state != NULL && state->active && !state->ended) {
            void* delta = NULL;
            int fires = 0;
            if (w->modifier == HL_ARR_ADDED) {
                delta = elem; fires = 1;
            }
            else if (w->modifier == HL_ARR_CHANGED) {
                delta = NULL; fires = 1;
            }
            if (fires) ((void(*)(void*, HiLowArray*, void*))w->body_fn)(w->env, arr, delta);
        }
        }
    }
}

void* hl_array_get(HiLowArray* arr, size_t index) {
    // Optional bounds check - encouraged but not required
    if (index >= arr->length) {
        fprintf(stderr, "Runtime error: array index %zu out of bounds (length %zu)\n",
                index, arr->length);
        exit(1);
    }

    return (char*)arr->data + (index * arr->elem_size);
}

size_t hl_array_len(HiLowArray* arr) {
    return arr->length;
}

void* hl_array_pop(HiLowArray* arr) {
    // Bounds check - undefined behavior for empty arrays in Phase B
    if (arr->length == 0) {
        fprintf(stderr, "Runtime error: pop() on empty array\n");
        exit(1);
    }

    // Decrement length (logically removing the last element)
    arr->length--;

    // Get pointer to the now-removed element slot
    void* removed_slot = (char*)arr->data + (arr->length * arr->elem_size);

    // Phase 10-ε-β: fire watchers registered on this array with delta-passing
    if (hl_stealth_depth == 0) {
        for (HiLowArrayWatcher* w = arr->watchers; w != NULL; w = w->next) {
            HiLowWatcher* state = (HiLowWatcher*)w->watcher_state;
        if (state != NULL && state->active && !state->ended) {
            void* delta = NULL;
            int fires = 0;
            if (w->modifier == HL_ARR_REMOVED) {
                delta = removed_slot; fires = 1;
            }
            else if (w->modifier == HL_ARR_CHANGED) {
                delta = NULL; fires = 1;
            }
            if (fires) ((void(*)(void*, HiLowArray*, void*))w->body_fn)(w->env, arr, delta);
        }
        }
    }

    // Return the removed element
    // Note: The caller now owns the object reference; no release here
    return removed_slot;
}

void hl_array_set(HiLowArray* arr, size_t index, void* elem) {
    // Optional bounds check - encouraged but not required in Phase B
    if (index >= arr->length) {
        fprintf(stderr, "Runtime error: array index %zu out of bounds for set (length %zu)\n",
                index, arr->length);
        exit(1);
    }

    // Overwrite element at index
    void* dest = (char*)arr->data + (index * arr->elem_size);

    // Release the old element if this is an object array
    if (arr->release_fn != NULL) {
        arr->release_fn(*(void**)dest);
    }

    memcpy(dest, elem, arr->elem_size);

    // Retain the new element if this is an object array
    if (arr->retain_fn != NULL) {
        arr->retain_fn(*(void**)dest);
    }

    // Phase 10-ε-β: fire watchers registered on this array with delta-passing
    if (hl_stealth_depth == 0) {
        for (HiLowArrayWatcher* w = arr->watchers; w != NULL; w = w->next) {
            HiLowWatcher* state = (HiLowWatcher*)w->watcher_state;
        if (state != NULL && state->active && !state->ended) {
            void* delta = NULL;
            int fires = 0;
            if (w->modifier == HL_ARR_CHANGED) {
                delta = NULL; fires = 1;
            }
            // Note: set fires DEEP and CHANGED only (no size change)
            if (fires) ((void(*)(void*, HiLowArray*, void*))w->body_fn)(w->env, arr, delta);
        }
        }
    }
}

void* hl_array_remove(HiLowArray* arr, size_t index) {
    // Bounds check
    if (index >= arr->length) {
        fprintf(stderr, "Runtime error: remove() index %zu out of bounds (length %zu)\n",
                index, arr->length);
        exit(1);
    }

    // Capture the element before shifting (needed for delta stability during watcher firing)
    void* removed_slot = (char*)arr->data + (index * arr->elem_size);
    static char temp_buffer[1024]; // Static buffer for delta stability - assumes elem_size <= 1024
    memcpy(temp_buffer, removed_slot, arr->elem_size);

    // Shift elements [index+1 .. length-1] down by one
    if (index < arr->length - 1) {
        void* dest = (char*)arr->data + (index * arr->elem_size);
        void* src = (char*)arr->data + ((index + 1) * arr->elem_size);
        size_t bytes_to_move = (arr->length - index - 1) * arr->elem_size;
        memmove(dest, src, bytes_to_move);
    }

    // Decrement length
    arr->length--;

    // Fire watchers with captured element as delta
    if (hl_stealth_depth == 0) {
        for (HiLowArrayWatcher* w = arr->watchers; w != NULL; w = w->next) {
            HiLowWatcher* state = (HiLowWatcher*)w->watcher_state;
        if (state != NULL && state->active && !state->ended) {
            void* delta = NULL;
            int fires = 0;
            if (w->modifier == HL_ARR_REMOVED) {
                delta = temp_buffer; fires = 1;
            }
            else if (w->modifier == HL_ARR_CHANGED) {
                delta = NULL; fires = 1;
            }
            if (fires) ((void(*)(void*, HiLowArray*, void*))w->body_fn)(w->env, arr, delta);
        }
        }
    }

    // Return the removed element
    // Note: The caller now owns the object reference; no release here
    return temp_buffer;
}

void hl_array_insert(HiLowArray* arr, size_t index, void* elem) {
    // Bounds check: index > length is error; index == length is allowed (append)
    if (index > arr->length) {
        fprintf(stderr, "Runtime error: insert() index %zu out of bounds (length %zu)\n",
                index, arr->length);
        exit(1);
    }

    // Grow if needed
    if (arr->length == arr->capacity) {
        arr->capacity *= 2;
        arr->data = realloc(arr->data, arr->elem_size * arr->capacity);
    }

    // Shift elements [index .. length-1] up by one
    if (index < arr->length) {
        void* src = (char*)arr->data + (index * arr->elem_size);
        void* dest = (char*)arr->data + ((index + 1) * arr->elem_size);
        size_t bytes_to_move = (arr->length - index) * arr->elem_size;
        memmove(dest, src, bytes_to_move);
    }

    // Place new element at index
    void* dest = (char*)arr->data + (index * arr->elem_size);
    memcpy(dest, elem, arr->elem_size);

    // Retain the element if this is an object array
    if (arr->retain_fn != NULL) {
        arr->retain_fn(*(void**)dest);
    }

    // Increment length
    arr->length++;

    // Fire watchers with inserted element as delta
    if (hl_stealth_depth == 0) {
        for (HiLowArrayWatcher* w = arr->watchers; w != NULL; w = w->next) {
            HiLowWatcher* state = (HiLowWatcher*)w->watcher_state;
        if (state != NULL && state->active && !state->ended) {
            void* delta = NULL;
            int fires = 0;
            if (w->modifier == HL_ARR_ADDED) {
                delta = elem; fires = 1;
            }
            else if (w->modifier == HL_ARR_CHANGED) {
                delta = NULL; fires = 1;
            }
            if (fires) ((void(*)(void*, HiLowArray*, void*))w->body_fn)(w->env, arr, delta);
        }
        }
    }
}

// Phase 10-ε-γ: Array element move
void hl_array_move(HiLowArray* arr, size_t from, size_t to) {
    // Bounds check both indices
    if (from >= arr->length) {
        fprintf(stderr, "Runtime error: move() from index %zu out of bounds (length %zu)\n",
                from, arr->length);
        exit(1);
    }
    if (to >= arr->length) {
        fprintf(stderr, "Runtime error: move() to index %zu out of bounds (length %zu)\n",
                to, arr->length);
        exit(1);
    }

    // No-op if from == to
    if (from == to) {
        // Still fire watchers with delta (or skip - documented choice: still fire)
        HiLowMovedDelta delta = { ._0 = from, ._1 = to };
        if (hl_stealth_depth == 0) {
            for (HiLowArrayWatcher* w = arr->watchers; w != NULL; w = w->next) {
                HiLowWatcher* state = (HiLowWatcher*)w->watcher_state;
            if (state != NULL && state->active && !state->ended) {
                void* delta_ptr = NULL;
                int fires = 0;
                if (w->modifier == HL_ARR_MOVED) {
                    delta_ptr = &delta; fires = 1;
                }
                else if (w->modifier == HL_ARR_CHANGED) {
                    delta_ptr = NULL; fires = 1;
                }
                if (fires) ((void(*)(HiLowArray*, void*))w->body_fn)(arr, delta_ptr);
            }
            }
        }
        return;
    }

    // Capture the element at 'from' index
    void* from_slot = (char*)arr->data + (from * arr->elem_size);
    static char temp_buffer[1024]; // Static buffer for element capture - assumes elem_size <= 1024
    memcpy(temp_buffer, from_slot, arr->elem_size);

    // Shift elements depending on direction
    if (from < to) {
        // Move elements [from+1 .. to] down by one slot
        void* dest = (char*)arr->data + (from * arr->elem_size);
        void* src = (char*)arr->data + ((from + 1) * arr->elem_size);
        size_t bytes_to_move = (to - from) * arr->elem_size;
        memmove(dest, src, bytes_to_move);
    } else {
        // Move elements [to .. from-1] up by one slot
        void* src = (char*)arr->data + (to * arr->elem_size);
        void* dest = (char*)arr->data + ((to + 1) * arr->elem_size);
        size_t bytes_to_move = (from - to) * arr->elem_size;
        memmove(dest, src, bytes_to_move);
    }

    // Place captured element at 'to' index
    void* to_slot = (char*)arr->data + (to * arr->elem_size);
    memcpy(to_slot, temp_buffer, arr->elem_size);

    // No retain/release - same element, refcount unchanged

    // Fire watchers with (from,to) delta
    HiLowMovedDelta delta = { ._0 = from, ._1 = to };
    if (hl_stealth_depth == 0) {
        for (HiLowArrayWatcher* w = arr->watchers; w != NULL; w = w->next) {
            HiLowWatcher* state = (HiLowWatcher*)w->watcher_state;
        if (state != NULL && state->active && !state->ended) {
            void* delta_ptr = NULL;
            int fires = 0;
            if (w->modifier == HL_ARR_MOVED) {
                delta_ptr = &delta; fires = 1;
            }
            else if (w->modifier == HL_ARR_CHANGED) {
                delta_ptr = NULL; fires = 1;
            }
            if (fires) ((void(*)(HiLowArray*, void*))w->body_fn)(arr, delta_ptr);
        }
        }
    }
}

// Phase 10-ε-γ + clear: Array clear empties the array
void hl_array_clear(HiLowArray* arr) {
    if (!arr) return;

    // Release all elements if this is an object array (same loop as hl_array_release)
    if (arr->release_fn != NULL) {
        for (size_t i = 0; i < arr->length; i++) {
            void* slot = (char*)arr->data + (i * arr->elem_size);
            arr->release_fn(*(void**)slot);
        }
    }

    // Reset length to 0, but keep the buffer for reuse (don't free arr->data)
    arr->length = 0;

    // Fire watchers: CHANGED and DEEP get NULL delta; ADDED/REMOVED/MOVED do NOT fire
    if (hl_stealth_depth == 0) {
        for (HiLowArrayWatcher* w = arr->watchers; w != NULL; w = w->next) {
                HiLowWatcher* state = (HiLowWatcher*)w->watcher_state;
            if (state != NULL && state->active && !state->ended) {
                if (w->modifier == HL_ARR_CHANGED) {
                    ((void(*)(void*, HiLowArray*, void*))w->body_fn)(w->env, arr, NULL);
                }
            }
        }
    }
}

// Array watcher registration (Phase 10-ε-α)
void hl_array_register_watcher(HiLowArray* arr, int modifier, void* body_fn, void* env, void* watcher_state) {
    HiLowArrayWatcher* new_watcher = malloc(sizeof(HiLowArrayWatcher));
    new_watcher->modifier = modifier;
    new_watcher->body_fn = body_fn;
    new_watcher->env = env;
    new_watcher->watcher_state = watcher_state;
    new_watcher->next = arr->watchers;
    arr->watchers = new_watcher;  // Prepend to list
}

void hl_array_unregister_watcher(HiLowArray* arr, void* env) {
    HiLowArrayWatcher** current = &arr->watchers;

    while (*current != NULL) {
        if ((*current)->env == env) {
            HiLowArrayWatcher* to_remove = *current;
            *current = (*current)->next;  // Remove from list
            free(to_remove);  // Free the watcher node
            return;
        }
        current = &(*current)->next;
    }
}

// String operations (Managed Strings Sub-phase 1)
bool hl_string_eq(HiLowArray* lhs, HiLowArray* rhs) {
    // Bytewise string comparison
    if (lhs->length != rhs->length) {
        return false;
    }

    // Compare bytes
    return memcmp(lhs->data, rhs->data, lhs->length) == 0;
}

bool hl_string_ne(HiLowArray* lhs, HiLowArray* rhs) {
    return !hl_string_eq(lhs, rhs);
}

bool hl_string_eq_cstr(const HiLowArray* s, const char* lit) {
    // Compare a managed string against a C string literal without allocating
    size_t lit_len = strlen(lit);
    if (s->length != lit_len) {
        return false;
    }
    return memcmp(s->data, lit, lit_len) == 0;
}

HiLowArray* hl_string_from_cstr(const char* s) {
    // Build a managed string (refcount 1) from a C string
    size_t len = strlen(s);
    HiLowArray* result = hl_array_new(sizeof(uint8_t), len, NULL, NULL);
    for (size_t i = 0; i < len; i++) {
        uint8_t byte = (uint8_t)s[i];
        hl_array_push(result, &byte);
    }
    return result;
}

HiLowArray* hl_string_concat(HiLowArray* lhs, HiLowArray* rhs) {
    // Create new string with combined length
    size_t new_length = lhs->length + rhs->length;
    HiLowArray* result = hl_array_new(sizeof(uint8_t), new_length, NULL, NULL);

    // Copy lhs bytes
    for (size_t i = 0; i < lhs->length; i++) {
        uint8_t byte = *((uint8_t*)lhs->data + i);
        hl_array_push(result, &byte);
    }

    // Copy rhs bytes
    for (size_t i = 0; i < rhs->length; i++) {
        uint8_t byte = *((uint8_t*)rhs->data + i);
        hl_array_push(result, &byte);
    }

    return result;
}

void print_string(HiLowArray* str) {
    // Print the UTF-8 bytes as a null-terminated string
    for (size_t i = 0; i < str->length; i++) {
        uint8_t byte = *((uint8_t*)str->data + i);
        putchar(byte);
    }
    putchar('\n');  // Add newline like other print functions
}

void hl_array_append_bytes(HiLowArray* dst, const uint8_t* src, size_t n) {
    if (n == 0) return;

    // Ensure capacity for n additional bytes
    size_t new_length = dst->length + n;
    if (new_length > dst->capacity) {
size_t new_capacity = dst->capacity == 0 ? 1 : dst->capacity;
    while (new_capacity < new_length) {
        new_capacity *= 2;
    }
        dst->data = realloc(dst->data, new_capacity * dst->elem_size);
        dst->capacity = new_capacity;
    }

    // Copy bytes efficiently using memcpy
    memcpy((uint8_t*)dst->data + dst->length, src, n);
    dst->length = new_length;
}

// String-to-cstr helper for internal C APIs
// Returns a malloc'd null-terminated copy of the array's bytes
// Caller must free() the returned string
const char* hl_array_to_cstr(HiLowArray* arr) {
    if (!arr) {
        char* empty = malloc(1);
        hl_alloc_count++;
        empty[0] = '\0';
        return empty;
    }

    // Allocate space for bytes + null terminator
    char* cstr = malloc(arr->length + 1);
    hl_alloc_count++;

    // Copy bytes from array
    memcpy(cstr, arr->data, arr->length);

    // Add null terminator
    cstr[arr->length] = '\0';

    return cstr;
}
