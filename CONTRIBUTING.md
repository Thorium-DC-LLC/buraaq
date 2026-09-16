# Contributing to Buraaq

Thank you for helping make Buraaq production-ready.

## Getting started

1. Clone the repository
2. `cd compiler && cargo test` — all tests must pass
3. Read `docs/INTERNALS.md` and `docs/COMPILER_ARCHITECTURE.md`

## Change guidelines

- **Minimal diffs** — match existing style in each crate
- **Diagnostics first** — user-facing errors need stable IDs, spans, and help text
- **Measure optimizations** — update `mir/tests/opt.rs` or benchmarks; no blind `-O3`
- **No silent unsafe** — document FFI and `unsafe` escape hatches

## Pull request checklist

- [ ] `cargo test` passes in `compiler/`
- [ ] New syntax has parser recovery + at least one grammar test
- [ ] User-visible behavior documented in `docs/` or CHANGELOG
- [ ] Fuzz-sensitive code (lexer, parser, pkg) has no new `unwrap()` on untrusted input

## Running benchmarks

```powershell
cd benchmarks
.\run_suite.ps1 -Release        # -O2
.\run_suite.ps1 -ReleaseFast    # -O3 + thin LTO
```

Report unfavorable results honestly in PR descriptions.

## Fuzzing

```bash
cd compiler
cargo test -p buraaq_parser fuzz_
cargo test -p buraaq_lexer fuzz_
cargo test -p buraaq_pkg fuzz_
```

## Code of conduct

Be direct, technical, and respectful. Debate design in issues/ADRs, not personal terms.
