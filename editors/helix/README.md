# Helix editor

Merge [languages.toml](languages.toml) into your Helix config. Syntax highlighting uses Helix’s tree if available; LSP provides diagnostics via `buraaq lsp-server`.

```bash
mkdir -p ~/.config/helix
cat editors/helix/languages.toml >> ~/.config/helix/languages.toml
```
