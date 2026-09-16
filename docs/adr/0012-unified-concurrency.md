# ADR 0012: Unified Concurrency — spawn + Implicit Suspension

## Status

Accepted (supersedes split mental model in ADR 0004 §Disadvantages for surface syntax)

## Date

2026-09-13

## Problem

Buraaq needs concurrent software that is **easier than C++** and **less conceptually intrusive than typical async/await**. Two separate models (OS threads vs async tasks) force users to choose APIs early and invite "colored function" problems.

## First-Principles Design

### One surface, two execution strategies (compiler-chosen)

Programmers write:

```buraaq
spawn {
    process_request()
}

body = fetch("https://example.com/api")
```

The **compiler** classifies each function and call site:

| Context | `spawn { cpu work }` | `fetch(url)` | `sleep(ms)` |
|---------|---------------------|--------------|-------------|
| Sync function (default) | OS thread + `JoinHandle` | Blocks calling thread | Blocks |
| Task context (inside suspending spawn) | Lightweight task on executor | Yields thread; I/O on reactor | Timer wheel yield |
| `parallel for` body | Task on work-stealing pool | Same as task context | Same |

**No `async fn` required** for network code. `async fn` remains an opt-in explicit coroutine export for library authors who want typed `Task[T]` boundaries.

### Structured concurrency (always)

- Every `spawn` returns `JoinHandle[T]`; dropping without join is a **warning** (future: error under `#![deny(unstructured_concurrency)]`).
- `parallel for` lowers to scoped task group with barrier.
- Task groups nest: parent cannot complete until children complete (thread-local join stack).

### Send / Sync (inferred, never written)

Same rules as ADR 0009 — compiler rejects cross-thread capture of non-Send values with capture-chain diagnostics (E04xx). Users never write `Send` unless implementing unsafe FFI wrappers.

### Cancellation & timeouts

- `CancelToken` propagated into I/O waits; `fetch(url, timeout: ms(500))` registers timer + I/O race.
- Parent task group cancel cascades to children.

## Alternatives Rejected

| Alternative | Why rejected |
|-------------|--------------|
| JS-style async/await everywhere | Colored functions; infects parsers/kernels |
| Rust async state machines as default | Visible `Pin`, `Future`, `.await` noise |
| Go goroutines + GC | Violates zero-GC ownership model |
| C++ std::async only | No structured concurrency; easy to leak threads |

## Runtime Architecture

```
┌─────────────────────────────────────────┐
│  Buraaq program (compiled user code)    │
├─────────────────────────────────────────┤
│  Compiler: effect analysis → spawn kind │
│  (OS thread vs pool task vs coroutine)  │
├─────────────────────────────────────────┤
│  buraaq_runtime.c                       │
│  ├─ OS thread wrapper (pthread/Win32)     │
│  ├─ Work-stealing executor (N workers)  │
│  ├─ Reactor (poll/epoll / IOCP stub v1) │
│  ├─ Channels, mutex, atomics              │
│  ├─ Timer wheel (timeouts)              │
│  └─ Cancel tokens + task groups           │
└─────────────────────────────────────────┘
```

Linked only when concurrency is used (`buraaq build` always links in v0.7 dev; future: dead-strip unused).

## Implementation Phases

| Phase | Deliverable |
|-------|-------------|
| **v0.7a** (this prompt) | C runtime, benchmarks, design docs, stdlib FFI |
| v0.7b | MIR spawn → runtime calls; GFA Send checks |
| v0.7c | Implicit `fetch` suspension in task contexts |
| v0.8 | io_uring / IOCP reactor; real TCP async |

## Performance Claims

All claims in `docs/benchmarks/CONCURRENCY.md` — reproducible harness, pinned compiler versions, no tuned-for-Buraaq inputs.
