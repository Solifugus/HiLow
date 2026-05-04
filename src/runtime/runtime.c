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

// Object support implementation (Phase 7a)

// Prototype chain support (Phase 7b)
#define MAX_PROTO_DEPTH 100

// Forward declaration for prototype helper
static HiLowObject* hl_object_get_proto(HiLowObject* obj);

HiLowObject* hl_object_new(void) {
    HiLowObject* obj = malloc(sizeof(HiLowObject));
    obj->properties = NULL;
    obj->property_count = 0;
    obj->property_capacity = 0;
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
        obj->property_capacity = new_capacity;
    }
}

// Helper function to add or update a property
static void set_property(HiLowObject* obj, const char* key, HiLowValue value) {
    Property* existing = find_property(obj, key);
    if (existing) {
        existing->value = value;
    } else {
        ensure_capacity(obj);
        obj->properties[obj->property_count].key = strdup(key);  // Duplicate the key string
        obj->properties[obj->property_count].value = value;
        obj->property_count++;
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

void hl_object_set_str(HiLowObject* obj, const char* key, const char* value) {
    HiLowValue val = { .type = HL_VALUE_STR, .value.str_val = strdup(value) };  // Duplicate string
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

char* hl_object_get_str(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop) {
            if (prop->value.type == HL_VALUE_STR) {
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

// Helper function to get the proto property as an object (Phase 7b)
static HiLowObject* hl_object_get_proto(HiLowObject* obj) {
    Property* proto_prop = find_property(obj, "proto");
    if (proto_prop && proto_prop->value.type == HL_VALUE_OBJECT) {
        return proto_prop->value.value.obj_val;
    }
    return NULL;
}

// Phase 7c-i: Object is check - walks the prototype chain
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

// Phase 7c-i: Function object setters and getters
void hl_object_set_function(HiLowObject* obj, const char* key, void* value) {
    // For now, store function pointer as generic pointer
    // TODO: Implement proper closure representation
    HiLowValue val = { .type = HL_VALUE_OBJECT, .value.obj_val = (HiLowObject*)value };
    set_property(obj, key, val);
}

void* hl_object_get_function(HiLowObject* obj, const char* key) {
    HiLowObject* current = obj;
    int depth = 0;

    while (current && depth < MAX_PROTO_DEPTH) {
        Property* prop = find_property(current, key);
        if (prop && prop->value.type == HL_VALUE_OBJECT) {
            return (void*)prop->value.value.obj_val;
        }

        // Property not found on this object - check prototype
        HiLowObject* proto = hl_object_get_proto(current);
        if (!proto) break;
        current = proto;
        depth++;
    }

    if (depth >= MAX_PROTO_DEPTH) {
        fprintf(stderr, "Maximum prototype chain depth reached\n");
        exit(1);
    }

    // Property not found
    fprintf(stderr, "property '%s' not found\n", key);
    exit(1);
}