#!/usr/bin/env bash
# Pack the full compiler into dist/ for user install (Rust is not required on the user machine).
# Contributors only: this script may invoke Cargo on a packager machine.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXE="$ROOT/compiler/target/release/buraaq"
if [ ! -x "$EXE" ]; then
  if ! command -v cargo >/dev/null 2>&1; then
    echo "pack-dist needs an already-built compiler or Cargo on the packager machine." >&2
    exit 1
  fi
  (cd "$ROOT/compiler" && cargo build -p buraaq --release)
fi
mkdir -p "$ROOT/dist"
cp "$EXE" "$ROOT/dist/buraaq"
chmod +x "$ROOT/dist/buraaq"
echo "Packed $ROOT/dist/buraaq"
echo "Users install with: ./install.sh  (Rust is not required)"
