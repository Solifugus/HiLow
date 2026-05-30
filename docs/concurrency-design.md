# HiLow Concurrency Design — Notes from 2026-05-28 evening

Status: design solved (load-bearing decisions made). Implementation phase is real future work, but the "what shape?" question is answered.

## The model in one paragraph

Concurrency in HiLow is **goroutine-style spawning with explicit write capabilities and watcher-based coordination.** A `spawn` block runs concurrently, captures variables from its enclosing scope, and is restricted by a write list — anything not in the list is read-only inside the spawn. The default is permissive (no auto-wait, parallel mutation allowed on write-listed variables) but safer than Go because the read-only-by-default check eliminates the most common race-bug class at compile time. Coordination happens via watchers (reactive) or explicit `wait(...)` (blocking). Watchers don't extend scope lifetime; spawned processes can keep their watchers alive while they run.

## Syntax (sketch — not committed)

```hilow
let pid = spawn(arr, counter) {
    // arr and counter are writable here
    // everything else captured from outer scope is read-only
    arr.push(42)
    counter = counter + 1
}

// pid is a handle: monitor, pause(?), cooperatively cancel
let _ = watcher((complete)pid) { print("done") }

// or explicit wait
wait(pid1, pid2)
```

## Decisions made

1. **Spawn syntax**: `let pid = spawn(write_list) { ... }`. The write list is explicit; defaults to empty.
2. **Read-only by default**: variables captured from outer scope are READ-ONLY inside the spawn body unless listed in the write capability list. This eliminates accidental races on shared state at compile time.
3. **Return semantics**: `return` returns when reached. No auto-wait for spawned children. If the programmer wants to wait, they use `wait(pid1, pid2)` explicitly. (Departure from structured concurrency — chose Go's flexibility over Trio's safety.)
4. **Spawn captures extend lifetime**: a spawn block keeps refcounts on its captures so they aren't freed while the spawn is still running. (Necessary because `return` doesn't wait — without this, captures would be freed mid-spawn → use-after-free.)
5. **Watcher/process lifecycle**: 
   - A watcher does NOT keep its enclosing scope alive.
   - A spawned process DOES keep its watchers alive (until the process ends).
   - Process ends → its watchers end too.
6. **Coordination primitives**:
   - **Reactive**: `watcher((complete)pid) { ... }` — fires when pid finishes.
   - **Blocking**: `wait(pid1, ...)` — blocks the calling scope until all listed pids complete. (Plural form needs design: wait_all vs wait_any; possibly timeout.)
