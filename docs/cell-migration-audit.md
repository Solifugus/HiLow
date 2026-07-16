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
| `temp_owners` (:129), `pending_statement_decls` (:133), `temp_counter` (:131) | | statement-end temporary cleanup (Phase 11a) | **replaced** in migration Phase 4 by per-statement release lists through the single cell-release function |
| `pending_statement_stmts: Vec<String>` | :135 | inert landing pad (commit `a21a6de`): flushed at 1027–1032 before each statement body, **no producers yet** | groundwork for Phase 4 — producers get wired in then |
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
  reallocs. Weak-after-death access semantics remain unadjudicated; see
  STATUS.md Open questions.) *Gate: full suite + gate green with an empty
  KNOWN_MEMORY_BUGS list.*
- **1.5d** Add gap tests §4.4 items 1, 4, 5, 6, 7, 9 (all pin current
  behavior). Item 1 (re-entrant mutation) may *expose* the temp_buffer bug —
  if it fails, mark `#[ignore]` with a STATUS entry naming Phase 2c as the
  fix, per CLAUDE.md's ignore policy. *Gate: suite green (ignores documented).*

### Phase 2 — arrays first

- **2a Cell header.** Introduce `HiLowCell` header fields (watcher list,
  parent list, version, deep-watched flag) onto `HiLowArray` — mechanical
  layout change, firing behavior identical. *Gate: full suite, no behavior
  change expected.*
- **2b Watcher values + owned envs.** Array watcher registration constructs a
  runtime watcher object owning a refcounted env; `hl_array_register_watcher`
  takes the watcher; delete codegen env-free/unregister
  (`emit_scope_cleanup`/`emit_early_return_cleanup`/`emit_temp_cleanup`
  Environment arms), delete `array_watcher_registrations`. Scope exit releases
  the watcher value instead. Kills §3.4(b)(c)(d). Write §4.4 item 2 and 3
  tests at the head of this step. *Gate: full suite + valgrind; the
  dies-with-scope and capture tests are the sentinels.*
- **2c One firing ABI + value deltas.** Unify all eight mutator firing loops
  on the `(env, cell, delta)` call through a single notify helper; fix the
  move 2-arg bug (§3.4a); replace both static temp_buffers with caller-owned /
  heap-copied deltas; move `hl_array_remove`'s return off the static buffer.
  Un-`#[ignore]` the re-entrancy test. *Gate: full suite + re-entrancy test
  live.*
- **2d Parent lists + deep.** Container cells get parent lists; `(deep)`
  subscription sets the deep-watched bit down the chain; mutators walk parents
  only when the bit is set. New tests: nested-array deep watch (§4.4 gap —
  currently untestable because deep on nested arrays doesn't exist). *Gate:
  full suite + new deep tests; zero-cost check = un-deep-watched benchmarks
  or at minimum codegen-diff inspection showing no walk.*

### Phase 3 — scalars

- **3a Boxing analysis.** Compile-time pass marking variables that are ever
  subscribed (decl-form, expression-form, or captured-into-watcher). No
  codegen change yet; expose as a queryable attribute + unit tests on the
  analysis. *Gate: full suite (no behavior change).*
- **3b hl_cell_set.** Boxed scalars lower to cells; assignment to them becomes
  `hl_cell_set`; delete the scalar firing block (1999–2136). Unwatched
  variables stay raw locals — verify by inspecting generated C for an
  unwatched program (zero-cost check). *Gate: full suite; changed/assigned
  and stealth scalar tests are the sentinels.*
- **3c Runtime watcher lifecycle.** Watcher declarations construct runtime
  watcher values (both forms share the heap path); delete static
  `_active`/`_ended` bools, the four static helpers, activation/deactivation
  emission (Phase 4/5 loops in both block walkers + `emit_main_function`),
  and the static-dispatch method arm. Scope exit releases the watcher —
  reaches early returns for free; §4.4 item 4 test flips from pinning the
  hole to pinning the fix. *Gate: full suite; pause/resume/end/isActive,
  scope-bounding, factory tests are the sentinels.*
- **3d Delete name-keyed subscription.** Delete `watcher_subscribers`,
  `heap_watcher_subscribers`, `watcher_name_to_id`,
  `scalar_watcher_captures`, shadow-masking (672–705, 6946–7015), the
  `WatcherSubscription`/`HeapWatcherSubscription` structs, and the
  `temp_watcher_expr_*` side-channel. Shadowing tests must pass unchanged.
  **User decision needed at this step:** do the two escape-rejection tests
  (§4.3 last row) remain spec (keep a deliberate check) or fall away (boxing
  makes escape sound)? *Gate: full suite (modulo that decision).*

### Phase 4 — temporaries

- **4a** Wire producers into `pending_statement_stmts` (the inert buffer from
  `a21a6de`): every heap temporary produced in an expression registers into a
  statement-local release list, released at statement end via the single
  cell-release function. Retire `temp_owners`. *Gate: full suite + the
  string-operand leak tests (string_concat/string_equality valgrind-clean —
  closes the STATUS open question).*

### Phase 5 — queues

- **5a** Per-thread notification queue type + safe-point drain hooks
  (allocation/syscall boundaries), single-threaded semantics preserved
  (same-thread fires may stay synchronous). Deltas become queued values.
  *Gate: full suite unchanged.*
- **5b** `spawn` capture = cell retain/release; write-list compile check reads
  cell identity; `pid` as watchable cell; `hl_stealth_depth` becomes
  per-thread. *Gate: full suite + new concurrency tests (docs/concurrency-design.md
  scope).*

### Phase 6 — process tier

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
