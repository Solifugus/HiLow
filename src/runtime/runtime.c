#define _GNU_SOURCE  // For timegm
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>      // Phase 6a: malloc/free/exit used by shm code below line 2640's includes
#include <string.h>      // Phase 6a: memcpy/strlen for segment-name construction
#include <pthread.h>     // Phase 5a: per-thread inbox mutex + thread-id
#include <stdatomic.h>   // Phase 5a: inbox-nonempty flag (racy safe-point read)
#include <stdint.h>      // Phase 6a: fixed-width header fields
#include <errno.h>       // Phase 6a: EEXIST discrimination on shm_open
#include <fcntl.h>       // Phase 6a: O_CREAT/O_EXCL/O_RDWR for shm_open
#include <sys/mman.h>    // Phase 6a: shm_open/mmap/munmap
#include <sys/stat.h>    // Phase 6a: fstat (attacher segment size), mode bits
#include <unistd.h>      // Phase 6a: ftruncate/close, nanosleep companion
#include <time.h>        // Phase 6a: nanosleep for the bounded init-wait backoff
#include "runtime.h"

// Phase 6a: forward decl — the placed-cell header teardown (hl_cell_release,
// below) detaches the segment; the definition lives with the shm section next
// to the scalar constructors. Only the opaque pointer type is needed here.
static void hl_shm_detach(HiLowShmSegment* seg);

// Phase 5b: threaded runtime mode. When the compiled program uses `async`
// (or, later, `shared`), the compiler defines HILOW_THREADED for BOTH this
// runtime and the generated main.c (one `cc` invocation, one -D). In threaded
// mode every reference-count inc/dec is a single atomic RMW so a value shared
// across the async thread and its declaring thread is refcounted without a
// data race (memory safety carried by atomics, not scheduling luck — brief
// §5b). Single-threaded programs get no -D, so these expand to the exact plain
// `++`/`--` the runtime always used and behavior is unchanged; the fields stay
// plain `int` (the pointer-cast-to-_Atomic idiom, lock-free for int on the
// supported targets). HL_RC_DEC returns the NEW (post-decrement) value, matching
// the `x--; if (x <= 0)` shape it replaces.
#ifdef HILOW_THREADED
  #define HL_RC_INC(field) \
      ((void)atomic_fetch_add_explicit((_Atomic int*)&(field), 1, memory_order_relaxed))
  #define HL_RC_DEC(field) \
      (atomic_fetch_sub_explicit((_Atomic int*)&(field), 1, memory_order_acq_rel) - 1)
#else
  #define HL_RC_INC(field) ((void)((field)++))
  #define HL_RC_DEC(field) (--(field))
#endif

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
    hl_thread_safepoint();  // Phase 5a: syscall/output safe point (dormant single-threaded)
    printf("%s\n", value);
}

// Nothing type support (Phase 9a)
HiLowNothing the_nothing = { 42 }; // Global singleton

void print_nothing(void) {
    printf("nothing\n");
}

// Phase 10a-stealth: watcher suppression depth.
// Phase 5a: thread-local. Stealth is producer-side — a write made during a
// stealth block on one thread suppresses notification everywhere (same-thread
// fire and cross-thread enqueue), so the depth is per-mutating-thread. Stays a
// named global symbol (not a context-struct field) because generated code emits
// `hl_stealth_depth++/--` directly (codegen byte-identical).
_Thread_local int hl_stealth_depth = 0;

// Phase 2d: deep-walk epoch. Each parent walk takes a fresh epoch and stamps
// visited cells' version fields — diamonds (and any future cycles) collapse
// to one visit per cell per walk.
// Phase 5a: thread-local. Cross-thread visibility flows only through `shared`
// values (5c), so deep walks over non-shared structure are declaring-thread-only
// by construction; each thread owns its epoch counter.
static _Thread_local uint64_t hl_deep_epoch = 0;

// Phase 2d containment-bookkeeping helpers (defined with the array mutators).
static void array_element_stored(HiLowArray* arr, void* slot);
static void array_element_removed(HiLowArray* arr, void* slot);

// Phase 2e: object-side containment bookkeeping and the audience check are
// needed by set_property, which precedes their definitions.
static bool cell_has_audience(const HiLowCell* c);
static void object_property_stored(HiLowObject* holder, const HiLowValue* v);
static void object_property_removed(HiLowObject* holder, const HiLowValue* v);

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
        HL_RC_INC(unknown->refcount);
    }
}

