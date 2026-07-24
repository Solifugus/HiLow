# HiLow Programming Language Design

HiLow is a compiled programming language with two modes: **High** for application development and **Low** for systems programming. Both modes share the same syntax, operators, and core semantics — they differ in which features are available and what the compiler enforces. A single program can mix both modes naturally, dropping into Low for performance-critical sections or hardware access while staying in High for everything else.

## Design Principles

- **One language, two modes**: High and Low share syntax and most semantics; mode determines which features are available
- **JS-comfort, systems-power**: Application code feels like JavaScript; systems code has C-level control
- **No type coercion**: Strong typing without implicit conversions in either mode
- **No runtime, no GC**: Both modes compile to native code with predictable execution
- **Explicit reactive primitive**: `watcher` for event-driven and concurrent code in both modes
- **Pragmatic correctness**: Optional formal verification through constraints and contracts
- **First-class application types**: `time` and `money` are built in, not library afterthoughts

## Philosophy

HiLow targets the gap between systems languages (C, Rust, Zig) and application languages (JavaScript, Python, Go). Most languages pick one side. HiLow lets a single project span both:

- **Low mode**: Direct memory access, inline assembly, fixed-layout structs, no flexible objects, mandatory type annotations on function signatures, scope-based ownership with explicit memory mode keywords (`manual`, `arena`, `shared`).
- **High mode**: Flexible prototype-based objects, full type inference, implicit reference counting for escaped values, closures that capture freely, reflection.

Both modes compile to the same execution model — no runtime, no GC pauses — making High suitable for everything from embedded systems to web servers, while Low handles drivers, codecs, and hot paths within High applications.

## Hello, HiLow World!

```hilow
high program(args: [string]): i32 {
  print("Hello, HiLow World!")
  return 0
}
```

A HiLow program is wrapped in a `program` block that declares its default mode. Everything inside `high program` defaults to High mode. A systems program would use `low program` instead. There is no `main()` function — the program *is* the entry point.

## Modes

### Mode Declaration

A HiLow source file is one of:

- A **program** with `high program(...)` or `low program(...)` — the entry point
- A **module** with `high module { ... }` or `low module { ... }` — a library

```hilow
// app.hl - an entry point
high program(args: [string]): i32 {
  print("Application code here")
  return 0
}
```

```hilow
// codec.hl - a library
low module {
  export function fastEncode(input: *u8, len: usize): *u8 {
    // Low-mode implementation
  }
  
  export function fastDecode(input: *u8, len: usize): *u8 {
    // Low-mode implementation
  }
}
```

### Mode Inheritance

Mode flows from the enclosing scope to inner scopes unless explicitly overridden. The rule is uniform across three nesting levels:

1. **Program/module level**: Sets the default for everything inside
2. **Function level**: Can override the program/module default
3. **Block level**: Can override the function default

```hilow
high program(): i32 {
  // Inherits high mode from program declaration
  
  function processRequest(data: bytes): Response {
    // Inherits high mode from program
  }
  
  low function fastEncode(input: *u8, len: usize): *u8 {
    // Function-level override to low mode
  }
  
  function processData(input: object): Response {
    let parsed = json.parse(input.body)  // high
    
    low {
      // Block-level override to low mode
      let buf: [u8; 4096]
      // ... tight loop with explicit memory ...
    }
    
    return { status: 200, body: parsed }
  }
  
  return 0
}
```

### Calling Across Modes

A High function can call a Low function freely. The two share calling conventions, primitive types, and execution model:

```hilow
high program(): i32 {
  let data = readInput()                  // high
  let encoded = fastEncode(data, data.length)  // calls low function
  print(f"Encoded {encoded.length} bytes")
  return 0
}

low function fastEncode(input: *u8, len: usize): *u8 {
  // ... low implementation ...
}
```

A Low function can call a High function only if that function is marked as low-callable, meaning it uses no High-only features (flexible objects, implicit refcounting, reflection):

```hilow
@low-callable
high function computeChecksum(data: bytes): u32 {
  // Restricted to features Low can call into
  let sum: u32 = 0
  for (let i = 0; i < data.length; i += 1) {
    sum = sum + data[i]
  }
  return sum
}

low function processBlock(block: *u8, len: usize) {
  let crc = computeChecksum(slice(block, len))  // OK - marked low-callable
}
```

The compiler verifies the `@low-callable` annotation: if the function body uses any High-only feature, compilation fails with a clear error.

### What Each Mode Provides

**Shared between High and Low** (the core language):
- All primitive types, `time`, `money`, `nothing`, `unknown`
- All operators including `?=`, `!=`, `is`, `(qualifier)=`
- All control flow constructs
- F-strings and quote recursion
- `watcher` reactive primitive
- Pattern matching and destructuring
- Module imports/exports
- Constraints and function contracts (`requires`/`ensures`)
- Fixed-layout structs

**High mode adds**:
- Flexible prototype-based objects with dynamic property access
- Type inference on function signatures (not just locals)
- Implicit reference counting for escaped values
- Closures that capture freely
- Reflection on objects (iterating keys, dynamic property access)
- Standard library: HTTP, JSON, file I/O, networking

**Low mode adds**:
- Pointer types (`*T`, `**T`)
- Pointer arithmetic
- Inline assembly (`asm { ... }`)
- Memory mode keywords: `manual`, `arena`, `shared`
- Explicit struct memory layout (`@packed`, `@align(N)`)
- Bit-level field control

**Low mode restricts**:
- No flexible objects (only fixed structs)
- No implicit refcounting (scope-based ownership only)
- Function signatures must have explicit type annotations
- No reflection
- No closures that escape their defining scope (closures usable but not stored on heap)

### Why This Split

The split reflects a fundamental difference in what the two modes optimize for. High mode optimizes for **developer comfort** — type inference reduces ceremony, flexible objects make data manipulation easy, refcounting handles ownership without manual tracking. Low mode optimizes for **predictability and control** — every operation is explicit, every allocation is visible, every cost is knowable.

Both serve real needs. HiLow lets you have both in one project.

## Lexical Structure

### Comments

```hilow
// Single-line comment

/* Multi-line
   comment */

/// Documentation comment (extracted by tooling)
```

### Identifiers

Identifiers begin with a letter or underscore, followed by letters, digits, or underscores. Convention is `camelCase` for variables and functions, `PascalCase` for type names.

### Keywords

```
and       arena     async     break     case      continue
decreases default   defer     else      ensures   excluding
export    false     for       from      function  heap
high      if        import    in        invariant is
let       loop      low       manual    match     module
not       nothing   or        program   requires  return
shared    stack     stealth   switch    this      true
unknown   watcher   when      while
```

### Reserved for Future Use

```
class     interface  trait     yield     enum
```

## Type System

### Primitive Types

```hilow
// Integers - explicit sizes
let a: i8 = -128            // 8-bit signed
let b: u8 = 255             // 8-bit unsigned
let c: i16 = -32768         // 16-bit signed
let d: u16 = 65535          // 16-bit unsigned
let e: i32 = -2147483648    // 32-bit signed
let f: u32 = 4294967295     // 32-bit unsigned
let g: i64                  // 64-bit signed
let h: u64                  // 64-bit unsigned
let i: i128                 // 128-bit signed
let j: u128                 // 128-bit unsigned

// Pointer-sized integers
let s: isize                // Signed pointer-size
let u: usize                // Unsigned pointer-size

// Floating point
let x: f32 = 3.14           // 32-bit float
let y: f64 = 2.71828        // 64-bit double

// Boolean
let flag: bool = true

// Strings (UTF-8 by default)
let name: string = "Alice"
```

### Type Inference

In **High mode**, type inference applies everywhere — locals, function parameters, function returns:

```hilow
high program(): i32 {
  let count = 42                    // Inferred: i32
  let pi = 3.14159                  // Inferred: f64
  let name = "Alice"                // Inferred: string
  
  function double(x) {              // Parameter and return inferred from body
    return x * 2
  }
  
  let result = double(21)           // Inferred: i32
  return 0
}
```

In **Low mode**, type inference applies to locals only — function signatures must be explicit:

```hilow
low module {
  export function double(x: i32): i32 {  // Signature explicit
    let result = x * 2                    // Local inferred from body
    return result
  }
}
```

### No Type Coercion

Neither mode performs implicit type coercion. This is a deliberate choice for predictability:

```hilow
let s = "5"
let n = 2

// Both modes: these are errors
s + n         // ✗ Cannot add string and i32
s * n         // ✗ Cannot multiply string and i32
5 ?= "5"      // ✗ Cannot compare i32 and string for equality

// Explicit conversion required
let parsed = parseInt(s)
parsed + n    // ✓ i32 + i32

// String formatting uses f-strings, not concatenation
f"{s} and {n}"  // ✓ "5 and 2"
```

This differs from JavaScript intentionally. F-strings handle the cases where coercion would have been used for formatting; explicit parsing handles the cases where conversion is genuinely needed.

### Special Types

#### Nothing

`nothing` represents true absence — uninitialized variables, missing properties, deallocated memory.

```hilow
let x                       // x is nothing
let y = nothing             // Explicit nothing

let obj = { a: 1 }
obj.b                       // nothing - property doesn't exist

// Type checking with `is`
if (x is nothing) {
  print("x has no value")
}

// Falsy in conditions
if (not x) {
  print("x is nothing or other falsy")
}
```

#### Unknown

`unknown` carries rich error information — the reason for failure and suggested options.

```hilow
// Creating unknown values
function divide(a: i32, b: i32): i32 | unknown {
  if (b ?= 0) {
    return unknown("division by zero", options: ["use different divisor"])
  }
  return a / b
}

// Checking with `is`
let result = divide(10, 0)
if (result is unknown) {
  print(f"Error: {result.reason}")
  print(f"Options: {result.options}")
  return 1
}
print(f"Result: {result}")

// Unknown properties
unknown.reason: string      // Why it failed
unknown.options: [string]   // Possible solutions

// Function return shorthand
function getUser(id: i32): object? {  // ? suffix means "may return unknown"
  // ... implementation
}
```

The `T?` syntax is shorthand for `T | unknown`. Use the union form when clarity matters; use `?` when concise.

### First-Class Types

#### Time

The `time` type represents an instant in time, stored internally as i64 nanoseconds since epoch. `duration` represents a span of time.

```hilow
let now: time = time.now()
let birthday: time = time.parse("1990-06-15T14:30:00Z")

// Duration literals
let later = now + 2h + 30m + 15s
let tomorrow = now + 1d
let precise = now + 500ms + 250us + 100ns

// Calendar operations
let nextTuesday = now.next(.tuesday)
let secondTuesday = now.month().nthWeekday(2, .tuesday)
let endOfMonth = now.month().end()

// Comparisons
if (meeting > now and meeting < now + 1h) {
  print("Meeting within the hour")
}

// Formatting
let formatted = now.format("YYYY-MM-DD HH:mm:ss")
let iso = now.toISO()

// Duration arithmetic
let elapsed: duration = endTime - startTime
print(elapsed.hours())     // 2.5
print(elapsed.minutes())   // 150

// Iteration
for (let day = startDate; day <= endDate; day += 1d) {
  print(day.format("YYYY-MM-DD"))
}

// Timezone support
let ny = time.now(.timezone("America/New_York"))
let tokyo = ny.in(.timezone("Asia/Tokyo"))

// Domain-specific equality (qualifier form)
if (meeting1 (same-day)= meeting2) {
  print("Same day")
}
if (meeting1 (within: 1h)= meeting2) {
  print("Within an hour")
}
```

##### Time Precision

Every `time` value carries a *precision tag* indicating the smallest unit it was specified to: `year`, `month`, `day`, `hour`, `minute`, `second`, `millisecond`, `microsecond`, or `nanosecond`. The precision tag is set when the value is created:

