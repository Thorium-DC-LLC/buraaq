# Buraaq for VS Code / Cursor

Language support for `.bq` files via the Buraaq Language Server.

## Features

- Syntax highlighting (TextMate grammar)
- Real-time teacher-style diagnostics
- Autocomplete (keywords + definitions)
- Hover information
- Go to definition / references / rename
- Format document
- Code actions (quick fixes from compiler suggestions)
- Signature help
- Semantic tokens
- Document & workspace symbols
- Call hierarchy

## Requirements

Build the Buraaq compiler and ensure `buraaq` is on your `PATH`:

```bash
cd compiler
cargo build --release
```

The extension launches `buraaq lsp-server` over stdio.

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `buraaq.lsp.path` | `buraaq` | Path to the Buraaq CLI |
| `buraaq.lsp.trace` | `off` | LSP trace verbosity |

## Development

```bash
cd editors/vscode
npm install
npm run compile
```

Press F5 to launch an Extension Development Host.
