# Buraaq for Neovim

```lua
-- init.lua
vim.opt.runtimepath:append("~/Desktop/buraaq/editors/vim")
require("buraaq").setup()  -- after adding editors/neovim to package.path / lua path
```

With [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig), copy `buraaq.lua` into your config and call `setup()` so `buraaq lsp-server` attaches to `.bq` buffers.

Requires `buraaq` on `PATH`.
