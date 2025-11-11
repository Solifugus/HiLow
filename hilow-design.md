# HiLow Programming Language Design

HiLow is a compiled programming language that bridges systems programming and application development. It combines JavaScript's pragmatic ergonomics with formal verification, explicit memory management, and systems-level control.

## Design Principles

- **Pragmatic correctness**: Optional formal verification through constraints and contracts
- **Prototype-based objects**: No classes, pure prototype delegation
- **Explicit memory management**: Stack/heap allocation with scope-based lifetime
- **Reactive programming**: The `watch()` primitive for event-driven and concurrent code
- **Type safety with coercion**: Strong typing that coerces naturally where safe
- **Application-friendly**: First-class time, money, and error handling

## Philosophy

HiLow targets the gap between systems languages (C, Rust, Zig) and application languages (JavaScript, Python). It provides:

- **Low-level capabilities**: Direct memory access, inline assembly, no runtime
- **High-level ergonomics**: First-class types for time/money, string interpolation, reactive programming
- **One language**: Write device drivers and web applications with the same tool

## Hello World

```hilow
function main(args: [string]): i32 {
  print("Hello from HiLow!");
  return 0;
}
```

## Type System

### Primitive Types

```hilow
// Integers - explicit sizes
let a: i8 = -128;           // 8-bit signed
let b: u8 = 255;            // 8-bit unsigned
let c: i16 = -32768;        // 16-bit signed
let d: u16 = 65535;         // 16-bit unsigned
let e: i32 = -2147483648;   // 32-bit signed
let f: u32 = 4294967295;    // 32-bit unsigned
let g: i64;                 // 64-bit signed
let h: u64;                 // 64-bit unsigned
let i: i128;                // 128-bit signed
let j: u128;                // 128-bit unsigned

// Floating point
let x: f32 = 3.14;          // 32-bit float
let y: f64 = 2.71828;       // 64-bit double

// Boolean
let flag: bool = true;

// Strings (UTF-8 by default)
let name: string = "Alice";

// Type inference for locals
let inferred = 42;          // Inferred as i32
let pi = 3.14159;           // Inferred as f64
```

### Special Types

```hilow
// nothing - represents absence, uninitialized, or deallocated
let x;                      // x is nothing
let y = nothing;            // Explicit nothing

// unknown - rich error information
let result = someFunction(); // May return unknown
if (result ??= unknown) {
  print(f"Error: {result.reason}");
  print(f"Options: {result.options}");
}

// Creating unknown values
return unknown("file not found", options: ["check path", "create file"]);
```

### First-Class Types

#### Time

```hilow
// Time type (i64 nanoseconds internally)
let now: time = time.now();
let birthday: time = time.parse("1990-06-15T14:30:00Z");

// Duration literals
let later = now + 2h + 30m + 15s;
let tomorrow = now + 1d;
let precise = now + 500ms + 250us + 100ns;

// Calendar operations
let nextTuesday = now.next(.tuesday);
let secondTuesday = now.month().nthWeekday(2, .tuesday);
let endOfMonth = now.month().end();

// Comparisons
if (meeting > now and meeting < now + 1h) {
  print("Meeting within the hour");
}

// Formatting
let formatted = now.format("YYYY-MM-DD HH:mm:ss");
let iso = now.toISO();

// Duration type
let elapsed: duration = endTime - startTime;
print(elapsed.hours());     // 2.5
print(elapsed.minutes());   // 150

// Iteration
for (let day = startDate; day <= endDate; day += 1d) {
  print(day.format("YYYY-MM-DD"));
}

// Timezone support
let ny = time.now(.timezone("America/New_York"));
let tokyo = ny.in(.timezone("Asia/Tokyo"));
```

#### Money

