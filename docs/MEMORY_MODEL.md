# Buraaq Memory Model

Buraaq provides **memory safety by default**, **deterministic destruction**, and **no garbage collector** for normal systems programming. Safety is enforced by **Guarded Flow Analysis (GFA)** at compile time; the programmer never writes lifetime annotations.

---

## 1. Principles

1. **Single owner** for non-`copy` values at any point in time (unless borrowed).
2. **Borrowing** temporarily splits access into shared (`ref`) or exclusive (`ref mut`) views.
3. **Destruction** occurs at scope exit, on move-out, or on panic—never nondeterministically.
4. **Heap allocation** is explicit via `new`; deallocation is compiler-inserted unless ownership transferred.
5. **Unsafe** is the only gate for raw pointers, unchecked memory, and C type punning.

---

## 2. Storage Categories

| Category | Introduced by | Lifetime | Deallocation |
|----------|---------------|----------|--------------|
| **Stack slot** | `let x = expr` (non-escaping) | Enclosing scope | Automatic drop at scope end |
| **Heap object** | `new T(...)` | Until drop/transfer | `T.drop` + allocator free |
| **Static/global** | `static`, `const` data | Program lifetime | Never |
| **Arena chunk** | `arena.alloc(T)` | Arena reset/drop | Bulk free arena |
| **Foreign (C)** | `extern c` alloc | Programmer contract in `unsafe` | Manual or wrapper RAII |

---

## 3. Ownership Rules

### 3.1 Move-by-default

For types not marked `copy`:

```buraaq
let a = new List[i32]()
let b = a        # ERROR: use of moved value `a`
let b = give a   # OK: explicit move when required by clarity pass
```

Function arguments and return values use move semantics. The compiler elides copies when source is unused (NRVO/move elision).

### 3.2 Copy types

Types implementing `copy` (primitives, `bool`, `char`, tuples of copy types, user types with `impl copy`) duplicate bitwise on assignment.

User-defined `copy` requires:

- All fields are `copy`
- No `drop fn` defined
- Compiler verifies no interior pointers to non-static data

### 3.3 Ownership transfer

| Operation | Effect |
|-----------|--------|
| `give x` to function | Caller loses access to `x` |
| `return x` | Ownership moves to caller |
| `field = give x` | Ownership moves into struct field |
| Pattern bind `let y = give x` | Rebinds ownership |

---

## 4. Borrowing

### 4.1 When borrows appear

Most code never writes `ref`. The compiler inserts borrows internally for method calls like `list.push(1)`.

Programmers write `ref` / `ref mut` when:

- GFA cannot disambiguate reborrowing
- Low-level APIs require explicit borrowed views
- Compiler diagnostic suggests it

### 4.2 Rules (GFA)

At any program point:

- Either **one** active `ref mut T` to a value,
- Or **any number** of active `ref T` borrows,
- Never both simultaneously for the same memory location.

Borrow scope = lexical region computed by MIR dataflow (may be shorter than syntactic block after optimization).

### 4.3 Slice and text views

`text` is owned UTF-8. `text[]` or method `.slice(start, end)` produces a borrowed view tied to owner's lifetime (compiler-tracked).

Returning a slice referencing local stack data is rejected:

```buraaq
fn bad() -> text[]:          # compile error
    local = "temporary"
    return local.slice(0, 3)  # would outlive `local`
```

---

## 5. Deterministic Destruction

### 5.1 Drop protocol

Types may define:

```buraaq
struct Resource:
    handle: Handle
    drop fn:
        os.close(handle)
```

Drop runs:

1. Field drops in declaration order
2. Struct `drop fn` body
3. Stack slot deallocated

### 5.2 Drop order in scope

```buraaq
fn demo():
    let a = Resource(...)
    let b = Resource(...)
    # drops: b, then a
```

### 5.3 Partial moves

If struct field moved, remaining fields still drop; moved field not dropped twice.

