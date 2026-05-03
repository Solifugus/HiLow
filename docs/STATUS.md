# Project Status

> This file is the persistent state record for the HiLow compiler project. Claude Code reads this at the start of every session and updates it at the end. The user reads it between sessions.

---

## Current state

**Phase:** Phase 4a — Codegen Foundation and Basic Programs
**Status:** Ready to start
**Branch:** main
**Last commit:** Phase 3: Basic type system and type checker

---

## Open questions

*(none currently)*

---

## Recent sessions

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

*(none currently)*
