#ifndef HILOW_RUNTIME_H
#define HILOW_RUNTIME_H

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>  // for malloc
#include <string.h>  // for strcat
#include <stdio.h>   // for sprintf

// Forward declarations
typedef struct HiLowArray HiLowArray;

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
HiLowUnknown* hl_unknown_new(HiLowArray* reason);
HiLowUnknown* hl_unknown_new_with_options(HiLowArray* reason, const char** options, int options_count);

// Unknown memory management
void hl_unknown_retain(HiLowUnknown* unknown);
void hl_unknown_release(HiLowUnknown* unknown);

// Unknown property access
HiLowArray* hl_unknown_get_reason(HiLowUnknown* unknown);
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
    HL_OPT_OBJECT,   // Phase 1.5e: weak property reads yield object-or-unknown
    // Add others as needed by tests
} HiLowOptionalKind;

// Forward declaration: HiLowOptional can carry an object payload (Phase 1.5e)
struct HiLowObject;

typedef struct HiLowOptional {
    int refcount;
    HiLowOptionalKind kind;
    union {
        int32_t i32_val;
        HiLowArray* str_val;
        HiLowUnknown* unk_val;
        HiLowTime time_val;
        HiLowDuration duration_val;
        HiLowMoney money_val;
        struct HiLowObject* obj_val;  // owned (+1) when kind == HL_OPT_OBJECT
    } payload;
} HiLowOptional;

// Unknown type checking (updated for HiLowOptional)
bool hl_is_unknown(HiLowOptional* opt);

// Optional constructor functions
HiLowOptional* hl_optional_new_i32(int32_t v);
HiLowOptional* hl_optional_new_string(HiLowArray* s);
HiLowOptional* hl_optional_new_unknown(HiLowUnknown* u);
HiLowOptional* hl_optional_new_time(HiLowTime t);
HiLowOptional* hl_optional_new_duration(HiLowDuration d);
HiLowOptional* hl_optional_new_money(HiLowMoney m);

// Optional memory management
void hl_optional_retain(HiLowOptional* opt);
void hl_optional_release(HiLowOptional* opt);

// F-string format helpers
HiLowArray* hl_format_binary(unsigned long long value);
HiLowArray* hl_format_center(HiLowArray* value, int width);

// Function value support (Phase 7c-β)
typedef struct HiLowFunction {
    int refcount;          // Reference count (Phase 8b)
    void* fn_ptr;          // pointer to the C function
    void* env;             // captured environment; NULL for non-closures
    void (*env_dtor)(void*); // releases heap fields inside env before free; NULL if none
} HiLowFunction;

// ---------------------------------------------------------------------------
// The cell header (Phase 2a, cell-redesign brief "Core decision"): embedded as
// the FIRST member of every watchable heap value. Arrays in this phase;
// objects, strings, and boxed scalars embed the same header in later phases.
// Nothing array-specific lives in the header, its node structs, or the cell
// operations.
// ---------------------------------------------------------------------------

// Element function pointer type (Phase C; used by arrays and deltas)
typedef void (*hl_elem_fn)(void*);

// One mutation notification (Phase 2c). Self-contained and heap-owned —
// QUEUEABLE: owns a copy of its payload bytes and (for object arrays) a
// retained element reference; no pointers into array storage, caller
// stacks, or static buffers. It can be stored and released at any later
// time from any site via hl_delta_release. Phase 5 (queues) changes only
// WHEN and WHERE bodies run and who calls hl_delta_release — not this
// shape.
typedef struct HiLowDelta {
    int event;                  // HL_ARR_ADDED / REMOVED / CHANGED / MOVED
    void* payload;              // heap-owned copy of the element bytes, or NULL
    size_t payload_size;        // elem_size for element payloads, else 0
    hl_elem_fn payload_release; // element release fn (object arrays), else NULL
    size_t from, to;            // MOVED only, else 0
} HiLowDelta;

