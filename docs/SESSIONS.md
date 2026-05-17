# Session Log

> Chronological session log for the HiLow compiler project. Brief entries — for full detail, see git log and commit messages. For current state, see `STATUS.md`. For session-start brief, see `CLAUDE.md`.

Most recent first. Each entry: date, phase, commit, headline, key points.

---

### 2026-05-17 (afternoon) — Phase 10-θ-fixup: Nested Watcher Codegen

- **Commit:** 74288ed
- **Tests:** 132 → 135 integration (+3)
- **Summary:** Closed the nested-watcher gap from Phase 10-θ. Added `resolved_var_type` and `resolved_alias_type` RefCells to Subscription AST node, populated by type checker, read by codegen at watcher emission time. This solves the function-local variable type lookup problem at C file-scope emission. Static initializers flipped from `true` to `false`; activation emitted at declaration position (not block entry — caught and corrected mid-implementation). Deactivation emitted at scope exit. Misleading "Phase 10-γ variable resolution" error message removed.
- **Three new integration tests:** watcher-in-function-body-scope-bounded, watcher-in-if-branch-scope-bounded, pre-declaration-assignment-does-not-fire.
- **Issues caught in spot-verification:** (1) Scope-exit deactivation emits after `return` in functions, making it unreachable. Doesn't cause incorrect behavior for current patterns but is dead code — tracked as code-quality item for 10-δ. (2) Activation loop duplicated between two block walkers — small extraction opportunity, not blocking.
- **Baked time:** 18m 9s — closer to typical range (11-16 min) than the 25m of Phase 10-θ. The judgment-call disclosures in the debrief (activation-at-block-entry initially, corrected to declaration-position; type conversion approach; error message replacement) suggest the iteration was honest about what happened.

### 2026-05-17 — Phase 10-θ: Nested Declarations in Blocks (partial)

- **Commit:** 325af86 (also 1e15ae0, db7fc76, d222238)
- **Tests:** 131 → 132 integration, 67 → 68 parser
- **Summary:** Structural change landed (`Block.statements` → `Block.items` with `BlockItem` dispatch); nested function declarations work end-to-end with proper hoisting. The watcher half — scope-bounded activation of watchers declared in nested blocks — did NOT land. Failed at codegen due to variable-type lookup gap: watcher bodies emit at C file scope but need types of function-local subscribed variables. Phase committed under a name that promised both halves.
- **Methodology note:** First instance of "silent scope deferral under extended baked time" pattern. 25 minutes baked vs 11-16 min for comparable phases. Debrief evasive about the missing watcher half. Documented in STATUS.md methodology lessons.
- **Decision:** Keep the structural work; follow up with Phase 10-θ-fixup for the watcher half.

### 2026-05-16 (evening) — Phase 10-γ-fixup: Watcher Methods

- **Commit:** a75777d (also b1c8335 STATUS update, eeed991 paper-cut fix)
- **Tests:** 128 → 131 integration
- **Summary:** Implemented the four watcher methods (`.pause()`, `.resume()`, `.end()`, `.isActive()`) with per-watcher static state. Watcher value methods dispatch via existing MemberAccess pattern (parallel to `time.now()`). Notification site gates on `_active` flag. `.end()` is permanent — `.resume()` cannot revive an ended watcher.
- **Three new integration tests:** pause/resume, end-is-permanent, isActive query.
- **Methodology note:** Chat debrief was summary bullets, literal verification landed in STATUS.md and commit body. Misallocation-of-output pattern, not fabrication.

### 2026-05-16 (afternoon) — Phase 10-γ: Codegen for Watcher Firing

- **Commit:** 274cd1f
- **Tests:** 125 → 128 integration
- **Summary:** Watchers fire on assignment for numeric/bool primitive types with `(changed)` and `(assigned)` modifiers. Watcher bodies emitted as C functions. Assignment notification emits inline: `(changed)` wraps with old-vs-new comparison, `(assigned)` fires unconditionally. Out-of-scope cases (other modifiers, non-primitive types, expression form, compound assignment) all error at codegen with clear phase-naming messages.
- **Three new integration tests:** changed-on-i32, assigned-fires-every, changed-multiple-subscriptions.
- **Methodology note:** Chat debrief thin; literal generated C inspection landed in STATUS.md.

### 2026-05-16 (midday) — Phase 10-β: Type Checking for Watchers

