# ADR 0003: Result-Based Error Handling with `?` Propagation

## Status

Accepted

## Date

2026-09-13

## Problem

Systems code must handle failure explicitly. Exceptions (C++, Java) hide control flow and add runtime cost. C errno/check patterns are error-prone. Go's `if err != nil` is verbose. Buraaq needs errors that are visible, composable, zero-cost, and readable for beginners.

## Alternatives Considered

### A. Exceptions with stack unwinding

**Pros:** Familiar to application developers.  
**Cons:** Hidden control flow; requires landing pads; unsuitable for `no_std`/embedded; violates predictability.

### B. errno / sentinel returns (C style)

**Pros:** Minimal runtime.  
**Cons:** Easy to ignore; no type safety; compositional nightmare.

### C. Multiple return values `(T, Error?)` (Go style)

**Pros:** Explicit.  
**Cons:** Verbose; `if err != nil` boilerplate; errors not in type signature unless documented.

### D. Typed `Result[T, E]` with `?` propagation (Rust-like semantics, Buraaq syntax)

**Pros:** Explicit; composable; zero-cost; errors visible in types.  
**Cons:** Generics on return types; learning curve for `?`.

### E. Effect system / algebraic effects

**Pros:** Flexible.  
**Cons:** Heavy compiler; unfamiliar errors; over-engineered for v1.

## Selected Design

**Alternative D** with Buraaq-specific sugar:

### Core types

```buraaq
enum Result[T, E]:
    Ok(T)
    Err(E)

enum Option[T]:
    Some(T)
    None
```

### Propagation

Postfix `?` on any expression of type `Result[T, E]`:

- Inside function returning `Result[_, E]`: early-return `Err`.
- Inside function returning `Result[_, E2]` where `E` converts to `E2` via `From`: wrap conversion.
- Inside `throws E` function: desugars to `Result` return (see below).

### `throws` sugar

```buraaq
fn load(path: text) throws IOError -> Config:
    data = read_file(path)?
    return parse_config(data)
```

Desugars to:

```buraaq
fn load(path: text) -> Result[Config, IOError]:
    ...
```

Beginners may omit `throws` and write `Result` explicitly; both are equivalent.

### Must-not-ignore

Discarding a `Result` without handling produces a **compile warning** (deny-by-default in `buraaq build --release`).

### Panics vs Errors

- **Errors**: recoverable, returned in-band.
- **Panics**: programmer bugs (`assert`, bounds failure in debug, explicit `panic()`). Unwind only when `build unwind` profile enabled (default off on embedded).

## Advantages

- Control flow visible in source and types.
- No exception runtime on hot paths.
- `?` eliminates nesting while staying readable.
- `throws` sugar lowers beginner barrier.

## Disadvantages

- Generic error types can produce long signatures (mitigated by type aliases and `throws`).
- Error conversion requires `From` impls or explicit mapping.

## Performance Implications

`Result` is a tagged union; on success path LLVM often optimizes tag checks away when inlined. No stack unwinding in default error path. Expected identical to Rust `Result` codegen.

## Implementation Implications

- Type checker tracks expected error type in `?` context.
- MIR lowers `?` to branch + return sequence.
- Standard library defines `Error` trait with `message() -> text` and `source() -> Option[Error]`.
- Diagnostic: "cannot use `?` in function returning `Config`; expected `Result` or `throws` clause."

## Future Compatibility

- `try` blocks for scoped error handling may be added: `try: ... catch e: ...`.
- Typed catch-all aliases (`type IOResult[T] = Result[T, IOError]`) encouraged in std.
- No exceptions planned for safe code.