```hilow
// Money type with currency
let price: money = 19.99 USD;
let euro: money = 50.00 EUR;
let yen: money = 1000 JPY;

// Arithmetic (same currency only)
let total = price + 5.00 USD;
let doubled = price * 2;
let split = total / 3;

// Currency mixing is compile error
let bad = price + euro;     // ✗ Error: cannot add USD and EUR

// Explicit conversion
let converted = euro.convert(USD, rate: 1.08);

// Display formatting (respects currency conventions)
print(price);               // "$19.99"
print(euro.format());       // "€50.00"
print(yen.format());        // "¥1,000"

// Precision: display + 4 decimal places internally
// USD: display 2, store 6 (19.990000)
// JPY: display 0, store 4 (1000.0000)

// Rounding modes
let tax = price * 0.08;
let rounded = tax.round(.halfUp);
let bankers = tax.round(.halfEven);

// Allocation (guarantees sum equals original)
let bill = 100.00 USD;
let split = bill.allocate([1, 1, 1]);  // [33.34, 33.33, 33.33]

// Type-safe currency
function calculateTax(price: money<USD>, rate: f64): money<USD> {
  return price * rate;
}
```

### Arrays and Collections

```hilow
// Fixed-size arrays
let fixed: [i32; 10];       // 10 integers
let initialized = [1, 2, 3, 4, 5];

// Dynamic arrays
let dynamic: [i32];         // Growable array
dynamic.push(42);
dynamic.pop();

// Iteration
for (let item in array) {
  print(item);
}

for (let index, value in array) {
  print(f"[{index}] = {value}");
}

// No special array methods - use loops
let doubled = [];
for (let item in array) {
  doubled.push(item * 2);
}
```

### Objects (Prototype-based)

```hilow
// Object literals
let point = {
  x: 10,
  y: 20,
  proto: nothing
};

// Prototype delegation
let animal = {
  proto: nothing,
  speak: function() {
    print("some sound");
  }
};

let dog = {
  proto: animal,
  name: "Rover",
  speak: function() {
    print("woof");
  }
};

dog.speak();  // "woof"

// Property access
let value = obj.property;
let computed = obj[key];    // For dynamic keys only

// Adding methods
let obj = {};
obj.calculate = function(x) {
  return x * 2;
};

// Iteration
for (let key, value in obj) {
  print(f"{key}: {value}");
}
```

## String System

### Quote Recursion

```hilow
// Only double quotes - no single quotes or backticks
"simple string"

// Empty string
""

// Strings with quotes inside (use multiple quotes)
""My name is "Joe" and I'm happy""

// Deeper nesting
"""He said ""hello"" to me"""

// Multi-line strings
"
Line 1
Line 2
Line 3
"

// Rule: N adjacent quotes open, N adjacent quotes close
// Any quotes inside with count < N are literal
```

### F-Strings (Template Literals)

```hilow
// Python-style f-strings (no backticks, no $)
let name = "Alice";
let age = 30;
f"Hello {name}! You are {age} years old."

// Expressions
let price = 19.99;
f"Total: {price * 1.08}"

// Format specifiers
f"Price: {price:.2f}"           // "Price: 19.99"
f"Hex: {255:x}"                  // "Hex: ff"
f"Binary: {42:b}"                // "Binary: 101010"
f"Padded: {7:04d}"               // "Padded: 0007"

// Money formatting
let amount = 1234.56 USD;
f"Total: {amount}"               // "Total: $1,234.56"
f"Amount: {amount:.4f}"          // "Amount: $1,234.5600"

// Time formatting
let now = time.now();
f"Current: {now:YYYY-MM-DD}"
f"Time: {now:HH:mm:ss}"

// Alignment
f"|{value:>10}|"   // Right align
f"|{value:<10}|"   // Left align
f"|{value:^10}|"   // Center

// F-strings with quote recursion
f""Name: "Joe", Age: {age}""

// Multi-line f-strings
f"
  Dear {name},
  
  Your balance is {amount}.
"

// Raw strings (no escape processing)
r"C:\Users\Alice\Documents"
r"\d+\.\d+"         // For regex

// Raw f-strings
rf"Path: {userPath}\file.txt"  // {userPath} interpolates, \f is literal
```

