# Changelog

All notable changes to Buraaq are documented here.

Format based on [Keep a Changelog](https://keepachangelog.com/).

## [1.0.0] — 2026-09-16

Public language 1.0. Site: [buraaq.dev](https://buraaq.dev). Engineering ledger: [docs/STATUS.md](docs/STATUS.md).

### Added

- Self-hosted compiler frontend in Buraaq: lexer, parser, names, MIR, LLVM text (M3–M10). Guest LLVM compiles `lexer.bq` / `parser.bq`.
- Install prefers `dist/buraaq` + LLVM sidecar (`buraaq doctor`). Cargo is optional for *using* the language.
- `std.stream` — live frames (WebSocket job): `stream`, `wire`, `say`, `hear`
- `std.hold` — named columns (Pandas job): `hold`, `stow`, `pick`, `keep`
- `std.grid` — numeric arrays (NumPy job): `zeros`, `dot`, `matmul`
- Expanded `std.math` scalars
- Typed `print` / `println` (text, int, float, bool) and project auto-import of unique `std.*`
- Keel API keys, CORS allow-lists, auto TLS certs, `BURAAQ_HTTP_PORT` / `BURAAQ_TLS_PORT`
- `buraaq ship HOST --bundle`, `buraaq build --emit-ir --target linux`
- Land kit for Hetzner / AWS / Azure / GCP / bare metal
- Forge example: ownership, concurrency, Keel + Postgres, pack, land
- Compiler stress/fuzz smoke (mutated programs + random bytes)
- Loop `defer` on `break` / `continue`

### Performance

Gate B vs C++ `-O2` (clang 22, equivalent n): integer_sum **0.00×** (Orbit wrapping affine fold), nested_loop **0.16×**, float_saxpy 0.97×, numerical_loop 1.01×, fib_iter **1.02×**. n was not reduced. Worst case published.

### Self-host

M11: guest LLVM compiles `llvm.bq`, links, and `dump_llvm` emits a module. rustc still builds the host CLI.

### Security

Legitimate-use policy; Buraaq administration cooperates with lawful agency requests. See [SECURITY.md](SECURITY.md).

## [Unreleased]

### Added

- MIR optimization pipeline: constant folding, DCE, CFG simplification, small-call inlining
- `--release-fast` (`-O3` + thin LTO) and `--size` (`-Os`)
- Cross-language benchmark suite
- Buraaq Ship / Keel / Dock / Land
- Fuzz tests: lexer, package manifest parser

### Changed

- Parser missing-expression diagnostic uses **E0102** (E0101 reserved for unknown names)
- Ownership/borrow errors use multi-span teacher-style diagnostics

### Fixed

- E0101 code collision between parser and resolver
- Inverted `Span` ranges no longer panic on fuzz input

## [0.1.0] — pre-1.0 development

Initial public development snapshot: lexer, parser, semantic analysis, MIR, LLVM codegen, stdlib, package manager, concurrency runtime, LSP.
