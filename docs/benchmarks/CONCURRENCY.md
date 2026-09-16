# Concurrency Benchmark Methodology

Reproducible comparison of Buraaq's native runtime against C++, Rust, Go, and Zig.

## What we measure

| Benchmark | Metric | Description |
|-----------|--------|-------------|
| `task_spawn` | ns/op | Submit + join empty task (50,000 iterations) |
| `channel_ping` | ns/op | Send + recv on bounded channel (200,000 iterations, cap=256) |
| `mutex_contention` | ns/op | Lock/inc/unlock single mutex (500,000 iterations, 1 thread) |
| `context_switch` | ns/op | Two tasks ping-pong via capacity-1 channel (100,000 rounds each) |
| `tcp_throughput` | Mbit/s | *Planned v0.8* — echo server, 1 KiB messages |
| `http_throughput` | req/s | *Planned v0.8* — local stub server |
| `multi_core_scaling` | speedup | *Planned* — parallel sum N=10M across 1..N cores |
| `async_file_io` | ns/op | *Planned v0.8* — read 4 KiB files from tmpfs |

v0.7a ships the first four; network/async I/O await reactor backend (ADR 0012 phase v0.8).

## Hardware (fill when running)

Document your machine when publishing results:

```
CPU:     <model, e.g. AMD Ryzen 7 5800X>
Cores:   <physical / logical>
RAM:     <size>
OS:      <Windows 11 / Ubuntu 24.04>
```

## Compiler / toolchain versions

Record exact versions from your run:

```powershell
clang --version
rustc --version
go version
g++ --version   # or clang++ for libc++
zig version
```

### Optimization flags (must match across native benchmarks)

| Language | Debug run | Release run |
|----------|-----------|-------------|
| Buraaq runtime (C) | `-O0` | `-O2` |
| Rust | `cargo run` (dev) | `cargo run --release` |
| Go | default | default (Go always optimizes) |
| C++ | `-O0 -std=c++17` | `-O2 -std=c++17` |
| Zig | `-O Debug` | `-O ReleaseFast` |

**We do not** use LTO, PGO, or `-march=native` unless all languages support equivalent flags.

## Source implementations

| Benchmark | Buraaq | Rust | Go | C++ | Zig |
|-----------|--------|------|-----|-----|-----|
| task_spawn | `benchmarks/harness/buraaq_task_spawn.c` | `refs/rust/task_spawn/` | `refs/go/task_spawn/` | `refs/cpp/task_spawn.cpp` | `refs/zig/task_spawn.zig` |
| channel | `benchmarks/harness/buraaq_channel.c` | `refs/rust/channel/` | `refs/go/channel/` | — | — |

Buraaq harness links `stdlib/runtime/buraaq_runtime.c` + `buraaq_std.c`.

### Fairness rules

1. **Same iteration counts** hard-coded in each source file.
2. **Same algorithms** — empty task, sync channel ping-pong, mutex increment.
3. **No warm-up gaming** — one timed loop; report cold-start inclusive.
4. **Single process** — no fork between timed regions.
5. **Buraaq task_spawn** uses work-stealing pool; Rust/Go/C++/Zig refs use **OS threads** for spawn benchmark (apples-to-apples for thread creation cost). A separate `task_spawn_pool_*` comparison is planned when all runtimes expose a pool API.

## How to reproduce

```powershell
cd benchmarks
.\run.ps1              # debug / -O0
.\run.ps1 -Release     # release / -O2
```

Output format (machine-parseable):

```
BENCH|<name>|iters=<N>|total_sec=<S>|per_op_ns=<NS>
```

Results append to `benchmarks/results/run_<timestamp>.txt`.

## Interpreting results

- **Lower `per_op_ns` is faster** for latency benchmarks.
- Mutex benchmark is **single-threaded** — measures lock/unlock cost, not scalability.
- Context switch includes channel synchronization + scheduler overhead.
- Buraaq v1 executor uses **mutex-backed deques**; lock-free upgrade will change numbers — document version in results.

## Example results (illustrative — run locally)

Replace with your `run_*.txt` output. Do not cite numbers without running on your hardware.

```
# Example structure only — NOT measured claims
BENCH|task_spawn_buraaq|iters=50000|total_sec=...|per_op_ns=...
BENCH|task_spawn_rust|iters=50000|total_sec=...|per_op_ns=...
```

## Buraaq compiler version

When benchmarking compiled `.bq` programs (future):

```
buraaq --version
# Record commit SHA, --release flag, target triple
```

Current harness benchmarks the **C runtime directly** — the same code linked by `buraaq build`.

## Reporting issues

If Buraaq wins or loses dramatically, verify:

1. Debug vs release matched across languages
2. CPU frequency scaling disabled (optional, document if enabled)
3. Antivirus / background load noted
4. Iteration counts unchanged from source

We **do not** tune iteration counts, buffer sizes, or thread counts to favor Buraaq.