### Escape Sequences

```hilow
// Minimal escapes (quote recursion handles most cases)
"\n"           // Newline
"\t"           // Tab
"\r"           // Carriage return
"\\"           // Backslash
"\u{1F600}"    // Unicode
"\x41"         // Hex byte

// But prefer quote recursion for quotes
""contains "quotes""   // Better than "contains \"quotes\""
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

### Assignment

```hilow
x = y          // Assignment
x += y         // Add and assign
x -= y         // Subtract and assign
x *= y         // Multiply and assign
x /= y         // Divide and assign
x %= y         // Modulo and assign
```

### Comparison

```hilow
x ?= y         // Equality (with type coercion)
x ??= y        // Strict equality (types must match)
x != y         // Inequality (with coercion)
x !!= y        // Strict inequality
x < y          // Less than
x > y          // Greater than
x <= y         // Less than or equal
x >= y         // Greater than or equal
```

### Logical

```hilow
x and y        // Logical AND (short-circuit)
x or y         // Logical OR (short-circuit)
not x          // Logical NOT

// Precedence: not > and > or
if (not x and y or z) { }  // ((not x) and y) or z

// Natural language operators improve readability
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

### Type Coercion

```hilow
// String concatenation
"Age: " + 30;          // "Age: 30"
"Count: " + true;      // "Count: true"

// Numeric context
"5" * 2;               // 10
"10" - 3;              // 7

// But comparison operators are explicit
5 ?= "5";              // true (with coercion)
5 ??= "5";             // false (strict, types differ)

// Truthy/falsy
if (value) { }         // 0, "", nothing, unknown are falsy
if (array.length) { }  // Empty array is falsy (length = 0)
```

### No Redundant Operators

```hilow
// Removed from JavaScript:
x++, x--, ++x, --x     // Use x += 1, x -= 1
x ||= y                // Use: if (not x) x = y;
x &&= y                // Use: if (x) x = y;
x ??= y                // Use explicit nothing check
x?.y                   // Use explicit checks
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

// No ternary operator - use if/else
```

### Switch Statements

```hilow
switch (value) {
  case 0:
    print("zero");
    break;
  case 1:
    print("one");
    break;
  default:
    print("other");
}

// Switch on strings
switch (command) {
  case "start":
    startServer();
    break;
  case "stop":
    stopServer();
    break;
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
  nothing => print("no value"),
  unknown => print(f"error: {result.reason}"),
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
// for loop - multiple patterns
for (let i = 0; i < 10; i += 1) {
  print(i);
}

for (let item in array) {
  print(item);
}

for (let index, value in array) {
  print(f"[{index}] = {value}");
}

for (let key, value in object) {
  print(f"{key}: {value}");
}

// while loop
while (condition) {
  // code
}

// Infinite loop
for (;;) {
  // code
  if (done) break;
}

// Loop control
break;     // Exit loop
continue;  // Next iteration
```

## Functions

### Function Declaration

```hilow
// Full "function" keyword (not fn)
function add(a: i32, b: i32): i32 {
  return a + b;
}

// Type inference for locals
function greet(name: string) {
  let message = f"Hello {name}";  // Type inferred
  print(message);
}

// Multiple returns
function divmod(a: i32, b: i32): (i32, i32) {
  return (a / b, a % b);
}

let quotient, remainder = divmod(10, 3);
```

### Function Expressions

```hilow
// Functions as values
let add = function(a: i32, b: i32): i32 {
  return a + b;
};

// Closures
function makeCounter(): function {
  let count = 0;
  return function(): i32 {
    count += 1;
    return count;
  };
}

let counter = makeCounter();
print(counter());  // 1
print(counter());  // 2
```

### Object Methods

```hilow
let obj = {};
obj.calculate = function(x: i32): i32 {
  return x * 2;
};

obj.calculate(5);  // 10

// Or inline
let point = {
  x: 10,
  y: 20,
  distance: function(): f64 {
    return sqrt(this.x * this.x + this.y * this.y);
  }
};
```

