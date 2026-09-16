#!/usr/bin/env bash
# Link Buraaq LLVM IR to a Linux executable with the already-unpacked sidecar clang.
# No rustc, no cargo.
set -euo pipefail
LLVM="$HOME/.local/share/buraaq/llvm"
TINFO="$HOME/.local/share/buraaq/libtinfo5"
if [ ! -f "$TINFO/lib/x86_64-linux-gnu/libtinfo.so.5" ] && [ ! -f "$TINFO/lib/libtinfo.so.5" ]; then
  mkdir -p "$TINFO/lib"
  if [ -f /lib/x86_64-linux-gnu/libtinfo.so.6 ]; then
    ln -sf /lib/x86_64-linux-gnu/libtinfo.so.6 "$TINFO/lib/libtinfo.so.5"
  else
    mkdir -p /tmp/tinfo5
    cd /tmp
    curl -fsSL -o libtinfo5.deb "http://security.ubuntu.com/ubuntu/pool/universe/n/ncurses/libtinfo5_6.3-2ubuntu0.3_amd64.deb"
    dpkg-deb -x libtinfo5.deb "$TINFO"
  fi
fi
export LD_LIBRARY_PATH="$TINFO/lib/x86_64-linux-gnu:$TINFO/lib:${LD_LIBRARY_PATH:-}"
export PATH="$LLVM/bin:$PATH"
if [ ! -e "$LLVM/bin/ld" ] && [ -x "$LLVM/bin/ld.lld" ]; then
  ln -sf ld.lld "$LLVM/bin/ld"
fi
"$LLVM/bin/clang" --version | head -n 1
ROOT="/mnt/c/Users/w3asi/Desktop/buraaq"
LL="$ROOT/examples/forge/target/release/forge.ll"
OUT="$ROOT/examples/forge/target/linux/forge"
mkdir -p "$(dirname "$OUT")"
"$LLVM/bin/clang" -O2 -pthread -lm -ldl \
  -target x86_64-unknown-linux-gnu \
  -fuse-ld=lld \
  "$LL" \
  "$ROOT/compiler/runtime/buraaq_rt.c" \
  "$ROOT/stdlib/runtime/buraaq_std.c" \
  "$ROOT/stdlib/runtime/buraaq_grid.c" \
  "$ROOT/stdlib/runtime/buraaq_hold.c" \
  "$ROOT/stdlib/runtime/buraaq_stream.c" \
  "$ROOT/stdlib/runtime/buraaq_runtime.c" \
  "$ROOT/stdlib/runtime/buraaq_server.c" \
  "$ROOT/stdlib/runtime/buraaq_lumen.c" \
  -o "$OUT"
file "$OUT"
echo LINK_OK
