# ADR 0004: Opt-In Async Task Model

## Status

Accepted

## Date

2026-09-13

## Problem

Async/await syntax in Rust, JavaScript, and C# "infects" entire call stacks—every callee must become async. Buraaq requires first-class async I/O without forcing synchronous systems code (drivers, parsers, game loops) to adopt async signatures.

## Alternatives Considered

### A. Async/await as default IO model (Rust async, Node.js)

**Pros:** Scales to millions of connections.  
**Cons:** Async contagion; colored functions; complex pinning.

### B. Callbacks only

**Pros:** No language support needed.  
**Cons:** Callback hell; poor ergonomics.

### C. Goroutines + scheduler (Go)

**Pros:** Simple `go fn()`.  
**Cons:** Requires GC or complex stack management; hidden scheduling.

### D. OS threads only

**Pros:** Simple mental model.  
**Cons:** Heavy for 100K connections; platform limits.

### E. Lazy async tasks + explicit await (opt-in)

**Pros:** Sync by default; async at boundaries; no signature contagion beyond one level.  
**Cons:** Two concurrency models to learn (threads + tasks).

## Selected Design

**Alternative E: Lazy Task Model**

### Syntax

```buraaq
task = async:
    data = socket.read(1024)?
    return process(data)

result = await task
```

- `async:` block produces type `Task[T]` (lazy; not started until `await` or `spawn_task`).
- `await expr` blocks current thread on task completion (uses thread pool + io_uring/epoll/kqueue/IOCP under std).
- `spawn_task async: ...` schedules on runtime thread pool immediately.

### No async contagion rule

Functions calling `await` must be marked `async fn` OR be inside an `async:` block. **Synchronous functions cannot await.** Callers of sync functions never change.

Bridge pattern:

```buraaq
fn handle_sync(conn: TcpStream):        // stays sync
    spawn_task async:
        await handle_async(conn)

async fn handle_async(conn: TcpStream) throws IOError:
    ...
```

### Runtime

- `std.async` provides default multi-threaded runtime (work-stealing queue).
- `#![no_async]` crate flag removes runtime; only blocking IO available.
- Embedded uses blocking IO only in v1.

## Advantages

- Majority of systems code stays synchronous and easy to reason about.
- High-concurrency servers use async at the edge only.
- Thread-based parallelism (`spawn`, `parallel for`) remains separate concern.

## Disadvantages

- Two models (threads vs tasks) vs one unified model.
- `await` inside sync code is a compile error—users must refactor to boundary.

## Performance Implications

- Tasks are stackless coroutines (LLVM coroutine lowering or custom state machines) — same order as Rust async when optimized.
- `await` on hot path in sync code forbidden—no hidden state machine bloat in parsers/kernels.
- Thread pool size defaults to `num_cpus`; tunable.

## Implementation Implications

- HIR distinguishes `async fn` from `fn`; MIR uses coroutine transform pass.
- GFA extended for suspended state across await points (borrow checking across yields).
- Std runtime crate `buraaq_rt` linked only if `async` used (dead code elimination otherwise).

## Future Compatibility

- `select`/`race` for multiple tasks in v1.1.
- io_uring-first Linux backend in v1.0 std.
- No plan to make all IO async-by-default.