- **Commit:** cf0901c
- **Tests:** 13 → 28 typecheck_module (+15)
- **Summary:** Added `Type::Watcher`; `check_watcher` and `check_watcher_expression` replace Phase 10-α typecheck guard rails. Modifier-variable compatibility enforced: `(deep)` rejects primitives; `(added)`/`(removed)` require collections; `(moved)` requires arrays. Alias types committed: `(added)`/`(removed)` → `[T]`, `(moved)` → `[(usize, usize)]`. No-return-with-value rule enforced.
- **Methodology note:** Debrief contained fabricated "manual verification" claiming exit code 42 for a test file that didn't exist on disk. The missing codegen guard rail at program-body level was also caught during spot-verification. Both fixes applied before commit. Documented in STATUS.md.

### 2026-05-16 (morning) — Phase 10-α: Parser Support for Watcher Syntax

- **Commit:** 2cd119e
- **Tests:** 47 → 67 parser (+20)
- **Summary:** AST additions for watcher declarations and expressions: `SubscriptionModifier` enum, `Subscription` struct, `Watcher` struct, `WatcherExpr` struct, `BlockItem::Watcher` variant, `Module.watchers` field, `Expression::WatcherExpr` variant. Lexer rename `Watch` → `Watcher`. Parser additions for declaration form, expression form, subscription modifier syntax (bare, `(modifier)x`, `(alias=modifier)x`). Type checker and codegen guard rails error clearly for any watcher node.
- **In-session bug fix:** module-body dispatch was parsing watchers but not assigning them to `Module.watchers`.

### 2026-05-15 — Phase 11b-fixup: Duplicated Epilogue + Cross-Module Init Rule

- **Commit:** d5d5fee
- **Tests:** 11 → 13 typecheck_module
- **Summary:** Item A — duplicated leak-check epilogue removed via `main_explicitly_returned` flag (issue predated module work, since Phase 9b, but was harmless because unreachable). Item B — cross-module init rule enforced: exported `let` initializers cannot call functions from other modules.

### 2026-05-14 — Phase 11b: Cyclic Module Graphs

- **Commit:** 92a5f80
- **Tests:** 122 → 125 integration
- **Summary:** Module system end-to-end including cycles. Resolver appends cycle members to topo order alphabetically. CodeGenerator gains `forward_declarations` field for cross-module forward declarations. Three new cycle integration tests (two-cycle even/odd, three-cycle a→b→c→a, iseven/isodd variant).
- **Cleanup debt completed:** all three items from Phase 11a accumulated cleanup debt resolved across Phase 11a-ε (item 1), 11a-ζ-1 (item 3), 11a-ζ-2 (item 2). Net code reduction -154 lines.

### 2026-05-14 — Phase 11a-ζ-2: Consolidate Import-Type Resolution

- **Commit:** (in series)
- **Summary:** Codegen reads from TypeChecker's `module_exports` via new public accessor. Deleted 56-line `populate_import_types` function; replaced with 8-line inline loop. Pure refactor, no behavior change.

### 2026-05-14 — Phase 11a-ζ-1: Unify `ParsedFile` and `TopLevel`

- **Commit:** (in series)
- **Summary:** `ParsedFile` enum deleted from resolver; `TopLevel` gained `imports()` accessor. ~35 mechanical reference-site substitutions across resolver, codegen, typecheck, main.rs, tests. Pure refactor.

### 2026-05-14 — Phase 11a-δ-β: Multi-Module Graphs End-to-End

- **Commit:** (in series)
- **Tests:** 120 → 122 integration
- **Summary:** Removed 2-node boundary check in `compile_graph`. Added chain (app→middle→leaf produces 14) and diamond (app imports a+b; both import util; produces 13) integration tests. Generated C confirmed: shared dependencies not duplicated.

### 2026-05-14 — Phase 11a-ε: Consolidate Duplicate `main()` Emission

- **Commit:** (in series)
- **Summary:** Extracted shared `emit_main_function` helper. Both single-file and module-graph paths now call it. Net -45 lines.

### 2026-05-13 — Phase 9f: Call-Site Argument Type Checking

- **Commit:** (in series)
- **Tests:** 51 → 57 typecheck (+6)
- **Summary:** Type checker now validates argument types match parameter types for all function calls. Used `check_expression_with_expected_type` against parameter types in `check_call`. Format: "Type mismatch in argument N: expected TYPE but got TYPE". Resolved Phase 7c-β TODO. Gap had existed since at least Phase 6.

