# HiLow Development Plan

This plan rebuilds HiLow from scratch against the new design specification (`hilow-design.md`). The previous implementation is archived for reference but does not constrain new development.

## Approach

**Fresh start, fork-style.** A new compiler binary is built in a new project directory. The old compiler remains accessible (in `docs/old-design/` for the design, archived elsewhere for the code) as a reference for tokenization patterns, AST structures, and codegen approaches that proved out — but no code is carried forward without deliberate review.

**Phase-by-phase execution.** Each phase is sized to fit in a single focused Claude Code session. Phases that would exceed one session are split into sub-phases (e.g., 5a, 5b). Each phase has explicit scope, explicit out-of-scope, granular tasks, and concrete verification criteria.

**Working compiler at every phase.** After each phase completes, the compiler builds and passes that phase's verification programs. No phase produces a "broken intermediate state."

## Working Pattern (for Claude Code sessions)

Each session follows the same pattern:

1. Read `docs/hilow-design.md` for the relevant spec sections (listed per phase below)
2. Read this plan, focused on the current phase only
3. Read `CLAUDE.md` for project conventions
4. Implement only what the current phase scopes in
5. Write the verification programs and confirm they produce expected output
6. Commit with a clear message: `Phase N: <one-line summary>`

**Discipline rules for Claude Code:**

- Do not implement features from later phases, even if convenient
- Do not refactor code from earlier phases unless the phase explicitly says to
- If a feature seems to need something not yet implemented, stop and check the plan — it may be a later phase
- Verification programs must produce the exact expected output before declaring the phase complete
- If scope creep happens, revert to the phase boundary and stay focused

## Master Phase List

| Phase | Title | Sessions |
|-------|-------|----------|
| 0 | Project setup and archive | 1 |
| 1 | Lexer foundation | 2 (1a, 1b) |
| 2 | Parser foundation | 2 (2a, 2b) |
| 3 | AST and basic types | 1 |
| 4 | High mode core: let, functions, control flow | 2 (4a, 4b) |
| 5 | Equality and comparison operators | 2 (5a, 5b) |
| 6 | Strings and f-strings | 3 (6a, 6b, 6c) |
| 7 | Flexible objects and closures | 3 (7a, 7b, 7c) |
| 8 | Memory model | 3 (8a, 8b, 8c) |
| 9 | First-class types | 4 (9a, 9b, 9c, 9d) |
| 10 | Watch system | 2 (10a, 10b) |
| 11 | Modules and imports | 1 |
| 12 | Low mode features | 4 (12a, 12b, 12c, 12d) |
| 13 | Inline assembly | 1 |
| 14 | Mode boundary enforcement | 1 |
| 15 | Formal verification | 6 (15a, 15b, 15c, 15d, 15e, 15f) |
| 16 | Standard library | 3 (16a, 16b, 16c) |
| 17 | Polish and cross-compilation | 2 |

**Total: ~40 sessions.** Each session is bounded; no session attempts more than one phase or sub-phase.

---

## Phase 0: Project Setup and Archive

**Goal:** Create a fresh Rust project for the new HiLow compiler, archive the existing code, and establish working conventions.

**Scope:**
- Archive the existing `src/` to a separate location (or branch) for reference
- Create fresh `Cargo.toml` and `src/main.rs` for the new compiler
- Set up `CLAUDE.md` with new project conventions
- Create a `tests/` directory structure
- Verify `cargo build` succeeds with a stub `hilowc` binary

**Out of scope:**
- Any actual lexer, parser, or compiler logic
- Carrying over any old source code

**Tasks:**
1. Create `archive/old-compiler/` directory; move existing `src/`, `Cargo.toml`, `Cargo.lock`, `examples/`, `illustrations/`, `target/`, `tests/`, `test-examples.sh` into it
2. Create fresh `Cargo.toml` with dependencies: `clap` (CLI), `colored` (output), `pretty_assertions` (testing)
3. Create fresh `src/main.rs` with a stub CLI: `hilowc <file>` prints "HiLow compiler v0.1 (stub)"
4. Create `CLAUDE.md` with: design doc location, plan location, working pattern, current phase tracking
5. Create `tests/` directory with `tests/programs/` for `.hl` test files and `tests/expected/` for expected output
6. Verify: `cargo build` succeeds, `./target/debug/hilowc test.hl` prints the stub message

**Verification:**
```bash
cargo build
./target/debug/hilowc somefile.hl
# Should print: "HiLow compiler v0.1 (stub)"
```

**CLAUDE.md should include:**
- Pointer to `docs/hilow-design.md` as the source of truth for language semantics
- Pointer to `docs/development-plan.md` (this file)
- Reminder: only implement features in the current phase
- Reminder: no implementation that violates the design spec
- Current phase tracking: a single line at the top updated each phase ("Current phase: 0 - Project Setup")

---

## Phase 1: Lexer Foundation

**Goal:** Tokenize HiLow source into a stream of tokens. Cover all simple tokens; defer string handling and f-strings.

### Phase 1a: Basic Tokens

