# Buraaq for VS Code / Cursor

Language support for `.bq` files via **`buraaq lsp-server`**.

## Features

- Syntax highlighting (TextMate)
- Snippets (`main`, `fn`, `keel`, `lumen`, `flowdesk`, …)
- Diagnostics, completion, hover, go-to-definition (via LSP)
- Format / rename / symbols when the server supports them
- Command: **Buraaq: Restart Language Server**

## Install (VS Code / Cursor)

Marketplace (after publish): search **Buraaq** or:

```text
ext install ThoriumDCLLC.buraaq
```

From the GitHub Release VSIX:

```powershell
code --install-extension https://github.com/Thorium-DC-LLC/buraaq/releases/latest/download/buraaq-1.0.0.vsix
```

Publish steps: [PUBLISH.md](PUBLISH.md).


| Setting | Default | Description |
|---------|---------|-------------|
| `buraaq.lsp.path` | `buraaq` | Path to the Buraaq CLI |
| `buraaq.lsp.trace` | `off` | `off` / `messages` / `verbose` |

Publisher id for Marketplace: `ThoriumDCLLC` (publish with `vsce` when ready).
