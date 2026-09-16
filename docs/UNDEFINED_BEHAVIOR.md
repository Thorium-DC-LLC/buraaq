# Undefined behavior policy

## Safe Buraaq

A program that compiles without `unsafe` and without a compiler/runtime bug must not have **undefined behavior**. Wrong results, trapped overflows, or a clean abort are defined outcomes. Silent memory corruption is not.

Signed overflow in Buraaq is **wrapping** at the LLVM lowering used today (`wrapping_add` in MIR const-fold; LLVM `add` for runtime integers). This is defined, not C-style UB. It may be refined with debug traps later; that will be a documented change.

## Unsafe Buraaq

These are **undefined** if the programmer's contract is violated:

- Dereferencing a dangling, unaligned, or null pointer
- Data races on unsynchronized shared mutation
- Using a value after it was explicitly freed through raw allocation
- Lying to FFI about pointer validity or object lifetime

The compiler does not promise to diagnose these.

## Compiler bugs

Invalid MIR or LLVM IR is a **compiler bug**. The MIR verifier turns many of these into:

```text
internal compiler error (MIR): …
```

Report those; they are not user errors.
