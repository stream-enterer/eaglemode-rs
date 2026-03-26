# Self-Conforming Verification Harness for zuicchini/emCore Behavioral Equivalence

## Thesis

A verification harness for this project must solve a specific problem: proving that ~100 Rust source files in `zuicchini/src/emCore/` produce behavior-equivalent output to ~90 C++ headers in Eagle Mode's `include/emCore/`, across 8 distinct output domains, under tolerance regimes that vary from exact-match (behavioral, notice, input) to per-channel-pixel-tolerance (painter, compositor) to floating-point-epsilon (layout, trajectory). The harness must be self-conforming: it must verify that it itself satisfies the requirements it claims to enforce, without relying on external attestation.

The following design composes 9 of the 15 available patterns into 6 concrete components, connected by a filesystem-based state protocol. Each component is a CLI script or Rust test, runnable by both humans and Claude Code. The harness is not a framework -- it is a collection of shell scripts, JSON schemas, and Rust integration tests that compose via Unix pipes and shared file conventions.

---

## Component 1: The Contract Registry

**Patterns used:** feature-list-as-immutable-contract (1), structured-output-specification (6), filesystem-based-agent-state (8)

**Requirements satisfied:** V1 (input identity), M1 (oracle relationship defined), F1 (actual behavior not specified in contract), V8 (no suppression)

### Design

The contract registry is a single JSON file at `zuicchini/tests/harness/contract.json` that enumerates every golden test case, its category, its oracle relationship, and its tolerance parameters. This file is the single source of truth. The harness refuses to run if any golden data file on disk is not present in the contract, or if any contract entry lacks a corresponding golden file.

```json
{
  "schema_version": 1,
  "categories": {
    "painter": {
      "oracle": "pixel_comparison",
      "tolerance": { "channel_tolerance": 1, "max_failure_pct": 0.1 },
      "golden_ext": ".painter.golden",
      "comparison_fn": "compare_images"
    },
    "compositor": {
      "oracle": "pixel_comparison",
      "tolerance": { "channel_tolerance": 2, "max_failure_pct": 0.5 },
      "golden_ext": ".compositor.golden",
      "comparison_fn": "compare_images"
    },
    "layout": {
      "oracle": "rect_comparison",
      "tolerance": { "eps": 1e-9 },
      "golden_ext": ".layout.golden",
      "comparison_fn": "compare_rects"
    },
    "behavioral": {
      "oracle": "exact_match",
      "tolerance": {},
      "golden_ext": ".behavioral.golden",
      "comparison_fn": "compare_behavioral"
    },
    "trajectory": {
      "oracle": "f64_tolerance",
      "tolerance": { "tolerance": 50.0 },
      "golden_ext": ".trajectory.golden",
      "comparison_fn": "compare_trajectory"
    },
    "widget_state": {
      "oracle": "structured_exact",
      "tolerance": {},
      "golden_ext": ".widget_state.golden",
      "comparison_fn": "compare_widget_state"
    },
    "notice": {
      "oracle": "flag_translation_exact",
      "tolerance": { "mask": "0x0FFF" },
      "golden_ext": ".notice.golden",
      "comparison_fn": "compare_notices"
    },
    "input": {
      "oracle": "exact_match",
      "tolerance": {},
      "golden_ext": ".input.golden",
      "comparison_fn": "compare_input"
    }
  },
  "cases": [
    { "name": "rect_solid", "category": "painter", "rust_test": "painter_rect_solid" },
    { "name": "rect_alpha", "category": "painter", "rust_test": "painter_rect_alpha" }
  ]
}
```

The `cases` array must enumerate all 214 golden data files (42 painter + 61 compositor + 31 layout + 25 behavioral + 20 trajectory + 16 widget_state + 13 notice + 6 input). Each entry maps a golden file name to a Rust test function name.

### Immutability enforcement

The contract file is checked into git. A pre-commit check (integrated into the existing hook at `.git/hooks/pre-commit`) runs:

```bash
scripts/harness_check_contract.sh
```

This script:
1. Reads `contract.json` and extracts all `(category, name)` pairs.
2. Scans `tests/golden/data/` for all `*.golden` files.
3. Fails if any golden file has no contract entry (prevents untracked oracles).
4. Fails if any contract entry has no golden file (prevents phantom entries).
5. Fails if any tolerance parameter decreased since the last committed version (prevents tolerance relaxation without explicit justification). Tolerance can only increase via a separate `scripts/harness_relax_tolerance.sh` that requires a `--reason` flag and writes a JSONL audit entry to `tests/harness/tolerance_changes.jsonl`.

