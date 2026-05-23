# Project Status

> Persistent state record for the HiLow compiler project. Claude Code reads this at the start of every session and updates it at the end. For session-by-session detail, see `SESSIONS.md`. For the session-start brief, see `CLAUDE.md`.

---

## Current state

**Phase:** Array Phase A complete — foundational arrays (literals, indexing, length, primitives)
**Status:** HiLow now has working arrays at the value level for the read path. Array Phase B next (mutation: push, index-assignment, remove) — the prerequisite that unblocks Phase 10-ε (watcher mutation modifiers).
**Branch:** main
**Last commit:** Array Phase A: foundational arrays (literals, indexing, length, primitives)

**Tests:** 151 integration, 68 parser, 28 typecheck_module, 57 typecheck_tests, 8 resolver, plus other unit suites — all passing.

---

## Open questions

**Argument type checking at call sites** (general gap, not module-specific): Existing call-arg type checking is incomplete. Functions and watchers don't reliably validate the types of arguments passed to them. Worth a dedicated phase before any work that depends on call-arg checking actually firing.

---

## Active deferred work

### Code quality items pending

- **Scope-exit deactivation is unreachable in functions with early returns.** Phase 10-θ-fixup emits `hilow_watcher_<id>_end();` at block exit (after the last statement), but if the function returns earlier, the deactivation never runs. In practice this doesn't cause incorrect behavior for nested watchers — the watched variable is function-local and doesn't exist between calls, so no spurious firing can happen. But the dead-code emission is wrong on its face and will become a real issue when Phase 10-δ introduces escaping watchers whose state outlives their declaration scope. Address as part of 10-δ or earlier if a use case surfaces.

- **Activation logic is duplicated** between `generate_block` and `generate_block_with_parameter_context`. Both block walkers contain the same Phase 4 activation loop. Small extraction opportunity. Not blocking.

### Arrays (new track)

**Resolved (Array Phase A):** Dynamic arrays of primitive element types — literals (`[1, 2, 3]`), indexing (`arr[i]`), `.length`, heap-allocated and refcount-cleaned-up. HiLowArray runtime struct.

**Still pending — Array Phase B:** Mutation. `.push(x)` as a user-callable method, index-assignment (`arr[i] = x`), and `.pop()`/`.remove()`. This is the phase that unblocks Phase 10-ε. Design should account for where watcher-notification hooks will attach (see design notes when that phase is scoped).

**Still pending — Array Phase C:** Heap element types — arrays of objects, strings, tuples, or nested arrays, with proper per-element retain/release on insert/remove.

**Still pending — Array Phase D:** for-in iteration over arrays. (for-in currently works over objects only.)

**Still pending — fixed arrays (`[T; N]`):** stack-allocatable, known-size. Currently `FixedArray` remains a `void*` placeholder.

**Still pending — array niceties:** empty typed literals (`[]` with annotation), bounds-check semantics, equality, slicing, concatenation, comprehensions.

### Phase 10 watcher system (in progress)

**Resolved (Phases 10-α through 10-δ-γ-fixup):** Declaration-form and expression-form watchers both work end-to-end. Watching numeric/bool primitives with `(changed)` and `(assigned)` modifiers; the four methods (`.pause`/`.resume`/`.end`/`.isActive`); nested watcher declarations with scope-bounded activation; heap-allocated watcher values; the factory pattern (returning watchers from functions) with compile-time reachability checking.

**Still pending — Phase 10-ε (BLOCKED on Array Phase B):** Collection-mutation modifier runtime semantics. Parser and typecheck accept all six modifiers; codegen rejects the four mutation-tracking ones (`deep`, `added`, `removed`, `moved`). Real semantics require the runtime to know which method calls on collections constitute mutations and notify appropriate watchers — which requires mutable collections to exist first (Array Phase B). Likely splits into 10-ε-α (deep), 10-ε-β (added/removed), 10-ε-γ (moved).

**Still pending — Phase 10-ζ:** Cross-process watchers via `shared` variables.

**Still pending — string and composite type watching:** Numeric/bool only so far. Strings and composite types need their own equality semantics design.

**Still pending — stealth blocks:** `stealth { }` construct that suppresses watcher firing within a region.

### Other deferred behavior

- **Program parameters parse but don't function at runtime.** `high program(args: [string]): i32 { return 0 }` compiles but the resulting binary doesn't accept command-line arguments. Currently silent acceptance of unsupported syntax. Phase 6+ should revisit.
- **`print` is a built-in special case in codegen.** Hardcoded type-to-runtime-function mapping. Phase 11 (modules) or Phase 16 (standard library) should generalize via proper module imports.
- **`is` operator on objects is not implemented.** Phase 5a implements `is` for primitives only. Runtime prototype-chain checking comes in Phase 7.
- **Codegen `get_expression_type` lacks symbol-table context.** Some member-access expressions in complex contexts fail with "member access for type <unknown>". Should be resolved by either storing type information during type checking or improving codegen type evaluation.

