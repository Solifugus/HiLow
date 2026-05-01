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
| 6 | Strings and f-strings | 2 (6a, 6b) |
| 7 | Flexible objects and closures | 3 (7a, 7b, 7c) |
| 8 | Memory model | 3 (8a, 8b, 8c) |
| 9 | First-class types | 4 (9a, 9b, 9c, 9d) |
| 10 | Watch system | 2 (10a, 10b) |
| 11 | Modules and imports | 1 |
| 12 | Low mode features | 4 (12a, 12b, 12c, 12d) |
| 13 | Inline assembly | 1 |
| 14 | Mode boundary enforcement | 1 |
| 15 | Formal verification | 4 (15a, 15b, 15c, 15d) |
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
- Equality operators (`?=`, `~=`, `?!=`, `~!=`) — Phase 5
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

### Phase 1b: Equality Operators and Mode Tokens

**Scope:**
- Equality operators: `?=`, `?!=`, `~=`, `~!=`
- The `is` keyword (already in keyword table from 1a; verify it tokenizes correctly)
- The qualifier-paren-equals pattern `(...)=` — lexer treats `(` and `)` as separate tokens; the parser will recognize the `(qualifier)=` pattern
- Multi-line operator disambiguation (e.g., `?` followed by `=` vs `?` alone)

**Out of scope:**
- Parsing `(qualifier)=` semantically — parser's job
- Validating qualifier names — type checker's job

**Tasks:**
1. Add `TokenKind` variants: `EqStrict` (`?=`), `NotEqStrict` (`?!=`), `EqApprox` (`~=`), `NotEqApprox` (`~!=`)
2. Implement lexing for `?=` (look for `?` followed by `=` or `!=`)
3. Implement lexing for `~=` (look for `~` followed by `=` or `!=`)
4. Disambiguate `?` alone (used in `T?` type syntax) from `?=` and `?!=`
5. Disambiguate `~` alone (bitwise NOT) from `~=` and `~!=`

