# Buraaq for VS Code / Cursor

**Publisher:** [ThoriumDC](https://marketplace.visualstudio.com/publishers/ThoriumDC) · **Extension id:** `ThoriumDC.buraaq`

Language support for `.bq` files via **`buraaq lsp-server`**.

## Features

- Syntax highlighting (TextMate)
- Snippets (`main`, `fn`, `keel`, `lumen`, `flowdesk`, …)
- Diagnostics, completion, hover, go-to-definition (via LSP)
- Format / rename / symbols when the server supports them
- Command: **Buraaq: Restart Language Server**

## Install

From the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=ThoriumDC.buraaq):

```text
ext install ThoriumDC.buraaq
```

Or from the GitHub Release VSIX:

```powershell
code --install-extension https://github.com/ThoriumDC/buraaq/releases/latest/download/buraaq-1.0.0.vsix
```

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `buraaq.lsp.path` | `buraaq` | Path to the Buraaq CLI |
| `buraaq.lsp.trace` | `off` | `off` / `messages` / `verbose` |

Publish: [PUBLISH.md](PUBLISH.md) — use PAT + `npx vsce publish` (do not hand-create the extension listing).
