# Buraaq Syntax Reference

Authoritative surface syntax for Buraaq v0.2. Formal grammar: [`grammar/buraaq.ebnf`](../grammar/buraaq.ebnf).  
Semantics: [`SEMANTICS.md`](./SEMANTICS.md). Architecture context: [`LANGUAGE_PHILOSOPHY.md`](./LANGUAGE_PHILOSOPHY.md).

Buraaq reads like pseudocode. There is **one way** to do each basic thing.

---

## 1. Design Rules

| Rule | Example |
|------|---------|
| One binding syntax | `name = expr` or `mut name = expr` |
| One block syntax | `{ ... }` |
| One import syntax | `use path.item` |
| One error propagate | `expr?` |
| One optional type | `Option[T]` / `none` / `some(x)` |
| One unsafe gate | `unsafe { ... }` |
| Inference first | Types on bindings optional unless ambiguous |
| No headers | File path = module path |
| No preprocessor | `const` and `cfg` attributes replace macros |

---

## 2. Lexical Grammar

### 2.1 Files

- Extension: `.bq`
- Encoding: UTF-8
- Newline: LF (formatter normalizes CRLF)
- Indentation: 4 spaces inside blocks; tabs forbidden

### 2.2 Comments

```buraaq
# line comment

""" block comment """
```

### 2.3 Identifiers

```
[A-Za-z_][A-Za-z0-9_]*
```

Case-sensitive. Keywords reserved (see §10).

### 2.4 Literals

#### Integers

Default type: `int` (32-bit signed; see §3.1).

```
42  1_000  0xFF  0b1010
```

#### Floats

Default type: `float` (64-bit IEEE).

```
3.14  1.0  6.022e23
```

#### Booleans

```
true  false
```

#### Characters

```
'a'  '\n'  '\x41'
```

#### Strings

```
"hello"
"multi\nline"
```

#### String interpolation

Only `$` + `{expr}` form (no `%s` printf style):

```buraaq
name = "Asim"
age = 30
msg = "Hello, {name}! You are {age} years old."
```

Expression in `{ }` must implement `ToText` or be a primitive the compiler knows how to format.

#### Bytes

```
b"raw\x00bytes"
```

#### Option / Result literals

```buraaq
none
some(42)
ok(value)
err(IOError.NotFound("config.toml"))
```

---

## 3. Types

### 3.1 Built-in aliases (preferred in application code)

| Alias | Fixed type | Notes |
|-------|------------|-------|
| `int` | `i32` | Default for integer literals |
| `uint` | `u32` | |
| `float` | `f64` | Default for float literals |
| `bool` | `bool` | |
| `text` | owned UTF-8 string | Never null |
| `byte` | `u8` | |
| `bytes` | owned byte buffer | |
| `void` | unit `()` | Function with no return value |

### 3.2 Fixed-width (systems / FFI)

```
i8 i16 i32 i64 i128
u8 u16 u32 u64 u128
f32 f64
isize usize
char
```

### 3.3 Composite

```buraaq
(T1, T2)           # tuple
[T; N]             # fixed array (N compile-time constant)
T[]                # slice (borrowed view)
List[T] Map[K,V]   # std collections
Option[T] Result[T,E]
fn(A, B) -> C      # function pointer type
```

Generics use square brackets: `Box[T]`, not angle brackets.

### 3.4 Type annotations

Only where needed—parameters, public APIs, disambiguation:

```buraaq
count: int = 0
fn parse(input: text) -> Result[int, ParseError]
```

---

## 4. Variables and Constants

### 4.1 Immutable binding (default)

```buraaq
name = "Asim"
pi = 3.14159
```

Binding cannot be reassigned. Mutating *through* a binding requires `mut` or interior mutability (`Mutex`, etc.).

### 4.2 Mutable binding

```buraaq
mut score = 0
score = score + 10
```

### 4.3 Constants

Compile-time evaluated; must be inferable or annotated:

```buraaq
const MAX_CONNECTIONS = 1024
const APP_NAME: text = "Buraaq"
```

### 4.4 No `let` keyword

Buraaq uses assignment-form bindings only. There is no separate `let`/`var` split.

---

## 5. Functions

### 5.1 Definition

```buraaq
fn add(a: int, b: int) -> int {
    a + b
}
```