**Verification:**
Add tests to `tests/lexer/equality.rs`:
- `x ?= y` lexes as `[ident, eq_strict, ident]`
- `x ?!= y` lexes as `[ident, not_eq_strict, ident]`
- `x ~= y` lexes as `[ident, eq_approx, ident]`
- `x ~!= y` lexes as `[ident, not_eq_approx, ident]`
- `x ? y` lexes as `[ident, question, ident]` (the `?` alone is for `T?` types — Phase 9)
- `~x` lexes as `[bitnot, ident]`
- `result is unknown` lexes as `[ident, is_keyword, unknown_keyword]`

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
- Expression parsing with full operator precedence: arithmetic, comparison (including new `?=`/`~=`/`is`), logical (`and`/`or`/`not`), bitwise
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
   - `?=`, `?!=`, `~=`, `~!=`, `is`, `is not`, `<`, `>`, `<=`, `>=`
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
  if (a ?= b) {
    return 0
  }
  if (a ~= b) {
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

## Phase 5: Equality and Comparison Operators

**Goal:** Full implementation of `?=`, `?!=`, `~=`, `~!=`, `is`, and `(qualifier)=`.

### Phase 5a: Strict and Approximate Equality

**Scope:**
- Codegen for `?=` (strict equality): types must match exactly, value comparison
- Codegen for `?!=` (strict inequality)
- Codegen for `~=` and `~!=` for numeric types (default: same as `?=`; per-type approximate semantics defined in Phase 9)
- The `is` operator for primitive type tests (`x is i32`, `x is bool`)

**Out of scope:**
- `~=` for strings (case-insensitive) — Phase 6
- `~=` with numeric tolerance — needs type-level config, Phase 9
- `is` for objects/prototypes — Phase 7
- `(qualifier)=` operators — Phase 5b

**Tasks:**
1. Codegen for `?=`: for primitives, emit C `==`. For strings (when added), emit `strcmp() == 0`. Type mismatch is a compile error.
2. Codegen for `?!=`: emit C `!=` (or `!strcmp(...)` for strings)
3. Codegen for `~=` on primitives: same as `?=` for now (the "approximate" comes into play with floats and strings later)
4. Implement `is` operator: at compile time, verify the operand's type matches the named type. Emit `1` (true) or `0` (false) since types are static.
5. Edge case: `is` with `nothing`/`unknown` is checked at runtime (those types come in Phase 9; for now, `is` on primitives is compile-time).

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
  
  if (a ?!= c) {
    print(2)         // expected: 2
  }
  
  if (a ~= b) {
    print(3)         // expected: 3
  }
  
  if (a is i32) {
    print(4)         // expected: 4
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

```bash
hilowc equality.hl -o equality && ./equality
# Output: 1\n2\n3\n4

hilowc type_mismatch.hl
# Error: Phase 5a type mismatch on line 4
```

### Phase 5b: Qualified Operators

**Scope:**
- Lexer recognizes `(qualifier)=` pattern as a contextual operator (parser handles this — lexer just emits `(`, identifier(s), `)`, `=`)
- Parser: in expression context, after a value, recognize `( ident ... )=` as a qualified equality operator
- Initial qualifiers: `(or)=`, `(and)=`, `(bitor)=`, `(bitand)=`, `(bitxor)=` for assignment forms
- Initial equality qualifiers: deferred to specific types — Phase 9 will add `(within: N)=`, `(case-insensitive)=`, `(same-day)=`, etc.
- For Phase 5b: implement only the assignment forms listed above (`(or)=`, `(and)=`, `(bitor)=`, `(bitand)=`, `(bitxor)=`)

**Out of scope:**
- Qualified equality (vs. assignment) — those qualifiers are type-specific, Phase 9
- Atomic/saturating/volatile qualifiers — Low mode, Phase 12
- Custom user-defined qualifiers — Phase 17 or later

**Tasks:**
1. Update parser to recognize `expr (ident)= expr` and `expr (ident: expr)= expr` patterns
2. AST: add `QualifiedAssign { target, qualifier, args, value }` variant
3. Codegen for `x (or)= y`: emit `x = x || y`
4. Codegen for `x (and)= y`: emit `x = x && y`
5. Codegen for `x (bitor)= y`: emit `x = x | y`
6. Codegen for `x (bitand)= y`: emit `x = x & y`
7. Codegen for `x (bitxor)= y`: emit `x = x ^ y`
8. Reject qualifiers not in this list (clear error: "qualifier 'foo' not supported in this context")

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
  
  return 0
}
```

```bash
hilowc qualified_assign.hl -o qa && ./qa
# Output: 1\n5
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
- F-strings — Phase 6b
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

### Phase 6b: F-Strings

**Scope:**
- F-string parsing: `f"..."` with `{expr}` interpolation
- F-string with format specifiers: `{expr:.2f}`, `{expr:x}`, `{expr:>10}`, etc. for primitives
- Multi-line f-strings
- F-string with quote recursion: `f""embedded "quotes" with {var}""`
- Raw f-strings: `rf"..."`

**Out of scope:**
- F-string formatting for `time` and `money` — Phase 9
- Custom format specifiers for user types — Phase 17

**Tasks:**
1. Lexer: emit `FStringStart`, `FStringText`, `FStringExprStart`, `FStringExprEnd`, `FStringEnd` token sequence
2. Parser: assemble f-string into `FString { parts: Vec<FStringPart> }` where parts are either Text or Expression
3. Format specifier parsing: after `:` inside `{}`, parse format spec
4. Codegen: emit a series of `printf`-style calls or build a buffer; handle each format spec correctly for primitives
5. Runtime: helpers for formatting (or rely on `snprintf` for simple cases)

**Verification:**

```hilow
// fstrings.hl
high program(): i32 {
  let name = "Alice"
  let age = 30
  print(f"Hello {name}! You are {age} years old.")
  
  let pi = 3.14159
  print(f"Pi: {pi:.2f}")
  
  let n = 255
  print(f"Hex: {n:x}, Bin: {n:b}, Padded: {n:08d}")
  
  print(f"|{name:>15}|")
  print(f"|{name:<15}|")
  
  return 0
}
```

```bash
hilowc fstrings.hl -o fs && ./fs
# Output:
# Hello Alice! You are 30 years old.
# Pi: 3.14
# Hex: ff, Bin: 11111111, Padded: 00000255
# |          Alice|
# |Alice          |
```

---

## Phase 7: Flexible Objects and Closures (High mode)

**Goal:** High-mode object system with prototype delegation, plus closure support.

### Phase 7a: Object Literals and Property Access

**Scope:**
- Object literal syntax: `{ x: 10, y: 20 }`
- Object type in type system: `object` (general flexible object) and shape inference for known structures
- Property access: `obj.prop`, `obj["dynamic"]`
- Property assignment: `obj.prop = value`
- Object representation in C: struct with hashmap for dynamic properties

**Out of scope:**
- Prototype delegation — Phase 7b
- Closures — Phase 7c
- Iteration (`for-in`) — Phase 7c
- Match on types — Phase 7c

**Tasks:**
1. AST: `ObjectLiteral { fields: Vec<(String, Expression)> }`, `MemberAccess { obj, prop }`, `IndexAccess { obj, index }`
2. Parser: parse object literals with `{ key: value, ... }` syntax
3. Parser: dot notation for member access; `[]` for dynamic indexing (string keys)
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
  
  let key = "x"
  print(point[key])            // 10
  
  return 0
}
```

```bash
hilowc objects.hl -o objs && ./objs
# Output: 10\n20\n30\n10
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