7. **Pid handle surface** (rough): query status, watchable for completion, cooperative cancellation. NOT forced kill — cooperative cancel only (forced kill mid-mutation in a refcounted runtime is dangerous).
8. **Parallel mutation on write-listed variables is allowed**. The programmer can write `spawn(counter) { counter += 1 }` from two spawns simultaneously and get the race (intentionally — sometimes that's the design, e.g. speculative parallelism, parallel accumulation). No conflict detection across simultaneous spawns (could come later if useful; design has subtleties — see "Open Questions" below).
9. **Read-only check is COMPILE-TIME, LEXICAL**: codegen tracks a writable-set on entering a spawn body (populated from the capture list); every direct assignment / mutating method call in the spawn body is checked against it; not-in-set → compile error. Cheap to implement (analogous to existing heap-owners tracking), composes with the capture list trivially.
10. **Function calls within spawns are TRUSTED, not checked**: if a spawn calls `someFn(arr)` and `someFn` internally mutates `arr`, that's not caught by the read-only check. Accepts the laundering risk in exchange for not requiring Rust-style permission annotations on every function signature. (HiLow-philosophy choice: catch the easy cases cheaply, trust the programmer for the rest.)

## What this is, design-space-wise

- **Closer to Go than to Rust or Trio.** Programmer-flexible, lightweight, low-ceremony.
- **Safer than Go** because of compile-time read-only-by-default. Catches the common "accidentally shared a variable across spawns" bug class that Go leaves to the programmer.
- **Simpler than Rust** because the read-only check is lexical and not type-system-based. No borrow checker, no lifetime annotations, no ownership transfer machinery.
- **Native integration with watchers** is the genuinely original part. Most languages bolt a concurrency model onto reactive primitives separately. HiLow uses watchers AS coordination primitives.

## Open design questions (real, not yet decided)

- **Wait semantics**: `wait(pid1, pid2)` = wait_all or wait_any? Probably both forms needed; need to pick syntax.
- **Conflict detection across simultaneous spawns**: should the runtime detect two simultaneous spawns with overlapping write lists and error? Conceptually appealing but has subtleties — partial-overlap cases, sub-variable granularity (`arr[0]` vs `arr[1]` are technically non-overlapping but the system can't know that cheaply), runtime overhead. Probably defer; consider adding later if real-world use surfaces value.
- **Deep data in structures**: if `obj` is in the write list, can the spawn mutate `obj.deeply.nested.field`? Likely yes — mutation through a granted reference is allowed. But what about two spawns both granted `obj` mutating different fields? Technically not a race; system would reject it (false positive but safe). Acceptable.
- **`pause` semantics**: cooperative? Pauses for tight CPU loops are hard. Probably "checkpoint at allocation/syscall boundaries" or similar. Needs runtime design.
- **Cooperative cancellation mechanism**: how does the spawn body know it's been asked to stop? A flag the runtime sets that the body can check (`if (should_stop()) return`)? Or implicit at allocation points? Needs design.
- **Error propagation**: if a spawn throws/errors, how does it surface? Through the pid handle? Through the watcher? Needs design (probably both: pid carries the error state, watcher can observe it).
- **Memory model statement**: what does HiLow guarantee about ordering of reads/writes across processes? At minimum: word-sized reads/writes are atomic (hardware-true on basically all modern systems). Beyond that: need to state what synchronization primitives establish happens-before. Likely: completion of a spawn establishes happens-before from the spawn's writes to whoever waits on it. Anything stricter needs a sync primitive (mutex? atomic types in stdlib?). Decide as it comes up.

## Implementation cost estimate (very rough)

- Compile-time read-only check: ~1 evening, well-bounded.
- Spawn block parsing + AST + basic codegen (just spawn, no coordination): ~1 evening.
- Runtime threading layer (pthreads pool, basic spawn-and-run): ~1 weekend.
- Capture-lifetime-extension (spawn keeps captures alive): subtle, needs careful design — possibly a couple of evenings.
- `wait()` primitive: ~1 evening (depends on runtime choices).
- Watcher-pid integration: probably ~1 evening (the watcher system already does most of what's needed; just need pid handles to be watchable).
- Cooperative cancellation: ~1 evening.

Roughly 6-8 sessions of focused work for a first usable async. Real but bounded.

## NOT in scope (deferred to future or never)

- Cross-process shared memory (the plan mentioned it; almost certainly drop — process-level concurrency in a refcounted language is its own multi-month undertaking with marginal benefit for HiLow's audience).
- Borrow-checker-style ownership tracking.
- Function signature permission annotations.
- Async/await coloring or any kind of two-universe split.
- Forced kill of running processes (cooperative cancel only).

## Where this design departs from the plan as written

The plan's "Phase 10b: Async and Shared" describes a threads-and-locks-and-shared-keyword model that's basically 1995-vintage. This design *replaces* that with something more original (watcher-integrated, structured-ish, capability-aware). The plan needs updating to reflect the new design before implementation begins. **Don't implement 10b as the plan currently describes it.**

## What's still genuinely open (the things to sit with)

1. **Memory model formalization** — what HiLow guarantees about cross-process reads/writes. This determines what programs are correct vs. undefined. Worth writing up properly before implementation.
2. **The capture-lifetime mechanic** — how exactly does a spawn extend its captures' refcounts? Needs codegen design.
3. **Should there be ANY shared-but-controlled-mutation primitive?** Atomic counter type? Mutex-protected value? Channel? Or just "write-lists and trust the programmer" + nothing fancier? Lean toward minimal for first version.

## Next steps when picking this up

1. Write a real design doc (this scratch-pad is just notes — needs to become something you'd hand to a critical reader and have them push back).
2. Write 3-5 imaginary HiLow programs using this model and see if they feel good (producer/consumer, parallel reduce, UI event loop, fan-out fan-in, monitor daemon). If any feels awkward, that's data.
3. Decide the memory model statement.
4. THEN write the implementation prompts for the phases above.

---

**Saturday-morning-you starting from scratch: read this doc, decide if anything still feels off after sleep, then either iterate the design or start writing imaginary programs against it.**