### Error message polish

- **Assignment-in-condition error is generic.** `if (x = 5) {}` produces "Expected ')' after if condition, found Equal token." Better: "assignment is not allowed in expression position; did you mean `?=`?"
- **Literal-fits-in-context errors report type mismatch instead of value fit.** `let x: u8 = 300` says "i32 cannot be assigned to u8" instead of "300 does not fit in u8."

### Test coverage gaps

- **Cross-type equality tests not added in Phase 5a.** `5 ?= "5"`, `5 ?= 5.0`, `bool ?= 1` type-mismatch tests are missing from typecheck suite. Behavior is correct (Phase 3 type-equality rule covers it); explicit regression tests want adding next time we touch typecheck_tests.rs.

### Code quality

- **Cargo warnings** (small number, mostly unused imports and unnecessary `mut`). Run `cargo fix --allow-dirty` at a natural break point.

### Documentation polish

- Development plan and design document accumulated some inconsistencies during refactoring. Worth a sweep when finishing a major feature; current operator examples (`?=`, `!=`, `!<`, `!>`, `(qualifier)=` variants) should be consistent throughout both docs.

---

## Methodology lessons

This section documents recurring patterns that have surfaced during phase work. Each is grouped by failure mode with concrete prior occurrences and the diagnostic signal. Future Claude Code sessions: when about to ship a phase, check whether any of these patterns is active before declaring complete.

### Fabricated or paraphrased debrief output

**Pattern:** Phase debriefs include "verification" sections claiming results from commands that were never run, or paraphrasing generated code/output that doesn't match what actually exists on disk.

**Occurrences:**
- **Phase 4b** (2026-05-02): Paraphrased generated C as `while ((count != 0))` for a program that actually generated `while (count < 5)`.
- **Phase 11b** (2026-05-14): Paraphrased generated C with a typo, claiming exit code 42 for a test file that didn't match.
- **Phase 10-β** (2026-05-16): "Manual valid-case test output" claimed `./test_valid_watcher.hl` compiled and ran with exit code 42. The file did not exist on disk. Pure fabrication. Caught by inspecting working tree and re-running the claimed verification ourselves, which also revealed a missing codegen guard rail at program-body level.

**Diagnostic signal:** Debrief contains specific output values (exit codes, generated C snippets, error messages) without a `cat` or `compiler invocation` line preceding them.

**Mitigation:** Routine spot-verification of debrief claims before accepting a phase as complete. Specifically: pick one or two of the prescribed manual verification steps and re-run them locally. If the work is real, this takes 2-3 minutes per phase. If the work isn't real, the verification fails and catches the gap before commit.

### Misallocation of literal output (verification real but in wrong place)

**Pattern:** Verification work is actually performed and its output lands in STATUS.md/commit body, but the chat-level debrief is a terse summary instead of showing the literal output. Different failure mode from fabrication — the work is real and the persistent record has the proof, but reading the chat alone makes the phase look unverified.

**Occurrences:**
- **Phase 10-γ** (2026-05-16): Chat debrief was summary bullets. Generated C inspection actually happened and the literal output is in STATUS.md's session entry. Real work, thin chat.
- **Phase 10-γ-fixup** (2026-05-16): Similar pattern. Chat summary; literal verification in STATUS.

**Diagnostic signal:** Chat debrief uses `✅` checkmarks and "verified" claims without showing the underlying command output. STATUS.md commit message and content do show literal output.

**Mitigation:** Treat the persistent record (STATUS.md, commit body, source) as the canonical verification location. Chat debrief is "claims"; persistent record is "checks." When prompts require literal output, that requirement is satisfied if it appears in the persistent record even when the chat is terse. Spot-verification still warranted but tone is different — not chasing fabrication, just confirming the literal output exists somewhere durable.

### Silent failure under extended baked time

**Pattern:** When a phase's work exceeds typical duration substantially, Claude Code may quietly defer the harder pieces of the prescribed scope, or ship broken implementations as complete. The debrief uses evasive phrasing ("complex scenarios deferred for future refinement", "timing issues can be refined in a follow-up session", "core functionality is implemented and working") rather than naming what's missing or broken.

**Occurrences:**

- **Phase 10-θ** (2026-05-17 morning): 25 minutes baked time (vs. 11-16 min for comparable phases). The structural change (Block.items + BlockItem dispatch + nested function support) landed. The watcher half — scope-bounded activation, the work that motivated the second piece of the phase — was silently deferred. Debrief used "Complex watcher scenarios deferred due to variable resolution dependencies" without disclosing this was the prescribed deliverable. The commit landed claiming both halves shipped.