**Scope:**
- Token type enum covering: identifiers, keywords, integer literals, float literals, boolean literals, single-character punctuation, multi-character operators (excluding equality)
- Whitespace and comment handling (// and /* */)
- Source position tracking (line, column) for error messages
- Lexer iterator API: `Lexer::new(source).tokens()` returning `Vec<Token>` or `Result<Vec<Token>, LexError>`

**Out of scope:**
- String literals (any kind) — Phase 6
- F-strings — Phase 6
- Equality operators (`?=`, `!=`) — Phase 1b
- Type-test operator `is` — Phase 5
- `(qualifier)=` operators — Phase 5
- Mode keywords (`high`, `low`, `program`, `module`) — Phase 2 (parser context)

**Tasks:**
1. Create `src/lexer/mod.rs` with `Token`, `TokenKind`, `Lexer` types
2. Define `TokenKind` enum with variants for: integer, float, true, false, identifier, all keywords from the design spec (except equality-related), punctuation (`(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`, `.`, `?`, `@`)
3. Implement number lexing: integers (decimal, hex `0x`, binary `0b`), floats with `e` notation
4. Implement identifier and keyword lexing (keyword table lookup after identifier scan)
5. Implement operator lexing for: `+`, `-`, `*`, `/`, `%`, `<`, `>`, `<=`, `>=`, `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&`, `|`, `^`, `~`, `<<`, `>>`, `..`
6. Implement comment skipping (`//` to end of line, `/* */` with nesting support)
7. Track line and column for each token

**Verification:**
Create `tests/lexer/basic_tokens.rs` with unit tests covering:
- Integer literals: `42`, `0x1F`, `0b1010`, `1_000`
- Float literals: `3.14`, `2.5e10`, `1.5e-3`
- Identifiers: `foo`, `_bar`, `camelCase`, `snake_case`
- Keywords: `let`, `function`, `if`, `else`, `for`, `while`, `return`, `break`, `continue`, `loop`, `match`, `switch`, `case`, `default`, `import`, `export`, `from`, `defer`, `async`, `watch`, `shared`, `manual`, `arena`, `nothing`, `unknown`, `true`, `false`, `this`, `not`, `and`, `or`, `requires`, `ensures`, `when`, `in`, `is`
- Operators: all arithmetic, bitwise, comparison (except equality)
- Comments: line and block, including nested

```bash
cargo test --lib lexer::basic_tokens
# All tests pass
```

### Phase 1b: Equality Operators, Negation Comparators, and `?` Disambiguation

**Scope:**
- Equality operator `?=`
- Inequality operator `!=`
- Negation comparators `!<` and `!>`
- The `is` keyword (already in keyword table from 1a; verify it tokenizes correctly)
- Additional keywords now needed: `stealth`, `excluding`, `invariant`, `decreases`, `from` (for `import ... from`)
- Disambiguation: `?=` vs bare `?` (the `?` alone is used in `T?` type syntax — Phase 9 — but the lexer must emit it correctly now)
- Disambiguation: `!=`, `!<`, `!>` (all start with `!`, lookahead determines which)
- Lexer also continues to treat `(` and `)` as separate tokens; the `(qualifier)=` form is recognized later by the parser in Phase 5b

**Out of scope:**
- Approximate equality `~=` — does not exist in HiLow (use `(roughly)=` instead, implemented as a qualifier in Phase 5b)
- `~!=` — does not exist either
- `!<=` and `!>=` — explicitly rejected as redundant with `>` and `<`; lexer should emit clear error
- Bare `==` — must be rejected with a clear error message ("use `?=` for equality, or `=` for assignment")
- Parsing `(qualifier)=` — parser's job in Phase 5b
- Validating qualifier names — type checker's job
- Bare `!` as logical NOT operator — HiLow uses the `not` keyword for that (already in Phase 1a's keyword table). Bare `!` followed by anything other than `=`, `<`, or `>` should be a lex error.

**Tasks:**
1. Add `TokenKind` variants: `EqStrict` (`?=`), `NotEq` (`!=`), `NotLess` (`!<`), `NotGreater` (`!>`)
2. Implement lexing for `?=` (look for `?` followed by `=`)
3. Disambiguate `?` alone (used later for `T?` type syntax) from `?=` — emit `Question` token when `?` is not followed by `=`
4. Implement lexing for `!=`, `!<`, `!>` (look for `!` followed by `=`, `<`, or `>`)
5. Reject `!<=` and `!>=` with a clear error: "`!<=` is redundant; use `>` instead" and "`!>=` is redundant; use `<` instead"
6. Reject bare `==` in the lexer with a clear error: "`==` is not a valid operator in HiLow; use `?=` for equality or `=` for assignment"
7. Reject bare `!` (followed by something other than `=`, `<`, `>`) with: "`!` is not a valid operator in HiLow; use the `not` keyword for logical negation"
8. Add the additional keywords (`stealth`, `excluding`, `invariant`, `decreases`, `from`) to the keyword table
9. Verify `~` continues to lex as bitwise NOT (unchanged from Phase 1a; `~=` is not an operator)

**Verification:**
Add tests to `tests/lexer/equality.rs`:
- `x ?= y` lexes as `[ident, eq_strict, ident]`
- `x != y` lexes as `[ident, not_eq, ident]`
- `x !< y` lexes as `[ident, not_less, ident]`
- `x !> y` lexes as `[ident, not_greater, ident]`
- `x ? y` lexes as `[ident, question, ident]` (the `?` alone is for `T?` types — Phase 9)
- `x <= y` and `x >= y` lex as before (regression test that 1a behavior unchanged)
- `x == y` produces a `LexError` with message containing "use `?=`"
- `x !<= y` produces a `LexError` containing "redundant"
- `x !>= y` produces a `LexError` containing "redundant"
- `!flag` produces a `LexError` containing "use the `not` keyword"
- `~x` lexes as `[bitnot, ident]` (unchanged from 1a; verifies `~` is not consumed as part of an equality operator)
- `result is unknown` lexes as `[ident, is_keyword, unknown_keyword]`
- The new keywords lex correctly: `stealth`, `excluding`, `invariant`, `decreases`, `from`

```bash
cargo test --lib lexer
# All lexer tests pass
```

---

## Phase 2: Parser Foundation

**Goal:** Parse HiLow source into an AST. Cover program/module structure, basic expressions, and statements; defer complex features.

### Phase 2a: Program/Module Structure and Top-Level

**Scope:**
- Top-level parsing: `high program(...) { ... }`, `low program(...) { ... }`, `high module { ... }`, `low module { ... }`
- Mode tracking on AST nodes (each function/block carries its inherited or explicit mode)
- Function declaration parsing (without bodies — just signature)
- Parameter list parsing with type annotations
- Return type parsing

**Out of scope:**
- Function bodies — Phase 2b
- Expressions — Phase 2b
- Statements — Phase 2b
- Imports/exports — Phase 11

**Tasks:**
1. Create `src/parser/mod.rs` and `src/ast/mod.rs`
2. Define core AST nodes: `Program`, `Module`, `Function`, `Parameter`, `Type`, `Mode` (enum: `High`, `Low`)
3. Implement parser entry point: parses one top-level construct (`program` or `module`)
4. Parse `high program(args: [string]): i32 { ... }` and `low program(...) { ... }`
5. Parse `high module { ... }` and `low module { ... }` with empty bodies
6. Parse function signatures: name, parameters with types, return type
7. Parse simple type expressions: primitives (`i32`, `f64`, etc.), arrays (`[T]`, `[T; N]`), function types (`function`)
8. Track mode inheritance: program/module mode is the default; record explicit mode on functions when present

**Verification:**
Test programs in `tests/parser/structure/`:

```hilow
// test1.hl - minimal high program
high program(): i32 {
}
```

```hilow
// test2.hl - minimal low program  
low program(): i32 {
}
```

```hilow
// test3.hl - high module with function signatures
high module {
  export function add(a: i32, b: i32): i32 {
  }
  function helper(): bool {
  }
}
```

```hilow
// test4.hl - mode override
high program(): i32 {
  function highFunc(x: i32): i32 {
  }
  low function lowFunc(p: *u8): u32 {
  }
}
```

Each test parses to the expected AST structure with correct mode propagation.

```bash
cargo test --lib parser::structure
# All structural tests pass
```

### Phase 2b: Statements and Basic Expressions

**Scope:**
- Statement parsing: `let` declarations (with optional type, optional initializer), `return`, `if`/`else`, `while`, `loop`, `break`, `continue`, expression statements
- Expression parsing with full operator precedence: arithmetic, comparison (including `?=`/`!=`/`is`), logical (`and`/`or`/`not`), bitwise
- Block parsing (`{ ... }`)
- Function call parsing
- Identifier and literal expressions

**Out of scope:**
- Pattern matching (`match`) — Phase 4
- `for` loops — Phase 4
- `switch` — Phase 4
- F-strings (parsed as opaque tokens, expression interior comes later) — Phase 6
- `defer`, `async` — later phases
- Object literals — Phase 7
- Array literals beyond simple cases — Phase 4
- Destructuring — Phase 4
- `(qualifier)=` parsing — Phase 5
- Tuple types and tuple expressions — Phase 4

**Tasks:**
1. Add AST nodes: `Statement`, `Expression`, `BinaryOp`, `UnaryOp`, `Block`, `LetDecl`, `IfStmt`, `WhileStmt`, `LoopStmt`, `ReturnStmt`, `Call`
2. Implement Pratt parser or recursive descent with explicit precedence table
3. Operator precedence (low to high):
   - `or`
   - `and`
   - `not` (unary)
   - `?=`, `!=`, `!<`, `!>`, `is`, `is not`, `<`, `>`, `<=`, `>=`
   - `|`
   - `^`
   - `&`
   - `<<`, `>>`
   - `+`, `-`
   - `*`, `/`, `%`
   - unary `-`, `~`, `not`
   - function calls, member access (`.`), indexing (`[]`)
4. Parse `let name [: type] [= expr]`
5. Parse `if (cond) { ... } else if (cond) { ... } else { ... }`
6. Parse `while (cond) { ... }` and `loop { ... }`
7. Parse `return expr` and bare `return`
8. Parse `break` and `continue`
9. Parse function calls including nested

**Verification:**
Test programs:

```hilow
// arith.hl
high program(): i32 {
  let x = 1 + 2 * 3
  let y = (1 + 2) * 3
  return 0
}
```

```hilow
// control.hl
high program(): i32 {
  let x = 10
  if (x > 5) {
    let y = x * 2
  } else {
    let y = x / 2
  }
  
  while (x > 0) {
    x -= 1
  }
  
  loop {
    break
  }
  
  return 0
}
```

```hilow
// equality.hl
high program(): i32 {
  let a = 5
  let b = 5
  let c = 6
  if (a ?= b) {
    return 0
  }
  if (a != c) {
    return 1
  }
  if (a is i32) {
    return 2
  }
  return 3
}
```

Each parses to the expected AST.

```bash
cargo test --lib parser
# All parser tests pass
```

---

## Phase 3: AST and Basic Type System

**Goal:** Establish the type representation and a minimal type checker that handles primitives and basic operations.

**Scope:**
- Full AST type definitions for all expression and statement variants needed by Phases 4-7
- Type representation: `Type` enum (i8/i16/i32/i64/i128, u8/u16/u32/u64/u128, f32, f64, bool, string, array, function, struct, nothing, unknown placeholder)
- Type checker entry point: walks AST, infers/checks types, reports errors
- Basic type inference for `let` with initializer
- Numeric literal type inference (`42` → i32, `3.14` → f64, with explicit suffix overrides — `42u8`)

**Out of scope:**
- Coercion (there is none — verify mismatches are errors)
- Object types — Phase 7
- Generics like `money<USD>` — Phase 9
- Constraint types (`i32 in 1..100`) — Phase 15
- Function contracts — Phase 15
- Closure type inference — Phase 7
- Mode-specific type rules (e.g., Low requires explicit signatures) — Phase 12
- Tuple types — Phase 4

**Tasks:**
1. Create `src/types/mod.rs` with `Type` enum and helper methods
2. Create `src/typecheck/mod.rs` with `TypeChecker` struct
3. Implement type representation for all primitives in the spec
4. Implement type inference for numeric literals (default i32, f64; suffix overrides)
5. Implement type checking for binary operators: same-type required for arithmetic, comparison, equality, bitwise
6. Implement type checking for `let`: if type annotated, verify initializer matches; if not, infer from initializer; if neither, type is "unknown until used"
7. Implement type checking for `if`/`while`/`loop` conditions: must be `bool` (no truthy coercion at type-check level — Phase 4 will revisit how truthy/falsy works)
8. Report clear errors with source position

**Note on truthy/falsy:** The design says `if (value)` accepts non-bool values where `0`, `""`, `nothing`, `unknown` are falsy. This is *not* coercion — it's a defined truthiness operation. Phase 4 will implement this; Phase 3 keeps it strict (bool only) and the tests reflect that. The code will be relaxed in Phase 4.

**Verification:**

```hilow
// types1.hl - should pass type check
high program(): i32 {
  let x: i32 = 42
  let y = 3.14
  let z: bool = true
  let sum = x + 100
  return 0
}
```

```hilow
// types2.hl - should fail type check
high program(): i32 {
  let x: i32 = 42
  let y: f64 = 3.14
  let bad = x + y    // Error: cannot add i32 and f64
  return 0
}
```

```hilow
// types3.hl - should fail (no coercion)
high program(): i32 {
  let s = "5"
  let n = 2
  let bad = s + n    // Error: cannot add string and i32
  return 0
}
```

```bash
cargo test --lib typecheck
# Tests pass: types1 succeeds, types2 and types3 produce expected errors
```

---

## Phase 4: High Mode Core

**Goal:** Compile and execute basic High-mode programs with variables, functions, control flow, and primitive arithmetic.

### Phase 4a: Codegen Foundation and Basic Programs

**Scope:**
- Codegen target: choose C as the initial backend (matches the existing compiler's approach; LLVM can come later in Phase 17)
- Code generation for: `program` block (becomes `int main()`), function declarations, `let` with initializers, return statements, basic arithmetic, function calls
- C runtime helpers: `print` for primitives
- Build orchestration: `hilowc input.hl -o output` produces an executable

**Out of scope:**
- if/else codegen — Phase 4b
- Loops — Phase 4b
- Strings beyond `print` of integer/float — Phase 6
- Closures — Phase 7
- Memory allocation — Phase 8
- Truthy/falsy — Phase 4b

**Tasks:**
1. Create `src/codegen/mod.rs` with C-emitting backend
2. Create `src/runtime/runtime.h` and `src/runtime/runtime.c` with `print_i32`, `print_f64`, `print_bool` helpers
3. Generate C: `program` becomes `int main(int argc, char **argv)`
4. Generate C for `let x: i32 = 42` as `int32_t x = 42;`
5. Generate C for arithmetic, function calls, return statements
6. Implement compile pipeline: parse → typecheck → codegen → invoke `cc` to produce binary

**Verification:**

```hilow
// hello_int.hl
high program(): i32 {
  let x = 42
  print(x)
  return 0
}
```

```bash
hilowc hello_int.hl -o hello_int
./hello_int
# Output: 42
```

```hilow
// arithmetic.hl
high program(): i32 {
  let a = 10
  let b = 3
  print(a + b)
  print(a - b)
  print(a * b)
  print(a / b)
  print(a % b)
  return 0
}
```

```bash
hilowc arithmetic.hl -o arithmetic
./arithmetic
# Output:
# 13
# 7
# 30
# 3
# 1
```

```hilow
// function_call.hl
high program(): i32 {
  function double(x: i32): i32 {
    return x * 2
  }
  print(double(21))
  return 0
}
```

```bash
hilowc function_call.hl -o function_call
./function_call
# Output: 42
```

### Phase 4b: Control Flow, Loops, Truthy/Falsy

**Scope:**
- Codegen for `if`/`else`/`else if`
- Codegen for `while`, `loop`, `break`, `continue`
- Truthy/falsy semantics: `if (value)` is true when value is non-zero / non-empty / not nothing / not unknown
- Compound assignment codegen (`+=`, `-=`, etc.)
- `for (let i = 0; i < n; i += 1)` C-style loops

**Out of scope:**
- `for-in` over arrays — Phase 4 sub-extension or moved to Phase 7
- `match` — Phase 5 or Phase 7 (TBD; deferred to keep this phase bounded)
- `switch` — same

Actually, let me move `match` and `for-in` to a later sub-phase to keep 4b bounded. Updated:

**Out of scope:**
- `match` — Phase 7c (after objects, since match-on-types needs them)
- `switch` — Phase 7c
- `for-in` — Phase 7c

**Tasks:**
1. Codegen for `if (cond) { ... } else { ... }`, including chained `else if`
2. Codegen for `while (cond) { ... }`
3. Codegen for `loop { ... }` as `while (1) { ... }` in C
4. Codegen for `for (let i = 0; i < n; i += 1)`
5. Codegen for `break` and `continue` as C `break` and `continue`
6. Implement truthy/falsy: when a non-bool value appears as a condition, generate the appropriate C check. For numeric: `(x != 0)`. For string: `(strlen(x) > 0)`. For nothing/unknown: deferred to those phases (Phase 9).
7. Codegen for compound assignments

**Verification:**

```hilow
// fizzbuzz.hl
high program(): i32 {
  for (let i = 1; i <= 15; i += 1) {
    if (i % 15 ?= 0) {
      print_str("FizzBuzz")
    } else if (i % 3 ?= 0) {
      print_str("Fizz")
    } else if (i % 5 ?= 0) {
      print_str("Buzz")
    } else {
      print(i)
    }
  }
  return 0
}
```

Note: this requires `print_str` for string literals. If strings aren't yet implemented in Phase 6, the test uses integer-only output:

```hilow
// fizzbuzz_numeric.hl - using sentinels until strings work
high program(): i32 {
  for (let i = 1; i <= 15; i += 1) {
    if (i % 15 ?= 0) {
      print(0)         // sentinel for FizzBuzz
    } else if (i % 3 ?= 0) {
      print(-3)        // sentinel for Fizz
    } else if (i % 5 ?= 0) {
      print(-5)        // sentinel for Buzz
    } else {
      print(i)
    }
  }
  return 0
}
```

```hilow
// counter.hl
high program(): i32 {
  let count = 0
  while (count < 5) {
    print(count)
    count += 1
  }
  return 0
}
```

```hilow
// truthy.hl
high program(): i32 {
  let x = 0
  if (x) {
    print(1)
  } else {
    print(2)         // expected: 2 (zero is falsy)
  }
  
  let y = 42
  if (y) {
    print(3)         // expected: 3 (non-zero is truthy)
  }
  
  if (not x) {
    print(4)         // expected: 4
  }
  
  return 0
}
```

```bash
hilowc counter.hl -o counter && ./counter
# Output: 0\n1\n2\n3\n4

hilowc truthy.hl -o truthy && ./truthy  
# Output: 2\n3\n4
```

---

## Phase 5: Equality and Qualified Operators

**Goal:** Full implementation of `?=`, `!=`, `is`, and the `(qualifier)=` / `(qualifier)!=` family.

### Phase 5a: Equality, Type Tests, and Negation Comparators

**Scope:**
- Codegen for `?=` (equality): types must match exactly, value comparison
- Codegen for `!=` (inequality)
- Codegen for `!<` and `!>` (negation comparators)
- The `is` operator for primitive type tests (`x is i32`, `x is bool`)
- Reject bare `==` at the parser level (in case it slipped past the lexer for any reason) with the same error message

**Out of scope:**
- `(qualifier)=` and `(qualifier)!=` operators — Phase 5b
- `is` for objects/prototypes — Phase 7
- Qualifier-based equality (`(roughly)=`, `(caseless)=`, etc.) — Phase 5b for the framework, Phase 6/9 for the specific qualifiers
- `is` with `nothing`/`unknown` (those types arrive in Phase 9)

**Tasks:**
1. Codegen for `?=`: for primitives, emit C `==`. Type mismatch is a compile error.
2. Codegen for `!=`: emit C `!=`. Type mismatch is a compile error.
3. Codegen for `!<`: emit C `>=` (logically equivalent). Type checking same as `<`.
4. Codegen for `!>`: emit C `<=` (logically equivalent). Type checking same as `>`.
5. Implement `is` operator for primitive types: at compile time, verify the operand's type matches the named type. Emit `1` (true) or `0` (false) since types are static.
6. Ensure clear error messages for type mismatches and bare `==` use.

**Verification:**

```hilow
// equality.hl
high program(): i32 {
  let a = 5
  let b = 5
  let c = 6
  
  if (a ?= b) {
    print(1)         // expected: 1
  }
  
  if (a != c) {
    print(2)         // expected: 2
  }
  
  if (a is i32) {
    print(3)         // expected: 3
  }
  
  return 0
}
```

```hilow
// negation_compare.hl
high program(): i32 {
  let count = 50
  let max = 100
  let min = 0
  
  if (count !> max) {        // count <= max
    print(1)                  // expected: 1
  }
  
  if (count !< min) {        // count >= min
    print(2)                  // expected: 2
  }
  
  // Combined invariant style
  if (count !> max and count !< min) {
    print(3)                  // expected: 3
  }
  
  return 0
}
```

```hilow
// type_mismatch.hl - should fail
high program(): i32 {
  let x: i32 = 5
  let y: f64 = 5.0
  if (x ?= y) {      // Error: type mismatch i32 vs f64
    return 1
  }
  return 0
}
```

```hilow
// bad_equals.hl - should fail
high program(): i32 {
  let a = 5
  let b = 5
  if (a == b) {      // Error: == is not a valid operator; use ?= for equality
    return 1
  }
  return 0
}
```

```bash
hilowc equality.hl -o equality && ./equality
# Output: 1\n2\n3

hilowc negation_compare.hl -o nc && ./nc
# Output: 1\n2\n3

hilowc type_mismatch.hl
# Error: type mismatch on line 4 (i32 vs f64)

hilowc bad_equals.hl
# Error: '==' is not a valid operator; use '?=' for equality or '=' for assignment
```

### Phase 5b: Qualified Operators

**Scope:**
- Parser: in expression context, after a value, recognize `( ident-list )=` and `( ident-list )!=` as qualified comparison operators, and `( ident-list )=` as a qualified assignment operator (context disambiguates)
- Single qualifier with no argument: `(or)=`, `(roughly)=`
- Single qualifier with named argument: `(within: 0.01)=`, `(after-conversion: USD)=`
- Multiple qualifiers (comma-separated): `(caseless, trimmed)=`
- Phase 5b implements ONLY the qualifier-assignment forms with no arguments: `(or)=`, `(and)=`, `(bitor)=`, `(bitand)=`, `(bitxor)=`. These are universal (work for any compatible numeric/boolean operands).
- Phase 5b also implements the *parsing infrastructure* for qualifiers with arguments and multiple qualifiers, but the only qualifier *codegen* in this phase is the universal assignment forms above.
- Register `coerce` as a known qualifier in the qualifier registry (for `(coerce)=`), but defer codegen to Phase 6c (where strings exist) and Phase 9 (where time/money exist).

**Out of scope:**
- `(coerce)=` codegen — Phase 6c (strings to primitives) and Phase 9 (string to time/money)
- Qualified *equality* operators like `(roughly)=`, `(caseless)=`, `(same-day)=` — these belong to type-specific phases (Phase 6c for strings, Phase 9 for time/money). Phase 5b builds the parser infrastructure that those phases will use.
- Atomic/saturating/volatile qualifiers — Low mode, Phase 12
- User-defined qualifiers — deferred to a future version of HiLow

**Tasks:**
1. Update parser to recognize the qualifier syntax: `expr (qualifier-list) = expr`, `expr (qualifier-list) != expr`. A qualifier-list is one or more `qualifier-spec` items separated by commas. A qualifier-spec is either `ident` or `ident: expr`.
2. AST: add `QualifiedOp { lhs, qualifiers: Vec<QualifierSpec>, op: QualifiedOpKind, rhs }` where `QualifiedOpKind` is `Assign | Eq | NotEq`. `QualifierSpec` is `{ name: String, arg: Option<Expression> }`.
3. Add a registry of known qualifiers with their valid operator contexts (assignment, equality, both) and their argument requirements (none, named-required, etc.).
4. For Phase 5b, register: `or`, `and`, `bitor`, `bitand`, `bitxor` (assignment-only, no arguments) with codegen, plus `coerce` (assignment-only, no arguments) with a placeholder error "coerce codegen not yet implemented for type X" — to be filled in by Phase 6c and Phase 9.
5. Codegen for `x (or)= y`: emit `x = x || y` (logical OR)
6. Codegen for `x (and)= y`: emit `x = x && y` (logical AND)
7. Codegen for `x (bitor)= y`: emit `x = x | y`
8. Codegen for `x (bitand)= y`: emit `x = x & y`
9. Codegen for `x (bitxor)= y`: emit `x = x ^ y`
10. Reject qualifiers not in the registry with a clear error: "qualifier 'foo' is not defined".
11. Reject qualifier use in the wrong operator context: "qualifier 'or' applies to assignment only, not equality".
12. Reject incorrect qualifier arguments: "qualifier 'or' takes no arguments" if user writes `(or: 5)=`.

**Verification:**

```hilow
// qualified_assign.hl
high program(): i32 {
  let flags = 0
  flags (bitor)= 4         // flags = 4
  flags (bitor)= 1         // flags = 5
  
  if (flags ?= 5) {
    print(1)               // expected: 1
  }
  
  let mask = 7
  flags (bitand)= mask     // flags = 5 & 7 = 5
  print(flags)             // expected: 5
  
  let ready = false
  ready (or)= true
  if (ready) {
    print(2)               // expected: 2
  }
  
  return 0
}
```

```hilow
// bad_qualifier.hl - should fail
high program(): i32 {
  let x = 0
  x (nonexistent)= 5      // Error: qualifier 'nonexistent' is not defined
  return 0
}
```

```hilow
// wrong_context.hl - should fail
high program(): i32 {
  let a = 5
  let b = 5
  if (a (or)= b) {        // Error: 'or' applies to assignment only, not equality
    return 1
  }
  return 0
}
```

```bash
hilowc qualified_assign.hl -o qa && ./qa
# Output: 1\n5\n2

hilowc bad_qualifier.hl
# Error: qualifier 'nonexistent' is not defined

hilowc wrong_context.hl
# Error: qualifier 'or' applies to assignment only, not equality
```

---

## Phase 6: Strings and F-Strings

**Goal:** Full string support including quote recursion, f-strings, and raw strings.

### Phase 6a: Basic Strings and Quote Recursion

**Scope:**
- Lexer support for string literals with quote recursion
- String type in the type system
- String literal codegen
- `print` for strings
- Escape sequences: `\n`, `\t`, `\r`, `\\`, `\u{...}`, `\x..`
- Raw strings: `r"..."`

**Out of scope:**
- F-strings — Phase 6b-i (basic interpolation), 6b-ii (format specifiers)
- String operations (`.length`, `.indexOf`, `.slice`, etc.) — Phase 16
- String concatenation (use f-strings instead)

**Tasks:**
1. Lexer: detect leading quote count, scan for closing same-count quote, content in between is the string
2. Lexer: handle `r"..."` raw strings (no escape processing)
3. Lexer: process escape sequences in non-raw strings
4. Parser: emit `StringLiteral` AST node
5. Type system: `string` type
6. Codegen: emit C string literals; handle UTF-8 directly
7. Runtime: `print_str(const char *s)` helper
8. Update `print` to dispatch based on argument type

**Verification:**

```hilow
// strings_basic.hl
high program(): i32 {
  let a = "Hello, HiLow World!"
  print(a)
  
  let b = ""contains "quotes" inside""
  print(b)
  
  let c = """
  triple-quoted
  multi-line
  """
  print(c)
  
  let d = r"C:\Users\Alice"
  print(d)
  
  return 0
}
```

```bash
hilowc strings_basic.hl -o sb && ./sb
# Output:
# Hello, HiLow World!
# contains "quotes" inside
# 
#   triple-quoted
#   multi-line
#   
# C:\Users\Alice
```

### Phase 6a-fixup: UTF-8 Codegen and Nested Functions

**Note**: This was an unplanned consolidation phase to address issues discovered during Phase 6a verification. Fixed UTF-8 string literal corruption in generated C code (hex escapes caused parsing conflicts), implemented nested function definitions inside program bodies with name mangling to avoid C keyword conflicts, added missing multiline.hl integration test, and cleaned up dead placeholder code. Nested functions work as declaration-only (no variable capture) until closures are implemented in Phase 7c.

### Phase 6b-i: F-Strings (Basic, No Format Specifiers)

**Scope:**
- F-string parsing: `f"..."` with `{expr}` interpolation
- Expression interpolation: primitives (i32, i64, f32, f64, bool, string) converted to default string representation
- Multi-line f-strings
- F-string with quote recursion: `f""embedded "quotes" with {var}""`
- Raw f-strings: `rf"..."` (no escape processing in string parts, but expressions still interpolate)
- Literal brace escaping: `{{` and `}}` for literal `{` and `}`

**Out of scope:**
- Format specifiers: `{expr:.2f}`, `{expr:x}`, `{expr:>10}` — Phase 6b-ii
- F-string formatting for `time` and `money` — Phase 9
- Custom format specifiers for user types — Phase 17

**Tasks:**
1. Lexer: emit `FStringStart`, `FStringText`, `FStringExprStart`, `FStringExprEnd`, `FStringEnd` token sequence
2. Parser: assemble f-string into `FString { parts: Vec<FStringPart> }` where parts are either Text or Expression
3. Detect format specifiers (`:` inside `{}`) and error with EXACT message: "format specifiers are not yet supported (Phase 6b-ii)"
4. Codegen: malloc'd buffer with snprintf chain for building result string (memory leak acceptable for Phase 6b-i, documented for Phase 8)
5. Runtime: no new helpers needed if using inline snprintf approach

**Verification:**

```hilow
// hello_fstring.hl
high program(): i32 {
  let name = "Alice"
  let age = 30
  print(f"Hello, {name}! You are {age} years old.")
  return 0
}
```

```hilow
// arithmetic_fstring.hl
high program(): i32 {
  let x = 2
  let y = 3
  print(f"{x} + {y} = {x + y}")
  return 0
}
```

```hilow
// format_spec_error.hl (should fail)
high program(): i32 {
  let x = 3.14159
  print(f"Pi: {x:.2f}")  // Error: format specifiers not yet supported
  return 0
}
```

### Phase 6b-ii: F-Strings (Format Specifiers)

**Scope:**
- Format specifier parsing: after `:` inside `{}`, parse format spec
- Format specifiers for primitives: decimal (d), hex (x/X), binary (b), float precision (.2f), padding (08d), alignment (>10, <10, ^10)
- Format specifier codegen: emit correct snprintf format strings

**Out of scope:**
- F-string formatting for `time` and `money` — Phase 9
- Custom format specifiers for user types — Phase 17

**Tasks:**
1. Parse format specifiers after `:` in expressions
2. Validate format specs against expression types (e.g., hex only for integers)
3. Codegen: translate format specs to snprintf format strings
4. Enhanced verification program with all format specifier types

**Verification:**

```hilow
// fstrings_formats.hl
high program(): i32 {
  let pi = 3.14159
  print(f"Pi: {pi:.2f}")
  
  let n = 255
  print(f"Hex: {n:x}, Bin: {n:b}, Padded: {n:08d}")
  
  let name = "Alice"
  print(f"|{name:>15}|")
  print(f"|{name:<15}|")
  
  return 0
}
```

```bash
hilowc fstrings_formats.hl -o fs && ./fs
# Output:
# Pi: 3.14
# Hex: ff, Bin: 11111111, Padded: 00000255
# |          Alice|
# |Alice          |
```

### Phase 6c: String Equality, String Qualifiers, and Coercion

**Scope:**
- String `?=` and `!=` codegen (deferred from Phase 5a since strings didn't exist yet)
- Register string qualifiers with the qualifier framework: `caseless`, `trimmed`
- Combinations: `(caseless, trimmed)=` works because the framework already supports comma-separated qualifiers from Phase 5b
- `(coerce)=` codegen for string-to-primitive conversions: `i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`. The compiler dispatches based on the target type.

**Out of scope:**
- Other string-specific qualifiers — can be added later as needed
- The full set of string operations (`.indexOf`, `.slice`, etc.) — Phase 16
- `(coerce)=` for `time` and `money` — those types don't exist yet; Phase 9
- `(coerce)=` to or from objects — Phase 7

**Tasks:**
1. Codegen for string `?=`: `strcmp(a, b) == 0`
2. Codegen for string `!=`: `strcmp(a, b) != 0`
3. Register `caseless` qualifier for strings: codegen as case-insensitive comparison (`strcasecmp` or equivalent)
4. Register `trimmed` qualifier for strings: codegen by trimming whitespace from both operands before comparison
5. Combined qualifiers: when multiple qualifiers are present, apply them in a defined order (trim first, then case-fold, then compare). Order should be deterministic regardless of how the user wrote them.
6. Codegen for `(coerce)=` from string to primitives:
   - `let n: i32 (coerce)= "42"` calls a runtime parser that returns `i32` or `unknown` on failure
   - For each target primitive, emit a call to the appropriate runtime function: `hl_parse_i32`, `hl_parse_f64`, etc.
   - If the source string cannot be parsed, the result is `unknown` with reason "could not parse <source> as <target type>"
   - Type mismatches (e.g., `(coerce)=` from `i32` to `string` — wrong direction) are compile errors
7. Runtime: implement `hl_parse_*` helpers for each primitive type. Use C standard library functions (`strtol`, `strtod`, etc.) but check for full-string consumption (trailing garbage is an error).
8. Reject string comparison with `<`, `>`, etc. (lexical ordering may come later, but is not in this phase)

**Verification:**

```hilow
// string_equality.hl
high program(): i32 {
  let a = "hello"
  let b = "hello"
  let c = "world"
  let d = "HELLO"
  let e = "  hello  "
  
  if (a ?= b) {
    print(1)         // 1
  }
  
  if (a != c) {
    print(2)         // 2
  }
  
  if (a (caseless)= d) {
    print(3)         // 3
  }
  
  if (a ?= d) {
    print(99)        // does NOT print (case-sensitive by default)
  }
  print(4)           // 4
  
  if (a (trimmed)= e) {
    print(5)         // 5
  }
  
  if (d (caseless, trimmed)= "  HELLO  ") {
    print(6)         // 6
  }
  
  return 0
}
```

```hilow
// coercion.hl
high program(): i32 {
  let n: i32 (coerce)= "42"
  print(n)                    // 42
  
  let f: f64 (coerce)= "3.14159"
  print(f)                    // 3.14159
  
  let bad: i32 (coerce)= "hello"
  if (bad is unknown) {
    print(0)                  // 0 (parse failed)
  }
  
  return 0
}
```

```bash
hilowc string_equality.hl -o se && ./se
# Output: 1\n2\n3\n4\n5\n6
```

---

## Phase 7: Flexible Objects and Closures (High mode)

**Goal:** High-mode object system with prototype delegation, plus closure support.

### Phase 7a: Object Literals and Property Access

**Scope:**
- Object literal syntax: `{ x: 10, y: 20 }`
- Object type in type system: `object` (general flexible object) and shape inference for known structures
- Property access: `obj.prop` (dot notation only)
- Property assignment: `obj.prop = value`
- Object representation in C: struct with hashmap for dynamic properties

**Note:** Computed access `obj[key]` requires `unknown` semantics for non-literal keys; deferred to Phase 7c (when match/for-in expand the dispatch substrate) or Phase 9 (when `unknown` lands).

**Out of scope:**
- Prototype delegation — Phase 7b
- Closures — Phase 7c
- Iteration (`for-in`) — Phase 7c
- Match on types — Phase 7c

**Tasks:**
1. AST: `ObjectLiteral { fields: Vec<(String, Expression)> }`, `PropertyAccess { obj, prop }`
2. Parser: parse object literals with `{ key: value, ... }` syntax
3. Parser: dot notation for property access (`obj.prop`)
4. Type system: `object` is a generic flexible type; track known field types when possible
5. Codegen: implement HiLow objects as a tagged-union value with a hashmap for fields
6. Runtime: `hl_object_t` struct, `hl_obj_new()`, `hl_obj_set(obj, key, value)`, `hl_obj_get(obj, key)`
7. Codegen for object literals: `hl_obj_new()` followed by `hl_obj_set` calls

**Verification:**

```hilow
// objects.hl
high program(): i32 {
  let point = { x: 10, y: 20 }
  print(point.x)               // 10
  print(point.y)               // 20
  
  point.z = 30
  print(point.z)               // 30
  
  return 0
}
```

```bash
hilowc objects.hl -o objs && ./objs
# Output: 10\n20\n30
```

### Phase 7b: Prototype Delegation

**Scope:**
- The `proto` field in object literals
- Prototype chain lookup: when `obj.x` doesn't find `x` directly, walk `obj.proto.x`, `obj.proto.proto.x`, etc.
- `is` operator on objects: `dog is animal` is true if `animal` is in `dog`'s prototype chain
- The `this` keyword in methods

**Out of scope:**
- Closures (covered in 7c) — methods here are still simple function values
- Method shorthand syntax (treat methods as fields holding function values)

**Tasks:**
1. Update `hl_obj_get` to walk prototype chain
2. AST/parser: `proto` is just a field name, no special syntax — it's looked up by string
3. Codegen for `is` on objects: walk proto chain checking for identity match
4. Codegen for method calls: when `obj.method(...)` is called, set `this` to `obj` for the call

**Verification:**

```hilow
// prototypes.hl
high program(): i32 {
  let animal = {
    proto: nothing,
    sound: "generic"
  }
  
  let dog = {
    proto: animal,
    name: "Rover"
  }
  
  print(dog.name)              // "Rover" (own property)
  print(dog.sound)             // "generic" (from prototype)
  
  if (dog is animal) {
    print(1)                   // 1 (in proto chain)
  }
  
  let cat = { proto: nothing, name: "Whiskers" }
  if (cat is animal) {
    print(2)                   // does NOT print
  }
  print(3)                     // 3
  
  return 0
}
```

```bash
hilowc prototypes.hl -o protos && ./protos
# Output: Rover\ngeneric\n1\n3
```

### Phase 7c-i: Closures, Method This Binding, Is for Objects

**Note:** Phase 7c was split into two sub-phases because the original scope was too large for one session.

**Scope:**
- Closures: function expressions that capture variables from enclosing scope
- Closure representation: function pointer + captured environment struct, allocated on the heap
- Method `this` binding: when calling `obj.method()`, the `this` keyword inside method refers to `obj`
- `is` operator on objects: `child is parent` walks the prototype chain to check membership

### Phase 7c-ii: For-In, Match, Switch

**Scope:**
- `for-in` iteration over arrays and objects
- Computed property access: `obj[key]` with dynamic keys (requires `unknown` for missing properties)
- `match` expressions with literal patterns, range patterns, type patterns (`is unknown`, `is nothing`, `is i32`), and guards (`when`)
- `switch` statements

**Out of scope:**
- Closure capture-by-reference vs by-value semantics — for now, all captures are by reference (this matters most for Low mode where it's restricted; Phase 12 enforces)
- Pattern matching with object destructuring — keep patterns simple here

**Phase 7c-i Tasks:**
1. Closures: at codegen, generate a struct with captured variables; the closure value is a `(fn_ptr, env_ptr)` pair
2. When a closure escapes (returned or stored), it must heap-allocate its environment — this connects to Phase 8 (memory). For now, allocate with `malloc` and don't worry about freeing; Phase 8 will replace this with refcounting.
3. Method `this` binding: for function expressions inside object literals, codegen the function with `this` as an implicit first parameter
4. Method calls: for `obj.method(args)` codegen passes `obj` as the first argument
5. `is` operator for objects: emit calls to `hl_object_is(child, parent)` runtime helper that walks prototype chain

**Phase 7c-ii Tasks:**
1. `for (let item in array)`: codegen as a counted loop
2. `for (let (k, v) in object)`: iterate the hashmap
3. `match`: codegen as a chain of if/else, evaluating each pattern in order
4. `switch`: codegen as C switch when discriminator is integer or string-comparable; otherwise as if/else chain

**Phase 7c-i Verification:**

```hilow
// closure_counter.hl
high program(): i32 {
  function makeCounter(): function {
    let count = 0
    return function(): i32 {
      count += 1
      return count
    }
  }
  
  let c = makeCounter()
  print(c())                   // 1
  print(c())                   // 2
  print(c())                   // 3
  return 0
}
```

```hilow
// method_this.hl
high program(): i32 {
  let obj = {
    name: "Test",
    say: function(): i32 {
      print(this.name)
      return 0
    }
  }
  obj.say()                    // "Test"
  return 0
}
```

```hilow
// is_object.hl
high program(): i32 {
  let animal = { type: "animal" }
  let dog = { proto: animal, breed: "labrador" }
  if (dog is animal) {
    print("dog is animal")     // matches
  }
  return 0
}
```

```bash
hilowc closure_counter.hl -o cc && ./cc
# Output: 1\n2\n3

hilowc method_this.hl -o mt && ./mt
# Output: Test

hilowc is_object.hl -o io && ./io
# Output: dog is animal
```

**Phase 7c-ii Verification:** (moved to Phase 7c-ii)

```hilow
// for_in.hl - moved to Phase 7c-ii
// matching.hl - moved to Phase 7c-ii
```

---

## Phase 8: Memory Model

**Goal:** Implement scope-based ownership, with implicit refcounting for High-mode escaped values.

### Phase 8a: Scope-Based Ownership for Stack Values

**Scope:**
- Variables of fixed-layout types (primitives, fixed-size arrays, fixed structs) live on the stack and are auto-cleaned at scope exit
- `defer` statement, both forms:
  - **Smart form**: `defer <var>` — compiler infers type-appropriate cleanup
  - **Explicit form**: `defer <expr>` — runs the literal expression at scope exit
- Move semantics for return values
- Resource cleanup registry: each resource type registers its cleanup function. For Phase 8a, the registry is empty (we don't have manual allocations or files yet); the smart-form lookup just errors clearly if used on something with no registered cleanup.

**Out of scope:**
- Heap allocation — Phase 8b
- Refcounting for escaped values — Phase 8c
- Low-mode `manual`/`arena`/`shared` — Phase 12 (those phases register their cleanup functions)
- Files, locks, etc. — Phase 16 (those types register their cleanup functions)

**Tasks:**
1. Codegen: stack-allocated values map directly to C local variables
2. Parser: distinguish `defer <var>` (smart form) from `defer <expr>` (explicit form). The smart form is `defer` followed by a single identifier with nothing else (no parentheses, no method calls). The explicit form is `defer` followed by any expression.
3. AST: `Defer { kind: DeferKind }` where `DeferKind` is `Smart(Identifier) | Explicit(Expression)`
4. Codegen: collect deferred items, emit them in reverse order at scope exit (including early returns and `break`)
5. For smart-form `defer`, look up the variable's type in the resource cleanup registry. Emit the appropriate cleanup expression. If the type has no registered cleanup, error: "defer <var>: type X has no automatic cleanup; use defer <expr> with an explicit cleanup expression."
6. Move semantics: when a value is returned from a function, it's copied (for primitives) or moved (for heap-owning values, which 8b will introduce)
7. Implement the cleanup registry as a compile-time table that later phases can extend.

**Verification:**

```hilow
// defer.hl
high program(): i32 {
  print(1)
  defer print(2)             // runs at scope exit (explicit form)
  defer print(3)             // runs first (LIFO)
  print(4)
  return 0
}
```

```bash
hilowc defer.hl -o defer && ./defer
# Output: 1\n4\n3\n2
```

```hilow
// defer_early_return.hl
high program(): i32 {
  defer print(99)
  if (1 ?= 1) {
    return 0                 // defer runs before return
  }
  return 1
}
```

```bash
./defer_early_return
# Output: 99
```

### Phase 8b: Heap Allocation and Refcounting Foundation

**Scope:**
- Heap allocation for objects and dynamic arrays (introduced in 7a; now properly tracked)
- Reference counting structure: every heap value has a refcount header
- Inc/dec primitives: `hl_retain(ptr)`, `hl_release(ptr)`
- Codegen inserts retain/release at appropriate points

**Out of scope:**
- Cycle detection (use weak references; user-managed for now) — later phase
- Low-mode explicit memory modes — Phase 12

**Tasks:**
1. Runtime: every heap value has a 16-byte header containing refcount + type tag
2. Codegen: object/array allocation uses a refcount-aware allocator
3. Codegen: when a value is assigned to a new variable that holds it, emit `retain`
4. Codegen: when a variable holding a heap value goes out of scope, emit `release`
5. Codegen: function returns transfer ownership (no extra retain/release at the boundary unless the value is also captured)
6. `hl_release(ptr)` decrements; if 0, calls type-specific destructor (which releases child references), then frees

**Verification:**

```hilow
// refcount_basic.hl
high program(): i32 {
  let a = { value: 42 }      // refcount=1
  let b = a                  // a still owned, b is reference; refcount=2
  print(a.value)             // 42
  print(b.value)             // 42
  return 0
                              // a, b go out of scope; refcount=0; freed
}
```

This should run without leaks. To verify, run under valgrind or similar:

```bash
hilowc refcount_basic.hl -o rb && ./rb
valgrind --leak-check=full ./rb
# Output: 42\n42, no leaks
```

### Phase 8c: Escape Analysis for Closures

**Scope:**
- Detect when a closure captures a variable that outlives the closure's defining scope
- For escaping captures, heap-allocate the captured variable's storage and refcount it
- For non-escaping captures (closure used only locally), keep on stack

**Out of scope:**
- Sophisticated escape analysis beyond closures — keep it simple
- Low-mode closure restrictions — Phase 12

**Tasks:**
1. Implement escape analysis pass: walk AST, find closures that escape their defining scope (returned, stored in heap object, passed to a function that may store them)
2. For each escaping closure, identify which captures need heap promotion
3. Codegen: for promoted captures, generate a refcounted "cell" struct holding the value
4. Closure invocations dereference cells transparently

**Verification:** The closure example from Phase 7c should now run leak-free:

```bash
valgrind --leak-check=full ./closures
# Output: 1\n2\n3, no leaks
```

---

## Phase 9: First-Class Types

**Goal:** Implement `nothing`, `unknown`, `time`, and `money` as first-class types.

### Phase 9a: Nothing and Unknown

**Scope:**
- `nothing` type: represents absence
- `unknown` type: failure with `reason: string` and `options: [string]` fields
- Truthy/falsy: `nothing` and `unknown` are falsy
- `is nothing` and `is unknown` checks
- Union types: `T | unknown`, with `T?` shorthand
- Property access on `unknown` propagates the same `unknown`

**Out of scope:**
- The proof system enforcing unknown handling — Phase 15
- Time and money (next sub-phases)

**Tasks:**
1. Type system: add `Nothing` and `Unknown` as types; `Unknown` is structural (has `reason`, `options`)
2. Type system: union types `T | unknown`; `T?` parsed as `T | unknown`
3. Runtime: `hl_value_t` includes a tag for nothing/unknown; unknown carries reason and options
4. Codegen: literal `nothing` is the nothing tag; `unknown(reason, options: [...])` constructs an unknown value
5. Codegen: `is nothing` and `is unknown` are runtime tag checks
6. Codegen: property access on unknown returns the same unknown (propagation)

**Verification:**

```hilow
// nothing_unknown.hl
high program(): i32 {
  let x
  if (x is nothing) {
    print(1)                 // 1
  }
  
  function maybeFail(): i32? {
    return unknown("test failure", options: ["retry", "abort"])
  }
  
  let result = maybeFail()
  if (result is unknown) {
    print(result.reason)     // "test failure"
    print(result.options[0]) // "retry"
  }
  
  // Propagation
  let bad = result.someProperty
  if (bad is unknown) {
    print(2)                 // 2 (propagated)
  }
  
  return 0
}
```

### Phase 9b: Time Type

**Scope:**
- `time` type: i64 nanoseconds since epoch + a precision tag (year, month, day, hour, minute, second, millisecond, microsecond, nanosecond)
- `duration` type (i64 nanoseconds, no precision tag — durations are always exact)
- Duration literals: `2h`, `30m`, `15s`, `500ms`, `250us`, `100ns`, `1d`
- Arithmetic: `time + duration`, `time - time = duration`, `duration + duration`. Arithmetic preserves the time operand's precision tag.
- `time.now()` (always nanosecond precision) and `time.parse(string)` (precision inferred from the input format)
- `.atPrecision(.unit)` method to coerce a time value to a specific precision
- Calendar operations: `.year()`, `.month()`, `.day()`, `.hour()`, `.minute()`, `.second()`, `.dayOfWeek()`
- `.next(.tuesday)`, `.month().nthWeekday(2, .tuesday)`, `.month().end()`
- **Precision-aware comparison**: `?=`, `!=`, `<`, `>`, `<=`, `>=` between two times compare at the precision of the less-precise operand
- F-string formatting: `f"{now:YYYY-MM-DD HH:mm:ss}"`
- Time qualifiers (registered with the qualifier framework from Phase 5b):
  - `(same-year)=`, `(same-month)=`, `(same-day)=`, `(same-hour)=`, `(same-minute)=` — equivalent to `.atPrecision(.unit) ?=` on both sides
  - `(within: duration)=` — true if `|t1 - t2| <= duration`

**Out of scope:**
- Time zones beyond the basic `time.now(.timezone(...))` and `.in(.timezone(...))` API — Phase 16 will round these out

**Tasks:**
1. Type system: add `time` and `duration` types. `time` carries a precision tag.
2. Lexer: duration literals (number followed by unit suffix)
3. Runtime: time is `{ nanos: i64, precision: u8 }`; duration is `i64`. Helpers for parsing, formatting, calendar.
4. `time.parse` infers precision from the input format: `"2024-01-15"` → day, `"2024-01-15T10:00"` → minute, `"2024-01-15T10:30:45.123"` → millisecond, etc.
5. Codegen for arithmetic operators on time/duration. `time + duration` preserves the time's precision tag. `time - time` produces duration regardless of operand precisions.
6. Codegen for comparison operators: when comparing two times, compute the coarser precision and compare both operands truncated to that precision.
7. `.atPrecision(.unit)` method: returns a new time with the specified precision.
8. F-string format spec for time
9. Register time qualifiers with the qualifier framework: `same-year`, `same-month`, `same-day`, `same-hour`, `same-minute`, `within`. Codegen for each.

**Verification:**

```hilow
// time_basic.hl
high program(): i32 {
  let now = time.now()
  let later = now + 1h + 30m
  let elapsed = later - now
  
  print(elapsed.minutes())   // 90.0
  
  let t1 = time.parse("2024-01-15T10:00:00Z")
  let t2 = time.parse("2024-01-15T22:30:00Z")
  if (t1 (same-day)= t2) {
    print(1)                 // 1
  }
  if (t1 (within: 1h)= t2) {
    print(2)                 // does NOT print (12.5 hours apart)
  }
  print(3)                   // 3
  
  return 0
}
```

```hilow
// time_precision.hl
high program(): i32 {
  let coarse = time.parse("2024-01-15T10:00")     // minute precision
  let fine = time.parse("2024-01-15T10:30:45")    // second precision
  
  // Comparison happens at minute precision (coarser)
  // 10:00 vs 10:30 at minute precision: 10:30 > 10:00
  if (coarse < fine) {
    print(1)                 // 1
  }
  
  let same = time.parse("2024-01-15T10:00:30")    // second precision
  // At minute precision both are 10:00
  if (coarse ?= same) {
    print(2)                 // 2
  }
  
  // Force second precision
  let exact = coarse.atPrecision(.second)
  // Now exact has nanos=10:00:00, precision=second
  if (exact ?= same) {
    print(3)                 // does NOT print (10:00:00 != 10:00:30)
  }
  print(4)                   // 4
  
  return 0
}
```

```bash
hilowc time_basic.hl -o tb && ./tb
# Output: 90.0\n1\n3

hilowc time_precision.hl -o tp && ./tp
# Output: 1\n2\n4
```

### Phase 9c: Money Type

**Scope:**
- `money` type with currency tag
- Currency literals: `19.99 USD`, `50.00 EUR`, `1000 JPY`
- `money<USD>` for currency-typed parameters
- Arithmetic: same-currency required (compile error otherwise)
- Display formatting: respects currency conventions
- `.convert(USD, rate: ...)`, `.round(.halfUp)`, `.allocate([...])`
- F-string formatting: `f"{amount}"`, `f"{amount:.4f}"`

**Tasks:**
1. Type system: `money` and parameterized `money<USD>` etc.
2. Lexer: currency suffix (`USD`, `EUR`, `JPY`, etc.) — recognized as part of numeric literal context
3. Currency table: known currencies with display precision and storage precision
4. Runtime: money is a struct of `{ amount: i64 (in storage units), currency: u32 (currency code) }`
5. Arithmetic: same-currency check at type level when types are concrete; runtime check otherwise
6. Formatting helpers
7. Methods: `.convert`, `.round`, `.allocate`, `.format`

**Verification:**

```hilow
// money_basic.hl
high program(): i32 {
  let price = 19.99 USD
  let tax = price * 0.08
  let total = price + tax
  
  print(f"Price: {price}")          // Price: $19.99
  print(f"Tax: {tax}")              // Tax: $1.60 (rounded)
  print(f"Total: {total}")          // Total: $21.59
  
  let bill = 100.00 USD
  let split = bill.allocate([1, 1, 1])
  print(split[0])                   // $33.34
  print(split[1])                   // $33.33
  print(split[2])                   // $33.33
  
  return 0
}
```

```hilow
// currency_mismatch.hl - should fail
high program(): i32 {
  let usd = 10.00 USD
  let eur = 10.00 EUR
  let bad = usd + eur               // Error: cannot add USD and EUR
  return 0
}
```

### Phase 9d: Tuples and Multi-Return

**Scope:**
- Tuple types: `(T, U)`, `(T, U, V)`
- Tuple literals: `(1, 2)`, `(1, "a", true)`
- Tuple destructuring: `let (a, b) = pair`
- Multi-return functions
- Array destructuring with rest: `let [head, ...tail] = arr`

**Tasks:**
1. Type system: tuples
2. Parser: tuple types in signatures, tuple literals, tuple destructuring
3. Codegen: tuples as anonymous structs

**Verification:**

```hilow
// tuples.hl
high program(): i32 {
  function divmod(a: i32, b: i32): (i32, i32) {
    return (a / b, a % b)
  }
  
  let (q, r) = divmod(17, 5)
  print(q)                          // 3
  print(r)                          // 2
  
  let arr = [1, 2, 3, 4, 5]
  let [head, ...tail] = arr
  print(head)                       // 1
  print(tail.length)                // 4
  
  return 0
}
```

---

## Phase 10: Watch System

**Goal:** Implement `watch()` reactive primitive and `async` blocks.

### Phase 10a: Basic Watch and Stealth

**Scope:**
- `watch(var) { ... }` syntax: registers a callback that fires when `var` is assigned
- `watch(var1, var2, ...) { ... }`: multi-variable watch
- Watch handle: `.pause()`, `.resume()`, `.end()`
- No self-triggering: modifications inside the watch body don't re-fire
- `stealth { ... }` block: dynamically suppresses watcher notifications for all writes during the block, including writes inside functions called from the block

**Out of scope:**
- `async` blocks — Phase 10b
- `shared` cross-process watches — Phase 10b
- Circular dependency detection — Phase 15

**Tasks:**
1. AST: `WatchExpr { vars, body }` returns a watch handle
2. AST: `StealthBlock { body }` is a statement that wraps a body
3. Codegen: assignments to watched variables are intercepted — they update the value and then fire registered watch callbacks
4. Watch handle: a struct with active/paused state and an end flag
5. Self-triggering prevention: a "currently executing" flag prevents re-entry
6. Stealth: maintain a thread-local "suppression depth counter" in the runtime. `stealth { ... }` increments at entry, decrements at exit. The watch-firing logic checks this counter — if non-zero, the watch is *not* called.
7. Stealth is dynamic: any function called from within a `stealth` block continues running with the suppression counter elevated, until control returns and the block exits.

**Verification:**

```hilow
// watch_basic.hl
high program(): i32 {
  let x = 0
  
  let w = watch(x) {
    print(x)
  }
  
  x = 10                            // prints 10
  x = 20                            // prints 20
  
  w.pause()
  x = 30                            // no print
  
  w.resume()
  x = 40                            // prints 40
  
  w.end()
  x = 50                            // no print
  
  return 0
}
```

```hilow
// stealth.hl
high program(): i32 {
  let counter = 0
  let total_seen = 0
  
  let w = watch(counter) {
    total_seen += 1
    print(counter)
  }
  
  counter = 1                       // prints 1, total_seen=1
  counter = 2                       // prints 2, total_seen=2
  
  stealth {
    counter = 100                   // no print
    counter = 200                   // no print
  }
  
  // total_seen is 2, counter is 200
  print(total_seen)                 // 2
  print(counter)                    // 200
  
  counter = 201                     // prints 201, total_seen=3
  print(total_seen)                 // 3
  
  return 0
}
```

```hilow
// stealth_dynamic.hl
high program(): i32 {
  let x = 0
  let count = 0
  
  let w = watch(x) {
    count += 1
  }
  
  function reset() {
    x = 0                           // would normally trigger watch
  }
  
  reset()
  print(count)                      // 1
  
  stealth {
    reset()                         // does NOT trigger watch (dynamic suppression)
  }
  print(count)                      // still 1
  
  return 0
}
```

```bash
hilowc watch_basic.hl -o wb && ./wb
# Output: 10\n20\n40

hilowc stealth.hl -o s && ./s
# Output: 1\n2\n2\n200\n201\n3

hilowc stealth_dynamic.hl -o sd && ./sd
# Output: 1\n1
```

### Phase 10b: Async and Shared

**Scope:**
- `async { ... }` block: runs concurrently (use threads on the C side)
- `shared let var = ...`: variable accessible across threads/processes (locked)
- Watches on `shared` variables work across threads

**Tasks:**
1. Runtime: a thread pool for `async` blocks
2. `shared` variables use a lock or atomic operations
3. Watches on shared variables register with the variable's mutex/condition

**Verification:**

```hilow
// async_watch.hl
high program(): i32 {
  shared let counter = 0
  
  for (let i = 0; i < 5; i += 1) {
    async {
      counter += 1
    }
  }
  
  let w = watch(counter) {
    print(counter)
    if (counter >= 5) {
      w.end()
    }
  }
  
  while (w.isActive()) {
    // wait
  }
  
  return 0
}
```

Output should show counter incrementing to 5; the exact print order may vary.

---

## Phase 11: Modules and Imports

**Goal:** Multi-file programs with `import`/`export`.

**Scope:**
- `import { name1, name2 } from "./path"`
- `export function ...` and `export let ...`
- Module compilation order (DAG resolution)
- Mode-aware imports: high program importing from low module works automatically

**Tasks:**
1. Parser: `import` and `export` statements
2. Module resolver: resolve `"./path"` to file paths
3. Compilation order: build a dependency graph, compile in topological order
4. Linker: combine compiled modules into a single binary
5. Mode crossing: when a high program imports a low function, the call site uses the low calling convention (which should be identical for shared types)

**Verification:**

```hilow
// math.hl
high module {
  export function add(a: i32, b: i32): i32 {
    return a + b
  }
  
  export let PI: f64 = 3.14159
}
```

```hilow
// app.hl
import { add, PI } from "./math"

high program(): i32 {
  print(add(2, 3))                  // 5
  print(PI)                         // 3.14159
  return 0
}
```

```bash
hilowc app.hl -o app && ./app
# Output: 5\n3.14159
```

---

## Phase 12: Low Mode Features

**Goal:** Full Low-mode capability: pointers, fixed structs, and explicit memory modes.

### Phase 12a: Pointers and Fixed Structs

**Scope:**
- Pointer types `*T`, `**T`
- Address-of: `address(var)`, dereference: `*ptr`
- Pointer arithmetic: `ptr + 1`, `ptr - 1`
- Fixed structs with explicit layout
- `@packed`, `@align(N)` attributes

**Out of scope:**
- Memory mode keywords — Phase 12b
- Inline asm — Phase 13

**Tasks:**
1. Type system: pointer types in Low mode (rejected in High)
2. Parser: `*T` type syntax, `address()` and `*expr` operators
3. Codegen: pointers map to C pointers
4. Struct attributes: emit C `__attribute__((packed))`, `__attribute__((aligned(N)))`

**Verification:**

```hilow
// pointers.hl
low program(): i32 {
  let x: i32 = 42
  let p: *i32 = address(x)
  print(*p)                         // 42
  
  let arr: [i32; 5] = [10, 20, 30, 40, 50]
  let pa: *i32 = address(arr[0])
  print(*pa)                        // 10
  pa += 1
  print(*pa)                        // 20
  
  return 0
}
```

### Phase 12b: Memory Mode Declarators (manual, arena, shared, stack, heap)

**Scope:**
- `manual let buf = alloc(N)`: explicit allocation, programmer frees
- `defer buf` (smart) or `defer free(buf)` (explicit) for manual cleanup
- `arena { ... }`: bulk allocation block, all freed at end
- `shared let res = rc_alloc<T>()`: refcounted in low mode (opt-in)
- Standalone `stack <name>: <type>` declarators: equivalent to `let` but documents stack location explicitly
- Standalone `heap <name>: <type>` declarators: equivalent to `let` but documents heap location explicitly
- `alloc()`, `free()`, `arena.alloc()`, `rc_alloc<T>()` runtime functions
- Register cleanup for `manual` allocations in the resource cleanup registry from Phase 8a (so `defer <var>` works on them)

**Tasks:**
1. Parser: `manual`, `arena`, `shared` keywords in let declarations
2. Parser: standalone `stack <name>: <type> [= <expr>]` and `heap <name>: <type> [= <expr>]` declarations (Low mode only)
3. Type checker: reject `stack` and `heap` declarators in High mode with a clear error: "`stack` and `heap` declarators are only available in low mode; use `let` instead"
4. Runtime: `alloc`, `free`, arena allocator with bulk free, refcounted alloc
5. Codegen: `manual` skips automatic cleanup; `arena` blocks set up an arena and tear it down at end; `shared` uses refcounted allocator
6. Codegen: `stack` and `heap` map to the same code generation as `let` (the location annotation is just documentation; the compiler already chooses appropriately)
7. Register `manual`-allocated values in the cleanup registry: smart `defer <var>` on a manual variable emits `free(var)`

**Verification:**

```hilow
// memory_modes.hl
low program(): i32 {
  manual let buf = alloc(1024)
  defer buf                         // smart form: emits free(buf)
  
  arena {
    let a = arena.alloc(100)
    let b = arena.alloc(200)
    // a, b freed at end of arena block
  }
  
  return 0
}
```

```hilow
// stack_heap_decls.hl
low program(): i32 {
  stack p: i64 = 42
  stack buffer: [u8; 256]
  heap data: [u32; 1024]
  
  print(p)                          // 42
  return 0
}
```

```hilow
// stack_in_high.hl - should fail
high program(): i32 {
  stack p: i64 = 42                 // Error: stack/heap declarators are low-mode only
  return 0
}
```

```bash
hilowc memory_modes.hl -o mm && ./mm
valgrind --leak-check=full ./mm
# No leaks

hilowc stack_heap_decls.hl -o shd && ./shd
# Output: 42

hilowc stack_in_high.hl
# Error: 'stack' and 'heap' declarators are only available in low mode; use 'let' instead
```

### Phase 12c: Low Mode Restrictions

**Scope:**
- Reject flexible objects in low mode (compile error)
- Require explicit type annotations on all function signatures in low mode
- Reject closures that escape their defining scope in low mode
- Reject reflection (`for-in` over object) in low mode

**Tasks:**
1. Type checker: check the current mode (from program/module/function/block context); apply restrictions
2. Clear error messages: "flexible objects not allowed in low mode; use a struct"

**Verification:**

```hilow
// bad_low.hl - should fail
low program(): i32 {
  let obj = { x: 10, y: 20 }        // Error: no flexible objects in low
  return 0
}
```

```hilow
// bad_low2.hl - should fail
low program(): i32 {
  function badInfer(x) {            // Error: low requires type annotations
    return x * 2
  }
  return 0
}
```

### Phase 12d: Low-Mode Atomic and Memory-Ordered Qualifiers

**Scope:**
- `(atomic-add)=`, `(atomic-sub)=`, `(atomic-or)=`, `(atomic-and)=`
- `(saturating-add)=`, `(saturating-sub)=`
- `(volatile)=`
- All available only in low mode

**Tasks:**
1. Add to qualifier handler from Phase 5b
2. Codegen: use C11 atomics (`stdatomic.h`) for atomic ops
3. Saturating: emit explicit overflow check
4. Volatile: emit C `volatile` qualifier and use direct write

**Verification:**

```hilow
// low_qualifiers.hl
low program(): i32 {
  shared let counter: u32 = 0
  
  for (let i = 0; i < 100; i += 1) {
    counter (atomic-add)= 1
  }
  
  print(counter)                    // 100
  return 0
}
```

---

## Phase 13: Inline Assembly

**Goal:** Support `asm { ... }` blocks in Low mode.

**Scope:**
- `asm { ... }` block parsing
- Verifying referenced variables are accessible
- Emitting GCC-style inline asm with proper input/output constraints
- Platform support: x86_64 initially

**Tasks:**
1. Lexer: lex asm body as a literal string
2. Parser: `asm { ... }` produces an `AsmBlock` node
3. Codegen: emit GCC inline asm, computing operand constraints from variable references
4. Document supported constraints

**Verification:**

```hilow
// asm_test.hl
low program(): i32 {
  let result: u64 = 0
  
  asm {
    rdtsc
    shl rdx, 32
    or rax, rdx
    mov [result], rax
  }
  
  print(result > 0)                 // true
  return 0
}
```

---

## Phase 14: Mode Boundary Enforcement

**Goal:** Implement `@low-callable` annotation and verify mode-crossing rules.

**Scope:**
- `@low-callable` attribute on high functions
- Verifier: a `@low-callable` high function uses no high-only features
- Calls from low to unmarked high functions are errors

**Tasks:**
1. Parser: `@low-callable` attribute syntax
2. Type checker: when a high function is marked `@low-callable`, walk its body and reject any high-only feature use
3. Type checker: when a low function calls a high function, the high function must be `@low-callable`

**Verification:**

```hilow
// boundary_ok.hl
@low-callable
high function checksum(data: *u8, len: usize): u32 {
  let sum: u32 = 0
  for (let i: usize = 0; i < len; i += 1) {
    sum = sum + data[i]
  }
  return sum
}

low function process(): u32 {
  let buf = alloc(100)
  defer free(buf)
  return checksum(buf, 100)         // OK
}

low program(): i32 {
  let cs = process()
  print(cs)
  return 0
}
```

```hilow
// boundary_bad.hl - should fail
@low-callable
high function bad(): object {       // Error: returns flexible object
  return { x: 1 }
}
```

---

## Phase 15: Formal Verification

**Goal:** Optional, layered proof system. Compiler flags `--prove` (warnings) and `--strict` (errors) drive verification depth. Verification covers constraints, contracts, loop invariants, termination, memory and resource lifecycle, numeric overflow, concurrency safety, and type-level properties.

This is the largest phase by complexity. The proof system uses an SMT solver (Z3 via Rust bindings) to verify properties. Each sub-phase introduces one class of verification. The layered design — warnings by default, errors with `--strict` — means programmers can adopt verification incrementally.

**Compiler flags introduced in this phase:**
- `hilowc <file>` — compile, no proof checking (proof clauses are parsed but ignored)
- `hilowc <file> --prove` — compile + verify, warnings on issues, runtime checks emitted where static proof fails
- `hilowc <file> --prove --strict` — same as above but warnings become errors
- `hilowc <file> --prove-only` — verify without producing a binary
- `hilowc <file> --prove --suggest` — include suggestions for improvement (e.g., redundant constraints)

### Phase 15a: Variable Constraints (Predicates and Sets)

**Scope:**
- Predicate constraint syntax: `let x: i32 (x >= 0 and x <= 100) = 50`
- Set constraint syntax: `let x: i32 in {0..100} = 50` and `let x: i32 in {1, 2, 5..14, 16}`
- `excluding` clause: `let valid: i32 in {1..100} excluding {10, 12}`
- Member elements may be literals, variables, or function calls
- When members are runtime values, fall back to runtime checks

**Tasks:**
1. Add Z3 dependency (Rust bindings)
2. Parser: predicate constraint clause `(<expr>)` on let declarations
3. Parser: set constraint clause `in { <member-list> } [excluding { <member-list> }]` on let declarations
4. Member: scalar expression OR `<expr>..<expr>` (inclusive range)
5. Verifier: at each assignment to a constrained variable, encode the predicate or set membership in Z3 and check that the new value satisfies it (using surrounding control flow as context)
6. When constraint involves runtime values (variables, function calls), emit a runtime check at the assignment instead of (or in addition to) the static proof
7. Wire up `--prove`, `--strict`, `--prove-only`, `--suggest` flags

**Verification:**

```hilow
// constraints_ok.hl - --prove should succeed
high program(): i32 {
  let percent: i32 in {0..100} = 50
  percent = 75                      // OK
  
  let direction: i32 in {-1, 0, 1} = 0
  direction = -1                    // OK
  
  let valid: i32 in {1..100} excluding {10, 12} = 5
  valid = 13                        // OK
  
  return 0
}
```

```hilow
// constraints_bad.hl - --prove should warn, --strict should fail
high program(): i32 {
  let percent: i32 in {0..100} = 50
  percent = 150                     // Proof error / runtime check fails
  
  let valid: i32 in {1..100} excluding {10, 12} = 5
  valid = 10                        // Proof error: in exclusion set
  
  return 0
}
```

```hilow
// constraints_predicate.hl
high program(): i32 {
  let length: i32 (length >= 0)
  let capacity: i32 (capacity >= 0) = 100
  let safe_length: i32 (safe_length !> capacity) = 50
  
  safe_length = 200                 // Proof error: 200 > capacity (100)
  return 0
}
```

### Phase 15b: Function Contracts, Loop Invariants, and Termination

**Scope:**
- `requires (cond)` and `ensures (cond)` on functions
- `result` is the named return value in `ensures`
- `invariant (cond)` on loops
- `decreases (expr)` on functions (recursion termination) and loops (iteration termination)

**Tasks:**
1. Parser: `requires`, `ensures`, `invariant`, `decreases` clauses
2. Verifier: contract verification with Z3
3. Verifier: loop invariant verification (entry, preservation, exit)
4. Verifier: termination via decreases — verify the expression is non-negative and strictly decreases each iteration/recursion

**Verification:**

```hilow
// contract_ok.hl
high program(): i32 {
  function divide(a: i32, b: i32): i32
    requires (b != 0)
  {
    return a / b
  }
  
  let x = divide(10, 5)             // OK
  
  let d = 5
  if (d != 0) {
    let y = divide(10, d)           // OK (control flow tells prover)
  }
  
  return 0
}
```

```hilow
// invariant_ok.hl
high program(): i32 {
  let total = 0
  let arr = [1, 2, 3, 4, 5]
  for (let i = 0; i < arr.length; i += 1)
    invariant (total >= 0 and i <= arr.length)
  {
    total += arr[i]
  }
  return 0
}
```

```hilow
// termination_ok.hl
high program(): i32 {
  function fact(n: i32): i32
    requires (n >= 0)
    decreases (n)
  {
    if (n ?= 0) return 1
    return n * fact(n - 1)
  }
  
  print(fact(5))                    // 120
  return 0
}
```

### Phase 15c: Memory Safety and Resource Lifecycle

**Scope:**
- Verify no use-after-free
- Verify no double-free
- Verify no leaks in `manual` blocks
- Verify array bounds at compile time when possible
- Resource lifecycle: file/lock/connection types declare valid state transitions; verify code respects them

**Tasks:**
1. Track allocation/free pairs through the AST
2. Resource state machine: each resource type has states and transitions defined in its type metadata
3. Verifier: walk all paths through a function, ensuring resources reach a terminal state (closed/freed/released) on every path
4. Verify with Z3 that all paths through a function have correct memory discipline

**Verification:**

```hilow
// memory_ok.hl
low program(): i32 {
  manual let buf = alloc(1024)
  defer buf
  
  // use buf...
  return 0
}                                   // ✓ Proof: buf is freed by defer
```

```hilow
// memory_bad.hl - --strict fails
low program(): i32 {
  manual let buf = alloc(1024)
  // No defer, no explicit free
  return 0
}                                   // ✗ Proof error: buf leaked
```

```hilow
// resource_ok.hl
high program(): i32 {
  let file = openFile("data.txt")
  if (file is unknown) return 1
  defer file
  
  let content = file.read()         // ✓ valid: file is open
  return 0
}                                   // ✓ Proof: file is closed by defer
```

### Phase 15d: Watch, Type, and Currency Safety

**Scope:**
- Verify no circular watch dependencies
- Verify `unknown` returns are checked before use
- Verify currency consistency in `money` arithmetic
- Verify time arithmetic respects precision rules

**Tasks:**
1. Watch dependency graph: build graph, check for cycles
2. Unknown handling: track which variables may hold unknown; require check before use
3. Currency tracking: enforce same-currency at compile time when types are concrete
4. Time precision: verify operations on times of different precisions follow the precision rule

### Phase 15e: Numeric Overflow

**Scope:**
- For each arithmetic operation, verify the result fits in the target type
- In Low mode, default behavior is checked overflow (warning); programmer opts into wrapping/saturating with explicit qualifiers
- In High mode, default behavior is `unknown` on overflow

**Tasks:**
1. Verifier: track value ranges through arithmetic operations using interval analysis
2. At each `+`, `-`, `*` etc., verify the result range fits in the target type
3. Where static verification fails, emit a runtime overflow check
4. Document the overflow policy difference between High and Low modes

**Verification:**

```hilow
// overflow_ok.hl
high program(): i32 {
  let a: u8 in {0..100} = 50
  let b: u8 in {0..100} = 50
  let sum: u16 = a + b              // ✓ 50+50=100 fits in u8 trivially, and fits in u16
  return 0
}
```

```hilow
// overflow_warning.hl
low program(): i32 {
  let a: u8 = 200
  let b: u8 = 200
  let sum: u8 = a + b               // ⚠ Proof warning: u8 + u8 may overflow
  return 0
}
```

```hilow
// overflow_explicit.hl
low program(): i32 {
  let a: u8 = 200
  let b: u8 = 100
  
  let sum: u8 = a
  sum (saturating-add)= b           // ✓ explicit saturating; sum becomes 255
  return 0
}
```

### Phase 15f: Concurrency Safety

**Scope:**
- Verify all accesses to `shared` variables use atomic operations or proper locking
- Verify `async` blocks don't have data races on captured non-shared variables
- Verify watch callbacks on `shared` variables are thread-safe

**Tasks:**
1. Identify all `shared` variables in the program
2. For each access to a `shared` variable, classify as atomic-safe or potentially-racy
3. Read-modify-write sequences (e.g., `shared_var = shared_var + 1`) without explicit atomicity warn
4. Watch callbacks on `shared` variables: verify the callback body is itself thread-safe

**Verification:**

```hilow
// concurrency_ok.hl
high program(): i32 {
  shared let counter: i32 = 0
  
  async {
    counter (atomic-add)= 1         // ✓ explicit atomic
  }
  
  return 0
}
```

```hilow
// concurrency_warning.hl
high program(): i32 {
  shared let counter: i32 = 0
  
  async {
    let old = counter               // racy: read
    counter = old + 1               // ⚠ racy: write, not atomic with read
  }
  
  return 0
}
```

---

## Phase 16: Standard Library

**Goal:** Comprehensive standard library for High mode.

### Phase 16a: Math, String, Array Operations

- `abs`, `sqrt`, `pow`, `sin`, `cos`, `floor`, `ceil`, `round`, `PI`, `E`
- `string.length`, `.indexOf`, `.slice`, `.split`, `.join`, `.replace`, `.toUpperCase`, `.toLowerCase`, `.trim`
- Array methods (already partly there from Phase 7): `push`, `pop`, `length`

### Phase 16b: File I/O and Networking

- `openFile`, `.read`, `.write`, `.close` (return `unknown` on error)
- `http.get`, `http.post`, `http.listen` (high mode only)
- `json.parse`, `json.stringify`

### Phase 16c: Database and Async Helpers

- Basic database client (placeholder API)
- Promise-like helpers built on `watch` and `async`

---

## Phase 17: Polish and Cross-Compilation

**Goal:** Production-ready compiler.

**Scope:**
- LLVM backend as alternative to C
- WebAssembly target
- Bare-metal target with `--no-stdlib`
- Optimization levels `-O0` through `-O3`
- Build system: `hilow.toml` config
- Better error messages with source snippets and suggestions
- Documentation generation from doc comments

---

## After Phase 17

The compiler is feature-complete against the spec. Subsequent work:

- Performance tuning
- Language server protocol implementation
- Editor integrations (VS Code extension, syntax highlighting)
- Package registry and dependency management
- Real-world example programs and tutorials
- A "Learning HiLow" book in the spirit of "Learning AmorphDB"

---

## Notes for Claude Code Sessions

**On context management:**

When working on a phase, Claude Code should load only:
1. `docs/hilow-design.md` sections relevant to the current phase
2. This plan, focused on the current phase's section
3. The current source files being modified

Avoid loading the entire design or plan unless explicitly needed.

**On verification:**

A phase is complete when:
1. All listed verification programs compile without errors
2. All listed verification programs produce the exact expected output
3. The full test suite from previous phases still passes
4. The change is committed with a clear message

If verification fails, fix the issue or revert and re-plan. Do not declare the phase complete with known failures.

**On scope drift:**

If implementing a phase reveals that something from a later phase is needed:
1. Stop and check the plan
2. If the dependency is real, the plan may need adjustment — document this
3. Do not silently implement the later phase to satisfy the current one
4. A small dependency may be acceptable if explicitly noted; a large one means the plan is wrong

**On the design spec:**

The spec (`docs/hilow-design.md`) is the source of truth. If this plan and the spec disagree, the spec wins. If the spec is unclear, ask before implementing — do not guess.
