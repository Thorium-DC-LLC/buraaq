# ADR 0007: C FFI Strategy — `extern c` Blocks and C ABI by Default

## Status

Accepted

## Date

2026-09-13

## Problem

Systems languages must interoperate with existing C libraries, OS APIs, and hardware interfaces. Binding generators add friction. Buraaq requires calling C to be as straightforward as including a declaration.

## Alternatives Considered

### A. Separate binding generator (cbindgen/swift-clang-importer style)

**Pros:** Handles large headers.  
**Cons:** Extra tool; stale bindings; beginner unfriendly.

### B. Inline `extern c` declarations in Buraaq source

**Pros:** Explicit; no generator for common cases.  
**Cons:** Manual for huge APIs.

### C. Direct `#include` in Buraaq

**Pros:** Automatic.  
**Cons:** Brings C preprocessor; type system pollution.

### D. `@link` only, all declarations in `.h` companion loaded by clang importer

Hybrid clang AST importer invoked by `buraaq bind`.

## Selected Design

**B primary, D optional for bulk headers**

### Declaring C functions

```buraaq
extern c:
    fn printf(fmt: c.text, ...) -> c.int
    fn malloc(size: c.usize) -> c.void_ptr

use libc.{printf, malloc}
```

### C type namespace

All C types live under `c.*` prefix:

| Buraaq | C |
|--------|---|
| `c.int` | `int` |
| `c.char` | `char` |
| `c.void_ptr` | `void*` |
| `c.text` | `const char*` |

Buraaq `text` is NOT silently compatible with `c.text`—explicit conversion required:

```buraaq
c.printf("%s\n", text.to_c())   // borrows as NUL-terminated for call duration
```

### Exporting Buraaq to C

```buraaq
#[export(c)]
pub fn buraaq_version() -> c.int:
    return 1
```

Generates symbol `buraaq_version` with C ABI in object file; `buraaq pkg export` emits `.h`.

### Bulk import

```bash
buraaq bind curl/curl.h --out src/curl/curl.bq
```

Clang parses header; generates `extern c` module (checked into repo).

## Advantages

- Small FFI is copy-paste simple.
- Type namespace prevents accidental C/Buraaq confusion.
- No preprocessor in language grammar.

## Disadvantages

- Large APIs need `buraaq bind` step.
- Manual declarations can drift from headers.

## Performance Implications

- C calls use System V / Windows ABI—zero overhead vs C.
- `text.to_c()` may stack-allocate temporary buffer; documented lifetime rule: valid only for synchronous call unless pinned.

## Implementation Implications

- LLVM codegen uses platform C calling convention.
- `buraaq bind` ships as subcommand using libclang.
- GFA treats C pointers as `unsafe` unless wrapped in safe newtypes.

## Future Compatibility

- `extern c++` limited to C++ functions with C linkage only in v1.
- Objective-C interop via platform bindings, not language core.
