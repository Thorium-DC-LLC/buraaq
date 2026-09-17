# Buraaq 1.0

[![Release](https://img.shields.io/github/v/release/Thorium-DC-LLC/buraaq)](https://github.com/Thorium-DC-LLC/buraaq/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-22d3ee.svg)](LICENSE)

**Write like Python. Run like C.**

A self-hosted systems language from **Thorium DC, LLC**. You write `.bq`. LLVM emits a native binary. There is no garbage collector, no interpreter, and no Docker image on the default path. Unique `std.*` names import themselves. Rust is not required to install.

```buraaq
fn main() {
    name = "Buraaq"
    println("Hello, {name}")
}
```

```text
buraaq new hello --cli
cd hello
buraaq run
```

That is not a sketch. That is the product. If you still think you need a VM, a venv, and a lifetime tutorial to ship a service — you have not run this yet.

Site: [buraaq.dev](https://buraaq.dev)

---

## Install (one step)

You need **Buraaq** and **clang**. You do **not** need Rust, Cargo, Node, or a JVM.

### Windows (recommended)

After the package is on winget (see [packaging/winget](packaging/winget/README.md)):

```powershell
winget install buraaq
buraaq doctor
```

Until the winget PR is merged, install from the [latest GitHub Release](https://github.com/Thorium-DC-LLC/buraaq/releases/latest) zip (`buraaq-*-windows-x64.zip`: exe + sysroot), or:

```powershell
git clone https://github.com/Thorium-DC-LLC/buraaq.git
cd buraaq
.\install.ps1
buraaq doctor
```

```bash
git clone https://github.com/Thorium-DC-LLC/buraaq.git
cd buraaq
./install.sh
buraaq doctor
```

The installer copies packaged `dist/buraaq` and the stdlib sysroot next to it, then sidecars LLVM if clang is missing. Cargo exists only for people who *package* a new `dist/` (`scripts/pack-dist.ps1`).

---

## What you type

| You type | You get | Not |
|----------|---------|-----|
| `.bq` | Native binary via LLVM | A VM, a GIL, a collector |
| `page` / `api` / `run` | TLS APIs and pages (**Keel**) | Express + a reverse-proxy homework |
| `stream` / `wire` / `say` | Live frames (**Stream**) | WebSockets-as-a-library |
| `hold` / `stow` / `pick` | Named columns (**Hold**) | Pandas |
| `zeros` / `dot` / `matmul` | Numeric arrays (**Grid**) | NumPy |
| `buraaq pack` / `ship` / `land` | Hashed `.bur` on any host | A guest Linux inside Linux |

Complexity stays in the compiler. Application source stays small.

```buraaq
fn main() {
    page("/", "public/index.html")
    api("accounts", "name, note")
    run()
}
```

`buraaq up` packs, docks, and runs. `buraaq land --cloud hetzner` then `buraaq ship HOST` is the same program on a real VM.

---

## Measured vs C++ `-O2`

Gate B: equivalent **n**, Buraaq `--release` vs C++ `-O2`, clang 22. n was not reduced to look pretty.

| Bench | n | C++ `-O2` | Buraaq | Ratio |
|-------|---:|----------:|-------:|------:|
| nested_loop | 1e4² | 0.010s | 0.002s | **0.16×** |
| integer_sum | 1e8 | 0.015s | <0.001s | **0.00×** |
| float_saxpy | 1e7 | 0.004s | 0.004s | 0.97× |
| numerical_loop | 1e8 | 0.083s | 0.084s | 1.01× |
| fib_iter | 1e8 | 0.020s | 0.021s | 1.02× |

`integer_sum` is an Orbit fold of wrapping `sum = sum * 3 + i` (same n=1e8; n=10 checksum `44281`). Worst published: `fib_iter`. Method: [docs/PERFORMANCE.md](docs/PERFORMANCE.md).

---

## Already on the metal

Buraaq ships native binaries and Land kits. End-to-end demos (Forge ledger, Flowdesk, etc.) live in a **separate** local tree — `buraaq-play` — and are not part of this public repository.

Secrets stay in host env. Never in git. Test only machines you own or are authorized to use. **Do not abuse.** [SECURITY.md](SECURITY.md) — Thorium DC cooperates with lawful agency requests.

---

## Documentation

| | |
|--|--|
| [buraaq.dev](https://buraaq.dev) | Public docs: install → ledger CLI → modules → live API → ship |
| [Syntax](docs/SYNTAX_REFERENCE.md) | The language |
| [The stack](docs/STACK.md) | Keel, Stream, Hold, Grid, Ship, Dock, Land |
| [Status](docs/STATUS.md) | What 1.0 measured, what still hardens |
| [Performance](docs/PERFORMANCE.md) | Gate B vs C++ |

The compiler frontend (lexer, parser, names, MIR, LLVM text) is written in Buraaq. rustc still *builds* the packaged CLI on a packager machine. Users never see it. That is not a fake rustc-off. [docs/BOOTSTRAP.md](docs/BOOTSTRAP.md).

## Layout

| Path | Purpose |
|------|---------|
| `compiler-buraaq/` | Compiler written in Buraaq |
| `stdlib/` | Standard library + C runtime |
| `examples/` | Language-tour / engineer-suite / release-gate |
| `benchmarks/` | Gate B vs C++ `-O2` |
| `docs/` | Spec, stack, book |
| `compiler/` | Host CLI used to *build* this tree |
| `dist/` | Packaged `buraaq` for `install.ps1` / `install.sh` |

```text
buraaq build
buraaq build --release
buraaq build --release-fast
buraaq check
```

---

Buraaq 1.0 is a **Thorium DC, LLC** project. Founding author: [Asim](https://linkedin.com/in/mdasimaslam).

Copyright © 2026 Thorium DC, LLC. MIT — see [LICENSE](LICENSE).