### Phase 7c: Closures, For-In, Match

**Scope:**
- Closures: function expressions that capture variables from enclosing scope
- Closure representation: function pointer + captured environment struct
- `for-in` iteration over arrays and objects
- `match` expressions with literal patterns, range patterns, type patterns (`is unknown`, `is nothing`, `is i32`), and guards (`when`)
- `switch` statements

**Out of scope:**
- Closure capture-by-reference vs by-value semantics — for now, all captures are by reference (this matters most for Low mode where it's restricted; Phase 12 enforces)
- Pattern matching with object destructuring — keep patterns simple here

**Tasks:**
1. Closures: at codegen, generate a struct with captured variables; the closure value is a `(fn_ptr, env_ptr)` pair
2. When a closure escapes (returned or stored), it must heap-allocate its environment — this connects to Phase 8 (memory). For now, allocate with `malloc` and don't worry about freeing; Phase 8 will replace this with refcounting.
3. `for (let item in array)`: codegen as a counted loop
4. `for (let (k, v) in object)`: iterate the hashmap
5. `match`: codegen as a chain of if/else, evaluating each pattern in order
6. `switch`: codegen as C switch when discriminator is integer or string-comparable; otherwise as if/else chain

**Verification:**

```hilow
// closures.hl
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
// for_in.hl
high program(): i32 {
  let arr = [10, 20, 30]
  for (let v in arr) {
    print(v)                   // 10, 20, 30
  }
  
  for (let (i, v) in arr) {
    print(i)
    print(v)                   // 0,10, 1,20, 2,30
  }
  
  return 0
}
```

```hilow
// matching.hl
high program(): i32 {
  let x = 42
  match x {
    0 => print(0),
    1..10 => print(1),
    11..100 => print(2),       // matches: 42 is in 11..100
    _ => print(3)
  }
  
  match x {
    n when n < 0 => print(10),
    n when n ?= 0 => print(11),
    n when n > 0 => print(12)  // matches
  }
  return 0
}
```

```bash
hilowc closures.hl -o closures && ./closures
# Output: 1\n2\n3

hilowc for_in.hl -o fi && ./fi
# Output as expected

hilowc matching.hl -o m && ./m
# Output: 2\n12
```

---

## Phase 8: Memory Model

**Goal:** Implement scope-based ownership, with implicit refcounting for High-mode escaped values.

### Phase 8a: Scope-Based Ownership for Stack Values

**Scope:**
- Variables of fixed-layout types (primitives, fixed-size arrays, fixed structs) live on the stack and are auto-cleaned at scope exit
- `defer` statement
- Move semantics for return values

**Out of scope:**
- Heap allocation — Phase 8b
- Refcounting for escaped values — Phase 8c
- Low-mode `manual`/`arena`/`shared` — Phase 12

**Tasks:**
1. Codegen: stack-allocated values map directly to C local variables
2. `defer expr` codegen: collect deferred expressions, emit them in reverse order at scope exit (including early returns and `break`)
3. Move semantics: when a value is returned from a function, it's copied (for primitives) or moved (for heap-owning values, which 8b will introduce)

**Verification:**

```hilow
// defer.hl
high program(): i32 {
  print(1)
  defer print(2)             // runs at scope exit
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
- `time` type (i64 nanoseconds since epoch)
- `duration` type (i64 nanoseconds)
- Duration literals: `2h`, `30m`, `15s`, `500ms`, `250us`, `100ns`, `1d`
- Arithmetic: `time + duration`, `time - time = duration`, `duration + duration`
- `time.now()` and `time.parse(string)`
- Calendar operations: `.year()`, `.month()`, `.day()`, `.hour()`, `.minute()`, `.second()`, `.dayOfWeek()`
- `.next(.tuesday)`, `.month().nthWeekday(2, .tuesday)`, `.month().end()`
- F-string formatting: `f"{now:YYYY-MM-DD HH:mm:ss}"`
- Qualified equality: `(same-day)=`, `(within: duration)=`

**Tasks:**
1. Type system: add `time` and `duration` types
2. Lexer: duration literals (number followed by unit suffix)
3. Runtime: time and duration are i64; helpers for parsing, formatting, calendar
4. Codegen for arithmetic operators on time/duration
5. F-string format spec for time
6. `(same-day)=` and `(within: ...)=` qualifier handlers for time

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

### Phase 10a: Basic Watch

**Scope:**
- `watch(var) { ... }` syntax: registers a callback that fires when `var` is assigned
- `watch(var1, var2, ...) { ... }`: multi-variable watch
- Watch handle: `.pause()`, `.resume()`, `.end()`
- No self-triggering: modifications inside the watch body don't re-fire

**Out of scope:**
- `async` blocks — Phase 10b
- `shared` cross-process watches — Phase 10b
- Circular dependency detection — Phase 15

**Tasks:**
1. AST: `WatchExpr { vars, body }` returns a watch handle
2. Codegen: assignments to watched variables are intercepted — they update the value and then fire registered watch callbacks
3. Watch handle: a struct with active/paused state and an end flag
4. Self-triggering prevention: a "currently executing" flag prevents re-entry

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

### Phase 12b: Manual, Arena, Shared Memory Modes

**Scope:**
- `manual let buf = alloc(N)`: explicit allocation, programmer frees
- `defer free(buf)` for manual cleanup
- `arena { ... }`: bulk allocation block, all freed at end
- `shared let res = rc_alloc<T>()`: refcounted in low mode (opt-in)
- `alloc()`, `free()`, `arena.alloc()`, `rc_alloc<T>()` runtime functions

**Tasks:**
1. Parser: `manual`, `arena`, `shared` keywords in let declarations
2. Runtime: `alloc`, `free`, arena allocator with bulk free, refcounted alloc
3. Codegen: `manual` skips automatic cleanup; `arena` blocks set up an arena and tear it down at end; `shared` uses refcounted allocator

**Verification:**

```hilow
// memory_modes.hl
low program(): i32 {
  manual let buf = alloc(1024)
  defer free(buf)
  
  arena {
    let a = arena.alloc(100)
    let b = arena.alloc(200)
    // a, b freed at end of arena block
  }
  
  return 0
}
```

Run under valgrind to confirm no leaks.

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

**Goal:** Optional proof system for constraints, contracts, and safety properties.

This is the largest phase by complexity. The proof system uses an SMT solver (Z3 via Rust bindings) to verify properties. Each sub-phase introduces one class of verification.

### Phase 15a: Variable Constraints

**Scope:**
- Constraint syntax on `let`: `let x: i32 (x >= 0 and x <= 100) = 50`
- Range sugar: `let x: i32 in 1..100 = 50`
- `--prove` compiler flag enables verification
- Verify constraints hold at every assignment

**Tasks:**
1. Add Z3 dependency
2. Parser: constraint clause on let declarations
3. Verifier: at each assignment to a constrained variable, check that the new value satisfies the constraint (using surrounding control flow as context)

**Verification:**

```hilow
// constraints_ok.hl - --prove should succeed
high program(): i32 {
  let percent: i32 in 1..100 = 50
  percent = 75                      // OK
  return 0
}
```

```hilow
// constraints_bad.hl - --prove should fail
high program(): i32 {
  let percent: i32 in 1..100 = 50
  percent = 150                     // Proof error
  return 0
}
```

### Phase 15b: Function Contracts

**Scope:**
- `requires (cond)` and `ensures (cond)` on functions
- `result` is the named return value in `ensures`
- Verifier: at each call, check `requires`. At end of function, check `ensures`.

**Tasks:**
1. Parser: `requires` and `ensures` clauses
2. Verifier: contract verification with Z3

**Verification:**

```hilow
// contract_ok.hl
high program(): i32 {
  function divide(a: i32, b: i32): i32
    requires (b ?!= 0)
  {
    return a / b
  }
  
  let x = divide(10, 5)             // OK
  
  let d = 5
  if (d ?!= 0) {
    let y = divide(10, d)           // OK (control flow tells prover)
  }
  
  return 0
}
```

### Phase 15c: Memory and Bounds

**Scope:**
- Verify no use-after-free
- Verify no double-free
- Verify no leaks in `manual` blocks
- Verify array bounds at compile time when possible

**Tasks:**
1. Track allocation/free pairs
2. Verify with Z3 that all paths through a function have correct memory discipline

### Phase 15d: Watch and Type Safety

**Scope:**
- Verify no circular watch dependencies
- Verify `unknown` returns are checked before use
- Verify currency consistency in `money` arithmetic

**Tasks:**
1. Watch dependency graph: build graph, check for cycles
2. Unknown handling: track which variables may hold unknown; require check before use
3. Currency tracking: enforce same-currency at compile time when types are concrete

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
