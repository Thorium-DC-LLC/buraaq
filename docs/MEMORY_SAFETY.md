# Buraaq Memory Safety

Buraaq provides **Rust-grade memory safety without lifetime syntax**. Safety is enforced through a layered compile-time analysis pipeline that combines ownership tracking, region-based borrow checking, and Guarded Flow Analysis (GFA) on MIR.

---

## 1. Guiding Principle

> **Infer safety whenever it can be proven. Ask the programmer only when the compiler genuinely cannot determine intent.**

Normal application code should look like:

```buraaq
fn user_name(user: User) {
    print(user.name)
}
```

The compiler tracks ownership, borrows, and destruction internally. Programmers write `let`, `mut`, and occasionally `ref` / `give` — never `'a` lifetime annotations.

If safety cannot be proven, the compiler emits a precise diagnostic with a suggested fix. It never fakes safety.

---

## 2. Analysis Pipeline

```
AST
 ↓
Symbol resolution     (E01xx — unknown names, shadowing)
 ↓
Type checking         (E02xx — types, inference, generics)
 ↓
Ownership analysis    (E03xx — init, moves, use-after-move)
 ↓
Borrow checking       (E03xx — aliasing, conflicting borrows)
 ↓
MIR lowering
 ↓
Guarded Flow Analysis (E04xx — escape, async, Send/Sync)
 ↓
Code generation
```

### Compiler crates

| Crate | Responsibility |
|-------|----------------|
| `buraaq_types` | Interned type representation, built-ins, substitution |
| `buraaq_semantic` | Scopes, resolution, type inference, orchestration |
| `buraaq_ownership` | Definite initialization, move semantics, drop order |
| `buraaq_borrow` | Shared/exclusive loans, region stack, aliasing |

---

## 3. Ownership Model

### 3.1 Single owner

Every non-`copy` value has exactly one owner at any program point, unless actively borrowed.

| State | Meaning |
|-------|---------|
| `Uninitialized` | Declared but not yet assigned — use is an error (`E0301`) |
| `Valid` | Initialized and usable |
| `Moved` | Ownership transferred — further use is an error (`E0302`) |
| `PartiallyMoved` | A field or element was moved out |

### 3.2 Copy vs move

| Category | Behavior on assignment / pass-by-value |
|----------|----------------------------------------|
| **Copy types** | `int`, `float`, `bool`, `char`, tuples of copy types | Bitwise duplicate |
| **Move types** | `text`, `bytes`, structs, enums, containers | Ownership transfers |

Explicit operations:

| Syntax | Effect |
|--------|--------|
| `give x` | Explicit move (clarity when inference is ambiguous) |
| `copy x` | Explicit copy (only valid for copy types) |

### 3.3 Definite initialization

All bindings must be initialized on every control-flow path before use:

```buraaq
fn bad() {
    mut x: int
    if coin_flip() {
        x = 1
    }
    print(x)   # E0303: may be uninitialized
}
```

The ownership checker tracks initialization across `if`/`elif`/`else`, loops, and `match` arms.

### 3.4 Deterministic destruction

Drop order within a scope is **reverse declaration order** (stack unwind semantics):

```buraaq
fn demo() {
    a = open_file("a.txt")
    b = open_file("b.txt")
}   # `b` dropped, then `a`
```

User-defined `drop fn` blocks run before stack slot deallocation. The compiler inserts drop calls; programmers never call `drop()` manually.

---

## 4. Borrowing

### 4.1 When borrows appear

Most code never writes `ref`. The compiler inserts borrows internally for method calls and temporary views.

Programmers write `ref` / `ref mut` when:

- Taking an explicit borrowed view
- Satisfying a function signature requiring a reference
- Responding to a compiler suggestion

### 4.2 Rules

At any program point for a given place:

- **Either** one active `ref mut T` (exclusive),
- **Or** any number of active `ref T` (shared),
- **Never both**.

Violations emit `E0311` with the conflicting borrow highlighted.

### 4.3 Region inference

Every borrow is tagged with an inferred **region** (scope depth). Regions are:

- Created on block entry (`{`, `if` arms, loop bodies)
- Ended on block exit (all loans in that region expire)
- Never written in source code

This replaces explicit lifetime parameters.

### 4.4 Escape prevention

