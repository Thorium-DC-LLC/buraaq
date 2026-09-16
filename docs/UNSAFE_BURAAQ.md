# Unsafe Buraaq

Safe Buraaq must not perform operations whose correctness the compiler cannot check.

## What requires `unsafe`

| Operation | Why |
|-----------|-----|
| Raw pointer dereference (`*p` on `ptr[T]`) | The compiler cannot prove the address is valid |
| Pointer arithmetic | Same — provenance and bounds are a human contract |
| Arbitrary integer↔pointer casts | Can fabricate invalid addresses |
| FFI calls that take raw pointers | The C callee's contract is not checked |
| Raw allocation / free | Pairing and aliasing are not tracked automatically |
| Union field access (if added) | Active variant is not tracked |

## What does **not** require `unsafe`

- Ordinary moves and borrows
- Indexing with a runtime bounds check
- Calling a safe `extern` wrapper that the stdlib authors audited
- Spawning tasks with owned or borrowed data the checker accepts

## The contract

An `unsafe` block means: **the programmer has checked the invariants**. The compiler still parses and type-checks the block; it does not turn off ownership for surrounding safe code.

Unsafe operations **outside** an `unsafe` block are compile errors (E0312 on raw dereference). `ptr[T]` is the raw-pointer type. Prefix `*` is dereference.

## Do not hide danger

A function that performs unsafe work should be `unsafe fn` (when that syntax is stable) or clearly documented. Wrapping raw FFI in a safe API is encouraged **only** when the wrapper enforces the C contract (null checks, lifetime, encoding).

## See also

- [UNDEFINED_BEHAVIOR.md](UNDEFINED_BEHAVIOR.md)
- [MEMORY_MODEL.md](MEMORY_MODEL.md)