**Self-conformance check (V8):** The contract checker script includes a self-test mode (`--self-test`) that verifies it correctly rejects: (a) a contract missing an entry for a known golden file, (b) a contract with an entry for a nonexistent golden file, (c) a contract with a relaxed tolerance and no audit entry. This mode runs in CI / pre-commit.

### Tradeoff: F1 vs. practicality

F1 requires that the oracle captures actual behavior, not specified behavior. In this project the oracle IS actual behavior: the golden data is generated by executing the C++ code (`gen_golden.cpp` linked against `libemCore.so`). The contract registry documents this relationship (the `oracle` field) but does not contain the expected values itself. The golden `.golden` files are binary snapshots of C++ execution output, satisfying F3 (captured from execution, not hand-authored).

---

## Component 2: The Divergence Ledger

**Patterns used:** rich-feedback-loops (5), structured-output-specification (6), filesystem-based-agent-state (8)

**Requirements satisfied:** V2 (exhaustive comparison), V3 (thorough diagnostics), V10 (multiple output variables), F2 (suspicious values flaggable), M7 (result classification)

### Design

The existing `MEASURE_DIVERGENCE=1` JSONL output and `DIVERGENCE_LOG=<path>` mechanism in `common.rs` already produce structured per-test records:

```json
{"test":"rect_solid","tol":1,"fail":0,"total":65536,"pct":0.0000,"max_diff":0,"pass":true}
```

The divergence ledger extends this into a complete run report. A script `scripts/harness_divergence_run.sh` performs:

```bash
#!/usr/bin/env bash
set -euo pipefail
OUTDIR="$1"
mkdir -p "$OUTDIR"
DIVERGENCE_LOG="$OUTDIR/divergence.jsonl" \
  MEASURE_DIVERGENCE=1 \
  cargo test --test golden -- --test-threads=1 2>"$OUTDIR/stderr.log"

# Post-process: classify results
scripts/harness_classify.sh "$OUTDIR/divergence.jsonl" > "$OUTDIR/classification.json"
```

The classifier (`scripts/harness_classify.sh`) reads the JSONL and produces a summary JSON:

```json
{
  "run_id": "2026-03-24T14:30:00",
  "commit": "c6dbc79",
  "total_cases": 214,
  "reported_cases": 214,
  "pass": 210,
  "fail": 0,
  "suspicious": 4,
  "unreported": 0,
  "categories": {
    "painter": { "total": 42, "pass": 42, "fail": 0, "suspicious": 0 },
    "compositor": { "total": 61, "pass": 57, "fail": 0, "suspicious": 4 }
  },
  "suspicious_cases": [
    { "test": "widget_colorfield_alpha_near", "reason": "fail_pct=0.08 approaching threshold 0.5", "pct": 0.08, "threshold": 0.5 }
  ]
}
```

**Suspicious value detection (F2):** A case is "suspicious" if it passes but its failure percentage exceeds 50% of its category's `max_failure_pct` threshold, or if its `max_diff` exceeds 75% of `channel_tolerance`. This catches tests drifting toward failure before they actually fail.

**V10 (multiple output variables):** For pixel comparisons, the JSONL already reports `fail_count`, `total`, `pct`, and `max_diff` -- four independent output variables. For layout/trajectory, the classifier additionally parses test stderr for per-rect/per-step diff magnitudes when they are emitted by the comparison functions.

**Self-conformance check (V3):** The classifier itself is tested by feeding it a synthetic JSONL file containing known pass/fail/suspicious cases and verifying the output classification matches expectations. This test lives in `scripts/test_harness_classify.sh`.

**Exhaustiveness check (V2):** The classifier cross-references reported cases against the contract registry. If `reported_cases < total_cases`, it flags the missing cases as `unreported` and exits non-zero. This catches the case where a test is silently skipped (e.g., `require_golden!()` returning early because golden data is missing).

---

## Component 3: The Correspondence Auditor

**Patterns used:** spec-as-test-feedback-loop (3), deterministic-security-scanning-build-loop (2)

**Requirements satisfied:** V1 (input identity -- verifies the mapping is complete), V7 (identify uncovered code for inspection), F6 (conversion exercise), M3 (statement coverage)

### Design

The project's File and Name Correspondence rules require that every C++ header has a Rust file (or a `.no_rust_equivalent` marker), and every Rust-only file has a `.rust_only` marker. The correspondence auditor is a script `scripts/harness_correspondence.sh` that enforces this structurally:

```bash
#!/usr/bin/env bash
set -euo pipefail

CPP_DIR="$HOME/git/eaglemode-0.96.4/include/emCore"
RS_DIR="zuicchini/src/emCore"

# 1. Every .h file must have exactly one of: .rs, .no_rust_equivalent
for h in "$CPP_DIR"/*.h; do
  base=$(basename "$h" .h)
  if [ ! -f "$RS_DIR/$base.rs" ] && [ ! -f "$RS_DIR/$base.no_rust_equivalent" ]; then
    echo "MISSING: $base.h has no .rs or .no_rust_equivalent"
    exit 1
  fi
done

# 2. Every .rs file (except mod.rs) must either:
#    a) Have a matching .h file in CPP_DIR, OR
#    b) Have a .rust_only marker, OR
#    c) Have a SPLIT: comment on line 1
for rs in "$RS_DIR"/*.rs; do
  base=$(basename "$rs" .rs)
  [ "$base" = "mod" ] && continue
  if [ ! -f "$CPP_DIR/$base.h" ] && [ ! -f "$RS_DIR/$base.rust_only" ]; then
    if ! head -5 "$rs" | grep -q 'SPLIT:'; then
      echo "ORPHAN: $base.rs has no C++ header, no .rust_only, and no SPLIT: comment"
      exit 1
    fi
  fi
done

# 3. Count: report correspondence statistics
rs_count=$(ls "$RS_DIR"/*.rs | wc -l)
no_equiv_count=$(ls "$RS_DIR"/*.no_rust_equivalent 2>/dev/null | wc -l)
rust_only_count=$(ls "$RS_DIR"/*.rust_only 2>/dev/null | wc -l)
echo "Correspondence: ${rs_count} .rs files, ${no_equiv_count} .no_rust_equivalent, ${rust_only_count} .rust_only"
```

This script runs as part of `scripts/harness_full.sh` and can be integrated into pre-commit.

**Coverage integration (M3, V7):** The existing `scripts/coverage.sh` and `scripts/coverage_uncovered.sh` already produce LLVM source-based coverage data. The correspondence auditor extends this by cross-referencing coverage data against the contract registry:

```bash
# Which Rust files have zero golden test coverage?
scripts/coverage_uncovered.sh --summary | while read line; do
  file=$(echo "$line" | awk '{print $NF}')
  # Check if this file is exercised by any golden test
  if ! grep -q "$file" tests/harness/contract.json; then
    echo "UNCOVERED-BY-GOLDEN: $file"
  fi
done
```

**F6 (conversion exercise):** The auditor tracks the conversion ratio: `(rs_files - rust_only) / (cpp_headers - no_rust_equivalent)`. This metric, reported in the classification JSON, quantifies the port's structural completeness. Currently: `(100 - 5) / (cpp_total - 15)` = 95 Rust files covering C++ headers.

**Self-conformance check:** The auditor runs a self-test that creates a temporary directory with a known set of `.h`, `.rs`, `.no_rust_equivalent`, and `.rust_only` files, then verifies the auditor correctly accepts valid states and rejects invalid ones (missing `.rs` for a `.h`, orphan `.rs` without marker).

---

## Component 4: The Regression Gate

**Patterns used:** hook-based-safety-guard-rails (11), cross-cycle-consensus-relay (9), cli-first-skill-design (12)

**Requirements satisfied:** V4 (re-runnable, regression detection), M8 (regression as quality metric), V9 (stopping criterion), F4 (change-guided coverage), F8 (deployed-system bug policy)

### Design

The regression gate is the enforcement layer that prevents behavioral regressions from being committed. It operates at two levels:

**Level 1: Pre-commit gate (existing, extended)**

The existing pre-commit hook at `.git/hooks/pre-commit` already runs `cargo fmt`, `cargo clippy`, and `cargo test`. The harness extends this with a lightweight regression check:

```bash
# Added to pre-commit hook after clippy passes:
run_check "golden-contract" scripts/harness_check_contract.sh
run_check "golden-tests" cargo test --test golden --quiet
```

The golden tests already compare Rust output against C++ golden data. If any test fails, the commit is blocked. This satisfies V4 (re-runnable, regression detection).

**Level 2: Full divergence run (on-demand)**

For deeper analysis, the developer (or Claude Code) runs:

```bash
scripts/harness_full.sh target/harness/$(date +%Y%m%d_%H%M%S)
```

