#!/usr/bin/env bash
# Run strict golden tests and produce a JSONL baseline snapshot.
# Usage: bash scripts/strict_baseline.sh [output_file]
# Default output: target/strict_baseline.jsonl
set -euo pipefail

OUT="${1:-target/strict_baseline.jsonl}"
mkdir -p "$(dirname "$OUT")"

cd "$(dirname "$0")/.."

echo "Running strict golden tests..." >&2
STRICT_GOLDEN=1 MEASURE_DIVERGENCE=1 \
  cargo test --test golden -- --nocapture --test-threads=1 2>&1 \
  | grep '"test":' \
  | python3 -c "
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line.startswith('{'):
        continue
    try:
        d = json.loads(line)
        print(json.dumps(d, separators=(',', ':')))
    except json.JSONDecodeError:
        pass
" > "$OUT"

TOTAL=$(wc -l < "$OUT")
FAIL=$(grep -c '"pass":false' "$OUT" || true)
PASS=$((TOTAL - FAIL))
echo "Baseline written to $OUT: $TOTAL tests, $PASS pass, $FAIL fail" >&2
