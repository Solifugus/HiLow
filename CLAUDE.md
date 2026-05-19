# CLAUDE.md

**Current phase: Phase 10-δ-α (take 2) — Heap-Allocated Watcher Values Without Notification**

> Update this line when starting a new phase. The phase listed here governs what work is in scope for the session.

## What this project is

HiLow is a compiled programming language with two modes — **High** for application development, **Low** for systems programming. Both modes share syntax and most semantics; they differ in which features are available and what the compiler enforces.

This is a fresh implementation. The previous compiler is archived (see "Archived code" below) and is reference material only — its decisions do not constrain new work.

## Sources of truth

These two documents are authoritative. When in doubt, consult them in this order:

1. **`docs/hilow-design.md`** — the language specification. Defines syntax, semantics, type system, operators, modes, and standard library. If this document and the implementation disagree, the document wins (or the document needs updating, which is a deliberate decision, not a silent change).

2. **`docs/development-plan.md`** — the phase-by-phase implementation plan. Defines what each phase implements, what it explicitly does *not* implement, and how to verify completion. The current phase governs what work is in scope.

If these two documents disagree with each other, stop and ask — do not pick one silently.

## Project status tracking

`docs/STATUS.md` is the persistent state record. At the start of every session:

1. Read `docs/STATUS.md` to see what was last done, what's in progress, and what open questions exist
2. Resolve any open questions before proceeding (or note them in the prompt if they're unresolved)

At the end of every session:

1. Append a new entry to "Recent sessions" with date, phase, what was actually done, judgment calls, surprises
2. Update "Current state" to reflect where things stand (phase complete? in progress? blocked?)
3. Add to "Open questions" anything that needs user input before next session can proceed
4. Add to "Known issues / TODOs" anything deferred or worth revisiting

Keep entries factual. "Implemented X" is better than "Successfully implemented X to a high standard." If something didn't work and was worked around, say so.

## Verification Ritual (Mandatory)

Every session that modifies code must end by running the verification ritual and pasting its literal output in the debrief.

The verification ritual is this command:

```
cargo test 2>&1 | grep -E "(test result|could not compile|error\[E)" | head -30
```

Expected output: every "test result" line shows "ok" with "0 failed". No "could not compile" lines. No "error[E" lines.

If the output shows any test failure, compilation error, or anything other than "ok ... 0 failed":

- The phase is NOT complete, regardless of how minor the failure seems.
- The session must STOP and report. Do not continue with new work.
- Do not declare success based on "manual testing" or by reasoning that the failures are "out of scope".

### Forbidden Framings

The following phrases are not acceptable in debriefs as justifications for declaring success while tests fail:

- "pre-existing"
- "unrelated to my changes"
- "different issue"
- "not blocking"
- "minor issue"
- "out of scope"
- "this is a separate concern"

If you find yourself wanting to write any of these about a failing test, STOP. Write that observation as a question in the debrief instead. The decision of whether a failure is acceptable to defer is the user's, not yours.

### Why This Exists

A test suite that includes failing tests is broken, period. The presence of "expected failures" or "known broken" tests masks new failures that get introduced. Once the team accepts a baseline of "30 passed, 6 failed", any number from "30 passed, 7 failed" onward stops being noticed.

The only acceptable baselines are:

- All tests pass
- Failing tests are explicitly marked with `#[ignore]` and the reason is documented in STATUS.md "Known issues / TODOs"

A test that fails when run is not in either category. It is broken state that should never have been committed.

### Session Start Procedure

At the start of every session that modifies code:

1. Run the verification ritual.
2. If the baseline is not clean (any failures or compilation errors), STOP. Report the broken state and do not proceed with new work until the user gives direction.
3. If the baseline is clean, proceed with the assigned work.

This catches breakage that may have happened in a previous session that wasn't reported correctly. It also gives the user a known starting point for assessing whether the current session's changes introduced any regressions.

## Canonical Examples Are Integration Tests

Every phase prompt that asks to "show the actual generated C for X" or similar — where X is a specific HiLow program example — implies that X must exist as an integration test. The example program must be:

1. Saved as a `.hl` file in `tests/programs/`
2. Have an expected output file in `tests/expected/`
3. Be exercised by an integration test in `tests/integration_tests.rs` that:
   - Invokes `hilowc` on the program
   - Runs the resulting binary
   - Compares stdout to the expected output file

Unit tests of the parser, type checker, or codegen are not substitutes. A phase is not complete if its canonical examples can compile in unit tests but fail end-to-end.

### Why

Unit tests verify components in isolation. Integration tests verify the components work together. Many bugs live at the seams between components — type information not flowing into codegen, runtime helpers not linked, escape sequences mangling output, etc. These bugs are invisible to unit tests but cause complete failure of canonical examples.

The rule "show the generated C for X" exists in prompts to make Claude Code paste real compiler output. If the example program doesn't exist as an integration test, "showing the generated C" requires inventing the workflow on the spot, and it's easy to paste plausible-looking but inaccurate C. Integration tests prevent this by making the example permanent and verifiable.

### How to Apply

When reading a phase prompt:

1. Identify every canonical example mentioned (programs the prompt asks to show output for, or programs in the verification section).
2. For each one, write a `.hl` file, a `.expected.txt` file, and an integration test function.
3. Run the integration test before declaring the phase complete.
4. The verification ritual must include all integration tests passing.

If a canonical example demonstrates an error path (e.g., "should fail with X message"), the integration test asserts the compilation fails with the expected error.

### Forbidden Patterns

In addition to the previously-forbidden framings ("pre-existing", "unrelated", "minor", "different issue"), these are also not acceptable:

- "Documented for future refinement"
- "Technical limitation"
- "Will be implemented in future phases" — this is acceptable for phase-deferred features, but only when those features are explicitly listed as out of scope in the prompt's "Phase N is explicitly NOT" section. Using it for in-scope features means the phase is incomplete.
- "Core functionality is complete, with one [limitation/issue/exception]" — usually disguises an incomplete feature

The pattern these phrases share: they reframe an incomplete-feature problem as a documentation problem. If a feature listed in the phase scope doesn't work end-to-end, the phase is not complete. No exceptions, no documentation hand-waves.

## Tests Must Contain Assertions

Every test added to the test suite must contain at least one `assert!`, `assert_eq!`, `assert_ne!`, or equivalent assertion. A test that calls functions without asserting on their results is not a test — it's setup code labeled as a test.

### What's Forbidden

```rust
#[test]
fn test_feature_X() {
    let result = do_something();
    // TODO: implement feature X assertion
}
```

```rust
#[test]
fn test_feature_X() {
    let result = do_something();
    // Test passes if no panic occurs
}
```

```rust
#[test]
fn test_feature_X() {
    do_something();
    // Verifies feature X parses (without checking what it parses to)
}
```

These tests exist on paper but don't verify behavior. They satisfy the letter of "tests added" without satisfying the spirit.

### What's Required

Every test must answer the question: "what would have to be wrong for this test to fail?" If the answer is "nothing in the code under test," the test is theater.

```rust
#[test]
fn test_feature_X() {
    let result = do_something();
    assert!(result.is_ok(), "Expected success, got: {:?}", result);
    let value = result.unwrap();
    assert_eq!(value, expected_value);
}
```

### When You're Tempted to Write an Empty Test

If you're about to write a test body without assertions, that's a signal. Stop and ask:

- Is the feature this test is named after actually implemented?
- If not, is the test a placeholder for future work?

If the feature isn't implemented, **the right action is to STOP and report**, not to commit a placeholder test. Phase prompts are explicit about what's in and out of scope. If a test for an in-scope feature can't be written meaningfully, the feature isn't done. Report back rather than committing the test.

### Why This Exists

Empty tests are worse than no tests. They:
- Inflate test counts, suggesting more coverage than exists
- Pass cargo test, suggesting the feature works
- Bury TODO markers in code that's hard to audit
- Are not caught by the verification ritual (they pass)

A test count is only meaningful if every test asserts. The only way to maintain this is to make empty tests forbidden by policy and to grep for them periodically.

### Audit Heuristic

To check the test corpus for empty tests:

```bash
# Heuristic: tests that have no assert statements
for f in tests/*.rs; do
  awk '/#\[test\]/{flag=1; name=""; body=""; line_no=0}
       flag && /^fn /{name=$0; line_no=NR}
       flag{body=body $0 "\n"; if($0 ~ /^}/ && line_no > 0){
         if(body !~ /assert/){print f ":" name}
         flag=0
       }}' "$f"
done
```

This isn't perfect (it might miss assertions through helper functions), but catches the common case.

## Discipline rules

These rules apply to every session. They exist because the alternative produces broken intermediate states, scope creep, and silent shortcuts.

**Stay in the current phase.** The phase listed at the top of this file is the only phase in scope. Do not implement features from later phases, even if convenient or "while I'm here." If something seems to need a feature from a later phase, stop and check the plan — it may be that the dependency is a mistake in the plan (raise it) or that the current phase needs to be redefined (raise that too). Do not silently pull from the future.

**Read "Out of scope" sections in the plan.** Each phase explicitly lists what it does *not* do. These are guardrails against scope drift. Honor them.

**Verify before declaring complete.** A phase is done when:
- All listed verification programs compile without errors
- All listed verification programs produce the *exact* expected output
- The full test suite from previous phases still passes
- The change is committed with a clear message

Producing the right *kind* of output is not enough. The expected output is specified. Match it.

**Never silently work around a problem.** If a verification fails, fix it or revert and re-plan. Do not declare partial completion. Do not leave tests commented out. Do not skip cases that "should work but don't matter for this phase."

**Match the spec exactly.** HiLow has specific design choices that are easy to miss or "fix":
- Equality is `?=` not `==`
- Approximate equality is `~=` not `~~`
- Type test is `is`, not `instanceof` or `typeof`
- There is no type coercion — `"5" + 2` is an error, not "52"
- The entry point is `high program(...)` or `low program(...)`, not `main()`
- Statements are semicolon-optional; do not insist on them
- Tuple destructuring uses parens: `let (a, b) = pair`
- Use `loop { }` for infinite loops, not `for (;;)`

When the spec and your training data disagree, the spec wins.

**Ask before changing the spec or plan.** If you discover something that genuinely needs to change in `hilow-design.md` or `development-plan.md`, raise it — don't edit it unilaterally. The user wants to make those decisions.

## Working pattern for a session

1. Read the "Current phase" line at the top of this file
2. Read the relevant sections of `docs/hilow-design.md` for the features being implemented
3. Read the relevant phase section of `docs/development-plan.md` (current phase only — earlier phases are reference)
4. Implement only what the current phase scopes in
5. Write or update the verification programs from the plan
6. Run them and confirm exact expected output
7. Run the full test suite to verify earlier phases still pass
8. Commit with a message like `Phase N[a/b]: <one-line summary>`
9. If the phase is now complete, update "Current phase" at the top of this file

## Project conventions

**Language**: Rust (compiler), C (codegen target initially; LLVM later in Phase 17)

**Project structure** (as it grows):
```
HiLow/
├── CLAUDE.md                         (this file)
├── Cargo.toml
├── docs/
│   ├── hilow-design.md               (language spec)
│   ├── development-plan.md           (implementation plan)
│   └── old-design/                   (archived previous design)
├── src/
│   ├── main.rs                       (CLI entry point)
│   ├── lexer/
│   ├── parser/
│   ├── ast/
│   ├── types/                        (added in Phase 3)
│   ├── typecheck/                    (added in Phase 3)
│   ├── codegen/                      (added in Phase 4)
│   └── runtime/                      (C runtime support, added in Phase 4)
├── tests/
│   ├── lexer/                        (Rust unit tests for lexer)
│   ├── parser/                       (Rust unit tests for parser)
│   ├── typecheck/                    (Rust unit tests for type checker)
│   ├── programs/                     (HiLow source files for end-to-end tests)
│   └── expected/                     (expected output for each test program)
└── archive/
    └── old-compiler/                 (previous Rust implementation)
```

Do not place files outside this structure without a reason. Do not move files between directories without a reason.

**Commit conventions:**
- One commit per phase or sub-phase, ideally
- Subject: `Phase Na: <summary>` (e.g., `Phase 1a: Basic tokens`)
- Body: brief notes on what was implemented, any deviations from the plan

**Test discipline:**
- New features get tests in the same session they are written
- Tests live alongside the feature: `tests/lexer/equality.rs` for the equality operators
- Verification programs from the plan go in `tests/programs/<phase-name>/` with expected output in `tests/expected/<phase-name>/`

**Error messages:**
- Always include source position (line, column)
- Always describe the problem in HiLow terms, not Rust terms (the user is a HiLow programmer, not a Rust internals reader)
- Where possible, suggest a fix

## Things that are easy to get wrong

**Do not invent operator behavior.** The spec defines exactly what `?=`, `~=`, `is`, and `(qualifier)=` mean. Implement them as specified. If the spec is unclear on an edge case, ask.

**Do not add coercion "for convenience."** `5 + "10"` is an error. So is `5 ?= "5"`. The user has explicitly chosen no coercion as a design principle.

**Do not implement closures that escape Low mode.** Low mode forbids closures that escape their defining scope. This is enforced in Phase 12c. Until then, just don't write tests that exercise this in Low mode.

**Do not auto-add semicolons.** Semicolons are optional in HiLow. Generated test programs should generally omit them; the language is JS-style.

**Do not assume `main()`.** HiLow programs are wrapped in `high program(args: [string]): i32 { ... }` or `low program(...) { ... }`. The keyword is `program`, the mode is part of the declaration.

**Do not produce hidden runtime dependencies.** HiLow has no GC, no async runtime by default, and compiles to standalone native code. Codegen choices that pull in Boehm GC, libuv, or similar are wrong.

**Do not skip the verification step.** "It compiles" is not "it works." The plan specifies what programs to run and what output to produce. Match the output exactly.

## Archived code

This is a fresh implementation from scratch. There is no previous HiLow compiler code to reference. The `archive/` directory will be created as needed for future archival purposes.

## When something is unclear

The user is the project owner and the design authority. When the spec or plan is ambiguous on a real question, ask. Specifically:

- Spec is silent on a behavior: ask, don't guess
- Plan and spec seem to contradict: ask
- A phase seems to need something not yet implemented: ask whether to adjust the plan
- An implementation choice has multiple reasonable paths: list them and ask which to take

Do not silently pick one and proceed. The user prefers a brief consultation over rework.

## Tone and style

- Concise, technical, factual
- Do not pad responses with restating the request or summarizing what was just done
- When something is broken, say so plainly
- When proposing a non-obvious change, give the reasoning
- Code over prose; show the diff or the file rather than describing it in English when both are options
