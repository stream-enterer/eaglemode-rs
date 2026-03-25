#!/usr/bin/env bash
# harness_regression_check.sh — Compare current vs previous classification for regressions.
#
# Pattern: cross-cycle-consensus-relay
# Requirements: V4 (regression detection), M8 (regression as quality metric)
#
# A regression is: a test that was "pass" in the previous run but is now "fail".
# A drift is: a suspicious case whose metrics worsened.
#
# Usage: .harness/harness_regression_check.sh PREV_JSONL CURR_JSONL
#   Reads the raw divergence JSONL from both runs and compares per-test results.
#   Output: text report to stdout, exits non-zero if regressions found.

set -euo pipefail

PREV="${1:?Usage: harness_regression_check.sh PREV_JSONL CURR_JSONL}"
CURR="${2:?Usage: harness_regression_check.sh PREV_JSONL CURR_JSONL}"

[ -f "$PREV" ] || { echo "No previous run found at $PREV — skipping regression check." ; exit 0; }
[ -f "$CURR" ] || { echo "Error: current JSONL not found at $CURR" >&2; exit 1; }

REGRESSIONS=0
IMPROVEMENTS=0
NEW_CASES=0

# Extract test records from both (skip event records)
prev_tests=$(grep '"test":' "$PREV" 2>/dev/null || true)
curr_tests=$(grep '"test":' "$CURR" 2>/dev/null || true)

# Get all test names from current run
echo "$curr_tests" | jq -r '.test' 2>/dev/null | sort -u | while IFS= read -r test_name; do
  [ -z "$test_name" ] && continue

  prev_rec=$(echo "$prev_tests" | jq -c "select(.test == \"$test_name\")" 2>/dev/null | head -1 || true)
  curr_rec=$(echo "$curr_tests" | jq -c "select(.test == \"$test_name\")" 2>/dev/null | head -1 || true)

  if [ -z "$prev_rec" ]; then
    echo "NEW: $test_name (no previous data)"
    NEW_CASES=$((NEW_CASES + 1))
    continue
  fi

  prev_pass=$(echo "$prev_rec" | jq -r '.pass')
  curr_pass=$(echo "$curr_rec" | jq -r '.pass')

  if [ "$prev_pass" = "true" ] && [ "$curr_pass" = "false" ]; then
    prev_pct=$(echo "$prev_rec" | jq -r '.pct // 0')
    curr_pct=$(echo "$curr_rec" | jq -r '.pct // 0')
    echo "REGRESSION: $test_name (was pass pct=$prev_pct, now fail pct=$curr_pct)"
    REGRESSIONS=$((REGRESSIONS + 1))
  elif [ "$prev_pass" = "false" ] && [ "$curr_pass" = "true" ]; then
    echo "IMPROVED: $test_name (was fail, now pass)"
    IMPROVEMENTS=$((IMPROVEMENTS + 1))
  fi
done

echo ""
echo "Regression check: $REGRESSIONS regression(s), $IMPROVEMENTS improvement(s), $NEW_CASES new case(s)."

if [ "$REGRESSIONS" -gt 0 ]; then
  exit 1
fi
