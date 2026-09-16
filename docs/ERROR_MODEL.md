# Buraaq Error Model

Buraaq treats recoverable failures as **values**, not control-flow exceptions. Unrecoverable programmer bugs use **panics**. This document defines error types, propagation, conversion, and interaction with I/O and concurrency.

---

## 1. Design Principles

1. **Errors are explicit in types** — callers know what can fail.
2. **Success path stays flat** — `?` propagates without nested `match`.
3. **Zero-cost on success** — `Result` compiles to direct returns when optimized.
4. **Must-not-ignore** — unused `Result` triggers warning (deny in release).
5. **No stack unwinding for errors** — only panics may unwind (profile-dependent).

---

## 2. Core Types

### 2.1 Result

```buraaq
enum Result[T, E]:
    Ok(T)
    Err(E)
```

### 2.2 Option

```buraaq
enum Option[T]:
    Some(T)
    None
```

Used for optional values, not general errors (prefer typed `E`).

---

## 3. Declaring Fallible Functions

### 3.1 Explicit Result return

```buraaq
fn read_byte(f: File) -> Result[u8, IOError]:
    ...
```

### 3.2 `throws` sugar

```buraaq
fn read_byte(f: File) throws IOError -> u8:
    ...
```

Desugars to `Result[u8, IOError]`. Body may:

- `return value` → wrapped as `Ok(value)` automatically
- `raise err` → `return Err(err)`
- `expr?` → propagate

---

## 4. Error Propagation (`?`)

### 4.1 Semantics

For expression `expr?` where `expr: Result[T, E]`:

```buraaq
match expr:
    Ok(v):
        v                    # bind value in place
    Err(e):
        return Err(convert(e))  # early return
```

### 4.2 Context requirements

Function must return:

- `Result[_, E]` with matching `E`, or
- `Result[_, E2]` where `From[E, E2]` exists, or
- `throws E2` sugar form

Otherwise:

```
error[E0402]: the `?` operator cannot be applied to type `Result[T, ParseError]`
  expected function to return `Result[_, IOError]`
  help: implement `From[ParseError, IOError]` or map error explicitly
```

### 4.3 Chaining

```buraaq
fn load(path: text) throws AppError -> Data:
    raw = File.read_all(path)?              # IOError → AppError via From
    parsed = parse(raw)?                    # ParseError → AppError
    return validate(parsed)?                # ValidationError → AppError
```

---

## 5. Error Conversion (`From`)

```buraaq
impl From[IOError, AppError]:
    fn from(err: IOError) -> AppError:
        return AppError.Io(err)
```

Compiler auto-inserts on `?` boundary.

Explicit map:

```buraaq
parse(raw).map_err(AppError.Parse)?
```

---

## 6. Handling Errors

### 6.1 match

```buraaq
match read_config("app.toml"):
    Ok(cfg):
        run(cfg)
    Err(e):
        log.error(e.message())
        exit(1)
```

### 6.2 if let

```buraaq
if let Ok(cfg) = read_config("app.toml"):
    run(cfg)
```

### 6.3 try block (v1.1 sugar)

```buraaq
cfg = try read_config("app.toml") catch e:
    log.error(e)
    return
```

---

## 7. Standard Error Trait

```buraaq
trait Error:
    fn message(self) -> text
    fn source(self) -> Option[Error]   # chain cause
```

All std error enums implement `Error`.

---

## 8. Panics vs Errors

| Mechanism | When | Recovery |
|-----------|------|----------|
| `Result` / `throws` | Expected failures (I/O, parse, network) | Caller decides |
| `panic(msg)` | Logic bug, violated invariant | Process abort/unwind |
| `assert cond` | Debug invariant | Debug panic; stripped in `--release` if `NDEBUG` |
| `expr!` unwrap | Quick prototype | Panic on Err/None |

### 8.1 Panic behavior

| Profile | Behavior |
|---------|----------|
| `debug` | Print message + stack trace; unwind if enabled |
| `release` | Abort by default (fast fail) |
| `release-unwind` | Unwind for FFI boundaries |

Embedded: panic → `abort()` hook.

---

## 9. Standard Library Error Types

| Type | Domain |
|------|--------|
| `IOError` | Files, sockets |
| `ParseError` | Text/byte parsing |
| `NetError` | DNS, connect, timeout |
| `AllocError` | OOM (fallible alloc mode) |
| `SyncError` | Mutex poison, channel disconnect |
| `Utf8Error` | Invalid UTF-8 conversion |

Each is an `enum` with structured variants:

```buraaq
enum IOError:
    NotFound(text)
    PermissionDenied(text)
    UnexpectedEof
    Os(code: i32, message: text)
```

---

## 10. Error Context Annotations

```buraaq
File.read_all(path)
    .context("failed reading config")?   # wraps message, preserves source chain
```

Implemented via `Context` trait on `Result`.

---

## 11. Logging and Observability

Errors implement `Display`/`Debug` for logging:

```buraaq
log.warn("request failed: {}", err)
```

Structured logging (v1.1): `err.to_json()`.

---

## 12. Concurrency and Errors

### 12.1 Thread join

```buraaq
match handle.join():
    Ok(result):
        ...
    Err(panic_payload):
        ...
```

Panics in spawned threads propagate as `JoinError.Panic`.

### 12.2 Channel errors

`recv()?` on closed channel → `SyncError.Disconnected`.

### 12.3 Async tasks

`await task` returns `Result[T, TaskError]` if task panicked.

---

## 13. FFI Errors

C functions return errno/sentinel — wrap in safe API:

```buraaq
fn open(path: c.text) -> Result[File, IOError]:
    unsafe:
        fd = c.open(path, O_RDONLY)
        if fd < 0:
            return Err(IOError.from_errno())
        return Ok(File.from_fd(fd))
```

Never expose raw errno to safe callers without conversion.

---

## 14. Lint Levels

| Lint | Default | Release |
|------|---------|---------|
| `unused_result` | warn | deny |
| `unwrap_used` | allow | warn |
| `panic_in_drop` | deny | deny |

Configure in `buraaq.pkg`:

```toml
[lints]
unused_result = "deny"
```

---

## 15. Diagnostic Quality

Errors include **actionable fixes**:

```
error[E0402]: cannot use `?` in `main`
  --> src/main.bq:5:25
   |
 5 |     let cfg = load("x.toml")?
   |                         ^ `main` returns `()`
   |
help: change signature to `fn main() throws AppError:` or handle with `match`
```

`buraaq explain E0402` expands with tutorial link.

---

## 16. Comparison

| Feature | Go | Rust | Buraaq |
|---------|-----|------|--------|
| Typed errors | optional | yes | yes |
| Propagation sugar | no | `?` | `?` |
| Exceptions | no | no | no |
| Signature sugar | no | no | `throws` |
| Must-use | no | warn | warn/deny |

This model keeps failure visible while minimizing ceremony—aligned with [LANGUAGE_PHILOSOPHY.md](./LANGUAGE_PHILOSOPHY.md).
