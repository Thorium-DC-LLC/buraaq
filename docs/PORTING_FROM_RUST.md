# Porting from Rust

## Similarities

- Ownership, moves, borrows (`ref` / `mut ref`)
- `Result` / `Option`, pattern matching
- No null — use `Option`
- LLVM backend

## Differences

| Rust | Buraaq |
|------|--------|
| `let` | `give` |
| `fn foo()` | `fn foo()` (same) |
| `'a` lifetimes | mostly elided; regions internal |
| `impl Trait` | traits + impl blocks |
| Cargo | `buraaq` CLI + `buraaq.pkg` |
| `unwrap()` culture | explicit `expect` in tests |

## Diagnostics

Buraaq prioritizes **multi-span teaching** over bare error codes. You'll see move sites and fix suggestions inline.

## Async

Single runtime model — not `async`/executor proliferation. See `docs/CONCURRENCY.md`.

## When Rust stays right

- Maximum ecosystem (crates.io scale) today
- Stable self-hosting compiler today
- `#![no_std]` embedded niches (Buraaq embedded track post-1.0)

Buraaq target: **same systems work, less cognitive tax**.
