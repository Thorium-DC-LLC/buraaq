# ADR 0005: LLVM as Primary Code Generation Backend

## Status

Accepted

## Date

2026-09-13

## Problem

Buraaq must compile to optimized native binaries across Linux, macOS, Windows, and embedded ARM/RISC-V targets. The backend choice affects optimization quality, compile time, team velocity, and long-term maintenance.

## Alternatives Considered

### A. LLVM IR backend

**Pros:** World-class optimizer; broad target support; mature toolchains (lld, debug info); Rust/Swift precedent.  
**Cons:** LLVM version coupling; compile-time cost; large dependency.

### B. Cranelift backend

**Pros:** Faster compile times; Rust ecosystem integration.  
**Cons:** Narrower target support historically; less mature optimizations for aggressive systems workloads.

### C. Custom machine code emitter

**Pros:** Full control; potentially faster compile.  
**Cons:** Multi-year effort; optimization gap vs LLVM for v1.

### D. C transpilation (Cython/Zig stage1 style)

**Pros:** Bootstrap simplicity.  
**Cons:** Loses control of diagnostics mapping; double compile; debug info pain.

### E. Dual backend (LLVM + Cranelift)

**Pros:** dev=fast, release=optimized.  
**Cons:** Two codegen paths to test; doubles QA matrix.

## Selected Design

**Alternative A: LLVM 18+ as sole backend for v1.0**

### Pipeline position

```
Buraaq MIR → Buraaq LLVM IR (BIR) lowering → LLVM IR → object file → linker (lld or platform)
```

### Profiles

| Profile | LLVM opt | Debug info | Use |
|---------|----------|------------|-----|
| `debug` | -O0 | full | development |
| `release` | -O2 | line tables | production |
| `release-fast` | -O3 + LTO thin | line tables | benchmarks |
| `size` | -Os | none | embedded |

### Target triples (v1.0 GA)

- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `aarch64-apple-darwin`
- `aarch64-unknown-linux-gnu`
- `thumbv7em-none-eabihf` (embedded tier-2)

## Advantages

- Immediate parity with C/C++/Rust optimization on hot loops.
- Sanitizer support (ASan/TSan) via LLVM passes for debug builds.
- Single codegen path reduces maintenance.

## Disadvantages

- LLVM build dependency complicates bootstrap (mitigated: binary LLVM SDK bundled with releases).
- Compile time slower than Cranelift for debug builds (mitigated: incremental MIR cache).

## Performance Implications

- Runtime: best-in-class with `-O2`/LTO.
- Compile time: budget 3s cold / 200ms incremental for 10 KLOC module on laptop.

## Implementation Implications

- `buraaq` driver invokes bundled `llc`/`clang` linker driver or platform linker.
- Debug info: DWARF on Unix, PDB via LLVM on Windows.
- Coroutine lowering uses LLVM coroutine passes for async.
- CI pins LLVM version; ABI tests against clang compatibility.

## Future Compatibility

- Cranelift backend may be added as `buraaq build --backend=cranelift` for debug iteration in v1.x—does not replace LLVM for release.
- WASM target via LLVM `wasm32-unknown-unknown` in v1.1.
