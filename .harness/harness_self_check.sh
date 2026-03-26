#!/usr/bin/env bash
# harness_self_check.sh — Verify the harness itself satisfies its own requirements.
#
# Pattern: self-critique-evaluator-loop
# Requirements: Self-conformance (harness gates itself before running)
#
# These are STRUCTURAL checks only — no compilation, no test execution.
# They verify that the harness infrastructure is correctly configured.
# This is Phase 0 of harness_full.sh and must complete in < 5 seconds.
#
# Usage: .harness/harness_self_check.sh

set -uo pipefail

CONTRACT=".harness/contract.json"
ERRORS=0
CHECKS=0

check() { CHECKS=$((CHECKS + 1)); }
pass()  { echo "  OK: $1"; }
fail()  { echo "  FAIL: $1" >&2; ERRORS=$((ERRORS + 1)); }

echo "Self-conformance checks:"

# ── SC-1: Contract exists and is valid JSON ───────────────────────────────────
check
if [ -f "$CONTRACT" ]; then
  if jq . "$CONTRACT" > /dev/null 2>&1; then
    pass "SC-1: contract.json exists and is valid JSON"
  else
    fail "SC-1: contract.json is not valid JSON"
  fi
else
  fail "SC-1: contract.json not found at $CONTRACT"
fi

# ── SC-2: Contract defines all 8 categories ──────────────────────────────────
check
sc2_ok=true
for cat in painter compositor layout behavioral trajectory widget_state notice input; do
  if ! jq -e ".categories.\"$cat\"" "$CONTRACT" > /dev/null 2>&1; then
    fail "SC-2: missing category '$cat' in contract"
    sc2_ok=false
  fi
done
[ "$sc2_ok" = true ] && pass "SC-2: all 8 categories defined"

# ── SC-3: Every category has an oracle field ──────────────────────────────────
check
sc3_ok=true
for cat in $(jq -r '.categories | keys[]' "$CONTRACT" 2>/dev/null); do
  if ! jq -e ".categories.\"$cat\".oracle" "$CONTRACT" > /dev/null 2>&1; then
    fail "SC-3: category '$cat' has no oracle field (M1 violation)"
    sc3_ok=false
  fi
done
[ "$sc3_ok" = true ] && pass "SC-3: all categories have oracle relationship defined"

# ── SC-4: C++ generator source exists (F3: oracles from execution) ────────────
check
if [ -f "tests/golden/gen/gen_golden.cpp" ] && [ -f "tests/golden/gen/Makefile" ]; then
  pass "SC-4: C++ golden generator source exists"
else
  fail "SC-4: C++ generator missing (F3 violation — oracles must be from execution)"
fi

# ── SC-5: No #[ignore] on golden tests (V8: no suppression) ──────────────────
check
if grep -rn '#\[ignore\]' tests/golden/*.rs > /dev/null 2>&1; then
  fail "SC-5: golden tests contain #[ignore] attributes (V8 violation)"
  grep -rn '#\[ignore\]' tests/golden/*.rs >&2
else
  pass "SC-5: no #[ignore] on golden tests"
fi

# ── SC-6: No tolerance overrides without diverged=true ────────────────────────
check
bad_overrides=$(jq -r '.cases[] | select(.tolerance_override != null and (.diverged // false) != true) | .name' "$CONTRACT" 2>/dev/null || true)
if [ -n "$bad_overrides" ]; then
  fail "SC-6: tolerance_override without diverged=true: $bad_overrides"
else
  pass "SC-6: all tolerance overrides have diverged=true with reason"
fi

# ── SC-7: No duplicate test names in contract (F5: uniqueness) ────────────────
check
dupes=$(jq -r '.cases[].name' "$CONTRACT" 2>/dev/null | sort | uniq -d)
if [ -n "$dupes" ]; then
  fail "SC-7: duplicate test names in contract: $dupes"
else
  pass "SC-7: all test names unique"
fi

# ── SC-8: Harness scripts exist and are executable ────────────────────────────
check
sc8_ok=true
for script in \
  .harness/harness_check_contract.sh \
  .harness/harness_correspondence.sh \
  .harness/harness_divergence_run.sh \
  .harness/harness_classify.sh \
  .harness/harness_regression_check.sh \
  .harness/harness_full.sh; do
  if [ ! -x "$script" ]; then
    fail "SC-8: $script missing or not executable"
    sc8_ok=false
  fi
done
[ "$sc8_ok" = true ] && pass "SC-8: all harness scripts present and executable"

# ── SC-9: Golden data directory exists ────────────────────────────────────────
check
if [ -d "tests/golden/data" ]; then
  golden_count=$(find tests/golden/data -name '*.golden' | wc -l | tr -d ' ')
  if [ "$golden_count" -gt 0 ]; then
    pass "SC-9: golden data directory exists with $golden_count files"
  else
    fail "SC-9: golden data directory exists but contains no .golden files"
  fi
else
  fail "SC-9: golden data directory not found (run generator first)"
fi

# ── SC-10: Coverage scripts exist ─────────────────────────────────────────────
check
if [ -x "scripts/coverage.sh" ]; then
  pass "SC-10: coverage.sh exists"
else
  fail "SC-10: coverage.sh missing or not executable (V7, M3 violation)"
fi

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
if [ $ERRORS -gt 0 ]; then
  echo "SELF-CONFORMANCE FAILED: $ERRORS of $CHECKS checks failed." >&2
  exit 1
fi

echo "SELF-CONFORMANCE PASSED: $CHECKS of $CHECKS checks passed."