### Variadic Functions

```hilow
function sum(values: [i32]): i32 {
  let total = 0;
  for (let v in values) {
    total += v;
  }
  return total;
}

sum([1, 2, 3, 4, 5]);  // 15
```

## Memory Management

### Stack vs Heap

```hilow
// Stack allocation (automatic cleanup)
stack let buffer: [u8; 256];

// Heap allocation (manual management)
heap let data = alloc(1024);

// Default is stack for fixed-size, heap for dynamic
let array = [1, 2, 3];      // Stack
let dynamic: [i32];         // Heap
dynamic.push(42);
```

### Deallocation

```hilow
// Explicit deallocation
heap let buffer = alloc(1024);
// ... use buffer ...
buffer = nothing;  // Free memory

// Automatic with defer
heap let resource = allocateResource();
defer resource = nothing;
// Automatically freed at scope exit
```

### Scope-Based Lifetime

```hilow
{
  stack let temp = [1, 2, 3];
  // temp is valid here
}
// temp is automatically freed here

function process() {
  heap let data = alloc(1024);
  defer data = nothing;
  
  // Use data
  processData(data);
  
  // data freed here automatically
}
```

### Pointers

```hilow
// Explicit pointer types
let x: i32 = 42;
let ptr: *i32 = address(x);
let value = *ptr;  // Dereference

// Pointer arithmetic
let array = [1, 2, 3, 4, 5];
let ptr = address(array[0]);
ptr += 1;  // Points to array[1]
```

## Watch System (Reactive Programming)

### Basic Watch

```hilow
// Watch returns a handle
let balance = 1000;

let watcher = watch(balance) {
  print(f"Balance changed to: {balance}");
};

balance = 2000;  // Triggers watch

// Lifecycle management
watcher.pause();   // Temporarily disable
balance = 3000;    // Doesn't trigger

watcher.resume();  // Re-enable
balance = 4000;    // Triggers again

watcher.end();     // Permanently destroy
balance = 5000;    // Never triggers again
```

### Multiple Variables

```hilow
let x = 0;
let y = 0;

let w = watch(x, y) {
  print(f"x={x}, y={y}");
};

x = 10;  // Triggers
y = 20;  // Triggers
```

### Watch with Conditions

```hilow
let enabled = true;
let value = 0;

let w = watch(value, enabled) {
  if (not enabled) return;  // Manual gating
  print(f"Value: {value}");
};

enabled = false;
value = 100;  // Doesn't print

enabled = true;
value = 200;  // Prints
```

### No Self-Triggering

```hilow
let counter = 0;

let w = watch(counter) {
  // Modifying counter here does NOT re-trigger
  counter = counter + 1;
};

counter = 10;  // Triggers once, counter becomes 11
// Watch does NOT trigger again from internal modification
```

### Async Operations

```hilow
let response = nothing;

let w = watch(response) {
  if (response ??= nothing) return;
  print(f"Got response: {response}");
};

// Start async operation
async {
  response = http.get("https://api.example.com/data");
};
```

### Cross-Process Watches

```hilow
// Shared variable across processes
shared let counter = 0;

// Process 1
async {
  for (let i = 0; i < 100; i += 1) {
    counter += 1;
  }
}

// Process 2
let w = watch(counter) {
  print(f"Counter: {counter}");
  if (counter >= 100) {
    print("Done!");
    w.end();
  }
};
```

### Watch Collections

```hilow
// Managing multiple watches
let watchers = [];

watchers.push(watch(price) {
  print(f"Price: {price}");
});

watchers.push(watch(quantity) {
  print(f"Quantity: {quantity}");
});

// Pause all
for (let w in watchers) {
  w.pause();
}

// End all
for (let w in watchers) {
  w.end();
}
```

## Error Handling with Unknown

### Creating Unknown Values