---

## 6. Heap Allocation

### 6.1 Default allocator

Global allocator wraps platform `malloc`/`free` (or embedded `dlmalloc`).

```buraaq
let list = new List[i32]()   # allocates List header + buffer
```

When `list` goes out of scope, `List.drop` frees buffer.

### 6.2 Custom allocators (v0.8)

```buraaq
arena = Arena.new(64 * 1024)
p = arena.alloc(Point { x: 1.0, y: 0.0 })
# no individual drop; arena.reset() or arena drop bulk-frees
```

### 6.3 Allocation failure

`new` returns `Result[T, AllocError]` in `#![fallible_alloc]` mode; default profile panics on OOM (matches Rust/C++ new behavior).

---

## 7. Interior Mutability

Safe interior mutation patterns:

| Type | Use case |
|------|----------|
| `Cell[T]` | Single-thread replace (`copy` T only) |
| `RefCell[T]` | Single-thread borrow runtime check (rare; has runtime cost) |
| `Mutex[T]` | Cross-thread |
| `Atomic[T]` | Lock-free primitives |

`let mut` binding alone does not allow mutation through shared reference—requires one of above or exclusive `ref mut`.

---

## 8. Reference Counting (Opt-In)

Not used by default.

| Type | Thread-safe | Use |
|------|-------------|-----|
| `Rc[T]` | No | Shared ownership single-thread |
| `Arc[T]` | Yes | Shared ownership cross-thread |

Cycles require `Weak[T]` to break—documented in std docs.

---

## 9. Unsafe Memory

Inside `unsafe:` or `unsafe fn`:

- Dereference `*T` / `*mut T`
- Call `extern c` functions not marked safe wrapper
- `transmute`-equivalent `mem.bitcast` (explicit API)
- Inline assembly `asm! { ... }`

Safe wrappers (e.g., `c.malloc` → `OwnedCBytes`) encapsulate unsafe invariants.

---

## 10. Concurrency and Memory

Cross-thread transfer requires `Send`. Shared references across threads require `Sync`. Compiler infers; see ADR 0009.

Data race on mutable aliased memory without synchronization is impossible in safe code.

---

## 11. Comparison to Other Models

| Aspect | C | Rust | Buraaq |
|--------|---|------|--------|
| Lifetime syntax | None | Explicit | None (inferred) |
| Move syntax | None | Default | Default + optional `give` |
| GC | No | No | No |
| RAII | Manual | `Drop` trait | `drop fn` in type |
| Borrow syntax | Manual | `&`/`&mut` | `ref`/`ref mut` when needed |

---

## 12. Diagnostic Examples

**E0301 borrow conflict:**

```
error[E0301]: cannot use `buffer` after moving it
  --> src/main.bq:14:5
   |
12 |     process(give buffer)
   |             ------------ `buffer` moved here
13 |
14 |     buffer.clear()
   |     ^^^^^^ use after move
   |
help: borrow instead of move if `process` accepts `ref mut bytes`
```

**E0310 escape violation:**

```
error[E0310]: returned reference to local variable `tmp`
help: return owned `text` instead of `text[]` slice
```

---

## 13. Embedded / `no_std` Profile

- No implicit heap: `new` disabled unless arena or custom allocator linked
- Panic = abort hook
- Stack-only by default; static allocation encouraged

---

## 14. Formal Properties (Safe Subset)

Safe Buraaq programs (no `unsafe`, no FFI unsafety) guarantee:

1. No use-after-free
2. No double-free
3. No out-of-bounds memory access (checked or proven safe)
4. No data races
5. Deterministic drop order

Proof strategy: GFA + MIR semantics formalized in `compiler/gfa/spec/` (future artifact).

This memory model aligns with [LANGUAGE_PHILOSOPHY.md](./LANGUAGE_PHILOSOPHY.md): complexity inside the compiler, not in source code.