### 2026-05-12 — Phase 11a-γ: Two-Pass Type Checking for Module Graphs

- **Commit:** fb522ab (amended from over-claiming b04ea4f)
- **Tests:** 11 module typecheck
- **Summary:** New `TypeChecker::check_graph` method. Two-pass implementation: `collect_module_exports` builds ExportTable per module; `check_module_bodies` populates imports into scope.
- **Methodology note:** First attempt over-claimed completion with one failing test. Reverted to 11a-β, diagnosed: the failing test was based on a faulty premise (HiLow's existing type checker didn't validate argument types at all). Replaced the test with a symmetry-asserting version; recovered. Documented the recipe held for the additive parts; the rationalize-and-ship path was cheaper than diagnose-and-stop when an unexpected interaction surfaced. Worth splitting phases of this size further.

### 2026-05-11 — Phase 11a-β: Module Resolver

- **Commit:** (in series)
- **Tests:** 8 new resolver tests
- **Summary:** Pure functional resolver in `src/resolver/mod.rs`. Takes file-lookup callback (no filesystem deps). DFS for dependency loading, Kahn's algorithm for topological sort. Error types: SelfImport, Cycle, ModuleNotFound. Additive — zero changes to existing files.

### 2026-05-10 — Phase 9e: Tuples Complete

- **Commit:** (Phase 9 complete)
- **Tests:** 118 integration
- **Summary:** Per-tuple-type C structs (`HiLowTuple_i32_string`). Tuple literals as struct initializers. Field access via `._N`. Destructuring via temporary + field extraction. Print and f-string support per tuple type. 6 integration tests including arity-mismatch rejection.

### 2026-05-10 — Phase 9d: Money Type with Currencies

- **Commit:** (in series)
- **Summary:** Money type with currency tags. Same-currency arithmetic enforced; mismatch produces clear error. Type checker preserves specific currency info (`MoneyOf("USD")`) in symbol table. Codegen handles money binary operations with proper type inference. Phase landed after two fixes for type inference and mismatch detection.

### 2026-05-10 — Phase 9c: Time Type Complete (after multi-variable narrowing fix)

- **Commit:** (in series)
- **Summary:** `time` and `duration` types with nanosecond precision. Duration literals (`2h`, `30m`, `100ms`). Arithmetic: `time + duration → time`, `time - time → duration`. Precision-aware comparison. ISO 8601 formatting. Multi-variable narrowing fix: split `exit_scope` (preserve refinements) from `exit_function_scope` (clear refinements). Refinements now accumulate across sequential narrowing blocks within a function.

### 2026-05-10 — Phase 9b: Unknown Type Complete

- **Commit:** (in series with three fixes)
- **Summary:** `unknown(reason)` as first-class error value. HiLowUnknown struct with heap-tracked reason string. Type narrowing via `is unknown` checks. Optional type integration (`T?` auto-promotes from `unknown`). Required three iterative fixes for HiLowOptional cleanup, f-string reason access, and unknown-typed local heap tracking.

### 2026-05-09 — Phase 9a: Nothing Type Complete

- **Commit:** (in series)
- **Summary:** `nothing` type as first-class absence value. Singleton runtime representation (`the_nothing`). Missing property access returns nothing instead of erroring (breaking change from Phase 7a). Uninitialized `let` bindings now valid. `is nothing` checks via pointer comparison. Falsy in all boolean contexts.

### 2026-05-09 — Phase 8c: Weak References Complete; Phase 8 Complete

- **Commit:** (in series)
- **Summary:** Weak references break refcount cycles. `weak EXPR` syntax. HiLowObject extended with weak_refs linked list. Memory ordering: weak properties unregistered first, strong properties released normally, weak_refs invalidated to NULL, then object freed. Manual cycle breaking for High mode; `manual` and `defer` deferred to Phase 12 (Low mode).

### 2026-05-09 — Phase 8b: Refcounting Complete

- **Commit:** (in series)
- **Tests:** 89 integration (78 + 11 previously ignored)
- **Summary:** Refcount field added to HiLowObject and HiLowFunction. retain/release functions. Multi-owner scenarios now work: variable aliasing, function in object, escaping closure, object property with heap value. Removed four compile-time rejection points.