// Heap-copies elem_bytes; when retain_fn != NULL (object arrays) retains the
// element reference. release_fn is stored for hl_delta_release.
HiLowDelta* hl_delta_new_elem(int event, const void* elem_bytes, size_t elem_size,
                              hl_elem_fn retain_fn, hl_elem_fn release_fn);
HiLowDelta* hl_delta_new_moved(size_t from, size_t to);
// Releases the object payload (if any), frees the byte copy, frees the delta.
void hl_delta_release(HiLowDelta* d);

// One subscription: a watcher value attached to a cell for one modifier.
// body_fn is a HiLowWatcherBody (one firing ABI as of Phase 2c).
typedef struct HiLowCellWatcher {
    int modifier;                    // HL_ARR_ADDED / REMOVED / CHANGED / MOVED
    void* body_fn;
    void* env;                       // borrowed from the owning watcher (Phase 2b)
    struct HiLowWatcher* watcher;    // gating (active/ended) + identity; NOT owned
    struct HiLowCell* origin;        // Phase 3e-γ: the SLOT cell whose follow
                                     // created this node (decl-form container
                                     // content subscriptions); NULL for every
                                     // other node. hl_slot_retarget moves only
                                     // nodes whose origin is the rebinding
                                     // slot; NULL-origin nodes never move.
    struct HiLowCellWatcher* next;
} HiLowCellWatcher;

// Parent-list node (live as of Phase 2d): one NON-OWNING entry per
// containment of this cell's value inside a parent container. The same child
// stored twice in one parent gets two entries; each removal removes exactly
// one. Whichever side dies first unlinks (containment implies the parent
// holds a refcount, so a contained child cannot die; parent teardown drops
// its containments' entries).
typedef struct HiLowCellParent {
    struct HiLowCell* parent;        // NOT owned
    struct HiLowCellParent* next;
} HiLowCellParent;

typedef struct HiLowCell {
    int refcount;
    HiLowCellWatcher* watchers;      // subscription list (live as of Phase 2a)
    HiLowCellParent* parents;        // containment backrefs (live as of Phase 2d)
    uint64_t version;                // deep-walk epoch stamp (Phase 2d): visited
                                     // marking for cycle/diamond termination
    bool deep_watched;               // set when this cell or any ancestor has a
                                     // (deep) subscriber (Phase 2d); stale-true
                                     // after unsubscribe is allowed (wasted
                                     // walk, never a wrong fire)
} HiLowCell;

// Watcher value support (Phase 10-δ-α; subscription backrefs added Phase 2a)
// A cell→watcher node and its watcher→cell backref are BOTH non-owning;
// whichever side dies first unlinks itself from the other (hl_cell_release /
// hl_watcher_release). No refcount cycle, no dangling pointer either way.
typedef struct HiLowWatcherSub {
    struct HiLowCell* cell;          // NOT owned
    struct HiLowWatcherSub* next;
} HiLowWatcherSub;

