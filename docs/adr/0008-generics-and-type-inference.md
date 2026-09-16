# ADR 0008: Parametric Generics with Local Type Inference

## Status

Accepted

## Date

2026-09-13

## Problem

Systems languages need generic containers and algorithms without C++ template error spew or explicit type parameter ceremony on every call site. Buraaq must monomorphize for performance while keeping call sites clean.

## Alternatives Considered

### A. Monomorphized generics with required type args (`Vec[T]` always explicit)

**Pros:** Simple compiler.  
**Cons:** Verbose call sites.

### B. Hindley-Milner global inference (ML/Haskell)

**Pros:** Minimal annotations.  
**Cons:** Slow error messages; ambiguous types in large APIs.

### C. Rust-style generics + turbofish (`Vec::<i32>::new()`)

**Pros:** Precise.  
**Cons:** Turbofish is noise; Buraaq rejects as default UX.

### D. Local inference + constraints on declarations only

Function signatures declare type parameters; call sites infer unless ambiguous.

## Selected Design

**Alternative D**

### Syntax

```buraaq
fn first[T](list: List[T]) -> Option[T]:
    if list.is_empty():
        return None
    return Some(list[0])

fn main():
    nums = List[i32].from([1, 2, 3])
    x = first(nums)          # T inferred as i32
```

- Type parameters in square brackets on types and functions: `List[T]`, `fn map[A, B](...)`.
- Inference applies to: local `let`, function call type args, numeric literals (default `i32`/`f64` unless context demands).
- Constraints use `where` clauses only when necessary; trait bounds inline: `fn sort[T: Comparable](...)`.

### Monomorphization

Default: full monomorphization at compile time—each `(fn, type args)` tuple generates specialized MIR/LLVM IR.

Dynamic dispatch via `dyn Trait` object types is opt-in for runtime polymorphism (rare in systems code).

## Advantages

- Call sites stay clean.
- Errors localized to function definition when constraint fails.
- Performance matches C++ templates without header-only model.

## Disadvantages

- Code bloat if many instantiations (mitigated: `-Os`, dedup pass).
- Complex error messages possible—compiler must truncate instantiation chains at depth 5 with summary.

## Performance Implications

- Zero-cost: monomorphized code identical to hand-specialized version.
- Compile time: cache specialization `(symbol, type_hash)` for incremental builds.

## Implementation Implications

- Type checker maintains substitution map during inference.
- MIR duplication pass creates specialized functions before GFA.
- Name mangling: `_ZN4list5firstIiE...` Itanium-style for linker compatibility.

## Future Compatibility

- Const generics `Array[T, N]` in v0.8.
- Specialization (`impl for T where T: ...`) deferred to v1.2 to avoid coherence complexity early.
