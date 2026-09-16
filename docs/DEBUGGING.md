# Debugging Buraaq programs

## Today

`buraaq build` produces a native executable via clang. Debug builds use `-O0`.

- **Windows:** use LLDB or Visual Studio if the binary has debug info from clang (`-g` is not yet passed by default — track as a known limitation).
- **Linux:** GDB / LLDB on the linked ELF.

Source locations in **compiler diagnostics** (not the native debugger) are the supported 1.0-quality path: multi-span errors, LSP hover, go-to-definition.

## Planned

```bash
buraaq debug
```

will launch an installed debugger with:

- DWARF (Linux/macOS) or CodeView/PDB (Windows) from `clang -g`
- pretty-printers for `text`, arrays, `Option`, `Result`

Until that lands, compile with clang directly on the emitted `.ll` if you need `-g`.

## Runtime errors

Unhandled failures should print a message. Stack symbolization is **not** complete; treat traces as best-effort.
