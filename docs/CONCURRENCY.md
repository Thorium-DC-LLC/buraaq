# Buraaq Concurrency Model

Buraaq concurrency is designed **from first principles**: one obvious way to run work concurrently, with the compiler choosing how — not the programmer.

## Core idea

You write sequential-looking code. The compiler decides whether an operation **must block an OS thread** or **can suspend a lightweight task**.

```buraaq
# CPU-bound — compiler picks OS thread (avoid pool starvation)
spawn {
    hash_password(input)
}

# I/O — in a task context, yields instead of blocking a thread
response = fetch("https://api.example.com/v1/data")
data = json.parse(response.body)
```

There is **no async contagion**: synchronous functions stay synchronous unless they contain suspending calls in a context that the compiler promotes to a task.

## Primitives

### `spawn { ... }`

Creates concurrent work. Returns `JoinHandle[T]`.

- **CPU-shaped** bodies (no I/O effects) → **native OS thread**
- **I/O-shaped** bodies → **executor task** on work-stealing pool
- Compiler effect analysis makes the choice; same syntax either way

```buraaq
handle = spawn {
    process(item)
}
result = handle.join()
```

### `parallel for`

Structured fork-join over collections:

```buraaq
parallel for item in items {
    transform(item)
}
# implicit barrier — all iterations complete before continuing
```

### `fetch(url)` and blocking-looking I/O

`fetch` is the canonical HTTP GET. In sync code it blocks. Inside task/spawn-I/O contexts the runtime registers the socket with the reactor and **yields** the worker.

Timeouts are explicit:

```buraaq
body = fetch(url, timeout: ms(2000))
```

### Channels

```buraaq
ch = channel[int](64)   # bounded capacity 64
ch.send(42)
v = ch.recv()
```

One channel type — bounded, blocking send/recv (task-aware when in executor).

### Synchronization

| Type | Purpose |
|------|---------|
| `Mutex[T]` | Exclusive access |
| `AtomicInt` | Lock-free counter |
| `RwLock[T]` | Many readers / one writer (planned) |

### Cancellation

```buraaq
token = cancel_source()
spawn {
    if token.is_cancelled() { return }
    long_running()
}
token.cancel()
```

Task groups inherit parent cancellation.

## Safety without syntax noise

Buraaq infers **Send** and **Sync** (see ADR 0009). Cross-thread capture errors explain the chain:

```
error[E0401]: cannot spawn: `Rc[Buffer]` is not Send
  note: captured here in closure passed to spawn
  note: `Rc` uses non-atomic reference counts — use `Arc` for sharing across threads
```

No `Send + Sync` bounds in user signatures unless writing `unsafe extern` shims.

## Runtime (`buraaq_runtime.c`)

Small, native, **no GC**:

- Work-stealing deque per worker (mutex-backed in v1; lock-free upgrade path documented)
- OS threads via pthread / Win32
- Channels: ring buffer + condvar
- Atomics: C11 `_Atomic` / platform intrinsics
- Timer wheel for timeouts (coarse ms resolution v1)
- Reactor stub for async I/O benchmarks (real epoll/IOCP in v0.8)

## What we deliberately avoid

- Hidden thread pools behind every call (Go-style)
- Mandatory `async fn` on all I/O (Rust/JS-style)
- Garbage-collected goroutine stacks
- Exposing executor configuration in beginner code

## Compiler pipeline (roadmap)

```
AST → effect analysis (cpu/io/sync) → spawn lowering → MIR → GFA (Send/await) → LLVM + runtime
```

Current status: runtime + benchmarks land in v0.7a; full MIR lowering in v0.7b.

## See also

- [ADR 0012: Unified Concurrency](adr/0012-unified-concurrency.md)
- [ADR 0009: Send/Sync Inference](adr/0009-concurrency-model.md)
- [Benchmark methodology](benchmarks/CONCURRENCY.md)
