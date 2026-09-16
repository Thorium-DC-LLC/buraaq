# Buraaq Language Specification (v0.1 Draft)

This document is the authoritative reference for Buraaq syntax and semantics. Implementation status is tracked in [ROADMAP.md](./ROADMAP.md).

---

## 1. Lexical Structure

### 1.1 Source files

- Extension: `.bq`
- Encoding: UTF-8
- Line ending: LF (formatter normalizes CRLF)
- Indentation: 4 spaces per level; tab characters are rejected

### 1.2 Comments

```buraaq
# Line comment to end of line

"""
Block comment
 spanning lines
"""
```

### 1.3 Identifiers

`[A-Za-z_][A-Za-z0-9_]*` — case-sensitive.

### 1.4 Keywords

```
fn async let if else elif while for parallel match return break continue
struct enum trait impl module use pub unsafe extern c throws ref mut copy
new give true false none some ok err spawn await defer assert panic
as is where dyn
```

### 1.5 Literals

| Kind | Examples |
|------|----------|
| Integer | `42`, `0xFF`, `0b1010`, `1_000_000` (default type `i32`) |
| Float | `3.14`, `1e-9` (default type `f64`) |
| Boolean | `true`, `false` |
| Character | `'a'`, `'\n'` |
| Text | `"hello"`, `"line\n"`, `""` |
| Bytes | `b"raw\x00bytes"` |

### 1.6 Operators (precedence high → low)

```
() [] . ? !
-unary not
* / %
+ -
<< >>
< <= > >=
== !=
&
^
|
&&
||
??        # default value if Option/Result branch
=
```

Postfix `?` propagates errors. Postfix `!` unwraps `Option`/`Result` in debug contexts (panics on failure).

---

## 2. Program Structure

```buraaq
module myapp.main    # optional; defaults from file path

use std.io.{print, println}
```

Every package has exactly one entry module (`buraaq.pkg` → `entry = "main"`).

---

## 3. Declarations

### 3.1 Variables

```buraaq
let x = 10              # immutable binding; type inferred i32
let name: text = "Buraaq"
let mut counter = 0     # mutable binding; rebinding still forbidden
counter = counter + 1
```

`let` introduces a binding. Immutability means the **binding** cannot be reassigned; interior mutability requires `mut` binding or `Mutex`.

### 3.2 Functions

```buraaq
fn add(a: i32, b: i32) -> i32:
    return a + b

fn greet(name: text) -> text:
    return "Hello, " + name
```

Trailing return type optional when inferrable from single `return` body expression:

```buraaq
fn double(n: i32) -> i32:
    n * 2
```

### 3.3 Structs

```buraaq
struct Point:
    x: f64
    y: f64

    fn distance(self) -> f64:
        return math.sqrt(self.x * self.x + self.y * self.y)
```

Field access: `p.x`. Methods take explicit `self` parameter.

### 3.4 Enums

```buraaq
enum Color:
    Red
    Green
    Blue
    Rgb(i32, i32, i32)

enum Result[T, E]:
    Ok(T)
    Err(E)
```

Pattern matching via `match`:

```buraaq
match color:
    Color.Red:
        print("red")
    Color.Rgb(r, g, b):
        print("rgb", r, g, b)
    _:
        print("other")
```

### 3.5 Traits and Impl

```buraaq
trait Printable:
    fn to_text(self) -> text

impl Printable for Point:
    fn to_text(self) -> text:
        return "Point(" + self.x.to_text() + ", " + self.y.to_text() + ")"
```

---

## 4. Control Flow

### 4.1 Conditionals

```buraaq
if x > 0:
    print("positive")
elif x < 0:
    print("negative")
else:
    print("zero")
```

### 4.2 Loops

```buraaq
while n > 0:
    n = n - 1

for i in 0..10:
    print(i)

for item in collection:
    process(item)
```

Range `a..b` is half-open `[a, b)`.

### 4.3 Loop control

```buraaq
break
continue
break 42    # labeled break value in for loops returning Option
```

---

## 5. Types (Surface)

| Type | Description |
|------|-------------|
| `i8`…`i128`, `u8`…`u128` | Fixed-width integers |
| `f32`, `f64` | IEEE floats |
| `bool`, `char` | Boolean, Unicode scalar |
| `text` | UTF-8 string (owned, immutable) |
| `bytes` | Byte sequence |
| `T[]` | Slice/view (borrowed sequence) |
| `List[T]`, `Map[K,V]` | Standard collections |
| `Option[T]`, `Result[T,E]` | Standard enums |
| `fn(A, B) -> C` | Function pointer |
| `*T`, `*mut T` | Raw pointers (`unsafe` only) |

