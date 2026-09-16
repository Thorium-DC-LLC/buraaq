# The Buraaq Book

*A narrative guide for systems programmers who want less ceremony and more clarity.*

## Part I — Why Buraaq?

Modern systems languages split into two failures:

1. **C/C++** — full control, minimal safety, maximal cognitive load
2. **Rust** — safety, but the compiler feels like a second job

Buraaq asks: *what if memory safety and performance were defaults, and the compiler taught instead of scolded?*

## Part II — First program

```buraaq
use std.io.println

fn main() {
    println("Hello, Buraaq!")
}
```

Run: `buraaq run`. No build system debates — `buraaq.pkg` + `buraaq` CLI.

## Part III — Ownership without jargon

Bindings use `give`:

```buraaq
give socket = connect("127.0.0.1", 8080)
send_all(socket, payload)   # ownership may transfer
# socket is gone here — compiler explains if you use it again
```

Need temporary access? **Borrow:**

```buraaq
send_all(ref socket, payload)
socket.close()   # still owned here
```

The compiler shows *where* ownership moved and *what to change* — not `E0382`.

## Part IV — Errors that teach

Diagnostics include:

- Primary span (where you went wrong)
- Secondary spans (where the conflict started)
- Plain-language help
- Suggested fix (LSP code action)

See `docs/errors/` for stable error catalog entries.

## Part V — Concurrency one way

Threads, async tasks, channels, and mutexes share one runtime (`stdlib/runtime/`). No three async ecosystems.

```buraaq
give ch = channel[i32](64)
spawn {
    ch.send(42)
}
give v = ch.recv()
```

## Part VI — Tooling as a feature

| Tool | Command |
|------|---------|
| Build | `buraaq build --release` |
| Test | `buraaq test` |
| Format | `buraaq fmt` |
| IDE | `buraaq lsp-server` + VS Code extension |

## Part VII — Performance

Buraaq lowers to LLVM. Use `--release-fast` for production. Measure with `benchmarks/run_suite.ps1`.

See [PERFORMANCE.md](PERFORMANCE.md).

## Part VIII — Production services

Complexity belongs in the runtime. You declare pages and APIs; `run()` binds TLS and serves.

```buraaq
use std.keel.{page, api, run}

fn main() {
    page("/", "public/index.html")
    api("items", "title, body")
    run()
}
```

See [STACK.md](STACK.md) for Keel → Ship → Dock → Land (AWS, Azure, GCP, Hetzner, bare metal).

## Part IX — Stability and 1.0

Pre-1.0: expect evolution. At 1.0: syntax + ownership rules frozen per [STABILITY.md](STABILITY.md).

## Part X — Porting

- [From C](PORTING_FROM_C.md)
- [From C++](PORTING_FROM_CPP.md)
- [From Rust](PORTING_FROM_RUST.md)

## Part XI — What Buraaq is not

- Not a scripting language with hidden GC
- Not a Rust clone with different spelling
- Not "simple" by removing `unsafe` when you need FFI

**Simplicity through design, not deletion.**
