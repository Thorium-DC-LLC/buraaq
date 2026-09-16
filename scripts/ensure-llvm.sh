#!/usr/bin/env bash
# Locate clang for Buraaq, or print how to install a sidecar (~/.local/share/buraaq/llvm).
set -euo pipefail
SIDECAR="${BURAAQ_HOME:-$HOME/.local/share/buraaq}/llvm/bin/clang"
candidates=("${BURAAQ_CLANG:-}" "$SIDECAR" /usr/bin/clang /usr/local/bin/clang /opt/homebrew/bin/clang)
for c in "${candidates[@]}"; do
  if [ -n "$c" ] && command -v "$c" >/dev/null 2>&1 || [ -x "$c" ]; then
    if "$c" --version >/dev/null 2>&1; then
      echo "clang: $c"
      exit 0
    fi
  fi
done
if command -v clang >/dev/null 2>&1; then
  echo "clang: $(command -v clang)"
  exit 0
fi
echo "clang not found." >&2
echo "Install LLVM (Debian: sudo apt install clang lld; macOS: brew install llvm)" >&2
echo "Or unpack clang into $HOME/.local/share/buraaq/llvm and re-run." >&2
exit 1
