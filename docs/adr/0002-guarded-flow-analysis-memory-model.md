# ADR 0002: Guarded Flow Analysis for Memory Safety

## Status

Accepted

## Date

2026-09-13

## Problem

Memory-safe systems languages typically expose ownership and lifetime syntax to the programmer (Rust) or defer safety to runtime (Go) or abandon safety (C/C++). Buraaq requires memory safety by default without a GC, without lifetime annotations, and without forcing programmers to learn affine type theory.

## Alternatives Considered

### A. Manual memory management (C/Zig default)

Programmer calls `alloc`/`free` or uses an allocator API explicitly.

**Pros:** Maximum control; zero analysis cost.  
**Cons:** Use-after-free and leaks are programmer errors; violates Buraaq safety goals.

### B. Garbage collection (Go, optional in some langs)

**Pros:** Simple value semantics for programmers.  
**Cons:** Nondeterministic pauses; unsuitable for embedded/real-time; hidden cost.

### C. Explicit ownership syntax (Rust model)

`&`, `&mut`, `'a`, `Pin`, etc.

**Pros:** Proven; precise; no runtime overhead.  
**Cons:** Steep learning curve; syntax infects all code; violates Buraaq philosophy.

### D. Reference counting by default (Swift/Swift-like)

**Pros:** Easier surface.  
**Cons:** Atomic refcount overhead; cycle leaks without weak refs; unpredictable retain traffic.

### E. Compiler-proven safety without surface syntax (Guarded Flow Analysis)

Compiler tracks ownership, borrows, and lifetimes internally; programmer uses `let`, `ref`, and `give`.

**Pros:** Safe by default; minimal syntax; deterministic destruction preserved.  
**Cons:** Complex compiler; occasional "compiler couldn't prove" errors requiring refactor.

## Selected Design

**Alternative E: Guarded Flow Analysis (GFA)**

### Surface syntax

| Construct | Meaning |
|-----------|---------|
| `let x = expr` | Bind value; ownership inferred |
| `ref x` | Shared borrow (read-only) when disambiguation required |
| `ref mut x` | Exclusive borrow when disambiguation required |
| `give x` | Explicit ownership transfer (move) |
| `new T(...)` | Heap allocation; returns owned `T` |
| `copy x` | Explicit copy; only valid if `T` implements `Copy` |
| `unsafe:` | Escape hatch for raw pointers and unchecked ops |

### Compiler responsibilities

1. **Linearity:** Non-`Copy` values have exactly one active owner unless borrowed.
2. **Borrow rules:** At most one `ref mut` OR any number of `ref` borrows—not both simultaneously.
3. **Lifetime inference:** Every borrow's scope is computed; no `'a` syntax.
4. **Destruction insertion:** `drop(x)` calls inserted at scope end for owned non-trivial types.
5. **Escape analysis:** Values cannot outlive their referents; returning interior pointers is rejected unless converted to owned data.

### Copy vs Move default

- Types marked `copy` (primitive numerics, `bool`, `char`, pointers in `unsafe`) duplicate bitwise on assignment.
- All other types **move** on assignment and argument passing—no implicit clone.

## Advantages

- Beginners write `let f = File.open("x")?` without learning ownership notation.
- Deterministic destruction without GC.
- Performance identical to Rust when analysis succeeds (same LLVM IR shape).
- Single escape hatch (`unsafe`) keeps safe subset auditable.

## Disadvantages

- Compiler team bears complexity cost.
- Some valid programs require renaming, scope splitting, or explicit `ref`—users may find errors unfamiliar until diagnostics improve.
- Analysis time increases compile time vs C (acceptable per philosophy).

## Performance Implications

- **Runtime:** Zero overhead vs manually correct Rust/C when in safe code—drops inlined, moves are pointer copies for heap handles.
- **Compile time:** GFA runs on MIR; expected O(n) per function with caching on incremental rebuild. Budget: ≤15% of total compile time for 10 KLOC modules.

## Implementation Implications

- MIR includes explicit `Move`, `BorrowShared`, `BorrowMut`, `Drop` operations for GFA.
- GFA errors mapped to source spans with `explain` subcommand showing borrow timeline.
- `Copy` trait is compiler-known for primitives; user types opt-in via `impl copy for T` only when provably bitwise-safe.
- Phase 1: intraprocedural GFA; Phase 2 (v0.9): interprocedural with summary edges.

## Future Compatibility

- Region-based allocation (`arena`) can layer on GFA without syntax changes.
- `@nogc` attribute for embedded profiles disables implicit heap `new` in a module.
- If a pattern repeatedly fails GFA, sugar like `with mut x:` blocks may be added—never lifetime annotations.