```hilow
let t1 = time.parse("2024-01-15")              // day precision
let t2 = time.parse("2024-01-15T10:00")        // minute precision
let t3 = time.parse("2024-01-15T10:30:45.123") // millisecond precision
let t4 = time.now()                             // always nanosecond precision
```

When two times are compared (with `?=`, `!=`, `<`, `>`, `<=`, `>=`), comparison happens at the precision of the *less precise* operand. This matches how humans reason about time: "the meeting is at 10am" means "sometime in the 10am hour," not "10:00:00.000000000."

```hilow
let t1 = time.parse("2024-01-15T10:00")        // minute precision
let t2 = time.parse("2024-01-15T10:30:45")     // second precision

// Comparison happens at minute precision (the coarser of the two)
if (t1 ?= t2) { print("equal") }    // true: t2 falls within t1's minute? No - t2 is at 10:30, t1 at 10:00
if (t1 < t2) { print("t2 later") }  // true: 10:30 > 10:00 at minute precision

let t3 = time.parse("2024-01-15T10:00:30")     // second precision
if (t1 ?= t3) { print("equal") }    // true: at minute precision both are 10:00
```

Arithmetic preserves the operand's precision: `t1 + 1h` keeps `t1`'s precision tag. Adding a sub-precision duration (e.g., adding `500ms` to an hour-precision time) keeps the coarser precision; the sub-precision quantity is held in storage but ignored in comparisons until the precision tag is changed.

To force a specific precision, use `.atPrecision(.unit)`:

```hilow
let exact = t1.atPrecision(.second)            // round/truncate to second precision
let coarse = t3.atPrecision(.day)              // truncate to day
```

This precision rule applies in both High and Low modes. Low mode does not have different semantics — but Low code typically creates times via `time.now()` or equivalent precise sources, so Low values tend to be nanosecond-precise. The rule is a property of the value, not of the mode.

The qualifier forms `(same-year)=`, `(same-month)=`, `(same-day)=`, etc. are equivalent to comparing both operands `.atPrecision(.unit)`. They exist for readability — `t1 (same-day)= t2` is more obviously about day-equality than `t1.atPrecision(.day) ?= t2.atPrecision(.day)`.

#### Money

The `money` type tracks both amount and currency. Mixing currencies is a compile error.

```hilow
let price: money = 19.99 USD
let euro: money = 50.00 EUR
let yen: money = 1000 JPY

// Arithmetic (same currency only)
let total = price + 5.00 USD
let doubled = price * 2
let split = total / 3

// Currency mixing is compile error
let bad = price + euro              // ✗ Cannot add USD and EUR

// Explicit conversion
let converted = euro.convert(USD, rate: 1.08)

// Display formatting
print(price)                        // "$19.99"
print(euro.format())                // "€50.00"
print(yen.format())                 // "¥1,000"

// Internal precision: display + 4 decimal places
// USD: display 2, store 6 (19.990000)
// JPY: display 0, store 4 (1000.0000)

// Rounding modes
let tax = price * 0.08
let rounded = tax.round(.halfUp)
let bankers = tax.round(.halfEven)

// Allocation (guarantees sum equals original)
let bill = 100.00 USD
let split = bill.allocate([1, 1, 1])  // [33.34, 33.33, 33.33]

// Type-safe currency
function calculateTax(price: money<USD>, rate: f64): money<USD> {
  return price * rate
}
```

### Arrays and Collections

```hilow
// Fixed-size arrays
let fixed: [i32; 10]                // 10 integers, stack-allocated
let initialized = [1, 2, 3, 4, 5]   // Inferred [i32; 5]

// Dynamic arrays (heap-allocated)
let dynamic: [i32]                  // Growable
dynamic.push(42)
dynamic.pop()

// Iteration
for (let item in array) {
  print(item)
}

for (let (index, value) in array) {
  print(f"[{index}] = {value}")
}

// Building new arrays - explicit loops, no map/filter/reduce
let doubled = []
for (let item in array) {
  doubled.push(item * 2)
}
```

### Fixed Structs (both modes)

Fixed structs have a known memory layout. They work in both High and Low.

```hilow
struct Point {
  x: f64
  y: f64
}

let p: Point = { x: 10.0, y: 20.0 }
print(p.x)                          // 10.0

// Methods
struct Circle {
  center: Point
  radius: f64
  
  function area(): f64 {
    return PI * this.radius * this.radius
  }
}

let c: Circle = { center: { x: 0.0, y: 0.0 }, radius: 5.0 }
print(c.area())

// In Low mode: explicit layout control
@packed
struct PacketHeader {
  version: u8
  flags: u8
  length: u16
  checksum: u32
}

@align(16)
struct AlignedBuffer {
  data: [u8; 1024]
}
```

### Flexible Objects (High mode only)

In High mode, prototype-based flexible objects work like JavaScript — properties can be added dynamically, and prototypes provide delegation. Property removal is not yet implemented; property indices are append-only. When removal is later added, it will tombstone the slot — indices are never compacted or reused. (Weak-reference bookkeeping keys on `(holder, property index)` and any future slot-stability guarantees depend on this.)

```hilow
// Object literals
let point = {
  x: 10,
  y: 20,
  proto: nothing
}

// Prototype delegation
let animal = {
  proto: nothing,
  speak: function() {
    print("some sound")
  }
}

let dog = {
  proto: animal,
  name: "Rover",
  speak: function() {
    print("woof")
  }
}

dog.speak()                         // "woof"

// Property access
let value = obj.property
let computed = obj[key]             // For dynamic keys

// Adding methods at runtime
let obj = {}
obj.calculate = function(x) {
  return x * 2
}

// Iteration
for (let (key, value) in obj) {
  print(f"{key}: {value}")
}

// Type test with `is`
if (dog is animal) {                // Prototype membership check
  print("dog inherits from animal")
}
```

In Low mode, flexible objects are not available. Use fixed structs instead.

## Strings

### Quote Recursion

HiLow uses only double quotes for strings — no single quotes, no backticks. Quotes inside strings are handled by *quote recursion*: N adjacent quotes open a string, N adjacent quotes close it, and any sequence of fewer than N quotes inside is literal.

```hilow
"simple string"

""

""My name is "Joe" and I'm happy""    // Two-quote delimiter

"""He said ""hello"" to me"""           // Three-quote delimiter

// Multi-line strings
"
Line 1
Line 2
Line 3
"
```

The rule: count the leading quotes. The string ends at the first sequence of *exactly* that many quotes. This eliminates the need to escape quotes in most cases.

### F-Strings

F-strings interpolate expressions. Prefix with `f`. Python-style, no backticks.

```hilow
let name = "Alice"
let age = 30
f"Hello {name}! You are {age} years old."

// Expressions
let price = 19.99
f"Total: {price * 1.08}"

// Format specifiers
f"Price: {price:.2f}"               // "Price: 19.99"
f"Hex: {255:x}"                     // "Hex: ff"
f"Binary: {42:b}"                   // "Binary: 101010"
f"Padded: {7:04d}"                  // "Padded: 0007"

// Money formatting
let amount = 1234.56 USD
f"Total: {amount}"                  // "Total: $1,234.56"
f"Amount: {amount:.4f}"             // "Amount: $1,234.5600"

// Time formatting
let now = time.now()
f"Current: {now:YYYY-MM-DD}"
f"Time: {now:HH:mm:ss}"

// Alignment
f"|{value:>10}|"                    // Right align
f"|{value:<10}|"                    // Left align
f"|{value:^10}|"                    // Center

// F-strings with quote recursion
f""Name: "Joe", Age: {age}""

// Multi-line f-strings
f"
  Dear {name},
  
  Your balance is {amount}.
"
```

### Raw Strings

Raw strings don't process escape sequences. Prefix with `r`. Combine with `f` as `rf`.

```hilow
r"C:\Users\Alice\Documents"
r"\d+\.\d+"                         // Regex pattern

rf"Path: {userPath}\file.txt"       // {userPath} interpolates, \f is literal
```

### Escape Sequences

Used in non-raw strings for characters that quote recursion can't handle:

```hilow
"\n"           // Newline
"\t"           // Tab
"\r"           // Carriage return
"\\"           // Backslash
"\u{1F600}"    // Unicode codepoint
"\x41"         // Hex byte
```

Quote recursion is preferred over `\"` escaping:

```hilow
""contains "quotes""               // Preferred
"contains \"quotes\""              // Discouraged (still legal)
```

## Operators

### Arithmetic

```hilow
x + y          // Addition
x - y          // Subtraction
x * y          // Multiplication
x / y          // Division
x % y          // Modulo
```

No coercion — operands must be the same numeric type. Mixing `i32` and `f64` requires explicit conversion: `f64(i32_value) + f64_value`.

### Comparison

HiLow has two equality operators plus an open-ended qualifier form for unusual cases:

```hilow
// Equality - types must match exactly, no coercion
x ?= y         // Equal
x != y         // Not equal

// Type/prototype membership
x is T         // x is of type T (or has T in prototype chain)
x is not T     // x is not of type T

// Ordering
x < y          // Less than
x > y          // Greater than
x <= y         // Less than or equal
x >= y         // Greater than or equal
x !< y         // Not less than (equivalent to x >= y)
x !> y         // Not greater than (equivalent to x <= y)
```

The leading `?` in `?=` makes the equality operator visually distinct from assignment, eliminating the classic `=` vs `==` typo bug. Inequality uses the familiar `!=` since it has no assignment-confusion risk and is universal across languages. The asymmetry is intentional — `!=` is so universally recognized that requiring something like `?!=` would be needless friction.

The `!<` and `!>` operators are equivalent to `>=` and `<=` respectively, but read more naturally in invariant-style code: "x has not exceeded the limit" is `x !> limit`. Both forms are valid; choose the one whose framing matches your reasoning. The redundant forms `!<=` and `!>=` are not provided — use `>` and `<` instead.

Bare `==` is **not** a valid operator in HiLow. Using it produces a clear compile error suggesting `?=` for equality or `=` for assignment. This catches the typo in both directions: someone reaching for `==` is told to use `?=`, and someone who typed `=` in a condition position by mistake is told that's an assignment.

### Qualified Equality

Approximate, fuzzy, or domain-specific comparisons use the `(qualifier)=` form. Inequality uses `(qualifier)!=`:

```hilow
// Approximate numeric equality
if (a (roughly)= b) {
  print("Roughly equal")
}

// Numeric tolerance
if (a (within: 0.01)= b) {
  print("Close enough")
}

// Case-insensitive string comparison
if (s1 (caseless)= s2) {
  print("Match (any case)")
}

// Combined qualifiers (comma-separated)
if (s1 (caseless, trimmed)= s2) {
  print("Match ignoring case and whitespace")
}

// Time comparisons
if (m1 (same-day)= m2) {
  print("Same calendar day")
}
if (m1 (within: 1h)= m2) {
  print("Within an hour")
}

// Money comparisons
if (p1 (after-conversion: USD)= p2) {
  print("Equal value when converted to USD")
}

// Negation form
if (s1 (caseless)!= s2) {
  print("Different (even ignoring case)")
}
```

The qualifier name is interpreted by the type involved — `time` knows about `same-day` and `within`, `string` knows about `caseless` and `trimmed`, etc. Multiple qualifiers can be combined with commas; their order does not matter. Qualifier names are *contextual identifiers* — they are recognized only inside the `(...)=` position and do not pollute the keyword namespace, so a user can still name a variable `roughly` or `caseless`.

The same `(qualifier)=` form handles unusual *assignment* operations. Coercion — converting from one type to another with type-aware parsing — uses `(coerce)=`:

