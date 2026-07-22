# HiLow Watcher Redesign: The Cell Model

Status: agreed design direction, July 2026. This brief records decisions already
made. Do not re-litigate the architecture; audit the codebase against it and
produce a migration plan.

## Motivation

HiLow currently has two watcher implementations (a third was pending):

1. **Scalar watchers** — compile-time. Subscriptions live in name-keyed maps in
   codegen (`watcher_subscribers`); firing code is injected inline at every
   assignment site; per-watcher static `_active`/`_ended` bools; scope-exit
   deactivation is emitted code.
2. **Array/heap watchers** — runtime. Registered on the array, fired from
   mutation sites inside runtime.c, with heap-allocated env structs for capture.

This split is the root cause of the recurring bug taxonomy: name-based
subscription requiring shadow masking; name-keyed codegen maps leaking across
function boundaries; env use-after-free because envs are owned by the declaring
scope; unreachable scope-exit deactivation under early returns; non-reentrant
static delta buffers; ad-hoc handling of expression temporaries. Each new axis
(closures, deep structure, concurrency) currently must be solved twice.

## Core decision: one runtime cell model

- A **cell** is a small runtime header + value: `{ refcount, watcher list,
  parent list, version, deep-watched flag }`.
- **Identity-based subscription.** A watcher holds a pointer to the cell it
  watches — never a lexical name. Shadowing becomes trivially correct. All
  name-keyed subscription maps in codegen are deleted.
- **Boxing is selective.** A compile-time pass boxes a scalar into a cell only
  if it is ever subscribed. Unwatched variables remain raw C locals —
  zero-cost when unused is preserved.
- **One firing path.** Assignment to a watched variable compiles to
  `hl_cell_set(cell, value)` (equality check + notify). Array/object mutators
  (`push`, index-assign, `move`, remove, …) fire from inside the runtime.
  Codegen injects no firing code anywhere.
- **Watchers own their envs.** Watcher objects are runtime values; captured
  envs are refcounted and owned by the watcher, not the declaring scope.
  Scope death cannot orphan a live watcher. Deactivation is a runtime call,
  not emitted end-of-block code.

## Deep data and arrays

- Container cells hold a **parent list** (reuse the watcher-list mechanism).
- Shallow modifiers (`changed`, `added`, `removed`, `moved`) fire on the
  mutated cell's own list, from the runtime mutator, with the delta as a value.
- `(deep)` propagation: after firing its own list, a mutated cell walks its
  parent list; each ancestor fires only its `(deep)` subscribers. A
  "deep-watched" bit set down the chain at subscription time lets the walk be
  skipped entirely when nobody deep-watches — zero cost when unused.
- Deep-fire parameter binds the subscribed variable's current value; the
  precise delta (what changed, where) is optional alias data.

## Firing semantics across threads (decided: declaring-thread model)

- Each thread owns a **notification queue**. A write on thread A to a cell
  watched from thread B enqueues `(watcher, delta)` onto B's queue.
- Watcher bodies always execute on their **declaring thread**, at safe points
  where the queue is drained (allocation/syscall boundaries — the same
  checkpoints used for cooperative pause/cancel).
- Same-thread fires may run synchronously or via the queue; pick whichever
  simplifies re-entrancy, but semantics must be: a body never runs on a
  foreign thread, never races with its declaring thread.
- Deltas are values pushed onto the queue — this replaces the static
  `temp_buffer` and is naturally re-entrant.

## Expression temporaries

- Per-statement release list ("autorelease" pattern): codegen registers every
  heap temporary produced during an expression into a statement-local list,
  released at statement end via the single cell-release function.

## Concurrency tiers

- **Threads (spawn):** share cells. Write-list compile-time check stays as
  designed. Spawn capture lifetime = cell retain/release. `pid` is a
  watchable cell; `(complete)pid` and `wait()` build on it.
- **Processes (OS-level, new tier):** share nothing — Erlang-style isolation.
  No write lists across processes; communication is watching + sending only.
  The per-thread queue generalizes: cross-process it becomes a pipe /
  shared-memory ring buffer, and deltas are serialized and deep-copied.
  Same watcher syntax on both sides; the runtime picks the transport.
  Process lifecycle monitoring = watcher on the process handle (monitor
  semantics).
- **Sendable check (new compile-time rule):** values crossing a process
  boundary must be deep-copyable — no raw pointers, watchers, or open handles
  inside. Structurally analogous to the write-list check.
- Channels are library sugar, not a primitive: a watched array where one side
  pushes and the other watches `(added)`.

## Migration phases (proposed order)

1. **Audit** (see task below) — no code changes.
2. **Arrays first:** retrofit the cell header onto existing array/heap values;
   move all firing fully into runtime mutators; watcher-owned envs; delete
   array-related codegen injection.
3. **Scalars:** boxing pass; `hl_cell_set`; delete `watcher_subscribers`,
   shadow masking, static active/ended bools, emitted deactivation.
4. **Temporaries:** per-statement release lists.
5. **Queues:** per-thread notification queues + safe-point draining; make
   spawn/write-list use cells.
6. **Process tier:** sendable check, serialized transport, process monitors.

Each phase must keep the full test suite green before the next begins.

## Non-goals / constraints

- Language surface (watcher syntax, modifiers, `!>`/`!<`, write lists) does
  not change except for the additions named here (process tier, sendable).
- No GC. Refcounting + explicit release lists only.
- Zero cost when unused: unwatched scalars stay raw locals; un-deep-watched
  mutations skip the parent walk.
- Runtime C grows; codegen shrinks. That trade is intentional — runtime is
  testable in isolation (valgrind), injected codegen paths are combinatorial.

## Phase-5b amendments (2026-07-21, recorded before implementation)

Two clarifications to "Firing semantics across threads" as 5b lands minimal
`async`:

1. **Same-thread firing stays synchronous (R1).** The section above leaves
   same-thread fires as "synchronous or via the queue; pick whichever
   simplifies re-entrancy." 5b picks **synchronous** (nested within
   `hl_notify_depth`): the inbox is the *cross-thread* path exclusively. Owner
   ruling (AskUserQuestion, 2026-07-21). Consequence: the previously-`#[ignore]`d
   `watcher_reentrant_deferred` fixture — which pinned a *deferred* same-thread
   output written under the earlier reading — has its expected output rewritten
   to the synchronous/nested result and the test is activated (renamed
   `watcher_reentrant_sync`). No runtime behavior change; the existing
   synchronous binary already produced this output.

2. **Drain safe points gain a loop back-edge check in threaded mode.** 5a's
   safe points are runtime-entry points (allocation/syscall). 5b adds, **in
   threaded runtime mode only** (a program that uses `async`/`shared`), a
   codegen-emitted safe-point check on loop back-edges (`for`, `while`,
   `loop`). Rationale: with runtime-only safe points a pure-compute loop never
   allocates and so never drains — the spec's own `loop { }` event-loop idiom
   would starve its thread's inbox forever. The check is a read of the
   inbox-nonempty flag (`hl_thread_safepoint()`), draining only at
   `hl_notify_depth == 0`. Single-threaded mode emits **nothing** — the
   corpus byte-identical invariant is preserved by the mode switch, not by
   hope.

