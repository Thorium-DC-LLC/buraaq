# Performance guide

Buraaq targets **competitive performance with C++** on systems workloads — honestly measured, not marketing charts.

## Gate B (equivalent-n vs C++ `-O2`)

Buraaq `--release` versus C++ `-O2`, clang 22. Same `n` in both sources. Unfavorable numbers stay published.

| Bench | n | C++ `-O2` | Buraaq | Ratio |
|-------|---:|----------:|-------:|------:|
| nested_loop | 1e4² | 0.010s | 0.002s | **0.16×** |
| integer_sum | 1e8 | 0.015s | <0.001s | **0.00×** |
| float_saxpy | 1e7 | 0.004s | 0.004s | **0.97×** |
| numerical_loop | 1e8 | 0.083s | 0.084s | 1.01× |
| fib_iter | 1e8 | 0.020s | 0.021s | 1.02× |

Four benches beat or match C++. `integer_sum` (`sum = sum * 3 + i`) is an Orbit fold: wrapping affine matrix doubling in i32, O(log n), same n, checksum `44281` at n=10. The timed region falls under the clock; C++ still runs n=1e8 iterations. Worst published case is `fib_iter` **1.02×**. n was not reduced.

Signed `add` / `sub` emit LLVM `nsw` (same as clang for C++ signed math). `mul` stays wrapping so the recurrence matches C++ overflow. `for i in 1..=n` parses when `n` is an identifier.

Public write-up: [buraaq.dev/docs/systems/performance](https://buraaq.dev/docs/systems/performance). Ledger: [STATUS.md](STATUS.md).

## Optimization layers

| Layer | What runs | When |
|-------|-----------|------|
| **MIR** | const fold, DCE, CFG simplify, small inline, Orbit (affine wrap fold + unroll) | `--release`, `--release-fast` |
| **LLVM/clang** | inlining, vectorization, LTO | link step |
| **Runtime** | work-stealing executor, channels | concurrency programs |

Disable MIR opts for debugging codegen: `buraaq build --release --no-mir-opt`.

## Build profiles

| Flag | clang | MIR | LTO |
|------|-------|-----|-----|
| (default) | `-O0` | off | no |
| `--release` | `-O2` | on | no |
| `--release-fast` | `-O3` | on | thin (`-flto=thin`) |
| `--size` | `-Os` | on | no |

Gate B used `--release` so it matches C++ `-O2`. Use `--release-fast` when you have a profile that cares about the last percent.

## Measuring MIR optimizations

Each compile records `OptStats`:

- `const_folds` — folded arithmetic/branches
- `dead_stmts_removed` — unused assignments eliminated
- `blocks_removed` — unreachable CFG blocks
- `calls_inlined` — trivial call sites expanded

Run unit tests: `cargo test -p buraaq_mir opt::`

## Benchmark suite

```powershell
cd benchmarks
.\run_suite.ps1 -Release
.\run_suite.ps1 -ReleaseFast
```

Languages: `cpp/`, `rust/`, `go/`, `zig/`, `buraaq/`

Results: `benchmarks/results/suite/report-*.txt`

**Policy:** unfavorable Buraaq results are published. Investigate before claiming parity.

## Known gaps

- No monomorphization / devirtualization in MIR yet
- No escape analysis / stack promotion
- No bounds-check elimination pass
- Generic code lowers without specialization

## Tuning tips

1. Prefer `--release-fast` for production binaries on supported platforms
2. Keep hot loops free of unnecessary allocations — ownership moves have zero cost when optimized
3. Use `ref` borrows to avoid moves in inner loops
4. Write the obvious `while` or `for`. Orbit will close wrapping affine recurrences; forced unroll / alwaysinline made `integer_sum` worse when the algebra was missing
5. Profile with external tools (perf, VTune)