See [TYPE_SYSTEM.md](./TYPE_SYSTEM.md) for full rules.

---

## 6. Memory Operations (Surface)

```buraaq
let p = Point { x: 1.0, y: 2.0 }     # stack struct
let list = new List[i32]()           # heap allocation
give list_to_fn(list)                # explicit move when compiler asks
ref r = list                         # shared borrow
ref mut w = list                     # exclusive borrow
```

See [MEMORY_MODEL.md](./MEMORY_MODEL.md).

---

## 7. Error Handling

```buraaq
fn read_u32(path: text) throws IOError -> u32:
    data = std.io.read_all(path)?
    return parse_u32(data)?
```

See [ERROR_MODEL.md](./ERROR_MODEL.md).

---

## 8. Modules

See [MODULE_SYSTEM.md](./MODULE_SYSTEM.md).

---

## 9. Unsafe

```buraaq
unsafe:
    ptr = c.malloc(64)
    # ...
    c.free(ptr)
```

Safe code cannot dereference raw pointers or call `unsafe fn`.

---

## 10. Attributes

```buraaq
#[inline]
#[cold]
#[export(c)]
#[test]
```

---

## 11. Complete Examples

### 11.1 Hello World

```buraaq
# examples/hello.bq
use std.io.println

fn main():
    println("Hello, world!")
```

### 11.2 Variables

```buraaq
fn main():
    # integers and floats
    age = 25
    pi = 3.14159

    # text
    name = "Buraaq"

    # explicit types
    count: u64 = 1_000_000

    # mutability
    let mut score = 0
    score = score + 10

    # constants (compile-time evaluated)
    const MAX: i32 = 1024

    println(name + " v" + age.to_text())
```

### 11.3 Functions

```buraaq
fn factorial(n: u64) -> u64:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

fn clamp(value: i32, min: i32, max: i32) -> i32:
    if value < min:
        return min
    if value > max:
        return max
    return value

fn main():
    println(factorial(10).to_text())
```

### 11.4 Loops

```buraaq
fn sum_to_n(n: i32) -> i32:
    let mut total = 0
    for i in 1..=n:
        total = total + i
    return total

fn fizzbuzz():
    for i in 1..=100:
        if i % 15 == 0:
            println("FizzBuzz")
        elif i % 3 == 0:
            println("Fizz")
        elif i % 5 == 0:
            println("Buzz")
        else:
            println(i.to_text())

fn main():
    println(sum_to_n(100).to_text())
    fizzbuzz()
```

### 11.5 Collections

```buraaq
use std.collections.{List, Map}

fn main():
    nums = List[i32].from([3, 1, 4, 1, 5])
    nums.push(9)
    nums.sort()

    for n in nums:
        print(n.to_text() + " ")

    scores = Map[text, i32].new()
    scores["alice"] = 10
    scores["bob"] = 8

    match scores.get("alice"):
        Some(v):
            println("alice: " + v.to_text())
        None:
            println("not found")
```

### 11.6 Structs

```buraaq
struct User:
    id: u64
    name: text
    active: bool

    fn display(self) -> text:
        status = if self.active: "active" else: "inactive"
        return self.name + " (" + status + ")"

fn main():
    u = User { id: 1, name: "Ada", active: true }
    println(u.display())
```

### 11.7 Enums

```buraaq
enum Shape:
    Circle(f64)
    Rectangle(f64, f64)

fn area(shape: Shape) -> f64:
    match shape:
        Shape.Circle(r):
            return 3.14159 * r * r
        Shape.Rectangle(w, h):
            return w * h

fn main():
    shapes = [Shape.Circle(2.0), Shape.Rectangle(3.0, 4.0)]
    for s in shapes:
        println(area(s).to_text())
```

### 11.8 Errors

```buraaq
enum ParseError:
    InvalidChar(char)
    EmptyInput

fn parse_digit(c: char) -> Result[i32, ParseError]:
    if c >= '0' and c <= '9':
        return Ok(c as i32 - '0' as i32)
    return Err(ParseError.InvalidChar(c))

fn parse_number(input: text) throws ParseError -> i32:
    if input.is_empty():
        raise ParseError.EmptyInput
    let mut value = 0
    for c in input.chars():
        d = parse_digit(c)?
        value = value * 10 + d
    return value

fn main():
    match parse_number("123"):
        Ok(n):
            println(n.to_text())
        Err(e):
            println("failed: " + e.to_text())
```

