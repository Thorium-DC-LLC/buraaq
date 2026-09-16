# Buraaq Language Semantics

This document defines the **meaning** of Buraaq programs. Syntax is in [`SYNTAX_REFERENCE.md`](./SYNTAX_REFERENCE.md); formal grammar in [`grammar/buraaq.ebnf`](../grammar/buraaq.ebnf).

Buraaq semantics prioritize: **safe defaults**, **inference when sound**, **explicit diagnostics when ambiguous**.

---

## 1. Semantic Principles

1. **Single owner** for non-`copy` values unless borrowed.
2. **No null in safe code** — absence is `none`.
3. **No implicit narrowing** — `int` to `byte` requires explicit cast.
4. **No exceptions** — errors are `Result` values; panics are bugs.
5. **Deterministic drop** — resources release at scope exit, `defer`, or move.
6. **Thread safety inferred** — `Send`/`Sync` not written by users.
7. **Ambiguity is an error** — compiler never silently picks a type.

---

## 2. Name Binding

### 2.1 Binding forms

| Form | Semantics |
|------|-----------|
| `x = expr` | Introduce immutable binding `x` in current scope |
| `mut x = expr` | Introduce mutable binding; `x = ...` reassignment allowed |
| `const X = expr` | Compile-time constant; no runtime storage |

Rebinding without `mut` is a compile error:

```
error[E0101]: cannot assign to immutable binding `score`
help: declare as `mut score` if mutation is intended
```

### 2.2 Shadowing

Inner scope may bind same name, shadowing outer:

```buraaq
x = 1
{
    x = 2   # new immutable binding, outer x unchanged
}
```

### 2.3 Uninitialized use

Using a binding before definite assignment is rejected:

```
error[E0102]: use of possibly uninitialized `result`
```

---

## 3. Type Inference

### 3.1 Algorithm

Constraint-based **local** inference (inside function bodies):

1. Collect constraints from expressions and expected types (signature, assignment target).
2. Solve at function boundary.
3. If multiple solutions remain → error with annotation hint.
4. If no solution → error with expected/found types.

### 3.2 Default literal types

| Literal | Default type |
|---------|--------------|
| Integer | `int` (`i32`) |
| Float | `float` (`f64`) |
| `"..."` | `text` |
| `true`/`false` | `bool` |

### 3.3 Contextual typing

```buraaq
mut list: List[int] = List.new()
list.push(42)        # 42 typed as int from list element type
```

Empty collection without context:

```
error[E0205]: cannot infer type for `List.new()`
help: annotate: `List[int].new()` or binding type `List[int]`
```

### 3.4 Ambiguity policy

The compiler **never** silently coerces conflicting types. Example:

```buraaq
x = if cond { 1 } else { 2.0 }   # error: int vs float
```

Fix: make branches agree or add explicit cast.

---

## 4. Evaluation Order

- Function arguments: left-to-right.
- Binary operators: both operands evaluated left-to-right.
- Short-circuit: `&&`, `||`, `??` skip right operand when determined.
- Field evaluation in struct literal: declaration order.
- `match` arms: not evaluated until selected.

---

## 5. Function Semantics

### 5.1 Calls

Arguments passed **by move** for non-`copy` types, **by copy** for `copy` types. Caller loses moved values.

### 5.2 Return

- `return expr` — immediate exit.
- Trailing expression without `;` — implicit return (must match declared or inferred return type).

### 5.3 `throws` desugaring

```buraaq
fn f() throws E -> T { body }
```

Equivalent to:

```buraaq
fn f() -> Result[T, E] { body' }
```

Where:

- `return v` becomes `return Ok(v)`
- `raise e` becomes `return Err(e)`
- `expr?` propagates `Err`

### 5.4 `async fn`

Body transformed to state machine; calling without `await`/`spawn_task` returns `Task[T]` without running.

---

## 6. Control Flow Semantics

### 6.1 `if`

Condition must be `bool`. Branches must unify to same type when used as expression.

### 6.2 `while` / `for`

`for ident in expr` calls `expr.iter()` protocol (std defines for ranges, slices, collections).

Range `a..b` produces half-open `[a, b)`. `a..=b` inclusive.

### 6.3 `match`

Exhaustiveness required for enums unless `_` arm present. Pattern bindings introduce scope in arm body.

Guard clauses (future v1.1): `pat if cond =>`.

---

## 7. Struct Semantics

### 7.1 Layout

Default: compiler-reordered for size (`repr(Rust)`). `#[repr(C)]` for C layout.

### 7.2 Field access

`obj.field` — borrows or copies field per field type and use context (GFA).

### 7.3 Methods

`obj.method(args)` desugars to `Type.method(obj, args)` with appropriate ownership/borrow of `obj`.

---

## 8. Enum Semantics

Sum types with tagged representation. Niche optimization: e.g. `Option[*T]` same size as `*T`.

Variant payloads are moved out via `match`; use-after-move rejected.

---

## 9. Trait Semantics

Static dispatch by default (monomorphization). `dyn Trait` uses vtable + fat pointer.

Orphan rule: `impl` must be in crate defining type or trait.

---

