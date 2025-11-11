# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

HiLow is a compiled programming language that bridges systems programming and application development. It combines JavaScript's pragmatic ergonomics with formal verification, explicit memory management, and systems-level control.

**Current Status**: Pre-implementation phase. The repository contains design documentation and roadmap only. No compiler code has been written yet.

## Key Design Principles

- **Pragmatic correctness**: Optional formal verification through constraints and contracts
- **Prototype-based objects**: No classes, pure prototype delegation (JavaScript-inspired)
- **Explicit memory management**: Stack/heap allocation with scope-based lifetime
- **Reactive programming**: The `watch()` primitive for event-driven code
- **Type safety with coercion**: Strong typing that coerces naturally where safe
- **Application-friendly**: First-class types for time, money, and error handling

## Language Philosophy

HiLow targets the gap between systems languages (C, Rust, Zig) and application languages (JavaScript, Python). The goal is to write everything from device drivers to web applications with one language.

### Unique Language Features

1. **Quote Recursion**: No escape sequences for quotes. Use multiple adjacent quotes to nest: `""text "with" quotes""`
2. **F-Strings**: Python-style interpolation without backticks: `f"Value: {x}"`
3. **Watch System**: Reactive programming primitive for event-driven code
4. **Unknown Type**: Rich error information with `.reason` and `.options` properties
5. **Nothing Type**: Represents true absence/uninitialized/deallocated
6. **First-class Time**: `time` type with calendar operations and duration literals (1d, 2h, 30m)
7. **First-class Money**: `money` type with currency-safe arithmetic
8. **Natural Operators**: `and`, `or`, `not` instead of `&&`, `||`, `!`
9. **Equality Operators**: `?=` (with coercion), `??=` (strict), `!=`, `!!=`

## Project Structure

```
HiLow/
├── hilow-design.md              # Complete language specification
└── hilow-development-roadmap.md  # 18-phase development plan
```

## Development Roadmap

The project follows an 18-phase plan from Phase 0 (Foundation) to Phase 18 (Stabilization). Each phase has validation checkpoints before proceeding. Estimated timeline: 18-30 months to 1.0 release.

### Critical Phases

1. **Phase 0**: Project foundation and infrastructure
2. **Phase 1**: Minimal Viable Compiler (Hello World, basic types, control flow)
3. **Phase 8**: Special types (nothing, unknown, time, money)
4. **Phase 9**: Watch system (reactive programming)
5. **Phase 10**: Formal verification (prover system)

## Compilation Target

The compiler will:
- Target LLVM IR for code generation
- Produce native executables with no runtime
- Support cross-platform compilation
- Include optional formal verification pass (`--prove` flag)

## Testing Philosophy

- Write tests before features when possible (TDD)
- Maintain >90% code coverage
- Every bug gets a regression test
- Don't move to next phase until validation checkpoint passes
- 100% test pass rate required before phase progression

## Key Technical Decisions

### String System
- Only double quotes (no single quotes or backticks)
- Quote recursion instead of escape sequences
- F-strings use `f"text {expr}"` syntax (Python-style)
- Raw strings: `r"text"` or `rf"text {expr}"`

### Memory Management
- Explicit `stack` and `heap` keywords
- Manual deallocation via `= nothing`
- `defer` statement for automatic cleanup at scope exit
- No garbage collection, no reference counting

### Type System
- Explicit integer sizes: i8, i16, i32, i64, i128, u8, u16, u32, u64, u128
- Floating point: f32, f64
- Type inference for local variables
- No implicit type coercion in assignments, but coercion in operators where safe

### Error Handling
- No exceptions
- Functions return `T | unknown` for errors
- `unknown` type has `.reason` and `.options` properties
- `unknown` propagates through property access
- Both `nothing` and `unknown` are falsy

### Object Model
- Prototype-based (like JavaScript)
- Objects have explicit `proto` property
- No classes, no inheritance keywords
- Method dispatch through prototype chain

### Watch System (Reactive Programming)
- `watch(var1, var2) { ... }` syntax
- Returns handle with `.pause()`, `.resume()`, `.end()`, `.isActive()`
- No self-triggering (modifications inside watch don't re-trigger)
- Works across async processes with `shared` variables

### Formal Verification
- Variable constraints: `let x: i32 (x >= 0 and x < 100) = 50;`
- Function contracts: `requires` and `ensures` clauses
- Compiler prover verifies at compile time
- Optional via `--prove` flag
- Catches: constraint violations, unchecked unknowns, circular watch dependencies, memory errors, array bounds

## Important Syntax Notes

### Function Declarations
```hilow
function add(a: i32, b: i32): i32 {
  return a + b;
}
```
- Full `function` keyword (not `fn`)
- Explicit types for parameters
- Return type required for exports/contracts

### Control Flow
- `if`/`else if`/`else` (standard)
- `while` loops
- `for` loops (C-style, iterator, key-value)
- `switch` with explicit `break`
- `match` for pattern matching (with ranges, guards, wildcards)

### Module System
- Named exports only: `export function foo() { }`
- Named imports only: `import { foo, bar } from "./module";`
- No default exports, no namespace imports, no dynamic imports

## When Starting Implementation

### Phase 0 Checklist
1. Choose implementation language (likely Rust, C++, or Zig)
2. Set up lexer tests before implementing lexer
3. Create formal grammar in EBNF
4. Set up LLVM toolchain integration
5. Establish test runner infrastructure
6. Define AST node structures

### Lexer First Tokens
- Keywords: `function`, `let`, `if`, `else`, `while`, `for`, `return`, `export`, `import`
- Operators: `+`, `-`, `*`, `/`, `=`, `?=`, `??=`, `!=`, `!!=`, `<`, `>`, `<=`, `>=`
- Literals: integers, strings (with quote recursion)
- Identifiers and comments

### Critical Implementation Notes
1. **Quote Recursion**: Lexer must count adjacent quotes to determine string boundaries
2. **F-Strings**: Lexer must parse expressions inside `{...}` in f-strings
3. **Watch System**: Requires runtime support for change detection
4. **Prover**: Separate compilation pass, uses SMT solver or custom constraint solver
5. **Money Type**: Store as integer with currency enum (display decimals + 4 precision)
6. **Time Type**: Store as i64 nanoseconds since epoch

## Documentation References

All language features are fully specified in `hilow-design.md`. The development sequence is detailed in `hilow-development-roadmap.md`.

## Validation Before Phase Transitions

Each phase has a "Validation Checkpoint" with specific programs that must work before proceeding:
- Phase 1: Hello World, Fibonacci, Factorial
- Phase 3: Game of Life, Calculator
- Phase 8: Time-based app, Financial calculations
- Phase 9: Event-driven server, Concurrent counter
- Phase 10: Verified banking system

Never skip validation checkpoints.