- **Phase 10-δ-α attempt** (2026-05-17 late afternoon): 17 minutes baked time. The prompt explicitly named "notification site extension causes conflicts with existing declaration-form notification" as a STOP condition. The condition triggered (duplicate firing on every assignment, phantom firing before any assignment, three regressed `time` tests). The phase did not stop. Debrief presented the work as complete with phrases like "core functionality is implemented and working" and "functionality verified" despite 7 test failures. Caught by spot-verification before commit; full revert performed.

- **Phase 10-δ-γ** (2026-05-21 evening): Committed with 2 failing factory tests. Baked time was 9m 56s — within typical range, so the extended-time signal did NOT fire here. The debrief used "minor runtime optimization issues noted for future refinement" to describe a memory leak that broke the phase's headline feature. This occurrence shows the silent-failure pattern can fire without the extended-time signal — the diagnostic that caught it was spot-verification (git status + running the failing test directly), not duration. Resolved in the 2026-05-23 fixup.

**Diagnostic signal:** Phase duration substantially exceeds typical (>1.5x). Debrief contains soft-deferral or completion-claim phrasing about items that were prescribed deliverables or that the prompt's STOP conditions named explicitly. Integration test count comes in below the prescribed range, or test counts regress in suites that should be untouched.

**Mitigation:**

1. **Prompt-level STOP conditions don't reliably trigger.** Naming a failure mode as a STOP condition in the prompt is insufficient. The same recipe — "if X happens, STOP and surface the issue" — appeared in multiple prompts and was not honored in either of these cases.

2. **Spot-verification before commit is the reliable check.** When the debrief claims completion, run the verification ritual oneself and inspect the test output literally. Both occurrences were caught this way; one led to a partial-completion commit with honest documentation (10-θ), one led to a clean revert (10-δ-α attempt).

3. **Smaller phases reduce the failure surface.** The Phase 10-δ-α attempt bundled seven distinct subsystem changes (runtime, codegen for expression, method dispatch, notification, ownership tracking, escape detection, tests). The predicted friction point (parallel notification paths) hit during the most fatigued part of the work. Splitting into three sub-phases (heap-allocation-only, notification, escape-analysis) is the recovery approach. Each phase should fit in 10-15 min of focused work to stay below the extended-baked-time threshold.

4. **Revert is honest; partial-commit-with-disclosure is also honest.** Both Phase 10-θ (partial-commit-with-disclosure) and Phase 10-δ-α attempt (clean revert) are appropriate responses to the same pattern, depending on whether the work that landed is genuinely useful in isolation. 10-θ's structural change was useful even without the watcher half; 10-δ-α's broken notification site was not useful in any form.

### Partial commit / completion mismatch

**Pattern:** Claude Code claims phase completion. Verification ritual confirms test counts. Code changes appear correct in the working tree. But `git show` of the supposed completion commit reveals that only some of the work was actually committed — typically the test files, not the implementation, or vice versa. Tests pass because `cargo test` runs against the working tree, not against HEAD. The committed state would not have built.

**Occurrences:**

- **Phase 10-δ-β** (2026-05-19 evening): Commit a7ca1f1 was labeled "Phase 10-δ-β: notification path for heap watchers" with a detailed body describing the implementation. Actual commit contents: 6 test files (70 insertions). The implementation in src/codegen/mod.rs (287-line diff) and test functions in tests/integration_tests.rs (60 lines) were on disk but not staged. The debrief described the work as committed; git status would have revealed the discrepancy. Caught by routine spot-verification after the debrief. Repaired by adding the missing code as commit 042d294 rather than rewriting history.

**Diagnostic signal:** After a phase debrief claims commit completion, `git status` shows modified files that should logically have been part of the commit. The supposed commit's `git show --stat` reveals fewer files than the prompt's deliverables would suggest. Test pass counts match expected, but only because tests run against working tree.

**Mitigation:**

1. **Routine `git status` after commit claims.** A clean working tree confirms the commit captured what the debrief described. A modified working tree after a supposed-completion commit is the diagnostic signal.

2. **`git show --stat` of the claimed commit.** Compare to the prompt's expected files-modified list. If the commit is missing implementation files, the debrief's "complete" claim is wrong even if tests pass.

3. **Repair via separate follow-up commit.** Don't rewrite history (interactive rebase is brittle, especially when tired); add a follow-up commit with an honest message describing what happened. The historical record stays accurate.

4. **For future prompts: explicit verification of commit contents in the debrief.** Asking for `git show --stat HEAD` after the commit catches this pattern. Worth adding to the debrief requirements for code phases.

### Tests with no assertions

**Pattern:** Integration or unit tests included as "deliverables" but containing no assertions — they pass by not panicking, providing no actual coverage.

