# HiLow Programming Language Design

HiLow is a compiled programming language with two modes: **High** for application development and **Low** for systems programming. Both modes share the same syntax, operators, and core semantics — they differ in which features are available and what the compiler enforces. A single program can mix both modes naturally, dropping into Low for performance-critical sections or hardware access while staying in High for everything else.

## Design Principles

- **One language, two modes**: High and Low share syntax and most semantics; mode determines which features are available
- **JS-comfort, systems-power**: Application code feels like JavaScript; systems code has C-level control
- **No type coercion**: Strong typing without implicit conversions in either mode
- **No runtime, no GC**: Both modes compile to native code with predictable execution
- **Explicit reactive primitive**: `watch()` for event-driven and concurrent code in both modes
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
- All operators including `?=`, `~=`, `is`, `(qualifier)=`
- All control flow constructs
- F-strings and quote recursion
- `watch()` reactive primitive
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
default   defer     else      ensures   export    false
for       function  heap      high      if        import
in        is        let       loop      low       manual
match     module    not       nothing   or        program
requires  return    shared    stack     switch    this
true      unknown   when      while     watch
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

In High mode, prototype-based flexible objects work like JavaScript — properties can be added or removed dynamically, and prototypes provide delegation.

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

HiLow distinguishes three kinds of equality, each with a distinct operator family:

```hilow
// Strict equality - types must match exactly
x ?= y         // Equal
x ?!= y        // Not equal

// Approximate equality - for numeric tolerance, case-insensitive strings, etc.
x ~= y         // Approximately equal
x ~!= y        // Not approximately equal

// Type/prototype membership
x is T         // x is of type T (or has T in prototype chain)
x is not T     // x is not of type T

// Ordering
x < y          // Less than
x > y          // Greater than
x <= y         // Less than or equal
x >= y         // Greater than or equal
```

The leading marker (`?` or `~`) makes the equality operator visually distinct from assignment, eliminating the classic `=` vs `==` typo bug. The negation marker `!` is consistent across both operator families: `?!=` and `~!=`.

### Qualified Equality

For unusual or domain-specific comparisons, the `(qualifier)=` form is expressive and extensible:

```hilow
// Float tolerance
if (a (within: 0.01)= b) {
  print("Close enough")
}

// Case-insensitive string comparison
if (s1 (case-insensitive)= s2) {
  print("Match (any case)")
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
```

The qualifier name is interpreted by the type involved — `time` knows about `same-day` and `within`, `string` knows about `case-insensitive`, etc. New qualifiers can be defined for user types.

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
x === y                     // HiLow uses ?= (strict by default)
x == y (with coercion)      // HiLow has no coercion; use ~= for approximate
typeof x                    // Use: x is T
?:  (ternary)               // Use if/else
```

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
  requires (b ?!= 0)
  ensures (result * b <= a and result * b + b > a)
{
  return a / b
}

// Prover checks at call sites
let x = divide(10, 5)               // ✓ Proven safe
let y = divide(10, 0)               // ✗ Proof error: precondition violated

// Prover understands control flow
let divisor = getUserInput()
if (divisor ?!= 0) {
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

### Low Mode: Explicit Memory Modes

Low mode does not insert refcounting. Sharing requires explicit choice:

```hilow
// Default: scope-based, single owner
let buf = alloc(1024)
// freed at scope exit

// Manual: you control the lifetime
manual let graph = alloc(size_of<Graph>)
defer free(graph)
// freed when defer runs

// Refcounted: opt-in shared ownership
shared let resource = rc_alloc<Connection>()
// freed when last reference drops

