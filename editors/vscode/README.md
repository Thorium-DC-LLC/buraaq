# Buraaq for VS Code / Cursor

**Publisher:** [ThoriumDC](https://marketplace.visualstudio.com/publishers/ThoriumDC) · **Extension id:** `ThoriumDC.buraaq`

Language support for `.bq` files via **`buraaq lsp-server`**, plus optional **one-click compiler install**.

## Features

- Syntax highlighting (TextMate)
- Snippets (`main`, `fn`, `keel`, `lumen`, …)
- Diagnostics, completion, hover, go-to-definition (via LSP)
- Format / rename / symbols when the server supports them
- **Install Compiler Toolchain** — downloads the GitHub Release zip (`buraaq.exe` + `sysroot`) into extension storage
- **Install LLVM (clang)** — winget `LLVM.LLVM` when clang is missing (Windows)
- Integrated terminals get the managed toolchain on `PATH` for this window

## Install

From the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=ThoriumDC.buraaq):

```text
ext install ThoriumDC.buraaq
```

Or from the GitHub Release VSIX.

## First open

If `buraaq` is not on `PATH`, the extension offers to install the official Windows x64 toolchain. You can also run:

- **Buraaq: Install Compiler Toolchain**
- **Buraaq: Install LLVM (clang)**

Building still needs **clang**; the Release zip is the Buraaq CLI + stdlib, not a full LLVM tree.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `buraaq.lsp.path` | `buraaq` | Path to the Buraaq CLI |
| `buraaq.lsp.trace` | `off` | `off` / `messages` / `verbose` |
| `buraaq.toolchain.autoInstall` | `true` | Prompt to download toolchain when missing |

Publish: [PUBLISH.md](PUBLISH.md).