```hilow
function divide(a: i32, b: i32): i32 | unknown {
  if (b ?= 0) {
    return unknown("division by zero", options: ["use different divisor"]);
  }
  return a / b;
}

function getUser(id: i32): object | unknown {
  let result = database.query(f"SELECT * FROM users WHERE id = {id}");
  
  if (not result) {
    return unknown("database error", options: ["retry", "check connection"]);
  }
  
  if (result.length ?= 0) {
    return unknown("user not found", options: ["check id", "create user"]);
  }
  
  return result[0];
}
```

### Checking Unknown

```hilow
let result = divide(10, 0);

// Strict check
if (result ??= unknown) {
  print(f"Error: {result.reason}");
  print(f"Options: {result.options}");
  return;
}

// Truthy check (unknown is falsy)
if (not result) {
  print(f"Failed: {result.reason}");
  return;
}

print(f"Result: {result}");
```

### Unknown Properties

```hilow
// unknown has two properties
unknown.reason: string      // Why it failed
unknown.options: [string]   // Possible solutions

// Example
let user = getUser(999);
if (user ??= unknown) {
  print(user.reason);       // "user not found"
  print(user.options[0]);   // "check id"
}
```

### Unknown Propagation

```hilow
// unknown propagates through property access
let user = getUser(999);  // Returns unknown

user.name;                // unknown (same instance)
user.address.street;      // unknown (propagates)

// Safe to chain
let street = user.address.street;
if (street) {
  print(f"Street: {street}");
} else {
  print("No street available");
}
```

### Logic Based on Unknown

```hilow
function fetchData(url: string): object | unknown {
  let response = http.get(url);
  
  if (response.status != 200) {
    if (response.status ?= 404) {
      return unknown("not found", options: ["check url", "try alternate"]);
    } else if (response.status ?= 500) {
      return unknown("server error", options: ["retry", "contact admin"]);
    } else {
      return unknown(f"http error {response.status}", options: ["retry"]);
    }
  }
  
  return response.body;
}

// Handle based on reason
let data = fetchData("https://api.example.com/data");

if (data ??= unknown) {
  if (data.reason ?= "not found") {
    print("Resource doesn't exist");
  } else if (data.reason ?= "server error") {
    // Retry logic
    data = fetchData("https://api.example.com/data");
  } else {
    print(f"Unknown error: {data.reason}");
  }
}
```

## Nothing vs Unknown

### Nothing - True Absence

```hilow
// Uninitialized
let x;                    // x is nothing
print(x);                 // "nothing"

// Non-existent property
let obj = { a: 1 };
obj.b;                    // nothing

// Explicit deallocation
heap let data = alloc(1024);
data = nothing;           // Free memory

// Type checking
if (x ??= nothing) {
  print("x has no value");
}

// Falsy
if (not x) {
  print("x is nothing or other falsy");
}
```

### Unknown - Rich Failure

```hilow
// Function failure with context
let result = failedOperation();

if (result ??= unknown) {
  print(result.reason);   // Why it failed
  print(result.options);  // What to do
}

// Also falsy
if (not result) {
  handleError(result);
}
```

### Distinguishing Them

```hilow
let a;                    // nothing
let b = someOperation();  // might be unknown

// Strict checks distinguish
if (a ??= nothing) {
  print("truly absent");
}

if (b ??= unknown) {
  print(f"failed: {b.reason}");
}

// Both falsy
if (not a) { }  // true
if (not b) { }  // true (if unknown)
```

## Formal Verification

### Variable Constraints

```hilow
// Constraints define valid ranges
let percent: i32 (percent >= 1 and percent <= 100) = 50;
let temperature: f32 (temperature >= -273.15);
let port: u16 (port >= 1024);
let balance: money (balance >= 0.00 USD);

// Prover verifies assignments
percent = 150;  // ✗ Proof error: violates constraint
percent = 75;   // ✓ Proven safe
```

### Function Contracts

