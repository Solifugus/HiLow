# HiLow Programming Language

HiLow is a compiled programming language with two modes: **High** for application development and **Low** for systems programming. Both modes share syntax and most semantics; they differ in which features are available and what the compiler enforces.

## Current Status

**Fresh Implementation in Progress** - This is a complete rewrite following the new HiLow design specification.

### Phase 0: Complete ✅
- ✅ Project setup and architecture
- ✅ Basic CLI structure
- ✅ Build system (Rust + Cargo)

### Upcoming Phases
- Phase 1: Lexer foundation (basic tokens, equality operators)
- Phase 2: Parser foundation (program structure, expressions, statements)  
- Phase 3: AST and basic type system
- Phase 4: High mode core (compilation, codegen)
- And 13 more phases through full language implementation...

## Language Design

This implementation follows the complete specification in [`docs/hilow-design.md`](docs/hilow-design.md), featuring:

### Two-Mode System
- **High Mode**: Application development with flexible objects, type inference, automatic memory management
- **Low Mode**: Systems programming with pointers, explicit memory modes, inline assembly

### Key Features (Planned)
- No type coercion (explicit conversions only)
- Equality operators: `?=` (strict), `~=` (approximate), `is` (type test)
- First-class types: `time`, `money`, `nothing`, `unknown`
- Reactive programming with `watch()` primitive
- Scope-based ownership with refcounting
- F-strings with quote recursion
- Optional formal verification
- Compiles to native code (no GC, no runtime)

## Building the Compiler

Currently builds a stub compiler:

```bash
cargo build
./target/debug/hilowc example.hl
# Output: HiLow compiler v0.1 (stub)
```

## Development Plan

See [`docs/development-plan.md`](docs/development-plan.md) for the complete 18-phase implementation plan. This is a systematic rebuild designed for clarity and correctness.

**Development Approach:**
- Phase-by-phase implementation
- Working compiler at every phase  
- Comprehensive verification at each step
- Test-driven development

## Example (Future)

```hilow
high program(args: [string]): i32 {
  let name = "HiLow"
  print(f"Hello from {name}!")
  return 0
}
```

## Project Structure

```
HiLow/
├── CLAUDE.md                    # AI assistant guidance
├── Cargo.toml                   # Rust project configuration
├── docs/
│   ├── hilow-design.md         # Complete language specification
│   └── development-plan.md     # 18-phase implementation plan
├── src/
│   └── main.rs                 # Compiler CLI (stub)
└── tests/
    ├── programs/               # HiLow test programs
    └── expected/               # Expected outputs
```

## Requirements

- Rust 1.70+ (for building the compiler)
- GCC (for compiling generated C code in later phases)

## Contributing

This is the early stages of a systematic language implementation. The design is stable and documented. Contributors welcome for:

1. Implementation of planned phases
2. Test case development
3. Documentation improvements
4. Standard library design

Please follow the phase-by-phase plan and maintain test coverage.

## License

HiLow is licensed under the GNU General Public License v2.0. See [LICENSE](LICENSE) for the full license text.

## Credits

**Author**: Matthew C. Tedder (Solifugus)  
**License**: GNU GPL v2  
**Repository**: https://github.com/Solifugus/HiLow

Fresh implementation designed for clarity, correctness, and systematic development.