### 2026-05-09 — Phase 8a Fix: Multi-Owner Rejection + Leak Coverage

- **Commit:** (in series)
- **Summary:** Two gaps closed. Compile-time rejection now actually detects multi-owner cases (was missing despite being declared complete). Leak detector now runs after main's cleanup (was being bypassed by early-return code paths).

### 2026-05-09 — Phase 8a: Scope-Based Memory Cleanup

- **Commit:** (in series)
- **Summary:** Debug allocator (hl_alloc_count, hl_free_count). Ownership tracking in codegen. Auto-free at scope boundaries. Ownership transfer for return values. Multi-ownership detection with Phase 8b deferral messages. 11 tests marked ignored for Phase 8b.

### 2026-05-08 — Phase 7c-θ: Switch Statements Complete; Phase 7 Complete

- **Commit:** (in series)
- **Tests:** 78 integration
- **Summary:** C-style switch with explicit fallthrough for integer/boolean, if/else chain with strcmp for strings (no fallthrough). Break statement validation extended to allow break in switch as well as loops.

### 2026-05-08 — Phase 7c-η: Match Expressions

- **Commit:** (in series)
- **Tests:** 72 integration
- **Summary:** `match expr { pattern => body, _ => default }` syntax. Literal and wildcard patterns. Exhaustiveness checking for expression context (statement context is non-exhaustive). Both expression and block bodies.

### 2026-05-08 — Phase 7c-ζ: For-In Iteration

- **Commit:** (in series)
- **Tests:** 65 integration
- **Summary:** `for (let (key, value) in obj) { body }` iterates object own properties. `Type::ObjectIterValue` for polymorphic iteration values. Runtime dispatch on `__v_type` tag for print/f-string operations. Prototype properties excluded from iteration.

### 2026-05-08 — Phase 7c-ε: Method `this` Binding

- **Commit:** (in series)
- **Tests:** 60 integration
- **Summary:** Functions called via dot notation receive `this` as the calling object. Method signature is `(void* env, HiLowObject* this_obj, args...)`. `this` correctly binds to receiver, not prototype, when methods are inherited.

### 2026-05-08 — Phase 7c-δ Fix: Closure Codegen Bugs

- **Commit:** (two fixes)
- **Tests:** 55 integration (all closure tests now pass)
- **Summary:** Two bugs caught after phase declared complete. Bug 1: captured parameters not copied to environment after allocation. Bug 2: captured variable types not propagating to closure body codegen. Both fixed.

### 2026-05-08 — Phase 7c-δ: Closures with Variable Capture

- **Commit:** (in series)
- **Summary:** Heap-allocated environments for captured variables. All function expressions take `void* env` as first parameter. Variable hoisting from stack to env struct. References rewritten to env-> access. Initial implementation had two codegen bugs requiring fixup.

### 2026-05-08 — Phase 7c-δ Fix: Parameterized Function Type Syntax

- **Commit:** (in series)
- **Summary:** `function(param_types): return_type` syntax. Was needed before closures could land properly — bare `function` type was too coarse.

### 2026-05-07 — Phase 7c-γ: Capture Detection Metadata

- **Commit:** (in series)
- **Summary:** `FunctionExpr` gained `captures: RefCell<Vec<(String, Type, Position)>>` field. Capture detection algorithm walks function body AST identifying outer-scope references. Capture rejection error now lists specific captured variables with types and positions.

### 2026-05-07 — Phase 7c-α Completion Fix + Empty Test Prohibition

- **Commit:** (two commits)
- **Summary:** `test_function_expression_variable_capture_rejected` was declared a deliverable of Phase 7c-α but had no assertions — it called `type_check_program` and ignored the result. Implementation of real capture rejection in type checker. New CLAUDE.md section "Tests Must Contain Assertions" codifying the rule that tests must have at least one assert call.

### 2026-05-07 — Phase 7c-β: Function Expression Codegen (No Capture)

- **Commit:** (in series)
- **Summary:** Function expressions generate unique top-level C functions (`hilow_anon_N`). `HiLowFunction` struct with function pointer + env (NULL for non-closures). Object properties can store/retrieve function values. C keyword conflict resolution (e.g., `double` → `hl_double`).

### 2026-05-03 — Phase 7a Completion Fix + Canonical Examples Rule