This orchestrates:
1. Contract validation (`harness_check_contract.sh`)
2. Correspondence audit (`harness_correspondence.sh`)
3. Full golden divergence run (`harness_divergence_run.sh`)
4. Coverage collection (`scripts/coverage.sh --text-only`)
5. Classification and regression detection (`harness_classify.sh`)
6. Regression comparison against previous run

The regression comparison reads the previous run's `classification.json` (found via a `latest` symlink at `target/harness/latest/`) and flags:
- Any test that was `pass` in the previous run but is now `fail` or `suspicious` -- this is a regression.
- Any test that was `suspicious` in the previous run and is now more suspicious (higher `pct` or `max_diff`) -- this is a drift.
- Any new test in the contract that has no previous run data -- this is a new case (acceptable).

**M8 (regression as quality metric):** The classification JSON includes a `regressions` count. This number must be 0 for the harness to exit cleanly. The harness reports it prominently:

```
HARNESS RESULT: 214 cases, 210 pass, 4 suspicious, 0 fail, 0 regressions
```

**V9 (stopping criterion):** The harness defines convergence as: zero regressions, zero failures, zero unreported cases, and suspicious count non-increasing across the last 3 runs. When these conditions hold, the port is "converged" for the covered test surface. The harness prints `CONVERGED` or `NOT CONVERGED` in its summary.

**F4 (change-guided coverage):** The harness accepts an optional `--changed-files` flag that restricts the golden test run to only tests whose contract entries reference Rust files that were modified. This uses `git diff --name-only HEAD~1` to determine changed files and filters the test list. Implemented as:

```bash
cargo test --test golden $(scripts/harness_changed_tests.sh | sed 's/^/-- /') --test-threads=1
```

**F8 (deployed-system bug policy):** When a golden test fails and the failure is determined to be a genuine C++ behavior that the Rust port intentionally diverges from (e.g., a C++ bug), the developer creates a `DIVERGED:` entry in the contract:

```json
{ "name": "some_edge_case", "category": "painter", "rust_test": "painter_some_edge_case",
  "diverged": true, "reason": "C++ has off-by-one in boundary clamp, Rust fixes it",
  "tolerance_override": { "channel_tolerance": 3, "max_failure_pct": 1.0 } }
```

The classifier distinguishes "intentional divergence" from "regression" using this field. Diverged cases are still tested but with their overridden tolerances.

**Cross-cycle relay (pattern 9):** The `target/harness/latest/classification.json` serves as the relay document. Each harness run reads the previous classification to detect regressions and writes a new one. This enables stateful tracking across Claude Code sessions without requiring in-memory state.

---

## Component 5: The Crash and Hang Detector

**Patterns used:** multi-step-analysis-pipeline-orchestration (13), cli-first-skill-design (12)

**Requirements satisfied:** M2 (crash/hang detection), V5 (failure independence assumption stated), V6 (coincident failure documentation)

### Design

The existing `cargo test` framework handles crashes via Rust's panic mechanism, which produces a stack trace and a test failure. However, hangs (infinite loops in layout algorithms, infinite recursion in panel trees) are not caught by the default test runner.

**Hang detection:** The harness wraps golden test execution with a per-test timeout:

```bash
# In harness_divergence_run.sh:
timeout 120 cargo test --test golden -- --test-threads=1
```

For finer granularity, individual test functions can use the `#[timeout]` attribute from a test helper, but the project already keeps tests fast (golden tests are pure computation, no I/O waits), so a global 120-second timeout suffices.

**Crash classification:** If `cargo test` exits with a signal (segfault from unsafe code, stack overflow), the harness captures the exit code and classifies it:

```bash
set +e
timeout 120 cargo test --test golden -- --test-threads=1 2>"$OUTDIR/stderr.log"
exit_code=$?
set -e

if [ $exit_code -eq 124 ]; then
  echo '{"event":"HANG","timeout_seconds":120}' >> "$OUTDIR/divergence.jsonl"
elif [ $exit_code -gt 128 ]; then
  signal=$((exit_code - 128))
  echo "{\"event\":\"CRASH\",\"signal\":$signal}" >> "$OUTDIR/divergence.jsonl"
elif [ $exit_code -ne 0 ]; then
  echo "{\"event\":\"TEST_FAILURE\",\"exit_code\":$exit_code}" >> "$OUTDIR/divergence.jsonl"
fi
```

**V5 (failure independence assumption):** The harness explicitly states its assumption: golden tests are independent. Each test creates its own `emImage`, `PanelTree`, or `emView` from scratch. No test depends on state from a previous test. This is verified by running golden tests in both `--test-threads=1` (sequential) and `--test-threads=4` (parallel) modes and comparing the JSONL output. If results differ, the independence assumption is violated. This check runs in `harness_full.sh`:

```bash
# Sequential run
DIVERGENCE_LOG="$OUTDIR/seq.jsonl" cargo test --test golden -- --test-threads=1
# Parallel run
DIVERGENCE_LOG="$OUTDIR/par.jsonl" cargo test --test golden -- --test-threads=4
# Compare
diff <(sort "$OUTDIR/seq.jsonl") <(sort "$OUTDIR/par.jsonl") || echo "INDEPENDENCE VIOLATION"
```

**V6 (coincident failure documentation):** When multiple tests fail in the same category (e.g., 5 compositor tests fail simultaneously), the classifier groups them and checks for a common cause. It reports:

```json
{
  "coincident_failures": [
    { "category": "compositor", "count": 5, "common_max_diff": 12,
      "note": "All failures in compositor suggest a shared compositing pipeline change" }
  ]
}
```

The heuristic: if >3 tests in a single category fail with similar `max_diff` values (within 2x of each other), they are flagged as coincident. This does not suppress them -- it annotates them for the developer.

---

## Component 6: The Self-Conformance Verifier

**Patterns used:** self-critique-evaluator-loop (7), spec-as-test-feedback-loop (3)

**Requirements satisfied:** Self-conformance (the harness verifies it satisfies its own requirements)

### Design

The self-conformance verifier is the meta-layer. It is a script `scripts/harness_self_check.sh` that verifies the harness itself satisfies the requirements it claims to enforce. It is the first thing `harness_full.sh` runs -- if self-conformance fails, no other checks execute.

The self-conformance checks are:

**SC-1 (Contract completeness -- satisfies V1, M1):**
```bash
# contract.json must exist
[ -f tests/harness/contract.json ] || fail "SC-1: contract.json missing"
# contract.json must be valid JSON
jq . tests/harness/contract.json > /dev/null || fail "SC-1: contract.json invalid JSON"
# contract must define all 8 categories
for cat in painter compositor layout behavioral trajectory widget_state notice input; do
  jq -e ".categories.$cat" tests/harness/contract.json > /dev/null || fail "SC-1: missing category $cat"
done
```

**SC-2 (Oracle relationship -- satisfies M1, F3):**
```bash
# Every category must have an oracle field
for cat in $(jq -r '.categories | keys[]' tests/harness/contract.json); do
  jq -e ".categories.$cat.oracle" tests/harness/contract.json > /dev/null \
    || fail "SC-2: category $cat has no oracle defined"
done
# The C++ generator must exist (oracles are from execution, not hand-authored)
[ -f tests/golden/gen/gen_golden.cpp ] || fail "SC-2: C++ generator missing (F3 violation)"
[ -f tests/golden/gen/Makefile ] || fail "SC-2: generator Makefile missing"
```

**SC-3 (No suppression -- satisfies V8):**
```bash
# No #[ignore] attributes on golden tests
if grep -r '#\[ignore\]' tests/golden/*.rs; then
  fail "SC-3: golden tests contain #[ignore] (V8 violation)"
fi
# No tolerance overrides without diverged=true and reason
jq -r '.cases[] | select(.tolerance_override != null and .diverged != true) | .name' \
  tests/harness/contract.json | while read name; do
  fail "SC-3: $name has tolerance_override without diverged=true"
done
```

**SC-4 (Diagnostics -- satisfies V3):**
```bash
# compare_images must emit JSONL when MEASURE_DIVERGENCE=1
# Verified by running a single known-pass test and checking stderr
output=$(MEASURE_DIVERGENCE=1 cargo test --test golden painter_rect_solid -- --test-threads=1 2>&1 >/dev/null)
echo "$output" | grep -q '"test":"rect_solid"' || fail "SC-4: MEASURE_DIVERGENCE output missing"
echo "$output" | grep -q '"max_diff"' || fail "SC-4: max_diff field missing from diagnostics"
echo "$output" | grep -q '"pct"' || fail "SC-4: pct field missing from diagnostics"
```

**SC-5 (Regression detection -- satisfies V4, M8):**
```bash
# The classifier must detect regressions
# Verified by feeding it two synthetic JSONL files where the second has a failure
# that was a pass in the first
echo '{"test":"x","tol":1,"fail":0,"total":100,"pct":0.0,"max_diff":0,"pass":true}' > /tmp/harness_sc5_prev.jsonl
echo '{"test":"x","tol":1,"fail":50,"total":100,"pct":50.0,"max_diff":128,"pass":false}' > /tmp/harness_sc5_curr.jsonl
scripts/harness_classify.sh /tmp/harness_sc5_curr.jsonl > /tmp/harness_sc5_class.json
# Regression detection requires comparing against previous
scripts/harness_regression_check.sh /tmp/harness_sc5_prev.jsonl /tmp/harness_sc5_curr.jsonl \
  | grep -q "REGRESSION" || fail "SC-5: regression detector did not flag known regression"
```

