-- Neovim: place under ~/.config/nvim/lua/buraaq.lua and require("buraaq")
-- or copy into your plugin manager.

local M = {}

function M.setup()
  vim.filetype.add({ extension = { bq = "buraaq" } })

  -- Treesitter: use TextMate via no built-in parser; rely on LSP + basic syntax from Vim runtime if linked.
  local ok, lspconfig = pcall(require, "lspconfig")
  if not ok then
    return
  end

  local configs = require("lspconfig.configs")
  if not configs.buraaq then
    configs.buraaq = {
      default_config = {
        cmd = { "buraaq", "lsp-server" },
        filetypes = { "buraaq" },
        root_dir = lspconfig.util.root_pattern("buraaq.pkg", ".git"),
        single_file_support = true,
      },
    }
  end
  lspconfig.buraaq.setup({})
end

return M