// Arena: bulk allocation, freed all at once
arena {
  let a = arena.alloc(100)
  let b = arena.alloc(200)
  // ... no individual frees ...
}                                   // entire arena freed here
```

The four modes cover the full spectrum:
- **Default (scope)**: single owner, automatic cleanup, no overhead
- **Manual**: explicit control, for unusual lifetimes
- **Shared (refcount)**: shared ownership when needed, opt-in cost
- **Arena**: bulk allocation for batched work

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

```hilow
function process() {
  manual let resource = allocateResource()
  defer free(resource)              // Runs at scope exit
  
  if (errorCondition) {
    return                          // resource is freed
  }
  
  doWork(resource)
}                                   // resource is freed
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

## Watch System

`watch()` is HiLow's primitive for reactive programming. It expresses "when these values change, run this code." It's available in both modes and underlies HiLow's approach to async, concurrency, and event handling.

### Basic Watch

```hilow
let balance = 1000

let watcher = watch(balance) {
  print(f"Balance changed to: {balance}")
}

balance = 2000                      // Triggers watch

// Lifecycle
watcher.pause()
balance = 3000                      // No trigger
watcher.resume()
balance = 4000                      // Triggers
watcher.end()
balance = 5000                      // Never triggers again
```

### Multiple Variables

```hilow
let x = 0
let y = 0

let w = watch(x, y) {
  print(f"x={x}, y={y}")
}

x = 10                              // Triggers
y = 20                              // Triggers
```

### No Self-Triggering

A watch does not re-trigger from modifications made within its own body:

```hilow
let counter = 0

let w = watch(counter) {
  counter = counter + 1             // Does NOT cause recursion
}

counter = 10                        // Triggers once, counter becomes 11
```

### Async Operations

```hilow
let response = nothing

let w = watch(response) {
  if (response is nothing) return
  print(f"Got response: {response}")
}

async {
  response = http.get("https://api.example.com/data")
}
```

The `async { }` block runs concurrently. When `response` is assigned, the watcher fires.

### Cross-Process Watches

Variables marked `shared` can be watched across processes:

```hilow
shared let counter = 0

// Process 1
async {
  for (let i = 0; i < 100; i += 1) {
    counter += 1
  }
}

// Process 2
let w = watch(counter) {
  print(f"Counter: {counter}")
  if (counter >= 100) {
    print("Done!")
    w.end()
  }
}
```

### Conditions Inside Watches

```hilow
let enabled = true
let value = 0

let w = watch(value, enabled) {
  if (not enabled) return
  print(f"Value: {value}")
}

enabled = false
value = 100                         // Watch fires but returns early

enabled = true
value = 200                         // Watch fires and prints
```

### Watch Verification

The proof system detects circular watch dependencies at compile time:

```hilow
let a = 0
let b = 0

watch w1(a) {
  b = a + 1
}

watch w2(b) {
  a = b + 1                         // ✗ Proof error: circular dependency
}
```

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
  
  if (response.status ?!= 200) {
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

Constraints define valid ranges for values. The variable being constrained is referenced by its name:

```hilow
let percent: i32 (percent >= 1 and percent <= 100) = 50
let temperature: f32 (temperature >= -273.15)
let port: u16 (port >= 1024)
let balance: money (balance >= 0.00 USD)

// Range sugar for the common case
let percent: i32 in 1..100 = 50
let port: u16 in 1024..65535
```

The prover verifies assignments:

```hilow
percent = 150                       // ✗ Proof error: violates constraint
percent = 75                        // ✓ Proven safe
```

### Function Contracts

```hilow
function divide(a: i32, b: i32): i32
  requires (b ?!= 0)
  ensures (result * b <= a and result * b + b > a)
{
  return a / b
}

let x = divide(10, 5)               // ✓
let y = divide(10, 0)               // ✗ Proof error: precondition violated

let divisor = getUserInput()
if (divisor ?!= 0) {
  let z = divide(100, divisor)      // ✓ Prover follows control flow
}
```

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

```bash
# Normal compilation (no proof checking)
hilowc program.hl -o program

# Verify all constraints
hilowc program.hl --prove

# Output:
# ✓ All variable constraints verified
# ✓ All function contracts satisfied
# ✓ All unknown returns handled
# ✓ No circular watch dependencies
# ✓ All memory deallocations verified
# ✓ Array bounds checked
# ✓ Currency types verified

# Suggestions for improvement
hilowc program.hl --prove --suggest

# Output may include:
# 💡 Suggestion: line 23 - constraint always true
# 💡 Suggestion: line 67 - use i32 instead of f64
```

### Gradual Verification

Start without proofs, add them incrementally:

```hilow
// No proofs
function divide(a, b) {
  return a / b
}

// Basic precondition
function divide(a: i32, b: i32): i32
  requires (b ?!= 0)
{
  return a / b
}

// Full contract
function divide(a: i32, b: i32): i32
  requires (b ?!= 0)
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

if (response.status ?!= 200) {
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
  
  watch w(server.connections) {
    if (server.connections.length > 0) {
      let conn = server.connections.pop()
      connections.push(conn)
      
      watch cw(conn.requests) {
        if (conn.requests.length > 0) {
          let req = conn.requests.pop()
          let resp = handleRequest(req)
          conn.send(resp)
        }
      }
    }
  }
  
  loop {
    // Event loop - watches handle the work
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
  
  watch w(server.requests) {
    if (server.requests.length > 0) {
      let req = server.requests.pop()
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
  
  // Watch for completion
  let w = watch(counter) {
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

`!=` for inequality is preserved (familiar, no ambiguity), with `?!=` for the strict inequality where consistency matters.

### Why `~=` for Approximate Equality

Languages with type coercion conflate "are these equal" with "are these *kind of* equal." HiLow separates them: `?=` is strict equality (types must match, values must match exactly), `~=` is approximate equality (numeric tolerance, case-insensitive, whatever the type defines). The `~` symbol reads as "fuzzy" or "approximately," which matches the semantics.

### Why `(qualifier)=` for Unusual Operations

Many useful operations don't fit comfortably into single-symbol operators. Atomic operations, saturating arithmetic, volatile access, domain-specific equality (same-day, within-tolerance, after-conversion) — each is rare enough that a dedicated operator would be wasteful, but common enough that calling functions feels heavy.

The `(qualifier)=` form gives unusual operations a clear, expressive syntax without consuming the limited supply of operator characters. It's open-ended — new qualifiers can be added without language changes.

### Why No Type Coercion

JavaScript's coercion (`"5" * 2 == 10`) is convenient for short scripts but a quiet bug source in larger systems. The cases where coercion was useful (string formatting, mostly) are now handled by f-strings. The cases where conversion is genuinely needed are explicit (`parseInt(s)`, `f64(i32_value)`).

In a language that compiles to native code with formal verification, coercion would also undermine the proof system — every comparison and arithmetic operation would have hidden type-conversion semantics that the prover would have to reason about.

### Why `is` for Type Tests

Equality (`?=`, `~=`) compares values. Type/prototype membership is a different question. Conflating them via overloaded equality creates ambiguity (is `unknown` a type or a value?). `is` is short, reads naturally, and works for both type checks and prototype membership: `if (result is unknown)`, `if (dog is animal)`.

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

### Why `watch()` Instead of async/await or Promises

Async/await and Promises are abstractions that require runtime support and complicate the type system (every async function returns a `Promise<T>` that must be awaited). They also separate "things that happen over time" into a special category, when reactive programming applies more broadly than just async I/O.

`watch()` is a single primitive: "when these values change, run this code." It handles async (watch a result variable), events (watch an event queue), reactive UI (watch state), and concurrency (watch shared variables across processes) with one mechanism. It compiles to ordinary callbacks at runtime — no async runtime required.

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
- `watch()` for reactive programming
- Equality operators `?=` (strict), `~=` (approximate)
- Type test operator `is`
- Qualified operators `(qualifier)=` for domain-specific operations
- Logical operators `and`, `or`, `not`
- Constraint-based verification
- Function contracts with `requires`/`ensures`

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