**SC-6 (Coverage identification -- satisfies V7, M3):**
```bash
# coverage.sh must exist and be executable
[ -x scripts/coverage.sh ] || fail "SC-6: coverage.sh missing or not executable"
# coverage_uncovered.sh must exist and be executable
[ -x scripts/coverage_uncovered.sh ] || fail "SC-6: coverage_uncovered.sh missing or not executable"
```

**SC-7 (Crash/hang detection -- satisfies M2):**
```bash
# Verify timeout is used in divergence run
grep -q 'timeout' scripts/harness_divergence_run.sh || fail "SC-7: no timeout in divergence runner"
# Verify crash signal detection exists
grep -q 'signal' scripts/harness_divergence_run.sh || fail "SC-7: no crash signal detection"
```

**SC-8 (Stopping criterion -- satisfies V9):**
```bash
# Verify the classifier produces a convergence verdict
scripts/harness_classify.sh /tmp/harness_sc5_prev.jsonl > /tmp/harness_sc5_class.json
jq -e '.converged' /tmp/harness_sc5_class.json > /dev/null \
  || fail "SC-8: classification output lacks 'converged' field"
```

**SC-9 (F5 -- branch path uniqueness):**
```bash
# Verify no duplicate test names in contract
dupes=$(jq -r '.cases[].name' tests/harness/contract.json | sort | uniq -d)
[ -z "$dupes" ] || fail "SC-9: duplicate test names in contract: $dupes"
```

**Execution order:** `harness_self_check.sh` runs ALL checks and reports all failures at once (not fail-fast). It exits non-zero if any check fails. The full harness refuses to proceed past self-check:

```bash
# In harness_full.sh:
scripts/harness_self_check.sh || { echo "SELF-CONFORMANCE FAILED — harness cannot run"; exit 1; }
```

---

## Orchestration: harness_full.sh

**Pattern used:** discrete-phase-separation (4), multi-step-analysis-pipeline-orchestration (13)

The full harness run is a sequential pipeline with gated phases:

```bash
#!/usr/bin/env bash
set -euo pipefail

OUTDIR="${1:-target/harness/$(date +%Y%m%d_%H%M%S)}"
mkdir -p "$OUTDIR"

# Phase 0: Self-conformance (gate: must pass or entire harness aborts)
echo "=== Phase 0: Self-conformance ==="
scripts/harness_self_check.sh 2>&1 | tee "$OUTDIR/self_check.log"

# Phase 1: Contract validation
echo "=== Phase 1: Contract validation ==="
scripts/harness_check_contract.sh 2>&1 | tee "$OUTDIR/contract_check.log"

# Phase 2: Correspondence audit
echo "=== Phase 2: Correspondence audit ==="
scripts/harness_correspondence.sh 2>&1 | tee "$OUTDIR/correspondence.log"

# Phase 3: Divergence measurement (the actual golden test run)
echo "=== Phase 3: Divergence measurement ==="
scripts/harness_divergence_run.sh "$OUTDIR"

# Phase 4: Independence verification
echo "=== Phase 4: Independence verification ==="
DIVERGENCE_LOG="$OUTDIR/par.jsonl" \
  cargo test --test golden -- --test-threads=4 2>/dev/null || true
diff <(jq -S '.test' "$OUTDIR/divergence.jsonl" | sort) \
     <(jq -S '.test' "$OUTDIR/par.jsonl" | sort) > /dev/null \
  || echo "WARNING: test independence violated" | tee -a "$OUTDIR/warnings.log"

# Phase 5: Classification and regression detection
echo "=== Phase 5: Classification ==="
scripts/harness_classify.sh "$OUTDIR/divergence.jsonl" > "$OUTDIR/classification.json"

if [ -L target/harness/latest ] && [ -f "$(readlink -f target/harness/latest)/classification.json" ]; then
  prev="$(readlink -f target/harness/latest)/classification.json"
  scripts/harness_regression_check.sh "$prev" "$OUTDIR/classification.json" \
    | tee "$OUTDIR/regression.log"
fi

# Update latest symlink
ln -sfn "$(basename "$OUTDIR")" target/harness/latest

# Phase 6: Summary
echo "=== Summary ==="
jq -r '"Cases: \(.total_cases), Pass: \(.pass), Fail: \(.fail), Suspicious: \(.suspicious), Regressions: \(.regressions // 0), Converged: \(.converged)"' \
  "$OUTDIR/classification.json"
```