typedef struct HiLowWatcher {
    int refcount;          // Reference count for memory management
    bool active;           // Whether the watcher is currently active
    bool ended;            // Whether the watcher has been permanently ended
    HiLowWatcherSub* subs; // cells this watcher is subscribed on (Phase 2a)
    void* env;             // captured environment, OWNED: released on final
                           // release (Phase 2b). The watcher's refcount is the
                           // env's refcount — envs are never shared.
    void (*env_dtor)(void*); // Phase 3b: env slots RETAIN their cells (escape
                           // soundness); the generated dtor releases each
                           // cell. The runtime frees the env itself either
                           // way; NULL means no cells to release.
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
    HL_VALUE_MONEY,
    HL_VALUE_ARRAY    // Phase 2e: array-valued properties (strong, container cell)
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
        HiLowArray* str_val;
        HiLowArray* arr_val;   // Phase 2e: same shape as str_val, distinct tag
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

// Object representation (heap-allocated with property table).
// Phase 2e: the cell header replaced the bare refcount — objects are the
// second container in the cell model (arrays joined in Phase 2a).
typedef struct HiLowObject {
    HiLowCell cell;            // cell header — MUST be first member (Phase 2e)
    Property* properties;
    size_t property_count;
    size_t property_capacity;
    struct WeakRef* weak_refs; // linked list of weak references to this object
} HiLowObject;

// Weak reference tracking (Phase 8c; reworked in Phase 1.5c). Keyed on
// (holder, property index) — stable across property-array reallocs, unlike a
// raw slot address.
typedef struct WeakRef {
    HiLowObject* holder;     // object holding the weak property
    size_t prop_index;       // index of the weak property in holder->properties
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
void hl_object_set_str(HiLowObject* obj, const char* key, HiLowArray* value);
void hl_object_set_object(HiLowObject* obj, const char* key, HiLowObject* value);
void hl_object_set_function(HiLowObject* obj, const char* key, HiLowFunction* value);
void hl_object_set_array(HiLowObject* obj, const char* key, HiLowArray* value);   // Phase 2e

int32_t hl_object_get_i32(HiLowObject* obj, const char* key);
int64_t hl_object_get_i64(HiLowObject* obj, const char* key);
uint32_t hl_object_get_u32(HiLowObject* obj, const char* key);
uint64_t hl_object_get_u64(HiLowObject* obj, const char* key);
float hl_object_get_f32(HiLowObject* obj, const char* key);
double hl_object_get_f64(HiLowObject* obj, const char* key);
bool hl_object_get_bool(HiLowObject* obj, const char* key);
HiLowArray* hl_object_get_str(HiLowObject* obj, const char* key);
HiLowObject* hl_object_get_object(HiLowObject* obj, const char* key);
HiLowFunction* hl_object_get_function(HiLowObject* obj, const char* key);
HiLowArray* hl_object_get_array(HiLowObject* obj, const char* key);   // Phase 2e: borrow, proto-chain walk

// Function value operations (Phase 7c-β)
HiLowFunction* hl_function_new(void* fn_ptr);
HiLowFunction* hl_function_new_with_env(void* fn_ptr, void* env);
HiLowFunction* hl_function_new_with_env_dtor(void* fn_ptr, void* env, void (*env_dtor)(void*));

// Watcher value operations (Phase 10-δ-α)
HiLowWatcher* hl_watcher_new(void);
void hl_watcher_retain(HiLowWatcher* w);
void hl_watcher_release(HiLowWatcher* w);
void hl_watcher_pause(HiLowWatcher* w);
void hl_watcher_resume(HiLowWatcher* w);
void hl_watcher_end(HiLowWatcher* w);
bool hl_watcher_is_active(HiLowWatcher* w);

// The one watcher-body ABI (Phase 2c): env-first, for every modifier. Bodies
// borrow cell, delta, and the payload for the duration of the call; anything
// kept beyond it must be copied/retained by the body. `cell` is the cell of
// the container that mutated (containers embed the cell as first member).
typedef void (*HiLowWatcherBody)(void* env, HiLowCell* cell, const HiLowDelta* delta);

// Cell operations (Phase 2a). All take HiLowCell* — never a container type.
// hl_cell_notify (the one firing path) landed in Phase 2c; the parent walk is 2d.
void hl_cell_retain(HiLowCell* c);
// Returns true when the refcount hit zero: the CALLER (the typed container)
// then does its type-specific teardown and free. On zero this tears down the
// subscription list (unlinking each node's backref from its watcher) and the
// parent list.
bool hl_cell_release(HiLowCell* c);
// Subscribe w to c for one modifier: appends a node AND records the backref
// in w->subs. Runtime-internal (called from hl_watcher_new_subscribed[_origins]
// and hl_slot_retarget — never emitted by codegen). origin is the slot cell
// this subscription is attributed to for retargeting (Phase 3e-γ), NULL for
// non-followed subscriptions.
void hl_cell_subscribe(HiLowCell* c, int modifier, void* body_fn, void* env, HiLowWatcher* w, HiLowCell* origin);
// Remove ALL of w's nodes from c and w's backrefs to c. Idempotent.
void hl_cell_unsubscribe_watcher(HiLowCell* c, HiLowWatcher* w);
// Phase 3e-γ: remove only w's nodes on c whose origin matches (one backref
// dropped per removed node — nodes with other origins keep theirs). Used by
// hl_slot_retarget; teardown paths keep the unfiltered variant.
void hl_cell_unsubscribe_watcher_origin(HiLowCell* c, HiLowWatcher* w, HiLowCell* origin);
// The ONE firing path (Phase 2c; deep walk added 2d). No-op under stealth
// (which therefore suppresses deep fires too). One walk of c->watchers in
// list order; a node fires iff its watcher is active and not ended AND (its
// modifier == event OR == HL_ARR_CHANGED OR == HL_ARR_DEEP) — every mutation
// event is also a change, and a (deep) subscriber on the mutated cell itself
// fires for every mutation. Then, only when c->deep_watched and c has
// parents, the parent walk: collect the unique ancestor set (epoch-stamped
// via cell->version — diamonds and any future cycles terminate), then fire
// ONLY each ancestor's HL_ARR_DEEP nodes with the SAME delta (one delta per
// mutation) and the ancestor's own cell (the body binds the subscribed
// variable's current value). The delta is borrowed by the callee; the caller
// releases it after this returns. May be NULL for payload-less
// CHANGED-only notifications.
void hl_cell_notify(HiLowCell* c, int event, const HiLowDelta* delta);
// Containment backref maintenance (Phase 2d). remove_parent removes exactly
// ONE matching entry (duplicate containments carry duplicate entries).
void hl_cell_add_parent(HiLowCell* child, HiLowCell* parent);
void hl_cell_remove_parent(HiLowCell* child, HiLowCell* parent);

// Watcher construction that registers by construction (Phase 2a): creates the
// watcher AND subscribes it to n (HiLowCell*, int modifier) varargs pairs.
// The ONLY way generated code attaches a watcher — there is no separate
// register call for codegen to forget. Phase 3b: env_dtor releases the env's
// retained cells on final release; the runtime then frees the env.
HiLowWatcher* hl_watcher_new_subscribed(void* body_fn, void* env, void (*env_dtor)(void*), int n, ...);
// Phase 3e-γ variant for decl-form watchers with followed variables: varargs
// are n (HiLowCell*, int modifier, HiLowCell* origin) TRIPLES — origin is the
// slot cell a container-content subscription is attributed to (NULL for slot
// (changed)/(assigned) subscriptions and FOLLOW markers). Emitted only when
// the watcher follows at least one variable; everything else keeps the pairs
// symbol above.
HiLowWatcher* hl_watcher_new_subscribed_origins(void* body_fn, void* env, void (*env_dtor)(void*), int n, ...);

// Boxed scalar (Phase 3b): a watched scalar variable lowered to a cell.
// Cell header first (all notify/subscribe/parent machinery applies unchanged);
// the payload is HiLowValue — the ONE payload representation (3a design
// statement): boxed scalars and, later, HiLowOptional kinds converge on it.
// Only the kinds the corpus needs have constructors — i32 as of 3b.
typedef struct HiLowScalar {
    HiLowCell cell;            // cell header — MUST be first member
    HiLowValue value;          // {kind, union} payload
} HiLowScalar;

HiLowScalar* hl_scalar_new_i32(int32_t v);
void hl_scalar_retain(HiLowScalar* s);
void hl_scalar_release(HiLowScalar* s);
int32_t hl_scalar_get_i32(HiLowScalar* s);
// The hl_cell_set family: store + equality check + notify. Fires CHANGED only
// when the value differed (spec: "(changed)" = value differs from previous),
// then HL_SCALAR_ASSIGNED on every call — changed subscribers before assigned
// subscribers, matching the legacy firing block's order. Store happens under
// stealth too; stealth suppresses only the notifications.
void hl_cell_set_i32(HiLowScalar* s, int32_t v);
// Reference-payload slots (Phase 3e-α): the slot ADOPTS a +1 reference at
// construction and on every set (retain a borrow first); getters BORROW;
// hl_scalar_release tears the payload down. (changed) fires iff unequal
// under the type's OWN equality — value equality for strings, identity for
// containers (audit §5 item 10a); (assigned) fires on every set.
HiLowScalar* hl_scalar_new_str(HiLowArray* v);
HiLowScalar* hl_scalar_new_array_ref(HiLowArray* v);
HiLowScalar* hl_scalar_new_object_ref(struct HiLowObject* v);
HiLowArray* hl_scalar_get_str(HiLowScalar* s);
HiLowArray* hl_scalar_get_array_ref(HiLowScalar* s);
struct HiLowObject* hl_scalar_get_object_ref(HiLowScalar* s);
void hl_cell_set_str(HiLowScalar* s, HiLowArray* v);
void hl_cell_set_array_ref(HiLowScalar* s, HiLowArray* v);
void hl_cell_set_object_ref(HiLowScalar* s, struct HiLowObject* v);

typedef struct HiLowArray {
    HiLowCell cell;                // cell header — MUST be first member (Phase 2a)
    size_t length;
    size_t capacity;
    size_t elem_size;
    void* data;
    hl_elem_fn retain_fn;          // NULL for primitive arrays, hl_object_retain for object arrays
    hl_elem_fn release_fn;         // NULL for primitive arrays, hl_object_release for object arrays
} HiLowArray;

// Cell event constants (Phase B scaffolding; DEEP added in Phase 2d, filling
// the long-documented gap at 4). The HL_ARR_ prefix is historical — as of
// Phase 2e the same constants serve every container cell: object property
// stores fire CHANGED (existing key) or ADDED (new key) on the object's cell.
#define HL_ARR_ADDED 1
#define HL_ARR_REMOVED 2
#define HL_ARR_CHANGED 3
#define HL_ARR_DEEP 4
#define HL_ARR_MOVED 5
// Scalar assignment event (Phase 3b): fired on EVERY hl_cell_set_* call,
// after the CHANGED notify when the value differed. An equal-value assignment
// is NOT a mutation: CHANGED and DEEP subscribers do not fire on it, and it
// never triggers the deep parent walk.
#define HL_SCALAR_ASSIGNED 6
// Follow marker (Phase 3e-β): a subscription node with this modifier on a
// variable-slot cell marks its watcher as FOLLOWING the variable — on
// rebinding, the watcher's nodes on the old value's cell retarget to the new
// value's cell (audit §5 item 10b). Never fired: it matches no event, and the
// implicit-CHANGED/DEEP arms in hl_cell_notify are modifier-keyed.
#define HL_SLOT_FOLLOW 7

HiLowArray* hl_array_new(size_t elem_size, size_t initial_capacity, hl_elem_fn retain_fn, hl_elem_fn release_fn);
void hl_array_retain(HiLowArray* arr);
void hl_array_release(HiLowArray* arr);
void hl_array_push(HiLowArray* arr, void* elem);   // copies elem_size bytes from elem into the buffer, growing if needed, now with firing loop
void* hl_array_get(HiLowArray* arr, size_t index); // returns pointer to element slot (caller casts and dereferences)
size_t hl_array_len(HiLowArray* arr);
void* hl_array_pop(HiLowArray* arr);                // removes and returns pointer to the last element slot; decrements length
void hl_array_set(HiLowArray* arr, size_t index, void* elem); // overwrites element at index with firing loop
void hl_array_remove(HiLowArray* arr, size_t index, void* out); // removes element at index into caller-owned out (elem_size bytes), shifting trailing elements down
void hl_array_insert(HiLowArray* arr, size_t index, void* elem); // inserts element at index, shifting trailing elements up

// Phase 10-ε-γ: moved-alias binding type (fixed layout compatible with
// Tuple(Usize, Usize)). As of Phase 2c no longer a firing payload — bodies
// construct it from HiLowDelta.from/.to for the alias binding.
typedef struct {
    size_t _0;  // from index
    size_t _1;  // to index
} HiLowMovedDelta;

void hl_array_move(HiLowArray* arr, size_t from, size_t to); // moves element from index 'from' to index 'to'
void hl_array_clear(HiLowArray* arr); // empties array by releasing all elements and setting length to 0

// Phase 2d: set the deep-watched bit on arr and (recursively) every nested
// array under it — and, as of Phase 2e, every object element too (deep
// marking is mutually recursive across the two container kinds). Invariant:
// marked implies the entire STRONG-reachable subtree is marked — all setters
// (subscription, containment-add into a marked parent) mark full subtrees,
// so the early-return on an already-set bit is sound and terminates
// diamonds. Weak properties are skipped: deep does not cross weak
// references. Clearing is deliberately not implemented (stale bit = one
// wasted walk, never a wrong fire).
void hl_array_mark_deep(HiLowArray* arr);
void hl_object_mark_deep(HiLowObject* obj);   // Phase 2e

// Array watcher registration/unregistration lives on the cell as of Phase 2a:
// subscription happens only inside hl_watcher_new_subscribed; removal happens
// only through watcher release (hl_cell_unsubscribe_watcher — see cell
// operations above). The env-keyed safety net died in Phase 2b.

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
HiLowArray* hl_object_property_value_str_at(HiLowObject* obj, size_t index);
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
#define TYPE_ARRAY 11    // Phase 2e: array-valued properties

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
void hl_object_weak_register(HiLowObject* target, HiLowObject* holder, size_t prop_index);
void hl_object_weak_unregister(HiLowObject* target, HiLowObject* holder, size_t prop_index);
void hl_object_set_object_weak(HiLowObject* obj, const char* key, HiLowObject* target);

// Weak property reads (Phase 1.5e, audit §5 item 6): reading a weak property
// yields an optional — the referent (retained) while it is alive, or unknown
// with reason "weak referent released" after its death. Member access through
// that optional propagates the same unknown; on a live referent it wraps the
// property value in a fresh optional. All of these return a fresh +1 optional
// the caller owns.
HiLowOptional* hl_object_get_weak(HiLowObject* obj, const char* key);
HiLowOptional* hl_optional_new_object(HiLowObject* o);   // takes ownership of o (+1)
HiLowObject* hl_optional_unwrap_object(HiLowOptional* opt);  // retain-on-return
HiLowOptional* hl_optional_member_i32(HiLowOptional* opt, const char* key);
HiLowOptional* hl_optional_member_str(HiLowOptional* opt, const char* key);
HiLowOptional* hl_optional_member_object(HiLowOptional* opt, const char* key);

// Retain-and-return helpers (Phase 1.5c): borrowed reference -> owned +1 inline
HiLowObject* hl_object_ref(HiLowObject* obj);
HiLowFunction* hl_function_ref(HiLowFunction* fn);
HiLowArray* hl_array_ref(HiLowArray* arr);

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
HiLowArray* hl_optional_unwrap_string(HiLowOptional* opt);
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
HiLowOptional* hl_time_parse(HiLowArray* iso_string);

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

// Phase 10a-stealth: watcher suppression depth.
// Becomes thread-local in Phase 10b when async is added.
extern int hl_stealth_depth;

// String operations (Managed Strings Sub-phase 1)
bool hl_string_eq(HiLowArray* lhs, HiLowArray* rhs);
bool hl_string_ne(HiLowArray* lhs, HiLowArray* rhs);
bool hl_string_eq_cstr(const HiLowArray* s, const char* lit);
HiLowArray* hl_string_from_cstr(const char* s);
HiLowArray* hl_string_concat(HiLowArray* lhs, HiLowArray* rhs);
void hl_array_append_bytes(HiLowArray* dst, const uint8_t* src, size_t n);
void print_string(HiLowArray* str);

// String-to-cstr helper for internal C APIs
const char* hl_array_to_cstr(HiLowArray* arr);

#endif // HILOW_RUNTIME_H