# Buraaq editor support

All editors talk to the same Language Server: **`buraaq lsp-server`** (stdio).  
Shared TextMate grammar: [`shared/buraaq.tmLanguage.json`](shared/buraaq.tmLanguage.json).

| Editor | Path | What you get |
|--------|------|----------------|
| **VS Code / Cursor** | [`vscode/`](vscode/) | Full extension: grammar, snippets, LSP client |
| **Sublime Text** | [`sublime/`](sublime/) | TextMate + LSP package settings |
| **Vim** | [`vim/`](vim/) | filetype + syntax |
| **Neovim** | [`neovim/`](neovim/) | lspconfig stub |
| **Helix** | [`helix/`](helix/) | `languages.toml` fragment |
| **Zed** | [`zed/`](zed/) | extension.toml |
| **JetBrains** | [`jetbrains/`](jetbrains/) | TextMate bundle + LSP4IJ notes |

## Prerequisite

```powershell
# Windows (after winget publishes) or:
.\install.ps1
buraaq doctor
```

`buraaq` must be on `PATH` for LSP features. Syntax highlighting works from the grammar alone.

## VS Code / Cursor (recommended)

```powershell
cd editors\vscode
npm install
npm run compile
```

Then **Extensions: Install from Location…** → select `editors/vscode`, or press **F5** for Extension Development Host.

Package a `.vsix`:

```powershell
npm run package
```

## Cursor

Cursor loads VS Code extensions. Install the same VSIX or open `editors/vscode` as a local extension.
