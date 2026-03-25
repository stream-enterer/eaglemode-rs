#!/usr/bin/env bash
# harness_divergence_run.sh — Run golden tests with structured JSONL output.
#
# Pattern: rich-feedback-loops, structured-output-specification
# Requirements: V2 (exhaustive comparison), V3 (diagnostics), M2 (crash/hang detection)
#
# Runs all golden tests with MEASURE_DIVERGENCE=1 and captures output to JSONL.
# Detects crashes (signals) and hangs (timeout). Classifies exit conditions.
#
# Usage: .harness/harness_divergence_run.sh OUTDIR

set -uo pipefail

OUTDIR="${1:?Usage: harness_divergence_run.sh OUTDIR}"
TIMEOUT="${HARNESS_TIMEOUT:-120}"

mkdir -p "$OUTDIR"

# ── Run golden tests with divergence logging ──────────────────────────────────

set +e
timeout "$TIMEOUT" env \
  DIVERGENCE_LOG="$OUTDIR/divergence.jsonl" \
  MEASURE_DIVERGENCE=1 \
  cargo test --test golden -- --test-threads=1 \
  > "$OUTDIR/stdout.log" 2> "$OUTDIR/stderr.log"
exit_code=$?
set -e

# ── Classify exit condition ───────────────────────────────────────────────────

if [ $exit_code -eq 0 ]; then
  echo '{"event":"CLEAN_EXIT","exit_code":0}' >> "$OUTDIR/divergence.jsonl"
elif [ $exit_code -eq 124 ]; then
  echo "{\"event\":\"HANG\",\"timeout_seconds\":$TIMEOUT}" >> "$OUTDIR/divergence.jsonl"
  echo "HANG: golden tests exceeded ${TIMEOUT}s timeout" >&2
elif [ $exit_code -gt 128 ]; then
  signal=$((exit_code - 128))
  echo "{\"event\":\"CRASH\",\"signal\":$signal,\"exit_code\":$exit_code}" >> "$OUTDIR/divergence.jsonl"
  echo "CRASH: golden tests killed by signal $signal" >&2
else
  echo "{\"event\":\"TEST_FAILURE\",\"exit_code\":$exit_code}" >> "$OUTDIR/divergence.jsonl"
fi

# ── Report ────────────────────────────────────────────────────────────────────

if [ -f "$OUTDIR/divergence.jsonl" ]; then
  test_count=$(grep -c '"test":' "$OUTDIR/divergence.jsonl" 2>/dev/null || echo 0)
  pass_count=$(grep -c '"pass":true' "$OUTDIR/divergence.jsonl" 2>/dev/null || echo 0)
  fail_count=$(grep -c '"pass":false' "$OUTDIR/divergence.jsonl" 2>/dev/null || echo 0)
  echo "Divergence run: $test_count tests reported, $pass_count pass, $fail_count fail (exit=$exit_code)"
else
  echo "WARNING: No divergence.jsonl produced" >&2
fi

exit $exit_code
