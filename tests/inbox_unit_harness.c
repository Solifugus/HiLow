// Phase 5a inbox unit tests (C-level). Only one thread exists through Phase 5a,
// so the cross-thread queue path is exercised here directly against the runtime
// API rather than from a HiLow program. Scaffolding-grade but permanent (the
// queue's contract, brief §4). Compiled + run (and run under valgrind) by
// tests/inbox_unit.rs.
//
// Each case asserts an order-insensitive invariant of the inbox mechanism:
// coalescing by watcher identity, mandatory delta accumulation (R3), watcher
// retain-on-enqueue / drop-on-drain (R6), drain-time liveness (ended watchers
// drop WITHOUT firing — R5), and teardown leaving an empty inbox (axiom 6).

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include "runtime.h"

// A test body: counts fires and records the events it saw (in order).
typedef struct {
    int fires;
    int events[64];
    int nevents;
} FireLog;

static void test_body(void* env, HiLowCell* cell, const HiLowDelta* delta) {
    (void)cell;
    FireLog* log = (FireLog*)env;
    log->fires++;
    if (log->nevents < 64) {
        log->events[log->nevents++] = delta ? delta->event : -1;  // -1 = bare fire
    }
}

// A standalone, retain/release-able cell: an int array's cell header (the cell is
// the first member of every container, so (HiLowCell*)arr is the container). The
// owner frees it with the TYPED release hl_array_release (hl_cell_release does
// only cell-header teardown; the typed release frees the struct) — so the owning
// ref must outlive the inbox's, which it always does below.
static HiLowCell* make_cell(HiLowArray** out_arr) {
    HiLowArray* arr = hl_array_new(sizeof(int32_t), 0, NULL, NULL);
    *out_arr = arr;
    return (HiLowCell*)arr;  // cell header is the first member
}

static HiLowDelta* added_delta(int32_t v) {
    return hl_delta_new_elem(HL_ARR_ADDED, &v, sizeof(v), NULL, NULL);
}

// Case 1: enqueue one bare (changed) fire → one pending entry; drain fires once.
static void case_enqueue_and_drain(void) {
    HiLowThreadContext* self = hl_current_ctx();
    FireLog log = {0};
    HiLowWatcher* w = hl_watcher_new();
    HiLowArray* arr; HiLowCell* cell = make_cell(&arr);

    hl_inbox_enqueue(self, w, cell, (void*)test_body, &log, HL_ARR_CHANGED, NULL);
    assert(hl_inbox_pending_count(self) == 1);
    assert(w->refcount == 2);   // R6: retained across the gap

    size_t fired = hl_thread_drain_inbox();
    assert(fired == 1);
    assert(log.fires == 1);
    assert(log.events[0] == -1);           // bare (changed) fire, NULL delta
    assert(hl_inbox_pending_count(self) == 0);
    assert(w->refcount == 1);   // dropped after fire-or-skip

    hl_watcher_release(w);
    hl_array_release(arr);
    printf("case_enqueue_and_drain OK\n");
}

// Case 2: coalescing + mandatory delta accumulation (R3). Two enqueues of the
// SAME watcher → ONE pending entry; both deltas accumulate and both fire.
static void case_coalesce_accumulates_deltas(void) {
    HiLowThreadContext* self = hl_current_ctx();
    FireLog log = {0};
    HiLowWatcher* w = hl_watcher_new();
    HiLowArray* arr; HiLowCell* cell = make_cell(&arr);

    hl_inbox_enqueue(self, w, cell, (void*)test_body, &log, HL_ARR_ADDED, added_delta(10));
    hl_inbox_enqueue(self, w, cell, (void*)test_body, &log, HL_ARR_ADDED, added_delta(20));
    assert(hl_inbox_pending_count(self) == 1);   // coalesced by watcher identity
    assert(w->refcount == 2);                     // retained ONCE despite two enqueues

    size_t fired = hl_thread_drain_inbox();
    assert(fired == 1);            // one live entry drained...
    assert(log.fires == 2);        // ...but both accumulated deltas delivered (R3)
    assert(log.events[0] == HL_ARR_ADDED && log.events[1] == HL_ARR_ADDED);
    assert(w->refcount == 1);

    hl_watcher_release(w);
    hl_array_release(arr);
    printf("case_coalesce_accumulates_deltas OK\n");
}

// Case 3: two DIFFERENT watchers do NOT coalesce → two entries, each fires.
static void case_distinct_watchers_dont_coalesce(void) {
    HiLowThreadContext* self = hl_current_ctx();
    FireLog log1 = {0}, log2 = {0};
    HiLowWatcher* w1 = hl_watcher_new();
    HiLowWatcher* w2 = hl_watcher_new();
    HiLowArray* arr; HiLowCell* cell = make_cell(&arr);

    hl_inbox_enqueue(self, w1, cell, (void*)test_body, &log1, HL_ARR_CHANGED, NULL);
    hl_inbox_enqueue(self, w2, cell, (void*)test_body, &log2, HL_ARR_CHANGED, NULL);
    assert(hl_inbox_pending_count(self) == 2);

    hl_thread_drain_inbox();
    assert(log1.fires == 1 && log2.fires == 1);

    hl_watcher_release(w1);
    hl_watcher_release(w2);
    hl_array_release(arr);
    printf("case_distinct_watchers_dont_coalesce OK\n");
}

