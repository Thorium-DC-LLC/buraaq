#!/usr/bin/env bash
# Record real elapsed fuzz time. Never invent a 7-day result.
# Usage: ./fuzz_duration.sh [seconds]
set -euo pipefail
SECONDS_REQ="${1:-60}"
COMPILER="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$COMPILER/fuzz-evidence"
mkdir -p "$OUT_DIR"
started="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
started_epoch="$(date +%s)"
deadline=$((started_epoch + SECONDS_REQ))
iters=0
cd "$COMPILER"
while [ "$(date +%s)" -lt "$deadline" ]; do
  cargo test --workspace fuzz_ -- --nocapture
  iters=$((iters + 1))
done
ended="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
ended_epoch="$(date +%s)"
elapsed=$((ended_epoch - started_epoch))
seven_days=604800
if [ "$elapsed" -ge "$seven_days" ]; then
  seven="true"
else
  seven="false"
fi
report="BURAAQ FUZZ DURATION EVIDENCE
started_utc=$started
ended_utc=$ended
elapsed_sec=$elapsed
requested_sec=$SECONDS_REQ
iterations=$iters
seven_days_elapsed=$seven"
path="$OUT_DIR/elapsed-$(date +%Y%m%d-%H%M%S).txt"
printf '%s\n' "$report" > "$path"
printf '%s\n' "$report"
echo "wrote $path"
if [ "$seven" != "true" ]; then
  echo "Gate D remains FAIL: 7 days have not elapsed."
fi
