# ADR 0006: File-System-Aligned Module System with Integrated Package Manager

## Status

Accepted

## Date

2026-09-13

## Problem

C/C++ header hell, Rust's module path confusion, and Go's GOPATH/module migration pain demonstrate that module systems must be simple, deterministic, and integrated with builds. Buraaq requires zero external build tools for standard projects and reproducible dependencies.

## Alternatives Considered

### A. CPP `#include` model

Rejected: no encapsulation; duplicate symbols; slow compiles.

### B. Java-style packages mirroring directories only

**Pros:** Simple mapping.  
**Cons:** Verbose; no crisp crate boundary.

### C. Rust `mod` tree + `Cargo.toml`

**Pros:** Proven.  
**Cons:** `mod` declarations redundant with file paths confuse beginners.

### D. Go modules

**Pros:** URL-based versioning; minimal syntax.  
**Cons:** Case sensitivity quirks; cgo complexity.

### E. File = module, directory = package, manifest = `buraaq.pkg`

One `.bq` file is a module; directory with `buraaq.pkg` is a package (crate equivalent).

## Selected Design

**Alternative E**

### Layout

```
myapp/
  buraaq.pkg          # package manifest
  src/
    main.bq           # module main → package entry
    http/
      server.bq       # module http.server
      client.bq       # module http.client
  tests/
    server_test.bq
```

### Import syntax

```buraaq
use http.server.{handle, listen}
use std.io as io
```

- Module path = dot-separated relative path from `src/` without extension.
- No `mod foo;` declarations—file presence defines modules.
- `pub` keyword exports items from a module.

### `buraaq.pkg` manifest

```toml
[package]
name = "myapp"
version = "0.1.0"
entry = "main"           # module name under src/

[dependencies]
buraaq-std = "1.0"
serde = { git = "https://github.com/example/serde-bq", tag = "v0.3" }

[targets.linux]
link = ["pthread"]
```

### Package manager commands

| Command | Action |
|---------|--------|
| `buraaq init name` | Scaffold package |
| `buraaq get dep` | Resolve and lock dependency |
| `buraaq build` | Compile entry target |
| `buraaq test` | Run `*_test.bq` modules |
| `buraaq fmt` | Format all sources |
| `buraaq doc` | Generate docs |

Lockfile: `buraaq.lock` (content-addressed; committed to VCS).

## Advantages

- Module path equals file path—no duplicate declarations.
- Single tool for build + deps.
- Works offline after first fetch.

## Disadvantages

- Large packages produce deep directory trees (same as Go/Java).
- Git-based deps require network on first get.

## Performance Implications

- Incremental compilation keyed by `(package_id, module_path, hash(source))`.
- Dependency packages compiled to `.bqo` object cache; relink only on change.

## Implementation Implications

- Compiler driver reads manifest; builds DAG of packages.
- Name resolution: `use a.b.c` maps to filesystem or cached package extract path.
- Cyclic imports forbidden at module level; detected in resolver.

## Future Compatibility

- Workspace manifests (`buraaq.pkg` with `[workspace] members`) in v0.9.
- Official registry at `packages.buraaq.dev` in v1.0 GA—not required for v0.1 (git/path deps only).
