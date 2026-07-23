# Phase 6 Brief — Process Tier (Cross-Process `shared` via Placement)

Status: adjudicated brief, ratified rulings R-A–R-E. Supersedes audit
§Phase 6 entirely (annotated SUPERSEDED in the maintenance commit
5136008); the audit's "sendable check / serialized transport /
lifecycle monitors" plan was written against a counterfactual 5b and a
write-list check that was never built. Ground truth below is from the
6a step-zero reconciliation and the maintenance debrief. No session
implements from audit §6.

This document is a **source of truth for Phase 6**, alongside
`hilow-design.md` (spec) and `development-plan.md`. Where it and the
audit disagree, this brief wins; where it and the spec disagree, stop
and ask (per CLAUDE.md).

---

## 1. Ground truth (fresh tree, as of 5136008)

- There is no process construct in the language or the tree, and none
  will be added: the spec's cross-process model (design:1765–1791) is
  two **separately-launched programs** sharing a `shared` variable.
- `shared` today (5c): in-process, cross-thread, scalars only; atomic
  payload; per-shared-cell subscriber lock; containers and `(deep)`
  across shared rejected with diagnostics.
- Delivery machinery (Phase 5): per-thread coalescing MPSC inboxes,
  in-process only; same-thread synchronous (nested, R1); drain safe
  points = runtime entries + loop back-edges (threaded mode only);
  coalescing license R3 with mandatory delta accumulation.
- Threaded runtime mode is engaged by `async`/`shared` tokens; atomic
  refcounts; 353-program single-threaded corpus byte-identical through
  all of Phase 5.
- Valgrind gate: split lanes (single-threaded ~34s, threaded ~2s,
  --fair-sched=yes, HILOW_GATE_SPIN_N knob, is_threaded_source
  auto-routing).
- Ignored ledger: 2 → becomes 1 in this phase's opening commit (see
  §3-6a step zero). KNOWN_MEMORY_BUGS empty. REJECTION_FIXTURES 28.

## 2. Rulings (ratified, binding)

R-A. **Placement, not transport.** Cross-process `shared` is a typed
     slot in a named shared-memory segment mapped by every
     participant. Nothing is serialized, sent, or received —
     "serialized transport" is retired as a concept. "Sendable" is
     replaced by **placeable**: fixed-size, position-independent,
     pointer-free, watcher-free, handle-free. In Phase 6 the placeable
     set is exactly the scalar types. Shared containers (in-process
     first, shm later if ever) are a separate future phase.

R-B. **No cross-process queues, ever.** In-process inboxes are
     untouched; no process can reach another's queue. A write to a
     cross-process shared cell publishes value + epoch to the segment;
     the watching process's declaring thread pulls at its existing
     drain safe points (epoch compare) and delivers locally.
     Correctness rests on the coalescing license R3, which
     epoch-compare implements word-for-word: at least one fire per
     logical change, rapid changes may coalesce, the body reasons
     about drain-time state.

R-C. **Surface**: `shared("name") let x: i32 = 0`. Bare `shared`
     keeps its exact 5c meaning (in-process, cross-thread). The
     gradation is: `let` → local; `shared let` → cross-thread;
     `shared("name") let` → cross-process; each step pays only its own
     cost. The segment name is a compile-time string literal (no
     interpolation), validated against `[A-Za-z0-9._-]+`; the runtime
     maps it to shm object `/hilow.<name>`. No manifest, no flag, no
     implicit coupling.

R-D. **Creation and attachment**: the `shm_open(O_CREAT|O_EXCL)`
     winner runs the initializer; it writes the header and payload,
     then publishes init-complete with release semantics. Later
     attachers open, map, wait for init-complete, then verify the
     header — magic, layout version, ABI version, type tag, payload
     size — and any mismatch is a **startup error with a diagnostic**,
     never a warning. Attaching observes the current value and fires
     nothing. A losing attacher's initializer is not run — this is the
     one place `shared("n") let x = 5` does not mean what it locally
     appears to mean; the docs and the diagnostic reference say so
     loudly.