- **Commit:** (two commits)
- **Summary:** Phase 7a was declared complete but `let p = { x: 1 }; print(p.x)` didn't compile. Codegen's `get_expression_type` lacked symbol table context. Fixed with `infer_expression_type_for_codegen` using codegen's own variable_types tracking. New CLAUDE.md section "Canonical Examples Are Integration Tests" codifying the rule that canonical examples in prompts must exist as integration tests. Extended Forbidden Patterns to include "documented for future refinement," "technical limitation," "core functionality complete with one [exception]."

### 2026-05-03 — Phase 7b: Prototype Delegation

- **Commit:** (in series)
- **Summary:** Objects can have a `proto` property acting as prototype. `hl_object_get_*` walks prototype chain. Property assignment always sets on immediate object (JavaScript semantics). Cycle detection with depth 100.

### 2026-05-03 — Phase 7b-extension: `is` Operator for Objects

- **Commit:** (in series)
- **Tests:** 45 integration
- **Summary:** `hl_object_is(child, parent)` walks prototype chain. Distinct AST node `ObjectIsCheck` separates compile-time-evaluated `is` for primitives from runtime-evaluated for objects.

### 2026-05-03 — Phase 7a: Object Literals and Property Access

- **Commit:** (in series)
- **Summary:** Object literal syntax `{ x: 10, y: 20 }`. Property access via dot notation. Property assignment. Structural typing. C hash table runtime. Parser disambiguates object literals vs blocks by context.

### 2026-05-03 — Verification Ritual Documentation + Qualifier Context Fix

- **Commit:** (multiple)
- **Summary:** Verification ritual codified in CLAUDE.md as mandatory. Exact command specified. Forbidden framings listed. Qualifier validation order bug fixed: context check now precedes type check (was producing misleading errors). Clean baseline achieved (230 tests passing) for first time after accumulated stale failures.

### 2026-05-03 — Integration Tests Race Condition Fix

- **Commit:** (in series)
- **Summary:** Race condition in `/tmp/runtime.h` fixed by using per-process unique temp directories. 33 integration tests now pass consistently (was 26 passing inconsistently). Stale format_spec test converted from failure to success assertion to match Phase 6b-ii behavior.

### 2026-05-03 — Parser Tests Fix (Critical)

- **Commit:** (in series)
- **Summary:** `parser_tests.rs` hadn't compiled since Phase 6a-fixup's AST field change from `body.statements` to `body.items`. Four consecutive phases (6a-fixup, 6b-i, 6b-i bugfix, 6b-ii) had declared complete with broken parser tests because `cargo test` silently skips test binaries that fail to compile. 21 test references updated. Methodology lesson banked.

### 2026-05-03 — Phase 6b-ii: F-String Format Specifiers

- **Commit:** (in series)
- **Summary:** Format specs after `:` parse as `[fill align] [width] ['.' precision] [type]`. Type checker validates format/type compatibility. Codegen maps to printf format strings. Binary formatting via `hl_format_binary`, center alignment via `hl_format_center`.

### 2026-05-03 — Phase 6b-i Bugfix: F-String Whitespace Preservation

- **Commit:** (in series)
- **Summary:** Lexer was eating whitespace after closing `}` in f-string expressions. Fixed by skipping whitespace only when not in f-string text mode. Phase 6b-i debrief had described this as "minor" without verifying the canonical test passed.

### 2026-05-03 — Phase 6b-i: F-Strings

- **Commit:** (in series)
- **Summary:** Lexer emits f-string token sequence (Start/Text/ExprStart/ExprEnd/End). Parser assembles `FString` AST node. Codegen uses malloc'd buffer with snprintf chain. Format spec deferral working.

### 2026-05-02 — Phase 6a-fixup

- **Commit:** (in series)
- **Tests:** 27 integration
- **Summary:** UTF-8 codegen fix (raw bytes instead of hex escapes). Nested function definitions: AST extended with ProgramBody/BlockItem for mixed statements/functions. Function name mangling for C keyword conflicts.

### 2026-05-02 — Phase 6a: Strings

- **Commit:** (in series)
- **Summary:** Quote recursion algorithm for string literals (N adjacent quotes open/close, fewer inside are literal). Raw strings (`r"..."`). Escape sequences (`\n`, `\t`, `\u{...}`, `\x..`). Multi-line strings with line tracking. UTF-8 pass-through.

