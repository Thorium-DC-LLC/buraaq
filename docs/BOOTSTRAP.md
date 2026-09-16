# Buraaq bootstrap track (M3–M11)

Rust `compiler/` is the **host** until a guest-built compiler rebuilds `compiler-buraaq` (including `llvm.bq`). `compiler-buraaq/` is the **guest** written in Buraaq. A milestone is **PASS** only when a test in this repo fails if the guest drifts.

This is dogfooding: each milestone is a Buraaq program the Rust compiler must compile, then the guest must match the host on a frozen golden.

| Milestone | Guest in `compiler-buraaq/` | Evidence | Rust still required? |
|-----------|-----------------------------|----------|----------------------|
| **M3** Lexer | `src/lexer.bq` | `bootstrap_m3` — token kinds match `buraaq_lexer` on `golden/sample.bq` | Yes (host compiles the lexer) |
| **M4** Parser | `src/parser.bq` | `bootstrap_m4` — AST event stream matches Rust `Parser` on the same golden | Yes |
| **M5** Names | `src/names.bq` | `bootstrap_m5_names_match_rust` — fn/param/local/builtin/fnref match the host AST walk | Yes |
| **M6** MIR subset | `src/mir.bq` | `bootstrap_m6_mir_subset_matches_rust` — icmp/add/const/call/ret match the host walk | Yes |
| **M7** LLVM text | `src/llvm.bq` | `bootstrap_m7_llvm_links_and_prints_five` — guest `.ll` clang-links; stdout is `5` | Yes (clang, not rustc) |
| **M8** Driver | `llvm build` in `src/main.bq` | `bootstrap_m8_guest_build_prints_five` — guest writes `.ll` and shells clang; stdout is `5` | rustc not used for that file |
| **M9** Self-host (lexer/parser) | guest LLVM compiles `lexer.bq` / `parser.bq` | `bootstrap_m9_*` | rustc still builds `compiler-buraaq` (including `llvm.bq`) |
| **M10** Install | `dist/` + LLVM sidecar | `bootstrap_m10_clang_sidecar_lookup` | No (to *use* Buraaq) |
| **M11** Guest compiles `llvm.bq` | unbounded ABI tape, declare-on-bind, string weave, IR newlines | `bootstrap_m11_guest_llvm_compiles_llvm_bq` | Yes (host still rustc-built) |

## What M3–M10 already proved

The lexer is not a sketch. `bootstrap_m3` compiles `compiler-buraaq` with the host, runs it on `golden/sample.bq`, and asserts stdout kinds equal Rust `TokenKind::golden_name()`.

M4–M6 dump events from the same golden. M7/M8 lower `print(add(2,3))` to LLVM, link with `buraaq_rt.c`, and run: stdout is `5`. M8 is the Buraaq driver invoking clang; the host is only used to *build* that driver.

**M9** is the guest LLVM emitter compiling `lexer.bq` and `parser.bq` (plus a thin `main`). Those guest-built programs still pass the M3 kinds golden and a non-empty M4-style dump.

**M11** is that same guest compiling `llvm.bq` itself: dump_llvm prints a module with `define`. That is not “rustc off”: rustc still builds the host CLI. Full self-host is a later rebuild of `compiler-buraaq` by a guest-built compiler.

**M10** is install/doctor finding clang via PATH, `BURAAQ_CLANG`, or `%LOCALAPPDATA%\buraaq\llvm` (not a 400MB LLVM tree in git). User install copies `dist/buraaq` and does not invoke Cargo.

## Rules

1. **Host wins until golden matches.** Do not delete `compiler/` because the guest exists.
2. **Subset first.** M4–M8 cover the golden (and later `lexer.bq` itself). Not the whole language.
3. **No fake self-host.** M9 is not “we have `.bq` files.” It is “a Buraaq-built LLVM path compiles the lexer/parser and goldens still pass.”
4. **LLVM is the remaining native dependency after M8.** Pack it (M10). Do not pack Rust.

## M10 layout (install)

```
%LOCALAPPDATA%\buraaq\   or  ~/.local/share/buraaq/
  bin/buraaq.exe
  llvm/bin/clang.exe     (fetched or copied, not committed)
```

`buraaq doctor` prints host triple, sysroot, clang path, bootstrap status, and whether this tree still needs Cargo to *build* the compiler.

## Not this track

Gate D (7-day fuzz), registry, DAP, channels. Those stay on the Rust host and the 1.0 gate list.