- Parameters require types (anchor for inference inside body).
- Return type optional if body is a single expression block with implicit return.
- Explicit return: `return expr`

### 5.2 Fallible functions

**One** error declaration style—`throws`:

```buraaq
fn read_config(path: text) throws IOError -> Config {
    data = read_file(path)?
    parse_config(data)
}
```

Desugars to `-> Result[Config, IOError]`. Use `raise expr` for early error return inside `throws` functions.

Alternative (equivalent, more verbose):

```buraaq
fn read_config(path: text) -> Result[Config, IOError] { ... }
```

Do not mix styles in the same codebase; `throws` is preferred for application code.

### 5.3 Methods

Defined inside `struct` or via `impl`:

```buraaq
struct Point {
    x: float
    y: float

    fn distance(self) -> float {
        math.sqrt(self.x * self.x + self.y * self.y)
    }
}
```

- `self` — by value (move unless `copy` type)
- `mut self` — exclusive mutable access
- `ref self` — shared borrow (when compiler requires explicit borrow)

### 5.4 Generics

```buraaq
fn first[T](items: List[T]) -> Option[T] {
    if items.is_empty() {
        return none
    }
    some(items[0])
}
```

Constraints when needed:

```buraaq
fn max[T: Comparable](a: T, b: T) -> T {
    if a > b { a } else { b }
}
```

---

## 6. Blocks and Statements

### 6.1 Blocks

```buraaq
{
    x = 1
    y = 2
    x + y   # value of block if used as expression
}
```

### 6.2 Semicolons

Optional. Newline ends statement when unambiguous. Use `;` only for multiple statements on one line.

### 6.3 Implicit return

Last expression in a function block without trailing `;` is returned:

```buraaq
fn double(n: int) -> int {
    n * 2
}
```

---

## 7. Expressions and Operators

### 7.1 Precedence (high → low)

| Level | Operators |
|-------|-----------|
| Postfix | `()` call, `[]` index, `.` field/method, `?` `!` |
| Unary | `-` `!` `not` `ref` `ref mut` `give` `copy` `await` |
| Multiplicative | `*` `/` `%` |
| Additive | `+` `-` |
| Shift | `<<` `>>` |
| Bitwise AND | `&` |
| Bitwise XOR | `^` |
| Bitwise OR | `\|` |
| Comparison | `<` `<=` `>` `>=` |
| Equality | `==` `!=` |
| Logical AND | `&&` |
| Logical OR | `\|\|` |
| Assignment | `=` |

### 7.2 Arithmetic

```buraaq
a + b - c * d / e % f
```

Integer `/` truncates toward zero. Use `float(a) / float(b)` for floating division.

### 7.3 Comparison and logic

```buraaq
if age >= 18 && active {
    print("Adult")
}
```

No implicit truthiness: only `bool` allowed in conditions.

### 7.4 Error propagation

```buraaq
data = read_file(path)?
```

Postfix `?` only—no `try/catch`, no exceptions.

### 7.5 Force unwrap (discouraged)

```buraaq
value = opt!    # panics if none/err — lint warns outside tests
```

### 7.6 Null-coalescing

```buraaq
name = opt ?? "default"
```

---

## 8. Control Flow

### 8.1 If

```buraaq
if score >= 60 {
    print("Pass")
} elif score >= 50 {
    print("Marginal")
} else {
    print("Fail")
}
```

If expression:

```buraaq
label = if ok { "yes" } else { "no" }
```

### 8.2 While

```buraaq
mut n = 10
while n > 0 {
    n = n - 1
}
```

### 8.3 For

Iterator (preferred):

```buraaq
for user in users {
    print(user.name)
}
```

Range:

```buraaq
for i in 0..10 { }      # 0..9 half-open
for i in 1..=10 { }     # 1..10 inclusive
```

Structured parallelism:

```buraaq
parallel for item in items {
    process(item)
}
```

### 8.4 Match

Justified for enums and tagged unions—one pattern form:

```buraaq
match result {
    Ok(v) => print(v),
    Err(e) => print(e.message()),
}
```

Block body per arm:

```buraaq
match shape {
    .Circle(r) => {
        pi * r * r
    },
    .Rectangle(w, h) => w * h,
    _ => 0.0,
}
```