### 2026-05-02 — Phase 5b: Qualified Operators Framework

- **Commit:** (in series)
- **Summary:** Parser disambiguates function calls from qualified operators via peek-ahead. AST: `QualifierSpec`, `QualifiedOp`, `QualifiedOpKind`. Universal assignment qualifiers with codegen: `(or)=`, `(and)=`, `(bitor)=`, `(bitand)=`, `(bitxor)=`. Framework ready for type-specific qualifiers in future phases.

### 2026-05-02 — Phase 5a: Equality, Type Tests, Negation Comparators

- **Commit:** (in series)
- **Summary:** Codegen for `!<` (C `>=`) and `!>` (C `<=`). Compile-time `is` for primitive types. `IsCheck` expressions return Type::Bool. Lexer rejects `==` with clear "use `?=`" message.

### 2026-05-02 — Phase 4b: Control Flow, Loops, Truthy/Falsy

- **Commit:** (in series)
- **Summary:** Truthy/falsy semantics for if/while conditions (bool, integer, float). Loop depth tracking for break/continue validation. Codegen for compound assignment (+=, -=, *=, /=, %=).
- **Methodology note:** Debrief paraphrased generated C as `while ((count != 0))` for a program that actually generated `while (count < 5)`. First documented occurrence of paraphrased debrief output pattern.

### 2026-05-02 — Phase 4a: First Runnable HiLow Programs

- **Commit:** (in series)
- **Summary:** C codegen backend with full AST-to-C translation. C runtime library (print_i32, print_bool, etc.). Compilation pipeline: parse → typecheck → codegen → cc invocation. Temporary file management. `print()` as built-in special case. `hello_int.hl` prints "42"; arithmetic.hl prints all operations.

### 2026-05-02 — Phase 3: Type System and Type Checker

- **Commit:** (in series)
- **Summary:** Type enum with all primitive types + arrays. TypeChecker with lexical scoping. Numeric literal type inference (bare int → i32 or i64, bare float → f64). Strict no-coercion policy. Type-aware operator checking. Let statements require type annotation OR initializer.

### 2026-05-02 — Phase 2b: Statements and Expressions

- **Commit:** (in series)
- **Summary:** Full AST: Statement enum, Expression enum, supporting structures. Pratt parser with 12-level precedence. Let, return, if/else, while, loop, break/continue, assignments (=, +=, -=, *=, /=, %=). Function calls, member access, array indexing.

### 2026-05-02 — Phase 2a: Program/Module Structure and Signatures

- **Commit:** (in series)
- **Summary:** Hand-written recursive descent parser. Top-level parsing (high/low program/module). Function signature parsing (bodies skipped with brace counting). Type system: primitive types + arrays. Mode inheritance at parse time. Body placeholders for Phase 2b.

### 2026-05-02 — Phase 1b: Equality Operators and Negation Comparators

- **Commit:** (in series)
- **Summary:** TokenKind variants: EqStrict (?=), NotEq (!=), NotLess (!<), NotGreater (!>). Disambiguation: ?= vs ?, != vs !< vs !>, with multi-character lookahead. Error handling: == suggests ?=, !<= and !>= rejected as redundant.

### 2026-05-02 — Phase 1a (restart): Basic Tokens

- **Commit:** (in series)
- **Summary:** Complete lexer against refreshed spec. 46 keywords. Numeric literals: decimal, hex (0x), binary (0b), floats with scientific notation, underscore separators. Comments: line and block with nesting. Position tracking.

### 2026-05-02 — Design Refresh

- **Summary:** Substantive design changes after hands-on syntax exploration. Operators: added `!<` and `!>`; rejected `!<=` and `!>=`. Standalone `stack` and `heap` declarators (Low-only). Smart `defer <var>` plus explicit `defer <expr>`. `(coerce)=` as registered assignment qualifier. Watch: `stealth { }` block. Constraints: predicate or set form. Proofs: layered (--prove warnings, --strict errors, runtime fallback). Phase 15 split into 6 sub-phases. Reset Phase 1a code; project restarted from Phase 1a.

### 2026-05-02 — Phase 0: Project Setup

- **Summary:** `Cargo.toml` with clap, colored, pretty_assertions. Stub `src/main.rs`. `tests/programs/` and `tests/expected/` directories. CLAUDE.md "Archived code" section.
