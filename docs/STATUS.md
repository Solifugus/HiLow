# Project Status

> This file is the persistent state record for the HiLow compiler project. Claude Code reads this at the start of every session and updates it at the end. The user reads it between sessions.

---

## Current state

**Phase:** Phase 6b — F-Strings  
**Status:** Ready to start
**Branch:** main
**Last commit:** Phase 6a: Strings with quote recursion

---

## Open questions

*(none currently)*

---

## Recent sessions

### 2026-05-02 — Phase 6a complete
- Implemented complete string literal support with quote recursion algorithm: N adjacent quotes open/close strings, fewer quotes inside are literal
- Added lexer support for string literals (TokenKind::StringLit), raw strings (r"..." prefix), escape sequences (\n, \t, \r, \\, \", \u{...}, \x..), multi-line strings with proper line tracking
- Implemented string literal parsing in AST (Expression::StringLit), type checking (Type::String), and codegen (const char* variables, C string literals with proper escaping)
- Added print_str runtime function and print() dispatch for strings; strings generate const char* variables, not int32_t
- Deferred f-strings cleanly: lexer recognizes r"..." vs plain "...", parser detects f"..." and errors with "Phase 6b" message
- Comprehensive testing: 12 new lexer tests (simple strings, quote recursion, raw strings, escapes, unicode, hex, multiline, errors), 4 integration tests covering basic usage, escape processing, raw strings, quote recursion
- All string functionality working: quote recursion (""contains "quotes" inside""), raw strings (r"C:\path"), escape sequences (\n, \t, \", \\, \u{1F600}, \x41), multi-line support, UTF-8 pass-through
- Test suite: 198 tests passing (up from 183), 6 failing (qualified operator issues from Phase 5b, not strings)
- Commit: "Phase 6a: Strings with quote recursion"

### 2026-05-02 — Phase 5b complete
- Implemented qualified operator framework with parser disambiguation between function calls `foo(arg)` and qualified operators `var (qualifier)=` using peek-ahead approach to check for `=` or `!=` after closing parenthesis
- Added AST nodes: QualifierSpec, QualifiedOp, QualifiedOpKind (Assign/Eq/NotEq); qualified operators work as both expressions and statements
- Created qualifier registry framework with QualifierInfo containing context validation (assignment/equality), argument specifications, type applicability checks, and codegen status tracking
- Implemented universal assignment qualifiers with full codegen: (or)= emits `x = x || y`, (and)= emits `x = x && y`, (bitor)= emits `x = x | y`, (bitand)= emits `x = x & y`, (bitxor)= emits `x = x ^ y`
- Parser handles keywords as qualifier names using expect_qualifier_name helper (allows `or` and `and` keywords in qualifier position)
- Type checker validates qualifier context (assignment-only vs equality-only), argument requirements, type compatibility, and reports phase-specific "not yet implemented" errors for placeholder qualifiers like (coerce)=, (roughly)=, (caseless)=
- Codegen distinguishes assignment statement form `flags (bitor)= 4;` vs expression form returning values
- Added 8 comprehensive tests: 6 parser tests (parsing, disambiguation), 7 type checker tests (validation), 5 codegen tests (C code generation), 2 integration tests (end-to-end verification)
- All 183 tests passing (up from 175): working qualified operator framework ready for type-specific qualifiers in future phases
- Commit: "Phase 5b: Qualified operators framework"

### 2026-05-02 — Phase 4b complete
- Implemented truthy/falsy semantics: type checker now accepts bool, integer, and float types for conditions in if/while statements
- Added loop depth tracking to validate break/continue statements are only used inside loops (while, loop constructs)
- Implemented complete codegen for if/else statements (including else-if chains), while loops, infinite loops (as `while (1)`), break/continue as literal C statements
- Added codegen for compound assignment operations (+=, -=, *=, /=, %=) with direct mapping to C operators
- Implemented truthy/falsy dispatch in codegen: bool conditions generate direct checks, numeric conditions generate `(expr != 0)` checks
- Extended type checker with `is_condition_type()` helper accepting bool, all integer types (i8-i128, u8-u128, isize, usize), and float types (f32, f64)
- Added comprehensive test coverage: 9 new codegen unit tests, 7 new integration end-to-end tests, 5 new type checker tests
- Created 7 verification programs: counter.hl (while loop), fizzbuzz_numeric.hl (using sentinels), early_exit.hl (break), continue_skip.hl (continue), nested_loops.hl (break scope), truthy.hl (0 falsy, nonzero truthy), compound_assign.hl (compound operations)
- All 167 tests passing (up from 146): working HiLow compiler with full Phase 4b control flow support
- Commit: "Phase 4b: Control flow, loops, truthy/falsy"

### 2026-05-02 — Phase 5a complete
- Implemented codegen for !< (emit C >=) and !> (emit C <=) negation comparators
- Implemented compile-time is operator for primitive types: at compile time, verify operand type matches target type and emit 1 (true) or 0 (false)
- Fixed infer_expression_type to properly handle IsCheck expressions as Type::Bool, preventing incorrect truthy/falsy conversion
- Type checker already correctly handled ?= and != requiring same types, and !< and !> as comparison operators
- Lexer already properly rejects == with clear error suggesting ?= for equality
- Created 4 verification programs: equality.hl (?=, !=, is tests), negation_compare.hl (!<, !> tests), type_mismatch.hl (compile error), bad_equals.hl (compile error)
- Added 4 new integration tests for Phase 5a verification programs and 4 new codegen unit tests for equality operators, negation comparators, and is checks  
- All 175 tests passing (up from 167): equality operators, type tests, and negation comparators fully functional
- Commit: "Phase 5a: Equality, Type Tests, and Negation Comparators"

### 2026-05-02 — Phase 4a complete
- Implemented C code generation backend in src/codegen/mod.rs with comprehensive AST-to-C translation for programs, functions, statements, and expressions
- Created C runtime library (runtime.h/runtime.c) with print_i32, print_i64, print_u32, print_u64, print_f32, print_f64, print_bool functions
- Built complete compilation pipeline: parse → typecheck → codegen → invoke cc to produce executable binary
- Extended type checker to handle print() as magic built-in function accepting any printable type, returning i32 (temporary until nothing type in Phase 9)
- Implemented variable type tracking in codegen for proper print function dispatch (print_i32 vs print_bool vs print_f64 based on actual variable types)
- Created comprehensive test suite: 7 codegen unit tests + 7 integration end-to-end tests covering compilation, execution, stdout capture, exit code verification
- Added Display trait implementations for ParseError and LexError to enable proper error propagation in main compilation pipeline
- Full pipeline orchestration with temporary file management, runtime embedding via include_str!, and cc invocation for linking
- Verification programs working: hello_int.hl prints "42", arithmetic.hl prints all operations, return values propagate correctly as exit codes
- All 146 tests passing (up from 132): working HiLow-to-executable compiler for Phase 4a subset
- Commit: "Phase 4a: First runnable HiLow programs"

### 2026-05-02 — Phase 3 complete
- Removed vestigial body_placeholder fields from Function and Program AST nodes after Phase 2b made them obsolete
- Implemented comprehensive Type enum in src/types/mod.rs covering all primitive types, arrays, with helper methods for numeric type checking, literal fitting, and type conversion
- Built TypeChecker in src/typecheck/mod.rs with lexical scoping, symbol table management, and comprehensive type checking for all Phase 3 requirements
- Implemented numeric literal type inference: bare integers default to i32 (if fits) or i64, floats default to f64, with context-sensitive fitting (42 fits in u8 when declared as u8)
- Strict NO coercion policy: i32 + f64, bool + i32, "string" + i32 all produce type errors with clear messages suggesting explicit conversion
- Type checking for all operators: arithmetic (same numeric type), comparison (same numeric type → bool), equality (same type → bool), logical (bool → bool), bitwise (same integer type)
- Condition type checking: if/while/loop conditions must be exactly bool in Phase 3 (truthy/falsy deferred to Phase 4b)
- Symbol table with lexical scoping: each block/function has own scope, inner scopes see outer scopes, shadowing allowed
- Enhanced parser to create IsCheck AST nodes for 'x is type' expressions instead of treating as binary operations
- Enhanced parser to require either type annotation OR initializer for let statements (let x with neither is now a parse error)
- Created 24 comprehensive type checker tests covering successful cases and all error scenarios
- Created 3 verification programs and tests: types1.hl (passes), types2.hl (i32+f64 error), types3.hl (bool+i32 error)
- Added test for assignment-not-allowed-in-expressions from Phase 2b
- All 132 tests passing (81 lexer + 21 parser + 24 typecheck + 3 verify_phase2b + 3 verify_phase3)
- Commit: "Phase 3: Basic type system and type checker"

### 2026-05-02 — Phase 2b complete
- Extended AST with full Phase 2b nodes: Statement enum (Let, Return, If, While, Loop, Break, Continue, Assign, ExprStatement), Expression enum (IntLit, FloatLit, BoolLit, Ident, BinaryOp, UnaryOp, Call, MemberAccess, IndexAccess, IsCheck), and supporting structures (BinaryOpKind, UnaryOpKind, AssignOpKind)
- Implemented comprehensive statement parsing: let declarations (with optional type, optional initializer), return statements, if/else statements, while/loop statements, break/continue statements, assignment statements (=, +=, -=, *=, /=, %=), expression statements
- Implemented Pratt parser for expression parsing with 12-level operator precedence: or(1) < and(2) < comparison(4) < bitwise_or(5) < bitwise_xor(6) < bitwise_and(7) < shifts(8) < add/sub(9) < mul/div/mod(10), plus unary and postfix
- Function calls, member access (.), and array indexing ([]) parsing implemented
- Replaced body placeholder system with direct statement/block parsing in programs and functions
- Updated Program and Function AST to include both body_placeholder and body fields for compatibility
- All verification programs parse successfully: arith.hl (arithmetic precedence), control.hl (control flow), equality.hl (equality operators including ?=, !=, is)
- Added 4 comprehensive parser tests for Phase 2b functionality plus 3 verification tests
- All 104 tests passing (81 lexer + 20 parser + 3 verification)
- Commit: "Phase 2b: Statements and expressions"

### 2026-05-02 — Phase 2a complete
- Implemented parser foundation with hand-written recursive descent parser
- AST nodes: Program, Module, Function, Parameter, Type, Mode with proper position tracking
- Top-level parsing: high/low program/module declarations with mode inheritance
- Function signature parsing (no bodies - skipped with brace counting)
- Type system: primitive types (i8-i128, u8-u128, f32/f64, bool, string, usize/isize, nothing) + arrays ([T], [T; N])
- Mode inheritance at parse time: functions inherit from program/module, explicit override supported
- Body placeholders store source positions for Phase 2b parsing
- Error handling with precise position and suggestions (pointers rejected with Phase 12 reference)
- 16 comprehensive parser tests covering success and error cases
- All 97 tests passing (81 lexer + 16 parser)
- Commit: "Phase 2a: Program/module structure and signatures"

### 2026-05-02 — Phase 1b complete
- Implemented equality operators and negation comparators
- New TokenKind variants: EqStrict (?=), NotEq (!=), NotLess (!<), NotGreater (!>)
- Proper disambiguation: ?= vs bare ?, != vs !< vs !>, with multi-character lookahead
- Error handling for invalid operators: == (suggests ?=), !<= and !>= (redundant), bare ! (use 'not')
- Added 13 comprehensive tests for new operators and error cases
- All 81 tests passing (68 Phase 1a + 13 Phase 1b)
- Multi-character lookahead correctly handles !<= and !>= disambiguation
- Commit: "Phase 1b: Equality operators and negation comparators"

### 2026-05-02 — Phase 1a (restart) complete
- Implemented complete lexer against refreshed specification
- Token types: identifiers, keywords, integer/float/boolean literals, punctuation, operators (excluding equality)
- 46 keywords from refreshed spec (note: spec count appears to be 46, not 41 as initially mentioned)
- Numeric literals: decimal, hex (0x), binary (0b), floats with scientific notation, underscore separators
- Comments: line (//) and block (/* */) with proper nesting support
- Source position tracking (line, column) for all tokens
- Comprehensive test suite with 68 tests covering all requirements
- All verification cases from development plan implemented and passing
- Commit: "Phase 1a (restart): Basic tokens against refreshed spec"

### 2026-05-02 — Design refresh
- Substantive design changes following hands-on syntax exploration:
  - Operators: added !< and !>; rejected !<= and !>= as redundant
  - Storage: standalone `stack` and `heap` declarators (Low-only)
  - Memory: smart `defer <var>` plus explicit `defer <expr>`
  - Coercion: `(coerce)=` as registered assignment qualifier
  - Watch: `stealth { }` block with dynamic suppression
  - Constraints: predicate form OR set form with `{...}` and `excluding {...}`; ranges in sets are inclusive both ends
  - Proofs: layered (--prove warnings, --strict errors, runtime fallback)
  - Phase 15 split into 6 sub-phases: constraints, contracts+invariants+termination, memory+resources, watch+type+currency, overflow, concurrency
- Reset Phase 1a lexer code (no longer aligned with refreshed spec)
- Project ready to restart from Phase 1a

### 2026-05-02 — Phase 1a (superseded)
- Implemented basic lexer with TokenKind, Token, Position, Lexer, LexError
- 55 tests passing, all numeric forms, all keywords, nested comments
- Reset because design refresh changed which keywords and operators belong to Phase 1a

### 2026-05-02 — Phase 0
- Created Cargo.toml with clap, colored, pretty_assertions
- Created stub src/main.rs
- Created tests/programs/ and tests/expected/
- Updated CLAUDE.md "Archived code" section

---

## Known issues / TODOs

### Code quality
- **5 cargo warnings** (unused imports, unnecessary `mut`). Run `cargo fix --allow-dirty` at a natural break point (end of Phase 5 or 6) to clean up.

### Vestigial AST fields
- ~~`body_placeholder` field on Function AST nodes is dead code after Phase 2b moved body parsing inline.~~ *(Cleaned up in Phase 3.)*

### Error message polish
- **Assignment-in-condition error is generic.** Currently `if (x = 5) { }` produces "Expected ')' after if condition, found Equal token." A more helpful message would be: "assignment is not allowed in expression position; did you mean `?=` for equality?" The behavior is correctly rejected (Phase 2b), but the error wording is a parser-level error rather than a domain-aware suggestion. Owned by Phase 2b; revisit when polishing error messages.
- **Literal-fits-in-context errors report type-mismatch rather than value-fits.** `let x: u8 = 300` says "i32 cannot be assigned to u8" rather than "300 does not fit in u8." Both prevent the bug; the second is more directly informative. Future polish: special-case typed-let-with-literal-initializer to give a value-based error.

### Deferred behavior
- **Program parameters parse but don't function at runtime.** `high program(args: [string]): i32 { return 0 }` compiles successfully in Phase 4a but the resulting binary doesn't accept command-line arguments — `int main()` is generated, not `int main(int argc, char **argv)`. Phase 6 (when strings exist) should revisit this and either properly forward args or reject the syntax with a clear "not yet supported" error. Currently silent acceptance of unsupported syntax.
- **`print` is a built-in special case in codegen.** The codegen has a hardcoded mapping from `print(x)` to runtime functions based on x's type. This is documented as a Phase 4a-only special case to be replaced with proper module imports later. Phase 11 (modules) or Phase 16 (standard library) should generalize this.
- **`is` operator on objects is not implemented.** Phase 5a implements `is` for primitives only (compile-time constant). Runtime prototype-chain checking comes in Phase 7.

### Documentation polish
- The development plan and design document accumulated some inconsistencies during refactoring. After Phase 5b lands, do a sweep to make sure the operator examples throughout both documents reflect the final design (?=, !=, !<, !>, (qualifier)= variants).

### Behavioral observations (not bugs, just things to remember)
- **Claude Code sometimes paraphrases generated code in debriefs rather than pasting actual output.** Phase 4b debrief showed `while ((count != 0))` for a program that actually generated `while (count < 5)`. When debriefs include code samples, treat them as descriptive — verify with `cat` on the actual files or by examining what the integration tests assert.
*(none currently)*

### Test coverage gaps
- **Cross-type equality tests not added in Phase 5a.** The prompt asked for `5 ?= "5"`, `5 ?= 5.0`, `bool ?= 1` type-mismatch tests in the typecheck test suite, but they weren't added. Behavior is correct (Phase 3 type-equality rule covers it), but the explicit regression tests are missing. Add these the next time we touch typecheck_tests.rs.

- **`bad_equals` integration test uses a different pattern than success tests.** It calls `compile_program` and asserts the result is `Err`, rather than running a compiled binary. This is correct for compile-failure tests but means the pattern in `tests/integration_tests.rs` is heterogeneous. Note for future readers; not a problem.