```hilow
// Preconditions with "requires"
// Postconditions with "ensures"
function divide(a: i32, b: i32): i32
  requires (b != 0)
  ensures (result * b <= a and result * b + b > a)
{
  return a / b;
}

// Prover checks call sites
let x = divide(10, 5);   // ✓ Proven safe
let y = divide(10, 0);   // ✗ Proof error: precondition violated

// Prover understands control flow
let divisor = getUserInput();
if (divisor != 0) {
  let z = divide(100, divisor);  // ✓ Prover knows divisor != 0
}
```

### Unknown Verification

```hilow
function getUser(id: i32): object | unknown {
  // Implementation
}

// Prover ensures unknown is checked
let user = getUser(123);
print(user.name);  // ✗ Proof error: unknown not handled

// Correct
let user = getUser(123);
if (user ??= unknown) {
  print(f"Error: {user.reason}");
  return;
}
print(user.name);  // ✓ Proven safe
```

### Memory Safety

```hilow
heap let buffer = alloc(1024);

function process() {
  // Use buffer
  buffer = nothing;
}

process();
let x = buffer[0];  // ✗ Proof error: buffer is nothing
```

### Watch Dependencies

```hilow
let a = 0;
let b = 0;

watch w1(a) {
  b = a + 1;  // Modifies b
}

watch w2(b) {
  a = b + 1;  // Modifies a
}

// ✗ Proof error: circular watch dependency
// Infinite loop detected at compile time
```

### Array Bounds

```hilow
let items: [i32; 10];

function getItem(index: i32): i32
  requires (index >= 0 and index < 10)
{
  return items[index];
}

// Prover checks
let x = getItem(5);    // ✓ Literal within bounds
let y = getItem(15);   // ✗ Proof error: out of bounds

// With runtime check
let index = getUserInput();
if (index >= 0 and index < 10) {
  let z = getItem(index);  // ✓ Bounds satisfied
}
```

### Money Type Safety

```hilow
function calculateTotal(price: money<USD>, tax: f64): money<USD>
  requires (tax >= 0.0 and tax <= 1.0)
  ensures (result >= price)
{
  return price * (1.0 + tax);
}

let item = 100.00 USD;
let total = calculateTotal(item, 0.08);  // ✓ Proven safe

let euro = 50.00 EUR;
let bad = calculateTotal(euro, 0.08);  // ✗ Proof error: currency mismatch
```

### Time Constraints

```hilow
function scheduleMeeting(when: time): bool
  requires (when > time.now())
{
  if (when < time.now() + 1h) {
    return false;
  }
  return true;
}

let tomorrow = time.now() + 1d;
scheduleMeeting(tomorrow);  // ✓ Proven safe

let yesterday = time.now() - 1d;
scheduleMeeting(yesterday);  // ✗ Proof error: precondition violated
```

### Proof Modes

```bash
# Normal compilation (no proof checking)
hilowc program.hl -o program

# Proof pass (verify all constraints)
hilowc program.hl --prove

# Output:
# ✓ All variable constraints verified
# ✓ All function contracts satisfied
# ✓ All unknown returns handled
# ✓ No circular watch dependencies
# ✓ All memory deallocations verified
# ✓ Array bounds checked
# ✓ Currency types verified
# ⚠ Warning: line 45 - coercion may lose precision

# Proof with optimization hints
hilowc program.hl --prove --suggest

# Output includes:
# 💡 Suggestion: line 23 - use i32 instead of f64
# 💡 Suggestion: line 67 - constraint always true
```

### Gradual Verification

```hilow
// Start without proofs
function divide(a, b) {
  return a / b;
}

// Add basic proof
function divide(a: i32, b: i32): i32
  requires (b != 0)
{
  return a / b;
}

// Add complete proof
function divide(a: i32, b: i32): i32
  requires (b != 0)
  ensures (result * b <= a and result * b + b > a)
{
  return a / b;
}
```

## Module System

### Exports

```hilow
// math.hl
export function add(a: i32, b: i32): i32 {
  return a + b;
}

export function subtract(a: i32, b: i32): i32 {
  return a - b;
}

export let PI: f64 = 3.14159;

// Private (not exported)
function helper() {
  // Internal use only
}
```

