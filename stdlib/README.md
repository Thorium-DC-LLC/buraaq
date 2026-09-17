# Buraaq Standard Library (`buraaq-std`)

The first Buraaq standard library — small surface area, **one obvious way** for common tasks.

## Layout

| Path | Module | Purpose |
|------|--------|---------|
| `src/io.bq` | `std.io` | Console output |
| `src/fs.bq` | `std.fs` | Files: read / write / exists |
| `src/text.bq` | `std.text` | String helpers |
| `src/math.bq` | `std.math` | scalars: trig, log, pow, pi |
| `src/grid.bq` | `std.grid` | numeric arrays (NumPy-shaped) |
| `src/hold.bq` | `std.hold` | named columns (Pandas-shaped) |
| `src/stream.bq` | `std.stream` | live frames (WebSockets) |
| `src/time.bq` | `std.time` | Clocks and sleep |
| `src/collections/` | `std.collections.*` | List, Map, Set, Buffer |
| `src/net.bq` | `std.net` | TCP / UDP |
| `src/keel.bq` | `std.keel` | APIs + TLS servers (`std.service` is the old name) |
| `src/lumen.bq` | `std.lumen` | Native HD windows |
| `src/flowdesk.bq` | `std.flowdesk` | Borderless Windows app + Vein text |
| `src/db.bq` | `std.db` | PostgreSQL hatch (`exec`, `quote`) |
| `src/http.bq` | `std.http` | HTTP client (call other APIs) |
| `src/json.bq` | `std.json` | JSON parse/stringify |
| `src/process.bq` | `std.process` | Child processes |
| `src/thread.bq` | `std.thread` | OS threads |
| `src/sync.bq` | `std.sync` | Mutex, Channel, Atomic |
| `src/async.bq` | `std.async` | Async tasks |
| `src/os.bq` | `std.os` | Environment, argv |
| `src/crypto.bq` | `std.crypto` | SHA-256 |
| `src/led.bq` | `std.led` | Board / host LED (`on` / `off` / `wait` / `blink`) |
| `src/ai.bq` | `std.ai` | Mind: `model` / `chat` / `embed` via AI serve |
| `runtime/buraaq_std.c` | — | Native runtime (no GC) |
| `runtime/buraaq_board.c` | — | Host stub for `std.led` |

## Tests & benchmarks

```bash
cd stdlib
cargo test          # parse all modules + C runtime unit tests
cargo bench         # allocation benchmark (text concat)
```

## Design principles

- **No garbage collector** — ownership + explicit C runtime allocation
- **Thin Buraaq wrappers** over `runtime/buraaq_std.c`
- **Safe wrappers** where practical; `unsafe` only at FFI boundary
- **C interop** via `extern c { ... }` blocks

See [STACK.md](../docs/STACK.md), [docs/STDLIB.md](../docs/STDLIB.md) for the API map, [docs/CRUD_API.md](../docs/CRUD_API.md) to stand one up, and [docs/SERVICE.md](../docs/SERVICE.md) for the Keel contract.
