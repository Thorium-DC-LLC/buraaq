# Buraaq 1.0 stable API surface

This is the freeze list for language + stdlib entry points in **Buraaq 1.0**.
Engineering ledger: [STATUS.md](STATUS.md).

## Language

Frozen syntax: `fn`, `struct`, `enum`, `trait`, `impl`, `use`, `module`,
`if` / `elif` / `else`, `while`, `break`, `continue`, `return`, `spawn`,
`unsafe`, `drop { }`, `extern c`, `test` / `expect`.

Ownership: move by default, `ref` / `ref mut`, GFA (E0302 / E0312).

## CLI

| Command | Contract |
|---------|----------|
| `buraaq new NAME` | Keel API scaffold (`page` / `api` / `run`) |
| `buraaq new NAME --ui` | Lumen native window scaffold |
| `buraaq up` | Pack + local Dock + ship |
| `buraaq land [user@HOST]` | Write (and optionally SSH) Dock install for any cloud |
| `buraaq build [--release]` | Native executable via LLVM + clang |
| `buraaq run [--release] [-- args…]` | Build and execute |
| `buraaq test` | Run `test "…" { expect … }` blocks |
| `buraaq check` | Analyze without linking |
| `buraaq pack` | Release-build → `target/ship/<app>.bur` |
| `buraaq launch FILE.bur` | Verify hash and run a ship |
| `buraaq dock` | Host agent — receive ships on `:7422` |
| `buraaq ship [HOST]` | Pack + launch locally, or push to a dock |
| `buraaq --sysroot` | Print stdlib root |

Install: `install.ps1` / `install.sh`.

## Standard library (stable names)

| Module | Functions |
|--------|-----------|
| `std.io` | `print`, `println` (auto type), `print_int`, `print_float`, `print_bool` |
| `std.fs` | `read`, `write`, `exists` |
| `std.text` | `len`, `concat`, `eq`, `byte`, `slice` |
| `std.math` | `abs_int`, `sqrt`, `min`, `max`, `sin`/`cos`/`tan`, `exp`/`log`/`pow`, `pi`, `clamp`, `lerp` |
| `std.grid` | `zeros`, `ones`, `eye`, `row`, `at`, `put_at`, `dot`, `matmul`, `sum`, `mean` — numeric arrays |
| `std.hold` | `hold`, `stow`, `pick`, `keep`, `from_csv`, `col_mean` — named columns |
| `std.stream` | `stream`, `wire`, `say`, `hear`, `hangup`, `run_stream` — live frames (WebSockets) |
| `std.time` | `now_ms`, `now_sec`, `sleep`, `ms`, `sec` |
| `std.os` | `getenv`, `args`, `arg` |
| `std.keel` | `page`, `api`, `store`, `key`, `origin`, `call`, `run` — TLS APIs. `std.service` is the old name. [STACK.md](STACK.md) |
| `std.lumen` | `app`, `heading`, `note`, `field`, `button`, `bind`, `show` — native HD UI. [LUMEN.md](LUMEN.md) |
| `std.flowdesk` | `desk`, `show` — borderless Windows shell + Vein (UI Automation text) |
| `std.http` | `get` — `file://` all hosts; `https://` via WinINet (Windows) or OpenSSL when linked |
| `std.db` | `connect`, `connected`, `exec`, `quote`, `disconnect` |
| `std.json` | `parse`, `Value.field` (string or number/bool token) |
| `std.crypto` | `sha256` — real SHA-256 hex |

## C runtime

Symbols in `stdlib/runtime/buraaq_std.h` are the FFI boundary. Do not call
`buraaq_runtime_init` from user `main`; the C `main` trampoline owns process
startup. `buraaq_rt_set_args` is invoked by the generated `main`.
