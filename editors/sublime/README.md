# Buraaq (Sublime Text)

Copy this folder to Sublime Packages, or symlink:

```text
# Windows
%APPDATA%\Sublime Text\Packages\Buraaq

# macOS / Linux
~/Library/Application Support/Sublime Text/Packages/Buraaq
~/.config/sublime-text/Packages/Buraaq
```

Requires `buraaq` on PATH for LSP (via [LSP](https://packagecontrol.io/packages/LSP) package):

`Preferences → Package Settings → LSP → Settings`:

```json
{
  "clients": {
    "buraaq": {
      "enabled": true,
      "command": ["buraaq", "lsp-server"],
      "selector": "source.buraaq"
    }
  }
}
```
