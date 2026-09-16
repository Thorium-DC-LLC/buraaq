#!/usr/bin/env bash
# Build a Linux buraaq + Forge ship inside WSL (no sudo, no Hetzner load).
set -euo pipefail
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
if [ ! -x "$HOME/.cargo/bin/rustc" ]; then
  echo "install rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
fi
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
rustc -V
cargo -V
TINFO_LIB="$HOME/.local/tinfo5/lib/x86_64-linux-gnu"
if [ -d "$TINFO_LIB" ]; then
  export LD_LIBRARY_PATH="$TINFO_LIB:${LD_LIBRARY_PATH:-}"
fi
LLVM_DIR="$HOME/.local/share/buraaq/llvm"
if [ ! -x "$LLVM_DIR/bin/clang" ]; then
  echo "fetch portable llvm"
  mkdir -p "$HOME/.local/share/buraaq"
  cd /tmp
  URL="https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.8/clang+llvm-18.1.8-x86_64-linux-gnu-ubuntu-18.04.tar.xz"
  curl -L --fail --retry 3 -o llvm.tar.xz "$URL"
  mkdir -p "$LLVM_DIR"
  tar -xJf llvm.tar.xz -C "$LLVM_DIR" --strip-components=1
  rm -f llvm.tar.xz
fi
"$LLVM_DIR/bin/clang" --version | head -n 1
if [ -x "$LLVM_DIR/bin/ld.lld" ] && [ ! -e "$LLVM_DIR/bin/ld" ]; then
  ln -s ld.lld "$LLVM_DIR/bin/ld"
fi
export PATH="$LLVM_DIR/bin:$HOME/.local/bin:$PATH"
export CC="$LLVM_DIR/bin/clang"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$LLVM_DIR/bin/clang"
export BURAAQ_CLANG="$LLVM_DIR/bin/clang"
WIN_ROOT="/mnt/c/Users/w3asi/Desktop/buraaq"
SRC="$HOME/buraaq-src"
mkdir -p "$SRC"
echo "sync source -> $SRC"
rm -rf "$SRC/compiler" "$SRC/stdlib" "$SRC/examples"
mkdir -p "$SRC/examples"
cp -a "$WIN_ROOT/compiler" "$SRC/compiler"
rm -rf "$SRC/compiler/target"
cp -a "$WIN_ROOT/stdlib" "$SRC/stdlib"
cp -a "$WIN_ROOT/examples/forge" "$SRC/examples/forge"
rm -rf "$SRC/examples/forge/target"
echo "cargo build -p buraaq --release"
cd "$SRC/compiler"
cargo build -p buraaq --release
BQ="$SRC/compiler/target/release/buraaq"
"$BQ" --version || true
echo "pack forge"
cd "$SRC/examples/forge"
"$BQ" pack
ls -l "$SRC/examples/forge/target/ship/forge.bur"
file "$SRC/compiler/target/release/buraaq"
file "$SRC/examples/forge/target/release/forge"
mkdir -p /mnt/c/Users/w3asi/Desktop/buraaq/examples/forge/target/linux
cp -f "$SRC/compiler/target/release/buraaq" /mnt/c/Users/w3asi/Desktop/buraaq/examples/forge/target/linux/buraaq
cp -f "$SRC/examples/forge/target/ship/forge.bur" /mnt/c/Users/w3asi/Desktop/buraaq/examples/forge/target/linux/forge.bur
cp -f "$SRC/examples/forge/target/release/forge" /mnt/c/Users/w3asi/Desktop/buraaq/examples/forge/target/linux/forge
echo LINUX_PACK_OK