## 10. Error and Option Semantics

### 10.1 `Result[T, E]`

Two variants: `Ok(T)`, `Err(E)`. Must be handled or propagated—unused `Result` warns (deny in release).

### 10.2 `?` operator

If `Ok(v)`, bind `v`. If `Err(e)`, convert via `From` if needed and return early.

Illegal in non-fallible function:

```
error[E0402]: the `?` operator cannot be used in a function that returns `void`
help: add `throws AppError` or handle with `match`
```

### 10.3 `Option[T]`

`some(v)` or `none`. No null pointer conversion in safe code.

### 10.4 Panics

`panic(msg)`, `assert(cond)`, `expr!` — unrecoverable in default release profile (abort).

---

## 11. Memory Semantics (Guarded Flow Analysis)

See [`MEMORY_MODEL.md`](./MEMORY_MODEL.md). Summary:

| Operation | Semantics |
|-----------|-----------|
| Assignment | Move if non-copy; copy if `copy` |
| `give x` | Explicit move marker |
| `ref x` | Shared borrow for borrow-checker |
| `ref mut x` | Exclusive borrow |
| Scope end | Drop fields reverse order, then bindings reverse order |
| `new T()` | Heap allocate; owned return |
| `defer expr` | Register cleanup at scope exit |

### 11.1 Iterator invalidation

Mutating collection while iterating by borrow is rejected:

```
error[E0301]: cannot borrow `list` as mutable while iterator borrow is active
```

---

## 12. Concurrency Semantics

### 12.1 `spawn`

New OS thread. Captured environment must be `Send`.

### 12.2 `parallel for`

Work items may run concurrently; barrier before next statement.

### 12.3 `Mutex` / `Channel`

`lock()` returns guard; guard not `Send` unless `T: Send`. Channel send/receive block or fail per API.

### 12.4 Data races

Undefined behavior in `unsafe` only. Safe code cannot data race.

---

## 13. Async Semantics

- `async { block }` — constructs `Task[T]`; lazy until `await`.
- `await task` — suspend until complete; resume with result.
- Borrows cannot cross `await` unless `'static` or owned—GFA enforced.

---

## 14. Unsafe Semantics

Inside `unsafe`:

- Dereference raw pointers
- Call unchecked extern functions
- Type punning via `mem.bitcast`

Safe invariants are programmer proof obligation. Misuse is UB.

---

## 15. FFI Semantics

- C ABI calling convention.
- `c.text` pointers valid only for documented lifetime (usually synchronous call).
- Buraaq strings are not NUL-terminated unless `to_c()` called.

---

## 16. Coercions (Explicit Only)

| Allowed | Not allowed |
|---------|-------------|
| Widening int in mixed arithmetic (`int` + `i64`) | Narrowing without `as` |
| `text` → `text[]` borrow at call | `float` → `int` silent |
| Subtyping none for `Option[T]` | Pointer ↔ integer |
| `Err` conversion via `From` on `?` | `bool` ↔ int |

Cast syntax: `expr as target_type` — only for defined safe casts; truncating casts require explicit intent and may warn.

---

## 17. Generic Semantics

Monomorphization: each `(function, type args)` is a distinct compiled instance.

Type parameters unconstrained accept any type satisfying operations used (implicit bounds) or explicit `T: Trait`.

Coherence: overlapping impls rejected at compile time.

---

## 18. Module Semantics

- One file = one module namespace.
- `pub` exports symbol to importers.
- Private items accessible only within package subtree.
- Cyclic imports forbidden.

---

## 19. Constant Evaluation

`const` values evaluated at compile time. Allowed subset (v0.5): literals, arithmetic, const fn calls, struct/enum constructors of const fields.

Runtime `const` evaluation errors are compile errors, not runtime failures.

---

## 20. Diagnostic Contract

When inference fails or safety cannot be proved:

1. **Primary message** — what failed, in plain language.
2. **Span labels** — related locations (borrow start/end).
3. **Error code** — stable `E####`.
4. **Help** — smallest fix suggestion.
5. **Never guess** — no default type pick on ambiguity.

Example:

```
error[E0205]: type annotations needed for `items`
  --> tour/05_type_inference.bq:3:9
   |
 3 |     items = []
   |         ^ cannot infer element type of empty list
   |
help: specify element type: `items: List[int] = []` or `items = [1, 2, 3]`
```

---

## 21. Undefined Behavior (Summary)

Only in `unsafe` or violating documented FFI contracts:

- Dangling pointer dereference
- Data race
- Type punning violation
- Double free / use-after-free (safe code prevents)

---

## 22. Relation to Other Documents

| Topic | Document |
|-------|----------|
| Syntax | SYNTAX_REFERENCE.md |
| Grammar | grammar/buraaq.ebnf |
| Memory details | MEMORY_MODEL.md |
| Types | TYPE_SYSTEM.md |
| Errors | ERROR_MODEL.md |
| Modules | MODULE_SYSTEM.md |
| Compiler | COMPILER_ARCHITECTURE.md |

Semantics version **0.2** aligns with brace-block syntax (ADR 0011) and PROMPT 2 surface language.