Shorthand: leading `.Variant` when scrutinee enum type is known.

### 8.5 Break / Continue

```buraaq
for i in 0..100 {
    if i == 50 { break }
    if i % 2 == 0 { continue }
}
```

---

## 9. Structs

```buraaq
struct User {
    id: uint
    name: text
    active: bool
}
```

Literal:

```buraaq
u = User { id: 1, name: "Ada", active: true }
```

Field access: `u.name`

No inheritance. Composition only.

---

## 10. Enums (Tagged Unions)

```buraaq
enum Color {
    Red,
    Green,
    Blue,
}

enum Shape {
    Circle(float),              # tuple variant
    Rectangle { w: float, h: float },  # struct variant
}
```

Construct: `Shape.Circle(2.0)` or `.Circle(2.0)` when type inferred.

---

## 11. Traits (Protocols)

```buraaq
trait Printable {
    fn to_text(self) -> text
}

impl Printable for User {
    fn to_text(self) -> text {
        self.name
    }
}
```

Dynamic dispatch (rare): `dyn Printable` — opt-in only.

---

## 12. Modules and Imports

File `src/http/server.bq` → module `http.server`.

```buraaq
module http.server   # optional if matches path

use std.io.println
use std.collections.{List, Map}
use std.net as net
```

Visibility: `pub` prefix exports item.

```buraaq
pub fn listen(addr: text) -> Server { ... }
```

No include guards. No header files. No `mod` declarations.

---

## 13. Memory and Pointers

### 13.1 Stack (default)

```buraaq
p = Point { x: 1.0, y: 2.0 }
```

### 13.2 Heap

```buraaq
list = new List[int]()
```

Compiler inserts drop at scope end (see MEMORY_MODEL.md).

### 13.3 Explicit transfer

```buraaq
give list_to_fn(list)   # move ownership — only when diagnostic suggests
```

### 13.4 Borrows (when needed)

```buraaq
ref r = value           # shared
ref mut w = value       # exclusive
```

Most code never writes `ref`; compiler inserts implicitly.

### 13.5 Defer

```buraaq
f = File.open("x")?
defer f.close()
```

Runs at scope exit, reverse order.

---

## 14. Unsafe and FFI

### 14.1 Unsafe block

```buraaq
unsafe {
    ptr = c.malloc(64)
    # ...
    c.free(ptr)
}
```

Raw pointers `*T`, `*mut T` only in `unsafe`.

### 14.2 Extern C

```buraaq
extern c {
    fn puts(s: c.text) -> c.int
}
```

Buraaq `text` ≠ `c.text`; convert with `text.to_c()` for call duration.

---

## 15. Concurrency

### 15.1 Threads

```buraaq
handle = spawn {
    work()
}
handle.join()
```

### 15.2 Async (opt-in)

```buraaq
task = async {
    await stream.read(1024)?
}

result = await task
```

Sync functions stay sync unless marked `async fn` or inside `async { }`.

---

## 16. Attributes

```buraaq
#[test]
#[inline]
#[export(c)]
#[cfg(linux)]
```

---

## 17. Reserved Keywords

```
async await break const continue copy defer else enum err extern false fn
for give if impl in match module mut new none not ok parallel pub ref
return self some spawn struct trait true type unsafe use void while throws
raise parallel defer dyn where
```

---

## 18. Pain Points Removed

| C++ / Rust pain | Buraaq approach |
|-----------------|-----------------|
| Header files | File = module |
| Lifetime annotations | Compiler GFA (inferred) |
| Template error spew | Truncated chains; constraints on definition |
| Implicit conversions | Narrowing rejected; widening explicit or contextual |
| Preprocessor | `const`, `cfg` attributes |
| Null pointers | `Option[T]` only |
| Iterator invalidation | GFA rejects invalid borrows across mutating calls |
| Include guards | N/A |
| Cryptic link errors | Driver invokes linker with full symbol context |
| Build configuration | `buraaq.pkg` + `buraaq build` |
| Redundant types | Inference at bindings |
| Nested error handling | `?` propagation |

---

## 19. Example Tour

See [`examples/language-tour/`](../examples/language-tour/) for 60 small programs, one feature each.

Formal grammar: [`grammar/buraaq.ebnf`](../grammar/buraaq.ebnf).
