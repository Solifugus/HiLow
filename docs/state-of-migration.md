# State of the Cell Migration

> Preflight reconnaissance for Phase 5 (queues). Produced 2026-07-19 as a
> read-only audit — no code changed in the session that created it. Cross-checks
> `docs/cell-redesign-brief.md` (the agreed design) against the actual tree at
> commit `b4c78a5` (Phase 4a complete). Where the brief and the tree disagree,
> the tree is described from code, with file:function citations.
>
> This document exists because of the 4a lesson: the brief described the
> *archived* compiler in at least one place (the `pending_statement_stmts` /
> `a21a6de` landing pad, which is not an ancestor of HEAD). Assume it does so
> again until proven otherwise — every brief claim below is checked against code.

---

## Part A — 4a seal

### A.1 Producer-inventory provenance

The 4a plan required the producer inventory (`fresh_production_temp_kind`,
`src/codegen/mod.rs:7215`) to be **provably a projection of the "3b
classification functions."** Here is the precise relationship.

**The real census is a pair of predicates, and only one of them is a producer
classifier:**

- `expr_is_borrowed_ref` (`mod.rs:7192`) — the *borrow* classifier: `Ident` /
  `This` / `MemberAccess` / `IndexAccess` (+ `TypeAscription` / `WeakRef`
  recursion). These evaluate to a reference owned elsewhere; store sites never
  release them.
- `needs_site_release_after_store` (deleted in 4a) — the *fresh-production*
  classifier: `ObjectLiteral` / `FunctionExpr` / `FString` / `ArrayLit` /
  object-or-function-typed `Match`. These produced an untracked `+1` that the
  store site released.

**The projection holds against `needs_site_release_after_store`, not against the
two predicates the plan named as suspects.** `fresh_production_temp_kind`'s
domain is *exactly* `needs_site_release_after_store`'s domain — the five forms
map one-to-one:

| form | `fresh_production_temp_kind` → | was in `needs_site_release_after_store` |
|------|-------------------------------|------------------------------------------|
| `ObjectLiteral` | `(Object, "HiLowObject*")` | yes |
| `ArrayLit` | `(Array, "HiLowArray*")` | yes |
| `FString` | `(Array, "HiLowArray*")` | yes |
| `FunctionExpr` | `(Function, "HiLowFunction*")` | yes |
| `Match` (Object/Function result) | `(Object\|Function, …)` | yes (object/function-typed match) |

