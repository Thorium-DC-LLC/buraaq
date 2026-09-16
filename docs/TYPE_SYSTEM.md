# Buraaq Type System

Buraaq uses **static structural typing** with **nominal records** (structs/enums), **parametric polymorphism**, and **trait-based constraints**. Type inference handles most local bindings; function signatures anchor polymorphism.

---

## 1. Type Categories

### 1.1 Primitive types

| Type | Size (platform) | `copy` | Notes |
|------|-----------------|--------|-------|
| `i8`…`i128`, `u8`…`u128` | fixed | yes | Two's complement |
| `f32`, `f64` | fixed | yes | IEEE 754 |
| `bool` | 1 byte | yes | |
| `char` | 4 bytes | yes | Unicode scalar |
| `()` | 0 | yes | Unit |

Platform-dependent: `isize`, `usize` (pointer-sized).

### 1.2 Composite types

| Syntax | Kind |
|--------|------|
| `(T1, T2, ...)` | Tuple |
| `[T; N]` | Fixed array (const N, v0.8) |
| `T[]` | Slice (fat pointer: ptr + len) |
| `struct S { ... }` | Product type |
| `enum E { ... }` | Sum type |
| `fn(A) -> B` | Function item / pointer |

### 1.3 Special types

| Type | Role |
|------|------|
| `text` | Owned immutable UTF-8 |
| `bytes` | Owned byte buffer |
| `Option[T]` | Optional value |
| `Result[T,E]` | Success or error |
| `Task[T]` | Async lazy computation |
| `dyn Trait` | Heap trait object (vtable) |
| `*T`, `*mut T` | Raw pointers (`unsafe`) |

---

## 2. Type Inference

### 2.1 Algorithm

Buraaq uses **constraint-based local inference** (Algorithm J variant):

1. Each expression generates type variables and constraints.
2. Function signatures provide anchor types for parameters/returns.
3. Constraints solved at function boundary; unsolved vars default per rules.

### 2.2 Default types

| Context | Default |
|---------|---------|
| Untyped integer literal | `i32` |
| Untyped float literal | `f64` |
| Empty collection `[]` | Error unless expected type provided |
| `let x = ...` | Solve from RHS |

### 2.3 Ambiguity resolution

If multiple solutions exist, compiler requests annotation:

```
error[E0205]: type annotations needed
help: specify type: `let items: List[i32] = List.new()`
```

---

## 3. Generics

### 3.1 Parametric functions and types

```buraaq
struct Box[T]:
    value: T

fn identity[T](x: T) -> T:
    return x
```

### 3.2 Constraints

```buraaq
fn max[T: Comparable](a: T, b: T) -> T:
    if a > b:
        return a
    return b
```

Multiple bounds: `T: Comparable + Printable`.

`where` clause for readability:

```buraaq
fn merge[K: Hash + Eq, V](a: Map[K,V], b: Map[K,V]) -> Map[K,V]:
    ...
```

### 3.3 Monomorphization

Each `(generic_item, type_args)` specialization is a distinct MIR function. Linker may merge identical instantiations with identical LLVM IR (ICF).

### 3.4 Const generics (v0.8)

```buraaq
struct Array[T, N: u64]:
    data: [T; N]
```

---

## 4. Traits

### 4.1 Definition

```buraaq
trait Iterator[T]:
    fn next(mut self) -> Option[T]
```

### 4.2 Implementation

```buraaq
impl Iterator[i32] for Range:
    fn next(mut self) -> Option[i32]:
        ...
```

### 4.3 Trait objects

```buraaq
fn print_all(it: dyn Iterator[i32]):
    while true:
        match it.next():
            Some(v):
                print(v.to_text())
            None:
                break
```

Dynamic dispatch via vtable; heap-allocated trait object unless `impl Trait` return optimized.

### 4.4 Built-in marker traits (compiler magic)

| Trait | Meaning |
|-------|---------|
| `copy` | Bitwise copy allowed |
| `Send` | Safe to transfer across threads |
| `Sync` | Safe to share immutably across threads |
| `Drop` | Has destructor (auto if `drop fn` present) |

Not written by users except `impl copy for T` with compiler verification.

---

## 5. Subtyping and Coercion

Buraaq has **limited subtyping**:

| Coercion | Example |
|----------|---------|
| Integer widening | `i32` → `i64` in mixed arithmetic |
| Float widening | `f32` → `f64` |
| Owned → borrow | `text` → `text[]` at call site (implicit ref) |
| Enum variant → enum | `Color.Red` is `Color` |
| Error conversion | `?` with `From[E1,E2]` |

No inheritance subtyping. No implicit numeric narrowing.

---

## 6. Pattern Types

Pattern matching binds types:

```buraaq
match value:
    Some(x):    # x: T where Option[T] expected
        ...
    None:
        ...
```

Exhaustiveness checked: all enum variants covered or wildcard `_` present.

---

## 7. Nullability

Buraaq has **no null pointer in safe code**. Optional values use `Option[T]`.

`!` postfix unwraps with panic on `None`/`Err`—intended for prototypes/tests; linter warns in library code.

---

## 8. Type Aliases

```buraaq
type UserId = u64
type IOResult[T] = Result[T, IOError]
```

Transparent for inference; distinct for documentation.

---

## 9. Associated Types

```buraaq
trait Container:
    type Item
    fn get(self, index: u64) -> Option[Self.Item]
```

Used in advanced generic code; beginners rarely need them.

---

## 10. Variance (Internal)

Compiler tracks variance for generic types:

| Type constructor | Parameter variance |
|------------------|-------------------|
| `List[T]` | covariant in `T` (if T send-safe contexts) |
| `fn(T) -> U` | contravariant in `T`, covariant in `U` |
| `RefCell[T]` | invariant in `T` |

Users never annotate variance.

---

## 11. Type Layout and ABI

### 11.1 Representations

```buraaq
#[repr(C)]
struct Point:
    x: f64
    y: f64
```

Default `repr(Rust)`: compiler may reorder for size unless `repr(C)` / `repr(packed(N))`.

### 11.2 Enum layout

Tag + payload optimized niche filling (e.g., `Option[*T]` same size as `*T`).

### 11.3 C ABI exports

`#[export(c)]` uses C calling convention and stable layout under `repr(C)`.

---

## 12. Type Checking Phases

1. **Name resolution** — bind identifiers to defs
2. **Signature checking** — validate declared types well-formed
3. **Body inference** — generate constraints
4. **Trait resolution** — resolve method calls to impls
5. **Coherence check** — no overlapping impls (orphan rules like Rust)
6. **MIR lowering** — monomorphize then GFA

---

## 13. Orphan Rules

Impl must be in crate defining type OR trait:

- `impl Printable for Point` in crate defining `Point` — OK
- External crate cannot impl external trait on external type

---

## 14. Error Message Philosophy

Type errors state:

1. Expected type
2. Found type
3. Origin of expectation (signature, context)
4. Minimal fix (`as` cast discouraged; prefer correct type or conversion fn)

Example:

```
error[E0210]: mismatched types
  expected `Result[Config, IOError]`
  found    `Config`
  note: help: wrap with `Ok(...)` or add `?` to inner call
```

---

## 15. Relationship to Memory Model

Types carry kind information for GFA:

- `copy` types ignore move tracking
- Non-copy types single-owner unless borrowed
- `Send`/`Sync` computed from struct fields and impl bounds

See [MEMORY_MODEL.md](./MEMORY_MODEL.md).

This type system prioritizes **invisible complexity**: inference at call sites, constraints at definitions, monomorphization for speed.
