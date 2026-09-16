# ADR 0009: Threads + Structured Parallelism + Send/Sync Inference

## Status

Accepted

## Date

2026-09-13

## Problem

Data races are a primary source of systems bugs. Buraaq must provide excellent concurrency while keeping thread creation as simple as `spawn`. Async alone (ADR 0004) does not cover CPU-bound parallelism.

## Alternatives Considered

### A. OS threads only, no static race checking

**Pros:** Simple.  
**Cons:** Data races are UB in safe code—unacceptable.

### B. Rust Send/Sync with explicit markers

**Pros:** Proven.  
**Cons:** `Send + Sync` bounds appear in generics—noise.

### C. Compiler-inferred Send/Sync (no surface syntax unless error)

**Pros:** Beginners unaware until cross-thread move fails.  
**Cons:** Compiler must infer thread-safety structurally.

### D. Actor model only

**Pros:** No shared memory.  
**Cons:** Poor fit for systems/cache-friendly algorithms.

## Selected Design

**Alternative C: inferred Send/Sync with structural checking**

### Thread API

```buraaq
handle = spawn:
    work()

handle.join()
```

- Closure body must not capture non-`Send` values.
- `spawn` returns `JoinHandle[T]`.

### Structured parallelism

```buraaq
parallel for item in items:
    process(item)
```

Lowers to thread pool fork-join with deterministic barrier before continuation.

### Mutex / channels

```buraaq
lock = Mutex.new(data)
guard = lock.lock()
guard.field = 42
# guard dropped → unlock

ch = Channel[T].bounded(64)
ch.send(value)?
v = ch.recv()?
```

### Send / Sync rules (compiler-internal)

| Type | Send | Sync |
|------|------|------|
| `i32`, `f64`, `bool` | auto | auto |
| `ref T` | never | if `T: Sync` |
| `Mutex[T]` | if `T: Send` | yes |
| `Rc[T]` (single-thread ref count) | no | no |
| `Arc[T]` (atomic ref count) | if `T: Send` | if `T: Send` |

User never writes `Send` unless defining unsafe wrapper—compiler infers and explains violations.

## Advantages

- Threading looks like scripting languages; safety like Rust.
- Structured `parallel for` avoids manual thread pool code.

## Disadvantages

- Inference errors can be subtle—diagnostics must show capture chain.
- `Rc` vs `Arc` distinction requires teaching at some point.

## Performance Implications

- Mutex: platform futex/parking lot; no heavier than std Rust.
- Channels: lock-free bounded queue default; configurable.
- `parallel for` chunk size tuned by runtime (`num_cpus` aware).

## Implementation Implications

- MIR capture analysis for `spawn` closures.
- TSan integration in `--sanitize thread` debug builds.
- Std `thread`, `sync`, `channel` modules wrap OS primitives.

## Future Compatibility

- GPU / SIMD parallelism via separate `simd` module—not unified with threads.
- `atomic T` types with sequential consistency default; relaxed via method suffix `.relaxed()`.
