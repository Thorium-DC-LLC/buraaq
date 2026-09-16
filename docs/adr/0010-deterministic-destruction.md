# ADR 0010: Deterministic Destruction via Drop Protocol

## Status

Accepted

## Date

2026-09-13

## Problem

Resources (memory, files, locks) must be released predictably without GC. C++ RAII is powerful but requires manual destructor syntax. Buraaq needs automatic cleanup with opt-out for custom behavior.

## Alternatives Considered

### A. Manual `free()` everywhere

Rejected: error-prone.

### B. GC finalizers

Rejected: nondeterministic.

### C. C++ `~T()` destructors

**Pros:** Familiar.  
**Cons:** Exception interaction; syntax noise.

### D. Rust `Drop` trait auto-called

**Pros:** Explicit impl; clear.  
**Cons:** `impl Drop` ceremony.

### E. `drop fn` in type definition (Buraaq)

Single method in type body; compiler inserts calls.

## Selected Design

**Alternative E**

```buraaq
struct File:
    handle: os.Handle

    drop fn:
        os.close(handle)

    fn read(self, buf: mut bytes) throws IOError -> int:
        ...
```

- Compiler inserts drop at scope end, overwrite, and move-out unless `mem.forget(self)` in `unsafe`.
- Drop order: reverse declaration order in scope (same as Rust/C++).
- Drop cannot be called manually in safe code.
- Recursive drop on struct fields before struct body.

### Copy types

Types without `drop fn` and marked `copy` (primitives, simple aggregates) are bitwise-copied—no drop call.

### Panic in drop

If drop panics, process aborts after best-effort flush (same as Rust)—documented platform behavior.

## Advantages

- Resource cleanup colocated with type definition.
- No GC; suitable for embedded and real-time when drops bounded.

## Disadvantages

- Drop order education still needed for complex graphs.
- Circular `Rc` leaks require weak refs—documented pattern.

## Performance Implications

- Drops inlined through LLVM; elided when empty (trivial destructors).
- No runtime registration table.

## Implementation Implications

- MIR `Drop` terminators inserted by GFA pass.
- Lint: `drop fn` with empty body suggests removing it.
- `#[no_drop]` on union types only.

## Future Compatibility

- Deferred drops (`defer expr`) as syntactic sugar for scope exit—v1.1.
- Arena bulk-free bypasses per-object drop when `arena.alloc(T)` used.
