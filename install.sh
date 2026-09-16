#!/usr/bin/env bash
# Install the Buraaq compiler onto PATH (POSIX).
# One step. Rust is not required. Prefers packaged dist/buraaq.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${1:-$HOME/.local/bin}"
mkdir -p "$PREFIX"
SRC=""
if [ -x "$ROOT/dist/buraaq" ]; then
  SRC="$ROOT/dist/buraaq"
elif [ -x "$ROOT/compiler/target/release/buraaq" ]; then
  SRC="$ROOT/compiler/target/release/buraaq"
fi
if [ -z "$SRC" ]; then
  echo "Packaged compiler missing (dist/buraaq). Rust is not required to use Buraaq." >&2
  echo "Get a packaged tree, or on a packager machine run: ./scripts/pack-dist.sh" >&2
  exit 1
fi
echo "Using packaged compiler $SRC (Rust is not required)"
cp "$SRC" "$PREFIX/buraaq"
chmod +x "$PREFIX/buraaq"
if [ -x "$ROOT/scripts/ensure-llvm.sh" ]; then
  "$ROOT/scripts/ensure-llvm.sh" || true
fi
echo "Installed $PREFIX/buraaq"
echo "Add $PREFIX to PATH if buraaq is not found."
echo "Then run: buraaq doctor"
echo "Rust was not required."