Returning a reference to a local is rejected (`E0314`):

```buraaq
fn dangling() -> text[] {    # compile error
    local = "temporary"
    return local.slice(0, 3)
}
```

Storing a short-lived borrow in a longer-lived container is rejected (`E0315`).

---

## 5. Null Safety

Buraaq has no null pointer type in safe code:

- `Option[T]` replaces nullable references
- `??` coalesce operator provides defaults
- Raw pointers (`*T`, `*mut T`) require `unsafe`

---

## 6. Bounds Safety

Array and slice indexing emits bounds checks in debug builds. The MIR optimization pass eliminates proven-safe accesses. Out-of-bounds access in safe code is a panic, never undefined behavior.

---

## 7. Immutability Policy

**Immutable by default:**

```buraaq
name = "Asim"        # immutable binding
mut count = 0        # explicitly mutable
```

Function parameters are immutable unless declared with `mut`. Struct fields follow the same rule.

This prevents accidental mutation and simplifies borrow analysis (immutable bindings cannot be `ref mut` borrowed).

---

## 8. Unsafe Blocks

Operations the compiler cannot guarantee are gated behind `unsafe { ... }`:

- Raw pointer dereference (`*T`)
- FFI calls with C memory ownership
- Type punning and unchecked casts
- Manual memory allocation outside `new`

Safe code cannot trigger:

- Use-after-free
- Double-free
- Dangling references
- Data races (when combined with Send/Sync checking in GFA)

---

## 9. Techniques for Lifetime Elimination

Buraaq combines several analyses so programmers rarely think about memory:

| Technique | What it does |
|-----------|-------------|
| **Ownership inference** | Single-owner by default; moves inferred at assignment/call |
| **Region analysis** | Lexical scope regions replace `'a` annotations |
| **Escape analysis** | Rejects references that outlive their referent |
| **Automatic borrowing** | Inserts `ref` for method calls when needed |
| **Deterministic destruction** | Drop insertion at scope end and on move-out |
| **Copy inference** | Primitives and marked types copy; everything else moves |

### What still requires explicit syntax

| Situation | Required construct |
|-----------|-------------------|
| Ambiguous move vs copy | `give x` or `copy x` |
| Explicit borrowed view | `ref x` or `ref mut x` |
| Raw/FFI memory | `unsafe { ... }` |
| Unprovable aliasing | Compiler suggests `ref` or refactor |

---

## 10. Error Code Reference

| Code | Category | Example |
|------|----------|---------|
| `E0101` | Resolution | Unknown identifier |
| `E0200` | Type | Incomplete AST node |
| `E0201` | Type | Type mismatch |
| `E0205` | Type | Ambiguous inference |
| `E0301` | Ownership | Use before initialization |
| `E0302` | Ownership | Use after move |
| `E0303` | Ownership | Uninitialized at scope exit |
| `E0310` | Borrow | Mut borrow of immutable |
| `E0311` | Borrow | Conflicting borrows |
| `E0312` | Borrow | Raw deref outside unsafe |
| `E0314` | Borrow | Return ref to local |
| `E0315` | Borrow | Borrow escape |

---

## 11. Adversarial Test Coverage

The compiler includes adversarial tests for:

- Use-after-move
- Use-before-init
- Double mutable borrow
- Mutable borrow of immutable binding
- Unknown names (resolution)
- Compile-time overhead benchmarks (200-binding functions < 5ms)

Future GFA tests (on MIR) will cover:

- Dangling pointers across function returns
- Closure capture and sendability
- Thread/spawn variable capture
- FFI ownership transfer
- Container interior mutability

---

## 12. Comparison with Rust

| Aspect | Rust | Buraaq |
|--------|------|--------|
| Lifetime syntax | `'a`, `'static` | None (inferred regions) |
| Borrow syntax | `&T`, `&mut T` everywhere | `ref` only when needed |
| Move syntax | Implicit + `move` closures | Implicit + `give` for clarity |
| Drop | `Drop` trait | `drop fn` block |
| Unsafe | `unsafe fn`, `unsafe {}` | `unsafe {}` |
| GC | None | None |
| Safety proof | Borrow checker on MIR | Ownership + Borrow + GFA |

Buraaq targets the same safety guarantees with less surface syntax, at the cost of a more complex compiler.