void hl_unknown_release(HiLowUnknown* unknown) {
    if (unknown) {
        if (HL_RC_DEC(unknown->refcount) <= 0) {
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
        HL_RC_INC(opt->refcount);
    }
}

void hl_optional_release(HiLowOptional* opt) {
    if (opt) {
        if (HL_RC_DEC(opt->refcount) <= 0) {
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
    hl_thread_safepoint();  // Phase 5a: allocation safe point (dormant single-threaded)
    HiLowObject* obj = malloc(sizeof(HiLowObject));
    hl_alloc_count++;

    // Cell header (Phase 2e — mirrors hl_array_new)
    obj->cell.refcount = 1;
    obj->cell.kind = HL_CELL_OBJECT;  // Phase 5c: typed-teardown dispatch
    obj->cell.sub_lock = NULL;        // Phase 5c: not shared
    obj->cell.shm = NULL;             // Phase 6a: not placed (objects are never placeable)
    obj->cell.watchers = NULL;
    obj->cell.parents = NULL;
    obj->cell.version = 0;
    obj->cell.deep_watched = false;

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

// Helper function to add or update a property.
// Phase 2e: this is the single strong-store choke point, so it also carries
// the containment bookkeeping (parent backrefs for object/array values) and
// the object event mapping: overwriting an existing key fires CHANGED, a new
// key fires ADDED (currently unreachable from the surface — dynamic property
// addition is rejected at compile time — but the mapping is complete for the
// day it lands). No REMOVED: property removal is unimplemented (tombstone
// ruling). Deltas are NULL — object subscriptions carry no aliases.
static void set_property(HiLowObject* obj, const char* key, HiLowValue value) {
    Property* existing = find_property(obj, key);
    bool existed = (existing != NULL);
    if (existing) {
        // Phase 2e: an old strong container value loses its containment
        // backref before it is released (mirrors the 2d remove-before-release
        // ordering in the array mutators)
        if (!existing->is_weak) {
            object_property_removed(obj, &existing->value);
        }
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
        } else if (existing->value.type == HL_VALUE_ARRAY && existing->value.value.arr_val) {
            hl_array_release(existing->value.value.arr_val);
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
    } else if (value.type == HL_VALUE_ARRAY && value.value.arr_val) {
        hl_array_retain(value.value.arr_val);
    }

    // Phase 2e: a strong container store links the child back to this holder
    // (and pulls it into an already-deep-watched subtree)
    object_property_stored(obj, &value);

    // Phase 2e firing (same guard shape as the 2d array mutators; the guard
    // only skips work — hl_cell_notify remains the authoritative gate)
    if (hl_stealth_depth == 0 && cell_has_audience(&obj->cell)) {
        hl_cell_notify(&obj->cell, existed ? HL_ARR_CHANGED : HL_ARR_ADDED, NULL);
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

// Phase 2e: array-valued properties. set_property retains on store and
// handles containment + firing like every other strong store.
void hl_object_set_array(HiLowObject* obj, const char* key, HiLowArray* value) {
    HiLowValue val = { .type = HL_VALUE_ARRAY, .value.arr_val = value };
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

// Phase 2e: borrow, mirroring hl_object_get_object (the property keeps its
// own strong reference; callers that need to keep the array retain it).
HiLowArray* hl_object_get_array(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_ARRAY) {
                return prop->value.value.arr_val;
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

// ---------------------------------------------------------------------------
// Cell operations (Phase 2a). All take HiLowCell*; nothing array-specific.
// Lifetime rule: a cell→watcher node and its watcher→cell backref are BOTH
// non-owning; whichever side dies first unlinks itself from the other.
// ---------------------------------------------------------------------------

void hl_cell_retain(HiLowCell* c) {
    if (c) {
        HL_RC_INC(c->refcount);
    }
}

// Remove one backref to `cell` from `w->subs` (used when the cell dies first;
// duplicates — one backref per subscription node — are removed one at a time).
static void watcher_drop_backref(HiLowWatcher* w, HiLowCell* cell) {
    if (!w) return;
    HiLowWatcherSub** cur = &w->subs;
    while (*cur) {
        if ((*cur)->cell == cell) {
            HiLowWatcherSub* dead = *cur;
            *cur = dead->next;
            free(dead);
            return;
        }
        cur = &(*cur)->next;
    }
}

bool hl_cell_release(HiLowCell* c) {
    if (!c) return false;
    if (HL_RC_DEC(c->refcount) > 0) return false;

    // Subscription-list teardown: unlink each node's backref from its
    // watcher (the watcher value itself is owned by its binding, not us).
    HiLowCellWatcher* node = c->watchers;
    while (node) {
        HiLowCellWatcher* next = node->next;
        watcher_drop_backref(node->watcher, c);
        free(node);
        node = next;
    }
    c->watchers = NULL;

    // Parent list (dead until 2d, but tear down for shape-completeness).
    HiLowCellParent* p = c->parents;
    while (p) {
        HiLowCellParent* next = p->next;
        free(p);
        p = next;
    }
    c->parents = NULL;

    // Phase 5c: a shared cell owns its subscriber mutex — destroy and free it.
    if (c->sub_lock) {
        pthread_mutex_destroy(c->sub_lock);
        free(c->sub_lock);
        c->sub_lock = NULL;
    }

    // Phase 6a: a placed cell owns its mapped segment — detach (munmap + close).
    // No unlink: segments are persistent by default (they outlive any single
    // process — the separately-launched-programs model). hl_shm_detach is
    // declared just above the shm section; forward-declared here for teardown.
    if (c->shm) {
        hl_shm_detach(c->shm);
        c->shm = NULL;
    }

    return true;  // caller does its type-specific teardown and free
}

// Phase 5c (deviation 5a-ii graduation): the per-type teardown that each typed
// release runs after hl_cell_release returns true, factored out so
// hl_cell_release_full can invoke it by kind. Defined with their releases below.
static void hl_scalar_finalize(HiLowScalar* s);
static void hl_array_finalize(HiLowArray* arr);
static void hl_object_finalize(HiLowObject* obj);

bool hl_cell_release_full(HiLowCell* c) {
    if (!c) return false;
    if (!hl_cell_release(c)) return false;  // refcount + header teardown; false unless 0
    // Last reference gone: do the full TYPED teardown here, from whatever thread
    // landed the final release (the inbox may be the sole owner at drain — 5a-ii).
    switch (c->kind) {
        case HL_CELL_SCALAR: hl_scalar_finalize((HiLowScalar*)c); break;
        case HL_CELL_ARRAY:  hl_array_finalize((HiLowArray*)c); break;
        case HL_CELL_OBJECT: hl_object_finalize((HiLowObject*)c); break;
        default: break;  // untagged: header-only teardown already done (unreachable)
    }
    return true;
}

// Phase 5c: guard the subscriber list of a SHARED cell against concurrent
// add/remove (declaring thread) vs notify snapshot (producer threads). NULL
// sub_lock ⟺ non-shared → these are no-ops and the fast path is unchanged.
static inline void hl_cell_sub_lock(HiLowCell* c) {
    if (c->sub_lock) pthread_mutex_lock(c->sub_lock);
}
static inline void hl_cell_sub_unlock(HiLowCell* c) {
    if (c->sub_lock) pthread_mutex_unlock(c->sub_lock);
}

void hl_cell_subscribe(HiLowCell* c, int modifier, void* body_fn, void* env, HiLowWatcher* w, HiLowCell* origin) {
    HiLowCellWatcher* node = malloc(sizeof(HiLowCellWatcher));
    node->modifier = modifier;
    node->body_fn = body_fn;
    node->env = env;
    node->watcher = w;
    node->origin = origin;
    // Phase 3b: APPEND — subscribers fire in subscription order (the legacy
    // firing block's order: earlier-declared watchers fire first).
    node->next = NULL;
    hl_cell_sub_lock(c);   // Phase 5c: shared-cell list mutation under the lock
    HiLowCellWatcher** tail = &c->watchers;
    while (*tail) {
        tail = &(*tail)->next;
    }
    *tail = node;

    if (w) {
        HiLowWatcherSub* sub = malloc(sizeof(HiLowWatcherSub));
        sub->cell = c;
        sub->next = w->subs;
        w->subs = sub;
    }
    hl_cell_sub_unlock(c);
}

void hl_cell_unsubscribe_watcher(HiLowCell* c, HiLowWatcher* w) {
    if (!c || !w) return;
    hl_cell_sub_lock(c);   // Phase 5c: shared-cell list mutation under the lock
    HiLowCellWatcher** cur = &c->watchers;
    while (*cur) {
        if ((*cur)->watcher == w) {
            HiLowCellWatcher* dead = *cur;
            *cur = dead->next;
            // Symmetric unlink: one backref per subscription node.
            watcher_drop_backref(w, c);
            free(dead);
        } else {
            cur = &(*cur)->next;
        }
    }
    hl_cell_sub_unlock(c);
}

// Phase 3e-γ: origin-filtered removal for retargeting — only w's nodes
// attributed to `origin` (the rebinding slot's cell) are removed; w's nodes
// with other origins stay, and each of those keeps its own backref (one
// backref per node, dropped only for removed nodes — the symmetry
// hl_cell_subscribe establishes).
void hl_cell_unsubscribe_watcher_origin(HiLowCell* c, HiLowWatcher* w, HiLowCell* origin) {
    if (!c || !w) return;
    hl_cell_sub_lock(c);   // Phase 5c: no-op for non-shared (retargeting is
    HiLowCellWatcher** cur = &c->watchers;   // container-only; shared is scalar)
    while (*cur) {
        if ((*cur)->watcher == w && (*cur)->origin == origin) {
            HiLowCellWatcher* dead = *cur;
            *cur = dead->next;
            watcher_drop_backref(w, c);
            free(dead);
        } else {
            cur = &(*cur)->next;
        }
    }
    hl_cell_sub_unlock(c);
}

// Containment backrefs (Phase 2d). One NON-OWNING entry per containment;
// duplicates are deliberate (same child twice in one parent → two entries),
// remove drops exactly one.
void hl_cell_add_parent(HiLowCell* child, HiLowCell* parent) {
    HiLowCellParent* node = malloc(sizeof(HiLowCellParent));
    node->parent = parent;
    node->next = child->parents;
    child->parents = node;
}

void hl_cell_remove_parent(HiLowCell* child, HiLowCell* parent) {
    HiLowCellParent** cur = &child->parents;
    while (*cur) {
        if ((*cur)->parent == parent) {
            HiLowCellParent* dead = *cur;
            *cur = dead->next;
            free(dead);
            return;
        }
        cur = &(*cur)->next;
    }
}

// Value deltas (Phase 2c). Internal allocations deliberately do not touch
// hl_alloc_count/hl_free_count: a delta is always balanced inside one
// mutator call today (like arr->data reallocs); valgrind still sees them.
HiLowDelta* hl_delta_new_elem(int event, const void* elem_bytes, size_t elem_size,
                              hl_elem_fn retain_fn, hl_elem_fn release_fn) {
    HiLowDelta* d = malloc(sizeof(HiLowDelta));
    d->event = event;
    d->payload = malloc(elem_size);
    memcpy(d->payload, elem_bytes, elem_size);
    d->payload_size = elem_size;
    d->payload_release = release_fn;
    d->from = 0;
    d->to = 0;
    // Object arrays: the delta holds its own reference so it stays valid
    // independent of the array and the caller (queueable).
    if (retain_fn != NULL) {
        retain_fn(*(void**)d->payload);
    }
    return d;
}

HiLowDelta* hl_delta_new_moved(size_t from, size_t to) {
    HiLowDelta* d = malloc(sizeof(HiLowDelta));
    d->event = HL_ARR_MOVED;
    d->payload = NULL;
    d->payload_size = 0;
    d->payload_release = NULL;
    d->from = from;
    d->to = to;
    return d;
}

void hl_delta_release(HiLowDelta* d) {
    if (!d) return;
    if (d->payload != NULL) {
        if (d->payload_release != NULL) {
            d->payload_release(*(void**)d->payload);
        }
        free(d->payload);
    }
    free(d);
}

// Notification-walk depth (Phase 3e-β). While any hl_cell_notify walk is in
// flight, releases of a slot's OLD payload (the hl_cell_set_* step-6 release)
// are DEFERRED: a watcher body may run inside a walk of the very cell being
// released (a body rebinding its own followed variable), and the walk — plus
// the body's borrowed snapshot of the old value — must outlive it. The
// deferred list drains when the outermost walk completes; releases never
// notify, so draining cannot recurse into a walk.
// Phase 5a: thread-local (per-execution-context by nature — a notify walk runs
// entirely on one thread).
static _Thread_local int hl_notify_depth = 0;

typedef void (*hl_deferred_release_fn)(void*);
typedef struct {
    hl_deferred_release_fn fn;
    void* ptr;
} HiLowDeferredRelease;
// Phase 5a: thread-local. Deferred releases are freed by the thread that
// deferred them; cross-thread frees are precisely what this design avoids.
static _Thread_local HiLowDeferredRelease* hl_deferred_releases = NULL;
static _Thread_local size_t hl_deferred_len = 0;
static _Thread_local size_t hl_deferred_cap = 0;

static void hl_release_array_voidp(void* p) { hl_array_release((HiLowArray*)p); }
static void hl_release_object_voidp(void* p) { hl_object_release((HiLowObject*)p); }

// Release now, or park until the outermost in-flight notify walk finishes.
static void hl_release_or_defer(hl_deferred_release_fn fn, void* ptr) {
    if (hl_notify_depth == 0) {
        fn(ptr);
        return;
    }
    if (hl_deferred_len == hl_deferred_cap) {
        hl_deferred_cap = (hl_deferred_cap == 0) ? 8 : (hl_deferred_cap * 2);
        hl_deferred_releases = realloc(hl_deferred_releases, hl_deferred_cap * sizeof(HiLowDeferredRelease));
    }
    hl_deferred_releases[hl_deferred_len].fn = fn;
    hl_deferred_releases[hl_deferred_len].ptr = ptr;
    hl_deferred_len++;
}

static void hl_drain_deferred_releases(void) {
    // Deferrals are only pushed from inside a walk (depth > 0); we run at
    // depth 0, so the list cannot grow underneath this loop.
    for (size_t i = 0; i < hl_deferred_len; i++) {
        hl_deferred_releases[i].fn(hl_deferred_releases[i].ptr);
    }
    hl_deferred_len = 0;
}

// ===================== Phase 5a: notification queue =====================
// Same-thread delivery stays synchronous and exact (R1); this inbox is the
// EXCLUSIVELY cross-thread path. Through Phase 5a only one thread exists, so no
// watcher is ever owned by a thread other than the mutating one — hl_cell_notify
// always takes the synchronous branch and this machinery is exercised only by
// the inbox unit tests (tests/inbox_unit_harness.c). The single-threaded corpus
// stays byte-identical: the routing check, the safe-point drains, and the
// owner_ctx stamp are all no-ops or same-thread on one thread.

// One accumulated delta in an entry (R3: deltas are never dropped on coalesce;
// HiLowDelta carries no link of its own, so wrap it).
typedef struct HiLowInboxDelta {
    HiLowDelta* delta;
    struct HiLowInboxDelta* next;
} HiLowInboxDelta;

// One coalescing entry, keyed by watcher identity (§4 axioms 1-2).
typedef struct HiLowInboxEntry {
    HiLowWatcher* watcher;      // retained +1 at enqueue, dropped at drain (R6)
    HiLowCell* cell;            // retained (container refcount); the body's cell arg
    void* body_fn;              // snapshot of the subscription's fire closure
    void* env;                  // == watcher->env; valid while the watcher is retained
    int event;                  // event for a collapsed bare-(changed) fire
    HiLowInboxDelta* deltas;    // accumulated payload deltas, OWNED (fired in order)
    HiLowInboxDelta* deltas_tail;
    bool pending;               // at-least-once: fire even with no payload delta
    struct HiLowInboxEntry* next;
} HiLowInboxEntry;

typedef struct HiLowInbox {
    pthread_mutex_t lock;
    HiLowInboxEntry* head;      // MPSC: producers push under lock, owner drains
    atomic_int nonempty;        // cheap racy flag the owner tests at safe points
} HiLowInbox;

struct HiLowThreadContext {
    HiLowInbox inbox;
    uint64_t thread_id;                  // owner comparison / documented ordering
    struct HiLowThreadContext* reg_next; // global registry link
};

// Global thread registry (mutex-guarded; off the hot path). Threads self-register
// on first hl_current_ctx() and unregister at hl_thread_final_drain.
static pthread_mutex_t hl_registry_lock = PTHREAD_MUTEX_INITIALIZER;
static HiLowThreadContext* hl_registry = NULL;
static uint64_t hl_next_thread_id = 0;

static _Thread_local HiLowThreadContext* hl_tls_ctx = NULL;

HiLowThreadContext* hl_current_ctx(void) {
    if (hl_tls_ctx) return hl_tls_ctx;
    HiLowThreadContext* ctx = calloc(1, sizeof(HiLowThreadContext));
    pthread_mutex_init(&ctx->inbox.lock, NULL);
    ctx->inbox.head = NULL;
    atomic_init(&ctx->inbox.nonempty, 0);
    pthread_mutex_lock(&hl_registry_lock);
    ctx->thread_id = hl_next_thread_id++;
    ctx->reg_next = hl_registry;
    hl_registry = ctx;
    pthread_mutex_unlock(&hl_registry_lock);
    hl_tls_ctx = ctx;
    return ctx;
}

// Accumulate one delta into an entry (transfers ownership). A NULL delta (a bare
// (changed)/(assigned) fire) adds no node but marks the entry pending (R3: the
// changed-collapse; the payload deltas below always accumulate).
static void hl_inbox_accumulate(HiLowInboxEntry* e, HiLowDelta* delta) {
    e->pending = true;
    if (!delta) return;
    HiLowInboxDelta* node = malloc(sizeof(HiLowInboxDelta));
    node->delta = delta;
    node->next = NULL;
    if (e->deltas_tail) e->deltas_tail->next = node; else e->deltas = node;
    e->deltas_tail = node;
}

// Copy a borrowed notify delta into a queue-owned one. The delta reaching
// hl_cell_notify is borrowed (the mutator releases it after notify returns), so
// the cross-thread branch cannot adopt it — it copies. In all of Phase 5 the
// only cross-thread fires are scalar `shared` assignments (HL_SCALAR_ASSIGNED,
// NULL delta) — shared containers are rejected in 5c, so an object-payload delta
// never reaches this path; the object branch is a documented 5b/5c wiring point
// (delta-ownership transfer from the mutator), never hit in Phase 5, never a
// silent drop of a reachable delta.
static HiLowDelta* hl_delta_copy_for_queue(const HiLowDelta* d) {
    if (!d) return NULL;
    if (d->event == HL_ARR_MOVED) return hl_delta_new_moved(d->from, d->to);
    if (d->payload_release != NULL) return NULL; // object payload: unreachable in Phase 5
    return hl_delta_new_elem(d->event, d->payload, d->payload_size, NULL, NULL);
}

void hl_inbox_enqueue(HiLowThreadContext* owner, HiLowWatcher* w, HiLowCell* cell,
                      void* body_fn, void* env, int event, HiLowDelta* delta) {
    pthread_mutex_lock(&owner->inbox.lock);
    HiLowInboxEntry* e = owner->inbox.head;
    while (e && e->watcher != w) e = e->next;   // coalesce by watcher identity
    if (e) {
        hl_inbox_accumulate(e, delta);          // R3: merge, never append/drop
    } else {
        e = malloc(sizeof(HiLowInboxEntry));
        hl_watcher_retain(w);                   // R6: env/body_fn valid across gap
        hl_cell_retain(cell);                   // own the cell across the gap (axiom 5)
        e->watcher = w; e->cell = cell;
        e->body_fn = body_fn; e->env = env; e->event = event;
        e->deltas = NULL; e->deltas_tail = NULL; e->pending = false;
        hl_inbox_accumulate(e, delta);
        e->next = owner->inbox.head;
        owner->inbox.head = e;
    }
    atomic_store(&owner->inbox.nonempty, 1);
    pthread_mutex_unlock(&owner->inbox.lock);
}

size_t hl_inbox_pending_count(HiLowThreadContext* ctx) {
    size_t n = 0;
    pthread_mutex_lock(&ctx->inbox.lock);
    for (HiLowInboxEntry* e = ctx->inbox.head; e; e = e->next) n++;
    pthread_mutex_unlock(&ctx->inbox.lock);
    return n;
}

// Fire one drained entry on the declaring (calling) thread, then free it. Ended
// watchers drop WITHOUT firing (R5; the 3e-β dead-watcher ruling extended across
// the enqueue/drain gap). Owned resources are released either way (axioms 1,2,4).
static bool hl_inbox_fire_entry(HiLowInboxEntry* e) {
    HiLowWatcher* w = e->watcher;
    bool live = w && w->active && !w->ended;
    if (live) {
        if (e->deltas) {
            for (HiLowInboxDelta* d = e->deltas; d; d = d->next) {
                ((HiLowWatcherBody)e->body_fn)(e->env, e->cell, d->delta);
            }
        } else if (e->pending) {
            ((HiLowWatcherBody)e->body_fn)(e->env, e->cell, NULL);
        }
    }
    for (HiLowInboxDelta* d = e->deltas; d; ) {
        HiLowInboxDelta* next = d->next;
        hl_delta_release(d->delta);
        free(d);
        d = next;
    }
    // Phase 5c (5a-ii): the inbox may be the SOLE owner of the cell at drain
    // (a shared scalar whose declaring binding was released while an entry was
    // in flight). hl_cell_release_full does the full typed teardown at zero
    // from this — possibly the producer — thread, instead of leaking the struct.
    hl_cell_release_full(e->cell);
    hl_watcher_release(w);
    free(e);
    return live;
}

size_t hl_thread_drain_inbox(void) {
    HiLowThreadContext* ctx = hl_current_ctx();
    // Detach the whole list under the lock, then fire outside it so a body may
    // enqueue (same- or cross-thread) without deadlocking and same-thread
    // synchronous fires inside a body are unaffected.
    pthread_mutex_lock(&ctx->inbox.lock);
    HiLowInboxEntry* list = ctx->inbox.head;
    ctx->inbox.head = NULL;
    atomic_store(&ctx->inbox.nonempty, 0);
    pthread_mutex_unlock(&ctx->inbox.lock);

    size_t fired = 0;
    while (list) {
        HiLowInboxEntry* next = list->next;
        if (hl_inbox_fire_entry(list)) fired++;
        list = next;
    }
    return fired;
}

void hl_thread_safepoint(void) {
    // A drain is legal only at hl_notify_depth == 0 (never inside a body). Do NOT
    // create a context here: a thread that never watches keeps NULL and pays
    // nothing. The nonempty flag is a cheap racy read — a false negative is
    // corrected at the next safe point, a false positive just locks and finds
    // nothing. Single-threaded: no cross-thread producer, so always nonempty==0.
    if (hl_notify_depth != 0) return;
    HiLowThreadContext* ctx = hl_tls_ctx;
    if (!ctx) return;
    if (atomic_load(&ctx->inbox.nonempty) == 0) return;
    hl_thread_drain_inbox();
}

// Thread teardown: a final drain (axiom 6 — after this the inbox is empty), then
// unregister and destroy the context. In 5a nothing calls this on the main
// thread (no thread teardown hook yet — that is 5b's join path); it is exercised
// by the unit tests and is the permanent contract.
void hl_thread_final_drain(void) {
    HiLowThreadContext* ctx = hl_tls_ctx;
    if (!ctx) return;
    hl_thread_drain_inbox();
    pthread_mutex_lock(&hl_registry_lock);
    HiLowThreadContext** pp = &hl_registry;
    while (*pp && *pp != ctx) pp = &(*pp)->reg_next;
    if (*pp) *pp = ctx->reg_next;
    pthread_mutex_unlock(&hl_registry_lock);
    pthread_mutex_destroy(&ctx->inbox.lock);
    free(ctx);
    hl_tls_ctx = NULL;
}

// Phase 5b: async-thread registry. Every `async { }` block spawns a pthread
// via hl_async_spawn; the program joins them all at exit via hl_async_join_all
// (no detached threads). The list is a growable array guarded by a mutex so
// nested `async` (an async body that itself spawns) is safe. Only reachable in
// threaded programs; the single-threaded corpus never calls these.
static pthread_mutex_t hl_async_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_t* hl_async_threads = NULL;
static size_t hl_async_count = 0;
static size_t hl_async_cap = 0;

void hl_async_spawn(void* (*body)(void*), void* arg) {
    pthread_t t;
    pthread_create(&t, NULL, body, arg);
    pthread_mutex_lock(&hl_async_lock);
    if (hl_async_count == hl_async_cap) {
        size_t ncap = hl_async_cap ? hl_async_cap * 2 : 8;
        hl_async_threads = realloc(hl_async_threads, ncap * sizeof(pthread_t));
        hl_async_cap = ncap;
    }
    hl_async_threads[hl_async_count++] = t;
    pthread_mutex_unlock(&hl_async_lock);
}

void hl_async_join_all(void) {
    // Snapshot under the lock, join outside it. A joined thread performed its
    // own final drain before returning, so after this returns every producer's
    // cross-thread enqueues onto THIS thread's inbox have happened-before (the
    // join is the synchronization edge); the caller drains last.
    pthread_mutex_lock(&hl_async_lock);
    size_t n = hl_async_count;
    pthread_mutex_unlock(&hl_async_lock);
    for (size_t i = 0; i < n; i++) {
        pthread_join(hl_async_threads[i], NULL);
    }
    pthread_mutex_lock(&hl_async_lock);
    // Only free once every spawned thread has been joined (nested spawns during
    // join would have grown the list; join those too on a second pass is not
    // needed in 5b — async bodies are producers, not spawners — but keep the
    // free guarded so a re-entrant caller doesn't free a live array).
    if (hl_async_count == n) {
        free(hl_async_threads);
        hl_async_threads = NULL;
        hl_async_count = 0;
        hl_async_cap = 0;
    }
    pthread_mutex_unlock(&hl_async_lock);
}

// Phase 2d: recursively collect the not-yet-visited ancestors of c into a
// growable list, stamping each with the current epoch on first visit.
static void deep_collect_ancestors(HiLowCell* c, HiLowCell*** list, size_t* len, size_t* cap) {
    for (HiLowCellParent* p = c->parents; p != NULL; p = p->next) {
        HiLowCell* a = p->parent;
        if (a->version == hl_deep_epoch) continue;  // diamond/cycle: already visited
        a->version = hl_deep_epoch;
        if (*len == *cap) {
            *cap = (*cap == 0) ? 8 : (*cap * 2);
            *list = realloc(*list, *cap * sizeof(HiLowCell*));
        }
        (*list)[(*len)++] = a;
        deep_collect_ancestors(a, list, len, cap);
    }
}

// The ONE firing path (Phase 2c; deep walk added 2d). Single walk in list
// order; implicit CHANGED: every mutation event also fires (changed)
// watchers — mirroring the pre-2c per-mutator loops exactly — and, as of
// 2d, (deep) watchers on the mutated cell itself fire for every event.
// Then the parent walk fires ancestors' (deep) subscribers only, with the
// same delta. Collect-then-fire keeps the traversal atomic with respect to
// body execution: a nested mutation inside a deep body starts its own walk
// under a fresh epoch and cannot corrupt this one. The stealth check lives
// here, its single authoritative site (mutators also consult it only to
// skip delta construction) — stealth therefore suppresses deep fires too.
// Phase 3e-β hardening: snapshot of one subscription node, taken before any
// body runs. Bodies may retarget or unsubscribe nodes on the cell being
// walked (a body rebinding its own followed variable), so the walk must
// never touch live list links after a body has run. Watcher STATE pointers
// stay valid across a walk (a body cannot release a pre-existing watcher
// binding — bindings release at their owning scope's exit, which a body
// cannot pop; .end() sets flags only); active/ended are read at fire time.
typedef struct {
    int modifier;
    void* body_fn;
    void* env;
    HiLowWatcher* watcher;
    HiLowCell* origin;   // Phase 3e-γ: carried so retarget re-subscribes with
                         // the node's attribution intact; inert for notify
} HiLowNodeSnap;

#define HL_NODE_SNAP_INLINE 16

// Snapshot c's subscriber list into *snap (inline buffer of
// HL_NODE_SNAP_INLINE, heap beyond); returns the count.
static size_t snapshot_cell_nodes(HiLowCell* c, HiLowNodeSnap inline_buf[], HiLowNodeSnap** snap) {
    size_t n = 0, cap = HL_NODE_SNAP_INLINE;
    *snap = inline_buf;
    for (HiLowCellWatcher* node = c->watchers; node != NULL; node = node->next) {
        if (n == cap) {
            cap *= 2;
            if (*snap == inline_buf) {
                *snap = malloc(cap * sizeof(HiLowNodeSnap));
                memcpy(*snap, inline_buf, n * sizeof(HiLowNodeSnap));
            } else {
                *snap = realloc(*snap, cap * sizeof(HiLowNodeSnap));
            }
        }
        (*snap)[n].modifier = node->modifier;
        (*snap)[n].body_fn = node->body_fn;
        (*snap)[n].env = node->env;
        (*snap)[n].watcher = node->watcher;
        (*snap)[n].origin = node->origin;
        n++;
    }
    return n;
}

// Deliver one fire to a subscriber: synchronous on the owning thread (R1, exact
// same-thread semantics), or enqueued into the owner's inbox when the owner is a
// different thread (R6). Single-threaded: owner is always `self`, so every fire
// takes the synchronous branch and generated behavior is byte-identical.
static void hl_notify_deliver(HiLowThreadContext* self, HiLowWatcher* state,
                              void* body_fn, void* env, HiLowCell* cell,
                              int event, const HiLowDelta* delta) {
    HiLowThreadContext* owner = state->owner_ctx;
    if (owner == NULL || owner == self) {
        ((HiLowWatcherBody)body_fn)(env, cell, delta);
    } else {
        hl_inbox_enqueue(owner, state, cell, body_fn, env, event,
                         hl_delta_copy_for_queue(delta));
    }
}

void hl_cell_notify(HiLowCell* c, int event, const HiLowDelta* delta) {
    if (hl_stealth_depth != 0) return;
    hl_notify_depth++;
    HiLowThreadContext* self = hl_current_ctx();

    // Collect-then-fire (Phase 3e-β): the traversal is atomic with respect
    // to body execution — the same discipline the deep walk below has had
    // since 2d. A node retargeted away mid-walk still fires for this event
    // (it was subscribed when the event happened); a node subscribed
    // mid-walk does not.
    HiLowNodeSnap inline_buf[HL_NODE_SNAP_INLINE];
    HiLowNodeSnap* snap;
    // Phase 5c: for a SHARED cell the subscriber list may be concurrently
    // mutated by the declaring thread (watch declared, watcher ended), so take
    // the subscriber lock ONLY around the snapshot, then release it and fire the
    // snapshot with the lock NOT held (a body may re-subscribe or notify without
    // deadlock; enqueueing never touches this lock). Non-shared: no lock (the
    // snapshot is single-threaded), fast path unchanged.
    hl_cell_sub_lock(c);
    size_t n = snapshot_cell_nodes(c, inline_buf, &snap);
    hl_cell_sub_unlock(c);
    for (size_t i = 0; i < n; i++) {
        HiLowWatcher* state = snap[i].watcher;
        if (state == NULL || !state->active || state->ended) continue;
        // Phase 3b: an equal-value scalar assignment is NOT a mutation —
        // CHANGED and DEEP subscribers do not fire on HL_SCALAR_ASSIGNED.
        if (snap[i].modifier == event
            || (snap[i].modifier == HL_ARR_CHANGED && event != HL_SCALAR_ASSIGNED)
            || (snap[i].modifier == HL_ARR_DEEP && event != HL_SCALAR_ASSIGNED)) {
            hl_notify_deliver(self, state, snap[i].body_fn, snap[i].env, c, event, delta);
        }
    }
    if (snap != inline_buf) free(snap);

    // Parent walk (Phase 2d): zero cost unless someone deep-watches above.
    // Phase 3b: never for HL_SCALAR_ASSIGNED (not a mutation; unreachable for
    // scalar cells today — no parents, never deep-marked — but written so).
    if (event != HL_SCALAR_ASSIGNED && c->deep_watched && c->parents != NULL) {
        hl_deep_epoch++;
        c->version = hl_deep_epoch;  // own deep nodes already fired above
        HiLowCell** ancestors = NULL;
        size_t len = 0, cap = 0;
        deep_collect_ancestors(c, &ancestors, &len, &cap);
        for (size_t i = 0; i < len; i++) {
            HiLowNodeSnap a_inline[HL_NODE_SNAP_INLINE];
            HiLowNodeSnap* a_snap;
            size_t a_n = snapshot_cell_nodes(ancestors[i], a_inline, &a_snap);
            for (size_t j = 0; j < a_n; j++) {
                HiLowWatcher* state = a_snap[j].watcher;
                if (state == NULL || !state->active || state->ended) continue;
                if (a_snap[j].modifier == HL_ARR_DEEP) {
                    hl_notify_deliver(self, state, a_snap[j].body_fn, a_snap[j].env,
                                      ancestors[i], event, delta);
                }
            }
            if (a_snap != a_inline) free(a_snap);
        }
        free(ancestors);
    }

    hl_notify_depth--;
    if (hl_notify_depth == 0 && hl_deferred_len > 0) {
        hl_drain_deferred_releases();
    }
}

// Watcher value operations (Phase 10-δ-α)
HiLowWatcher* hl_watcher_new(void) {
    HiLowWatcher* w = malloc(sizeof(HiLowWatcher));
    hl_alloc_count++;
    w->refcount = 1;           // Initialize refcount to 1
    w->active = true;          // Start active
    w->ended = false;          // Not ended initially
    w->subs = NULL;            // No subscriptions yet (Phase 2a)
    w->env = NULL;             // No owned env (Phase 2b)
    w->env_dtor = NULL;        // No retained env cells (Phase 3b)
    w->owner_ctx = hl_current_ctx();  // Phase 5a: the declaring thread's context;
                               // hl_cell_notify routes a fire here. Single-thread:
                               // this is the main context, always == the mutating
                               // thread's, so delivery stays synchronous (R1).
    return w;
}

// Registration by construction (Phase 2a): creating a watcher value
// subscribes it. Varargs are n (HiLowCell*, int modifier) pairs.
// Phase 2b: the watcher takes OWNERSHIP of env — it is freed on the
// watcher's final release, never by scope cleanup. Phase 3b: env_dtor
// releases the env's retained cells at that point (the env slots own a
// retain on every cell they hold — escape soundness).
HiLowWatcher* hl_watcher_new_subscribed(void* body_fn, void* env, void (*env_dtor)(void*), int n, ...) {
    HiLowWatcher* w = hl_watcher_new();
    w->env = env;
    w->env_dtor = env_dtor;
    va_list args;
    va_start(args, n);
    for (int i = 0; i < n; i++) {
        HiLowCell* cell = va_arg(args, HiLowCell*);
        int modifier = va_arg(args, int);
        hl_cell_subscribe(cell, modifier, body_fn, env, w, NULL);
    }
    va_end(args);
    return w;
}

// Phase 3e-γ: (cell, modifier, origin) triples — decl-form watchers with
// followed variables attribute each container-content subscription to its
// slot's cell so hl_slot_retarget moves only the rebinding slot's nodes.
HiLowWatcher* hl_watcher_new_subscribed_origins(void* body_fn, void* env, void (*env_dtor)(void*), int n, ...) {
    HiLowWatcher* w = hl_watcher_new();
    w->env = env;
    w->env_dtor = env_dtor;
    va_list args;
    va_start(args, n);
    for (int i = 0; i < n; i++) {
        HiLowCell* cell = va_arg(args, HiLowCell*);
        int modifier = va_arg(args, int);
        HiLowCell* origin = va_arg(args, HiLowCell*);
        hl_cell_subscribe(cell, modifier, body_fn, env, w, origin);
    }
    va_end(args);
    return w;
}

void hl_watcher_retain(HiLowWatcher* w) {
    if (w != NULL) {
        HL_RC_INC(w->refcount);
    }
}

void hl_watcher_release(HiLowWatcher* w) {
    if (w != NULL) {
        if (HL_RC_DEC(w->refcount) == 0) {
            // Unsubscribe from every cell first (Phase 2a): no cell node may
            // outlive the watcher it points at.
            while (w->subs) {
                HiLowWatcherSub* sub = w->subs;
                HiLowCell* cell = sub->cell;
                // hl_cell_unsubscribe_watcher removes ALL of w's nodes on
                // that cell AND all matching backrefs — including `sub`
                // itself — so re-read w->subs rather than walking.
                hl_cell_unsubscribe_watcher(cell, w);
                // Defensive: if the backref survived (shouldn't happen),
                // drop it to guarantee loop progress.
                if (w->subs == sub) {
                    w->subs = sub->next;
                    free(sub);
                }
            }
            // Phase 2b: the watcher owns its env — free it after
            // unsubscribing. Phase 3b: the generated dtor first releases the
            // cells the env slots retain; the free itself stays here.
            if (w->env) {
                if (w->env_dtor) {
                    w->env_dtor(w->env);
                }
                free(w->env);
                hl_free_count++;
            }
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

// Boxed scalars (Phase 3b). A watched scalar variable lowered to a cell:
// cell header first, HiLowValue payload — the one payload representation.
// Only corpus-needed kinds have constructors (i32 as of 3b).
HiLowScalar* hl_scalar_new_i32(int32_t v) {
    HiLowScalar* s = malloc(sizeof(HiLowScalar));
    hl_alloc_count++;
    s->cell.refcount = 1;
    s->cell.kind = HL_CELL_SCALAR;  // Phase 5c: typed-teardown dispatch
    s->cell.sub_lock = NULL;        // Phase 5c: not shared (overridden by hl_scalar_new_i32_shared)
    s->cell.shm = NULL;             // Phase 6a: not placed (overridden by hl_scalar_new_i32_placed)
    s->cell.watchers = NULL;
    s->cell.parents = NULL;
    s->cell.version = 0;
    s->cell.deep_watched = false;
    s->value.type = HL_VALUE_I32;
    s->value.value.i32_val = v;
    return s;
}

// Phase 5c: a `shared` i32 scalar. Identical to hl_scalar_new_i32 except the
// cell gets a subscriber mutex (sub_lock), which marks it shared: payload
// access is atomic (hl_scalar_get_i32 / hl_cell_set_i32 branch on sub_lock) and
// the subscriber list is guarded across producer threads. Only emitted for
// `shared let` (a threaded-mode program).
HiLowScalar* hl_scalar_new_i32_shared(int32_t v) {
    HiLowScalar* s = hl_scalar_new_i32(v);
    s->cell.sub_lock = malloc(sizeof(pthread_mutex_t));
    pthread_mutex_init(s->cell.sub_lock, NULL);
    return s;
}

// ==========================================================================
// Phase 6a: process tier — cross-process `shared` by PLACEMENT
// (docs/phase6-brief.md, rulings R-A–R-E). A `shared("name") let x: i32` binds
// to a typed slot in a named POSIX shared-memory segment mapped by every
// participant. Nothing is serialized or sent (R-A); a write publishes payload +
// epoch (R-B, drained cross-process only in 6b). In 6a the write protocol is
// complete (epoch bumped on every write) but nothing pulls it yet.
// ==========================================================================

// The versioned segment header (axiom 2). Atomic fields are plain integers
// here and accessed via cast-to-_Atomic (the pre-5c idiom) so runtime.h — which
// generated main.c includes — needs no _Atomic in any struct. The header is the
// contract 6b/6c build on; version it from day one.
#define HL_SHM_MAGIC          0x484C5348u  // "HLSH"
#define HL_SHM_LAYOUT_VERSION 1u
#define HL_SHM_ABI_VERSION    1u
#define HL_SHM_TYPE_I32       1u
#define HL_SHM_INIT_INCOMPLETE 0u
#define HL_SHM_INIT_COMPLETE   1u
// Bounded init-wait: an attacher waits this long for the creator to publish
// init-complete, then fails as a startup error (so a creator that crashed
// mid-init cannot hang attachers forever — R-D). ~2s at 200us/iter. Overridable
// (#ifndef) so the shm unit harness can compile a fast-timeout runtime.
#ifndef HL_SHM_INIT_WAIT_ITERS
#define HL_SHM_INIT_WAIT_ITERS 10000
#endif
#define HL_SHM_INIT_WAIT_NS    200000L

typedef struct HiLowShmHeader {
    uint32_t magic;
    uint32_t layout_version;
    uint32_t abi_version;
    uint32_t type_tag;
    uint32_t payload_size;
    uint32_t init_state;     // accessed atomically; release-published by creator
    uint64_t epoch;          // accessed atomically; bumped (release) on every write
    // payload follows at HL_SHM_PAYLOAD_OFFSET
} HiLowShmHeader;

// Payload aligned to 8 after the header (i32 needs 4; 8 is future-proof and
// keeps the epoch's 8-byte alignment obvious).
#define HL_SHM_PAYLOAD_OFFSET ((sizeof(HiLowShmHeader) + 7u) & ~((size_t)7u))

struct HiLowShmSegment {
    int   fd;
    void* base;          // mmap'd region (header + payload)
    size_t map_size;
    char* shm_name;      // "/hilow.<name>", owned
    HiLowShmHeader* header;
    void* payload;       // base + HL_SHM_PAYLOAD_OFFSET
};

// Startup error: a placement precondition failed unrecoverably (name too long,
// shm syscall failure, header mismatch, init timeout). Diagnostic to stderr,
// exit(1) — matching the runtime's other unrecoverable diagnostics (property
// type mismatch, etc.). Never a warning (R-D).
static void hl_shm_startup_error(const char* seg_name, const char* what) {
    fprintf(stderr,
        "shared segment '%s': %s\n"
        "  (a `shared(\"...\")` cross-process scalar could not be placed; "
        "this is a startup error, not a warning)\n",
        seg_name ? seg_name : "?", what);
    exit(1);
}

static int32_t hl_shm_load_i32(HiLowShmSegment* seg) {
    // Acquire-load the payload (axiom 3): pairs with the writer's release store.
    return atomic_load_explicit((_Atomic int32_t*)seg->payload, memory_order_acquire);
}

// Store payload then bump the epoch, both release-ordered — the single
// publication point (axiom 3). Returns the prior payload value (so the caller's
// (changed) test works), like an exchange.
static int32_t hl_shm_exchange_i32(HiLowShmSegment* seg, int32_t v) {
    int32_t old = atomic_exchange_explicit((_Atomic int32_t*)seg->payload, v, memory_order_release);
    atomic_fetch_add_explicit((_Atomic uint64_t*)&seg->header->epoch, 1, memory_order_release);
    return old;
}

static void hl_shm_detach(HiLowShmSegment* seg) {
    if (!seg) return;
    if (seg->base && seg->base != MAP_FAILED) munmap(seg->base, seg->map_size);
    if (seg->fd >= 0) close(seg->fd);
    free(seg->shm_name);
    free(seg);
    // No shm_unlink — segments persist by default (cleanup policy is Phase 6c).
}

// Validate the user-facing name and build the shm object name "/hilow.<name>".
// Charset/length are ALSO checked at compile time (codegen), so a reachable
// program never trips these; they are defense-in-depth for the runtime call.
// Returns a malloc'd string, or NULL on invalid (caller raises startup error).
static char* hl_shm_object_name(const char* user_name) {
    if (!user_name) return NULL;
    size_t n = strlen(user_name);
    if (n == 0 || n > 64) return NULL;
    for (size_t i = 0; i < n; i++) {
        char ch = user_name[i];
        int ok = (ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z') ||
                 (ch >= '0' && ch <= '9') || ch == '.' || ch == '_' || ch == '-';
        if (!ok) return NULL;
    }
    const char* prefix = "/hilow.";
    size_t total = strlen(prefix) + n + 1;
    char* out = malloc(total);
    memcpy(out, prefix, strlen(prefix));
    memcpy(out + strlen(prefix), user_name, n);
    out[total - 1] = '\0';
    return out;
}

// Create-or-attach a segment for an i32 (R-D). O_CREAT|O_EXCL winner
// initializes and release-publishes init-complete; the loser attaches, waits
// (bounded) for init-complete, verifies every header field, and observes the
// current value — it does NOT run the initializer and fires nothing.
static HiLowShmSegment* hl_shm_attach_i32(const char* user_name, int32_t init_value) {
    char* name = hl_shm_object_name(user_name);
    if (!name) hl_shm_startup_error(user_name, "invalid segment name (must be [A-Za-z0-9._-]+, 1–64 chars)");

    HiLowShmSegment* seg = malloc(sizeof(HiLowShmSegment));
    seg->fd = -1; seg->base = NULL; seg->map_size = 0;
    seg->shm_name = name; seg->header = NULL; seg->payload = NULL;

    size_t map_size = HL_SHM_PAYLOAD_OFFSET + sizeof(int32_t);
    seg->map_size = map_size;

    int fd = shm_open(name, O_CREAT | O_EXCL | O_RDWR, 0600);
    if (fd >= 0) {
        // Creator: size, map, write header + payload, publish init-complete.
        seg->fd = fd;
        if (ftruncate(fd, (off_t)map_size) != 0)
            hl_shm_startup_error(user_name, "ftruncate failed");
        seg->base = mmap(NULL, map_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        if (seg->base == MAP_FAILED) hl_shm_startup_error(user_name, "mmap failed (creator)");
        seg->header = (HiLowShmHeader*)seg->base;
        seg->payload = (char*)seg->base + HL_SHM_PAYLOAD_OFFSET;
        seg->header->magic = HL_SHM_MAGIC;
        seg->header->layout_version = HL_SHM_LAYOUT_VERSION;
        seg->header->abi_version = HL_SHM_ABI_VERSION;
        seg->header->type_tag = HL_SHM_TYPE_I32;
        seg->header->payload_size = (uint32_t)sizeof(int32_t);
        atomic_store_explicit((_Atomic uint64_t*)&seg->header->epoch, 0, memory_order_relaxed);
        atomic_store_explicit((_Atomic int32_t*)seg->payload, init_value, memory_order_relaxed);
        // Release-publish: an attacher that observes COMPLETE (acquire) sees all
        // the header + payload writes above.
        atomic_store_explicit((_Atomic uint32_t*)&seg->header->init_state,
                              HL_SHM_INIT_COMPLETE, memory_order_release);
        return seg;
    }
    if (errno != EEXIST) hl_shm_startup_error(user_name, "shm_open(O_CREAT|O_EXCL) failed");

    // Attacher: open the existing object, map it, wait for init-complete.
    fd = shm_open(name, O_RDWR, 0600);
    if (fd < 0) hl_shm_startup_error(user_name, "shm_open (attach) failed");
    seg->fd = fd;
    struct stat st;
    if (fstat(fd, &st) != 0) hl_shm_startup_error(user_name, "fstat (attach) failed");
    if ((size_t)st.st_size < map_size)
        hl_shm_startup_error(user_name, "segment smaller than the i32 layout (type/layout mismatch)");
    seg->map_size = (size_t)st.st_size;
    seg->base = mmap(NULL, seg->map_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (seg->base == MAP_FAILED) hl_shm_startup_error(user_name, "mmap failed (attach)");
    seg->header = (HiLowShmHeader*)seg->base;
    seg->payload = (char*)seg->base + HL_SHM_PAYLOAD_OFFSET;

    // Bounded wait for the creator to publish init-complete.
    int ready = 0;
    for (int i = 0; i < HL_SHM_INIT_WAIT_ITERS; i++) {
        if (atomic_load_explicit((_Atomic uint32_t*)&seg->header->init_state,
                                 memory_order_acquire) == HL_SHM_INIT_COMPLETE) {
            ready = 1;
            break;
        }
        struct timespec ts = { 0, HL_SHM_INIT_WAIT_NS };
        nanosleep(&ts, NULL);
    }
    if (!ready) hl_shm_startup_error(user_name, "timed out waiting for segment initialization (creator crashed mid-init?)");

    // Verify every header field (R-D): any mismatch is a startup error.
    if (seg->header->magic != HL_SHM_MAGIC)
        hl_shm_startup_error(user_name, "bad magic (not a HiLow segment)");
    if (seg->header->layout_version != HL_SHM_LAYOUT_VERSION)
        hl_shm_startup_error(user_name, "layout-version mismatch");
    if (seg->header->abi_version != HL_SHM_ABI_VERSION)
        hl_shm_startup_error(user_name, "ABI-version mismatch");
    if (seg->header->type_tag != HL_SHM_TYPE_I32)
        hl_shm_startup_error(user_name, "type-tag mismatch (segment holds a different type)");
    if (seg->header->payload_size != (uint32_t)sizeof(int32_t))
        hl_shm_startup_error(user_name, "payload-size mismatch");

    // Attach observes the current value and fires nothing (construction never
    // notifies). init_value is deliberately ignored — this is the one place
    // `shared("n") let x = 5` does not mean what it locally appears to (R-D).
    (void)init_value;
    return seg;
}

// Phase 6a: a `shared("seg_name") let` i32 — a cross-process placed scalar.
// It is a superset of a `shared` scalar: it gets the subscriber mutex (so the
// in-process subscriber list stays cross-thread-safe, 5c machinery intact) AND
// a mapped segment whose slot holds the payload. The accessors branch on
// cell.shm; construction never notifies, so an attacher fires nothing.
HiLowScalar* hl_scalar_new_i32_placed(const char* seg_name, int32_t init_value) {
    HiLowScalar* s = hl_scalar_new_i32_shared(init_value);
    s->cell.shm = hl_shm_attach_i32(seg_name, init_value);
    return s;
}

#ifdef HL_SHM_TEST_SUPPORT
// Test-only (compiled ONLY when the shm unit harness defines HL_SHM_TEST_SUPPORT
// — zero footprint in any real binary). Forges a raw segment with a chosen
// header so the harness can exercise the attacher's verification/timeout paths
// that no i32-only HiLow program can reach (type-tag/magic mismatch, and — with
// mark_complete=0 — the crashed-mid-init timeout). The header layout stays
// encapsulated here (single source of truth); the harness only supplies the
// knobs. Returns 0 on success, -1 on a shm failure.
int hl_shm_test_forge(const char* user_name, uint32_t magic, uint32_t layout,
                      uint32_t abi, uint32_t type_tag, uint32_t payload_size,
                      int mark_complete) {
    char* name = hl_shm_object_name(user_name);
    if (!name) return -1;
    size_t map_size = HL_SHM_PAYLOAD_OFFSET + sizeof(int32_t);
    int fd = shm_open(name, O_CREAT | O_RDWR, 0600);
    free(name);
    if (fd < 0) return -1;
    if (ftruncate(fd, (off_t)map_size) != 0) { close(fd); return -1; }
    void* base = mmap(NULL, map_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (base == MAP_FAILED) { close(fd); return -1; }
    HiLowShmHeader* h = (HiLowShmHeader*)base;
    h->magic = magic;
    h->layout_version = layout;
    h->abi_version = abi;
    h->type_tag = type_tag;
    h->payload_size = payload_size;
    atomic_store_explicit((_Atomic uint64_t*)&h->epoch, 0, memory_order_relaxed);
    atomic_store_explicit((_Atomic int32_t*)((char*)base + HL_SHM_PAYLOAD_OFFSET), 0, memory_order_relaxed);
    atomic_store_explicit((_Atomic uint32_t*)&h->init_state,
        mark_complete ? HL_SHM_INIT_COMPLETE : HL_SHM_INIT_INCOMPLETE, memory_order_release);
    munmap(base, map_size);
    close(fd);
    return 0;
}

// Test-only: the current header contract values, so the harness can forge a
// header that differs in exactly one field.
uint32_t hl_shm_test_magic(void)   { return HL_SHM_MAGIC; }
uint32_t hl_shm_test_layout(void)  { return HL_SHM_LAYOUT_VERSION; }
uint32_t hl_shm_test_abi(void)     { return HL_SHM_ABI_VERSION; }
uint32_t hl_shm_test_type_i32(void){ return HL_SHM_TYPE_I32; }
#endif

void hl_scalar_retain(HiLowScalar* s) {
    if (s != NULL) {
        hl_cell_retain(&s->cell);
    }
}

// Phase 5c: the scalar teardown after the last cell release — reachable via
// hl_scalar_release (same-owner) OR hl_cell_release_full (cross-thread inbox).
static void hl_scalar_finalize(HiLowScalar* s) {
    // Phase 3e: reference payloads own one retained reference — release it
    // before the free. Scalar payloads are POD (no teardown).
    switch (s->value.type) {
        case HL_VALUE_STR:
            hl_array_release(s->value.value.str_val);
            break;
        case HL_VALUE_ARRAY:
            hl_array_release(s->value.value.arr_val);
            break;
        case HL_VALUE_OBJECT:
            hl_object_release(s->value.value.obj_val);
            break;
        default:
            break;
    }
    free(s);
    hl_free_count++;
}

void hl_scalar_release(HiLowScalar* s) {
    if (s == NULL) return;
    // hl_cell_release tears down the subscription and parent lists at zero.
    if (hl_cell_release(&s->cell)) hl_scalar_finalize(s);
}

int32_t hl_scalar_get_i32(HiLowScalar* s) {
    // Phase 6a: a placed (cross-process) cell reads its payload from the mapped
    // segment slot (acquire — axiom 3). Phase 5c: a shared (in-process) cell's
    // payload is read atomically (cross-thread visibility + no torn reads).
    // Non-shared: plain load, byte-identical to pre-5c. hl_scalar_get_i32 is an
    // out-of-line runtime call, so even a plain load inside a caller's loop is
    // re-read each iteration (no register hoist).
    if (s->cell.shm) {
        return hl_shm_load_i32(s->cell.shm);
    }
    if (s->cell.sub_lock) {
        return atomic_load_explicit((_Atomic int32_t*)&s->value.value.i32_val, memory_order_seq_cst);
    }
    return s->value.value.i32_val;
}

// Reference-payload slots (Phase 3e-α). The slot ADOPTS a +1 reference at
// construction and on every set (callers retain first when they hold a
// borrow); getters BORROW. The slot's release tears the payload down.
static HiLowScalar* hl_scalar_new_ref(HiLowValueType kind) {
    HiLowScalar* s = malloc(sizeof(HiLowScalar));
    hl_alloc_count++;
    s->cell.refcount = 1;
    s->cell.kind = HL_CELL_SCALAR;  // Phase 5c: typed-teardown dispatch
    s->cell.sub_lock = NULL;        // Phase 5c: not shared (overridden by hl_scalar_new_i32_shared)
    s->cell.shm = NULL;             // Phase 6a: not placed (overridden by hl_scalar_new_i32_placed)
    s->cell.watchers = NULL;
    s->cell.parents = NULL;
    s->cell.version = 0;
    s->cell.deep_watched = false;
    s->value.type = kind;
    return s;
}

HiLowScalar* hl_scalar_new_str(HiLowArray* v) {
    HiLowScalar* s = hl_scalar_new_ref(HL_VALUE_STR);
    s->value.value.str_val = v;
    return s;
}

HiLowScalar* hl_scalar_new_array_ref(HiLowArray* v) {
    HiLowScalar* s = hl_scalar_new_ref(HL_VALUE_ARRAY);
    s->value.value.arr_val = v;
    return s;
}

HiLowScalar* hl_scalar_new_object_ref(HiLowObject* v) {
    HiLowScalar* s = hl_scalar_new_ref(HL_VALUE_OBJECT);
    s->value.value.obj_val = v;
    return s;
}

HiLowArray* hl_scalar_get_str(HiLowScalar* s) {
    return s->value.value.str_val;
}

HiLowArray* hl_scalar_get_array_ref(HiLowScalar* s) {
    return s->value.value.arr_val;
}

HiLowObject* hl_scalar_get_object_ref(HiLowScalar* s) {
    return s->value.value.obj_val;
}

// Retargeting (Phase 3e-β, audit §5 item 10b steps 3–4). Called from the
// container set functions AFTER the store and BEFORE the slot's own fire:
// for each watcher FOLLOWING this slot (an HL_SLOT_FOLLOW node on the slot's
// cell), move that watcher's subscription nodes from the old value's cell to
// the new value's cell — unsubscribe old, subscribe new in collected order
// (hl_cell_subscribe appends, so relative fire order is preserved — §5 item
// 9). If any moved node is (deep), the new subtree is deep-marked (the 2d
// containment-add rule reused). Unconditional with respect to pause and
// stealth: retargeting moves NODES, not watcher state — a paused watcher's
// nodes still track the variable, and a stealth rebinding (store without
// notification) must still retarget. The old container is alive throughout:
// the caller holds its +1 until after the fire (step 6).
static void hl_slot_retarget(HiLowScalar* s, HiLowCell* old_cell) {
    HiLowCell* new_cell;
    bool new_is_object;
    switch (s->value.type) {
        case HL_VALUE_ARRAY:
            new_cell = &s->value.value.arr_val->cell;
            new_is_object = false;
            break;
        case HL_VALUE_OBJECT:
            new_cell = &s->value.value.obj_val->cell;
            new_is_object = true;
            break;
        default:
            return;  // strings/scalars: nothing to retarget
    }
    if (new_cell == old_cell) return;  // identity self-assignment

    // Walk the slot's OWN subscriber list for FOLLOW markers. Snapshot the
    // follower set first: hl_cell_subscribe below pushes backrefs, and the
    // slot list itself is never modified here, but keeping the discipline
    // uniform costs nothing.
    HiLowNodeSnap inline_buf[HL_NODE_SNAP_INLINE];
    HiLowNodeSnap* snap;
    size_t n = snapshot_cell_nodes(&s->cell, inline_buf, &snap);
    for (size_t i = 0; i < n; i++) {
        if (snap[i].modifier != HL_SLOT_FOLLOW) continue;
        HiLowWatcher* w = snap[i].watcher;
        if (w == NULL) continue;
        // Collect this watcher's nodes on the old cell, in list order —
        // Phase 3e-γ: only the nodes ATTRIBUTED to this slot (origin ==
        // &s->cell). Another followed variable holding the same container
        // keeps its own nodes; NULL-origin nodes (expression form) never
        // move.
        HiLowNodeSnap moved_inline[HL_NODE_SNAP_INLINE];
        HiLowNodeSnap* moved;
        size_t moved_n = 0, moved_cap = HL_NODE_SNAP_INLINE;
        moved = moved_inline;
        for (HiLowCellWatcher* node = old_cell->watchers; node != NULL; node = node->next) {
            if (node->watcher != w || node->origin != &s->cell) continue;
            if (moved_n == moved_cap) {
                moved_cap *= 2;
                if (moved == moved_inline) {
                    moved = malloc(moved_cap * sizeof(HiLowNodeSnap));
                    memcpy(moved, moved_inline, moved_n * sizeof(HiLowNodeSnap));
                } else {
                    moved = realloc(moved, moved_cap * sizeof(HiLowNodeSnap));
                }
            }
            moved[moved_n].modifier = node->modifier;
            moved[moved_n].body_fn = node->body_fn;
            moved[moved_n].env = node->env;
            moved[moved_n].watcher = node->watcher;
            moved[moved_n].origin = node->origin;
            moved_n++;
        }
        if (moved_n > 0) {
            // Origin-filtered removal: w's nodes attributed to OTHER slots
            // (aliased followed variables) stay on old_cell with their
            // backrefs intact.
            hl_cell_unsubscribe_watcher_origin(old_cell, w, &s->cell);
            for (size_t j = 0; j < moved_n; j++) {
                // Re-subscribe with the origin preserved — it is the slot's
                // own cell, constant across moves, so this slot's NEXT rebind
                // finds exactly these nodes wherever the payload then lives.
                hl_cell_subscribe(new_cell, moved[j].modifier, moved[j].body_fn, moved[j].env, w, moved[j].origin);
                if (moved[j].modifier == HL_ARR_DEEP) {
                    if (new_is_object) {
                        hl_object_mark_deep(s->value.value.obj_val);
                    } else {
                        hl_array_mark_deep(s->value.value.arr_val);
                    }
                }
            }
        }
        if (moved != moved_inline) free(moved);
    }
    if (snap != inline_buf) free(snap);
}

// The hl_cell_set family: store + equality check + notify. The store happens
// under stealth too — stealth suppresses only the notifications (the
// authoritative gate is inside hl_cell_notify; the outer check just skips
// the calls). CHANGED fires only when the value differed, then
// HL_SCALAR_ASSIGNED on every call — changed subscribers before assigned
// subscribers, the legacy firing block's order.
void hl_cell_set_i32(HiLowScalar* s, int32_t v) {
    // Phase 5c: a shared cell stores atomically and returns the prior value
    // (racy RMW for `x += 1` is two atomic ops — the prover's future warning,
    // not an error). Non-shared: plain load-then-store, byte-identical to pre-5c.
    // Shared cells skip the cell_has_audience fast path (racy read of the
    // subscriber list) and always route through hl_cell_notify, which takes the
    // subscriber lock and snapshots safely (empty list → fires nothing, cheap).
    int32_t old;
    bool placed = (s->cell.shm != NULL);
    bool shared = (s->cell.sub_lock != NULL);   // placed ⟹ shared (superset)
    if (placed) {
        // Phase 6a: store to the mapped slot and bump the epoch (release) — one
        // publication point (axiom 3). Nothing pulls the epoch cross-process
        // until 6b, but the write protocol is complete now.
        old = hl_shm_exchange_i32(s->cell.shm, v);
    } else if (shared) {
        old = atomic_exchange_explicit((_Atomic int32_t*)&s->value.value.i32_val, v, memory_order_seq_cst);
    } else {
        old = s->value.value.i32_val;
        s->value.value.i32_val = v;
    }
    bool changed = old != v;
    if (hl_stealth_depth == 0 && (shared || cell_has_audience(&s->cell))) {
        if (changed) {
            hl_cell_notify(&s->cell, HL_ARR_CHANGED, NULL);
        }
        hl_cell_notify(&s->cell, HL_SCALAR_ASSIGNED, NULL);
    }
}

// Reference-payload sets (Phase 3e-α). `v` arrives +1 (ADOPTED — callers
// retain a borrowed rhs first, so self-assignment reaches here at +2 and
// survives the old-release). (changed) fires iff unequal under the type's
// OWN equality — value equality for strings, identity for containers
// (audit §5 item 10a); (assigned) fires on every set. Store always; the
// old reference is released AFTER the store; stealth suppresses only the
// notifications.
static void hl_cell_set_ref_common(HiLowScalar* s, bool changed) {
    if (hl_stealth_depth == 0 && cell_has_audience(&s->cell)) {
        if (changed) {
            hl_cell_notify(&s->cell, HL_ARR_CHANGED, NULL);
        }
        hl_cell_notify(&s->cell, HL_SCALAR_ASSIGNED, NULL);
    }
}

void hl_cell_set_str(HiLowScalar* s, HiLowArray* v) {
    HiLowArray* old = s->value.value.str_val;
    bool changed = (old != v) && !hl_string_eq(old, v);
    s->value.value.str_val = v;
    // Phase 3e-β: release old AFTER the fire, deferred past any in-flight
    // walk — a body's borrowed snapshot of the old value must outlive the
    // body (strings have no nodes to retarget, but share this rule).
    hl_cell_set_ref_common(s, changed);
    hl_release_or_defer(hl_release_array_voidp, old);
}

void hl_cell_set_array_ref(HiLowScalar* s, HiLowArray* v) {
    HiLowArray* old = s->value.value.arr_val;
    bool changed = old != v;
    s->value.value.arr_val = v;
    // Phase 3e-β step 3/4: retarget followers' nodes old → new while old is
    // still alive, BEFORE the slot's own changed/assigned fire (step 5).
    hl_slot_retarget(s, &old->cell);
    hl_cell_set_ref_common(s, changed);
    // Step 6: release old LAST, deferred past any in-flight walk (a body
    // rebinding its own followed variable runs inside a walk of old's cell).
    hl_release_or_defer(hl_release_array_voidp, old);
}

void hl_cell_set_object_ref(HiLowScalar* s, HiLowObject* v) {
    HiLowObject* old = s->value.value.obj_val;
    bool changed = old != v;
    s->value.value.obj_val = v;
    hl_slot_retarget(s, &old->cell);
    hl_cell_set_ref_common(s, changed);
    hl_release_or_defer(hl_release_object_voidp, old);
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
        case HL_VALUE_ARRAY: return TYPE_ARRAY;
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
// Phase 5b: in threaded mode these are `_Atomic int` so the leak-check counters
// (incremented from every thread via the unchanged generated `hl_alloc_count++`)
// don't lose updates to a data race — `++` on an _Atomic object is an atomic RMW
// (C11), so the generated main.c bytes are identical (`hl_alloc_count++` either
// way) while the arithmetic becomes race-free. The final compare in main runs
// after hl_async_join_all, so the values are consistent.
#ifdef HILOW_THREADED
_Atomic int hl_alloc_count = 0;
_Atomic int hl_free_count = 0;
#else
int hl_alloc_count = 0;
int hl_free_count = 0;
#endif

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

// Refcounting operations (Phase 8b; cell-based as of Phase 2e — the cell
// header owns the refcount, hl_cell_release tears down the subscription and
// parent lists, and the object does its type-specific teardown on zero,
// mirroring hl_array_release)
void hl_object_retain(HiLowObject* obj) {
    if (obj) {
        hl_cell_retain(&obj->cell);
    }
}

// Phase 5c: the object teardown after the last cell release — reachable via
// hl_object_release (same-owner) OR hl_cell_release_full (cross-thread).
static void hl_object_finalize(HiLowObject* obj) {
    // Step 1: Handle weak properties first - unregister from targets, no release
    for (size_t i = 0; i < obj->property_count; i++) {
        if (obj->properties[i].is_weak && obj->properties[i].value.type == HL_VALUE_OBJECT && obj->properties[i].value.value.obj_val) {
            hl_object_weak_unregister(obj->properties[i].value.value.obj_val, obj, i);
        }
    }

    // Step 2: Release strong properties normally. Phase 2e: holder
    // death drops its containments' backrefs (symmetric unlink)
    // before releasing each container value.
    for (size_t i = 0; i < obj->property_count; i++) {
        if (!obj->properties[i].is_weak) {
            object_property_removed(obj, &obj->properties[i].value);
            if (obj->properties[i].value.type == HL_VALUE_OBJECT && obj->properties[i].value.value.obj_val) {
                hl_object_release(obj->properties[i].value.value.obj_val);
            } else if (obj->properties[i].value.type == HL_VALUE_FUNCTION && obj->properties[i].value.value.fn_val) {
                hl_function_release(obj->properties[i].value.value.fn_val);
            } else if (obj->properties[i].value.type == HL_VALUE_ARRAY && obj->properties[i].value.value.arr_val) {
                hl_array_release(obj->properties[i].value.value.arr_val);
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

void hl_object_release(HiLowObject* obj) {
    if (obj) {
        if (hl_cell_release(&obj->cell)) hl_object_finalize(obj);
    }
}

void hl_function_retain(HiLowFunction* fn) {
    if (fn) {
        HL_RC_INC(fn->refcount);
    }
}

void hl_function_release(HiLowFunction* fn) {
    if (fn) {
        if (HL_RC_DEC(fn->refcount) == 0) {
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
    bool existed = (existing != NULL);
    size_t prop_index;
    if (existing) {
        prop_index = (size_t)(existing - obj->properties);
        // Phase 2e: an overwritten strong container value loses its
        // containment backref before release. The weak store itself adds NO
        // parent link — weak is observation without ownership, and deep
        // propagation does not cross weak references.
        if (!existing->is_weak) {
            object_property_removed(obj, &existing->value);
        }
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
        } else if (existing->value.type == HL_VALUE_ARRAY && existing->value.value.arr_val) {
            hl_array_release(existing->value.value.arr_val);
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

    // Phase 2e: a weak store is still a mutation of the HOLDER — same event
    // mapping as set_property (weakness affects containment, not firing)
    if (hl_stealth_depth == 0 && cell_has_audience(&obj->cell)) {
        hl_cell_notify(&obj->cell, existed ? HL_ARR_CHANGED : HL_ARR_ADDED, NULL);
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
    if (obj) HL_RC_INC(obj->cell.refcount);
    return obj;
}

HiLowFunction* hl_function_ref(HiLowFunction* fn) {
    if (fn) HL_RC_INC(fn->refcount);
    return fn;
}

HiLowArray* hl_array_ref(HiLowArray* arr) {
    if (arr) HL_RC_INC(arr->cell.refcount);
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

// Phase 2a (audit requirement 0): these six were placeholders returning 0,
// and they ARE reachable from valid programs (typecheck accepts `i64?` etc.,
// codegen's return-wrap catch-all mis-kinds the value as HL_OPT_I32, and
// print/f-string dispatch lands here) — silently printing 0/false. Until the
// optional runtime grows real payload kinds for these types, reaching one is
// an internal error and must be LOUD, not silently wrong.
static void hl_optional_unwrap_unimplemented(const char* type_name) {
    fprintf(stderr, "internal error: optional unwrap for %s not implemented\n", type_name);
    abort();
}

int64_t hl_optional_unwrap_i64(HiLowOptional* opt) {
    (void)opt;
    hl_optional_unwrap_unimplemented("i64");
    return 0;  // unreachable
}

uint32_t hl_optional_unwrap_u32(HiLowOptional* opt) {
    (void)opt;
    hl_optional_unwrap_unimplemented("u32");
    return 0;  // unreachable
}

uint64_t hl_optional_unwrap_u64(HiLowOptional* opt) {
    (void)opt;
    hl_optional_unwrap_unimplemented("u64");
    return 0;  // unreachable
}

float hl_optional_unwrap_f32(HiLowOptional* opt) {
    (void)opt;
    hl_optional_unwrap_unimplemented("f32");
    return 0.0f;  // unreachable
}

double hl_optional_unwrap_f64(HiLowOptional* opt) {
    (void)opt;
    hl_optional_unwrap_unimplemented("f64");
    return 0.0;  // unreachable
}

bool hl_optional_unwrap_bool(HiLowOptional* opt) {
    (void)opt;
    hl_optional_unwrap_unimplemented("bool");
    return false;  // unreachable
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
    hl_thread_safepoint();  // Phase 5a: allocation safe point (dormant single-threaded)
    HiLowArray* arr = malloc(sizeof(HiLowArray));
    hl_alloc_count++;

    // Cell header (Phase 2a)
    arr->cell.refcount = 1;
    arr->cell.kind = HL_CELL_ARRAY;  // Phase 5c: typed-teardown dispatch
    arr->cell.sub_lock = NULL;       // Phase 5c: not shared
    arr->cell.shm = NULL;            // Phase 6a: not placed (arrays are never placeable)
    arr->cell.watchers = NULL;
    arr->cell.parents = NULL;      // dead until 2d
    arr->cell.version = 0;         // dead until later phases
    arr->cell.deep_watched = false; // dead until 2d

    arr->length = 0;
    arr->capacity = initial_capacity;
    arr->elem_size = elem_size;
    arr->data = malloc(elem_size * initial_capacity);
    arr->retain_fn = retain_fn;
    arr->release_fn = release_fn;

    return arr;
}

void hl_array_retain(HiLowArray* arr) {
    if (arr) {
        hl_cell_retain(&arr->cell);
    }
}

// Phase 5c: the array teardown after the last cell release — reachable via
// hl_array_release (same-owner) OR hl_cell_release_full (cross-thread).
static void hl_array_finalize(HiLowArray* arr) {
    // Release all elements if this is an object array
    if (arr->release_fn != NULL) {
        for (size_t i = 0; i < arr->length; i++) {
            void* slot = (char*)arr->data + (i * arr->elem_size);
            // Phase 2d: parent death drops its containments' backrefs
            // (symmetric unlink), before releasing the element
            array_element_removed(arr, slot);
            arr->release_fn(*(void**)slot);
        }
    }
    free(arr->data);
    free(arr);
    hl_free_count++;
}

void hl_array_release(HiLowArray* arr) {
    if (!arr) return;

    // hl_cell_release handles refcount + subscription/parent list teardown
    // (unlinking watcher backrefs); we do the array-specific teardown on zero.
    if (hl_cell_release(&arr->cell)) hl_array_finalize(arr);
}

// Phase 2d helpers (generalized to object elements in Phase 2e). Element
// kind detection: codegen passes exactly hl_array_retain for nested-array
// elements and hl_object_retain for object elements (strings/primitives
// NULL), so retain_fn identity is the discriminator.
static bool elems_are_arrays(const HiLowArray* arr) {
    return arr->retain_fn == (hl_elem_fn)hl_array_retain;
}

static bool elems_are_objects(const HiLowArray* arr) {
    return arr->retain_fn == (hl_elem_fn)hl_object_retain;
}

// A mutation needs notification when the cell has direct subscribers OR a
// deep-watched ancestor may exist above it. Mutators use this only to skip
// delta construction; hl_cell_notify remains the authoritative gate.
static bool cell_has_audience(const HiLowCell* c) {
    return c->watchers != NULL || (c->deep_watched && c->parents != NULL);
}

void hl_array_mark_deep(HiLowArray* arr) {
    if (arr->cell.deep_watched) return;  // marked implies subtree marked
    arr->cell.deep_watched = true;
    if (elems_are_arrays(arr)) {
        for (size_t i = 0; i < arr->length; i++) {
            void* slot = (char*)arr->data + (i * arr->elem_size);
            hl_array_mark_deep(*(HiLowArray**)slot);
        }
    } else if (elems_are_objects(arr)) {
        for (size_t i = 0; i < arr->length; i++) {
            void* slot = (char*)arr->data + (i * arr->elem_size);
            hl_object_mark_deep(*(HiLowObject**)slot);
        }
    }
}

// Phase 2e: object counterpart, mutually recursive with hl_array_mark_deep.
// Recurses STRONG object/array properties only — weak properties are
// skipped, because deep propagation does not cross weak references (the
// marked-implies-subtree-marked invariant is over the strong-reachable
// subtree).
void hl_object_mark_deep(HiLowObject* obj) {
    if (obj->cell.deep_watched) return;
    obj->cell.deep_watched = true;
    for (size_t i = 0; i < obj->property_count; i++) {
        if (obj->properties[i].is_weak) continue;
        if (obj->properties[i].value.type == HL_VALUE_OBJECT && obj->properties[i].value.value.obj_val) {
            hl_object_mark_deep(obj->properties[i].value.value.obj_val);
        } else if (obj->properties[i].value.type == HL_VALUE_ARRAY && obj->properties[i].value.value.arr_val) {
            hl_array_mark_deep(obj->properties[i].value.value.arr_val);
        }
    }
}

// Containment-add bookkeeping (push/insert/set store paths): link the child
// back to this parent and pull it into an already-deep-watched subtree.
static void array_element_stored(HiLowArray* arr, void* slot) {
    if (elems_are_arrays(arr)) {
        HiLowArray* child = *(HiLowArray**)slot;
        hl_cell_add_parent(&child->cell, &arr->cell);
        if (arr->cell.deep_watched) {
            hl_array_mark_deep(child);
        }
    } else if (elems_are_objects(arr)) {
        HiLowObject* child = *(HiLowObject**)slot;
        hl_cell_add_parent(&child->cell, &arr->cell);
        if (arr->cell.deep_watched) {
            hl_object_mark_deep(child);
        }
    }
}

// Containment-remove bookkeeping (pop/remove/set-overwrite/clear/teardown):
// drop exactly one backref. The deep_watched bit is deliberately left set
// (stale bit = wasted walk, never a wrong fire).
static void array_element_removed(HiLowArray* arr, void* slot) {
    if (elems_are_arrays(arr)) {
        HiLowArray* child = *(HiLowArray**)slot;
        hl_cell_remove_parent(&child->cell, &arr->cell);
    } else if (elems_are_objects(arr)) {
        HiLowObject* child = *(HiLowObject**)slot;
        hl_cell_remove_parent(&child->cell, &arr->cell);
    }
}

// Phase 2e: object-side containment bookkeeping — the property-table
// counterpart of array_element_stored/removed. Acts only on STRONG
// object/array values (the callers guard is_weak; weak stores never link).
static void object_property_stored(HiLowObject* holder, const HiLowValue* v) {
    if (v->type == HL_VALUE_OBJECT && v->value.obj_val) {
        hl_cell_add_parent(&v->value.obj_val->cell, &holder->cell);
        if (holder->cell.deep_watched) {
            hl_object_mark_deep(v->value.obj_val);
        }
    } else if (v->type == HL_VALUE_ARRAY && v->value.arr_val) {
        hl_cell_add_parent(&v->value.arr_val->cell, &holder->cell);
        if (holder->cell.deep_watched) {
            hl_array_mark_deep(v->value.arr_val);
        }
    }
}

static void object_property_removed(HiLowObject* holder, const HiLowValue* v) {
    if (v->type == HL_VALUE_OBJECT && v->value.obj_val) {
        hl_cell_remove_parent(&v->value.obj_val->cell, &holder->cell);
    } else if (v->type == HL_VALUE_ARRAY && v->value.arr_val) {
        hl_cell_remove_parent(&v->value.arr_val->cell, &holder->cell);
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

    // Phase 2d: containment backref + deep-bit propagation for array elements
    array_element_stored(arr, dest);

    // Phase 2c: one firing path — construct a value delta, notify the cell.
    // The guard only skips delta construction; stealth is authoritatively
    // checked inside hl_cell_notify.
    if (hl_stealth_depth == 0 && cell_has_audience(&arr->cell)) {
        HiLowDelta* d = hl_delta_new_elem(HL_ARR_ADDED, dest, arr->elem_size,
                                          arr->retain_fn, arr->release_fn);
        hl_cell_notify(&arr->cell, HL_ARR_ADDED, d);
        hl_delta_release(d);
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

    // Phase 2d: the element left this container — drop one backref
    array_element_removed(arr, removed_slot);

    // Phase 2c: one firing path
    if (hl_stealth_depth == 0 && cell_has_audience(&arr->cell)) {
        HiLowDelta* d = hl_delta_new_elem(HL_ARR_REMOVED, removed_slot, arr->elem_size,
                                          arr->retain_fn, arr->release_fn);
        hl_cell_notify(&arr->cell, HL_ARR_REMOVED, d);
        hl_delta_release(d);
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

    // Phase 2d: the old element leaves this container — drop one backref
    // (before release, while the pointer is still guaranteed valid)
    array_element_removed(arr, dest);

    // Release the old element if this is an object array
    if (arr->release_fn != NULL) {
        arr->release_fn(*(void**)dest);
    }

    memcpy(dest, elem, arr->elem_size);

    // Retain the new element if this is an object array
    if (arr->retain_fn != NULL) {
        arr->retain_fn(*(void**)dest);
    }

    // Phase 2d: containment backref + deep-bit propagation for the new element
    array_element_stored(arr, dest);

    // Phase 2c: one firing path. set fires CHANGED only (no size change);
    // payload-less, so no delta is constructed.
    if (hl_stealth_depth == 0 && cell_has_audience(&arr->cell)) {
        hl_cell_notify(&arr->cell, HL_ARR_CHANGED, NULL);
    }
}

void hl_array_remove(HiLowArray* arr, size_t index, void* out) {
    // Bounds check
    if (index >= arr->length) {
        fprintf(stderr, "Runtime error: remove() index %zu out of bounds (length %zu)\n",
                index, arr->length);
        exit(1);
    }

    // Copy the removed element into the caller-owned destination before
    // shifting (Phase 2c: no static buffer — no size cap, re-entrant).
    // Note: the caller now owns the object reference; no release here.
    void* removed_slot = (char*)arr->data + (index * arr->elem_size);
    memcpy(out, removed_slot, arr->elem_size);

    // Phase 2d: the element left this container — drop one backref
    array_element_removed(arr, out);

    // Shift elements [index+1 .. length-1] down by one
    if (index < arr->length - 1) {
        void* dest = (char*)arr->data + (index * arr->elem_size);
        void* src = (char*)arr->data + ((index + 1) * arr->elem_size);
        size_t bytes_to_move = (arr->length - index - 1) * arr->elem_size;
        memmove(dest, src, bytes_to_move);
    }

    // Decrement length
    arr->length--;

    // Phase 2c: one firing path
    if (hl_stealth_depth == 0 && cell_has_audience(&arr->cell)) {
        HiLowDelta* d = hl_delta_new_elem(HL_ARR_REMOVED, out, arr->elem_size,
                                          arr->retain_fn, arr->release_fn);
        hl_cell_notify(&arr->cell, HL_ARR_REMOVED, d);
        hl_delta_release(d);
    }
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

    // Phase 2d: containment backref + deep-bit propagation for array elements
    array_element_stored(arr, dest);

    // Phase 2c: one firing path
    if (hl_stealth_depth == 0 && cell_has_audience(&arr->cell)) {
        HiLowDelta* d = hl_delta_new_elem(HL_ARR_ADDED, dest, arr->elem_size,
                                          arr->retain_fn, arr->release_fn);
        hl_cell_notify(&arr->cell, HL_ARR_ADDED, d);
        hl_delta_release(d);
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
        // Still fire watchers with delta (documented choice: still fire).
        // Phase 2c: one firing path — the 2-arg env-dropping cast (§3.4(a))
        // is gone; bodies get the full (env, cell, delta) call.
        if (hl_stealth_depth == 0 && cell_has_audience(&arr->cell)) {
            HiLowDelta* d = hl_delta_new_moved(from, to);
            hl_cell_notify(&arr->cell, HL_ARR_MOVED, d);
            hl_delta_release(d);
        }
        return;
    }

    // Capture the element at 'from' index in a local heap scratch
    // (Phase 2c: no static buffer — no size cap, re-entrant).
    void* from_slot = (char*)arr->data + (from * arr->elem_size);
    void* moved_elem = malloc(arr->elem_size);
    memcpy(moved_elem, from_slot, arr->elem_size);

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
    memcpy(to_slot, moved_elem, arr->elem_size);
    free(moved_elem);

    // No retain/release - same element, refcount unchanged

    // Phase 2c: one firing path (was the second §3.4(a) 2-arg cast site)
    if (hl_stealth_depth == 0 && cell_has_audience(&arr->cell)) {
        HiLowDelta* d = hl_delta_new_moved(from, to);
        hl_cell_notify(&arr->cell, HL_ARR_MOVED, d);
        hl_delta_release(d);
    }
}

// Phase 10-ε-γ + clear: Array clear empties the array
void hl_array_clear(HiLowArray* arr) {
    if (!arr) return;

    // Release all elements if this is an object array (same loop as hl_array_release)
    if (arr->release_fn != NULL) {
        for (size_t i = 0; i < arr->length; i++) {
            void* slot = (char*)arr->data + (i * arr->elem_size);
            // Phase 2d: drop one backref per containment, before release
            array_element_removed(arr, slot);
            arr->release_fn(*(void**)slot);
        }
    }

    // Reset length to 0, but keep the buffer for reuse (don't free arr->data)
    arr->length = 0;

    // Phase 2c: one firing path. clear fires CHANGED only —
    // ADDED/REMOVED/MOVED stay deliberately silent, which the event-match
    // rule in hl_cell_notify gives for free.
    if (hl_stealth_depth == 0 && cell_has_audience(&arr->cell)) {
        hl_cell_notify(&arr->cell, HL_ARR_CHANGED, NULL);
    }
}

// Array watcher registration (Phase 10-ε-α)
// Array watcher registration/unregistration moved onto the cell (Phase 2a):
// hl_watcher_new_subscribed / hl_cell_unsubscribe_env, defined with the cell
// operations above.

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
