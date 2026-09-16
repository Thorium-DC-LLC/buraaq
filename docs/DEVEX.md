# Buraaq Developer Experience

One executable — **`buraaq`** — handles the entire workflow. No CMake, no separate package manager, no linker flag archaeology.

## Quick start

```bash
buraaq new myapp
cd myapp
buraaq run
buraaq test
```

## Project layout (convention over configuration)

```
myapp/
  buraaq.pkg          # package manifest (only config file needed)
  buraaq.lock         # reproducible dependency lock
  src/
    main.bq           # entry module (default name: main)
  tests/
    smoke.bq          # built-in test blocks
  benches/
  target/
    debug/            # build outputs
    release/
    doc/              # generated HTML docs
    .buraaq/cache/    # registry + offline cache
```

No `Buraaq.toml` unless you outgrow defaults — **`buraaq.pkg`** is the single manifest.

## Commands

| Command | Description |
|---------|-------------|
| `buraaq new NAME` | Scaffold project |
| `buraaq build [--release]` | Compile → `target/debug/NAME` |
| `buraaq run [--release]` | Build + execute |
| `buraaq test` | Run `test "…" { expect … }` |
| `buraaq bench` | Discover benchmarks in `benches/` |
| `buraaq add postgres [^0.1]` | Add dependency + lock |
| `buraaq remove postgres` | Remove dependency |
| `buraaq format` | Official formatter (4 spaces, LF) |
| `buraaq check` | Parse + typecheck all modules |
| `buraaq doc` | HTML API docs with search |

Legacy single-file mode still works: `buraaq build app.bq`.

## Testing

```buraaq
test "addition works" {
    expect 2 + 3 == 5
}
```

Place tests in `tests/` or inline in `src/`. The runner evaluates `expect` for constant expressions (v1); function calls coming in v0.8.

## Benchmarks

```buraaq
bench "hash map insertion" {
    # body — timed execution in v0.8
}
```

## Dependencies

```toml
# buraaq.pkg
[dependencies]
postgres = "^0.1"
vendor = { path = "../lib" }
gitlib = { git = "https://github.com/org/lib.bq.git", rev = "abc123" }
```

Resolution writes **`buraaq.lock`** with checksums for reproducible builds. Registry packages cache under `target/.buraaq/cache/`.

## Build system

Automatic:

- Source discovery under `src/`
- Module graph from `use` statements
- Incremental rebuild via content hashes (`target/.buraaq/build.json`)
- Native runtime linking (no manual `-lpthread`)
- Target triple detection

## Formatting

One official style — no `.editorconfig` wars:

- 4-space indent
- LF endings
- Trim trailing whitespace

```bash
buraaq format
```

## Documentation

```bash
buraaq doc
open target/doc/index.html
```

Searchable HTML generated from `pub` items in each module.

## vs C/C++ toolchain fragmentation

| Task | C/C++ typical | Buraaq |
|------|---------------|--------|
| New project | CMake + vcpkg/conan | `buraaq new` |
| Dependencies | Multiple tools | `buraaq add` |
| Build | cmake + ninja + flags | `buraaq build` |
| Test | gtest + cmake | `buraaq test` |
| Format | clang-format debates | `buraaq format` |
| Docs | Doxygen + setup | `buraaq doc` |

## See also

- [MODULE_SYSTEM.md](./MODULE_SYSTEM.md)
- [COMPILER_ARCHITECTURE.md](./COMPILER_ARCHITECTURE.md)