Each phase produces its output in `$OUTDIR/` as files. Phases 0-2 are structural checks (fast, <1 second). Phase 3 is the heavy computation (runs all 214 golden tests). Phase 4 is the independence check (runs tests again in parallel). Phase 5 is analysis. Phase 6 is reporting.

---

## Data Flow Diagram

```
contract.json ──────┐
                     │
C++ golden data ─────┼──→ Phase 1: Contract Check ──→ contract_check.log
(214 .golden files)  │
                     │
C++ headers ─────────┼──→ Phase 2: Correspondence ──→ correspondence.log
Rust sources ────────┘

Rust test binary ────────→ Phase 3: Divergence Run ──→ divergence.jsonl
                                                         │
                          Phase 4: Independence ─────→ par.jsonl
                                                         │
                          Phase 5: Classify ─────────→ classification.json
                                    │                    │
                     previous ──────┘                    │
                     classification.json                 │
                                                         │
                          Phase 6: Summary ──────────→ stdout
```

---

## Explicit Tradeoffs

### Tradeoff 1: Single-threaded golden runs vs. speed

Phase 3 runs with `--test-threads=1` because the JSONL divergence log relies on atomic line writes and deterministic ordering. Phase 4 runs with `--test-threads=4` specifically to verify independence. This doubles the golden test time, but the tests are pure computation (no disk I/O, no network), so the total is still under 60 seconds for 214 cases.

### Tradeoff 2: F5 (branch path uniqueness) is partially satisfied

True branch path uniqueness requires that every branch in the C++ code is exercised by a distinct golden test. This project cannot fully achieve this because the golden data generator exercises functions through high-level API calls, not branch-level fuzzing. The harness approximates F5 via: (a) LLVM coverage data (M3) identifies untested branches, and (b) the correspondence auditor (V7) flags files with low golden coverage. Full branch-path uniqueness would require a differential fuzzer, which is out of scope for a single-developer project.

### Tradeoff 3: Coverage runs are on-demand, not per-commit

LLVM instrumented coverage (`scripts/coverage.sh`) is expensive (full rebuild with `-C instrument-coverage`). Running it on every commit would triple commit time. Instead, coverage runs are on-demand via `harness_full.sh` or explicit `scripts/coverage.sh` invocation. The pre-commit hook runs the fast path (clippy + golden tests without instrumentation).

### Tradeoff 4: Contract population is manual

The 214-entry contract JSON must be manually populated (or generated by a one-time script that scans `tests/golden/data/` and matches test function names). This is a one-time cost, acceptable for a single-developer project. A script `scripts/harness_gen_contract.sh` can bootstrap it:

```bash
#!/usr/bin/env bash
# Generate initial contract.json from golden data files and test function names
for golden in tests/golden/data/*/*.golden; do
  category=$(basename $(dirname "$golden"))
  name=$(basename "$golden" | sed 's/\.[^.]*\.golden$//')
  echo "  { \"name\": \"$name\", \"category\": \"$category\" },"
done
```

The `rust_test` field must then be filled in by cross-referencing with `grep -r '#\[test\]' tests/golden/*.rs`.

### Tradeoff 5: Self-conformance checks are shell, not Rust

The self-conformance verifier is written in shell, not Rust, because: (a) it must run before Rust compilation to catch structural issues, (b) shell scripts compose with existing Unix tools (jq, grep, diff), and (c) shell scripts are transparent to both humans and Claude Code (pattern 12, cli-first-skill-design). The tradeoff is that shell scripts are less rigorous than typed Rust code, but the checks are simple enough (file existence, JSON parsing, string matching) that shell suffices.

### Tradeoff 6: Widget state comparison function not yet in common.rs

The contract references `compare_widget_state` but the current `common.rs` does not have a generic widget state comparator -- each widget test module implements its own comparison logic. The harness design assumes this will be consolidated. Until then, widget_state tests are verified by their individual test assertions, and the divergence JSONL captures their pass/fail status via the test framework's exit code rather than structured JSONL output. This is a known gap that should be closed.

---

## File Manifest

All paths relative to `zuicchini/`:

