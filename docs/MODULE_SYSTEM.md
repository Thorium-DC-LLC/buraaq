# Buraaq Module and Package System

Buraaq modules map directly to the file system. Packages are versioned units with integrated dependency management. No separate build generator is required for standard workflows.

---

## 1. Terminology

| Term | Definition |
|------|------------|
| **Module** | One `.bq` source file = one module |
| **Package** | Collection of modules + `buraaq.pkg` manifest (≈ crate) |
| **Workspace** | Multiple packages developed together (v0.9) |
| **Registry** | Package index at `packages.buraaq.dev` (v1.0 GA) |

---

## 2. Module Paths

File path under `src/` determines module path:

| File | Module path |
|------|-------------|
| `src/main.bq` | `main` |
| `src/http/server.bq` | `http.server` |
| `src/lib.bq` | `lib` |

Optional explicit declaration (must match path):

```buraaq
module http.server
```

Mismatch is compile error.

---

## 3. Visibility

Default: **private** to module.

```buraaq
pub fn listen(...) -> ...     # exported from module
pub struct Server              # exported type
struct Internal                # module-private
```

Import visibility:

```buraaq
use http.server.{Server, listen}   # only pub items accessible
```

---

## 4. Imports

### 4.1 Syntax

```buraaq
use std.io.println                    # single item
use std.collections.{List, Map}       # multiple items
use std.net as net                    # namespace alias
use std.io.*                          # glob (discouraged; lint warn)
```

### 4.2 Resolution order

1. Current package modules
2. Dependency packages (from lockfile)
3. Standard library (`buraaq-std`) — always available unless `#![no_std]`

### 4.3 Cyclic imports

Forbidden. Compiler error with cycle path:

```
error[E0108]: cyclic module dependency
  main → http.server → http.router → http.server
```

Break cycles by extracting shared types to `http.types` module.

---

## 5. Package Manifest (`buraaq.pkg`)

```toml
[package]
name = "myserver"
version = "0.1.0"
entry = "main"              # entry module name (under src/)
authors = ["you@example.com"]
license = "MIT"

[dependencies]
buraaq-std = "1.0"
json = { version = "^0.4", registry = "https://packages.buraaq.dev" }
vendor = { path = "../vendor-lib" }
gitdep = { git = "https://github.com/org/lib.bq.git", rev = "abc123" }

[dev-dependencies]
test-utils = "0.2"

[build]
opt = "release"             # default for buraaq build
targets = ["x86_64-unknown-linux-gnu"]

[lints]
unused_result = "warn"

[features]
default = ["async"]
async = ["buraaq-std/async"]
```

---

## 6. Lockfile (`buraaq.lock`)

Content-addressed resolution:

```toml
[[package]]
name = "json"
version = "0.4.2"
source = "registry+https://packages.buraaq.dev"
checksum = "sha256:..."
dependencies = ["buraaq-std@1.0.0"]
```

Committed to VCS for reproducible builds.

---

## 7. Standard Library Layout

`buraaq-std` is a package depended implicitly:

```
stdlib/
  src/
    io.bq
    fs.bq
    net.bq
    thread.bq
    sync.bq
    async.bq
    collections/list.bq
    ...
```

Modules imported as `std.io`, `std.net`, etc.

`#![no_std]` packages must not import `std.*` except `core.*` subset.

---

## 8. Package Commands

| Command | Description |
|---------|-------------|
| `buraaq init myapp` | Create `buraaq.pkg` + `src/main.bq` |
| `buraaq get github.com/org/pkg` | Add dependency + update lock |
| `buraaq build` | Build entry module to `target/debug/` or `target/release/` |
| `buraaq run` | Build + run binary |
| `buraaq test` | Compile and run `#[test]` functions + `tests/` modules |
| `buraaq fmt` | Format package sources |
| `buraaq doc` | Generate HTML docs to `target/doc/` |
| `buraaq publish` | Upload to registry (auth required, v1.0) |

---

## 9. Build Outputs

```
target/
  debug/
    myserver              # binary
    libjson.a             # static lib if library package
  release/
    myserver
  doc/
  .cache/                 # incremental artifacts (see COMPILER_ARCHITECTURE)
```

Library packages set `type = "lib"` in manifest; produce `.a` + metadata for dependents.

---

## 10. Conditional Compilation

```buraaq
#[cfg(linux)]
fn open_epoll():
    ...

#[cfg(feature = "async")]
use std.async
```

`buraaq build --features "async,extra"` enables features.

---

## 11. Testing Modules

### 11.1 Inline tests

```buraaq
#[test]
fn adds_numbers():
    assert add(2, 2) == 4
```

### 11.2 Integration tests

`tests/integration_test.bq` — separate module with `use myserver.*`.

Test runner compiles with `--cfg test` and links test harness from `buraaq_test`.

---

## 12. Documentation

`buraaq doc` generates HTML from doc comments:

```buraaq
/// Starts listening on the given address.
/// Returns error if port in use.
fn listen(addr: text) throws NetError -> Server:
    ...
```

---

## 13. Versioning and Compatibility

Semantic versioning for packages:

- **MAJOR**: breaking API changes
- **MINOR**: backward-compatible additions
- **PATCH**: fixes

Compiler version pinned in lockfile optional field `buraaq-version = ">=0.1,<0.2"`.

Language breaking changes bump compiler major; std major aligned.

---

## 14. Workspaces (v0.9)

Root `buraaq.pkg`:

```toml
[workspace]
members = ["crates/server", "crates/cli"]
```

Shared `target/` directory; unified lockfile.

---

## 15. Anti-Patterns Avoided

| Problem (C/C++/Rust) | Buraaq solution |
|----------------------|-----------------|
| Header/include guards | One module per file |
| CMake for every project | `buraaq build` |
| `mod foo;` + file path duplication | Path = module |
| Global namespace pollution | `use` imports only |
| Mystery dependency versions | Lockfile |

---

## 16. Example Project Tree

```
tiny_http/
  buraaq.pkg
  buraaq.lock
  src/
    main.bq
    handler.bq
  tests/
    handler_test.bq
  README.md
```

`main.bq`:

```buraaq
use handler.{handle, ROUTES}
use std.net.TcpListener

fn main() throws IOError:
    listener = TcpListener.bind("0.0.0.0:8080")?
    ...
```

See [LANGUAGE_SPEC.md](./LANGUAGE_SPEC.md) for full HTTP server example.

This module system delivers **near-zero boilerplate** build and dependency workflows per [LANGUAGE_PHILOSOPHY.md](./LANGUAGE_PHILOSOPHY.md).
