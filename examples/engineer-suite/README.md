# Buraaq Engineer Test Suite

Hands-on programs that mirror what engineers use day to day: variables, control flow,
functions, data modeling, errors, modules, filesystem, JSON/crypto, math, FFI, and
spawn — from **basic** to **advanced**, with **experimental** probes for unfinished areas.

Use this suite to shake out parser, codegen, and stdlib bugs (like the `"\n"` escape fix).

## Quick start

Requires **buraaq** and **clang** on PATH.

```powershell
cd examples\engineer-suite
..\..\install.ps1          # once, from repo root
.\run.ps1
```

```bash
cd examples/engineer-suite
../../install.sh           # once
chmod +x run.sh && ./run.sh
```

## Layout

| Path | Purpose |
|------|---------|
| `single/` | One-file programs — `buraaq run single/NN_name.bq` |
| `projects/` | Multi-module + stdlib integration (fs, json, crypto, math, **pico_blink**) |
| `experimental/` | Future syntax — compile-only probes (async, channels, parallel for) |
| `manifest.tsv` | Machine-readable list: path, mode, expected substring |

Pico W LED blink: `projects/pico_blink` — `buraaq run` on the host, or `.\flash.ps1` for the onboard LED (BOOTSEL).

## Categories covered

### Basics (01–07)
Hello, `\n` escapes, variables, `mut`, `if`/`elif`/`else`, `while`, `for` ranges, functions.

### Control & data (08–14, 17–20)
`break`/`continue`, structs (including trailing commas), error-style branching, generics, interpolation, arrays, `ref mut`, comparisons, modulo, nested blocks, enum/`match`.

### Systems (15–16, projects)
`spawn`, `extern c`, multi-module builds, `std.fs`, `std.json`, `std.crypto`, `std.math`.

### Experimental (compile-only unless `-Strict`)
`async`/`await`, `parallel for`, channels — failures are informational.

## Compiler gaps closed by this suite

| Area | Status |
|------|--------|
| Call then statement in same block | **Fixed** — tail heuristic only at block end |
| `for i in a..=b` lowering | **Fixed** |
| Trailing commas in `struct { ... }` | **Fixed** — was an infinite-parse OOM |
| Enum variant + `match` | **Fixed** for unit enums |
| `ref mut` parameters and `value[]` | **Fixed** |
| Array literals + index | **Fixed** |
| `extern c` declarations | **Fixed** |
| Struct field arithmetic (`p.x + p.y`) | **Fixed** |

## Still unfinished

- `async` / `await`, `parallel for`, channels
- Full `Result`/`Option` payload match (happy-path `if` works)
- Package registry, 7-day fuzz, full self-host

## Known patterns

1. **`ok` is a keyword** — use `Ok(x)` in patterns, not a variable named `ok`.
2. **`{` inside strings** starts interpolation — split literals or escape carefully.
3. **`buraaq test`** only evaluates `expect` with literal arithmetic — use `buraaq run` for real behavior.
4. Boolean logic uses `&&` / `||` (not `and` / `or`).

## Adding a test

1. Add `single/NN_topic.bq` or a folder under `projects/`.
2. Append a row to `manifest.tsv` (`id`, path, `single|project`, `run|compile`, expected substring).
3. Run `.\run.ps1` and fix any FAIL lines.

## CI

`cargo test -p buraaq_driver engineer_suite` runs the required subset from Rust (same manifest).
