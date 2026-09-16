# Compiler internals

High-level map of the Rust reference compiler (`compiler/`).

## Pipeline

```
.bq source
  → Lexer (buraaq_lexer)
  → Parser + recovery (buraaq_parser)
  → Semantic: resolve, infer, ownership, borrow (buraaq_semantic)
  → MIR lower (buraaq_mir::lower)
  → MIR optimize (buraaq_mir::opt)     ← NEW
  → LLVM IR text (buraaq_codegen)
  → clang link + runtime C objects
  → native executable
```

## Crate responsibilities

| Crate | Role |
|-------|------|
| `source` | Spans, files, line/column |
| `diagnostics` | Structured errors, LSP adapter, snapshots |
| `ast` | Untyped/typed syntax tree |
| `types` | Type interner, DefId |
| `ownership` / `borrow` | Memory safety passes |
| `mir` | Mid-level IR + optimizations |
| `codegen` | LLVM emission + link |
| `driver` | End-to-end compile API |
| `frontend` | IDE analysis entry |
| `lsp` | Language Server |
| `pkg` | Projects, lockfile, cache |

## MIR optimization (measured)

Pass order in `mir/src/opt/mod.rs`:

1. Constant folding (+ branch folding)
2. CFG simplification (unreachable blocks)
3. Small-call inlining
4. Dead code elimination
5. CFG simplification (again)

Stats returned in `CompileOutput.mir_opt_stats`.

## Architectural debt (pre-1.0 refactor targets)

1. **Single-file project mode** — multi-module graph exists in `pkg` but driver builds entry only
2. **AST-level ownership** — should migrate to MIR + GFA (`docs/adr/0002`)
3. **Hand-written LLVM IR** — no LLVM C API; limits advanced opts
4. **Stdlib import resolution** — `use std.*` parses; full resolution incomplete
5. **Async/spawn lowering** — runtime exists; MIR codegen partial
6. **Duplicate diagnostic handler state** — LSP vs CLI paths differ slightly
7. **No incremental compilation** — file-level hash cache only

Refactor priority before 1.0: (1) multi-module driver, (2) std import resolution, (3) async lowering.

## Service runtime

`stdlib/runtime/buraaq_server.c` is the TLS + HTTP/1.1 + libpq loop behind `std.keel`. `stdlib/runtime/buraaq_lumen.c` is the native window behind `std.lumen`. Application code should not call `buraaq_http_listen` or Win32 directly. See [SERVICE.md](SERVICE.md), [LUMEN.md](LUMEN.md), and [STACK.md](STACK.md).

## Ship

`compiler/ship` packs native `.bur` archives and runs `buraaq dock`. See [SHIP.md](SHIP.md).

## Bootstrap alignment

`compiler-buraaq/` ports components in dependency order (M3 lexer **PASS**, M4 parser guest). Rust crates remain source of truth until golden tests match. Track: [BOOTSTRAP.md](BOOTSTRAP.md).

## Fuzzing surface

| Component | Test |
|-----------|------|
| Lexer | `lexer/tests/fuzz.rs` |
| Parser | `parser/tests/fuzz.rs` |
| Manifest | `pkg/tests/fuzz_manifest.rs` |
| Semantic | `semantic/tests/adversarial.rs` (hand-crafted) |

## Sanitizer workflow (developers)

```bash
# Nightly Rust recommended
RUSTFLAGS="-Zsanitizer=address" cargo test -p buraaq_parser
```

Invalid `.bq` must never abort the compiler process (except OOM).
