# Project Status

> This file is the persistent state record for the HiLow compiler project. Claude Code reads this at the start of every session and updates it at the end. The user reads it between sessions.

---

## Current state

**Phase:** Phase 2a — Program/Module Structure and Top-Level
**Status:** Ready to start
**Branch:** main
**Last commit:** Phase 1b: Equality operators and negation comparators

---

## Open questions

*(none currently)*

---

## Recent sessions

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