### Imports

```hilow
// main.hl
import { add, subtract, PI } from "./math";

let sum = add(5, 3);
let diff = subtract(10, 4);
print(f"PI = {PI}");
```

### Module Rules

- Only named exports (no default exports)
- No namespace imports (no `import * as`)
- No dynamic imports
- Simple and explicit

## Destructuring

### Array Destructuring

```hilow
let array = [1, 2, 3, 4, 5];
let [first, second] = array;

// With rest
let [head, ...tail] = array;

// Swapping
let a = 1;
let b = 2;
[a, b] = [b, a];
```

### Object Destructuring

```hilow
let point = { x: 10, y: 20 };
let { x, y } = point;

// With different names
let { x: posX, y: posY } = point;

// With defaults
let { x, y, z = 0 } = point;
```

### Function Parameters

```hilow
function distance({ x, y }: object): f64 {
  return sqrt(x * x + y * y);
}

distance({ x: 3, y: 4 });  // 5.0
```

### No Complex Patterns

```hilow
// Removed (too complex):
let { x: newX, ...rest } = obj;    // ✗ Rest in objects
let [first, , third] = array;      // ✗ Skipping elements
let { a: { b: { c } } } = obj;     // ✗ Deep nesting
```

## Inline Assembly

```hilow
// Platform-specific assembly
function getTimestamp(): u64 {
  let result: u64;
  
  asm {
    rdtsc
    shl rdx, 32
    or rax, rdx
    mov [result], rax
  }
  
  return result;
}

// Assembly with constraints
function atomicIncrement(ptr: *u64): u64 {
  let result: u64;
  
  asm {
    mov rax, 1
    lock xadd [ptr], rax
    mov [result], rax
  }
  
  return result;
}
```

## Standard Library

### I/O

```hilow
// Console output
print("Hello");
print(f"Value: {x}");

// File operations (return unknown on error)
let file = openFile("data.txt");
if (file ??= unknown) {
  print(f"Error: {file.reason}");
  return;
}

let content = file.read();
if (content ??= unknown) {
  print(f"Read error: {content.reason}");
  file.close();
  return;
}

file.close();
```

### HTTP

```hilow
// HTTP requests (return unknown on error)
let response = http.get("https://api.example.com/data");

if (response ??= unknown) {
  print(f"Request failed: {response.reason}");
  return;
}

if (response.status != 200) {
  print(f"HTTP {response.status}");
  return;
}

let data = response.body;
```

### Math

```hilow
// Basic math functions
let x = abs(-5);        // 5
let y = sqrt(16);       // 4.0
let z = pow(2, 8);      // 256
let a = sin(PI / 2);    // 1.0
let b = cos(0);         // 1.0
let c = floor(3.7);     // 3
let d = ceil(3.2);      // 4
let e = round(3.5);     // 4
```

### String Operations

```hilow
let s = "hello world";

// Length
s.length;               // 11

// Index/search
s.indexOf("world");     // 6
s.indexOf("xyz");       // -1

// Substring
s.slice(0, 5);         // "hello"
s.slice(6);            // "world"

// Split/join
let parts = s.split(" ");        // ["hello", "world"]
let joined = parts.join("-");    // "hello-world"

// Replace
s.replace("world", "there");     // "hello there"

// Case
s.toUpperCase();       // "HELLO WORLD"
s.toLowerCase();       // "hello world"

// Trim
"  text  ".trim();     // "text"
```

## Compilation

### Compiler Invocation

```bash
# Compile to executable
hilowc program.hl -o program

# Compile with optimizations
hilowc program.hl -O2 -o program

# Compile with proof checking
hilowc program.hl --prove -o program

# Proof only (no compilation)
hilowc program.hl --prove

# Generate LLVM IR
hilowc program.hl --emit-llvm -o program.ll

# Cross-compilation
hilowc program.hl --target=arm64-linux -o program
```

