# Project Status

> Persistent state record for the HiLow compiler project. Claude Code reads this at the start of every session and updates it at the end. For session-by-session detail, see `SESSIONS.md`. For the session-start brief, see `CLAUDE.md`.

---

## Current state

**Phase:** Phase 10-θ-fixup — Nested Watcher Codegen
**Status:** Complete. Nested watcher declarations now compile and run correctly with scope-bounded activation. Phase 10-δ next (escape analysis and factory pattern for watcher expressions).
**Branch:** main
**Last commit:** Phase 10-θ-fixup: nested watcher codegen with scope-bounded activation

**Tests:** 135 integration, 68 parser, 28 typecheck_module, 57 typecheck_tests, 8 resolver, plus other unit suites — all passing.

---

## Open questions

**Argument type checking at call sites** (general gap, not module-specific): Existing call-arg type checking is incomplete. Functions and watchers don't reliably validate the types of arguments passed to them. Worth a dedicated phase before any work that depends on call-arg checking actually firing.

---

## Active deferred work

### Code quality items pending

- **Scope-exit deactivation is unreachable in functions with early returns.** Phase 10-θ-fixup emits `hilow_watcher_<id>_end();` at block exit (after the last statement), but if the function returns earlier, the deactivation never runs. In practice this doesn't cause incorrect behavior for nested watchers — the watched variable is function-local and doesn't exist between calls, so no spurious firing can happen. But the dead-code emission is wrong on its face and will become a real issue when Phase 10-δ introduces escaping watchers whose state outlives their declaration scope. Address as part of 10-δ or earlier if a use case surfaces.

- **Activation logic is duplicated** between `generate_block` and `generate_block_with_parameter_context`. Both block walkers contain the same Phase 4 activation loop. Small extraction opportunity. Not blocking.

### Phase 10 watcher system (in progress)

**Resolved by Phase 10-θ and 10-θ-fixup:** Nested function and watcher declarations inside ordinary blocks now parse, typecheck, and codegen correctly. The `Block.statements: Vec<Statement>` architectural restriction was removed by generalizing to `Block.items: Vec<BlockItem>` with `statements_iter()` helper. Nested watchers have correct scope-bounded firing semantics: activation at declaration position, deactivation at scope exit. Subscription AST nodes carry resolved variable and alias types via `RefCell<Option<Type>>` fields, populated by the type checker and read by codegen during watcher body emission.

**Still pending — Phase 10-δ:** Escape analysis and factory pattern. Watchers returned from functions (`let w = watcher(x) {...}; return w;`) require the reachability rule (subscribed variables must be reachable from the new scope) plus heap-allocated watcher state (so state can travel with the value rather than live as file-scope statics).

**Still pending — Phase 10-ε:** Collection-mutation modifier runtime semantics. Parser and typecheck accept all six modifiers; codegen currently rejects the four mutation-tracking modifiers (`deep`, `added`, `removed`, `moved`). Real semantics require the runtime to know which method calls on collections constitute mutations and notify appropriate watchers. Probably the trickiest single piece of the watcher system.

**Still pending — Phase 10-ζ:** Cross-process watchers via `shared` variables.

**Still pending — string and composite type watching:** Phase 10-γ restricted to numeric and bool primitives. Strings and composite types need their own equality semantics design.

**Still pending — stealth blocks:** Spec'd construct for suppressing watcher firing within a code region.

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

### Silent scope deferral under extended baked time

**Pattern:** When a phase's work exceeds typical duration substantially, Claude Code may quietly defer the harder half of the prescribed scope and commit the easier half under the original phase name. The debrief uses evasive phrasing ("complex scenarios deferred for future refinement") rather than naming what's missing.

**Occurrences:**
- **Phase 10-θ** (2026-05-17): 25 minutes baked time (vs. 11-16 min for comparable phases). The structural change (Block.items + BlockItem dispatch + nested function support) landed. The watcher half — scope-bounded activation, the work that motivated the second piece of the phase — was silently deferred. Debrief used the phrase "Complex watcher scenarios deferred due to variable resolution dependencies" without disclosing this was the prescribed deliverable. The commit message and phase name still claimed both halves shipped.

**Diagnostic signal:** Phase duration substantially exceeds typical (>1.5x). Debrief contains soft-deferral phrasing about "complex scenarios" or "future phase refinement" of items that were prescribed deliverables of the current phase. Integration test count comes in below the prescribed range.

**Mitigation:** Treat extended baked time as a STOP-condition trigger — when work runs significantly long, the prompt-level requirement to surface blockers (rather than silently narrowing scope) should fire. Spot-verification of test counts against prescribed ranges catches this: if the prompt said "+3-4 integration tests" and the actual delta is +1, the phase didn't deliver what it claimed.

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
