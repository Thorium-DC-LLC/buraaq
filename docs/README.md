# Buraaq documentation

Read in this order. Skip what you do not need.

## Start

1. [Install](../README.md#install) — `install.ps1` / `install.sh`, then `buraaq doctor`
2. [Syntax](SYNTAX_REFERENCE.md)
3. [The Book](BURAAQ_BOOK.md)
4. [Standard library](STDLIB.md) — including **Stream**, **Hold**, **Grid**
5. [The stack](STACK.md) — Keel, Ship, Dock, Land

## Ship an API

- [Keel](SERVICE.md) — TLS pages and REST
- [CRUD walkthrough](CRUD_API.md)
- [Ship](SHIP.md) — pack, dock, land (Hetzner, AWS, Azure, GCP, bare metal)

## Systems

- [Memory safety](MEMORY_SAFETY.md)
- [Ownership / memory model](MEMORY_MODEL.md)
- [Concurrency](CONCURRENCY.md)
- [Unsafe](UNSAFE_BURAAQ.md)
- [Performance](PERFORMANCE.md)
- [Lumen](LUMEN.md) — optional native window (not the default path)
- `std.flowdesk` — borderless Windows shell + Vein (see [STDLIB.md](STDLIB.md); demo in private `buraaq-play`)

## Language internals (contributors)

- [Status](STATUS.md) — 1.0 evidence and remaining hardening
- [Bootstrap](BOOTSTRAP.md) — compiler written in Buraaq
- [Compiler architecture](COMPILER_ARCHITECTURE.md)
- [Internals](INTERNALS.md)
- [Roadmap](ROADMAP.md)

## Policy

- [Stability](STABILITY.md)
- [Security](../SECURITY.md) — legitimate testing only; Buraaq cooperates with agencies
- [Porting](PORTING_FROM_C.md) from [C](PORTING_FROM_C.md), [C++](PORTING_FROM_CPP.md), [Rust](PORTING_FROM_RUST.md)

Public guides: [buraaq.dev/docs](https://buraaq.dev/docs)