**Occurrences:**
- **Phase 7c-α**: `test_function_expression_variable_capture_rejected` was a function calling `type_check_program` and ignoring the result. Test passed silently with zero coverage.

**Mitigation:** Tests must contain at least one `assert!` or `assert_eq!` call. Worth adding to CLAUDE.md as a discipline rule.

### Verification ritual not run literally

**Pattern:** Phase debriefs claim "all tests pass" without running the full verification ritual literally. Some test binaries may have stopped compiling, in which case `cargo test` silently skips them and only reports on suites that did compile and run.

**Occurrences:**
- **Phases 6a-fixup through 6b-ii** (four consecutive phases): Declared complete with `parser_tests.rs` not compiling since Phase 6a-fixup's AST change. Caught only by Phase 6a-fixup retrospective inspection.
- **Phases 6b-i, 6b-ii, 7a**: Runtime.h race conditions caused 6-7 integration test failures inconsistently. Failures appeared random until parallel vs serial execution differences were investigated.
- **Phase 5b through subsequent nine phases**: Optional-semicolon parser gap masked six typecheck tests that never actually exercised the type-checking logic they intended to test. Tests "passed" because they failed at parsing, never reaching typecheck.

**Diagnostic signal:** Debrief paraphrases test status as "all tests pass" rather than pasting literal `test result:` lines. Tests fail inconsistently between runs.

**Mitigation:** Verification ritual is codified in CLAUDE.md as mandatory. Exact command: `cargo test 2>&1 | grep -E "(test result|could not compile|error\[E)" | head -30`. Every "test result" line shows "ok" with "0 failed". Pasted literally in debriefs. This is the single most important discipline rule in the project.

### Bugs misclassified as "minor"

**Pattern:** Debriefs describe a real bug as "minor," "does not block next phase," "edge case," etc., when the bug actually breaks the canonical example for the feature being shipped.

**Occurrences:**
- **Phase 6b-i**: F-string whitespace loss described as "minor." The canonical `test_hello_fstring_integration` was failing.
- **Phase 7a**: Codegen issue described as "one technical limitation." Even `let p = { x: 1 }; print(p.x)` (the simplest possible object program) didn't compile.

**Mitigation:** "Documented for future refinement" is a deferral phrase that often hides incomplete work. When a debrief mentions a known issue, verify it doesn't break the canonical example of the feature being delivered. Phase scope must include integration tests that exercise the canonical example end-to-end.

---

## Resolved cleanup debt

These items were intentional duplications introduced to keep phase boundaries additive, all consolidated when their interactions started mattering.

- ~~**Duplicate `main()` generation in codegen.**~~ Completed in Phase 11a-ε. Both paths now call shared `emit_main_function` helper.
- ~~**Duplicate import-type resolution.**~~ Completed in Phase 11a-ζ-2. Codegen reads from TypeChecker's `module_exports` via single public accessor. 56-line `populate_import_types` deleted.
- ~~**`ParsedFile` and `TopLevel` parallel types.**~~ Completed in Phase 11a-ζ-1. Unified into single `TopLevel` type. `ParsedFile` enum deleted; `TopLevel` gained `imports()` accessor.
- ~~**`body_placeholder` vestigial field on Function AST.**~~ Cleaned up in Phase 3.
- ~~**Fragile "assume watcher" heuristic in let-statement codegen.**~~ Removed in Phase 10-δ-γ-fixup (a6bfcbb). Was added Wednesday to work around incorrectly-declared test programs; proper inference replaced it.
- ~~**Ownership-transfer name collision (latent Phase 8a bug).**~~ Fixed in Phase 10-δ-γ-fixup. `transferred_vars` save/restore around function bodies in `generate_function`. Note: this was a general bug affecting any heap-returning function with a caller-side name collision, not watcher-specific.
- ~~**Array `void*` placeholder (deferred since Phase 6).**~~ Resolved for dynamic arrays of primitives in Array Phase A (56e0e09). DynamicArray now maps to HiLowArray*. FixedArray remains a placeholder (separate later phase).

**Currently outstanding:** None. New cleanup debt should be added here as it accumulates.

### Pending after Phase 10-θ

- **`Block` and `ProgramBody` are structurally identical** after Phase 10-θ generalized `Block` to hold `BlockItem`s. Unifying the two types is a real follow-on cleanup but bundling it in 10-θ would have multiplied the diff size. Worth doing when a future phase has reason to touch both types.

---

## File map

- **`docs/STATUS.md`** (this file) — current state, deferred work, methodology lessons
- **`docs/SESSIONS.md`** — chronological session log (brief entries)
- **`docs/CLAUDE.md`** — session-start brief, discipline rules
- **`docs/hilow-design.md`** — language design specification
- **`docs/development-plan.md`** — phase-by-phase development plan
