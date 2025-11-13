# HiLow Illustration Programs

This directory contains practical example applications that demonstrate HiLow's capabilities for real-world use cases.

## Applications

### 1. CSV Parser (`csv_parser.hl`)

**Demonstrates**: String manipulation, arrays, closures, defer

A CSV data processor that shows how to:
- Split strings into arrays with `.split()`
- Use closures to capture state
- Process data with `.forEach()`
- Use defer for cleanup messages

**Features showcased**:
- `string.split("\n")` for line splitting
- `string.split(",")` for field splitting
- Closures capturing and mutating variables (`total_quantity`)
- `defer` statement executing at function end
- F-string formatting for output

**Run**:
```bash
../target/release/hilowc csv_parser.hl -o csv_parser
./csv_parser
```

### 2. Calculator with Variables (`calculator.hl`)

**Demonstrates**: Closures, pattern matching, stateful functions

A calculator that maintains memory using closures:
- Store values in closure state
- Operations that modify captured variables
- Pattern matching for operation selection
- Multiple closures sharing state

**Features showcased**:
- Closures with mutable captured variables
- State persistence across function calls
- Match expressions for operation selection
- Multiple closures accessing same captured variable
- Defer for session cleanup

**Run**:
```bash
../target/release/hilowc calculator.hl -o calculator
./calculator
```

### 3. Configuration Parser (`config_parser.hl`)

**Demonstrates**: Objects, match expressions, property access

A configuration management system showing:
- Object literals for configuration
- Property access and modification
- Pattern matching for validation
- Type-safe configuration handling

**Features showcased**:
- Object literals: `{port: 8080, timeout: 30, ...}`
- Property access: `config.port`
- Property mutation: `config.port = 443`
- Match expressions for mapping values
- F-strings for formatted output
- Defer for completion messages

**Run**:
```bash
../target/release/hilowc config_parser.hl -o config_parser
./config_parser
```

## Why These Examples Matter

These applications are not toy programs - they demonstrate patterns used in real software:

1. **CSV Parser** - Text processing, data transformation
   - Real use: Log analysis, data import, report generation

2. **Calculator** - State management with closures
   - Real use: Interactive tools, state machines, accumulators

3. **Config Parser** - Data structures and validation
   - Real use: Application configuration, settings management

## HiLow Features Highlighted

Across these three applications, you can see:

- **Closures with capture**: Variables automatically captured and mutated
- **Defer statements**: Guaranteed cleanup/finalization
- **Pattern matching**: Type-safe value-based dispatch
- **String methods**: split, trim, and other transformations
- **Array methods**: forEach for iteration
- **Objects**: Flexible data structures without classes
- **F-strings**: Clean, readable output formatting
- **Type safety**: Strong typing with inference

## Building All Illustrations

From this directory:
```bash
# Build all
../target/release/hilowc csv_parser.hl -o csv_parser
../target/release/hilowc calculator.hl -o calculator
../target/release/hilowc config_parser.hl -o config_parser

# Run all
./csv_parser
./calculator
./config_parser
```

## Extending These Examples

These are starting points. You could extend them to:

**CSV Parser**:
- Add filtering by column values
- Aggregate numeric columns
- Sort by fields
- Export filtered results

**Calculator**:
- Add more operations (multiply, divide, power)
- Multiple memory slots
- History of calculations
- Expression evaluation

**Config Parser**:
- Parse from actual config file format
- Validate ranges and types
- Default values for missing fields
- Nested configuration objects

## License

These example programs are part of the HiLow project and licensed under GNU GPL v2.