### Compilation Targets

- Native executables (no runtime required)
- LLVM IR for optimization
- WebAssembly for browsers
- Static libraries
- Dynamic libraries

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

### Hello World

```hilow
function main(args: [string]): i32 {
  print("Hello, HiLow!");
  return 0;
}
```

### HTTP Server

```hilow
function handleRequest(req: object): object {
  return {
    status: 200,
    body: f"Hello from HiLow at {time.now()}"
  };
}

function main(): i32 {
  let server = http.listen("0.0.0.0:8080");
  
  if (server ??= unknown) {
    print(f"Failed to start: {server.reason}");
    return 1;
  }
  
  print("Server listening on :8080");
  
  let connections: [object];
  
  watch w(server.connections) {
    if (server.connections.length > 0) {
      let conn = server.connections.pop();
      connections.push(conn);
      
      watch cw(conn.requests) {
        if (conn.requests.length > 0) {
          let req = conn.requests.pop();
          let resp = handleRequest(req);
          conn.send(resp);
        }
      };
    }
  };
  
  // Keep running
  for (;;) {
    // Event loop handled by watches
  }
  
  return 0;
}
```

### Banking System

```hilow
let balance: money<USD> (balance >= 0.00 USD) = 1000.00 USD;

function withdraw(amount: money<USD>): bool
  requires (amount >= 0.00 USD)
  ensures (result ?= true implies balance >= 0.00 USD)
{
  if (amount > balance) {
    return false;
  }
  
  balance = balance - amount;
  return true;
}

function deposit(amount: money<USD>): bool
  requires (amount >= 0.00 USD)
  ensures (balance >= 0.00 USD)
{
  balance = balance + amount;
  return true;
}

function transfer(to: Account, amount: money<USD>): object | unknown
  requires (amount >= 0.00 USD)
{
  if (not withdraw(amount)) {
    return unknown("insufficient funds", options: ["deposit more"]);
  }
  
  let result = to.deposit(amount);
  
  if (result ??= unknown) {
    // Rollback
    balance = balance + amount;
    return result;
  }
  
  return { success: true };
}

function main(): i32 {
  print(f"Initial balance: {balance}");
  
  if (withdraw(200.00 USD)) {
    print(f"After withdrawal: {balance}");
  }
  
  deposit(500.00 USD);
  print(f"After deposit: {balance}");
  
  return 0;
}
```

### Concurrent Counter

```hilow
shared let counter: i32 (counter >= 0) = 0;

function main(): i32 {
  // Start multiple processes
  for (let i = 0; i < 10; i += 1) {
    async {
      for (let j = 0; j < 100; j += 1) {
        counter += 1;
      }
    };
  }
  
  // Watch for completion
  let w = watch(counter) {
    print(f"Counter: {counter}");
    
    if (counter >= 1000) {
      print("All processes complete!");
      w.end();
    }
  };
  
  // Keep running until done
  while (w.isActive()) {
    // Wait
  }
  
  return 0;
}
```

## Language Summary

HiLow provides:

**Core Features:**
- Compiled to native code (no runtime)
- Static typing with inference
- Type coercion where safe
- Prototype-based objects (no classes)
- First-class functions and closures
- Explicit memory management
- Optional formal verification

**Special Types:**
- `nothing` - absence/deallocation
- `unknown` - rich error information
- `time` - first-class time and duration
- `money` - currency-safe financial operations

**Unique Features:**
- Quote recursion for strings (no escaping quotes)
- F-strings without backticks (Python-style)
- `watch()` for reactive programming
- Equality operators: `?=`, `??=`, `!=`, `!!=`
- Logical operators: `and`, `or`, `not`
- Constraint-based verification
- Function contracts (requires/ensures)

**Design Goals:**
- Small language surface area
- One obvious way to do things
- Pragmatic correctness
- Low-level control
- High-level ergonomics

HiLow bridges the gap between systems programming and application development, providing a single language for writing everything from device drivers to web applications.
