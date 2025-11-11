# HiLow Programming Language

HiLow is a compiled programming language that bridges systems programming and application development. It combines JavaScript's pragmatic ergonomics with formal verification, explicit memory management, and systems-level control.

## Current Status

**Phase 1 Complete!** The compiler now supports:

- ✅ Lexer with all basic tokens
- ✅ Parser for core language constructs
- ✅ Code generation (via C backend)
- ✅ Functions with parameters and return types
- ✅ Variables with type annotations
- ✅ Arithmetic and comparison operators
- ✅ Control flow (if/else, while loops, for loops)
- ✅ Function calls
- ✅ Integer types (i8, i16, i32, i64, i128, u8, u16, u32, u64, u128)
- ✅ Floating point types (f32, f64)
- ✅ Boolean type
- ✅ String literals
- ✅ Basic type inference

## Building the Compiler

```bash
cargo build --release
```

## Usage

Compile a HiLow program:

```bash
./target/release/hilowc examples/hello.hl
```

With options:

```bash
# Specify output file
./target/release/hilowc program.hl -o myprogram

# Show tokens (lexer output)
./target/release/hilowc program.hl --print-tokens

# Show AST (parser output)
./target/release/hilowc program.hl --print-ast

# Set optimization level (0-3)
./target/release/hilowc program.hl -O2
```

## Example Programs

### Hello World

```hilow
function main(): i32 {
  print("Hello from HiLow!");
  return 0;
}
```

### Fibonacci

```hilow
function fib(n: i32): i32 {
  if (n < 2) {
    return n;
  }
  return fib(n - 1) + fib(n - 2);
}

function main(): i32 {
  let result: i32 = fib(10);
  return result;
}
```

### Loops

```hilow
function main(): i32 {
  let sum: i32 = 0;
  let i: i32 = 0;

  while (i < 10) {
    sum = sum + i;
    i = i + 1;
  }

  return sum;
}
```

## Language Features

### Types

- **Integers**: `i8`, `i16`, `i32`, `i64`, `i128` (signed)
- **Unsigned**: `u8`, `u16`, `u32`, `u64`, `u128`
- **Floats**: `f32`, `f64`
- **Boolean**: `bool`
- **Strings**: `string`

### Operators

- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Comparison**: `<`, `>`, `<=`, `>=`, `?=` (equal), `??=` (strict equal)
- **Logical**: `and`, `or`, `not`
- **Bitwise**: `&`, `|`, `^`, `~`, `<<`, `>>`

### Control Flow

- `if` / `else if` / `else`
- `while` loops
- `for` loops
- `return` statements

### Functions

```hilow
function name(param: type, ...): return_type {
  // body
}
```

## Project Structure

```
HiLow/
├── src/
│   ├── main.rs          # Compiler entry point
│   ├── lexer/           # Tokenization
│   ├── parser/          # Parsing to AST
│   ├── ast/             # Abstract syntax tree definitions
│   └── codegen/         # Code generation (C backend)
├── examples/            # Example HiLow programs
├── tests/               # Test suite
├── hilow-design.md      # Complete language specification
├── hilow-development-roadmap.md  # Development plan
└── CLAUDE.md            # Guidance for AI assistants
```

## Development Roadmap

See [hilow-development-roadmap.md](hilow-development-roadmap.md) for the complete 18-phase development plan.

**Completed:**
- ✅ Phase 0: Project Foundation
- ✅ Phase 1: Minimal Viable Compiler

**Next Steps:**
- Phase 2: Complete Type System
- Phase 3: Control Flow and Operators
- Phase 4: String System (quote recursion, f-strings)
- Phase 5: Objects and Prototypes

## Language Design

See [hilow-design.md](hilow-design.md) for the complete language specification, including:

- Prototype-based objects (no classes)
- Quote recursion for strings
- F-strings (Python-style)
- Watch system for reactive programming
- First-class time and money types
- Optional formal verification
- Explicit memory management

## Requirements

- Rust 1.70+ (for building the compiler)
- GCC (for compiling generated C code)

## Testing

Run the test suite:

```bash
cargo test
```

Try the examples:

```bash
./target/debug/hilowc examples/hello.hl && ./examples/hello
./target/debug/hilowc examples/fibonacci.hl && ./examples/fibonacci
./target/debug/hilowc examples/loop.hl && ./examples/loop
```

## Contributing

HiLow is in early development. We're currently focusing on Phase 1-2 of the roadmap.

Key areas for contribution:
1. Test coverage
2. Error messages
3. Type system completion
4. Standard library design

## License

To be determined

## Credits

Designed and implemented as a bridge between systems and application programming paradigms.
