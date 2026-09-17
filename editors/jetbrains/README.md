# Buraaq in JetBrains IDEs (IntelliJ, Rider, CLion, WebStorm, …)

## Syntax highlighting

1. Settings → Editor → TextMate Bundles → **+**
2. Select `editors/jetbrains/Buraaq.tmbundle`
3. Open a `.bq` file — scope `source.buraaq`

## Language Server

Install the **LSP4IJ** plugin (or **IntelliJ LSP**), then add a server:

| Field | Value |
|-------|--------|
| Command | `buraaq` |
| Arguments | `lsp-server` |
| File type | `*.bq` |

Requires `buraaq` on PATH (`winget install buraaq` once published, or `install.ps1`).