```hilow
let price: f64 (coerce)= "9.95"          // parse "9.95" as f64
let count: i32 (coerce)= "42"            // parse "42" as i32
let amount: money (coerce)= "$19.99"     // parse currency-formatted string
let when: time (coerce)= "2024-01-15"    // parse ISO date
```

HiLow has no implicit type coercion. The `(coerce)=` qualifier makes coercion *explicit at every point of use*. This preserves type safety while giving programmers a concise way to perform conversions when they're warranted. Each type defines what coercion means for it; if a type doesn't support coercion from a particular source type, the compiler reports an error.

Built-in qualifiers in HiLow 1.0 include:

- **Numeric**: `roughly` (default tolerance), `within: <value>` (explicit tolerance)
- **String**: `caseless`, `trimmed`
- **Time**: `same-year`, `same-month`, `same-day`, `same-hour`, `same-minute`, `within: <duration>`
- **Money**: `after-conversion: <currency>`
- **Coercion**: `coerce` (assignment qualifier; type-aware string parsing and value conversion)
- **Low-mode atomic/memory**: `atomic-add`, `atomic-sub`, `atomic-or`, `atomic-and`, `saturating-add`, `saturating-sub`, `volatile` (assignment-only)

User-defined qualifiers are not supported in HiLow 1.0; the qualifier set is fixed by the standard library. Future versions may add a registration mechanism.

### Assignment

Standard assignment uses `=`. Common arithmetic compound assignments use familiar shorthand:

```hilow
x = y          // Assignment
x += y         // Add and assign
x -= y         // Subtract and assign
x *= y         // Multiply and assign
x /= y         // Divide and assign
x %= y         // Modulo and assign
```

Less common or domain-specific compound assignments use `(qualifier)=`:

```hilow
x (or)= y                  // x = x or y
x (and)= y                 // x = x and y
flags (bitor)= MASK_READY  // flags = flags | MASK_READY
counter (atomic-add)= 1    // atomic increment (Low mode)
sum (saturating-add)= delta // saturating add (Low mode)
register (volatile)= value // volatile write (Low mode)
```

The `(qualifier)=` form unifies a class of operators that other languages handle through scattered intrinsics or special syntax. New qualifiers can be added without new operator characters.

### Logical

```hilow
x and y        // Logical AND (short-circuit)
x or y         // Logical OR (short-circuit)
not x          // Logical NOT

// Precedence: not > and > or
if (not x and y or z) { }           // ((not x) and y) or z
```

Word operators (`and`, `or`, `not`) read more naturally than `&&`, `||`, `!` in conditions:

```hilow
if (enabled and not error or retry) {
  // Clear intent
}
```

### Bitwise

```hilow
x & y          // AND
x | y          // OR
x ^ y          // XOR
~x             // NOT
x << y         // Left shift
x >> y         // Right shift
```

### Truthy/Falsy

In conditions, the following are falsy:
- `0` (any numeric zero)
- `""` (empty string)
- `false`
- `nothing`
- `unknown`
- Empty arrays (length 0)

Everything else is truthy.

```hilow
if (value) { }              // True if value is not falsy
if (array.length) { }       // True if array is non-empty
if (not result) { }         // True if result failed (unknown) or is absent (nothing)
```

### Removed from JavaScript

The following JavaScript operators are **not** in HiLow, by design:

```
x++, x--, ++x, --x          // Use x += 1, x -= 1
x ||= y                     // Use: if (not x) x = y
x &&= y                     // Use: if (x) x = y
x ??= y (nullish assign)    // Use: if (x is nothing) x = y
x?.y                        // Use explicit checks: if (x) x.y
x === y                     // HiLow uses ?= (no coercion to begin with)
x !== y                     // Use !=
x == y (with coercion)      // HiLow has no coercion; use (roughly)= for fuzzy
typeof x                    // Use: x is T
?:  (ternary)               // Use if/else
```

Bare `==` is also not valid. Attempting to use it produces a compile error directing you to `?=` for equality or `=` for assignment.

## Control Flow

### If Statements

```hilow
if (condition) {
  // code
}

if (condition) {
  // code
} else {
  // code
}

if (condition1) {
  // code
} else if (condition2) {
  // code
} else {
  // code
}
```

No ternary operator — use if/else.

### Switch Statements

```hilow
switch (value) {
  case 0:
    print("zero")
    break
  case 1:
    print("one")
    break
  default:
    print("other")
}

// Switch on strings
switch (command) {
  case "start":
    startServer()
    break
  case "stop":
    stopServer()
    break
}
```

### Pattern Matching

```hilow
match value {
  0 => print("zero"),
  1..10 => print("small"),
  11..100 => print("medium"),
  _ => print("large")
}

// Match on types
match result {
  is nothing => print("no value"),
  is unknown => print(f"error: {result.reason}"),
  _ => print(f"value: {result}")
}

// Match with guards
match x {
  n when n < 0 => print("negative"),
  n when n ?= 0 => print("zero"),
  n when n > 0 => print("positive")
}
```

### Loops

```hilow
// For loop with explicit counter
for (let i = 0; i < 10; i += 1) {
  print(i)
}

// For-in over array
for (let item in array) {
  print(item)
}

// For-in with index (parens around the destructuring)
for (let (index, value) in array) {
  print(f"[{index}] = {value}")
}

// For-in over object (high mode)
for (let (key, value) in object) {
  print(f"{key}: {value}")
}

// While loop
while (condition) {
  // code
}

// Infinite loop
loop {
  // code
  if (done) break
}

// Loop control
break       // Exit loop
continue    // Next iteration
```

The `loop { }` form replaces C's `for(;;)` for clarity.

## Functions

### Function Declaration

```hilow
function add(a: i32, b: i32): i32 {
  return a + b
}

// Type inference (high mode only)
function greet(name) {
  let message = f"Hello {name}"
  print(message)
}

// Multiple returns - parens for tuple destructuring
function divmod(a: i32, b: i32): (i32, i32) {
  return (a / b, a % b)
}

let (quotient, remainder) = divmod(10, 3)
```

The full keyword `function` is used (not `fn`). This is intentional — function declarations are visually heavy and easy to scan for. For dense expression contexts where compactness matters, function expressions are still readable.

### Function Expressions

```hilow
let add = function(a: i32, b: i32): i32 {
  return a + b
}

// Closures (high mode)
function makeCounter(): function {
  let count = 0
  return function(): i32 {
    count += 1
    return count
  }
}

let counter = makeCounter()
print(counter())                    // 1
print(counter())                    // 2
```

In Low mode, closures cannot capture variables that escape their scope. A closure used as a callback within the same function is fine; a closure returned or stored outside its scope requires High mode.

### Function Contracts

Preconditions with `requires`, postconditions with `ensures`. The result is named `result` in the postcondition by default.

```hilow
function divide(a: i32, b: i32): i32
  requires (b != 0)
  ensures (result * b <= a and result * b + b > a)
{
  return a / b
}

// Prover checks at call sites
let x = divide(10, 5)               // ✓ Proven safe
let y = divide(10, 0)               // ✗ Proof error: precondition violated

// Prover understands control flow
let divisor = getUserInput()
if (divisor != 0) {
  let z = divide(100, divisor)      // ✓ Prover knows divisor != 0
}
```

### Variadic Functions

Variadic parameters are arrays:

```hilow
function sum(values: [i32]): i32 {
  let total = 0
  for (let v in values) {
    total += v
  }
  return total
}

sum([1, 2, 3, 4, 5])                // 15
```

### Object Methods (High mode)

```hilow
let point = {
  x: 10,
  y: 20,
  distance: function(): f64 {
    return sqrt(this.x * this.x + this.y * this.y)
  }
}

point.distance()
```

## Memory Management

HiLow uses scope-based ownership as the foundation in both modes. Each value has exactly one owner — the variable bound to it — and is freed when that variable goes out of scope.

### Scope-Based Ownership (both modes)

```hilow
function process() {
  let buffer = alloc(1024)          // owned by `buffer`
  // ... use buffer ...
}                                   // compiler inserts free here
```

When ownership needs to transfer (return from function, store in a struct), the value is *moved*:

```hilow
function makeBuffer(): *u8 {
  let buf = alloc(1024)
  return buf                        // ownership moves to caller
}                                   // no free here

let mine = makeBuffer()             // mine now owns
```

### Borrowing

Function parameters are borrowed by default — the caller retains ownership:

```hilow
function inspect(buf: *u8, len: usize) {
  // borrowed - caller still owns
  for (let i = 0; i < len; i += 1) {
    print(buf[i])
  }
}

let mine = alloc(1024)
inspect(mine, 1024)                 // mine still owns after call
```

The compiler enforces that borrowed pointers don't outlive the owner. In Low mode this is a strict check. In High mode, escaped references are handled by implicit refcounting (see below).

### High Mode: Implicit Reference Counting

When the compiler can prove single ownership, High generates code identical to Low. When it cannot — the value is captured in a closure, returned through complex paths, or stored in multiple places — the compiler transparently inserts reference counting.

```hilow
high program(): i32 {
  let user = { name: "Alice", id: 42 }
  
  let getNamer = function(): function {
    return function(): string {
      return user.name              // captures user - refcounted
    }
  }
  
  let getName = getNamer()
  print(getName())                  // "Alice"
  return 0
}
```

The developer doesn't see the refcounting. There's no GC pause, no runtime tracing — just a small inc/dec on the refcount when the value is shared. Cycle detection is opt-in via `weak` references when needed.

#### Weak References

A `weak` store breaks a reference cycle: the property holds the object
without owning it. Weak stores appear in object literals and in property
assignments:

```hilow
let target = { name: "T" }
let holder = { ref: weak target }   // literal form
holder.ref = weak target            // assignment form
```

Semantics:

- **No retain on store.** A weak property does not contribute to the
  referent's reference count. Overwriting or dropping the property releases
  nothing.
- **Slot nulled on referent death.** When the referent's last strong
  reference is released, every weak property pointing at it is cleared.
- **Reading a weak property yields `T?`** — the referent while it is alive,
  or `unknown` with reason `"weak referent released"` after its death.
  Binding the live referent (`let r = holder.ref`) holds a strong reference
  for the binding's lifetime.
- **Member access propagates.** Accessing a property through a weak read
  follows the standard unknown-propagation rule: on a live referent, a
  property of type `T` reads as `T?`; on a dead one, the access returns the
  same unknown.
- **No deep propagation.** A weak property creates no containment link, and
  `(deep)` watching does not cross a weak reference — mutations under a
  weakly-held value never fire the weak holder's watchers. Weak is
  observation without ownership.

```hilow
print(holder.ref.name)              // "T" while target is alive

target = { name: "T2" }             // old target's last strong ref released

let r = holder.ref
if (r is unknown) {
  print(r.reason)                   // "weak referent released"
}

let n = holder.ref.name             // string? — propagated unknown
if (n is unknown) {
  print("no referent")
}
```

### Low Mode: Explicit Memory Modes

Low mode does not insert refcounting. Sharing requires explicit choice:

```hilow
// Default: scope-based, single owner
let buf = alloc(1024)
// freed at scope exit

// Stack: explicit stack allocation (Low mode only)
stack p: i64
stack buffer: [u8; 256]
// freed at scope exit; same as default but documents intent

// Heap: explicit heap allocation (Low mode only)
heap data: [u32; 1024]
// freed at scope exit; same as default but documents the cost

// Manual: you control the lifetime
manual let graph = alloc(size_of<Graph>)
defer graph
// freed when defer runs

// Refcounted, atomic, and cross-context watchable: opt-in shared ownership
shared let resource = rc_alloc<Connection>()
// freed when last reference drops; safe to read/write and watch across
// threads (see "Concurrency Safety" and "Cross-Process Watchers")

// Arena: bulk allocation, freed all at once
arena {
  let a = arena.alloc(100)
  let b = arena.alloc(200)
  // ... no individual frees ...
}                                   // entire arena freed here
```