The doc-comment at `mod.rs:7213` states this identity outright ("This is the
exact set the deleted `needs_site_release_after_store` named"), and the code
matches it.

**The two predicates the 4a plan flagged as suspects were falsified by the fresh
tree.** `expression_produces_heap_value` and `is_heap_allocating_expression`
were **dead (no callers) and wrong** (e.g. `expression_produces_heap_value`
classified `Call` → None, contradicting the real heap-returning-`Call` temp mint
at `mod.rs:4289`). They were never the census; they were deleted in 4a. So the
projection requirement was **not** falsified — it just never referred to those
two. It refers to `needs_site_release_after_store`, and post-4a that predicate is
subsumed: `fresh_production_temp_kind` **is** the fresh-production census now.

**The complete temporary-producing census (post-4a)** is the union of:

1. **Eight self-minting arms** — expression forms whose own
   `generate_expression` arm mints a tracked temp in `Temporary` context:
   `StringLit` (`mod.rs:2717`), `WatcherExpr` (`:3459`), string-concat `str+str`
   (`:3969`), heap-returning `Call` (`:4289`), `unknown.reason` (`:~6372`),
   string object-member read (`:~6415`), optional member (`:~6505`),
   `time.parse` (`:~6730`).
2. **Five `fresh_production_temp_kind` forms** — folded in by the interception at
   `mod.rs:2692`, which captures the construction by recursing in `Owned` context
   and hoists it into `pending_statement_decls`.

Everything else is either a borrow (`expr_is_borrowed_ref`) or a non-heap
scalar/primitive.

**Does any value-producing site fall outside it? (from code inspection)**

One seam, and it **predates 4a** (it is not in either census predicate's
domain, and 4a did not touch it): **`Match` expressions whose result type is a
heap type other than `Object`/`Function`** (`String`, `DynamicArray`,
`Optional`, `Unknown`). `fresh_production_temp_kind` returns `None` for these
(`mod.rs:7221` only handles `Type::Object` / `Type::Function`), so the match
wrapper mints no temp of its own and adds no retain — the `ref_wrap` at
`generate_match_expression` (`mod.rs:7060–7078`) that turns borrowed arms into
`+1` references is **object/function-only**. Such a match relies entirely on its
*arm expressions* being self-minting (a `StringLit` arm self-mints at 2717 and is
accounted). The unaccounted case is a **borrowed arm** value (`x => someStr`)
bound in `Owned` context: `__match_result = someStr` (a borrow) is bound to the
`let`, whose scope-exit release then over-releases the borrowed referent.

**Probe result — CONFIRMED LIVE USE-AFTER-FREE.** The program

```
let a = "hello"
let n = 1
let s = match n { 1 => a  _ => "x" }
print(s); print(a)
```

compiles clean and, under valgrind, double-releases `a`'s cell — `Invalid read
of size 4` / `Invalid write of size 4` in `hl_cell_release` (freed then written).
The same failure reproduces under the **pre-4a** binary (`ced77dd`), confirming
it **predates 4a**. The control case (fresh-literal arms `1 => "yes" _ => "no"`)
is **valgrind-clean (0 errors)**, precisely bounding the seam to a *borrowed*
arm value in `Owned` context.

This was a **finding for the doc, not a 4a defect** — 4a's scope was the five
fresh-production forms, all correctly folded in, and the census is a complete and
correct projection *for what it covers*. The seam was a pre-existing
non-obj/fn-typed-`match` ownership gap (a 1.5c-era retain-on-store concern the
`ref_wrap` never extended past object/function results).

**EXTINGUISHED in Phase 4b** (commit follows this doc): `generate_match_expression`'s
`ref_wrap` (`mod.rs:7060`) now retains a borrowed arm of a `String`/`DynamicArray`
result in Owned context with `hl_array_ref`, mirroring the Object/Function case —
so every heap-typed match now hands a `+1` into Owned context. Reachability was
bounded by probe: only `String` and `DynamicArray` are affected (Optional-typed
match is untracked and never over-releases; `Unknown` values are not
constructible as bare arms; Object/Function were already retained). See the
extinct-classes table (§D.2) for the pinning tests.

**Census (post-4b), one sentence:** the temporary census is unchanged —
`fresh_production_temp_kind` still mints only Object/Function-typed match (∪ the
eight self-minting arms) as a tracked temp; every *other* heap-typed match
(String/DynamicArray) is accounted at the arm instead (borrowed arms retained via
`hl_array_ref` in Owned context; fresh-literal arms self-mint a
statement-end-released temp; Temporary-context borrows stay untracked borrows), so
match in all result types is now ownership-accounted without routing through the
temp mechanism.

### A.2 The 49-vs-50 diffing-program count

The 4a STATUS entry is internally inconsistent: the "Current state" block
(STATUS line 10) says "50 diffing programs verified old-vs-new stdout+exit
identical" while the session entry (STATUS line 28) says "255 programs
byte-identical C; 49 diff … ALL 50 diffing programs verified." 49 (the C-diff
count) and 50 (the output-verified count) cannot both be the number of programs
whose generated C differs.

**Re-derived authoritatively** by regenerating `main.c` for every corpus entry
(the valgrind gate's entry-discovery rule) under `ced77dd` (pre-4a) and
`b4c78a5` (4a) and diffing, with per-side double-capture to screen out the known
`modules/diamond` run-to-run emission nondeterminism:

| bucket | count |
|--------|-------|
| total entries | 333 |
| byte-identical C (both binaries) | 254 |
| **stable C-diff (both sides deterministic)** | **52** |
| nondeterministic (screened out) | 1 (`modules/diamond` — the known permuter) |
| one-sided (compiles only one binary) | 0 |
| no C either side (= `REJECTION_FIXTURES`) | 26 |
| compiling under both | 307 (254 + 52 + 1) |

Two independent validations that the harness is correct: `no-C-both = 26`
exactly equals `REJECTION_FIXTURES.len()`, and the single nondeterministic entry
is exactly `modules/diamond` (the pre-existing module-emission permuter recorded
since Phase 2e). Three spot-checked diffs (`format_basic`, `object_field_overwrite`,
`watcher/pause_resume`) are confined to the two declared classes —
temp-declaration hoisting (`__inline_fstr`/inline construction → hoisted
`__tmp_N`) and release-site placement (store-site `{…hl_X_release(__pv);}`
wrapper → statement-end release) — semantically identical, relocated only.

**The correct stable-C-diff count is 52** (with `modules/diamond` a
nondeterministic 53rd that diffs against *itself* and must be excluded from any
diff judgment); the correct byte-identical count is **254**, not 255. The 4a
entry's "49 diff" and "50 verified" both undercounted and were mutually
inconsistent. Most likely reconciliation: "49" counted pre-existing-corpus diffs
and excluded the three new `temp_nonstore_*` fixtures (49 + 3 = 52, and those
three appear in the diff list because they compile-but-leak under `ced77dd`);
"50" was an off-by-one output-verification tally. The STATUS session entry is
corrected accordingly.

---

## Part B — the notification / delivery machinery as it exists today

### B.1 The delivery path (`hl_cell_set` → `hl_cell_notify` → body)

Delivery is **fully synchronous and inline** — there is **no queue**.

- **Scalar/slot writes:** `hl_cell_set_i32` (`runtime.c:1526`),
  `hl_cell_set_str/_array_ref/_object_ref` (`:1553/1564/1577`) store the value,
  then — gated on `hl_stealth_depth == 0 && cell_has_audience(cell)` — call
  `hl_cell_notify(&s->cell, HL_ARR_CHANGED, NULL)` (only when the value differed
  under the type's own equality) followed by `hl_cell_notify(&s->cell,
  HL_SCALAR_ASSIGNED, NULL)` (always).
- **Container mutators:** `hl_array_push/_set/_remove/_insert/_move/_clear`
  (`:2848`–`:3128`) and object `set_property` (`:432`) each build a `HiLowDelta`
  on the heap via `hl_delta_new_elem` / `hl_delta_new_moved`, call
  `hl_cell_notify(&arr->cell, event, d)`, and `hl_delta_release(d)` **in the same
  call**. The delta never outlives the mutator.
- **`hl_cell_notify` (`:1157`), the one firing path:** short-circuits under
  stealth; increments `hl_notify_depth`; snapshots the subscriber list
  (`snapshot_cell_nodes`, collect-then-fire); walks the snapshot in list order
  invoking each active, non-ended node whose modifier matches the event (or is
  `CHANGED`/`DEEP`, excluding `HL_SCALAR_ASSIGNED`) through the **env-first ABI**
  `((HiLowWatcherBody)body_fn)(env, cell, delta)` (`:1177`); then, only when
  `c->deep_watched && c->parents`, collects ancestors under a fresh
  `hl_deep_epoch` and fires their `HL_ARR_DEEP` nodes with the same delta;
  decrements `hl_notify_depth`, draining deferred releases at depth 0.
- **Codegen injects no firing code** — confirming brief lines 36–38. All firing
  originates in runtime mutators.

The `HiLowWatcherBody` ABI (`runtime.h:364`): `void (*)(void* env, HiLowCell*
cell, const HiLowDelta* delta)` — env-first, one signature for every modifier.
Bodies borrow `cell`, `delta`, and the payload for the call's duration.

### B.2 The owned queueable deltas (from 2c)

`HiLowDelta` (`runtime.h:176`, `runtime.c:1003`) is **queue-*able* but not
queued.** It is heap-owned and self-contained: it owns a copy of the element
bytes (`hl_delta_new_elem` memcpys `elem_size` bytes) and, for object arrays,
holds a **retained** element reference (`retain_fn` at construction); it has no
pointers into array storage, caller stacks, or statics. It is created by the
mutator, borrowed by `hl_cell_notify`, and released by the mutator's own
`hl_delta_release` before the mutating call returns. **Who owns it:** the
producing mutator (sole owner for the call's duration). **Where the queue
lives:** nowhere — there is no queue. **When it drains:** N/A; the delta is
delivered synchronously and freed immediately. The header comment at
`runtime.h:169–175` states the design intent ("QUEUEABLE … Phase 5 (queues)
changes only WHEN and WHERE bodies run and who calls `hl_delta_release`") — the
*shape* is ready for Phase 5; the *queue* is not built.

### B.3 The 3e-β collect-then-fire snapshot + deferred-release machinery

Two distinct same-thread mechanisms, neither of which is the brief's
notification queue:

- **Node snapshot (collect-then-fire):** `snapshot_cell_nodes` (`:1134`) copies
  each subscriber node's `{modifier, body_fn, env, watcher, origin}` into an
  inline-16 buffer (heap beyond) **before any body runs**. It defers nothing —
  it lives and dies inside a single `hl_cell_notify` call. **What it protects:**
  traversal atomicity with respect to body execution. A body may retarget or
  unsubscribe nodes on the cell being walked (a body rebinding its own followed
  variable); the walk reads the snapshot, never live list links, so a node
  retargeted away mid-walk still fires for the in-flight event and a node
  subscribed mid-walk does not.
- **Deferred old-payload release:** `hl_notify_depth` + `hl_deferred_releases`
  (`:1050–1086`). `hl_cell_set_str/_array_ref/_object_ref` release the *old*
  payload via `hl_release_or_defer` (`:1065`): if a notify walk is in flight
  (`hl_notify_depth > 0`) the release is parked as a `(fn, ptr)` pair and drained
  by `hl_drain_deferred_releases` when the **outermost** walk completes (depth
  0). **What it defers:** the slot's owned `+1` on the old value. **For how
  long:** until the outermost in-flight walk returns. **What invariant it
  protects:** a watcher body running inside a walk of the very cell being
  released — plus that body's borrowed snapshot of the old value — must outlive
  the release.

### B.4 Existing thread awareness

**None**, as expected. Grep of `src/runtime/*.{c,h}` for `pthread` /
`_Thread_local` / `thread_local` / `_Atomic` / `mutex` / `memory_order` /
`<threads.h>` / `<pthread.h>` returns **zero** synchronization primitives (the
only "atomic" matches are the word in the collect-then-fire *comments* at
`runtime.c:1109,1161`). No thread-local storage, no atomics, no locks.

### B.5 Program-scope statics — what would be shared under multi-thread

**Runtime statics** (in `runtime.c`, linked into every program; all
process-global, unsynchronized):

| symbol | `runtime.c` | role | flagged for TLS? |
|--------|-------------|------|------------------|
| `int hl_stealth_depth` | 48 | stealth-suppression depth | yes — header `:668`, STATUS `:171` ("thread-local in 10b") |
| `static uint64_t hl_deep_epoch` | 53 | deep-walk epoch counter | **no** |
| `static int hl_notify_depth` | 1050 | notify-walk depth (release deferral gate) | **no** |
| `static HiLowDeferredRelease* hl_deferred_releases` + `_len` + `_cap` | 1057–59 | deferred old-payload release list | **no** |
| `int hl_alloc_count` / `int hl_free_count` | 1720–21 | debug allocator counters | **no** |
| `HiLowNothing the_nothing` | (global) | shared immutable sentinel | n/a (immutable) |

**Emitted-C program-scope statics** (codegen writes at file scope):

- `static HiLowScalar* <name> = NULL;` — one per **watched (boxed) program-scope
  scalar** (`mod.rs:1364, 1400`). These hold cell pointers; under a multi-thread
  model they are shared mutable cells.
- `static void <dtor>(void* raw)` env-dtor functions (`mod.rs:321`) — code, not
  mutable state.

**Phase-5 input:** only `hl_stealth_depth` is currently flagged to become
thread-local. `hl_deep_epoch`, `hl_notify_depth`, and the `hl_deferred_releases`
list are **equally process-global and equally unsafe** under real concurrency,
and are **not** yet flagged. A declaring-thread model must make all four (plus
the boxed program-scope cells' access discipline) thread-correct.

### B.6 Reconciliation against the brief's Phase 5 section

Every brief claim about queues, threads, or delivery, checked against the tree:

| # | brief claim | status in this tree |
|---|-------------|---------------------|
| 1 | "non-reentrant static delta buffers" (motivation, line 22) | **ALREADY FIXED.** Both static `temp_buffer`s were deleted in Phase 2c (`6fa4e3d`, STATUS line 48). No static delta buffer exists; deltas are heap-owned values and re-entrancy is handled by collect-then-fire + a fresh heap delta per mutation. |
| 2 | "Deltas are values pushed onto the queue — this replaces the static `temp_buffer`" (line 66) | **HALF DELIVERED, HALF UNBUILT.** The delta-as-self-contained-value redesign is done (`HiLowDelta`, 2c) and the static `temp_buffer` it replaces is already gone. The "pushed onto the queue" half is unbuilt — deltas are passed by borrow to synchronous `hl_cell_notify` and freed in the same call. |
| 3 | "Per-statement release list … released at statement end via the single cell-release function" (lines 71–73) | **DELIVERED (as the temp list), TERMINOLOGY STALE.** The release list exists (`temp_owners`) and 4a unified every fresh production onto it. "The single cell-release function" is the **codegen dispatcher** `emit_temp_release` (`mod.rs:7438`); there is **no** value-dispatching *runtime* release function (`hl_cell_release` is only the header primitive: refcount + subscription/parent teardown). This is expression-temporaries work the brief lists near Phase 5, but it is already complete (Phase 4a). |
| 4 | "Each thread owns a notification queue … enqueues `(watcher, delta)` onto B's queue" (lines 58–59) | **UNBUILT.** No queue type, no per-thread state, no enqueue. Delivery is 100% synchronous and inline. |
| 5 | "Watcher bodies execute on their declaring thread, at safe points where the queue is drained" (lines 60–62) | **UNBUILT.** No threads, no safe points, no notification-queue drain. (The depth-0 `hl_drain_deferred_releases` drains *old-payload releases*, not notifications — do not mistake it for the brief's queue drain.) |
| 6 | "Same-thread fires may run synchronously or via the queue" (lines 63–65) | **ONLY THE SYNCHRONOUS BRANCH EXISTS.** Sound as a Phase-5 design statement; as a description of the tree, there is no queue option. |
| 7 | "Array/object mutators fire from inside the runtime. Codegen injects no firing code anywhere." (lines 36–38) | **TRUE / holds exactly.** The one delivery claim that matches the tree as-is. |

**The 4a lesson applied forward:** the tree already contains something
queue-shaped — the `hl_deferred_releases` list — but it is a *release*-deferral
mechanism drained at same-thread notify-depth 0, **not** the brief's per-thread
*notification* queue. Phase 5 planning must not treat the former as a head start
on the latter.

---

## Part C — enqueue-ownership audit

Every site that enqueues, stores, or defers a delta, notification, or snapshot
node **beyond the producing statement**, with an ownership verdict. The concern:
4a releases fresh temporaries at the statement boundary; anything holding a
*borrow* of such a temp past that boundary is a use-after-free waiting for a
cross-thread drain to expose it.

| site | what it holds past the statement | verdict |
|------|----------------------------------|---------|
| `hl_release_or_defer` in `hl_cell_set_str` (`:1561`), `_array_ref` (`:1574`), `_object_ref` (`:1583`) | the slot's **old payload** pointer, parked as `(fn, ptr)` until depth-0 drain | **OWNED.** The slot adopted a `+1` on that payload at construction/prior set; the store overwrote the field but did not decrement. The deferral holds that live owned `+1` and drops it on drain. Not a borrow. Sound across a future cross-thread drain. |
| the `HiLowDelta` in each mutator (`:2873` etc.) | nothing — released by `hl_delta_release` in the same call | **N/A today; SELF-OWNED if queued.** Does not outlive the producing statement. For object arrays it holds a *retained* element ref, so it is already safe to enqueue in Phase 5 without further retains. |
| `snapshot_cell_nodes` buffer (`:1134`) | nothing — freed before `hl_cell_notify` returns | **INTRA-CALL.** Borrows `body_fn`/`env`/`watcher`/`origin`, but only for the walk. `env` is owned by the watcher; watcher STATE pointers are argued valid across a walk (a body cannot release a pre-existing binding). |
| `HiLowCellWatcher` subscription node (`hl_cell_subscribe`, `:917`) | borrows `env` from the owning watcher for the node's lifetime | **OWNED-BACKED.** The watcher owns `env` (`+1`, released on final release); node lifetime ⊆ watcher lifetime (whichever dies first unlinks). |

**Verdict: no borrow of a statement-scoped temporary is held at any enqueue or
defer site.** The single site that defers past the producing statement
(`hl_release_or_defer`, three callers) holds the slot's owned old-payload `+1`,
not a borrow — sound. Deltas and node snapshots do not outlive their producing
mutator call today; deltas already self-own their payload (safe to queue in
Phase 5), and node snapshots borrow only watcher-owned or walk-stable state.

**Phase-5 forward constraint (from this audit):** when Phase 5 actually queues
`(watcher, delta)` across a drain gap, the queued entry must **retain the
watcher** (for `env`/`body_fn` validity past the producing call) — the delta is
already self-owning. Today's synchronous delivery makes this free; a queue makes
it mandatory.

---

## Part D — Phase 5 planning inputs

### D.1 Deletions delivered vs. the brief's promised deletions

| brief promise | delivered | commit |
|---------------|-----------|--------|
| "All name-keyed subscription maps in codegen are deleted" (core decision, line 31); Phase 3 "delete `watcher_subscribers`" (line 99) | `watcher_subscribers`, `heap_watcher_subscribers`, `watcher_name_to_id` deleted | 3d `ff288d4` |
| Phase 3 "shadow masking" | shadow-masking block + three collect helpers deleted | 3d `ff288d4` |
| Phase 3 "static active/ended bools" | per-watcher static `_active`/`_ended` → `HiLowWatcher.active/.ended` | 3c `80d4503` |
| Phase 3 "emitted deactivation" | scope-exit deactivation loops → `hl_watcher_release` | 3c `80d4503` |
| Phase 2 "delete array-related codegen injection" (line 98) | all firing moved into runtime mutators; scalar firing block deleted | 2c `6fa4e3d` / 3b `9d1cdf9` |
| Phase 5 "replaces the static `temp_buffer`" (line 66) | both static `temp_buffer`s deleted (**ahead of Phase 5**) | 2c `6fa4e3d` |
| Expression temporaries "per-statement release list" (lines 71–73) | `temp_owners` release list; all fresh productions unified onto it; `needs_site_release_after_store` + 11 sites deleted | 4a `b4c78a5` |
| (beyond the brief's list) two dead+wrong predicates | `expression_produces_heap_value`, `is_heap_allocating_expression` deleted | 4a `b4c78a5` |
| (beyond the brief's list) typecheck escape machinery | `capture_unsafe_watchers`, `check_watcher_escape_reachability`, both rejection sites deleted | 3b `9d1cdf9` |

### D.2 Extinct bug classes (each cross-referenced to its proving test)

The brief's motivating bug taxonomy (lines 18–23), and the gate's original
memory-bug lists, class by class:

| extinct class | killed in | proving test(s) |
|---------------|-----------|-----------------|
| name-based subscription requiring shadow masking | 3d | `three_level_shadow_probe`, `same_name_caller_callee`, `watcher_shadow_array` |
| name-keyed codegen maps leaking across function boundaries | 3d | the shadowing sentinels above + corpus-wide zero-diff C check |
| env use-after-free (envs owned by declaring scope) | 2a/2b (watcher-owned env) + 3b (`env_dtor` retains cells) | `watcher_escape_capture_sound`, `watcher_escape_subscribed_local_sound` |
| unreachable scope-exit deactivation under early returns | 3c | `watcher_early_return_scalar`, `watcher_early_return_array` |
| non-reentrant static delta buffers | 2c (both `temp_buffer`s deleted; `HiLowDelta` heap value) | the valgrind gate as a whole; `watcher_reentrant_sync` (synchronous same-thread firing, valgrind-clean) |
| ad-hoc handling of expression temporaries | 4a (unification) | `temp_nonstore_object`, `temp_nonstore_array`, `temp_nonstore_arg`; `string_concat`, `string_equality` (string-operand clean) |
| non-obj/fn-typed `match` with a borrowed arm in Owned context — use-after-free | 4b (`ref_wrap` extended to `hl_array_ref`) | fix targets `match_borrow_string_owned`, `match_borrow_array_owned`; controls `match_borrow_string_bare`, `match_borrow_string_arg`, `match_fresh_arm_owned` |
| object double-release (17 programs — gate's original `KNOWN_MEMORY_BUGS`) | 1.5c | `KNOWN_MEMORY_BUGS` is now EMPTY (`valgrind_gate.rs:62`; comment lines 55–61) |
| §3.4(b)/(c)/(d) env-keying bugs | 2a | gate comment `valgrind_gate.rs:57` |
| §3.4(a) `.move` 2-arg env-dropping casts | 2c | gate comment `valgrind_gate.rs:59` |
| deviation 5a-ii: inbox sole-owner cross-thread last-release leaks the typed container (`hl_cell_release`-at-0 did only header teardown) | 5c (cell `kind` tag + `hl_cell_release_full` dispatches the typed `*_finalize` at refcount 0; inbox routes through it) | `case_inbox_sole_owner_frees_array`, `case_inbox_sole_owner_frees_scalar` (`tests/inbox_unit_harness.c` — each leaks 152B with pre-fix `hl_cell_release`, clean with the fix); HiLow-level `shared_exit_release` |

### D.3 Consolidated semantics adjudications since the brief

Rulings made in-session that the brief does not contain — one line each: the
ruling, the phase, the enforcing commit/test.

- **Declaration-order fire order** — `hl_cell_subscribe` *appends*; bodies fire
  in declaration order. Phase 3b (`9d1cdf9`). Tests `nested_watchers`,
  `expression_coexists`; flipped `added_and_changed_both_fire`,
  `moved_changed_both_fire`, `array_insert_watcher_fires`.
- **(changed)/(assigned) split** — `(changed)` fires iff the value differs under
  the type's own equality (strings by value via `hl_string_eq`, containers by
  identity); `(assigned)` fires on every set; changed-before-assigned. Phase 3b
  (scalars, `9d1cdf9`) + 3e-α (slots/strings/containers). Tests
  `watcher_changed_assigned_order`, `watcher_string_changed_fires`,
  `watcher_object_assigned_fires`, `watcher_array_assigned_fires`.
- **Deep semantics** — a `(deep)` subscriber on the mutated cell fires for every
  mutation; ancestors' `(deep)` nodes fire via collect-then-fire under a fresh
  epoch; the `deep_watched` bit skips the walk when nobody deep-watches. Phase
  2d. Tests: deep direct / 3-level nested / sibling-isolation / diamond /
  unsubscribe / stealth / new-child.
- **Weak rules** — a weak store adds no containment link but still fires on the
  holder; deep does not cross weak references; a dead-weak read yields `unknown`
  "weak referent released". Phase 2e (containment/firing) + 1.5e (after-death).
  Test `test_weak_after_death_unknown_integration`.
- **Collect-then-fire discipline** — `hl_cell_notify` snapshots the subscriber
  list before any body runs (direct + per-ancestor loops); a node subscribed
  mid-walk does not fire for the in-flight event, a node retargeted away still
  does. Phase 2d (deep walk) + 3e-β (direct loop, `1acbbbf`). Shape-B fixture.
- **Deferred old-payload release** — `hl_cell_set_*` defers the old-payload
  release while any notify walk is in flight (`hl_notify_depth`), drained at
  depth 0. Phase 3e-β (`1acbbbf`). Reentrant/borrowed-snapshot fixture.
- **Origin-identity attribution** — each decl-form container-content
  subscription records the *slot cell* whose follow created it
  (`HiLowCellWatcher.origin`, pointer identity); retarget moves only the
  rebinding slot's own nodes. Phase 3e-γ (`ced77dd`). Tests `alias_two_watchers`,
  `alias_one_watcher`, `alias_sequential_rebinds`, `alias_deep`.
- **Fresh-retain-at-store** — every store of a heap reference retains (`+1`); the
  producer's `+1` is released at statement end, not at the store site. Axiom
  Phase 1.5c; store path unified in 4a (`b4c78a5`). Tests
  `temp_nonstore_object/array/arg`.
- **Statement-boundary temp release** — every fresh heap temporary is released at
  the end of its producing statement via the one per-statement release list
  (`temp_owners` → `emit_temp_release`). Phase 4a (`b4c78a5`).
- **Expression-form vs decl-form subscription target** — expression-form
  watchers subscribe the value by identity; decl-form follows the variable slot
  (rebinding via retargeting); `(assigned)` subscribes the slot in either form.
  Phase 3e-α/β.
- **Decl-form watcher names are not first-class values** — a decl-form watcher
  name supports method calls only (not return/alias/assignment). Phase 3c
  (`80d4503`). Fixtures `watcher_decl_name_return_rejected`,
  `watcher_decl_name_alias_rejected`.
- **Module-level watcher rejection** — module-top watchers are rejected with a
  diagnostic (no specified construction / observation-start semantics). Phase 3c
  (`80d4503`). Fixture `modules/watcher_in_module`.
- **Open-shape dynamic property addition deferred** — `(added)`/`(assigned)obj`
  on closed object types are rejected (no reachable trigger); dynamic property
  addition is an unscheduled post-migration phase. Phase 2e/3a. Fixtures
  `object_watch_added_rejected`, `object_watch_removed_rejected`.

### D.4 Phase 5 planning inputs

**Parts B and C are the primary inputs** (above, verbatim): B establishes that
there is no queue, that delivery is synchronous, that deltas are already
queue-able self-owning values, and that four process-global runtime statics
(only one flagged) would be shared under concurrency; C establishes that no
enqueue/defer site holds a borrow of a statement-scoped temp today, and that a
Phase-5 notification queue must retain the queued watcher.

**Recorded Phase-5 notes from STATUS:**

- *Statement-boundary / drain-independence claim* (STATUS lines 22, 28, 127) —
  **UNVERIFIED CLAIM** (as recorded): "the statement-boundary temp release is
  independent of queue draining and imposes no constraint on it … Because
  watcher bodies are emitted as separate C functions with their own reset
  temp-frame stack, a watcher firing does not nest into the enclosing statement's
  temp frame at compile time."
  **Part C adjudication:** the claim **holds for the current synchronous tree** —
  no queued/deferred site borrows a statement-scoped temp, and watcher bodies
  reset `enclosing_temp_frames` at their function boundary, so a synchronously-
  fired body cannot unwind into an enclosing statement's temp frame. It is **not
  unconditional for Phase 5**: once notifications are *queued* and a body runs at
  a later drain point rather than inline, the queued `(watcher, delta)` must
  independently own the watcher (Part C forward constraint). The independence is
  of *compile-time temp-frame nesting*, which queuing does not change; it is not
  a guarantee that queued delivery needs no ownership work.
- *3e-β deferral precedent* (STATUS lines 16, 32) — the same-thread
  old-payload-release deferral (`hl_notify_depth` + `hl_deferred_releases`,
  drained at depth 0) is the existing precedent for "run the body now, drop the
  resource later." Phase 5's drain is a *different* mechanism (per-thread
  notification queue vs. same-thread release list) but the depth-gated drain
  shape is a reusable pattern.
- *`test_watcher_reentrant_deferred` flips in Phase 5* (STATUS lines 56, 125) —
  `#[ignore]`d; asserts deferred (declaring-thread-queue) firing per the brief.
  Current firing is synchronous; the program is valgrind-clean. This is the test
  that turns live when the queue lands.
- *Statics-as-shared question* (STATUS line 171; Part B.5 above) — only
  `hl_stealth_depth` is flagged for thread-local treatment; `hl_deep_epoch`,
  `hl_notify_depth`, and `hl_deferred_releases` are equally global and unflagged.

**Phase 5a scope reminder** (STATUS line 22): per-thread notification queue type
+ safe-point drain hooks (allocation/syscall boundaries), single-threaded
semantics preserved (same-thread fires may stay synchronous); deltas become
queued values. Gate: full suite unchanged.

---

## Part E — Phase 5a landed (2026-07-21): what was built + adjudications

Runtime-support-only (`src/runtime/runtime.{c,h}` + one `main.rs` cc flag); **no
`src/codegen/mod.rs` change**, so program-logic codegen is byte-identical by
construction and the single-threaded corpus is byte-identical (326/0/2, gate
green). The queue path has no HiLow-source surface until 5b, so it is proven by
C-level unit tests (`tests/inbox_unit_harness.c` via `tests/inbox_unit.rs`).

### E.1 Delivered
- The four §B.5 statics are now `_Thread_local` (`hl_stealth_depth` stays a named
  `extern _Thread_local` global — codegen emits it; the other three are
  `static _Thread_local`).
- `HiLowThreadContext` + mutex-guarded global registry, lazily created per thread
  (`hl_current_ctx`); holds the MPSC mutex-guarded `HiLowInbox`.
- Coalescing enqueue (`hl_inbox_enqueue`) keyed by watcher identity with mandatory
  delta accumulation (R3); watcher retained (R6), cell retained (axiom 5), delta
  ownership taken.
- `HiLowWatcher.owner_ctx`; routing in `hl_cell_notify` via `hl_notify_deliver`
  (same-thread → synchronous, byte-identical; else enqueue).
- Drain (`hl_thread_drain_inbox`, detach-under-lock-then-fire; ended watchers drop
  without firing, R5), safe-point hook (`hl_thread_safepoint` at `hl_object_new` /
  `hl_array_new` / `print_str`; dormant single-threaded), teardown
  (`hl_thread_final_drain`, axiom 6). `cc` gained `-pthread`.

### E.2 Adjudications / deviations (binding record)
| # | Deviation | Rationale |
|---|-----------|-----------|
| 5a-i | The four statics are `_Thread_local` **file-scope globals**, not `HiLowThreadContext` fields (the brief said "migrate into it"). | The per-thread invariant is what matters and it holds; `hl_stealth_depth` MUST stay a named global (generated C emits `hl_stealth_depth++`), so a struct field would force a codegen change. The context holds the inbox + registry — the parts that actually need cross-thread discovery. |
| 5a-ii | The inbox owns `+1` on the entry's cell via `hl_cell_retain`/`hl_cell_release`, but `hl_cell_release`-at-0 does only cell-header teardown (the typed container release frees the struct), so the inbox must not be the cell's **sole** owner at drain. | UNREACHABLE in single-threaded 5a (the inbox is dormant — no real program enqueues). Resolved in 5c: the only cross-thread cells are typed `shared` scalars, so the entry will carry the scalar's typed release and can free a solely-owned container. Not a reachable 5a bug (KNOWN_MEMORY_BUGS stays empty). |
| 5a-iii | The inbox entry snapshots `body_fn`/`env`/`cell`/`event` beyond the brief's `(watcher, delta, pending)` triple. | The body ABI is `body(env, cell, delta)`; the fire needs all three plus the event. `env` is the retained watcher's env (valid while retained, R6). |
| 5a-iv | Drain points are **runtime safe points** (allocation/syscall), not codegen-emitted statement boundaries. | Owner AskUserQuestion ruling: the CLAUDE.md phase line ("allocation/syscall boundaries") and the brief §5a ("statement boundaries") disagreed, and statement-boundary drains would require emitting `hl_safepoint()` per statement — a program-logic codegen diff that 5a's own expected-diff forbids. Runtime safe points keep codegen byte-identical. |

### E.3 Ordering non-guarantee (documented boundary, brief §5a)
Per drain pass: one cell's watchers fire in declaration order; one producer's
entries are observed in an order consistent with that producer's mutation order
under coalescing. There is **no global order across producer threads** — stated
here so it is a documented boundary, not a discovered surprise.

### E.4 Forward to 5b/5c
- 5b activates `test_watcher_reentrant_deferred` (ignored 2 → 1) and adds the first
  cross-thread fixtures (order-insensitive invariants only); threaded runtime mode
  (atomic refcounts iff `async`/`shared`) keeps the single-threaded corpus
  byte-identical.
- 5c resolves 5a-ii (typed `shared` scalars) and lands the `docs/hilow-design.md`
  keyword-unification edit (R4); shared containers and `(deep)` across `shared` are
  explicitly rejected.

### E.5 Phase-5b amendments (recorded 2026-07-21, before implementation)
Two clarifications adjudicated for 5b (also written into
`docs/cell-redesign-brief.md` "Phase-5b amendments"):
- **Same-thread firing stays synchronous (R1).** The inbox is the cross-thread
  path exclusively; same-thread fires run nested within `hl_notify_depth`
  (existing behavior, unchanged). The `#[ignore]`d `watcher_reentrant_deferred`
  fixture pinned a *deferred* same-thread output under the earlier "may defer"
  reading; its expected output is rewritten to the synchronous/nested result and
  the test is activated (renamed `watcher_reentrant_sync`). Owner AskUserQuestion
  ruling; the current binary already produces this output, so no runtime change.
- **Loop back-edge safe points, threaded mode only.** 5a's runtime-entry safe
  points (alloc/syscall) are joined by a codegen-emitted `hl_thread_safepoint()`
  at `for`/`while`/`loop` back-edges, emitted **only** when the program uses
  `async` (threaded mode). Without it a pure-compute loop never drains
  (`loop { }` event-loop starvation). Single-threaded programs emit nothing →
  corpus byte-identical by the mode switch.

## Part F — Phase 5b landed (2026-07-21): what was built + adjudications

Minimal `async`. A `Statement::Async` block (High mode) compiles to a
`void* fn(void*)` pthread entry (new `async_bodies` codegen buffer, before
`main`), spawned via `hl_async_spawn` and joined at program exit
(`hl_async_join_all`). Threaded runtime mode is engaged iff the program contains
`async` (pre-scan `program_body_has_async`); every threaded-mode emission is
gated on `uses_async`, so the single-threaded corpus is byte-identical
(**verified**: 352/352 compiling single-threaded programs byte-identical vs the
1c3bd2f binary; only diffs are the 5 new async fixtures + `modules/diamond`'s
pre-existing module-order nondeterminism).

### F.1 Delivered
- Parser/AST: `Statement::Async(Block, Position)`, parsed like `stealth`;
  typecheck arms mirror `StealthBlock`.
- Boxing: `walk_async` — an `async` block is a capture boundary (reuses the
  watcher-boundary machinery), so outer variables it references box and are
  reachable from the spawned thread as heap cells.
- Codegen `generate_async_block`: captures → heap env struct (`emit_watcher_env_struct`
  reused), packed with retained refs at the spawn site, released (`dtor` + `free`)
  when the body finishes; body references resolve to `env_cast->field` via
  `hoisted_variables`/`boxed_hoisted`; body ends `hl_thread_final_drain()`.
- Runtime: `hl_async_spawn`/`hl_async_join_all` (growable mutex-guarded thread
  list); `HL_RC_INC`/`HL_RC_DEC` atomic-under-`HILOW_THREADED` macros at all 8
  refcount owners + 3 ref helpers; `hl_alloc_count`/`hl_free_count` → `_Atomic
  int` in threaded mode.
- main.rs: `-DHILOW_THREADED` (both TUs, one `cc`) iff `codegen.uses_async()`.
- Codegen exit: join-then-main-drain on BOTH main-exit paths (explicit `return`
  and fall-through), before scope cleanup.

### F.2 Adjudications / deviations (binding record)
| # | Item | Resolution |
|---|------|------------|
| 5b-i | Same-thread firing: synchronous or deferred? The 5b re-entrancy statement says synchronous ("existing semantics unchanged", R1); the committed `watcher_reentrant_deferred` fixture pinned a deferred output; the brief (`cell-redesign-brief.md:63`) permits either. | Owner AskUserQuestion → **synchronous**. The inbox is the cross-thread path exclusively. Fixture activated, expected rewritten to the synchronous/nested output (which the current binary already produced), renamed `watcher_reentrant_sync`. |
| 5b-ii | Loop-backedge safe points are codegen-emitted (a program-logic codegen change), unlike 5a's runtime-only safe points. | Brief §5b amendment (recorded §E.5 before implementation): a runtime-only safe point never fires in a pure-compute `loop { }`. Gated on `uses_async` → zero single-threaded impact, so it does not reopen the 5a "byte-identical" constraint for non-async programs. |
| 5b-iii | Leak counters (`hl_alloc_count`/`hl_free_count`) are incremented from generated main.c via unchanged `hl_alloc_count++`; making them atomic must not change those bytes. | Declared `_Atomic int` under `HILOW_THREADED` (a C11 keyword, no stdatomic.h needed in main.c) → `++` on the atomic object is an atomic RMW; the generated bytes are identical, single-threaded stays plain `int`. |
| 5b-iv | Async captures a program-scope CONTAINER — but containers are main-locals, not statics (only boxed scalars hoist). | The heap env (retaining each captured pointer) is the uniform capture path for scalars AND containers, mirroring watcher envs. The `walk_async` boundary guarantees the referenced var boxes. |
| 5b-v | `async` is "High mode" but mode is unenforced anywhere in the checker. | No Low-mode gate added — it would be new infrastructure with no precedent. The designation is documentation; deferred with the rest of mode enforcement. |
| 5b-vi | Program-exit residual: the brief says "residual entries at end-of-world are released without firing" yet "the main thread drains last". | Reconciled: main's `hl_thread_final_drain` FIRES the in-flight entries (residual delivery, fixture 5); "released without firing" covers ended watchers (R5, dropped at drain) and any entry re-enqueued after the drain snapshot at teardown. In 5b, joined-then-drained main fires on its own thread synchronously, so no re-enqueue occurs. |

### F.3 Forward to 5c
- 5c resolves 5a-ii by giving the inbox entry the shared scalar's typed release.
- The helgrind lane is informational only (0 unsuppressed errors on the current
  fixtures); it stays non-gating through Phase 5.
- Not built in 5b (deferred, not silently dropped): arbitrary cross-thread
  mutation of shared CONTAINERS is unsafe (the container's internal state races,
  not just its refcount) — safe cross-thread mutation of scalars is `shared`
  (5c); multiple producers mutating the SAME cell race on the value (permitted,
  future prover warning). The 5b fixtures use single-producer-per-cell or
  disjoint-cell patterns accordingly.

## Part G — Phase 5c amendments (recorded 2026-07-21, before implementation)

Two additions this prompt makes to the brief, recorded here per the "record
before implementation" rule (the brief permits either but does not specify
these mechanisms):

### G.1 Per-shared-cell subscriber lock (structural synchronization — the phase's real content)
A shared cell's subscriber list may be mutated by its declaring context (a watch
declared, a watcher `.end()`ed) while producer threads are concurrently
`hl_cell_notify`ing it. WITHOUT a lock the 3e-β collect-then-fire snapshot races
the list mutation (a `free`d node walked, a half-linked `next`). Amendment: each
**shared** cell carries a mutex (`pthread_mutex_t* sub_lock`, non-NULL ⟺ shared,
allocated at construction). `hl_cell_subscribe` / `hl_cell_unsubscribe_*` take it
when the cell is shared; `hl_cell_notify` on a shared cell takes it, runs
`snapshot_cell_nodes` under it, releases it, THEN walks the snapshot
(fire/enqueue) with the lock NOT held (a body may re-subscribe or notify without
deadlock). Non-shared cells: `sub_lock == NULL`, no lock taken, the notify/
subscribe fast path is byte-for-byte the pre-5c path guarded by one predictable
`if (c->sub_lock)` branch. Watchers snapshotted-then-ended before delivery drop
at drain (R5) on the cross-thread path, and — pinned against the existing
same-thread `.end()` semantics — fire at most once post-end only where the
current same-thread behavior already does.

### G.2 Graduation of deviation 5a-ii (cross-thread last-release of a typed cell)
5a-ii: `hl_cell_release` at refcount 0 does only cell-header teardown; the typed
struct/payload teardown lives caller-side in `hl_array_release` /
`hl_object_release` / `hl_scalar_release`, so the inbox (which held a RAW cell
`+1` via `hl_cell_retain`/`hl_cell_release`) leaked the container if it was the
sole owner at drain. Reachable in 5c: a shared scalar's final release can land on
the async (producer) thread via the inbox. FIX (release path only): `HiLowCell`
gains a `kind` tag (`HL_CELL_SCALAR`/`ARRAY`/`OBJECT`, set at construction); each
typed release's post-`hl_cell_release` teardown is factored into a `*_finalize`
function; a new `hl_cell_release_full(HiLowCell*)` dispatches on `kind` at
refcount 0; the inbox's cell release (`hl_inbox_fire_entry`) routes through
`hl_cell_release_full`. The existing typed releases are unchanged in behavior
(they call the same finalize). Proven by a C-harness case (`inbox_unit`) where
the inbox is the SOLE owner at drain and the container is freed valgrind-clean.
5a-ii moves to extinct (§D.2) on landing.

## Part H — Phase 5c landed (2026-07-22): `shared` scalars; Phase 5 closure

`shared let` i32 scalars: refcounted + atomic + cross-context watchable. Every
threaded-mode emission is gated on `threaded_mode()` (= `uses_async || uses_shared`),
so the single-threaded corpus is byte-identical (verified vs bc51de4: the only
diffs are the new async/shared fixtures + `modules/diamond`'s pre-existing
module-order nondeterminism).

### H.1 Delivered
- Parser/AST: `LetDecl.is_shared`; `shared let` parsed via a `TokenKind::Shared`
  dispatch prefixing `let` (module-level shared is out of 5c scope).
- Boxing: `shared` lets box (`BoxReason::Shared`) — a shared scalar is a cell
  whether or not it is watched.
- Runtime: `HiLowCell.sub_lock` (`pthread_mutex_t*`, non-NULL ⟺ shared);
  `hl_scalar_new_i32_shared` allocates it. Payload access branches on it:
  `hl_scalar_get_i32` atomic-loads, `hl_cell_set_i32` atomic-exchanges then
  notifies (shared cells skip the racy `cell_has_audience` pre-check and always
  route through `hl_cell_notify`). Subscriber lock: `hl_cell_subscribe` /
  `hl_cell_unsubscribe_watcher[_origin]` take it; `hl_cell_notify` snapshots the
  subscriber list under it then fires the snapshot unlocked. Non-shared cells:
  NULL lock, fast path unchanged. `sub_lock` destroyed/freed in the cell's
  header teardown.
- Codegen: `uses_shared` pre-scan → `threaded_mode()`; `shared_vars` populated in
  the program-body pre-pass; the boxed-i32 let uses `hl_scalar_new_i32_shared`
  for shared vars (reads/writes need no codegen change — the runtime branches).
  `-DHILOW_THREADED` now keyed on `threaded_mode()` (main.rs).
- Spec R4 (`docs/hilow-design.md`): `shared` unified to one meaning (refcount +
  atomic + cross-context watchable), reconciling the memory-mode text with the
  concurrency text.

### H.2 Adjudications / deviations (binding record)
| # | Item | Resolution |
|---|------|------------|
| 5c-i | Atomic payload access: separate atomic accessors (codegen-selected) vs a runtime branch on `sub_lock`. | Runtime branch. `hl_scalar_get_i32`/`hl_cell_set_i32` are out-of-line calls (no register hoist across them, so even the plain path re-reads in a caller's loop); branching inside keeps the generated main.c byte-identical (same call names) and adds no cost to non-shared cells (a predictable NULL check). Only the construction differs (`_shared`). |
| 5c-ii | `x += 1` on shared: atomic-add vs two atomic ops. | Two atomic ops (atomic load + atomic store), i.e. a racy RMW — the prover's future warning, not a 5c error. The `(atomic-add)=` family stays deferred (brief §6). The spec's `+= 1 // ✓ atomic` (design:2186) is aspirational for that family; 5c does not implement it. |
| 5c-iii | `(deep)` across `shared`: dedicated diagnostic vs subsumption. | Subsumed. `shared` is scalar-only (shared containers rejected), so `(deep)` on a shared var is `(deep)` on a scalar, already rejected by the type checker ("(deep) modifier requires an array or object type"). No separate codegen check (it would be unreachable dead code); pinned by `test_shared_deep_rejected` against the typecheck message. The shared-container rejection has its own explicit codegen diagnostic (`test_shared_container_rejected`). |
| 5c-iv | `shared` is a memory-mode keyword (High/Low); mode is unenforced. | Program-scope `shared let` scalar only in 5c (matches the fixtures and the i32 payload matrix). Function-scope and non-i32 shared are deferred; non-i32 shared is rejected at the let with a clear diagnostic. |
| 5c-v | Cross-thread SOLE-owner-at-drain (async thread last-release) is hard to force from HiLow source (main joins async before its scope cleanup, so the binding co-owns the cell through the drain). | The exact teardown-at-zero is pinned by the C-harness (`case_inbox_sole_owner_frees_{array,scalar}`, each leaks 152B pre-fix). `shared_exit_release` pins the HiLow-level inbox-release path on a shared cell is leak-clean. |

### H.3 Phase 5 closure (what the notification queue guarantees, and what it does not)
- **Guarantees:** a write to a watched cell fires that cell's watchers on their
  DECLARING thread — same-thread synchronously (nested, R1), cross-thread via
  the declaring thread's coalescing MPSC inbox drained at safe points; a shared
  scalar's payload is atomic and its subscriber list is safe against concurrent
  declare/end vs notify; refcounting is atomic in threaded mode, so a cell is
  freed exactly once from whichever thread lands the last release (5a-ii).
- **Does NOT guarantee:** no global ordering across producer threads; on the
  cross-context path rapid changes may COALESCE (a watcher may observe the
  current value at fire time, not every intermediate — design:1782); `shared`
  containers and `(deep)` across `shared` are deferred (rejected); plain
  `+= 1` on shared is a racy RMW (future prover warning), and the
  `(atomic-add)=` family is deferred.

## Part I — Phase 6a landed (2026-07-23): cross-process `shared` by placement

Governed by `docs/phase6-brief.md` (rulings R-A–R-E), NOT audit §6 (SUPERSEDED).
Cross-process `shared` is a typed slot in a named POSIX shm segment mapped by
every participant — **placement, not transport** (nothing serialized/sent). The
"sendable check" landed as **placeability** rejection diagnostics, not a phase.
Existing corpus byte-identical (322 compiling programs, 0 diffs vs 84f707b —
every codegen change is gated on `shared_segment.is_some()`/`placed_vars`/
`uses_placement`, all empty for a program without `shared("...")`).

### I.1 Delivered
- Surface/AST: `LetDecl.shared_segment: Option<String>`; parser accepts
  `shared("name") let` (optional parenthesized string literal before `let`).
  Bare `shared` unchanged (5c). Invariant `shared_segment.is_some() ⟹ is_shared`.
- Placeability fence (codegen, next to the 5c i32 fence): a placed non-i32
  declaration is rejected in placeability language (container → pointer-bearing;
  watcher → watcher-bearing; function → pointer/env-bearing). Segment-name
  validation `[A-Za-z0-9._-]+`, 1–64 chars.
- Runtime: `HiLowCell.shm` (`HiLowShmSegment*`, non-NULL ⟺ placed). Versioned
  header { magic, layout_version, abi_version, type_tag, payload_size,
  init_state, epoch:u64 } + 8-aligned payload. `hl_shm_attach_i32` does
  create-or-attach per R-D: `O_CREAT|O_EXCL` winner initializes and
  release-publishes init-complete; loser attaches, bounded-waits (timeout →
  startup error, exit 1), verifies every header field (mismatch → startup
  error), observes, fires nothing. `/hilow.<name>`, 0600, persistent (no
  unlink). Accessors gain a 3rd branch: placed → mapped-slot acquire-load /
  exchange+epoch-bump(release); the write protocol is complete though nothing
  pulls the epoch until 6b. Placed ⊃ shared (sub_lock also set — in-process
  subscriber machinery intact). Header teardown detaches (munmap+close).
- Codegen: `hl_scalar_new_i32_placed("seg", <init>)` for placed i32;
  `uses_placement` → main.rs links `-lrt`.
- Two-process harness (`tests/xproc_harness.rs` + `tests/xproc/share/{a,b}.hl`):
  rewrites the segment name uniquely per run, compiles both halves, runs both
  under valgrind in a controlled order, asserts both outputs+exit codes, unlinks
  its segment. Both orderings (A-then-B, B-then-A) prove creator-initializes,
  attacher-ignores-init, persistence, same-name sharing.
- C-fork unit harness (`tests/shm_unit.rs` + `tests/shm_unit_harness.c`): the
  header-mismatch (type/magic), init-timeout, and empty-name paths are
  UNREACHABLE from HiLow (i32-only, and `""` does not lex), so they are proven
  at the C level via a forged segment (`hl_shm_test_forge`, compiled only under
  `-DHL_SHM_TEST_SUPPORT` — zero production footprint) and `fork()` exit-code
  checks. 5 cases (happy attach, type mismatch, bad magic, timeout, empty name).

### I.2 Adjudications / deviations (binding record)
| # | Item | Resolution |
|---|------|------------|
| 6a-i | Where does the sendable/placeability check live? | Codegen, next to the 5c i32 fence (same precedent). Placed ⟹ i32-only; non-i32 gets placeability-worded diagnostics. Not a transitive call-site check — there is no `send`, so nothing to launder through a call (resolves the parked boundary-locality question: per-declaration fence). |
| 6a-ii | Placed cell = superset of shared? | Yes. `hl_scalar_new_i32_placed` builds on `_shared` (gets sub_lock), so the in-process subscriber list stays cross-thread-safe; only the payload storage moves to the mapped slot. |
| 6a-iii | "runs BOTH under valgrind in the gate lanes" | Interpreted as gate-equivalent valgrind rigor in a dedicated two-process harness. Cross-process fixtures cannot live under tests/programs/ (auto-discovered as standalone single-process); they live under tests/xproc/ and the harness orchestrates + valgrinds both halves. |
| 6a-iv | Type-mismatch / timeout / empty-name tests | UNREACHABLE from HiLow source (only i32 is placeable; empty string literals do not lex — pre-existing). Proven at the C level (fork + forged segment). The empty-name codegen branch is defense-in-depth; the length rule (65-char name lexes) is pinned at the HiLow level (placed_long_name_rejected). |
| 6a-v | Attacher ignores its initializer | R-D: `shared("n") let x = 5` in an attacher does NOT set 5 — the creator initialized; the attacher observes the current value. The init expression is still evaluated (side-effect parity) then ignored. Documented loudly (runtime comment + diagnostics reference). |
| 6a-vi | Segment cleanup | Persistent by default (no unlink at detach — the separately-launched-programs model). The harness always unlinks its own segments; a leaked test segment is a harness bug. Cleanup policy proper is Phase 6c. |

### I.3 Forward to 6b (delivery — NOT in 6a)
- Nothing pulls the epoch cross-process yet. 6b: the declaring thread tracks
  (segment, last_seen epoch, last_delivered value) and, at existing drain safe
  points, acquire-loads the epoch; on advance, delivers locally per R3.
  `(assigned)` fires on epoch advance (at-least-once, coalescible);
  `(changed)` fires iff drain-time value ≠ last-delivered. Backoff polling; the
  spec's Cross-Process Watchers example becomes a real 2-program fixture.