Note: `raise E` inside `throws E` function is sugar for `return Err(E)`.

### 11.9 Files

```buraaq
use std.io.{File, read_line}
use std.fs.path

fn copy_file(src: text, dst: text) throws IOError:
    content = File.read_all(src)?
    File.write_all(dst, content)?

fn main():
    path = "notes.txt"
    File.write_all(path, b"Buraaq files are easy.\n")?

    file = File.open(path)?
    while true:
        line = read_line(file)?
        match line:
            None:
                break
            Some(text):
                print(text)
    # file closed automatically at scope end
```

### 11.10 Networking

```buraaq
use std.net.{TcpStream, SocketAddr}
use std.io

fn fetch_once(host: text, port: u16) throws NetError -> text:
    addr = SocketAddr.parse(host, port)?
    stream = TcpStream.connect(addr)?
    stream.write_all(b"GET / HTTP/1.1\r\nHost: " + host.to_bytes() + b"\r\n\r\n")?
    return stream.read_all_text()?

fn main():
    body = fetch_once("example.com", 80)?
    println(body.lines().first() ?? "")
```

### 11.11 Threading

```buraaq
use std.thread.{spawn, sleep}
use std.sync.Mutex
use std.time.Duration

fn main():
    counter = Mutex.new(0)

    handles = []
    for _ in 0..8:
        h = spawn:
            for _ in 0..1000:
                guard = counter.lock()
                guard[] = guard[] + 1
        handles.push(h)

    for h in handles:
        h.join()

    println(counter.lock()[].to_text())
```

### 11.12 Async Operations

```buraaq
use std.async.{await, spawn_task}
use std.net.TcpStream
use std.io

async fn fetch_async(host: text, port: u16) throws NetError -> text:
    stream = await TcpStream.connect_async(host, port)?
    await stream.write_all_async(b"GET / HTTP/1.1\r\n\r\n")?
    return await stream.read_all_text_async()?

fn main():
    # bridge: sync main can spawn async work
    task = spawn_task async:
        await fetch_async("example.com", 80)?

    result = await task
    println(result.lines().first() ?? "")
```

### 11.13 Pointers

```buraaq
# Safe code uses references; raw pointers require unsafe.

fn increment_safe(value: ref mut i32):
    value[] = value[] + 1

fn main():
    let mut n = 41
    increment_safe(ref mut n)
    println(n.to_text())   # 42

unsafe fn increment_raw(ptr: *mut i32):
    if ptr.is_null():
        return
    *ptr = *ptr + 1

unsafe fn demo_raw():
    let mut x = 10
    increment_raw(&mut x as *mut i32)
```

### 11.14 FFI (C Interoperability)

```buraaq
extern c:
    fn puts(s: c.text) -> c.int
    fn strlen(s: c.text) -> c.usize

use libc.{puts, strlen}

fn main():
    msg = "Hello from Buraaq via C"
    unsafe:
        puts(msg.to_c())
        len = strlen(msg.to_c())
    println(len.to_text())
```

### 11.15 Tiny HTTP Server

```buraaq
# examples/tiny_http.bq — threaded blocking server (~40 lines)
use std.io.{println, write_all}
use std.net.TcpListener
use std.thread.spawn

const RESPONSE = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nHello from Buraaq!\n"

fn handle(mut conn):
    _ = conn.read_until(b"\r\n\r\n")   # discard request headers
    write_all(conn, RESPONSE)?
    conn.shutdown()

fn main() throws IOError:
    listener = TcpListener.bind("127.0.0.1:8080")?
    println("Buraaq HTTP server listening on http://127.0.0.1:8080")

    while true:
        conn = listener.accept()?
        spawn handle(conn)
```

---

## 12. Semantic Summary

| Topic | Rule |
|-------|------|
| Assignment | Move for non-`copy` types; copy for `copy` types |
| Equality | Structural for primitives; defined per type otherwise |
| Numeric promotion | `i32` + `i64` → `i64`; mixed int/float → float |
| Scope | Lexical; indentation defines blocks |
| Drop | End of scope, reverse order |
| Main | `fn main()` or `fn main() throws E` entry |

---

## 13. Reserved for Future Versions

- Hygienic macros `macro name(...): ...`
- `const fn` compile-time evaluation (partial support v0.5)
- SIMD vector types `vec4[f32]`
- Embedded `#![no_std]` profile

Syntax in this document is stable for v0.1 implementation. Breaking changes require ADR and minor version bump.