The Low-mode declarators `stack` and `heap` are alternatives to `let` that document where the variable lives. They behave identically to `let` for ownership and cleanup, but make the storage location explicit. Use them when storage location matters for the reader's understanding (e.g., a `stack` declaration of a 256-byte buffer signals that this size is acceptable on the stack).

`stack` and `heap` are not available in High mode. High-mode developers do not generally need to think about storage location; the compiler manages it.

The five forms cover the full spectrum:
- **`let` (scope)**: single owner, automatic cleanup, no overhead
- **`stack` / `heap` (Low only)**: same ownership as `let`, but explicit about storage location
- **`manual`**: explicit control, for unusual lifetimes
- **`shared` (refcount + atomic + watchable)**: `shared` has one unified
  meaning across the language. A `shared` variable is refcounted (shared
  ownership, freed when the last reference drops), its scalar payload is
  accessed **atomically** by default, and it is **watchable across contexts**
  (threads and, via the process tier, processes) — the runtime routes a write
  on one context to watchers on the declaring context. The same keyword that
  opts into shared ownership in Low mode is the keyword that makes state safe
  to observe concurrently; there is not a separate concurrency-only `shared`.
  See "Concurrency Safety" and "Cross-Process Watchers". Opt-in cost.
- **`arena`**: bulk allocation for batched work

### Pointers (Low mode only)

```hilow
let x: i32 = 42
let ptr: *i32 = address(x)
let value = *ptr                    // Dereference

// Pointer arithmetic
let array = [1, 2, 3, 4, 5]
let p = address(array[0])
p += 1                              // Points to array[1]
print(*p)                           // 2

// Multi-level pointers
let pp: **i32 = address(ptr)
let v = **pp                        // 42
```

In High mode, pointers are not exposed. References to flexible objects are implicitly managed.

### The `defer` Statement

`defer` schedules cleanup to run when the current scope exits, including on early returns and `break`. It comes in two forms:

```hilow
// Smart form: defer the type-appropriate cleanup
function process() {
  manual let buffer = alloc(1024)
  defer buffer                       // compiler infers: free(buffer)
  
  let file = openFile("data.txt")
  defer file                         // compiler infers: file.close()
  
  doWork(buffer, file)
}                                    // both cleaned up here

// Explicit form: defer a specific expression
function process() {
  manual let buffer = alloc(1024)
  defer free(buffer)                 // explicit cleanup expression
  
  doWork(buffer)
}
```

The smart form (`defer <var>`) consults the type of `<var>` to determine the cleanup. Each resource type registers its cleanup function:

- `manual` allocations: `free(var)`
- File handles: `var.close()`
- Network connections: `var.close()`
- Locks: `var.release()`
- User types: cleanup is determined by the type's destructor

The explicit form (`defer <expr>`) runs the literal expression at scope exit. Use it when the smart form's inference doesn't match what you want, or when the cleanup involves multiple values.

Multiple `defer` statements run in LIFO order: the last `defer` runs first.

```hilow
function example() {
  defer print("third")
  defer print("second")
  defer print("first")
  // prints: first, second, third
}
```

`defer` is available in both modes. In Low it's commonly used with `manual` allocations; in High it's used for non-memory cleanup (closing files, releasing locks).

### Memory Safety Verification

The proof system verifies:

```hilow
// Use after free
manual let buffer = alloc(1024)
free(buffer)
buffer[0] = 42                      // ✗ Proof error: use after free

// Double free
manual let buf = alloc(1024)
free(buf)
free(buf)                           // ✗ Proof error: double free

// Memory leak
manual let buf = alloc(1024)
// no defer, no free
return                              // ✗ Proof error: leak

// Dangling borrow
function bad(): *u8 {
  let local = alloc(64)
  return local                      // ✓ ownership moved
}

function alsoBad(buf: *u8): *u8 {
  return buf                        // ✓ borrowed, returned to caller
}

function actuallyBad(): *u8 {
  let local: [u8; 64]
  return address(local[0])          // ✗ Proof error: stack address escapes
}
```

## Watcher System

A watcher is a reactive construct that runs in response to changes in subscribed variables. Watchers are how HiLow expresses **situation-aware programming**: code that responds to whatever combination of conditions emerges, regardless of how those conditions came to be. They handle constraints, self-healing, event-driven programming, async result aggregation, and multi-source coordination — all through one mechanism: "when these values change, run this code, and let the body decide what to do."

Watchers are a fundamental construct alongside functions. Both share body-level semantics — parameter binding, mode rules, scope, closures — but differ in their invocation model. A function is invoked by an explicit call expression. A watcher is invoked by the runtime when any subscribed variable changes.

### Watcher Declaration

Watchers parallel functions in syntax. The declaration form binds a name in the enclosing scope:

```hilow
let balance = 1000

watcher onBalance(balance) {
  print(f"Balance changed to: {balance}")
}

balance = 2000                      // Watcher fires
```

Like functions, watchers can be declared in either mode:

```hilow
high watcher onRequest(req) {
  // High mode body — flexible objects, refcounting, etc.
}

low watcher onHardwareFlag(flag) {
  // Low mode body — fixed memory, no refcounting
}
```

Mode is inherited from the enclosing context (program, module, or function) unless explicitly overridden. The same mode rules that govern functions govern watchers.

A watcher does **not** fire at declaration time. Declaration is setup, not execution. The body runs only on subsequent value changes to subscribed variables.

### Subscription List and Snapshot Semantics

The watcher's parameter list is a **subscription list** — each entry names an outer-scope variable to observe. When any subscribed variable's value changes, the runtime reads the current values of all subscribed variables and passes them into the body as parameters:

```hilow
let x = 0
let y = 0

watcher onPosition(x, y) {
  // Inside the body, x and y are local parameters
  // holding snapshot values at the moment the watcher fired.
  print(f"Position: ({x}, {y})")
}

x = 10                              // Fires with x=10, y=0
y = 20                              // Fires with x=10, y=20
```

Inside the body, the subscribed names are ordinary local parameters bound to snapshot values. They have no connection back to the outer variables — writing to them modifies the local parameter only:

```hilow
let counter = 0

watcher onCounter(counter) {
  counter = counter + 1             // Modifies the local parameter only.
  print(counter)                    // Prints the modified local value.
}                                   // Outer counter is untouched.

counter = 10                        // Fires with counter=10; prints 11.
                                    // Outer counter is still 10.
counter = 11                        // Fires with counter=11; prints 12.
```

Because the body cannot write to the outer variable through the parameter name, the self-triggering problem dissolves: a watcher fundamentally cannot trigger itself by writing through its parameters. (It could still trigger itself by assigning to the outer variable through a different path — for example, if a captured reference is used — but the common case is handled by the language semantics directly.)

### Value-Change Triggering

A watcher fires only when a subscribed variable's value actually changes. Assignment alone is not enough:

```hilow
let temperature = 20

watcher onTemperature(temperature) {
  print(f"Temperature: {temperature}")
}

temperature = 20                    // No fire — value unchanged
temperature = 21                    // Fires
temperature = 21                    // No fire — value unchanged
temperature = 20                    // Fires
```

**What a subscription targets.** A subscription watches either the **variable** (its binding slot) or the **value** it holds:

- A **declaration-form** watcher's `(changed)`/`(assigned)` subscriptions, and any **`(assigned)`** subscription in either form, watch the *variable*. They observe assignment: `(assigned)` fires on every assignment, and `(changed)` fires only when the newly-assigned value differs from the previous one under the type's own equality — value equality for primitives (strings compare by contents), identity for objects and arrays. Mutating a value in place (e.g., `list.push(x)`) is not an assignment and never fires a variable subscription. On one assignment that satisfies both modifiers, `(changed)` subscribers fire before `(assigned)` subscribers.
- An **expression-form** watcher's content modifiers (`(changed)`, `(deep)`, `(added)`, `(removed)`, `(moved)`) subscribe the *value* the variable holds when the watcher is constructed. For containers these fire on content mutation. The subscription belongs to the value itself: if the variable is later rebound, the watcher stays with the original value.
- Content modifiers in a **declaration-form** watcher (`(added)`, `(removed)`, `(moved)`, `(deep)`) subscribe the value the variable *currently* holds and **follow rebinding**: on assignment, the subscription moves from the old value to the new one — deep watching extends into the new value's nested structure — so the watcher always observes the variable's current contents. Mutations of the previously-held value no longer fire it after rebinding. Retargeting completes before the variable's own `(changed)`/`(assigned)` subscribers fire, and it moves subscriptions, not watcher state: a paused watcher's subscriptions still follow. Each followed variable's subscriptions move independently, on that variable's own rebinding only: two variables holding the same value do not move together — rebinding one leaves the other variable's subscriptions on the value it still holds.

This split keeps the default firing rule cheap while making both rebinding-watch and content-watch expressible.

For watchers with multiple subscriptions, each individual variable's change is evaluated independently. The watcher fires once per detected change to any subscribed variable.

### Subscription Modifiers

Each entry in a subscription list can carry a **modifier** that controls what kind of change triggers the watcher. The default modifier is `changed`, matching the value-change rule above. Other modifiers extend or refine the triggering behavior:

