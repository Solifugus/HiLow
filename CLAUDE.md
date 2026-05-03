# CLAUDE.md

**Current phase: Phase 2a — Program/Module Structure and Top-Level**

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