R-E. **Sub-phases**: 6a placement + attachment + the two-process test
     harness; 6b delivery; 6c lifecycle — 6c is deliberately a sketch
     here and gets its own mini-brief after 6b lands (the 4a lesson:
     detailed plans written far ahead of the tree rot).

## 3. Sub-phase structure

### 6a — Placement, attachment, harness

**Opening commit (separate, before phase work)**: delete
test_phase7a_integration per the sizing (superseded at birth, stale
reason, wrong assertions); if `hl_object_get_str` is uncovered by the
surviving object_* fixtures, add one small positive fixture exercising
a string-property read+print. Ignored ledger → 1
(unknown-with-options → 9b only); STATUS updated.

**Deliverables**

- Grammar + AST: the `shared("name")` form. Name validation at compile
  time (`[A-Za-z0-9._-]+`, non-empty, length-bounded for portability);
  invalid names are rejection diagnostics with tests.
- Type fence: `shared("name")` on any non-scalar type → rejection
  diagnostic in placeability language ("not placeable in a shared
  segment: contains <pointers/watchers/handles>"), tests pinning each
  category. This is the sendable check, landed as three diagnostics
  instead of a phase.
- Segment layout, one variable per segment (multi-variable segments
  deferred): header { magic, layout version, ABI version, type tag,
  payload size, init-state, epoch: u64 } + aligned payload. The header
  is the contract 6b and 6c both build on; version it from day one.
- Create/attach per R-D, including the attacher's bounded wait for
  init-complete (spin-then-yield with a timeout → startup error on
  timeout, so a crashed-mid-init creator cannot hang attachers
  forever).
- Detach at program exit: munmap + close. **Segments are persistent by
  default** — they outlive any single process, matching the
  separately-launched-programs model; no unlink in 6a/6b (cleanup
  policy is 6c's core question). The harness, however, MUST unlink its
  segments in teardown so test runs are hermetic; a leaked test
  segment is a harness bug.
- Permissions: segments created 0600 — same-user only. The
  `/hilow.<name>` namespace is global per user; collisions between
  unrelated applications of one user are possible and documented;
  namespacing refinements are deferred.
- In-process integration: the cross-process shared variable is still a
  normal cell in-process — its payload storage moves to the mapped
  slot (atomic load/store), its subscriber list, lock, and in-process
  delivery are exactly the 5c machinery. Writes in 6a already bump the
  segment epoch (release), even though nothing pulls it until 6b —
  the write protocol is complete from the start.
- **Two-process test harness** — the phase's real infrastructure
  investment: fixtures may declare a helper program; the harness
  builds both, runs both (ordering controllable), runs BOTH processes
  under valgrind in the gate lanes, asserts on both outputs and both
  exit codes, and unlinks segments in teardown. Assertions follow the
  5b discipline: order-insensitive invariants only.

**6a fixtures (minimum)**: create-then-attach both orderings (value
persistence: A writes and exits, B attaches and reads); type-mismatch
startup error (two programs, different types, one name); name-
collision behavior documented by a fixture (same name, same type,
independent programs — they share, by design); crashed-mid-init
timeout path (harness-induced); rejection matrix (non-scalar × 3
categories, invalid names); bare-`shared` control proving 5c behavior
byte-identical.

**Expected diffs**: none for the existing corpus (new syntax is the
only entry point); runtime + codegen additions reachable only from
`shared("name")` programs. Any diff in an existing program is
stop-and-report.

### 6b — Delivery

**Deliverables**

- Write path: atomic payload store + epoch increment with release
  ordering (single publication point), then the normal in-process
  notify. Same-thread and same-process semantics unchanged.
- Watch path: declaring thread's context tracks (segment, last_seen
  epoch, last_delivered value) per watched cross-process cell. At
  existing drain safe points: acquire-load epoch; on advance, deliver
  locally per the semantics below, update last_seen.
- **Fire semantics (adjudicated here)**: `(assigned)` fires iff the
  epoch advanced since last delivery — at-least-once, coalescible per
  R3; the count of fires is NOT the count of remote assignments and
  fixtures must not assert it is. `(changed)` fires iff the drain-time
  value differs from the last-DELIVERED value — an ABA sequence
  entirely inside one coalescing window is unobservable and fires
  nothing, which is exactly R3's license. Record both in
  state-of-migration §adjudications.
- Idle behavior: **backoff polling** — at idle safe points with an
  empty inbox and no epoch advance, back off (yield escalating to a
  bounded sleep, cap ~1ms). A blocking futex wait on the epoch word is
  a deferred optimization (trivially correct only in the
  single-watched-cell case; multi-cell wait needs design). 6b ships
  correct-and-simple; latency tuning is not this phase.
- The spec's Cross-Process Watchers example (design:1765–1791) becomes
  a real two-program fixture with a one-token edit (`shared` →
  `shared("…")`), and the spec text gets that edit in the same commit
  — the example must compile as written from 6b onward.

**6b fixtures (minimum)**: the spec example, realized; producer
process / consumer process monotone-threshold (proves epoch pull +
coalescing under real cross-process contention); ABA fixture pinning
the `(changed)` rule; two watcher processes on one segment (both
observe); watcher process exits mid-production (producer unaffected —
no cross-process coupling to prove the no-queues ruling); bare-shared
and single-threaded controls unchanged.

**Expected diffs**: existing corpus untouched; runtime delivery
additions + the spec edit.

### 6c — Lifecycle (sketch only — mini-brief after 6b)

Scope to be adjudicated then, expected to include: segment cleanup
policy (persistent-by-default vs unlink-on-last-detach, and what crash
tolerance means for either — pid-table scan, robust-futex-style
recovery, or explicit user-driven cleanup); attach/detach visibility;
and process liveness **as a watchable** — the audit's pid-as-cell idea
resurrected legitimately: attachment yields a handle whose aliveness
is a cell, so process supervision becomes ordinary watcher code. That
is the payoff that makes 6c worth its grit, and why it must not be
designed before 6b's reality exists.

## 4. Placement and protocol axioms (one place)

1. Placeable = fixed-size, position-independent, pointer-free,
   watcher-free, handle-free. Phase 6: scalars exactly.
2. One variable per segment; the header is versioned and verified on
   every attach; mismatch = startup error.
3. Every write is one release-ordered publication (payload + epoch);
   every remote observation begins with an acquire-ordered epoch load.
4. Subscriber lists, locks, inboxes, and watcher values never leave
   their process. The segment holds values and epochs only —
   never pointers, never watcher state.
5. First creator initializes; attachers verify and observe; nobody
   fires on attach.
6. Segments persist beyond processes (until 6c defines cleanup);
   the test harness always unlinks its own.
7. Delivery to a remote watcher happens only on its declaring thread,
   only at its own safe points, under R3 semantics — the process tier
   adds a *source* of local deliveries, never a second delivery
   system.

## 5. Invariants held across Phase 6

- Existing corpus (single-threaded AND threaded/5c fixtures):
  byte-identical C, identical outputs, both valgrind lanes clean, at
  every sub-phase close. Cross-process fixtures are new programs run
  under the two-process harness with both processes valgrind-clean.
- KNOWN_MEMORY_BUGS empty at each close; bugs found are fixed
  in-phase with class-pinning fixtures (4b pattern) or block the
  close.
- Ignored ledger: 1 from the 6a opening commit onward
  (unknown-with-options → 9b). REJECTION_FIXTURES grows in 6a; report
  counts.
- Full ritual per session; STATUS, state-of-migration adjudications,
  CLAUDE.md phase line in the same commit. One sub-phase, one session,
  one phase-commit (6a's opening ledger commit separate).
- Step zero of every session: verify this brief's ground-truth section
  against the tree; stale premises are stop-and-report, not
  workarounds.

## 6. Deliberately deferred (recorded so no session pulls them in)

- Shared containers — own phase; in-process design first; shm layout
  only after that.
- Multi-variable segments; segment namespacing/ACL refinements beyond
  0600 + documented per-user global namespace.
- Blocking futex wait (beyond the noted single-cell possibility);
  latency tuning generally.
- Cross-machine anything.
- Prover concurrency checks; the `(atomic-add)=` family.
- Serialization — retired concept, listed here so its absence is
  legible as a decision, not an omission.
