# Buraaq Benchmarks

Two harnesses:

| Script | Purpose |
|--------|---------|
| `run.ps1` | Concurrency runtime (C harness vs Rust/Go/C++/Zig) |
| `run_suite.ps1` | Cross-language CPU / alloc / string microbenchmarks |

## Concurrency

```powershell
.\run.ps1              # -O0
.\run.ps1 -Release     # -O2
```

See [docs/benchmarks/CONCURRENCY.md](../docs/benchmarks/CONCURRENCY.md).

## Performance suite

```powershell
.\run_suite.ps1              # debug (-O0)
.\run_suite.ps1 -Release     # -O2
.\run_suite.ps1 -ReleaseFast # -O3 + thin LTO
```

Languages: `cpp/`, `rust/`, `go/`, `zig/`, `buraaq/`

Results: `results/suite/report-*.txt`

**Policy:** unfavorable Buraaq numbers are published — see [docs/STATUS.md](../docs/STATUS.md) and [docs/PERFORMANCE.md](../docs/PERFORMANCE.md). Gate B: `integer_sum` **0.00×** (Orbit fold, same n=1e8), `nested_loop` 0.16×. Worst case **1.02×** (`fib_iter`).
