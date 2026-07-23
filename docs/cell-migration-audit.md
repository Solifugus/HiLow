# Cell-Model Migration Audit (Phase 1 of docs/cell-redesign-brief.md)

Date: 2026-07-14. This is the Phase 1 deliverable of the cell redesign brief: a
migration-impact audit. No code was changed in producing it.

**Line-number anchors.** Codegen references (`src/codegen/mod.rs`) are against
commit `a21a6de` (branch `lower-stmt-exprs` = main `831e209` + one 11-line
inert commit to codegen). Runtime references (`src/runtime/runtime.{c,h}`) are
identical on both branches. On main, codegen line numbers after ~line 135 are
smaller by up to 11.

All load-bearing claims below (in particular §3.4's latent bugs) were verified
by reading the cited code directly, not taken from summaries.

---

## 1. Firing-site inventory

Every location where watcher-firing code is injected by codegen, and every
runtime mutation site that fires array watchers. Third column: what replaces it
under the cell model.

### 1.1 Codegen-injected scalar assignment firing

All in `src/codegen/mod.rs`, inside the assignment-statement handler.

| Site | Location | Cell-model replacement |
|---|---|---|
| Combined firing block (entry) | 1999–2136: gated on `AssignOpKind::Assign` + `Expression::Ident` target; consults `watcher_subscribers` (2002) and `heap_watcher_subscribers` (2003); early-returns at 2133, bypassing the normal-assignment path | Assignment to a boxed variable compiles to `hl_cell_set(cell, value)`; the entire block is deleted |
| changed-path | 2010–2092: emits `{ old = var; var = expr; if (hl_stealth_depth == 0) { if (old != var) { …fire changed… } …fire assigned… } }`. Decl-form fires guard on `hilow_watcher_<id>_active` (2032); heap-form on `w != NULL && w->active && !w->ended` (2048–2051) | equality check + notify live inside `hl_cell_set`; stealth check moves into the runtime (one place instead of per-site) |
| assigned-only path | 2093–2132: plain assignment then stealth-guarded fires | same |
| Compound-assign rejection | 1988–1997: `+=` etc. to a watched variable is `UnsupportedFeature` | `hl_cell_set` composes with any read-modify-write; restriction can be lifted (post-migration follow-up, not required) |
| Call-arg emission helpers | `emit_watcher_call_args` 6927–6943, `emit_watcher_call_args_from_names` 6909–6925, `extract_watcher_id` (string-parses the id out of the fn name!) 6865–6877 | deleted; watcher bodies receive `(env, cell, delta)` uniformly |

### 1.2 Codegen-emitted activation / scope-exit deactivation

| Site | Location | Cell-model replacement |
|---|---|---|
| `generate_block` Phase 3 registration | 739–747: assigns watcher id, `generate_watcher`, `register_watcher_subscriptions`, `watcher_name_to_id` insert | `let`-like lowering: construct a runtime watcher value, subscribe it to cells |
| `generate_block` Phase 4 activation | 753–760: emits `hilow_watcher_<id>_active = true; hilow_watcher_<id>_ended = false;` at declaration point | watcher value is created active; no emitted flag writes |
| `generate_block` Phase 5 deactivation | 767–773: emits `hilow_watcher_<id>_end();` for every nested watcher at block end — **dead code under early return** (STATUS "Active deferred work") | scope exit emits a single runtime release of the watcher value (same path as any heap value); deactivation is `hl_watcher_end` semantics inside the runtime |
| `generate_block_with_parameter_context` duplicates | 805–813 / 819–826 / 833–839: identical Phase 3/4/5 loops (known duplication, STATUS) | both duplicates deleted; the duplication itself disappears with the emitted code |
| Program-body activation | `emit_main_function` 6563; activation loop 6577–6584 emits `_active = true / _ended = false` for all top-level watchers at `main()` start; their registration happens in `generate_program_body_functions` 882–889 | same as blocks — top level is not special |

### 1.3 Codegen-emitted per-watcher state and methods

| Site | Location | Cell-model replacement |
|---|---|---|
| Static bools | `generate_watcher` 6824–6829: `static bool hilow_watcher_<id>_active/_ended` | fields on the runtime watcher object (a `HiLowWatcher`-like header already exists: `runtime.h:153–157` with `refcount/active/ended`) |
| Four static helpers | 6832–6847: `_pause/_resume/_end/_isActive` C functions per watcher | `hl_watcher_pause/resume/end/is_active` already exist (`runtime.c:754–800`); they become the only implementation |
| Method dispatch, decl-form | 3552–3596 (`"pause"` arms at 3555/3578): decl-form dispatches to the static helpers via `watcher_name_to_id`; heap-form already dispatches to the runtime `hl_watcher_*` functions | decl-form joins the heap-form path; the static-helper dispatch arm is deleted |
| Watcher body emission | decl-form signature 6772–6793 (subscribed vars as value params); heap/array-form signature 2570–2594 — array watchers get `(void* env, HiLowArray* x, void* delta)` (2581) | one body ABI for all watchers: `(env, cell, delta)` |

### 1.4 Codegen-emitted array watcher (de)registration

| Site | Location | Cell-model replacement |
|---|---|---|
| Register | 1287–1306: per subscription, maps modifier → `HL_ARR_*` and emits `hl_array_register_watcher(arr, mod, body_fn, env, watcher_state)`; records into `array_watcher_registrations` (1302–1305) | registration survives as cell subscription (`hl_cell_subscribe`), but the codegen-side registration *tracking* is deleted — the watcher owns its env, so nothing needs to be unregistered at scope exit on the env's behalf |
| Env malloc + packing | 1247–1285: emits `malloc(sizeof(hilow_array_watcher_env_N))`, packs scalars by `&var`, arrays by pointer; tracked as `HeapType::Environment` (1282) | env is allocated once, refcounted, owned by the watcher object; scope owns a reference to the *watcher*, not the env |
| Unregister at scope exit | `emit_scope_cleanup` Environment arm 6329–6344; `emit_early_return_cleanup` duplicate 6473–6491; both guarded by a scope-depth comparison to avoid double-free | deleted entirely — deactivation/teardown is `hl_watcher_release`, a runtime call, reachable from every exit path because it is the same call scope cleanup already makes for heap values |
| Temp-env free | `emit_temp_cleanup` Environment arm 6420–6422: plain `free`, **no unregister** | deleted (envs are watcher-owned); see §3.4(d) for the latent bug this arm carries today |

### 1.5 Runtime mutation firing sites (`src/runtime/runtime.c`)

These stay in the runtime under the cell model (the brief's "one firing path")
but iterate the cell's watcher list and enqueue/notify via the cell header
instead of the per-array `watchers` list.

| Mutator | Function | Firing loop | Fires |
|---|---|---|---|
| push | `hl_array_push` 1735–1770 | 1753–1769 | ADDED (delta = elem), CHANGED (NULL) |
| pop | `hl_array_pop` 1787–1821 | 1800–1816 | REMOVED (delta = removed slot), CHANGED |
| index-assign | `hl_array_set` 1823–1861 | 1846–1860 | CHANGED only |
| remove | `hl_array_remove` 1863–1908 | 1888–1903 | REMOVED (delta = static `temp_buffer`), CHANGED |
| insert | `hl_array_insert` 1910–1961 | 1945–1960 | ADDED (delta = elem), CHANGED |
| move (from==to) | `hl_array_move` 1978–1997 | 1981–1996 | MOVED (delta = `&(from,to)`), CHANGED — **2-arg call, drops env; see §3.4(a)** |
| move (real) | 2000–2044 | 2028–2043 | MOVED, CHANGED — **same 2-arg bug** |
| clear | `hl_array_clear` 2047–2072 | 2062–2071 | CHANGED only (ADDED/REMOVED/MOVED deliberately silent) |

Supporting definitions: `HiLowArray` `runtime.h:262–271` (refcount, length,
capacity, elem_size, data, `watchers`, retain_fn, release_fn);
`HiLowArrayWatcher` node `runtime.h:276–282` (modifier, body_fn, env,
watcher_state, next); modifier constants `runtime.h:285–288` — `HL_ARR_ADDED 1,
REMOVED 2, CHANGED 3, MOVED 5`. There is **no** `HL_ARR_DEEP` (value 4 is a
gap); "DEEP" survives only in comments. The cell model's `(deep)` parent-walk
is new work, not a retrofit of an existing constant.

### 1.6 Stealth (`hl_stealth_depth`)

Global `int hl_stealth_depth` (`runtime.c:47`, `runtime.h:441`). Nine firing
sites check it: the 8 runtime loops above plus the codegen-emitted scalar check
(mod.rs 2025 / 2100). `stealth { }` blocks emit `hl_stealth_depth++/--`
(mod.rs 4784 / 4800) and **reject early `return` inside the block**
(4772–4783). Cell model: the check collapses into `hl_cell_set` / the runtime
mutators — one site. The counter must become per-thread when Phase 5 (queues)
lands; with the declaring-thread model, "stealth" is naturally a property of
the writing thread.

---

## 2. Name-keyed state inventory

Every codegen map/set keyed by lexical name, on `CodeGenerator`
(`src/codegen/mod.rs:96–182`). "Fate" is under the completed cell model.

| Field | Decl | Exists to work around | Fate |
|---|---|---|---|
| `watcher_subscribers: HashMap<String, Vec<WatcherSubscription>>` | :158 | name-based subscription: assignments must know at compile time who watches a *name*. Requires the surgical shadow-masking save/restore at function boundaries (672–705) and `collect_local_variable_names` + recursive helpers (6946–7015) | **deleted outright** — identity subscription makes shadowing trivially correct |
| `heap_watcher_subscribers: HashMap<String, Vec<HeapWatcherSubscription>>` | :169 | same, for expression-form watchers; masked by the same 672–705 block | **deleted outright** |
| `watcher_name_to_id: HashMap<String, usize>` | :165 | maps watcher names to static-bool ids for activation (758/824/6580) and method dispatch (3555+) | **deleted** — a watcher is a runtime value; methods are runtime calls |
| `scalar_watcher_captures: HashSet<String>` | :178 | captured vars inside scalar-watcher bodies need `*ptr` access (2369); cleared, not scope-saved (2740) | **deleted** — captures live in the watcher-owned env, accessed uniformly |
| `array_watcher_registrations: HashMap<String, (String, usize)>` | :181 | env→(array, scope_depth) so scope cleanup can unregister before freeing the env (the 2026-05-30 UAF fix) | **deleted** — watcher owns env; scope death cannot orphan a registration |
| `temp_watcher_expr_body_fn / _subscriptions / _captured_vars` | :172–176 | side-channel from `WatcherExpr` generation to the enclosing `let` handler (set 2754–2761, drained 1227–1231) | **deleted** — watcher construction becomes a single expression producing a runtime value |
| `heap_owners: HashMap<String, (HeapType, usize)>` | :126 | general heap-ownership tracking; `std::mem::take` at fn boundary (653/691) fixes name-collision pollution | **survives** (not watcher-specific). Interacts with Phase 3: a boxed scalar becomes a heap value this map must track (`HeapType::Cell` or reuse). The `HeapType::Environment` arm of all three cleanup routines is deleted |
| `transferred_vars: HashSet<String>` | :139 | ownership-transfer on return; take/restore 646/689 | **survives**; factory-pattern watchers become ordinary transferred heap values, so the watcher-specific escape analysis simplifies |
| `temp_owners` (:129), `pending_statement_decls` (:133), `temp_counter` (:131) | | statement-end temporary cleanup (Phase 11a) | **replaced** in migration Phase 4 by per-statement release lists through the single cell-release function. *(4a, 2026-07-19: `temp_owners` became the sole temp mechanism — the store-site release path was folded in and deleted; dead `scope_depth` dropped. It survives as the per-statement release list, not literally removed. See §5 Phase 4.)* |
| `pending_statement_stmts: Vec<String>` | :135 | inert landing pad (commit `a21a6de`): flushed at 1027–1032 before each statement body, **no producers yet** | groundwork for Phase 4 — producers get wired in then. *(4a, 2026-07-19: this landing pad is from the ARCHIVED compiler — `a21a6de` is not an ancestor of the fresh tree's HEAD and this field never existed here. The fresh tree realized 4a on `temp_owners` instead; see §5 Phase 4.)* |
| `variable_types` (:110), `hoisted_variables` (:115) | | general symbol/type info and closure hoisting | **survive**; Phase 3's boxing pass adds a "boxed?" attribute alongside `variable_types` (or a parallel set) |

The shadow-masking machinery (672–705 + 6946–7015) exists **only** to serve the
first two maps and is deleted with them. `WatcherSubscription` (:9–20) and
`HeapWatcherSubscription` (:24–36) structs are deleted.

---

## 3. Env and lifetime inventory

### 3.1 Allocation

The runtime never allocates envs; it stores the opaque `void*` it is handed
(`hl_array_register_watcher`, runtime.c:2075–2083). All allocation is
codegen-emitted:

- **Array-watcher envs**: mod.rs 1247–1285 — `malloc(sizeof(hilow_array_watcher_env_N))`,
  scalars packed by `&var` (by-reference), arrays by pointer (identity). Env
  struct typedef emitted at 2680–2694. `env == NULL` when no captures (1249).
- **Closure envs** (function expressions): mod.rs 5596–5660, struct
  `hilow_anon_N_env`, handed to `hl_function_new_with_env` (runtime.c:744–751).
  Same mechanism the array-watcher path was adapted from.

### 3.2 Ownership and free paths

Envs are `HeapType::Environment`, **owned by the declaring scope**, freed with
plain `free()` (not a refcounted release) in three places:

1. `emit_scope_cleanup` (6326–6344) — normal block exit. Unregisters from the
   array first *iff* the registration's scope differs from the array's scope
   (6333) or the array isn't in `heap_owners` (6337).
2. `emit_early_return_cleanup` (6473–6491) — return/break/continue; duplicate
   of the same logic.
3. `emit_temp_cleanup` (6420–6422) — statement-end temp envs; **frees without
   unregistering**.

`hl_array_release` (runtime.c:1709–1733) frees the watcher *nodes* when the
array dies (1722–1728) but never the env or watcher_state ("owned by the
binding"). `hl_array_unregister_watcher` (2085–2097) unlinks and frees one node
matched by **env pointer identity** (2089).

### 3.3 Bug-class participation

| Path | UAF on scope death | Unreachable deactivation (early return) | static temp_buffer | Expression temporaries |
|---|---|---|---|---|
| env alloc + register (1247–1306) | origin of the fixed 2026-05-30 UAF; residual risks in §3.4(b,c) | — | — | — |
| `emit_scope_cleanup` env arm | carries the fix (unregister-before-free) | not reached on early return (by design; the early-return path duplicates it) | — | — |
| `emit_early_return_cleanup` env arm | carries the fix for early exits | envs are handled here, but scalar-watcher `_end()` deactivation is **not** — it is only emitted at block end (767–773/833–839) and is dead under early return (STATUS known issue) | — | — |
| `emit_temp_cleanup` env arm | **latent UAF** — §3.4(d) | — | — | frees temp envs |
| `hl_array_remove` (1873, returned at 1907) | — | — | **yes** — non-reentrant, 1024-byte cap, and the removed element is *returned to the caller* through the static buffer | — |
| `hl_array_move` (2002) | — | — | **yes** — a second, independent static buffer (STATUS records only the remove one) | — |
| move firing sites (1993, 2040) | — | — | — | not a leak; ABI bug, §3.4(a) |
| `temp_owners` / `pending_statement_decls` machinery | — | — | — | this is the *mitigation* for expr-temp leaks; string operand temporaries still leak (STATUS open question) — the cell model's Phase 4 release lists are the designed fix |

### 3.4 Latent bugs found during this audit (all verified in source)

These are recorded here, not fixed (Phase 1 is doc-only). Each is in code the
migration deletes or rewrites; the refined plan (§5) says where each dies.

- **(a) `.move` drops the env when firing.** Both move firing sites call
  `((void(*)(HiLowArray*, void*))w->body_fn)(arr, delta_ptr)` — 2 args
  (runtime.c:1993, 2040) — while every other mutator calls the 3-arg env-first
  form and codegen emits every array-watcher body as
  `void f(void* env, HiLowArray* x, void* delta)` (mod.rs:2581). A `(moved)`
  or `(changed)` watcher **with captured variables** that fires from `.move`
  reads the array pointer as its env → garbage captures / likely segfault.
  Existing move tests pass because no fixture combines `.move` with captures.
- **(b) One env registered on multiple arrays unregisters from only one.**
  The registration loop (1287–1306) re-inserts the same `env_var_name` key
  per subscription, so `array_watcher_registrations` keeps only the **last**
  array; scope cleanup unregisters the env from that one array, leaving
  dangling nodes on the others → UAF when they later fire. Reachable by a
  multi-subscription array watcher with captures (e.g.
  `watcher((added)xs, (added)ys) { …capture… }`) where an array outlives the
  declaring scope.
- **(c) No-capture registrations key on the literal string `"NULL"`.** When
  `captured_vars` is empty, `env_var_name = "NULL"` (1249) and 1302 inserts
  under key `"NULL"` — a second no-capture watcher in the same scope
  overwrites the first's entry. Benign today only because a NULL env is never
  freed/unregistered, but it makes the map's contents unreliable.
- **(d) Temp-env free skips unregister** (6420–6422). Safe only under the
  assumption that a statement-temporary env is never registered on an array
  that survives the statement. Nothing enforces that assumption.

### 3.5 What the cell model does to §3

Watcher-owned refcounted envs delete the entire class: no scope-owned env means
no unregister-before-free, no scope-depth comparison, no early-return
duplicate, no temp-env special case — (b), (c), (d) become unrepresentable.
Queued delta *values* replace both static temp_buffers, fixing reentrancy and
the 1024-byte cap — and `hl_array_remove`'s return value must move to a
caller-owned copy at the same time (the return at 1907 shares the buffer with
the delta). (a) disappears when all bodies share the `(env, cell, delta)` ABI
through one notify path.

---

## 4. Risk map

### 4.0 Test-suite baseline (literal, 2026-07-14, branch `main`, `cargo test --no-fail-fast`)

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: FAILED. 226 passed; 13 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.66s
test result: ok. 110 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The 13 integration failures are the char*/HiLowArray* representation-split
failures (the current CLAUDE.md phase): `test_closure_string_capture_integration`,
`test_for_in_basic/mixed_types/proto_excluded_integration`,
`test_match_as_expression/block_body/string_integration`,
`test_method_this_basic/proto_integration`, `test_switch_string_integration`,
`test_unknown_basic/narrowing_in_else/optional_return_integration`.

**The brief requires every migration phase to keep the suite green. The suite
is not green now.** The representation-split fix must complete (or the 13 be
explicitly reconciled) before migration Phase 2 starts — otherwise "keep it
green" has no meaning. One of the 13
(`test_closure_string_capture_integration`) directly pins capture machinery the
migration touches.

### 4.1 Harness weakness (cross-cutting)

There is **no valgrind or ASan anywhere in the harness** — every "no leak" /
lifetime test asserts only `exit_code == 0` + empty stderr + golden stdout
(the exit-code check works because the runtime counts `hl_alloc_count` /
`hl_free_count`, which does catch missed frees, but catches neither
use-after-free nor double-free-then-lucky-survival). Phases 2–3 rewrite
exactly the machinery whose failure mode is silent memory corruption. Valgrind
(or ASan) gating on the lifetime tests is the single highest-value hardening
step before Phase 2.

### 4.2 Phase 2 (arrays) — what can break, and what pins it

| Behavior at risk | Pinned by (tests/integration_tests.rs) |
|---|---|
| Modifier firing per mutator (push/pop/index-assign/remove/move/clear; deltas; aliases) | ~28 tests: `test_array_watcher_*` (3138–3366), `test_array_remove_*` (3387–3482), `test_array_move_*` / `test_array_moved_*` (3883–3978), `test_array_clear_*` (3988–4064) |
| Closure capture semantics (by-reference scalars, identity arrays, multiple captures) | `test_array_capture_*` (4425–4482) |
| Env lifetime across scope death | `test_array_watcher_dies_with_scope_integration` (4501) — the one direct UAF probe; `test_array_capture_no_leak_integration` (4539) |
| Refcount neutrality / element retain-release | `test_array_move_objects_no_leak` (3940), `test_array_clear_objects_no_leak` (4007), object-array suites |
| Stealth suppression of array firing | `test_stealth_basic_array` (4178), `test_stealth_all_array_ops` (4197), `test_stealth_nested/after_exit/leak_check` (4235–4273) |
| Strings ride on HiLowArray, so the cell header lands under every string | `test_string_*` (4561–4669), f-string cleanup tests (1574, 1594) — plus whatever the representation-split fix adds |

Specific Phase 2 risks: the cell header changes `HiLowArray`'s layout —
anything that sizeof's or stack-copies arrays breaks silently; `.remove`'s
caller-visible return-through-static-buffer must change when the buffer dies
(runtime.c:1907); watcher-owned envs invert the free direction, so the
flat-scope double-free case the scope-guard currently protects (6333) needs an
equivalent answer in refcounting.

### 4.3 Phase 3 (scalars) — what can break, and what pins it

| Behavior at risk | Pinned by |
|---|---|
| changed vs assigned semantics, multiple subscriptions | `test_watcher_changed_*`, `test_watcher_assigned_*` (2474–2514) |
| pause/resume/end/isActive (both forms) | 2534–2574, `test_watcher_expression_methods` (2698) |
| Scope-bounded activation; pre-declaration silence | 2616–2656 |
| Expression-form lifecycle + heap release | 2678–2791 |
| Factory pattern (escaping watchers) | `test_watcher_factory_*` (2811–2831) |
| Shadowing correctness | `test_three_level_shadow_probe` (2851), `test_same_name_caller_callee_integration` (4368) — these pin the behavior the shadow-masking machinery exists for; they must pass **unchanged** when that machinery is deleted |
| Captures by-reference | `test_scalar_watcher_capture_*`, `test_by_reference_sees_current`, `test_multiple_captures`, `test_nested_watchers` (4292–4406) |
| Compile-time rejections | `test_watcher_expression_return_rejected` (2738), `test_watcher_escape_function_local_rejected` (2871) — **assert error-message text verbatim**; boxing makes escape legal-by-construction, so these tests encode a language decision: does the reachability restriction stay as spec, or fall away? Adjudicated 2026-07-14 (see §5 "Open questions — adjudicated" item 1): the restriction drops in Phase 3, in the same commit as the boxing pass |
| Stealth on scalars | `test_stealth_basic_scalar` (4159), `test_stealth_dynamic` (4216) |

Specific Phase 3 risks: the boxing pass changes variable access shape
(`x` → `cell->value` or equivalent) everywhere a watched name is read —
interactions with closures/env-hoisting (`hoisted_variables`) and with
`heap_owners` cleanup are the two seams where past bugs clustered; watched
variables that are *also* captured by function expressions now have two boxing
mechanisms meeting.

### 4.4 Gaps — new tests needed BEFORE migration

Behaviors the migration touches that no current test pins:

1. **Re-entrant mutation inside a watcher body** (body calls `.remove`/`.push`
   on another — or the same — array). Exercises the static temp_buffer
   clobbering directly. Must exist before Phase 2 replaces the buffers, both
   as a bug demonstration and as the regression pin.
2. **`.move` with captured variables** — currently would hit §3.4(a); write it
   at the start of Phase 2 (it cannot pass before the fix).
3. **Multi-array watcher with captures + surviving array** — pins §3.4(b).
4. **Early return + watcher lifetime** — a function that declares a watcher
   (scalar and array variants) and returns early; mutation after the early
   return must not fire. Pins the unreachable-`_end()` hole before Phase 3
   replaces the mechanism.
5. **Shadowing + array watchers combined** — shadowing is pinned for scalars
   only; identity subscription must be proven on the array path too.
6. **Insert-at-index watcher firing** — `hl_array_insert` fires ADDED/CHANGED
   (1945–1960) but no watcher test drives insert.
7. **Array-watcher factory** (watcher on an array returned/escaping) — factory
   is pinned for scalars only.
8. **Watcher on a string** — strings are HiLowArray<u8>; Phase 2's cell header
   lands under strings, and managed-strings sub-phase 4 plans `(changed)s`.
   At minimum, a test that registering a watcher on a string either works or
   is cleanly rejected (today: undefined territory).
9. **`stealth_return_rejected.hl`** exists as a fixture with **no `#[test]`
   driving it** — wire it up or delete it; unpinned rejection behavior tends
   to drift.
10. **Valgrind/ASan gating** on the ~5 `*no_leak*` and lifetime tests (§4.1).

Items 1, 4, 5, 6, 7, 9, 10 can and should be written now (they pass or
demonstrably pin current behavior). Items 2, 3 document known-broken behavior
— per the adjudicated latent-bug policy (§5 item 4) they are written in 1.5d
as expected-fail/`#[ignore]`d pins and flip live in Phase 2. Item 8 is
adjudicated (§5 item 2): compile-time diagnostic until 1.5a completes.

---

## 5. Refined phase plan

The brief's six phases, broken into individually-testable steps. Gate for
every step: `cargo test` fully green (per CLAUDE.md ritual) plus the
valgrind-gated lifetime tests once step 1.5b lands. Where a step deletes
machinery, the tests pinning its behavior must pass unchanged — that is the
point of deleting-with-confidence.

### Phase 1.5 — pre-migration hardening (entry criterion for Phase 2)

- **1.5a** Finish the char*/HiLowArray* representation-split fix (current
  CLAUDE.md phase) → 13 failing integration tests green. *Gate: full suite
  green.*
- **1.5b** Add valgrind gating (landed broader than planned: a gate over
  every program in tests/programs/, not just the lifetime tests — see
  tests/valgrind_gate.rs), plus control-transfer temp cleanup fixes the gate
  work surfaced. *Gate: full suite green including the valgrind gate.*
- **1.5c** Object ownership discipline (adjudicated 2026-07-15, §5 item 5):
  fix the object double-release class in the current retain/release
  machinery; remove all 17 entries from `KNOWN_MEMORY_BUGS` in
  tests/valgrind_gate.rs as they come clean. (Landed 2026-07-15, slightly
  broader than planned: the fix also required actually implementing weak
  properties — `is_weak` was never set anywhere — and re-keying WeakRef from
  a raw slot address to (holder, prop_index) to survive property-array
  reallocs. Weak-after-death access semantics were adjudicated 2026-07-15 —
  see item 6 under "Open questions — adjudicated". Note recorded next to the
  re-keying decision: **object property removal does not exist** — no removal
  syntax in the parser and no `hl_object_remove_*` in the runtime (the spec's
  "properties can be added or removed dynamically" at hilow-design.md:564 is
  aspirational) — so the (holder, prop_index) keying's append-only-indices
  assumption holds by construction; any future property-removal feature must
  revisit WeakRef keying in the same change.) *Gate: full suite + gate green
  with an empty KNOWN_MEMORY_BUGS list.*
- **1.5d** Add gap tests §4.4 items 1, 4, 5, 6, 7, 9 (all pin current
  behavior). Item 1 (re-entrant mutation) may *expose* the temp_buffer bug —
  if it fails, mark `#[ignore]` with a STATUS entry naming Phase 2c as the
  fix, per CLAUDE.md's ignore policy. (Landed 2026-07-15, tests only — no
  compiler/runtime changes. Live pins: early-return lifetime scalar+array,
  shadowing+array identity, insert firing order (CHANGED then ADDED),
  array-watcher factory, stealth-return rejection wired, string-watcher
  compile diagnostic pinned per §5 item 2. Item 1 written as adjudicated:
  asserts deferred declaring-thread-queue semantics per the brief, `#[ignore]`
  citing Phase 5 — current synchronous firing legitimately differs. The four
  §3.4 latent bugs each got an `#[ignore]`d expected-behavior test per §5
  item 4, their programs carried on the gate's KNOWN_MEMORY_BUGS; §3.4(d)'s
  current failure mode turned out to be upstream of the audit's description —
  a statement-temporary watcher expression never registers its subscriptions
  at all and leaks. Weak-after-death expected-behavior test written
  `#[ignore]`d citing 1.5e, its program on REJECTION_FIXTURES since it cannot
  compile before 1.5e.) *Gate: suite green (ignores documented).*
- **1.5e** Weak-after-death semantics (adjudicated item 6): dead-weak read
  yields `unknown` with reason "weak referent released"; member access on it
  propagates per the spec's unknown rules. Implementation and the
  hilow-design.md spec edit land in the same commit; un-ignore
  test_weak_after_death_unknown_integration and remove
  weak_after_death_unknown.hl from REJECTION_FIXTURES. (Landed 2026-07-15.
  Design: a weak property's shape type is `T?` — reading it emits
  `hl_object_get_weak`, returning a fresh optional wrapping the retained
  referent while alive or unknown "weak referent released" after death;
  HiLowOptional gained an object payload kind. Member access through the
  optional emits `hl_optional_member_{i32,str,object}`: unknown propagates as
  the same instance, a live referent's property wraps as `T?`. This rides the
  existing Phase 9 unknown ecosystem (hl_is_unknown, refinement to
  UnknownType for .reason, print_optional_*) rather than adding a parallel
  one. Surface changes beyond the ruling, all forced by it: `print(T?)` now
  typechecks for primitive/string inners (codegen already dispatched it —
  reachable before only under refinement), and the assignment-form weak store
  `holder.ref = weak target` accepts `T?`-slot vs `T`-value. Weak member
  propagation is implemented for property types i32/string/object; other
  property types through a weak read raise a compile-time diagnostic —
  extension deferred until the optional runtime grows those payload kinds.)
  *Gate: full suite + gate green, that test live.*

### Phase 2 — arrays first

- **2a Cell header.** Introduce `HiLowCell` header fields (watcher list,
  parent list, version, deep-watched flag) onto `HiLowArray` — mechanical
  layout change, firing behavior identical. *Gate: full suite, no behavior
  change expected.* (Landed 2026-07-16, scope adjudicated via the approved 2a
  plan: the phase instruction added "watcher construction must register by
  construction — the §3.4(d) never-registers hole becomes structurally
  impossible for arrays", which cannot be done soundly as layout-only —
  structural registration forces its dual, release-unsubscribes, which
  requires watcher→cell subscription backrefs (the watcher-value half of 2b).
  As landed: standalone `HiLowCell {refcount, watchers, parents, version,
  deep_watched}` embedded as HiLowArray's first member; cell ops
  (`hl_cell_retain/release/subscribe/unsubscribe_watcher/unsubscribe_env`)
  all take `HiLowCell*`, nothing array-specific in header or ops;
  `hl_watcher_new_subscribed(body, env, n, ...)` is the ONLY way generated
  code attaches an array watcher — the `temp_watcher_expr_*` side-channel is
  deleted for arrays (kept for scalars until Phase 3); cell→watcher and
  watcher→cell links are both non-owning with symmetric unlink-on-death.
  Consequence: §3.4(b)(c)(d) fixed HERE, their three tests live and their
  KNOWN_MEMORY_BUGS entries removed. §3.4(a) deliberately preserved — the
  eight firing loops keep their per-site call shapes byte-identical,
  including the 2-arg .move casts — dies in 2c. hl_cell_notify is 2c; the
  parent walk is 2d; parents/version/deep_watched are dead fields until
  then. Requirement-0 side task: the six placeholder optional-unwrap helpers
  proved REACHABLE and now abort loudly; enabling bugs recorded in
  STATUS.md.)
- **2b Watcher values + owned envs.** Array watcher registration constructs a
  runtime watcher object owning a refcounted env; `hl_array_register_watcher`
  takes the watcher; delete codegen env-free/unregister
  (`emit_scope_cleanup`/`emit_early_return_cleanup`/`emit_temp_cleanup`
  Environment arms), delete `array_watcher_registrations`. Scope exit releases
  the watcher value instead. Kills §3.4(b)(c)(d). Write §4.4 item 2 and 3
  tests at the head of this step. *Gate: full suite + valgrind; the
  dies-with-scope and capture tests are the sentinels.* (Rescoped by 2a's
  landing, 2026-07-16: watcher values, construction-registration,
  release-unsubscription, and the §3.4(b)(c)(d) fixes landed in 2a. What
  remains for 2b: **watcher-owned refcounted envs** — the watcher retains its
  env, freeing it on final release — and the deletion of the codegen
  scope-owned-env machinery (`array_watcher_registrations`, the Environment
  cleanup arms, the env-keyed `hl_cell_unsubscribe_env` safety net). Until
  then, envs remain scope-owned and the env-keyed net covers the
  capture-escape case exactly as before, including its known single-slot
  limitation for escaping multi-array capture watchers.) (Landed 2026-07-16
  as rescoped. `HiLowWatcher` owns `env` — freed on final release after
  unsubscription; the watcher's refcount IS the env's refcount (envs are
  never shared — a separate counter is deliberately not added until a phase
  shares them). `hl_cell_unsubscribe_env`, `array_watcher_registrations`,
  the `HeapType::Environment` variant, and all three Environment cleanup
  arms deleted. STOP-item resolved via the approved plan: capture-escape
  (`let w = <capturing watcher>; return w`) was reachable — the escape pass
  ran only on direct WatcherExpr returns and never inspected captures — and
  was kept sound only by the deleted net; it is now rejected at compile time
  (escape check walks captures; capture-unsafe watcher bindings tracked
  per-function and rejected at `return`) until Phase 3 boxing drops the
  restriction per §5 item 1. Residual known hole: laundering through helper
  calls is not caught (needs dataflow Phase 3 obsoletes) — STATUS.md Known
  issues. "§4.4 items 2 and 3" were already written in 1.5d per §4.4's own
  footnote — item 3 flipped in 2a, item 2 flips in 2c; no new copies. Also
  in this commit: step zero, §5 item 7 — optional-inner rejection.)
- **2c One firing ABI + value deltas.** Unify all eight mutator firing loops
  on the `(env, cell, delta)` call through a single notify helper; fix the
  move 2-arg bug (§3.4a); replace both static temp_buffers with caller-owned /
  heap-copied deltas; move `hl_array_remove`'s return off the static buffer.
  Un-`#[ignore]` the re-entrancy test. *Gate: full suite + re-entrancy test
  live.* (Landed 2026-07-17. The un-ignore note here was SUPERSEDED before
  this phase ran by the 1.5d ruling: the re-entrancy test asserts deferred
  declaring-thread-queue semantics per the brief and flips live in Phase 5;
  its program's live stdout is byte-identical pre/post-2c (verified by stash
  round-trip) and stays valgrind-clean. As landed: `HiLowDelta {event,
  payload (heap-owned byte copy), payload_size, payload_release, from, to}`
  is self-contained and queueable — the mutator constructs it (retaining
  object-array element refs via retain_fn), `hl_cell_notify(cell, event,
  delta)` fires, the same mutator releases it via hl_delta_release; Phase 5
  changes only when/where bodies run and who releases. hl_cell_notify is ONE
  walk in list order firing nodes where modifier == event OR modifier ==
  CHANGED (implicit-changed mirrors the old interleaved loops exactly,
  preserving the 1.5d-pinned firing orders); it is the authoritative stealth
  site, mutators check stealth+empty-list only to skip delta construction.
  One body ABI `HiLowWatcherBody(void* env, HiLowCell*, const HiLowDelta*)`
  — bodies rebind the watched name from the cell and bind aliases from
  delta->payload / delta->from,to (still copied at entry); HiLowMovedDelta
  survives only as the Tuple(Usize,Usize) alias-binding type. Both static
  temp_buffers deleted (grep-verified the only two): hl_array_remove became
  out-param `(arr, index, void* out)` (codegen hoists a caller-owned temp
  via pending_statement_decls — no 1024 cap, re-entrant), hl_array_move's
  shift scratch became a local malloc/free. §3.4(a) DEAD at both .move sites:
  test_watcher_move_capture_env_integration LIVE and valgrind 0;
  KNOWN_MEMORY_BUGS EMPTY. New fixture watcher_move_noop_capture_env pins
  the from==to site with captures, which 1.5d never covered.)
- **2d Parent lists + deep.** Container cells get parent lists; `(deep)`
  subscription sets the deep-watched bit down the chain; mutators walk parents
  only when the bit is set. New tests: nested-array deep watch (§4.4 gap —
  currently untestable because deep on nested arrays doesn't exist). *Gate:
  full suite + new deep tests; zero-cost check = un-deep-watched benchmarks
  or at minimum codegen-diff inspection showing no walk.* (Landed 2026-07-17,
  arrays only, per the approved plan. Parent links live on the CHILD:
  non-owning `HiLowCellParent` entries, one per containment, duplicates
  deliberate (same child twice in one parent → two entries; removal drops
  exactly one; the walk's epoch stamp, not entry uniqueness, prevents
  double-fire). Maintained at every containment change — push/insert/set
  add, pop/remove/set-overwrite/clear/parent-teardown remove one each,
  before any release; move is reorder-only. Element kind detected by
  retain_fn identity (codegen passes hl_array_retain for `[[T]]`). Cycle/
  diamond termination: the dead `version` field became the walk epoch stamp;
  the walk is collect-then-fire, so nested mutation inside a deep body runs
  its own walk under a fresh epoch without corrupting the outer traversal.
  A TRULY self-containing array is unrepresentable (no recursive types) —
  adjudicated via the plan: the diamond fixture pins single-fire through the
  identical revisit-suppression path. deep_watched set at subscription
  (codegen hoists `hl_array_mark_deep`, which recurses the subtree and
  early-returns on a set bit) and on containment-add into a marked parent;
  CLEARING IS DEFERRED — a stale bit is one wasted silent walk, never a
  wrong fire (STATUS.md Known issues). `hl_cell_notify` gained the deep
  semantics: own-list nodes also fire on modifier == HL_ARR_DEEP (a deep
  subscriber on the mutated cell fires for every event, per spec), then
  ancestors fire ONLY their deep nodes with the SAME delta (one delta per
  mutation) and their own cell (body binds the subscribed variable). Stealth
  suppresses deep fires via notify's existing single gate (pinned).
  `HL_ARR_DEEP 4` fills the documented constant gap. Surface re-admitted per
  §5 item 3: parser word, typecheck arm (arrays-only diagnostic; alias-on-
  deep rejected by the existing added/removed/moved-only rule — precise deep
  deltas deferred per the brief's "optional alias data"). SCOPE ADJUDICATION
  (via the plan): the audit is silent on which phase OBJECTS embed the cell
  header — deep crosses array-in-array only; object-held arrays do not
  propagate; recorded as a STATUS.md open question needing a user scheduling
  decision. Zero-cost check: generated C for a non-deep array program is
  byte-identical pre/post-2d (worktree diff); unwatched mutation cost is one
  bit+pointer check in `cell_has_audience`. Seven live deep fixtures + one
  rejection fixture; integration 269→277.)
- **2e Objects join the cell model.** (Added 2026-07-17 by user ruling,
  resolving the open question 2d surfaced: objects gain the cell header NOW,
  before boxing — **containers complete before scalars box**. Phase 3a
  follows 2e.) Cell header onto `HiLowObject` per the 2a playbook; containment
  parent links per the 2d maintenance-table pattern for every strong
  cross-container store (object property holding object or array, proto
  links — proto is an ordinary property — and object-in-array, which
  generalizes 2d's element helpers); minimal event mapping (existing-property
  set → CHANGED, new property → ADDED, proto reassignment → CHANGED, no
  REMOVED per the tombstone ruling); deep crosses objects both directions.
  Adjudicated: weak properties create NO parent links and deep does not
  cross weak — weak is observation without ownership (spec edit in the weak
  subsection, same commit). *Gate: full ritual green, single commit.*
  (Landed 2026-07-17 per the approved plan, with six plan-adjudicated items:
  (A) the runtime ADDED mapping is implemented in full but `(added)obj` is
  REJECTED with a diagnostic — dynamic property addition is not expressible
  (typecheck rejects unknown-property assignment), so the event has no
  reachable trigger and an admissible-but-inert subscription would be a trap;
  new STATUS.md open question for scheduling dynamic property addition.
  (B) True object cycles are UNREPRESENTABLE (self-property and proto-cycle
  constructions both type-rejected — probed), mirroring 2d's arrays; the
  object diamond fixture ({a: shared, b: shared}) pins single-fire through
  the same epoch revisit-suppression. (C) Array-valued object properties
  did not exist (codegen UnsupportedFeature; no HL_VALUE_ARRAY) — landed as
  minimal enabling work for the mandated array-in-object tests: HL_VALUE_ARRAY
  tag, hl_object_set_array/hl_object_get_array (borrow getter, proto-chain
  walk), ownership arms in set_property/release/teardown, codegen arms for
  member assign, object-literal property, member read, and let-binding
  retain. (D) `(assigned)obj` rejected citing Phase 3 (rebinding detection is
  boxing machinery). (E) String properties create no parent links — string
  watching stays compile-time rejected (§5 item 2) and no surface mutates a
  string in place. (F) Decl-form object watchers stay rejected — decl-form
  wires through the legacy name-keyed map that dies in Phase 3; objects join
  the expression-form cell path only. IMPLEMENTATION: all object stores
  funnel through set_property and hl_object_set_object_weak — one choke
  point each for firing (2d mutator guard shape; NULL deltas since object
  subscriptions carry no aliases; a weak store still fires on the HOLDER —
  weakness affects containment, not firing) and containment
  (object_property_stored/removed mirror array_element_stored/removed;
  remove-before-release ordering preserved; teardown unlinks per strong
  container property). hl_cell_notify needed ZERO changes — the walk was
  container-agnostic by construction. Deep marking became mutual recursion
  (hl_object_mark_deep skips weak properties; hl_array_mark_deep gained the
  object-element branch; marked ⇒ strong-reachable subtree marked).
  2d's mutators already CALLED the containment helpers for every element
  kind; the helpers just early-returned unless elems_are_arrays — 2e added
  elems_are_objects (retain_fn identity). Codegen: WatcherExpr admits
  objects (expression form); mixed array+object watchers rejected (the body
  prologue casts the fired cell to the first subscription's container type);
  multi-object watchers get multi-array semantics; object captures became
  identity captures (the by-reference scalar branch produced a
  void**/HiLowObject** mismatch — subscribed names are ALSO collected as
  captures because find_variable_in_outer_scope ignores shadowing, which
  arrays never exposed since their captures were already identity-stored).
  DISCOVERED during the zero-cost check: generated C is NONDETERMINISTIC
  across runs of the SAME compiler binary — scope-cleanup release order
  comes from a HashMap iteration (pre-existing on HEAD, verified by
  run-to-run diffs of the unmodified 2d binary; semantically harmless since
  releases within a cleanup block are order-independent under symmetric
  unlink; recorded in STATUS Known issues; byte-diff verification is
  therefore flaky for multi-owner scopes). Zero-cost result: watcher-free
  object program byte-identical pre/post-2e; array program identical as a
  line multiset with the release-order permutation present on HEAD alone;
  watcher_reentrant_deferred live stdout byte-identical pre/post-2e
  (HEAD-worktree comparison, no stash). Unwatched property-set cost is the
  cell_has_audience bit+pointer check. Twelve live fixtures (array-property
  basics, changed, proto-reassign CHANGED, object-in-object dual deep,
  array-in-object via direct member push, object-in-array, proto-chain deep,
  diamond single-fire, sibling isolation, stealth suppression, weak
  boundary with strong-holder positive control, new-child-marked with
  old-child-silenced) — every output predicted from semantics before
  running, all matched exactly, all valgrind 0 — plus four rejection
  fixtures with pinned diagnostics. Integration 277→293, ignored stays 2,
  KNOWN_MEMORY_BUGS stays EMPTY, REJECTION_FIXTURES +4.)

### Phase 3 — scalars

- **3a Boxing analysis.** Compile-time pass marking variables that are ever
  subscribed (decl-form, expression-form, or captured-into-watcher). No
  codegen change yet; expose as a queryable attribute + unit tests on the
  analysis. *Gate: full suite (no behavior change).*
  (Landed 2026-07-17 per the approved plan. CRITERION, as an invariant: a
  declaration D boxes iff (a) some subscription — either form — resolves to
  D, or (b) some watcher body references D (read OR write) across the
  watcher boundary. Conservative-correct: failing to box is a soundness
  bug, boxing unnecessarily only a performance bug — when uncertain, box.
  (b) covers §5 item 1's escape soundness with no dataflow: expression-form
  watcher values are first-class (incl. the 2b laundering hole), and
  decl-form watcher NAMES are also declared as first-class Watcher-typed
  variables today, so both forms' captures box; narrow decl-form in 3c if
  its runtime lifecycle pins scope-boundness. Function-expression closures
  are NOT watchers and don't box captures — unless nested inside a watcher
  body, where the boundary check still fires. Type-agnostic: marks
  DECLARATIONS; the mark is subsumed for containers (already cells); 3b
  applies it to scalars only. REPRESENTATION (design statement for 3b —
  nothing built in 3a): boxed scalar = HiLowCell header + {kind, payload
  union}, cell first member so notify/subscribe/parent machinery apply
  unchanged; ONE payload representation serves both boxed scalars and the
  §5 item 7 optional payload matrix — HiLowOptional's HL_OPT_* kinds
  converge onto it, and the 2b allow-list rejection lifts as kinds land.
  THE 2E CAPTURE FINDING: 3a provably does NOT inherit it — the analysis
  never reads WatcherExpr.captures; it re-derives references with its own
  shadow-correct resolver (subscription bindings, aliases, params, and
  body-local lets shadow outer names), pinned by three shadowing tests
  including the body-local-let case the legacy scan gets wrong. Not fixed
  either: the buggy scan feeds live codegen env packing (behavior-change
  out of gate) and is deleted/rewritten with its machinery in 3b–3d.
  IMPLEMENTATION: src/typecheck/boxing.rs — standalone AST walker, own
  scope stack, watcher-boundary indices; queryable BoxingAnalysis
  {is_boxed(name, decl_pos), decisions_for(name) in declaration order,
  boxed_count}; not wired into the compile pipeline (nothing consumes it
  until 3b). 14 tests in tests/boxing_analysis_tests.rs, every one
  asserting concrete decisions: unboxed (plain, closure-capture),
  subscription both forms, capture read + capture WRITE (assignment
  targets are references), the laundering escape, three shadowing pins,
  container subsumption, alias non-boxing, nested-watcher crossing — all
  passed on first run against invariant-derived expectations.
  DETERMINISTIC EMISSION (the 2e-discovered nondeterminism, fixed here):
  FOUR emitting iteration sites over HashMaps, not the planned three —
  emit_scope_cleanup and emit_early_return_cleanup (heap_owners),
  emit_temp_cleanup (temp_owners), and emit_enclosing_temp_releases
  (per-frame saved temp_owners maps, found during implementation) — all
  now collect-sort-by-name-emit. Generated C verified byte-identical
  across 5 runs each on 4 multi-owner programs incl. the one that
  permuted in 2e; byte-diff verifiability restored for 3b's zero-cost
  checks. Full ritual green, integration 293/0/2 unchanged, gate green.)
- **3b hl_cell_set.** Boxed scalars lower to cells; assignment to them becomes
  `hl_cell_set`; delete the scalar firing block (1999–2136). Unwatched
  variables stay raw locals — verify by inspecting generated C for an
  unwatched program (zero-cost check). *Gate: full suite; changed/assigned
  and stealth scalar tests are the sentinels.*
  (Landed 2026-07-17 per the approved plan. RUNTIME: HiLowScalar = HiLowCell
  header + HiLowValue payload — HiLowValue IS the one payload representation
  (3a design statement); only the i32 kind has ctor/get/set (corpus set,
  §5 item 7 — no speculative matrix, optional allow-list unchanged since no
  new kind landed). hl_cell_set_i32: store always (stealth suppresses only
  notification), CHANGED notify iff payload differed, then HL_SCALAR_ASSIGNED
  (6) always — changed subscribers before assigned, the legacy order.
  hl_cell_notify amended: CHANGED/DEEP modifiers do NOT fire on
  HL_SCALAR_ASSIGNED and it never triggers the deep parent walk (equal-value
  assignment is not a mutation). hl_cell_subscribe now APPENDS (see §5 item
  9). HiLowWatcher gained env_dtor: EVERY watcher env slot (subscribed and
  captured, containers included) is a RETAINED cell released by a generated
  per-watcher dtor at final release — escape soundness (§5 item 1); the
  runtime frees the env itself. LOWERING: boxed let → hl_scalar_new_i32 +
  HeapType::Scalar scope release (early returns included; `return x` copies
  the payload and does NOT transfer the cell); boxed reads →
  hl_scalar_get_i32; assignment (incl. compound, adjudication B) →
  hl_cell_set_i32; boxed params box in the prologue (name rebinds to the
  cell); boxed PROGRAM-scope declarations become file-scope statics so
  nested named functions subscribe/capture by cell identity (this replaces
  the name-collision accident the legacy factory fixtures leaned on). BOTH
  watcher forms subscribe via hl_watcher_new_subscribed (registration by
  construction): expression-form scalars migrated off the 2b heap path onto
  the 2c env-ABI path; decl-form bodies are env-ABI too, gated on the legacy
  statics (which stay until 3c), constructed at the declaration site as a
  hidden scope-owned watcher — pre-declaration assignments cannot fire (no
  subscriber on the cell yet), replacing the compile-time ordering trick.
  Watcher-env capture lists come from the 3a analysis
  (BoxingAnalysis::captures_for, shadow-correct) for scalar watchers and
  decl-form; container watchers keep the legacy list (its phantom
  subscribed-container slots are the multi-subscription rebind mechanism)
  with unboxed-scalar phantoms skipped. DELETED: the scalar firing block +
  Phase 10-γ compound rejection, the 2b scalar WatcherExpr branch (bare
  hl_watcher_new + side-channel + _cap_ pointer params),
  emit_watcher_call_args{,_from_names}, extract_watcher_id, the
  scalar-capture read branch, the typecheck escape machinery
  (capture_unsafe_watchers, current_function_scope_depth,
  check_watcher_escape_reachability, both rejection sites) and the spec's
  reachability rule (sound-escape text landed in its place). ADJUDICATIONS
  (plan approval): A decl-form on container variables → compile diagnostic
  (rebinding-watch = variable-slot bucket with (assigned)obj and strings;
  was untested, leaking, rebinding-firing); B compound assignment to watched
  scalars lowers and fires; C decl-form captures work (previously emitted
  non-compiling C); D watchable kinds narrowed to i32+containers with honest
  diagnostics; E boxed destructured bindings and boxed closure-hoisted vars
  reject cleanly; F env retains for all watcher envs. TESTS: −3 escape
  rejections (fixtures deleted, gate −3), +7 (changed/assigned order,
  compound fires, decl capture, capture-escape-sound, subscribed-local-
  escape-sound, decl-container rejection, destructured rejection; gate +2
  rejection entries) → integration 297/0/2. Three array expected files
  flipped to declaration order per §5 item 9. STRING watching stays rejected
  with the variable-slot wording (pinned substrings preserved). Zero-cost
  verified: 211 unwatched compilable corpus programs byte-identical HEAD vs
  3b (the 14 non-diffable are module-entry/rejection fixtures that never
  reach codegen). Sound escape demonstrated live + valgrind 0; the 2b
  laundering residual closes as resolved-by-design.)
- **3c Runtime watcher lifecycle.** Watcher declarations construct runtime
  watcher values (both forms share the heap path); delete static
  `_active`/`_ended` bools, the four static helpers, activation/deactivation
  emission (Phase 4/5 loops in both block walkers + `emit_main_function`),
  and the static-dispatch method arm. Scope exit releases the watcher —
  reaches early returns for free; §4.4 item 4 test flips from pinning the
  hole to pinning the fix. *Gate: full suite; pause/resume/end/isActive,
  scope-bounding, factory tests are the sentinels.*
  (Landed 2026-07-18 per the approved plan; no STOP conditions hit. ZERO
  runtime changes: HiLowWatcher already carried the full lifecycle
  (active/ended bools, hl_watcher_pause/resume/end/is_active, the
  hl_cell_notify per-node gate), verified semantically identical to the four
  static helpers before deletion. DELETED, each with its replacement on the
  watcher object: static bool emission → HiLowWatcher.active/.ended; four
  helpers → hl_watcher_* functions; body static gate → hl_cell_notify's
  node->watcher gate (already ran on every fire — decl-form was
  double-gated); activation lines at the construction site + the
  emit_main_function pre-activation loop → hl_watcher_new constructs
  active; Phase 5 deactivation loops in both block walkers
  (`hilow_watcher_N_end();`) → the scope-exit hl_watcher_release already
  emitted since 3b, which also reaches early returns (the deactivation
  emission never did — that hole closes); static-dispatch method arm → the
  existing heap arm, reached by constructing the decl-form watcher under
  the USER'S OWN name as a `HiLowWatcher*` variable (Type::Watcher in
  variable_types, heap_owners under that name). LATENT 3B BUG fixed by the
  renaming, found by probing during planning: a decl-form watcher named
  `w` segfaulted — the hidden variable `hilow_watcher_{id}_w` collided
  with the body function name `hilow_watcher_{id}_{name}` when name=="w",
  so the uninitialized C local (shadowing the function) was passed as
  body_fn; jump-to-garbage on first fire. Pinned by new fixture
  watcher_decl_named_w_fires (valgrind 0). MODULE-LEVEL WATCHERS: probed
  broken, not inert — parser accepted them, the multi-file typecheck path
  never checked them, and codegen died with an internal error on any real
  one ("subscription with no resolved type"); the spec's module section
  admits only export function/let and specifies no construction timing.
  Wiring one would have invented initialization semantics → REJECTED per
  the phase instruction's sanctioned alternative, on both typecheck paths,
  with the diagnostic citing the gap; the generate_graph bodies-only loop
  deleted (a debug_assert guards the invariant); rejection fixture
  modules/watcher_in_module + gate entry; STATUS open question records
  module initialization semantics as unscheduled. ADJUDICATION A (plan
  approval): a declaration-form watcher name is not a first-class value —
  any use outside method-call-receiver position is a compile diagnostic
  (previously those shapes emitted non-compiling C or died in codegen;
  corpus-clean, grep-verified). Two rejection fixtures (return, let-alias)
  + one spec paragraph landed deliberately in the same commit. DECL-FORM
  SCOPE-BOUNDNESS is thereby PROVEN (sole scope-owned reference, name
  unaliasable), resolving 3a's open narrowing question. FUTURE PERFORMANCE
  REFINEMENT (recorded, deliberately NOT implemented): the 3a analysis
  boxes captures of both watcher forms; variables boxed ONLY because
  decl-form watchers capture them could stay raw locals with raw-pointer
  envs, since the watcher provably dies before its frame. Revisit if
  profiling ever shows boxing overhead mattering; env packing is uniformly
  cell-based today and the narrowing is not a trivial consequence of 3c.
  §4.4 ITEM 4 FLIP: watcher_early_return_scalar.hl rewritten from the
  expression-form/function-local shape (which could not observe post-exit
  firing) to the observable decl-form shape — program-scope x, decl watcher
  in f, early return; mutations after BOTH the early-return call and a
  normal-exit call must not fire ("fired\ndone"); probed green + valgrind
  0 before the deletion and green after (standing rule); the array variant
  already pinned post-exit silence and is unchanged. STATUS known issues
  closed: unreachable-scope-exit-deactivation-on-early-return (mechanism
  deleted, fix pinned) and duplicated-activation-logic-in-block-walkers
  (loops deleted). ZERO-COST: 212 unwatched corpus entries — 17 rejection
  fixtures never reach codegen under either binary; 194 byte-identical
  3b-binary vs 3c-binary; the 1 remaining (modules/diamond) permutes
  run-to-run on the UNMODIFIED 3b binary alone (module emission order,
  3/3 split over 6 runs; both binaries emit the identical two-variant
  md5 pair) — a pre-existing module-graph nondeterminism OUTSIDE the 3a
  determinism fix's scope (which covered cleanup-map iteration), recorded
  in STATUS Known issues. Stays per the flip map: watcher_name_to_id,
  pass-3 subscription registration, shadow masking, the
  WatcherSubscription/HeapWatcherSubscription structs, and the inference
  mirror keyed on watcher_name_to_id — all die in 3d. TESTS: integration
  297→301 (+4: decl-named-w pin, 2 escape rejections, module rejection;
  early-return-scalar rewritten in place); REJECTION_FIXTURES +3;
  KNOWN_MEMORY_BUGS stays EMPTY; ignored stays 2. Full ritual green, gate
  green.)
- **3d Delete name-keyed subscription.** Delete `watcher_subscribers`,
  `heap_watcher_subscribers`, `watcher_name_to_id`,
  `scalar_watcher_captures`, shadow-masking (672–705, 6946–7015), the
  `WatcherSubscription`/`HeapWatcherSubscription` structs, and the
  `temp_watcher_expr_*` side-channel. Shadowing tests must pass unchanged.
  **User decision needed at this step:** do the two escape-rejection tests
  (§4.3 last row) remain spec (keep a deliberate check) or fall away (boxing
  makes escape sound)? *Gate: full suite (modulo that decision).*
  (Landed 2026-07-18 per the approved plan. The recorded user decision was
  already resolved by §5 item 1 in 3b — escape is sound, those tests are
  gone. PRE-DELETION INVENTORY (exhaustive reference audit, each read site
  verdicted live/dead): watcher_subscribers had ONE writer
  (register_watcher_subscriptions) and zero value-consuming reads;
  heap_watcher_subscribers had ZERO writers since 3b (always empty — its
  masking restore loop never iterated); the shadow-masking block in
  generate_function + collect_local_variable_names + two recursive helpers
  served only those two dead maps; watcher_name_to_id carried exactly two
  live roles — id TRANSPORT from the pass-3 allocation loops to the pass-4
  consumption loops (three get sites), and the inference mirror.
  scalar_watcher_captures / temp_watcher_expr_* / array_watcher_registrations
  / HeapType::Environment: grep-confirmed already gone (3b/2b). REPLACEMENTS:
  id transport became ORDER-BASED — pass 3 pushes allocated ids into a local
  Vec in item order (program-body split: new program_watcher_ids field,
  filled by the generate_program_body_functions pre-pass BEFORE nested
  functions generate, consumed by position in
  generate_program_body_statements) — numbering byte-exact, proven by a
  corpus-wide C diff of the transport-only state (286 identical, 28
  rejection fixtures no-C-either-side, modules/diamond = the recorded
  module-order permutation, same two-variant md5 pair on both binaries).
  The inference mirror re-keyed from watcher_name_to_id to
  variable_types == Type::Watcher (the same key the 3c dispatch arm uses) —
  the ONE disclosed behavior edge: expression-form watcher method-call
  inference unified with decl-form (isActive: i32→bool, pause/resume/end:
  i32→Nothing on the fall-through path). Consequence pinned live by new
  fixture watcher_expression_isactive_print (print(w.isActive()) now
  true/false, was 1/0 — probe-only shape, no fixture had pinned the split).
  DISCLOSED PREDICTION MISS: the plan declared zero expected corpus diffs
  from the re-key; the post-deletion corpus diff found ONE —
  watcher/expression_methods, where `if (w.isActive())` now emits
  `if (hl_watcher_is_active(w))` instead of
  `if ((hl_watcher_is_active(w) != 0))` (Bool inference drops the i32
  truthiness coercion) — semantically identical C, runtime output and
  valgrind verified unchanged; it is the adjudicated (2a) unification
  manifesting, not an undeclared class, but the zero-diff prediction was
  wrong on this fixture. Everything else byte-identical corpus-wide.
  Deletion resolved via dead-code checks alone: warning count 27→25,
  the delta being exactly the two accepted fields-never-read struct
  warnings from 3b. Shadowing sentinels passed unchanged. Tests 301→302
  (+1 pin, no drops); REJECTION_FIXTURES unchanged; KNOWN_MEMORY_BUGS
  EMPTY; ignored stays 2. Full ritual green, gate green.)
- **3e Variable-slot cells.** The rebinding-watch bucket, adjudicated
  (scheduled 2026-07-18 with plan approval of 3d, per the owner's
  instruction): string watching, decl-form watchers on container-typed
  variables, and `(assigned)obj` all mean watching the VARIABLE (rebinding),
  implemented via the boxing machinery extended to reference-typed payloads —
  a boxed variable slot whose cell fires on rebinding, distinct from the
  value's own cell. Scheduled immediately after 3d. The 3b STATUS
  open-question rider carries into this phase: the spec's
  `(changed)`-on-non-primitives reference-equality wording vs the cell
  model's content-mutation firing is resolved here, deliberately. The three
  standing rejections (string_watcher_rejected, watcher_decl_container_rejected,
  object_watch_assigned_rejected) flip live or re-scope in this phase.
  *Gate: full suite; the three rejection fixtures are the entry sentinels.*
  SPLIT (approved with the 3e-α plan, at the follow-subscription boundary
  the phase instruction named): **3e-α** = slot cells + `(assigned)` on all
  types (both forms) + string watching (both modifiers, both forms) — no
  retargeting (strings' only mutation IS rebinding; (assigned) never
  follows content); **3e-β** = decl-form content-following on containers
  (subscription-node retargeting on rebinding, old-container unsubscribe,
  new-subtree deep propagation, the follow-proof fixtures) — flips the
  third sentinel.
  (3e-α landed 2026-07-18 per the approved plan; no STOP conditions hit.
  RUNTIME: HiLowValue's existing STR/ARRAY/OBJECT kinds became slot
  payloads — hl_scalar_new_str/new_array_ref/new_object_ref ADOPT a +1,
  getters BORROW, hl_scalar_release tears the payload down by kind;
  hl_cell_set_str/set_array_ref/set_object_ref adopt the new +1, compute
  changed per item 10(a) (strings: identity fast path then hl_string_eq;
  containers: identity), store, release old AFTER the store
  (self-assignment safe: borrowed rhs is retained by codegen first), and
  fire through the identical stealth/audience/notify shape as
  hl_cell_set_i32. BOXING: BoxDecision gained slot_required +
  needs_slot(name, pos) — set by mark_subscription reading the
  subscription's modifier and resolved_var_type refcell ((assigned)
  anything, or any string subscription); expression-form container content
  subscriptions deliberately do NOT set it. CODEGEN: slot lets mirror the
  boxed-i32 rows (local + file-scope-static program lets, HeapType::Scalar
  release, adopting constructors with hl_array_ref/hl_object_ref retains
  for borrowed initializers); slot assignment emits the set family
  (compound on ref slots rejected); reads emit type-keyed getters
  (borrows); watcher classification routes SLOT-KIND subscriptions
  ((assigned) anything, strings) down the scalar/slot body path with
  type-keyed snapshot bindings, while VALUE-kind subscriptions on
  slot-boxed variables subscribe the CURRENT value's cell via a payload
  deref (identity at construction, item 10(b)) — a bug caught mid-session
  by the sentinel battery: the first deref condition also fired for boxed
  i32 (changed) subscriptions, segfaulting 23 tests; fixed to
  container-types-only and the suite came back fully green. Env slots for
  slot-boxed variables are EnvSlot::Scalar regardless of HiLow type (the
  representation decides); capture classification gained the boxed-first
  override in all three loops. GATES LIFTED: the typecheck (assigned)obj
  rejection (validate_subscription_modifier), and both codegen string
  arms; decl-form containers now allow (assigned)-only subscription lists
  (content modifiers still reject, message updated to cite 3e-β — pinned
  substrings preserved, test unedited); slot-needing reference-typed
  PARAMS get a bounded diagnostic (no boxing prologue for them yet; zero
  corpus coverage). DISCLOSED behavior fix: expression-form (assigned)xs
  compiled since 3b and silently never fired (subscribed the array's own
  cell with HL_SCALAR_ASSIGNED); it now fires on rebinding — pinned.
  TESTS: string_watcher_rejected + object_watch_assigned_rejected fixtures
  deleted (flipped); 5 live fixtures (string changed/assigned-order/
  decl-form, object assigned, array assigned+value-coexistence) + 1
  rejection pin (mixed slot/value subscriptions → the existing
  mixed-scalar-container gate) + the old (assigned)obj test rewritten as a
  compiles-now assertion → integration 302→307 (plan said 306; the +1 is
  that rewrite, kept instead of deleted). REJECTION_FIXTURES −2+1.
  Zero-cost vs the 3d binary: ALL 287 pre-existing corpus programs
  byte-identical (modules/diamond verified as the recorded two-variant
  permutation); 4 new fixtures compile only under α (flipped surface);
  every new fixture's output predicted from the adjudications before
  running, all matched, all valgrind 0. Spec gained the
  subscription-target passage + amended (changed)/(assigned) table rows.
  KNOWN_MEMORY_BUGS EMPTY; ignored stays 2.)
  (3e-β landed 2026-07-18 per the approved plan; no STOP conditions hit.
  Phase 3 is COMPLETE. RUNTIME: the item 10(b) step table implemented as
  written — container set functions became compute-changed → store →
  hl_slot_retarget (steps 3/4: for each HL_SLOT_FOLLOW node on the slot's
  cell, collect that watcher's nodes on the old value's cell in list
  order, unsubscribe, re-subscribe on the new value's cell appending =
  item 9 order; any moved (deep) node re-marks the new subtree) → fire
  (step 5, hl_cell_set_ref_common) → release old LAST (step 6).
  Retargeting is unconditional under pause (moves nodes, not state) and
  stealth (the store still happens). SOUNDNESS HARDENING, adjudicated
  with the plan (the α analysis's claim that the notify walk saves its
  next pointer before the body was refuted by the code — the α text
  itself scheduled β to verify): (A) hl_cell_notify's direct-subscriber
  loop and the deep walk's per-ancestor node loops are now
  collect-then-fire (node snapshots, active/ended read at fire time) —
  a body-triggered retarget/unsubscribe on the walked cell can no longer
  free live traversal links; semantic delta: a watcher subscribed DURING
  a walk no longer fires for the in-flight event (no fixture pinned the
  old tail-append visibility, verified by survey + the zero-diff corpus
  run). (B) hl_notify_depth + a deferred-release list: the set family's
  old-payload release defers while any walk is in flight, drained at
  depth 0 — the walked cell and every fired body's borrowed snapshot of
  the old value outlive the outermost walk (this hazard was reachable in
  α's surface, unpinned; now closed and pinned). hl_cell_set_str shares
  the deferral (same borrowed-snapshot hazard) though it never retargets
  — a disclosed one-line extension beyond the plan's "untouched" note.
  CODEGEN: decl-form generate_watcher lifts the container content-
  modifier rejection and the Added/Removed/Moved/Deep validation arm
  (containers only; scalars keep defense-in-depth); construction emits
  content subscriptions on the payload deref cell with the full modifier
  map, one HL_SLOT_FOLLOW node per followed variable, mark_deep at
  construction for (deep); the decl body prologue deduplicates snapshot
  bindings and binds (added)/(removed)/(moved) aliases from the delta
  (typecheck fills resolved_alias_type form-independently). BOXING:
  mark_subscription takes the form flag from its two walk_* call sites;
  decl-form container subscriptions (any modifier) set slot_required.
  TWO LATENT α BUGS FIXED (both reachable only through slot-boxed
  containers, hit by probes): (i) `let old = xs` with xs slot-boxed
  aliased the payload borrow but tracked `old` as HeapType::Scalar with
  NO retain (scope exit called hl_scalar_release on a HiLowArray* —
  C compile error at best, mis-release at worst); the Ident-initializer
  arm now retains and tracks by payload kind. (ii) the program-body
  pre-pass stored the typechecker's Type::Unknown for inferred container
  lets, so a nested function assigning a program-scope slot variable
  keyed the setter on Unknown and rejected; the pre-pass now falls back
  to codegen inference for slot-needing declarations only. RECORDED, NOT
  FIXED: watcher bodies cannot call nested functions (emission order —
  no forward declarations; pre-existing, hit while probing), so the two
  retarget-during-fire fixtures use a companion watcher on a captured
  trigger instead of a helper function (same mid-walk shape); the
  aliased-slot edge (two variables holding the same container, both
  followed — retarget matches by (watcher, old-cell) and moves both);
  the pre-existing NULL-delta shape (mixing (changed) with an aliased
  (added) in one watcher derefs a NULL delta on a changed fire, both
  forms). TESTS: watcher_decl_container_rejected.hl deleted (flipped);
  7 live fixtures (follows [alias through retarget + old-silence],
  changed_slot_only [the sentinel's shape: push silent, rebind fires],
  deep_follows, object_deep_follows, pause_retarget, and the two
  rebind-in-body soundness shapes); the old rejection test rewritten as
  a compiles-now assertion on changed_slot_only. Integration 307→314;
  REJECTION_FIXTURES −1 → 26; KNOWN_MEMORY_BUGS EMPTY; ignored stays 2.
  Every fixture output predicted from the semantics before running; all
  matched; all valgrind 0. ZERO-COST vs the α binary: all 294 programs
  compiling under both byte-identical (zero diffs — diamond emitted
  identically this run), 6 new-only fixtures, 26 no-C-both = the
  rejection list; the 7th fixture (assigned_rebind_in_body) is α-legal
  surface and compiled identically under both. Spec gained the
  decl-form content-following bullet; bullet 1 narrowed to
  (changed)/(assigned).)

### Phase 4 — temporaries

- **4a** Wire producers into `pending_statement_stmts` (the inert buffer from
  `a21a6de`): every heap temporary produced in an expression registers into a
  statement-local release list, released at statement end via the single
  cell-release function. Retire `temp_owners`. *Gate: full suite + the
  string-operand leak tests (string_concat/string_equality valgrind-clean —
  closes the STATUS open question).*

  **(4a landed 2026-07-19, owner-adjudicated premise reconciliation.** The
  `pending_statement_stmts` landing pad from `a21a6de` is archived-compiler
  groundwork — `a21a6de` is not an ancestor of the fresh tree's HEAD and the
  field does not exist here; the audit's `:1xx`/`102x` line numbers are that
  older codebase's. In the fresh tree, expression temporaries were ALREADY
  released per-statement via `temp_owners` + `emit_temp_cleanup` (string_concat
  / string_equality already valgrind-clean), so 4a's real content was
  eliminating the SECOND temp mechanism: the store-site release path
  `needs_site_release_after_store` (object/array/f-string/function-expr
  literals and object/function-typed match, released at let/assign/push/insert/
  property-set sites). Ruling (owner): "unify onto the temp list" — those five
  fresh-production forms now mint statement-scoped tracked temps in Temporary
  position (released at statement end through the single `emit_temp_release`
  dispatcher, the codegen realization of "the single cell-release function"
  — there is no value-dispatching runtime release, `hl_cell_release` being only
  the header primitive), the retaining store keeps its +1, and
  `needs_site_release_after_store` + its 11 call sites were deleted. This is
  refcount-neutral (releases relocate from store sites to statement end) and
  also closed a real leak class: a fresh literal in a bare statement or as a
  borrowing call argument, covered by neither mechanism, leaked (pinned clean
  by temp_nonstore_object/array/arg). Two dead, wrong classification predicates
  (`expression_produces_heap_value`, `is_heap_allocating_expression`) were
  deleted. The dead `scope_depth` component of the temp maps was dropped;
  `temp_owners` + `enclosing_temp_frames` were kept as the cached-top +
  unwind-stack form of the one mechanism (fusing them into a single vec was
  judged to add error-prone break/continue/return index-shifting for no
  behavioral gain — a disclosed deviation from the plan's "fuse" wording).
  Zero-cost: all program outputs byte-identical corpus-wide vs the 3e-γ binary;
  C-diffs confined to release-site placement + temp-declaration hoisting.)**

### Phase 5 — queues

- **5a** Per-thread notification queue type + safe-point drain hooks
  (allocation/syscall boundaries), single-threaded semantics preserved
  (same-thread fires may stay synchronous). Deltas become queued values.
  *Gate: full suite unchanged.*
- **5b** `spawn` capture = cell retain/release; write-list compile check reads
  cell identity; `pid` as watchable cell; `hl_stealth_depth` becomes
  per-thread. *Gate: full suite + new concurrency tests (docs/concurrency-design.md
  scope).*

  > **DESCOPED — this bullet was NOT built as written (annotated 2026-07-23).**
  > Real Phase 5b (`bc51de4`) landed *minimal `async`* instead: pthread-spawned
  > `async` blocks with heap-env captures and a threaded runtime mode. There is
  > **no `spawn` construct, no `pid`, and no write-list check** in the tree —
  > see the descope record and rationale in `docs/state-of-migration.md` §F
  > (Phase 5b landed). The write-list model was never implemented (its only
  > home is `docs/concurrency-design.md`, marked historical/non-canonical). Any
  > later text here that says "structurally like the write-list check" has no
  > referent in the codebase.

### Phase 6 — process tier

> **SUPERSEDED — pending the Phase 6 re-brief (annotated 2026-07-23).** With the
> spec's process model (separately-launched programs; `shared` is the only
> cross-process channel — `hilow-design.md` "Cross-Process Watchers") and the
> current type universe, the 6a "sendable check" (what may be declared `shared`)
> **already exists in full** as Phase 5c's scope fence (shared is scalar-only;
> shared containers and `(deep)`-across-shared rejected — `state-of-migration.md`
> §H.2). So 6a as a standalone phase dissolves. Phase 6 is being re-scoped as a
> chat brief against tree truth (transport for shared scalars, shared
> containers, process lifecycle — ordering TBD in the brief). **Do not implement
> the bullets below as written**; they predate 5c and the surface ruling and
> reference the never-built write-list check. They remain here only as the
> pre-re-brief record.

- **6a** Sendable check (compile-time, structurally like the write-list
  check): no raw pointers / watchers / open handles cross a process boundary.
  Rejection tests first. *Gate: full suite + rejection tests.*
- **6b** Serialized transport (pipe / shared-memory ring), deep-copied deltas,
  same watcher syntax across the boundary. *Gate: full suite + cross-process
  integration tests.*
- **6c** Process lifecycle monitors (watcher on process handle). *Gate: full
  suite + monitor tests.*

### Standing rules

- Every step lands as one commit with the ritual output in the message.
- A step that deletes codegen machinery must show the *pinning* tests green
  before and after — never weaken a test to make a deletion pass.
- The user decisions formerly carried as open here were adjudicated 2026-07-14
  — see "Open questions — adjudicated" below.

### Open questions — adjudicated (2026-07-14)

Adjudicated by the project owner; no longer open. These govern the phases
named and are not to be re-litigated.

1. **Escape rejection (3d / §4.3 last row): dropped from the spec in Phase 3.**
   The enforcement check and the spec text are removed in the same commit that
   lands the boxing pass, together with a test demonstrating a sound escape
   (a watcher escaping its defining scope, working correctly). Until that
   commit, the rule stays enforced and its rejection tests stay green as
   written.
2. **String watching (§4.4 item 8): compile-time diagnostic until 1.5a
   completes.** Registering a watcher on a string produces a clean
   compile-time diagnostic ("string watchers land with the unified
   representation") until Phase 1.5a completes. After unification, strings
   inherit cell semantics with zero string-specific watcher machinery — no
   string-specific diagnostic, registration path, or firing code.
3. **Parent list / deep-watched bit vs `(deep)` surface syntax: header fields
   land in Phase 2 from the start; syntax re-enters at 2d only.** The parent
   list and deep-watched bit go into the cell header in Phase 2a as designed.
   `(deep)` array syntax is re-admitted to the language surface at 2d only,
   gated on nested-container tests written as part of 2d.
4. **Latent-bug policy (§3.4 a–d): NOT fixed in the current machinery.** Each
   of the four verified latent bugs gets an expected-fail/`#[ignore]`d test
   named for the bug during Phase 1.5 gap-test work (1.5d), flipping to a
   real assertion when Phase 2 replaces the path the bug lives in. (This
   refines §4.4's placement of items 2–3 at "the head of Phase 2": the tests
   are written in 1.5d as documented-ignore pins, and go live in Phase 2.)
5. **Object double-release (adjudicated 2026-07-15): fix NOW, in the current
   machinery — new Phase 1.5c "object ownership discipline".** The 1.5b
   valgrind gate surfaced a pre-existing use-after-free class: an object
   referenced from two places (proto link, array element, weak target) is
   released down both paths — invisible to the alloc/free counter because the
   second release reads freed memory and usually skips the second free. 17
   programs exhibit it (see `KNOWN_MEMORY_BUGS` in tests/valgrind_gate.rs).
   Unlike the §3.4 env-keying latent bugs — whose machinery the migration
   deletes — retain/release call-site discipline transfers directly to the
   cell model, so fixing it now is not wasted work. The gate expects these
   programs to fail until 1.5c removes them from the list.
6. **Weak-after-death access (adjudicated 2026-07-15): dead-weak read yields
   `unknown` with reason "weak referent released".** Member access on the
   result propagates per the spec's existing unknown rules ("unknown
   propagates through property access"). Implementation is Phase 1.5e, with
   the hilow-design.md spec edit in the same commit (per the CLAUDE.md rule
   that spec changes are deliberate, never silent). The expected-behavior
   test was written in 1.5d as `#[ignore]`d
   (test_weak_after_death_unknown_integration), its program carried on the
   gate's REJECTION_FIXTURES until 1.5e makes it compile.
7. **Optional payload matrix (adjudicated 2026-07-16, implemented in Phase 2b
   step zero): optional inners without a runtime payload kind are rejected at
   compile time with a diagnostic citing Phase 3.** The 2a requirement-0
   audit found `i64?`/`f64?`/`bool?` (and every other inner the missing
   return-type check admitted) constructible as mis-kinded `HL_OPT_I32`
   optionals. Ruling: reject at declaration (allow-list: i32, string, time,
   duration, money, plus the internal object case weak reads produce); the
   full payload matrix is scheduled into Phase 3, where scalar boxing builds
   the representation anyway. The three enabling bugs each reject cleanly or
   were fixed in the same commit: the missing return-type check got a narrow
   optional-only version (returns into `T?` must be `T`, `T?`, or unknown —
   the general return-type gap stays open); the `hl_optional_new_i32`
   constructor catch-all became explicit arms (+time/duration/money, which
   were previously unconstructible from user functions) with a hard error
   default; the refined-access raw-variable fallback became a hard error,
   with a money arm added.
8. **Dynamic property addition (adjudicated 2026-07-17, recorded as step zero
   of Phase 3a): design direction is OPEN-SHAPE OBJECT TYPES.** An object
   type may declare an open tail; dynamic property reads on an open object
   type as `T?`, yielding `unknown` with a reason when the property is
   absent; `(added)` subscriptions become legal only on open objects. Phase
   UNSCHEDULED: post-migration, alongside the string-literal revision. Until
   it lands, the Phase 2e rejections stand — `(added)obj` stays a
   compile-time diagnostic, and the runtime's new-key → ADDED mapping
   (already implemented in 2e) stays surface-unreachable.
9. **Multi-watcher fire order (adjudicated 2026-07-17, during Phase 3b):
   SUBSCRIPTION (DECLARATION) ORDER — the earliest-subscribed watcher fires
   first.** The unified firing path exposed that the legacy split
   implementations pinned CONTRADICTORY orders for the same abstract program
   (two watchers, one cell): the scalar firing block fired in registration
   order (pinned by nested_watchers, expression_coexists), while
   hl_cell_subscribe's prepend made container watchers fire newest-first
   (pinned by three 2c-era array fixtures). No single subscribe discipline
   satisfies both — the owner ruled declaration order; hl_cell_subscribe now
   appends, the three array expected files
   (test_array_watcher_added_and_changed_both_fire,
   array_moved_changed_both_fire, array_insert_watcher_fires) were flipped
   deliberately as part of the ruling (not a test weakening), and the spec's
   watcher-lifecycle section gained the fire-order sentence. The
   changed-before-assigned ordering on one changing scalar assignment is a
   separate, compatible property (two notify calls in hl_cell_set).
10. **Variable-slot semantics (adjudicated 2026-07-18, Phase 3e step zero).**
   (a) A slot's `(changed)` fires iff the newly-assigned value is unequal to
   the previous one under the type's OWN equality: value equality for
   strings (hl_string_eq), identity for containers, value for scalars
   (existing). `(assigned)` fires on every assignment regardless; on one
   assignment satisfying both, changed subscribers fire before assigned
   subscribers (the established two-notify order). (b) Expression-form
   watchers subscribe the VALUE by identity — unchanged (container
   content-mutation firing stays exactly as pinned since 2c/2e; a value
   subscription stays with the original value if the variable is rebound).
   Decl-form watchers on a variable follow REBINDING via the slot cell: on
   assignment the watcher's container-subscription nodes retarget from the
   old value's cell to the new value's cell, with deep-watched propagation
   into the new subtree (implemented in 3e-β). `(assigned)` — in either
   form — subscribes the slot: it is inherently about the variable. Spec
   edits land with each surface (the slot-vs-value subscription-target
   passage landed with 3e-α; the decl-form content-following bullet with
   3e-β). (3e-β landed 2026-07-18: the container-subscription nodes are the
   content modifiers (added)/(removed)/(moved)/(deep) — decl-form
   (changed)/(assigned) are pure slot subscriptions per the spec's
   "mutating in place never fires a variable subscription" sentence, so
   decl-form (changed)xs + push fires nothing, pinned. Retargeting is
   driven by one HL_SLOT_FOLLOW marker node per followed variable on the
   slot's cell; hl_slot_retarget moves the follower's nodes old→new in
   collected order (hl_cell_subscribe appends = item 9 order), re-marks
   deep subtrees, and runs unconditionally under pause and stealth.)
   (3e-γ landed 2026-07-19, owner-ruled from the step-zero probe of the
   aliased-slot edge: ATTRIBUTION is the semantics — each content-
   subscription node records the SLOT CELL whose follow created it
   (HiLowCellWatcher.origin, pointer identity set at construction via the
   hl_watcher_new_subscribed_origins triples variant; NULL for
   expression-form/slot/FOLLOW nodes, which never move), and
   hl_slot_retarget moves only nodes whose origin is the rebinding slot,
   via origin-filtered unsubscribe. Two followed variables holding the
   same container stay independent; origin is constant across moves, so
   each slot's later rebind finds exactly its own nodes — the FOLLOW-
   orphan follow-on resolved by the same mechanism. Zero-cost: diffs
   confined to the construction call of decl-form container-content
   watchers; everything else byte-identical vs the β binary.)