// Case 4: an ended watcher drops at drain WITHOUT firing (R5), but its retained
// refs are still released (no leak) and its accumulated delta is freed.
static void case_ended_watcher_drops_without_firing(void) {
    HiLowThreadContext* self = hl_current_ctx();
    FireLog log = {0};
    HiLowWatcher* w = hl_watcher_new();
    HiLowArray* arr; HiLowCell* cell = make_cell(&arr);

    hl_inbox_enqueue(self, w, cell, (void*)test_body, &log, HL_ARR_ADDED, added_delta(7));
    w->ended = true;                 // ended after enqueue, before drain

    size_t fired = hl_thread_drain_inbox();
    assert(fired == 0);              // did not fire
    assert(log.fires == 0);
    assert(hl_inbox_pending_count(self) == 0);
    assert(w->refcount == 1);        // still dropped (no leak)

    hl_watcher_release(w);
    hl_array_release(arr);
    printf("case_ended_watcher_drops_without_firing OK\n");
}

// Case 5: teardown final-drain leaves the inbox empty (axiom 6).
static void case_final_drain_empties(void) {
    HiLowThreadContext* self = hl_current_ctx();
    FireLog log = {0};
    HiLowWatcher* w = hl_watcher_new();
    HiLowArray* arr; HiLowCell* cell = make_cell(&arr);

    hl_inbox_enqueue(self, w, cell, (void*)test_body, &log, HL_ARR_CHANGED, NULL);
    assert(hl_inbox_pending_count(self) == 1);
    hl_thread_final_drain();         // final drain fires the entry and drops its refs
    assert(log.fires == 1);          // axiom 6: residue delivered, inbox emptied
    // `self` is now freed (final_drain tore down the context); do not touch it.
    // The owner still holds its refs — the inbox was never the sole owner — so
    // free them now with the typed releases.
    hl_watcher_release(w);
    hl_array_release(arr);
    printf("case_final_drain_empties OK\n");
}

// Case 6 (Phase 5c, deviation 5a-ii graduation): the inbox is the SOLE owner of
// the cell at drain — the declaring binding released its ref while the entry was
// in flight (reachable in 5c: a shared scalar's last release lands on a producer
// thread via the inbox). The drain's hl_cell_release_full must do the full TYPED
// teardown, not just header teardown, or the container leaks. Under valgrind this
// case leaks (definite) with the pre-5c hl_cell_release; it is clean with the fix.
// (a) array cell — HL_CELL_ARRAY dispatch.
static void case_inbox_sole_owner_frees_array(void) {
    HiLowThreadContext* self = hl_current_ctx();
    FireLog log = {0};
    HiLowWatcher* w = hl_watcher_new();
    HiLowArray* arr; HiLowCell* cell = make_cell(&arr);   // owner refcount 1

    hl_inbox_enqueue(self, w, cell, (void*)test_body, &log, HL_ARR_CHANGED, NULL);
    // enqueue retained the cell → refcount 2 (owner + inbox).
    hl_array_release(arr);                 // owner drops → refcount 1; NOT freed
    assert(hl_inbox_pending_count(self) == 1);

    size_t fired = hl_thread_drain_inbox(); // inbox releases last → full teardown frees arr
    assert(fired == 1);
    assert(log.fires == 1);
    assert(hl_inbox_pending_count(self) == 0);

    hl_watcher_release(w);
    printf("case_inbox_sole_owner_frees_array OK\n");
}

// (b) scalar cell — HL_CELL_SCALAR dispatch (the 5c cross-thread case: shared
// scalars are the only cross-thread cells).
static void case_inbox_sole_owner_frees_scalar(void) {
    HiLowThreadContext* self = hl_current_ctx();
    FireLog log = {0};
    HiLowWatcher* w = hl_watcher_new();
    HiLowScalar* s = hl_scalar_new_i32(42);
    HiLowCell* cell = (HiLowCell*)s;       // cell header is the first member

    hl_inbox_enqueue(self, w, cell, (void*)test_body, &log, HL_ARR_CHANGED, NULL);
    hl_scalar_release(s);                  // owner drops → refcount 1; NOT freed
    assert(hl_inbox_pending_count(self) == 1);

    size_t fired = hl_thread_drain_inbox(); // inbox last release → hl_scalar_finalize frees s
    assert(fired == 1);
    assert(log.fires == 1);

    hl_watcher_release(w);
    printf("case_inbox_sole_owner_frees_scalar OK\n");
}

int main(void) {
    case_enqueue_and_drain();
    case_coalesce_accumulates_deltas();
    case_distinct_watchers_dont_coalesce();
    case_ended_watcher_drops_without_firing();
    case_final_drain_empties();
    case_inbox_sole_owner_frees_array();
    case_inbox_sole_owner_frees_scalar();
    printf("ALL INBOX UNIT TESTS PASSED\n");
    return 0;
}
