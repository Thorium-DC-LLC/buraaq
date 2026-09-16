#!/usr/bin/env bash
# Run the Buraaq engineer test suite.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
MANIFEST="$ROOT/manifest.tsv"
STRICT="${1:-}"
pass=0 fail=0 warn=0

command -v buraaq >/dev/null || { echo "buraaq not on PATH"; exit 1; }

while IFS=$'\t' read -r id rel kind mode expect _notes; do
  [[ -z "$id" || "$id" == \#* ]] && continue
  path="$ROOT/$rel"
  echo ""
  echo "=== $id ($mode) ==="
  if [[ "$kind" == "single" ]]; then
    cd "$ROOT"
    if [[ "$mode" == "run" ]]; then
      out="$(buraaq run "$path" 2>&1)" || { echo "FAIL run $id"; ((fail++)) || true; continue; }
      if [[ -n "$expect" && "$out" != *"$expect"* ]]; then
        echo "FAIL missing '$expect' in: $out"; ((fail++)) || true; continue
      fi
      echo "PASS run"; ((pass++)) || true
    else
      if buraaq build "$path" >/dev/null 2>&1; then
        echo "PASS compile"; ((pass++)) || true
      elif [[ "$STRICT" == "--strict" ]]; then
        echo "FAIL compile $id"; ((fail++)) || true
      else
        echo "WARN compile (experimental)"; ((warn++)) || true
      fi
    fi
  elif [[ "$kind" == "project" ]]; then
    proj="$ROOT/$rel"
    cd "$proj"
    if [[ "$mode" == "run" ]]; then
      out="$(buraaq run 2>&1)" || { echo "FAIL run $id"; ((fail++)) || true; continue; }
      if [[ -n "$expect" && "$out" != *"$expect"* ]]; then
        echo "FAIL missing '$expect'"; ((fail++)) || true; continue
      fi
      echo "PASS run"; ((pass++)) || true
    fi
  fi
done < "$MANIFEST"

echo ""
echo "--- SUMMARY --- pass=$pass fail=$fail experimental_warn=$warn"
[[ "$fail" -eq 0 ]]
