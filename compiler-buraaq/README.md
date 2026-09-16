Compiler **written in Buraaq**. Users install Buraaq and never open this folder. Track: [docs/BOOTSTRAP.md](../docs/BOOTSTRAP.md).

| Milestone | Status | Evidence |
|-----------|--------|----------|
| **M3** Lexer | **PASS** | `bootstrap_m3` — kinds match Rust on `golden/sample.bq` |
| **M4** Parser | **PASS** | `bootstrap_m4` — AST events match Rust `Parser` |
| **M5** Names | **PASS** | `bootstrap_m5_names_match_rust` |
| **M6** MIR subset | **PASS** | `bootstrap_m6_mir_subset_matches_rust` |
| **M7** LLVM text | **PASS** | guest `.ll` clang-links; golden prints `5` |
| **M8** Driver | **PASS** | guest `build` shells clang; golden prints `5` |
| **M9** Guest lexer/parser | **PASS** | `bootstrap_m9_m10` — guest LLVM compiles `lexer.bq`/`parser.bq`; lexer kinds match M3; parser dump has `fn`/`add` |
| **M10** Install sidecar | **PASS** | `clang_path` + install prefers `dist/` + ensure-llvm sidecar |

## Run

```bash
# M3 — token kinds
buraaq run -C compiler-buraaq -- golden/sample.bq

# M4 — parse events
buraaq run -C compiler-buraaq -- parse golden/sample.bq

# M5 / M6
buraaq run -C compiler-buraaq -- names golden/sample.bq
buraaq run -C compiler-buraaq -- mir golden/sample.bq

# M7 — LLVM text on stdout
buraaq run -C compiler-buraaq -- llvm golden/sample.bq

# M8 — guest clang (paths must have no spaces)
buraaq run -C compiler-buraaq -- build golden/sample.bq out.exe compiler/runtime/buraaq_rt.c clang
```

```bash
cargo test -p buraaq_driver --test bootstrap_m3
cargo test -p buraaq_driver --test bootstrap_m4
cargo test -p buraaq_driver --test bootstrap_m5_m8
cargo test -p buraaq_driver --test bootstrap_m9_m10
```

The host CLI still builds this package (including `llvm.bq`). M9 is guest LLVM compiling the lexer and parser. Full guest rebuild of `llvm.bq` is the remaining bootstrap step — [STATUS.md](../docs/STATUS.md).