| Modifier | Fires when |
|---|---|
| `(changed)` | Default. On a variable subscription: the assigned value differs from the previous one (type's own equality — strings by contents, containers by identity). On a container value subscription: any content mutation. |
| `(assigned)` | Every assignment to the variable, regardless of whether the value differs. |
| `(deep)` | Any mutation to the value, including in-place changes to nested structure. |
| `(added)` | One or more items added to a collection. |
| `(removed)` | One or more items removed from a collection. |
| `(moved)` | Items reordered within a collection without being added or removed. |

The modifier appears as a prefix in parentheses before the variable name:

```hilow
let temperature = 20

watcher onAnyAssignment((assigned)temperature) {
  // Fires even on `temperature = temperature` self-assignment.
}

let items = []

watcher onItemsMutated((deep)items) {
  // Fires when items.push(x), items[0] = y, etc.
  print(f"Items now: {items}")
}
```

**Default parameter binding.** A modifier without an alias binds the parameter name to the **current full value** of the outer variable. The modifier determines *when* the watcher fires; the parameter always carries the variable's current state. This means even `(added)items` gives you the full current list, not just the added items.

**Aliasing for delta information.** To receive delta-specific information (the items added, the items removed, etc.), provide an alias inside the parentheses with `alias=modifier`:

```hilow
let items = []

watcher onItemChange((newAdds=added)items) {
  // `items` is the full current list.
  // `newAdds` is the list of items that were just added.
  print(f"Added {newAdds.length} items; total is now {items.length}")
}
```

**Multiple modifiers on the same variable.** A single variable can appear multiple times in the subscription list with different modifiers — each gives the watcher a separate way to be triggered, with its own optional alias:

```hilow
watcher onCollectionEvent(
  (newAdds=added)items,
  (gone=removed)items,
  (shuffled=moved)items
) {
  // Each subscription fires the watcher independently.
  // newAdds, gone, shuffled are delta-bound by their aliases.
  // items is the full current state (post-mutation).
}
```

**Rules.**

- The same variable may appear multiple times with **different** modifiers; the same modifier on the same variable twice in one watcher is a parse error.
- An empty subscription list is a parse error — a watcher with no subscriptions has no triggering condition and can never fire.
- Aliases must be unique within a watcher's subscription list.

### Watcher Expressions

Like functions, watchers can also appear as expressions, producing a value that can be bound to a variable or stored in a structure:

```hilow
let w = watcher(balance) {
  print(f"Balance: {balance}")
}

let monitors = {
  onBalance: watcher(balance) {
    print(f"Balance: {balance}")
  },
  onTransactions: watcher((added)transactions) {
    print(f"Transactions: {transactions.length}")
  }
}
```

The expression form is useful when the watcher needs to be stored, passed, or referenced by an explicit handle. The declaration form is preferred when a watcher simply needs to exist for the duration of its enclosing scope.

### Lifecycle, Scope, and Escape

A watcher's value is first-class — it can be stored, passed as a parameter, or returned from a function — but its **subscriptions** are part of its identity. The subscription list is fixed at declaration; it determines which variables the watcher observes for the rest of its life.

Only the **expression form** produces a first-class value. A **declaration-form** watcher name is not a value: it supports only the four method calls (`.pause()`, `.resume()`, `.end()`, `.isActive()`) and cannot be aliased, passed, or returned — declaration-form watchers are therefore always bound to their declaring scope. To hand a watcher around, use the expression form.

**Within a scope.** A watcher declared in a scope lives for the duration of that scope. When the scope exits, the watcher ends automatically:

```hilow
function processSession(session) {
  watcher onUpdate(session.state) {
    log(session.state)
  }
  
  // ... session work ...
}                                   // Watcher ends here when scope exits
```

This matches HiLow's broader scope-based ownership model.

**Escape is sound.** A watcher value may escape its declaring scope — returned from a function, stored in an outer-scope variable, captured by another closure. The watcher holds its subscribed and captured variables alive: a watched variable lives as long as any watcher that references it, even after its declaring scope exits. There is no reachability restriction.

```hilow
function makeMonitor(target: [i32]): watcher {
  let count = 0
  return watcher((added)target) {
    print(count)                    // ✓ `count` outlives makeMonitor: the
  }                                 //   watcher keeps it alive.
}

let items = []: [i32]
let m = makeMonitor(items)
items.push(1)                       // Fires through m
```

A subscription to a variable that no surviving scope can reach is legal but inert — nothing can mutate the variable anymore, so the watcher simply never fires again through that subscription.

**Fire order.** When several watchers observe the same variable, a mutation fires them in subscription order: the watcher declared earliest fires first.

**Low mode.** Low mode forbids watcher escape entirely, matching the broader Low-mode closure restriction.

### Operations on Watcher Values

A watcher value (from a declaration or expression) supports four operations:

```hilow
watcher onCounter(counter) {
  print(counter)
}

onCounter.pause()                   // Suspend firing
counter = 5                         // No fire
onCounter.resume()                  // Resume firing
counter = 6                         // Fires
onCounter.end()                     // Permanently end (also happens at scope exit)
let active = onCounter.isActive()   // Query state
```

`.end()` is rarely needed because scope exit handles it automatically. It exists for cases where a watcher should stop before its scope ends — for example, a watcher that ends itself once a condition is met:

```hilow
watcher untilDone(counter) {
  if (counter >= 100) {
    print("Done!")
    untilDone.end()
  }
}
```

After `.end()`, the watcher value becomes inert: subsequent assignments to subscribed variables produce no fires. The value itself remains valid until it is dropped through normal scope or refcounting rules.

### Stealth Blocks

Sometimes you need to mutate watched state without firing watchers — during initialization, recovery, or internal bookkeeping that shouldn't be visible to observers. The `stealth { ... }` block provides this:

```hilow
let balance = 0
watcher onBalance(balance) {
  print(f"Balance changed: {balance}")
}

balance = 100                       // Fires: "Balance changed: 100"

stealth {
  balance = 0                       // No fire
  balance = 500                     // No fire
}                                   // Final state: balance is 500, no fires occurred

balance = 600                       // Fires: "Balance changed: 600"
```

`stealth` blocks are *dynamic* — they suppress watcher notifications for any writes that occur during the block's execution, including writes made inside functions called from the block:

```hilow
function reset() {
  balance = 0
  total_spent = 0
}

stealth {
  reset()                           // Writes inside reset() also don't fire watchers
}
```

The default behavior is the opposite — watchers fire for all value changes — because watchers exist for constraints, self-healing, monitoring, and reactive updates that should run by default. `stealth` is the explicit opt-out for operations that should not be observed.

`stealth` blocks do not change the values' final state, only the notifications. After a `stealth` block exits, watched variables reflect their actual current values; subsequent writes outside the block trigger watchers normally.

`stealth` is available in both High and Low modes.

### Situation-Aware Programming

Watchers shine when a program must respond to combinations of conditions that emerge from independent sources. Rather than encoding an execution order ("first A, then B, then C"), the programmer encodes situational responses ("when A is true, do X; when B is true, do Y; when A and B are both ready, do Z"). The watchers don't coordinate with each other — each just observes its variables and reacts. System-level behavior emerges from the watchers' collective responses to whatever state arises.

Errors are just another situation, observable through the same mechanism. The asymmetry between "success path" and "error path" that infects promise-based code disappears.

```hilow
let api_data = nothing
let db_data = nothing
let sensor_data = nothing

// Fires when all three sources have produced valid data.
watcher onComplete(api_data, db_data, sensor_data) {
  if (api_data is nothing) return
  if (db_data is nothing) return
  if (sensor_data is nothing) return
  if (api_data is unknown) return
  if (db_data is unknown) return
  if (sensor_data is unknown) return
  reconcile(api_data, db_data, sensor_data)
}

// Handles API failures independently.
watcher onApiFailure(api_data) {
  if (api_data is unknown) {
    log(f"API failed: {api_data.reason}")
    api_data = fallback_value
  }
}

async { api_data = fetch_from_api() }
async { db_data = fetch_from_db() }
async { sensor_data = read_sensor() }
```

The three `async` blocks run concurrently and complete in any order. The watchers observe whatever happens. `onComplete` waits — passively — for all three to be valid. `onApiFailure` independently handles the failure case for one source. Neither watcher knows about the other; both react to situations as they arise.

Other common watcher uses follow the same pattern:

```hilow
// Self-healing: correct state when it drifts out of bounds.
watcher onConnections(connectionCount) {
  if (connectionCount > maxAllowed) {
    closeOldestConnections(connectionCount - maxAllowed)
  }
}

// Constraint enforcement: detect and report violations.
watcher tempInRange(temperature) {
  if (temperature < -50.0 or temperature > 150.0) {
    alarm(f"Temperature out of range: {temperature}")
  }
}

// Event-driven: process work as it arrives.
watcher onRequest((added=added)server.requests) {
  for (let req in added) {
    handleRequest(req)
  }
}
```

Each watcher looks at the situation when it fires and decides what to do — including doing nothing. The body's logic is the heart of each watcher; the subscription list is just how the body gets invoked.

### Cross-Process Watchers

A `shared("name")` variable names a shared-memory segment (`shared("name")`,
Phase 6a). Two separately-launched programs that declare the same name share one
typed slot; a write in one is observed by a watcher in the other. `shared`
without a name stays in-process (cross-thread) only.

```hilow
// Process 1 (producer program)
shared("counter") let counter = 0
let i = 0
while (i < 100) {
  counter += 1
  i += 1
}
```

```hilow
// Process 2 (watcher program)
shared("counter") let counter = 0
let done = 0

watcher onCounter(counter) {
  if (counter >= 100) {
    print("Done!")
    done = 1
    onCounter.end()
  }
}
// keep observing (draining at the loop back-edge) until the threshold
while (done < 1) { }
```

The runtime handles the inter-process notification — the watcher syntax is
identical to single-process watchers; only the `("name")` on the declaration
opts the variable into the cross-process segment.

**Consistency model.** Across processes, the runtime guarantees that at least one watcher fire occurs per *logical* change to a `shared` variable, but may **coalesce** rapid changes. If process 1 writes `counter` from 5 to 6 to 7 to 8 in quick succession, process 2's watcher may fire once with `counter=8` rather than three times with successive values. The body should not assume it sees every intermediate value when watching shared state; it should reason about the current value at the moment of fire.

**Write idempotent bodies.** Coalescing is an *at-least-once* guarantee, and the emphasis belongs on *at least*: the delivery count is not the change count in either direction. A cross-process watcher body may run more times than there were logical changes, so any body whose effect is not naturally repeatable must make itself repeatable.

The common trap is a threshold. In the example above, `counter >= 100` is not a one-time event — it is a condition that stays true for every subsequent fire. Worse, a body can run twice for a *single* crossing: one delivery observes the crossing, and a later one delivers the epochs that were still outstanding when the first fired. Written naively, that prints `Done!` twice:

```hilow
// WRONG — the effect repeats
watcher onCounter(counter) {
  if (counter >= 100) {
    print("Done!")      // may print more than once for one crossing
  }
}
```

Both fixes are ordinary code. Ending the subscription on the first success stops any further delivery:

```hilow
watcher onCounter(counter) {
  if (counter >= 100) {
    print("Done!")
    onCounter.end()     // no further fires for this watcher
  }
}
```

Or guard the effect with state the body owns, which is the general form and also covers watchers that must keep observing:

```hilow
let announced = 0
watcher onCounter(counter) {
  if (counter >= 100) {
    if (announced < 1) {
      print("Done!")
      announced = 1
    }
  }
}
```

The rule generalizes: treat a cross-process watcher body the way you would treat a message handler that may see duplicates. Reading the current value, computing from it, and assigning a result are all naturally idempotent and need no guard. Incrementing a counter, appending to a log, or sending something outward are not, and do.

#### Segment Lifetime and Cleanup

A `shared("name")` declaration maps a POSIX shared-memory object named `/hilow.<name>`, created with mode `0600` — readable and writable only by the user who created it. The name is a single flat per-user namespace: two programs share state exactly when they use the same `name`, with no scoping by directory, package, or process tree. Choose names accordingly; a generic `"counter"` will collide with any other program of yours using `"counter"`.

**Segments persist.** A segment lives until it is explicitly removed, not until its last user exits. This is deliberate and is what makes the feature work: a program can write a value, exit, and have a program started an hour later attach and read it. It also means the state survives a crash — if a process is killed outright, the segment and its value remain intact and the next process to attach sees them.

The consequence is that cleanup is your responsibility. HiLow has no language surface for removing a segment, no automatic reclamation, and no reference counting across processes. A segment you no longer want is removed from outside the language. On Linux the objects are visible as ordinary files:

```
ls -l /dev/shm/hilow.*          # list every HiLow segment
rm /dev/shm/hilow.counter       # remove the segment named "counter"
rm /dev/shm/hilow.myapp.*       # remove a whole prefixed family
```

Removal unlinks the *name*. Processes already attached keep their existing mapping and continue to work; they are not disturbed. But the name is now free, so the next program to declare `shared("counter")` creates a brand-new, freshly initialized segment rather than joining the old one. Unlinking a live segment therefore silently splits its users into two groups, which is rarely what you want — remove segments when nothing is using them.

Segments do not survive a reboot.

Two practical habits follow. Prefix the names of segments belonging to one application (`"myapp.counter"`) so they can be listed and removed as a family. And in tests, always remove the segments you create: a leaked segment makes the next run of the same test attach to stale state instead of starting clean, which presents as an inexplicable failure. HiLow's own cross-process test harness unlinks every segment it creates, before and after each run, for exactly this reason.

### Conditions Inside Watchers

A watcher fires whenever any subscribed value changes (per its modifiers). The body can guard its logic with ordinary conditionals:

```hilow
let enabled = true
let value = 0

watcher onValue(value, enabled) {
  if (not enabled) return
  print(f"Value: {value}")
}

enabled = false
value = 100                         // Fires but returns early
enabled = true
value = 200                         // Fires and prints
```

### Relationship to Functions

At the body level, watchers and functions are nearly identical:

- Same statement and expression grammar
- Same mode rules and mode inheritance
- Same scoping and capture semantics
- Same closure rules per mode (high: free capture with refcounting; low: only non-escaping captures)
- Same parameter-binding mechanics at invocation

They differ in their invocation model and what each represents:

| | Function | Watcher |
|---|---|---|
| Invocation | Explicit call expression | Runtime, on subscribed-variable change |
| Parameters | Argument expressions at call site | Snapshot of subscribed variables (and deltas if aliased) |
| Return | Returns a value to caller | Returns nothing |
| Lifecycle | Tied to call/return | Tied to declaring scope (plus escape rules) |
| Handle | None — call by name | `.pause()`, `.resume()`, `.end()`, `.isActive()` |

A watcher body cannot declare a return type; the return type is implicitly nothing. Attempting to return a value is a compile error.

## Error Handling

HiLow uses `unknown` as its error mechanism — values that carry both a reason and suggested options for handling.

### Returning Unknown

```hilow
function divide(a: i32, b: i32): i32?  {
  if (b ?= 0) {
    return unknown("division by zero", options: ["use different divisor"])
  }
  return a / b
}

function getUser(id: i32): object? {
  let result = database.query(f"SELECT * FROM users WHERE id = {id}")
  
  if (not result) {
    return unknown("database error", options: ["retry", "check connection"])
  }
  
  if (result.length ?= 0) {
    return unknown("user not found", options: ["check id", "create user"])
  }
  
  return result[0]
}
```

### Checking Unknown

```hilow
let result = divide(10, 0)

// Type test with `is`
if (result is unknown) {
  print(f"Error: {result.reason}")
  print(f"Options: {result.options}")
  return
}

// Truthy check (unknown is falsy)
if (not result) {
  print(f"Failed: {result.reason}")
  return
}

print(f"Result: {result}")
```

### Unknown Propagation

`unknown` propagates through property access — accessing a property on an unknown returns the same unknown:

```hilow
let user = getUser(999)             // returns unknown

user.name                           // unknown (same instance)
user.address.street                 // unknown (propagates)

// Safe to chain - check at the end
let street = user.address.street
if (street is unknown) {
  print(f"Could not get street: {street.reason}")
} else {
  print(f"Street: {street}")
}
```

### Logic Based on Reason

```hilow
function fetchData(url: string): object? {
  let response = http.get(url)
  
  if (response.status != 200) {
    if (response.status ?= 404) {
      return unknown("not found", options: ["check url", "try alternate"])
    } else if (response.status ?= 500) {
      return unknown("server error", options: ["retry", "contact admin"])
    } else {
      return unknown(f"http error {response.status}", options: ["retry"])
    }
  }
  
  return response.body
}

let data = fetchData("https://api.example.com/data")

if (data is unknown) {
  if (data.reason ?= "not found") {
    print("Resource doesn't exist")
  } else if (data.reason ?= "server error") {
    data = fetchData("https://api.example.com/data")  // Retry
  } else {
    print(f"Unknown error: {data.reason}")
  }
}
```

### Verification

The proof system ensures unknown values are checked before use:

```hilow
let user = getUser(123)
print(user.name)                    // ✗ Proof error: unknown not handled

// Correct
let user = getUser(123)
if (user is unknown) {
  print(f"Error: {user.reason}")
  return
}
print(user.name)                    // ✓ Proven safe
```

### Nothing vs Unknown

These are distinct concepts:

- **`nothing`** is *true absence* — uninitialized variable, missing property, deallocated memory.
- **`unknown`** is *failure* — an operation completed but couldn't produce a value.

```hilow
let x                               // nothing - uninitialized
let y = someOperation()             // might be unknown - operation result

if (x is nothing) {
  print("never had a value")
}

if (y is unknown) {
  print(f"operation failed: {y.reason}")
}

// Both are falsy
if (not x) { }                      // true
if (not y) { }                      // true (if unknown)
```

## Formal Verification

HiLow's optional proof system verifies properties at compile time. It is more emphasized in Low mode (where memory and bounds bugs have severe consequences) but works in both modes.

### Variable Constraints

Constraints define valid values. HiLow has two forms: **predicates** (arbitrary boolean expressions) and **sets** (explicit domains).

#### Predicate Form

Wrap any boolean expression in parentheses after the type. The variable being constrained is referenced by its name:

```hilow
let percent: i32 (percent >= 0 and percent <= 100) = 50
let temperature: f32 (temperature >= -273.15)
let port: u16 (port >= 1024)
let balance: money (balance >= 0.00 USD)
let length: i32 (length !> capacity)        // length not greater than capacity
```

Predicates are general — any boolean expression works. They handle relational invariants (variables compared to other variables), function results (`isPrime(n)`), and arbitrary logic.

#### Set Form

For domains expressible as a list of values and ranges, the set form is more concise and clearer:

```hilow
let direction: i32 in {-1, 0, 1}                    // exactly one of these
let day: string in {"Mon", "Tue", "Wed", "Thu", "Fri"}
let bigMonth: i32 in {1, 3, 5, 7, 8, 10, 12}        // months with 31 days
let percent: i32 in {0..100}                         // range as set member
let port: u16 in {1024..65535}                       // valid TCP/UDP port range
let mixed: i32 in {1, 2, 5..14, 16}                  // scalars and ranges combined
let valid: i32 in {1..100} excluding {10, 12}        // exclusion clause
```

Set syntax:

```
in { member, member, ... } [excluding { member, member, ... }]
```

A member is either a scalar value or a range (`a..b`). Ranges in sets are **inclusive on both ends** — `5..14` means the integers 5, 6, 7, ..., 14. The `excluding` clause is optional and follows the same syntax.

Members can reference variables and function calls, not just literals:

```hilow
let port: u16 in {1024..max_port} excluding {reserved_port}
let valid_id: i64 in {1..get_max_id()}
```

When members are runtime values, the prover treats the constraint as a runtime check rather than a static guarantee — see "Proof Modes" below.

The set form is preferred for bounded domains; the predicate form is preferred for relational invariants and complex logic. Both compile to the same proof checks.

#### What's Not Provided

To keep the language small, the following are deliberately *not* in HiLow:

- Set unions, intersections, or arithmetic — use a predicate if you need this complexity
- Nested sets — `{1, {2, 3}}` is not valid
- Range syntax outside of `in {...}` membership — there is no general range type, no range iteration, no slicing with ranges

If a constraint cannot be expressed cleanly with a predicate or a single set-with-optional-exclusion, the language design suggests the constraint may be too complex and should be reconsidered.

The prover verifies assignments:

```hilow
percent = 150                       // ✗ Proof error: violates constraint
percent = 75                        // ✓ Proven safe
```

### Function Contracts

```hilow
function divide(a: i32, b: i32): i32
  requires (b != 0)
  ensures (result * b <= a and result * b + b > a)
{
  return a / b
}

let x = divide(10, 5)               // ✓
let y = divide(10, 0)               // ✗ Proof error: precondition violated

let divisor = getUserInput()
if (divisor != 0) {
  let z = divide(100, divisor)      // ✓ Prover follows control flow
}
```

### Loop Invariants

For loops with non-trivial state, an `invariant` clause tells the prover what holds before, during, and after each iteration:

```hilow
function sum_array(arr: [i32]): i32 {
  let total = 0
  for (let i = 0; i < arr.length; i += 1)
    invariant (total >= 0 and i <= arr.length)
  {
    if (arr[i] >= 0) {
      total += arr[i]
    }
  }
  return total
}
```

The prover verifies:
1. The invariant holds when the loop is first entered
2. Each iteration preserves the invariant (assuming it held at iteration start)
3. The invariant + the loop's exit condition give whatever postcondition the surrounding function requires

Without invariants, the prover handles only simple loops where the relevant properties can be inferred. Complex loops require explicit invariants to verify.

### Termination

A `decreases` clause provides a metric that strictly decreases with each loop iteration or recursive call. The prover uses this to verify the code terminates (does not loop forever):

```hilow
function factorial(n: i32): i32
  requires (n >= 0)
  decreases (n)
{
  if (n ?= 0) return 1
  return n * factorial(n - 1)         // n decreases on each call
}

function find_target(arr: [i32], target: i32): i32 {
  let i = 0
  while (i < arr.length and arr[i] != target)
    decreases (arr.length - i)
  {
    i += 1
  }
  return i
}
```

The decreases expression must be a non-negative integer that strictly decreases. A function or loop with a verified `decreases` clause is guaranteed to terminate. Functions without `decreases` are not verified for termination; they may loop forever, and the prover does not check.

A function that has both `requires`/`ensures` clauses AND a `decreases` clause AND no `unknown` returns is **total**: it always terminates with a valid result on inputs satisfying its preconditions.

### Array Bounds

```hilow
let items: [i32; 10]

function getItem(index: i32): i32
  requires (index >= 0 and index < 10)
{
  return items[index]
}

let x = getItem(5)                  // ✓ Literal within bounds
let y = getItem(15)                 // ✗ Proof error: out of bounds

let index = getUserInput()
if (index >= 0 and index < 10) {
  let z = getItem(index)            // ✓ Bounds satisfied
}
```

### Resource Lifecycle

Beyond memory, HiLow tracks the lifecycle of other resources: files, locks, network connections, database transactions, etc. Each resource type defines its valid state transitions; the prover verifies code respects them.

```hilow
let file = openFile("data.txt")     // state: open
if (file is unknown) {
  return                            // no cleanup needed; file was never opened
}
defer file                          // schedules: file.close() at scope exit

let content = file.read()           // ✓ valid: file is open
file.close()                        // explicit close; state: closed
let more = file.read()              // ✗ Proof error: read on closed file
```

Resources cannot be:
- Used after being released (read after close, lock acquired twice without release)
- Released twice (double-close, double-release)
- Leaked (allocated but never released, on any control flow path)

`defer <var>` is the most common way to satisfy lifecycle proofs — it guarantees the cleanup runs.

### Numeric Overflow

In Low mode, arithmetic on fixed-width integers can overflow silently. The prover verifies that arithmetic stays within the type's range, or that the programmer has explicitly opted into overflow behavior.

```hilow
low function unsafe_add(a: u8, b: u8): u8 {
  return a + b                      // ⚠ Proof warning: u8 + u8 may overflow
}

low function checked_add(a: u8, b: u8): u8
  requires (a + b <= 255)
{
  return a + b                      // ✓ precondition prevents overflow
}

low function saturating_add(a: u8, b: u8): u8 {
  let result: u8 = a
  result (saturating-add)= b        // ✓ explicit saturation
  return result
}
```

In High mode, overflow defaults to checked: an arithmetic overflow produces an `unknown` value rather than a silent wrap. Code that needs different behavior uses the explicit qualifier (`(saturating-add)=`, `(wrapping-add)=`, etc.).

### Concurrency Safety

For programs using `async` and `shared`, the prover checks for race conditions and improper synchronization:

```hilow
shared let counter: i32 = 0

async {
  counter += 1                      // ✓ shared variables use atomic operations by default
}

async {
  let old = counter
  counter = old + 1                 // ⚠ Proof warning: read-modify-write on shared
                                    //   without explicit atomicity is racy
}

async {
  counter (atomic-add)= 1           // ✓ explicit atomic operation
}
```

The prover verifies:
- All accesses to `shared` variables use atomic operations or proper locking
- Watch callbacks on `shared` variables don't race with concurrent writes
- `async` blocks don't have data races on captured non-shared variables

### Money and Time Constraints

```hilow
function calculateTotal(price: money<USD>, tax: f64): money<USD>
  requires (tax >= 0.0 and tax <= 1.0)
  ensures (result >= price)
{
  return price * (1.0 + tax)
}

let item = 100.00 USD
let total = calculateTotal(item, 0.08)  // ✓
let bad = calculateTotal(item, 1.5)     // ✗ Precondition violated

function scheduleMeeting(when: time): bool
  requires (when > time.now())
{
  return when >= time.now() + 1h
}

let tomorrow = time.now() + 1d
scheduleMeeting(tomorrow)               // ✓

let yesterday = time.now() - 1d
scheduleMeeting(yesterday)              // ✗ Precondition violated
```

### Memory Safety (Low mode)

```hilow
manual let buffer = alloc(1024)

function process() {
  free(buffer)
}

process()
let x = buffer[0]                   // ✗ Proof error: use after free
```

### Currency Type Safety

```hilow
function calculateTax(price: money<USD>, rate: f64): money<USD> {
  return price * rate
}

let euro = 50.00 EUR
let bad = calculateTax(euro, 0.08)  // ✗ Currency mismatch
```

### Proof Modes

The proof system is **optional and layered**. Compilation produces a runnable binary regardless of proof status; verification is a separate concern that can be turned up or down based on where you are in development.

```bash
# Normal compilation (no proof checking)
hilowc program.hl -o program

# Compile + verify (warnings, not errors)
hilowc program.hl --prove -o program

# Sample output:
# ✓ 14 constraints proven statically
# ⚠ 3 constraints fall back to runtime checks (see lines 23, 67, 91)
# ⚠ 1 unprovable assertion (line 134) — could not determine
# ✗ 1 constraint violation (line 89) — value out of bounds
# Compiled successfully with 5 verification warnings.

# Compile + verify in strict mode (warnings become errors)
hilowc program.hl --prove --strict -o program
# Same output, but compilation fails if any non-✓ items appear.

# Verify only, no compilation
hilowc program.hl --prove-only

# Suggestions for improvement
hilowc program.hl --prove --suggest

# Sample output may include:
# 💡 Suggestion line 23: constraint is always true; consider removing
# 💡 Suggestion line 67: use i32 instead of f64 (no fractional values seen)
```

There are four proof outcomes for each constraint or contract:

- **✓ Proven**: the prover statically verified this property
- **⚠ Runtime-checked**: the property cannot be proven statically (because it depends on runtime values), so the compiler inserts a runtime check
- **⚠ Unprovable**: the property is too complex for the prover, no runtime check is feasible — the property is documented but unverified
- **✗ Violated**: the prover found a path that violates the property

In normal `--prove` mode, only ✗ violations cause issues you can ignore (they print but don't fail compilation). In `--strict` mode, anything other than ✓ fails the build. This lets you:

1. **Develop with `--prove`** — see warnings as you work, fix at your own pace
2. **Gate releases with `--strict`** — your CI requires all properties verified
3. **Skip proofs during exploration** — bare `hilowc` ignores all proof clauses

The optional, layered approach means adding a constraint never breaks an existing build. You discover its impact through warnings, not failures.

### Runtime Checks vs Static Proofs

When a constraint references runtime-only values, static proof is impossible — the prover doesn't know what those values will be. In this case, the compiler emits a runtime check instead:

```hilow
let port: u16 in {1024..max_port}    // max_port may be runtime

// At assignment, the compiler emits:
//   if not (port >= 1024 and port <= max_port):
//     handle_constraint_violation()
```

Runtime-checked constraints are weaker — they detect violations only when the offending code runs, not at compile time. The `--prove` output makes the distinction clear so programmers know which guarantees are static and which depend on testing.

For maximum static verification, use literal values or `const` declarations in constraints; for flexibility, accept runtime checks.

### Gradual Verification

Start without proofs, add them incrementally:

```hilow
// No proofs
function divide(a, b) {
  return a / b
}

// Basic precondition
function divide(a: i32, b: i32): i32
  requires (b != 0)
{
  return a / b
}

// Full contract
function divide(a: i32, b: i32): i32
  requires (b != 0)
  ensures (result * b <= a and result * b + b > a)
{
  return a / b
}
```

## Modules

### Module Files

A module file declares its mode at the top:

```hilow
// math.hl
high module {
  export function add(a: i32, b: i32): i32 {
    return a + b
  }
  
  export function subtract(a: i32, b: i32): i32 {
    return a - b
  }
  
  export let PI: f64 = 3.14159
  
  // Private (not exported)
  function helper() {
    // Internal use only
  }
}
```

### Imports

```hilow
// main.hl
import { add, subtract, PI } from "./math"

high program(): i32 {
  let sum = add(5, 3)
  let diff = subtract(10, 4)
  print(f"PI = {PI}")
  return 0
}
```

### Mode-Crossing Imports

A High program can import from a Low module — the Low functions are called normally:

```hilow
import { fastEncode, fastDecode } from "./codec"  // codec is a low module

high program(): i32 {
  let data = readInput()
  let encoded = fastEncode(data, data.length)     // calls into low
  print(f"Encoded {encoded.length} bytes")
  return 0
}
```

A Low module can import from another Low module freely. A Low module importing from a High module can only use functions marked `@low-callable`.

### Module Rules

- Only named exports (no default exports)
- No namespace imports (no `import * as`)
- No dynamic imports
- Module files are either entirely `high module` or `low module` — no mixing at the module level (use function-level overrides for that)

## Destructuring

### Array Destructuring

```hilow
let array = [1, 2, 3, 4, 5]
let [first, second] = array

// With rest
let [head, ...tail] = array

// Swapping
let a = 1
let b = 2
[a, b] = [b, a]
```

### Object Destructuring (High mode)

```hilow
let point = { x: 10, y: 20 }
let { x, y } = point

// With different names
let { x: posX, y: posY } = point

// With defaults
let { x, y, z = 0 } = point
```

### Tuple Destructuring

```hilow
function divmod(a: i32, b: i32): (i32, i32) {
  return (a / b, a % b)
}

let (quotient, remainder) = divmod(10, 3)
```

### Function Parameters

```hilow
function distance({ x, y }: object): f64 {
  return sqrt(x * x + y * y)
}

distance({ x: 3, y: 4 })            // 5.0
```

### No Complex Patterns

To keep the language small:

```hilow
let { x: newX, ...rest } = obj      // ✗ Rest in objects
let [first, , third] = array        // ✗ Skipping elements
let { a: { b: { c } } } = obj       // ✗ Deep nesting
```

## Inline Assembly (Low mode)

```hilow
low function getTimestamp(): u64 {
  let result: u64
  
  asm {
    rdtsc
    shl rdx, 32
    or rax, rdx
    mov [result], rax
  }
  
  return result
}

low function atomicIncrement(ptr: *u64): u64 {
  let result: u64
  
  asm {
    mov rax, 1
    lock xadd [ptr], rax
    mov [result], rax
  }
  
  return result
}
```

Inline assembly is platform-specific. The compiler verifies that variables referenced in `asm` blocks are accessible and that types match assembly operand sizes.

## Statement Termination

HiLow uses optional semicolons, JavaScript-style. A semicolon is required only where syntactic ambiguity demands it:

```hilow
// Both valid
let x = 5
let y = 10

let x = 5;
let y = 10;

// Required only when statements are on the same line
let x = 5; let y = 10

// Or when a line could continue
let result = computeSomething(
  argument1,
  argument2
)                                   // No semicolon needed
```

Most code uses no semicolons. Add them when continuing onto another statement on the same line, or when the parser's behavior at line boundaries would be ambiguous.

## Standard Library

### I/O

```hilow
// Console output (both modes)
print("Hello")
print(f"Value: {x}")

// File operations (high mode)
let file = openFile("data.txt")
if (file is unknown) {
  print(f"Error: {file.reason}")
  return
}

let content = file.read()
if (content is unknown) {
  print(f"Read error: {content.reason}")
  file.close()
  return
}

file.close()
```

### HTTP (High mode)

```hilow
let response = http.get("https://api.example.com/data")

if (response is unknown) {
  print(f"Request failed: {response.reason}")
  return
}

if (response.status != 200) {
  print(f"HTTP {response.status}")
  return
}

let data = response.body
```

### Math (both modes)

```hilow
let x = abs(-5)                     // 5
let y = sqrt(16)                    // 4.0
let z = pow(2, 8)                   // 256
let a = sin(PI / 2)                 // 1.0
let b = cos(0)                      // 1.0
let c = floor(3.7)                  // 3
let d = ceil(3.2)                   // 4
let e = round(3.5)                  // 4
```

### String Operations (both modes)

```hilow
let s = "hello world"

s.length                            // 11
s.indexOf("world")                  // 6
s.indexOf("xyz")                    // -1
s.slice(0, 5)                       // "hello"
s.slice(6)                          // "world"

let parts = s.split(" ")            // ["hello", "world"]
let joined = parts.join("-")        // "hello-world"

s.replace("world", "there")         // "hello there"
s.toUpperCase()                     // "HELLO WORLD"
s.toLowerCase()                     // "hello world"
"  text  ".trim()                   // "text"
```

### Reflection (High mode only)

```hilow
let obj = { name: "Alice", age: 30 }

// Iterate properties
for (let (key, value) in obj) {
  print(f"{key}: {value}")
}

// Check property existence
if ("name" in obj) {
  print("has name")
}

// Dynamic property access
let key = "name"
let value = obj[key]
```

## Compilation

### Compiler Invocation

```bash
# Compile to executable
hilowc program.hl -o program

# With optimizations
hilowc program.hl -O2 -o program

# With proof checking
hilowc program.hl --prove -o program

# Proof only, no compilation
hilowc program.hl --prove

# Generate LLVM IR
hilowc program.hl --emit-llvm -o program.ll

# Cross-compilation
hilowc program.hl --target=arm64-linux -o program

# Embedded target (no stdlib)
hilowc program.hl --target=riscv32-bare --no-stdlib -o firmware
```

### Compilation Targets

- Native executables (no runtime required)
- LLVM IR for optimization
- WebAssembly for browsers
- Static libraries
- Dynamic libraries
- Bare-metal targets (with `--no-stdlib`)

### Build System

```toml
# hilow.toml
[package]
name = "myapp"
version = "1.0.0"

[build]
entry = "src/main.hl"
output = "build/myapp"
optimize = "2"

[verify]
prove = true
strict = true

[dependencies]
http = "1.2.0"
json = "0.9.0"
```

## Example Programs

### Hello, HiLow World!

```hilow
high program(): i32 {
  print("Hello, HiLow World!")
  return 0
}
```

### HTTP Server (High mode)

```hilow
high program(): i32 {
  let server = http.listen("0.0.0.0:8080")
  
  if (server is unknown) {
    print(f"Failed to start: {server.reason}")
    return 1
  }
  
  print("Server listening on :8080")
  
  let connections: [object]
  
  watcher w((added=added)server.connections) {
    for (let conn in added) {
      connections.push(conn)
      
      watcher cw((added=added)conn.requests) {
        for (let req in added) {
          let resp = handleRequest(req)
          conn.send(resp)
        }
      }
    }
  }
  
  loop {
    // Event loop - watchers handle the work
  }
  
  return 0
}

function handleRequest(req: object): object {
  return {
    status: 200,
    body: f"Hello from HiLow at {time.now()}"
  }
}
```

### Banking System (High mode with contracts)

```hilow
high program(): i32 {
  let balance: money<USD> (balance >= 0.00 USD) = 1000.00 USD
  
  function withdraw(amount: money<USD>): bool
    requires (amount >= 0.00 USD)
    ensures (result ?= true implies balance >= 0.00 USD)
  {
    if (amount > balance) {
      return false
    }
    balance = balance - amount
    return true
  }
  
  function deposit(amount: money<USD>): bool
    requires (amount >= 0.00 USD)
    ensures (balance >= 0.00 USD)
  {
    balance = balance + amount
    return true
  }
  
  print(f"Initial balance: {balance}")
  
  if (withdraw(200.00 USD)) {
    print(f"After withdrawal: {balance}")
  }
  
  deposit(500.00 USD)
  print(f"After deposit: {balance}")
  
  return 0
}
```

### Mixed Mode: High App with Low Codec

```hilow
// app.hl
import { fastEncode, fastDecode } from "./codec"

high program(): i32 {
  let server = http.listen("0.0.0.0:8080")
  
  watcher w((added=added)server.requests) {
    for (let req in added) {
      let body = req.body
      
      // Drop into low for the hot path
      let encoded = fastEncode(body, body.length)
      
      server.respond(req, {
        status: 200,
        body: encoded
      })
    }
  }
  
  loop { }
  return 0
}
```

```hilow
// codec.hl
low module {
  export function fastEncode(input: *u8, len: usize): *u8 {
    let output = alloc(len * 2)
    
    for (let i: usize = 0; i < len; i += 1) {
      output[i * 2] = (input[i] >> 4) + 0x30
      output[i * 2 + 1] = (input[i] & 0x0F) + 0x30
    }
    
    return output
  }
  
  export function fastDecode(input: *u8, len: usize): *u8
    requires (len % 2 ?= 0)
  {
    let output = alloc(len / 2)
    
    for (let i: usize = 0; i < len / 2; i += 1) {
      let high_nib = input[i * 2] - 0x30
      let low_nib = input[i * 2 + 1] - 0x30
      output[i] = (high_nib << 4) | low_nib
    }
    
    return output
  }
}
```

### Reactive Counter (High mode)

```hilow
high program(): i32 {
  shared let counter: i32 (counter >= 0) = 0
  
  // Spawn workers
  for (let i = 0; i < 10; i += 1) {
    async {
      for (let j = 0; j < 100; j += 1) {
        counter += 1
      }
    }
  }
  
  // Watcher for completion
  let w = watcher(counter) {
    print(f"Counter: {counter}")
    
    if (counter >= 1000) {
      print("All workers complete!")
      w.end()
    }
  }
  
  while (w.isActive()) {
    // Wait for completion
  }
  
  return 0
}
```

### Bare-Metal Bootloader Snippet (Low mode)

```hilow
low program(): i32 {
  // Initialize hardware - direct memory access
  let uart_base: *u32 = address_of(0x4000C000)
  uart_base[0] = 0x00000001                   // Enable UART
  
  let message = "HiLow boot\n"
  for (let i: usize = 0; i < message.length; i += 1) {
    while ((uart_base[1] & 0x80) ?= 0) {
      // Wait for transmit ready
    }
    uart_base[2] = message[i]
  }
  
  return 0
}
```

## Design Rationale

This section documents the reasoning behind HiLow's non-obvious design choices.

### Why Two Modes Instead of Two Languages

HiLow originally considered separating into two languages, "High" and "Low." On reflection, the differences between application and systems programming aren't deep enough to justify two languages — they share syntax, operators, control flow, and most semantics. What differs is which features are available and what the compiler enforces. That's a configuration, not a language difference.

The mode system gives you the benefits of separation (clear distinction between application and systems code, no unexpected feature overlap) without the cost (two specifications to maintain, two compilers, fragmented ecosystem).

### Why `program` Instead of `main()`

Other languages put the entry point in a function called `main` and pile conventions on top — fixed name, special signature, implicit project-level wrapper. HiLow makes the entry point explicit at the syntactic level: `high program(...)` or `low program(...)` says exactly what this is. The mode is part of the declaration because the mode is a property of the program, not a project-level setting.

This also eliminates the need for a separate file-level mode directive — the `program` block declares what kind of program this is.

### Why `?=` Instead of `==`

The classic `=` vs `==` typo is a real bug source. `if (x = 5)` looks almost identical to `if (x == 5)` to a tired reader. HiLow uses `?=` for equality so the operator is visually distinct from assignment — the leading `?` carries meaning ("this is a question") and breaks the visual symmetry that makes the typo possible.

`!=` for inequality is preserved (familiar, universal across languages, no assignment-confusion risk). The asymmetry with `?=` is intentional — requiring something like `?!=` would be needless friction.

Bare `==` is rejected with a compile error directing the user to `?=` for equality or `=` for assignment. This catches typos in both directions and prevents JavaScript-trained users from silently writing operators that mean something different in HiLow.

### Why `(qualifier)=` for Approximate and Domain-Specific Comparisons

Earlier drafts of HiLow used `~=` for approximate equality. This was dropped in favor of `(qualifier)=`. The reasoning: approximate equality is rare enough that a dedicated symbol is wasteful, and the *kinds* of approximation vary too much to fit one symbol. Numeric tolerance, case-insensitive strings, same-calendar-day for time, currency-conversion-equality for money — all are conceptually "loose equality" but require different parameters and behaviors.

The qualifier form names the kind of approximation explicitly: `(roughly)=`, `(caseless)=`, `(within: 0.01)=`, `(same-day)=`. This is more verbose for the rare case but eliminates ambiguity, stays open-ended for new equality kinds without grammar changes, and lets the qualifier carry parameters when needed. The same form handles unusual *assignment* operations: `(or)=`, `(atomic-add)=`, `(saturating-add)=`, `(volatile)=`.

Qualifier names are contextual identifiers — they live only inside the `(...)=` position and don't pollute the keyword namespace. Multiple qualifiers can be combined with commas: `(caseless, trimmed)=` means "compare case-insensitively after trimming whitespace."

### Why No Type Coercion

JavaScript's coercion (`"5" * 2 == 10`) is convenient for short scripts but a quiet bug source in larger systems. The cases where coercion was useful (string formatting, mostly) are now handled by f-strings. The cases where conversion is genuinely needed are explicit (`parseInt(s)`, `f64(i32_value)`).

In a language that compiles to native code with formal verification, coercion would also undermine the proof system — every comparison and arithmetic operation would have hidden type-conversion semantics that the prover would have to reason about.

### Why `is` for Type Tests

Equality (`?=`, `(qualifier)=`) compares values. Type/prototype membership is a different question. Conflating them via overloaded equality creates ambiguity (is `unknown` a type or a value?). `is` is short, reads naturally, and works for both type checks and prototype membership: `if (result is unknown)`, `if (dog is animal)`.

### Why Prototype Objects in High Mode

JavaScript's prototype model is cleaner than its class model — classes are just sugar over prototypes, and the sugar adds complexity without gaining capability. Prototype delegation is genuinely simple: each object has one prototype; lookup walks the chain. This handles inheritance, mixins, and dynamic dispatch with one mechanism.

For Low mode, flexible objects with dynamic dispatch don't fit the predictability requirements — you can't compute fixed memory layout for an object whose shape can change. Low uses fixed structs.

### Why No Closures in Low Mode

Closures that capture by reference and outlive their defining scope require either heap allocation (and refcounting) or escape analysis. Low mode avoids both — captured variables would force memory management complexity that Low explicitly rejects. Closures can still be used for callbacks within a function (they don't escape); they just can't be returned or stored.

### Why `function` Instead of `fn`

`function` is longer to type but easier to scan. In a language where function declarations are visually heavy (they have type annotations, parameters, returns, contracts), the keyword length is the smallest cost. The benefit is that "function" stands out clearly in source listings, and the keyword is unambiguous to readers from any language background.

### Why `nothing` and `unknown` Are Distinct

Other languages conflate "no value" (uninitialized, missing) with "operation failed." This is a category error: an uninitialized variable hasn't tried to do anything, while a failed operation has tried and failed with specific reasons.

`nothing` represents true absence — the variable was never set, the property doesn't exist, the memory was deallocated. `unknown` represents failure with context — the operation tried, didn't succeed, and carries the reason and possible solutions. Treating these as the same value (as JS's `null`/`undefined` does, or as Rust's `None` does for everything) loses information and creates ambiguous error handling.

### Why `watcher` Instead of async/await or Promises

Async/await and Promises are abstractions that require runtime support and complicate the type system (every async function returns a `Promise<T>` that must be awaited). They also separate "things that happen over time" into a special category, when reactive programming applies more broadly than just async I/O.

`watcher` is a single primitive: "when these subscribed values change, run this code." It handles async (watch a result variable), events (watch an event queue with `(added)`), reactive UI (watch state), and concurrency (watch shared variables across processes) with one mechanism. It compiles to ordinary callbacks at runtime — no async runtime required.

### Why First-Class Time and Money

These are the two most common "almost first-class" types in real applications, and the most common sources of subtle bugs when handled by libraries. Time has timezones, calendars, durations, and arithmetic with non-obvious rules. Money has currency, rounding, allocation, and arithmetic that requires same-currency operands.

Building these into the language ensures consistency (every library uses the same `time` and `money` types), enables special syntax (duration literals like `2h + 30m`, currency literals like `19.99 USD`), and lets the proof system reason about them (compile-time currency checks, time ordering constraints).

### Why Optional Semicolons

JavaScript's automatic semicolon insertion has corner cases that surprise programmers. HiLow's rule is simpler: semicolons are optional, used only when needed for disambiguation (multiple statements on a line). Most code has no semicolons. The cases where semicolons matter are clear from context.

### Why Parens for Tuple Destructuring

`let a, b = divmod(10, 3)` is ambiguous: is it `let a` (declared) and `b = ...` (assigned), or destructuring? Parens make the intent unambiguous: `let (a, b) = divmod(10, 3)`. This matches the tuple type syntax `(i32, i32)` for symmetry.

## Language Summary

**Core Features:**
- Compiled to native code (no runtime, no GC)
- Two modes: High (application) and Low (systems)
- Static typing without coercion
- Prototype-based objects in High; fixed structs in both
- First-class functions and closures (closures: High only when escaping)
- Scope-based memory ownership; refcounting in High when needed
- Optional formal verification

**Special Types:**
- `nothing` - absence, uninitialized, deallocated
- `unknown` - rich failure information with reason and options
- `time` - first-class time, durations, calendars, timezones
- `money` - currency-safe financial operations

**Distinctive Features:**
- Quote recursion for strings (no escaping quotes)
- F-strings without backticks (Python-style)
- `watcher` for reactive programming, `stealth { }` for suppressed mutations
- Equality operator `?=`, inequality `!=` (no bare `==`)
- Negation comparators `!<` and `!>` for invariant-style readability
- Type test operator `is`
- Qualified operators `(qualifier)=` for domain-specific operations including `(coerce)=` for explicit type conversion
- Logical operators `and`, `or`, `not`
- Constraint-based verification with predicate or set form
- Function contracts with `requires`/`ensures`/`invariant`/`decreases`
- Layered, optional proof system (warnings by default, errors with `--strict`)

**Mode Boundary:**
- `high program` / `low program` declares entry point mode
- `high module` / `low module` declares library mode
- `high function` / `low function` overrides at function level
- `high { }` / `low { }` overrides at block level
- High calls Low freely; Low calls High only with `@low-callable`

**Design Goals:**
- One language for everything from device drivers to web applications
- JS-comfort in High mode; C-power in Low mode
- Small surface area; one obvious way to do things
- Pragmatic correctness; opt-in proof system
- No runtime, no GC, predictable execution

HiLow bridges systems and application programming with a single language — write your application in High, drop into Low for hot paths or hardware access, and ship one binary with no runtime dependencies.