| File | Purpose | New or Existing |
|------|---------|-----------------|
| `tests/harness/contract.json` | Immutable test contract (pattern 1) | New |
| `tests/harness/tolerance_changes.jsonl` | Tolerance relaxation audit log | New |
| `scripts/harness_full.sh` | Top-level orchestrator (pattern 4, 13) | New |
| `scripts/harness_self_check.sh` | Self-conformance verifier (pattern 7) | New |
| `scripts/harness_check_contract.sh` | Contract completeness checker (pattern 1) | New |
| `scripts/harness_correspondence.sh` | C++/Rust file correspondence auditor (pattern 3) | New |
| `scripts/harness_divergence_run.sh` | Golden test runner with JSONL output (pattern 5) | New |
| `scripts/harness_classify.sh` | JSONL classifier and report generator (pattern 6) | New |
| `scripts/harness_regression_check.sh` | Cross-run regression detector (pattern 9) | New |
| `scripts/harness_changed_tests.sh` | Change-guided test selector (pattern 12) | New |
| `scripts/harness_gen_contract.sh` | One-time contract bootstrapper | New |
| `scripts/coverage.sh` | LLVM coverage runner | Existing |
| `scripts/coverage_uncovered.sh` | Uncovered code reporter | Existing |
| `tests/golden/common.rs` | Comparison functions with JSONL output | Existing |
| `tests/golden/data/**/*.golden` | C++ golden reference data (214 files) | Existing |
| `tests/golden/gen/gen_golden.cpp` | C++ golden data generator | Existing |
| `.git/hooks/pre-commit` | Pre-commit gate | Existing (extended) |

---

## Requirements Traceability Matrix

| Requirement | Component(s) | Mechanism |
|-------------|-------------|-----------|
| V1 (input identity) | Contract Registry, Correspondence Auditor | contract.json enumerates all inputs; correspondence script verifies completeness |
| V2 (exhaustive comparison) | Divergence Ledger | classifier cross-references reported vs. contract cases |
| V3 (thorough diagnostics) | Divergence Ledger | JSONL per-test records with 4+ output variables |
| V4 (re-runnable, regression) | Regression Gate | pre-commit golden tests + cross-run regression check |
| V5 (failure independence) | Crash/Hang Detector | sequential vs. parallel comparison; explicit assumption statement |
| V6 (coincident failure) | Crash/Hang Detector | classifier groups co-failing tests by category |
| V7 (uncovered code) | Correspondence Auditor | LLVM coverage + contract cross-reference |
| V8 (no suppression) | Contract Registry | self-check SC-3 rejects #[ignore] and unjustified overrides |
| V9 (stopping criterion) | Regression Gate | convergence = 0 regressions + 0 failures + non-increasing suspicious |
| V10 (multiple output vars) | Divergence Ledger | fail_count, total, pct, max_diff per pixel test |
| M1 (oracle relationship) | Contract Registry | category.oracle field defines relationship type |
| M2 (crash/hang detection) | Crash/Hang Detector | timeout wrapper + signal detection |
| M3 (statement coverage) | Correspondence Auditor | LLVM source-based coverage integration |
| M7 (result classification) | Divergence Ledger | pass / fail / suspicious / unreported classification |
| M8 (regression as metric) | Regression Gate | regressions count in classification.json |
| F1 (actual not specified) | Contract Registry | contract has tolerance params, not expected values |
| F2 (suspicious flagging) | Divergence Ledger | 50%/75% threshold proximity detection |
| F3 (captured from execution) | Contract Registry | golden data from C++ generator, not hand-authored |
| F4 (change-guided) | Regression Gate | --changed-files mode using git diff |
| F5 (branch uniqueness) | Correspondence Auditor | Partial: LLVM coverage + no duplicate test names |
| F6 (conversion exercise) | Correspondence Auditor | conversion ratio metric |
| F8 (bug policy) | Regression Gate | diverged field with reason in contract |
| Self-conformance | Self-Conformance Verifier | SC-1 through SC-9 checks gate all other phases |

---

## Conclusion

This harness is buildable from 10 new shell scripts, 1 new JSON file, and extensions to the existing pre-commit hook. It does not require new Rust crates, new build targets, or changes to the existing golden test infrastructure. It composes the existing `MEASURE_DIVERGENCE` JSONL output, the existing LLVM coverage tooling, and the existing C++ golden data generator into a closed verification loop where every requirement is traceable to a concrete mechanism, and every mechanism is tested by the self-conformance verifier before it is trusted. The harness is maximally conservative: it refuses to run unless it can prove its own checks work, it refuses to pass unless all oracles are present and all comparisons are exhaustive, and it refuses to suppress any failure without an audited justification.
