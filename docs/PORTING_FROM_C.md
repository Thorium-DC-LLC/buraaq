# Porting from C

## Mental model map

| C | Buraaq |
|---|--------|
| `malloc` / `free` | owned values + deterministic drop (planned) / `new` |
| `struct` | `struct` with methods |
| `enum` | `enum` (algebraic) |
| `void*` | `unsafe` + raw pointers |
| `#include` | `use` + modules |
| `printf` | `print` / `println` |

## FFI

Keep C libraries via `extern c`:

```buraaq
extern c {
    fn strlen(s: *u8) -> usize
}
```

Wrap in safe Buraaq APIs in stdlib or your crate.

## Ownership

C pointer lifetimes become compiler-checked. If C "steals" a pointer, document with `unsafe` and narrow scope.

## Errors

Replace `errno` / return codes with `Result` where possible.

## Performance

Buraaq release builds use LLVM — comparable to clang `-O2`/`-O3`. No GC on hot paths.
