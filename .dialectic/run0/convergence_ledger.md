# Convergence Ledger -- Dialectic Run 0

## Process Overview

| Metric | Count |
|--------|-------|
| Total propositions | 73 |
| Agent 1 (Methodology Purist) | 25 |
| Agent 2 (Harness Architect) | 23 |
| Agent 3 (Failure Analyst) | 25 |
| Tensions identified (Round 2) | 48 |
| Tensions addressed (Round 3) | 48 |
| Unresolved high-severity tensions | 0 |
| Rounds of dialectic | 3 (propositions, conflict mapping, prosecution/defense/adjudication) |

**Scoring axes:** Defensibility (grounding in sources), Specificity (concreteness/actionability), Robustness (resistance to counterargument), Compatibility (coherence with other propositions and practical constraints).

**Categories:**
- **Survivor** (composite >= 0.75, no axis below 0.50): Proposition withstood dialectic scrutiny.
- **Wounded** (composite 0.50-0.74, or any axis below 0.50): Proposition has merit but sustained significant damage.
- **Contested** (unresolved tension with severity >= 0.60): Proposition has unresolved conflicts.
- **Fallen** (composite below 0.50 or defensibility below 0.30): Proposition did not survive the dialectic.

## Final Scoreboard

All 73 propositions sorted by composite score (descending).

| Rank | ID | Cat | Def. | Spec. | Rob. | Comp. | Composite | Proposition (truncated) |
|------|----|-----|------|-------|------|-------|-----------|-------------------------|
| 1 | a3-04 | SURV | 0.98 | 0.95 | 0.90 | 0.80 | **0.9075** | The require_golden!() macro causes all 232 golden tests to r... |
| 2 | a3-15 | SURV | 0.94 | 0.90 | 0.88 | 0.90 | **0.9050** | The MEASURE_DIVERGENCE metric system exists but is opt-in, o... |
| 3 | a2-07 | SURV | 0.88 | 0.93 | 0.79 | 0.90 | **0.8750** | The correspondence auditor should enforce that every C++ hea... |
| 4 | a2-13 | SURV | 0.92 | 0.93 | 0.78 | 0.85 | **0.8700** | The self-conformance verifier should check that no #[ignore]... |
| 5 | a2-17 | SURV | 0.90 | 0.88 | 0.80 | 0.90 | **0.8700** | The harness must verify 8 distinct output domains (painter, ... |
| 6 | a1-19 | SURV | 0.92 | 0.80 | 0.85 | 0.90 | **0.8675** | Where practicality forces a deviation from published methodo... |
| 7 | a2-03 | SURV | 0.92 | 0.95 | 0.85 | 0.75 | **0.8675** | The harness must refuse to run if any golden data file on di... |
| 8 | a1-15 | SURV | 0.90 | 0.80 | 0.83 | 0.90 | **0.8575** | Golden tests must test observable behavior, not implementati... |
| 9 | a3-22 | SURV | 0.89 | 0.92 | 0.82 | 0.80 | **0.8575** | The translate_cpp_notice_flags function contains 10 manual b... |
| 10 | a3-08 | SURV | 0.93 | 0.88 | 0.85 | 0.75 | **0.8525** | The image comparison double gate (channel_tolerance + max_fa... |
| 11 | a1-11 | SURV | 0.85 | 0.90 | 0.80 | 0.85 | **0.8500** | The harness must record the exact C++ compiler version, flag... |
| 12 | a2-06 | SURV | 0.93 | 0.88 | 0.77 | 0.80 | **0.8450** | The divergence ledger should cross-reference reported test c... |
| 13 | a2-12 | SURV | 0.90 | 0.85 | 0.78 | 0.85 | **0.8450** | The self-conformance verifier must run before all other harn... |
| 14 | a1-09 | SURV | 0.91 | 0.78 | 0.88 | 0.80 | **0.8425** | The frozen oracle permanently bakes in any coincident failur... |
| 15 | a2-23 | SURV | 0.87 | 0.85 | 0.80 | 0.85 | **0.8425** | The self-conformance verifier should run all checks and repo... |
| 16 | a3-18 | SURV | 0.88 | 0.85 | 0.82 | 0.80 | **0.8375** | The parallel rendering tests prove determinism and consisten... |
| 17 | a1-04 | SURV | 0.93 | 0.70 | 0.90 | 0.80 | **0.8325** | In a manual C++-to-Rust port, failure independence is system... |
| 18 | a2-02 | SURV | 0.90 | 0.91 | 0.72 | 0.80 | **0.8325** | A single JSON contract registry file (contract.json) should ... |
| 19 | a3-07 | SURV | 0.91 | 0.95 | 0.82 | 0.65 | **0.8325** | Golden test tolerances have ratcheted monotonically looser o... |
| 20 | a3-12 | SURV | 0.87 | 0.82 | 0.85 | 0.78 | **0.8300** | LLM sessions modifying tolerance values operate without prov... |
| 21 | a2-16 | SURV | 0.88 | 0.90 | 0.73 | 0.80 | **0.8275** | When a golden test failure is determined to be an intentiona... |
| 22 | a2-11 | SURV | 0.82 | 0.93 | 0.70 | 0.85 | **0.8250** | The harness should wrap golden test execution with a global ... |
| 23 | a2-18 | SURV | 0.92 | 0.83 | 0.70 | 0.85 | **0.8250** | The golden data files are binary snapshots of C++ execution ... |
| 24 | a3-06 | SURV | 0.85 | 0.93 | 0.72 | 0.80 | **0.8250** | All widget golden tests use a single viewport configuration ... |
| 25 | a3-24 | SURV | 0.86 | 0.90 | 0.72 | 0.82 | **0.8250** | The uniform layout test epsilon of 1e-6 creates an illusion ... |
| 26 | a3-02 | SURV | 0.88 | 0.90 | 0.65 | 0.85 | **0.8200** | The golden binary format has no schema version, magic number... |
| 27 | a1-06 | SURV | 0.90 | 0.88 | 0.75 | 0.70 | **0.8075** | The harness must not have a mechanism to mark a failing gold... |
| 28 | a1-12 | SURV | 0.83 | 0.85 | 0.73 | 0.82 | **0.8075** | The harness must produce quantitative disagreement metrics (... |
| 29 | a1-20 | SURV | 0.87 | 0.83 | 0.73 | 0.80 | **0.8075** | The harness must identify which Rust code paths are exercise... |
| 30 | a1-25 | SURV | 0.88 | 0.80 | 0.85 | 0.70 | **0.8075** | If the C++ build environment becomes unavailable, the abilit... |
| 31 | a2-19 | SURV | 0.85 | 0.88 | 0.70 | 0.80 | **0.8075** | The full harness run should be a sequential 7-phase pipeline... |
| 32 | a2-20 | SURV | 0.88 | 0.80 | 0.70 | 0.85 | **0.8075** | Each harness component should include its own self-test mode... |
| 33 | a3-11 | SURV | 0.85 | 0.90 | 0.78 | 0.70 | **0.8075** | The settle() function in stateful golden tests hardcodes a f... |
| 34 | a1-01 | SURV | 0.92 | 0.80 | 0.80 | 0.70 | **0.8050** | The frozen oracle model (C++ executed once to produce golden... |
| 35 | a1-10 | SURV | 0.94 | 0.90 | 0.80 | 0.58 | **0.8050** | If the harness allows 'blessing' Rust output as golden data,... |
| 36 | a1-21 | SURV | 0.84 | 0.82 | 0.75 | 0.80 | **0.8025** | The harness must distinguish regressions (newly-introduced f... |
| 37 | a2-21 | SURV | 0.83 | 0.88 | 0.65 | 0.85 | **0.8025** | The harness state protocol should be filesystem-based, using... |
| 38 | a1-16 | SURV | 0.88 | 0.82 | 0.80 | 0.70 | **0.8000** | The asymmetric oracle relationship (C++ is oracle, Rust is s... |
| 39 | a1-23 | SURV | 0.90 | 0.72 | 0.82 | 0.75 | **0.7975** | The frozen oracle is a valid practical adaptation of Vouk's ... |
| 40 | a1-17 | SURV | 0.86 | 0.82 | 0.72 | 0.78 | **0.7950** | When oracle regeneration is required, it is a manual step ou... |
| 41 | a3-20 | SURV | 0.86 | 0.82 | 0.78 | 0.72 | **0.7950** | A bug in shared test infrastructure (TestHarness, NoticeBeha... |
| 42 | a2-05 | SURV | 0.82 | 0.95 | 0.55 | 0.85 | **0.7925** | A test case is classified as 'suspicious' if it passes but i... |
| 43 | a2-08 | SURV | 0.85 | 0.88 | 0.64 | 0.80 | **0.7925** | The regression gate should compare each harness run's classi... |
| 44 | a3-01 | SURV | 0.92 | 0.85 | 0.80 | 0.60 | **0.7925** | The C++ golden data generator (gen_golden.cpp) is itself unv... |
| 45 | a1-24 | SURV | 0.83 | 0.78 | 0.72 | 0.80 | **0.7825** | In the porting context where all code is being changed, Feat... |
| 46 | a2-04 | SURV | 0.88 | 0.90 | 0.65 | 0.70 | **0.7825** | Tolerance parameters should only be allowed to decrease (tig... |
| 47 | a3-13 | SURV | 0.83 | 0.78 | 0.80 | 0.72 | **0.7825** | An LLM is structurally incentivized to widen golden test tol... |
| 48 | a3-03 | SURV | 0.90 | 0.92 | 0.75 | 0.55 | **0.7800** | The compare_images function permanently excludes the alpha c... |
| 49 | a2-10 | SURV | 0.85 | 0.92 | 0.53 | 0.80 | **0.7750** | Test independence should be verified by running golden tests... |
| 50 | a3-21 | SURV | 0.92 | 0.75 | 0.88 | 0.55 | **0.7750** | The frozen oracle converts the relational property 'C++ and ... |
| 51 | a2-22 | SURV | 0.80 | 0.95 | 0.58 | 0.75 | **0.7700** | The contract registry must enumerate all 214 golden data fil... |
| 52 | a3-19 | SURV | 0.83 | 0.80 | 0.70 | 0.75 | **0.7700** | The notice, behavioral, and input golden test modules each v... |
| 53 | a3-05 | SURV | 0.82 | 0.80 | 0.70 | 0.75 | **0.7675** | Golden painter tests exercise individual drawing primitives ... |
| 54 | a3-14 | SURV | 0.80 | 0.88 | 0.65 | 0.70 | **0.7575** | The golden harness covers approximately 15 of 100 Rust sourc... |
| 55 | a2-01 | SURV | 0.80 | 0.75 | 0.65 | 0.82 | **0.7550** | The verification harness should be composed of shell scripts... |
| 56 | a3-17 | SURV | 0.85 | 0.82 | 0.75 | 0.60 | **0.7550** | The C++ reference version is fixed at Eagle Mode 0.96.4, mea... |
| 57 | a2-09 | WOUN | 0.75 | 0.90 | 0.58 | 0.75 | **0.7450** | Convergence should be defined as: zero regressions, zero fai... |
| 58 | a2-14 | WOUN | 0.75 | 0.85 | 0.53 | 0.85 | **0.7450** | When multiple tests fail in the same category with similar m... |
| 59 | a3-23 | WOUN | 0.80 | 0.78 | 0.75 | 0.65 | **0.7450** | The cascade of silent skip + tolerance ratchet + session amn... |
| 60 | a1-07 | WOUN | 0.82 | 0.85 | 0.75 | 0.50 | **0.7300** | The 'even within tolerance' clause in requirement V8 is not ... |
| 61 | a3-10 | WOUN | 0.80 | 0.72 | 0.68 | 0.70 | **0.7250** | The golden harness uses fixed, handcrafted inputs rather tha... |
| 62 | a3-16 | WOUN | 0.82 | 0.80 | 0.60 | 0.65 | **0.7175** | Golden data is stored in the same repository as Rust code, a... |
| 63 | a1-02 | WOUN | 0.85 | 0.85 | 0.60 | 0.55 | **0.7125** | Every output channel the C++ produces (pixels, layout rects,... |
| 64 | a2-15 | WOUN | 0.78 | 0.84 | 0.53 | 0.70 | **0.7125** | The harness should support change-guided coverage via a --ch... |
| 65 | a1-03 | WOUN | 0.85 | 0.73 | 0.55 | 0.65 | **0.6950** | Thorough investigation of disagreements is a process require... |
| 66 | a1-18 | WOUN | 0.85 | 0.73 | 0.60 | 0.60 | **0.6950** | Each golden test should be specific enough that only the cor... |
| 67 | a1-05 | WOUN | 0.87 | 0.63 | 0.70 | 0.55 | **0.6875** | Generated test inputs are essential to differential testing,... |
| 68 | a3-25 | WOUN | 0.75 | 0.65 | 0.70 | 0.60 | **0.6750** | As more golden tests are added, the coverage gap between tes... |
| 69 | a1-22 | WOUN | 0.78 | 0.60 | 0.55 | 0.70 | **0.6575** | The six missing requirements (V11 configuration management, ... |
| 70 | a1-08 | WOUN | 0.80 | 0.72 | 0.60 | 0.50 | **0.6550** | Vouk's stopping criterion is reliability-based (statistical ... |
| 71 | a1-13 | WOUN | 0.70 | 0.88 | 0.55 | 0.48 | **0.6525** | MEASURE_DIVERGENCE reporting must be always-on for methodolo... |
| 72 | a3-09 | WOUN | 0.78 | 0.70 | 0.63 | 0.50 | **0.6525** | The Rust port was developed by reading C++ source, violating... |
| 73 | a1-14 | WOUN | 0.82 | 0.75 | 0.45 | 0.40 | **0.6050** | The full generate-execute-compare cycle must be automated wi... |

## Survivors (56 propositions)

Propositions that withstood the full dialectic with composite >= 0.75 and no axis below 0.50.

### Top Tier (composite >= 0.85)

**a3-04** (composite 0.9075) -- D:0.98 S:0.95 R:0.90 C:0.80
> The require_golden!() macro causes all 232 golden tests to report 'ok' (passing) when golden data is absent, making it impossible for CI systems checking exit codes to distinguish between 'tested and passed' and 'silently skipped'.
> *Deltas applied:* 1 adjustments from tensions t-12,t-40

**a3-15** (composite 0.9050) -- D:0.94 S:0.90 R:0.88 C:0.90
> The MEASURE_DIVERGENCE metric system exists but is opt-in, outputs to stderr, and has no storage, comparison, or alerting mechanism, making it a diagnostic tool rather than a regression detection system.
> *Deltas applied:* 1 adjustments from tensions t-17

**a2-07** (composite 0.8750) -- D:0.88 S:0.93 R:0.79 C:0.90
> The correspondence auditor should enforce that every C++ header has exactly one of a .rs file or a .no_rust_equivalent marker, and every non-mod .rs file has a matching C++ header, a .rust_only marker, or a SPLIT: comment.
> *Deltas applied:* 1 adjustments from tensions t-46

**a2-13** (composite 0.8700) -- D:0.92 S:0.93 R:0.78 C:0.85
> The self-conformance verifier should check that no #[ignore] attributes exist on golden tests and no tolerance overrides exist without a diverged=true flag and reason, to prevent silent result suppression.
> *Deltas applied:* 1 adjustments from tensions t-11

**a2-17** (composite 0.8700) -- D:0.90 S:0.88 R:0.80 C:0.90
> The harness must verify 8 distinct output domains (painter, compositor, layout, behavioral, trajectory, widget_state, notice, input) with tolerance regimes varying from exact-match to per-channel pixel tolerance to floating-point epsilon.
> *No deltas applied -- scores unchanged from Round 1.*

**a1-19** (composite 0.8675) -- D:0.92 S:0.80 R:0.85 C:0.90
> Where practicality forces a deviation from published methodology, the deviation must be documented at the exact point where it occurs, with the specific methodological requirement it violates and the rationale for accepting the gap.
> *Deltas applied:* 1 adjustments from tensions t-26

**a2-03** (composite 0.8675) -- D:0.92 S:0.95 R:0.85 C:0.75
> The harness must refuse to run if any golden data file on disk is not present in the contract, or if any contract entry lacks a corresponding golden file.
> *No deltas applied -- scores unchanged from Round 1.*

**a1-15** (composite 0.8575) -- D:0.90 S:0.80 R:0.83 C:0.90
> Golden tests must test observable behavior, not implementation artifacts; a test that depends on Rust-specific implementation details (e.g., HashMap iteration order) rather than behavioral output violates Feathers's principle that characterization tests survive refactoring.
> *Deltas applied:* 1 adjustments from tensions t-21

**a3-22** (composite 0.8575) -- D:0.89 S:0.92 R:0.82 C:0.80
> The translate_cpp_notice_flags function contains 10 manual bit-position mappings that are a single point of unverified trust; a transposition error would cause symmetric false positives where every affected test passes while behavioral semantics diverge.
> *No deltas applied -- scores unchanged from Round 1.*

**a3-08** (composite 0.8525) -- D:0.93 S:0.88 R:0.85 C:0.75
> The image comparison double gate (channel_tolerance + max_failure_pct) is structurally incapable of detecting localized catastrophic errors because it places no cap on how wrong failing pixels can be, only on how many pixels exceed the per-channel threshold.
> *Deltas applied:* 1 adjustments from tensions t-24,t-25,t-47

**a1-11** (composite 0.8500) -- D:0.85 S:0.90 R:0.80 C:0.85
> The harness must record the exact C++ compiler version, flags, and source revision used to generate golden data, and the exact Rust compiler version and flags used to execute the port, because without this the golden data cannot be traced back to the build that produced it.
> *Deltas applied:* 1 adjustments from tensions t-16

### Mid Tier (composite 0.80-0.84)

**a2-06** (composite 0.8450) -- D:0.93 S:0.88 R:0.77 C:0.80
> The divergence ledger should cross-reference reported test cases against the contract registry, flagging any missing cases as 'unreported' and exiting non-zero to catch silently skipped tests.
> *Deltas:* robustness -0.08 (via t-12,t-40)

**a2-12** (composite 0.8450) -- D:0.90 S:0.85 R:0.78 C:0.85
> The self-conformance verifier must run before all other harness phases, and if it fails, the entire harness must abort without running any further checks.
> *Deltas:* robustness -0.02 (via t-48)

**a1-09** (composite 0.8425) -- D:0.91 S:0.78 R:0.88 C:0.80
> The frozen oracle permanently bakes in any coincident failures present at capture time; unlike live back-to-back testing where new inputs might expose different manifestations, the golden data is fixed and no amount of Rust-side testing will find coincident failures.

**a2-23** (composite 0.8425) -- D:0.87 S:0.85 R:0.80 C:0.85
> The self-conformance verifier should run all checks and report all failures at once (not fail-fast within the self-check phase), while the overall harness should abort if any self-conformance check fails.

**a3-18** (composite 0.8375) -- D:0.88 S:0.85 R:0.82 C:0.80
> The parallel rendering tests prove determinism and consistency between single-threaded and multi-threaded paths but provide zero additional correctness confidence, since both paths share the same scanline rendering code and would have the same bugs.

**a1-04** (composite 0.8325) -- D:0.93 S:0.70 R:0.90 C:0.80
> In a manual C++-to-Rust port, failure independence is systematically violated because the port author reads and translates the C++ code, making misunderstandings faithfully reproducible in Rust -- this is the worst case for Vouk's detection probability model.

**a2-02** (composite 0.8325) -- D:0.90 S:0.91 R:0.72 C:0.80
> A single JSON contract registry file (contract.json) should enumerate every golden test case, its category, oracle relationship, and tolerance parameters, serving as the single source of truth.
> *Deltas:* robustness -0.03 (via t-31,t-32); specificity -0.01 (via t-23)

**a3-07** (composite 0.8325) -- D:0.91 S:0.95 R:0.82 C:0.65
> Golden test tolerances have ratcheted monotonically looser over time, from (ch_tol=1, max_fail=0.1%) for basic primitives to (ch_tol=3, max_fail=10.0%) for tunnel widgets and from 1e-6 to 1e-2 for animator trajectories, with no record of why each tolerance was chosen and no mechanism to detect or prevent widening.
> *Deltas:* defensibility +0.03 (via t-24,t-25,t-47)

**a3-12** (composite 0.8300) -- D:0.87 S:0.82 R:0.85 C:0.78
> LLM sessions modifying tolerance values operate without provenance on why previous tolerances were set, causing tolerances to accumulate the maximum looseness ever needed across sessions because no session has context to tighten them.

**a2-16** (composite 0.8275) -- D:0.88 S:0.90 R:0.73 C:0.80
> When a golden test failure is determined to be an intentional divergence from C++ behavior (e.g., fixing a C++ bug), the contract should record this with diverged=true, a reason string, and a tolerance_override, and the classifier should distinguish these from regressions.
> *Deltas:* robustness -0.02 (via t-39)

**a2-11** (composite 0.8250) -- D:0.82 S:0.93 R:0.70 C:0.85
> The harness should wrap golden test execution with a global 120-second timeout to detect hangs, classifying exit code 124 as HANG and exit codes >128 as CRASH with signal extraction.

**a2-18** (composite 0.8250) -- D:0.92 S:0.83 R:0.70 C:0.85
> The golden data files are binary snapshots of C++ execution output generated by gen_golden.cpp linked against libemCore.so, satisfying the requirement that oracles capture actual behavior rather than specified behavior.
> *Deltas:* robustness -0.05 (via t-01,t-02); compatibility -0.05 (via t-09); specificity -0.02 (via t-29)

**a3-06** (composite 0.8250) -- D:0.85 S:0.93 R:0.72 C:0.80
> All widget golden tests use a single viewport configuration (800x600, tallness 0.75, NO_ACTIVE_HIGHLIGHT), testing one point in a continuous parameter space that includes zoom-dependent rendering behavior controlled by ViewConditionType.

**a3-24** (composite 0.8250) -- D:0.86 S:0.90 R:0.72 C:0.82
> The uniform layout test epsilon of 1e-6 creates an illusion of uniform precision because it represents wildly different relative tolerances across tests: 0.1% for layouts with widths of 0.001 versus 0.0003% for layouts with widths of 0.333.

**a3-02** (composite 0.8200) -- D:0.88 S:0.90 R:0.65 C:0.85
> The golden binary format has no schema version, magic number, or checksum, making it silently vulnerable to cross-platform endianness corruption and compiler-dependent struct padding changes.

**a1-06** (composite 0.8075) -- D:0.90 S:0.88 R:0.75 C:0.70
> The harness must not have a mechanism to mark a failing golden test as 'expected failure' without explicit documentation of why it fails and what investigation was performed; a blanket #[ignore] annotation violates Vouk's V3 and V8.
> *Deltas:* robustness -0.05 (via t-12,t-40)

**a1-12** (composite 0.8075) -- D:0.83 S:0.85 R:0.73 C:0.82
> The harness must produce quantitative disagreement metrics (percentage of pixels differing, maximum channel difference, distribution of differences), not merely binary pass/fail outcomes, because Vouk's detection model is inherently quantitative.
> *Deltas:* robustness -0.05 (via t-17)

**a1-20** (composite 0.8075) -- D:0.87 S:0.83 R:0.73 C:0.80
> The harness must identify which Rust code paths are exercised by golden comparisons and which are not, so that inspection effort per Vouk V7 can be directed at uncovered code; if 40% of code paths have no golden coverage, the harness must report this.
> *Deltas:* robustness -0.05 (via t-27); compatibility -0.02 (via t-27)

**a1-25** (composite 0.8075) -- D:0.88 S:0.80 R:0.85 C:0.70
> If the C++ build environment becomes unavailable, the ability to extend the golden test suite is permanently lost; the harness must document this dependency and the consequences of losing it.

**a2-19** (composite 0.8075) -- D:0.85 S:0.88 R:0.70 C:0.80
> The full harness run should be a sequential 7-phase pipeline (self-conformance, contract validation, correspondence audit, divergence measurement, independence verification, classification, summary) where each phase gates the next.
> *Deltas:* specificity -0.02 (via t-06)

**a2-20** (composite 0.8075) -- D:0.88 S:0.80 R:0.70 C:0.85
> Each harness component should include its own self-test mode that verifies correct behavior on synthetic inputs (e.g., the contract checker tests rejection of missing entries, the classifier tests detection of known regressions).

**a3-11** (composite 0.8075) -- D:0.85 S:0.90 R:0.78 C:0.70
> The settle() function in stateful golden tests hardcodes a fixed iteration count (typically 5) to convert temporal convergence into a spatial snapshot, hiding convergence rate differences between C++ and Rust and creating fragile tests that break if convergence requirements change.
> *Deltas:* specificity +0.02 (via t-21)

**a1-01** (composite 0.8050) -- D:0.92 S:0.80 R:0.80 C:0.70
> The frozen oracle model (C++ executed once to produce golden files, Rust compared later) must verify input identity rather than merely assume it, because the temporal decoupling means 'same input' is no longer guaranteed by construction.
> *Deltas:* robustness -0.05 (via t-01,t-02); compatibility -0.05 (via t-01,t-02)

**a1-10** (composite 0.8050) -- D:0.94 S:0.90 R:0.80 C:0.58
> If the harness allows 'blessing' Rust output as golden data, it is no longer characterizing C++ behavior but testing Rust against itself, which provides zero fault detection power; save_trajectory_golden is methodologically dangerous.
> *Deltas:* compatibility -0.05 (via t-14); robustness -0.05 (via t-14); compatibility -0.02 (via t-15)

**a1-21** (composite 0.8025) -- D:0.84 S:0.82 R:0.75 C:0.80
> The harness must distinguish regressions (newly-introduced failures) from pre-existing failures, which requires either a known-failure list or a mechanism to compare current results against previous results.

**a2-21** (composite 0.8025) -- D:0.83 S:0.88 R:0.65 C:0.85
> The harness state protocol should be filesystem-based, using target/harness/ as the output directory with timestamped subdirectories and a 'latest' symlink, enabling stateful tracking across Claude Code sessions without in-memory state.

**a1-16** (composite 0.8000) -- D:0.88 S:0.82 R:0.80 C:0.70
> The asymmetric oracle relationship (C++ is oracle, Rust is subject) is a project decision, not a methodological requirement of differential testing; McKeeman's original formulation treats all systems as peers and flags any disagreement for investigation.

### Lower Tier (composite 0.75-0.79)

**a1-23** (composite 0.7975) -- D:0.90 S:0.72 R:0.82 C:0.75
> The frozen oracle is a valid practical adaptation of Vouk's live back-to-back model, but none of the three source texts describe or endorse it, so every gap it introduces is a deviation from published methodology that must be justified on its own terms.

**a1-17** (composite 0.7950) -- D:0.86 S:0.82 R:0.72 C:0.78
> When oracle regeneration is required, it is a manual step outside the normal test cycle; the harness must detect staleness -- if golden data was generated from a different version of the C++ source than what is currently checked in, the comparison is unsound.

**a3-20** (composite 0.7950) -- D:0.86 S:0.82 R:0.78 C:0.72
> A bug in shared test infrastructure (TestHarness, NoticeBehavior, InputTrackingBehavior) would be invisible because all tests using that infrastructure were calibrated against its behavior, making the bug baked into expectations across the entire suite.

**a2-05** (composite 0.7925) -- D:0.82 S:0.95 R:0.55 C:0.85
> A test case is classified as 'suspicious' if it passes but its failure percentage exceeds 50% of the category's max_failure_pct threshold, or its max_diff exceeds 75% of channel_tolerance.
> *Deltas:* robustness -0.05 (via t-35)

**a2-08** (composite 0.7925) -- D:0.85 S:0.88 R:0.64 C:0.80
> The regression gate should compare each harness run's classification against the previous run (via a 'latest' symlink), flagging any test that moved from pass to fail/suspicious as a regression.
> *Deltas:* robustness -0.01 (via t-28)

**a3-01** (composite 0.7925) -- D:0.92 S:0.85 R:0.80 C:0.60
> The C++ golden data generator (gen_golden.cpp) is itself unverified bespoke test infrastructure, not production Eagle Mode code, so golden tests prove equivalence with the generator's configuration rather than with actual Eagle Mode runtime behavior.

**a1-24** (composite 0.7825) -- D:0.83 S:0.78 R:0.72 C:0.80
> In the porting context where all code is being changed, Feathers's 'coverage guided by change' principle means the harness must prioritize golden test coverage for code paths most likely to diverge in translation (integer arithmetic, pointer semantics, memory layout) rather than aiming for uniform coverage.

**a2-04** (composite 0.7825) -- D:0.88 S:0.90 R:0.65 C:0.70
> Tolerance parameters should only be allowed to decrease (tighten); increasing (relaxing) them requires a separate script with a mandatory --reason flag that writes a JSONL audit entry.
> *Deltas:* robustness -0.05 (via t-33,t-34)

**a3-13** (composite 0.7825) -- D:0.83 S:0.78 R:0.80 C:0.72
> An LLM is structurally incentivized to widen golden test tolerances rather than investigate pixel-level divergence root causes, because widening is a one-character change producing green tests while investigation requires deep cross-referencing of C++ source.

**a3-03** (composite 0.7800) -- D:0.90 S:0.92 R:0.75 C:0.55
> The compare_images function permanently excludes the alpha channel from comparison because C++ and Rust use semantically different alpha representations, creating a structural blind spot for any alpha-dependent behavioral divergence.

**a2-10** (composite 0.7750) -- D:0.85 S:0.92 R:0.53 C:0.80
> Test independence should be verified by running golden tests in both sequential (--test-threads=1) and parallel (--test-threads=4) modes and comparing JSONL output; differing results indicate an independence violation.
> *Deltas:* robustness -0.02 (via t-37)

**a3-21** (composite 0.7750) -- D:0.92 S:0.75 R:0.88 C:0.55
> The frozen oracle converts the relational property 'C++ and Rust produce the same output for the same input' into the weaker unary property 'Rust produces this specific output,' which does not guarantee equivalence for inputs not in the golden set.

**a2-22** (composite 0.7700) -- D:0.80 S:0.95 R:0.58 C:0.75
> The contract registry must enumerate all 214 golden data files across 8 categories (42 painter + 61 compositor + 31 layout + 25 behavioral + 20 trajectory + 16 widget_state + 13 notice + 6 input), each mapping a golden file name to a Rust test function name.
> *Deltas:* robustness -0.02 (via t-30)

**a3-19** (composite 0.7700) -- D:0.83 S:0.80 R:0.70 C:0.75
> The notice, behavioral, and input golden test modules each verify one aspect of panel tree behavior in isolation, but cross-cutting interactions where input triggers notices that trigger behavior callbacks are not tested by any golden comparison.

**a3-05** (composite 0.7675) -- D:0.82 S:0.80 R:0.70 C:0.75
> Golden painter tests exercise individual drawing primitives in isolation but do not systematically test compositions of primitives within a shared painter context where state accumulation (clip rects, transforms, canvas color) could expose interaction bugs.

**a3-14** (composite 0.7575) -- D:0.80 S:0.88 R:0.65 C:0.70
> The golden harness covers approximately 15 of 100 Rust source files in emCore, meaning file-level coverage is roughly 15%, and the 232-test count creates a completeness illusion that does not survive simple enumeration.
> *Deltas:* specificity +0.03 (via t-04,t-05,t-03)

**a2-01** (composite 0.7550) -- D:0.80 S:0.75 R:0.65 C:0.82
> The verification harness should be composed of shell scripts, JSON schemas, and Rust integration tests that compose via Unix pipes and shared file conventions, rather than being a framework.
> *Deltas:* compatibility -0.03 (via t-19,t-20)

**a3-17** (composite 0.7550) -- D:0.85 S:0.82 R:0.75 C:0.60
> The C++ reference version is fixed at Eagle Mode 0.96.4, meaning the golden data may encode bugs that were fixed in later versions, and the Rust port faithfully reproduces those bugs as verified-correct behavior.

## Wounded (17 propositions)

Propositions with composite 0.50-0.74, or any axis below 0.50. These have merit but sustained significant damage during the dialectic.

**a2-09** (composite 0.7450) -- D:0.75 S:0.90 R:0.58 C:0.75
> Convergence should be defined as: zero regressions, zero failures, zero unreported cases, and suspicious count non-increasing across the last 3 runs.
> *Deltas:* robustness -0.02 (via t-36); defensibility -0.01 (via t-36); defensibility -0.02 (via t-41)
> *Key adjudication (t-36):* Low-medium severity (0.58). The defense's scope distinction is valid, but a2-09's use of the word 'convergence' implies completeness that it does not measure. Small delta for the semantic overstatemen...

**a2-14** (composite 0.7450) -- D:0.75 S:0.85 R:0.53 C:0.85
> When multiple tests fail in the same category with similar max_diff values (within 2x of each other), they should be flagged as coincident failures suggesting a shared root cause.
> *Deltas:* robustness -0.02 (via t-44)
> *Key adjudication (t-44):* Low-medium severity (0.55). The detection is still valuable. Small delta for potential misattribution....

**a3-23** (composite 0.7450) -- D:0.80 S:0.78 R:0.75 C:0.65
> The cascade of silent skip + tolerance ratchet + session amnesia across multiple LLM sessions can produce a test suite with loose tolerances comparing against stale golden data that may not exist in some environments, all while reporting green.

**a1-07** (composite 0.7300) -- D:0.82 S:0.85 R:0.75 C:0.50
> The 'even within tolerance' clause in requirement V8 is not supported by Vouk's text; Vouk assumes exact comparison, so the correct requirement is that every disagreement exceeding the comparison threshold must be inspected, while sub-tolerance treatment is a design choice the sources do not address.
> *Deltas:* compatibility -0.05 (via t-13)
> *Key adjudication (t-13):* The defense's distinction between reporting and inspection is valid but thin. a1-13's actual claim is that always-on reporting is needed 'for methodology compliance,' which directly invokes the method...

**a3-10** (composite 0.7250) -- D:0.80 S:0.72 R:0.68 C:0.70
> The golden harness uses fixed, handcrafted inputs rather than random/generated inputs, eliminating the exploratory power of McKeeman-style differential testing and limiting verification to code paths the test author explicitly imagined.

**a3-16** (composite 0.7175) -- D:0.82 S:0.80 R:0.60 C:0.65
> Golden data is stored in the same repository as Rust code, allowing a single commit to modify both implementation and golden data simultaneously, which undermines the golden data's purpose as a fixed reference point.

**a1-02** (composite 0.7125) -- D:0.85 S:0.85 R:0.60 C:0.55
> Every output channel the C++ produces (pixels, layout rects, behavioral state, notice flags, trajectory data) must have a corresponding golden comparison; comparing only a subset of output channels violates Vouk's exhaustive pair comparison requirement.
> *Deltas:* robustness -0.10 (via t-04,t-05,t-03); compatibility -0.10 (via t-04,t-05,t-03); defensibility -0.03 (via t-04,t-05,t-03)
> *Key adjudication (t-04,t-05,t-03):* The defense makes a valid distinction: a1-02 is about channels per test, not file coverage. However, the alpha exclusion (t-04) is still a significant violation of the exhaustiveness principle, even i...

**a2-15** (composite 0.7125) -- D:0.78 S:0.84 R:0.53 C:0.70
> The harness should support change-guided coverage via a --changed-files flag that restricts golden test runs to only tests whose contract entries reference modified Rust files, using git diff to determine changes.
> *Deltas:* robustness -0.02 (via t-38); specificity -0.01 (via t-43)
> *Key adjudication (t-38):* Low-medium severity (0.55). The defense correctly frames it as optimization, not replacement. Small delta for the acknowledged transitive dependency blind spot....

**a1-03** (composite 0.6950) -- D:0.85 S:0.73 R:0.55 C:0.65
> Thorough investigation of disagreements is a process requirement, not merely a diagnostic output requirement; the harness must support classifying each disagreement as (a) port fault, (b) oracle fault, (c) specification ambiguity, or (d) acceptable implementation difference.
> *Deltas:* robustness -0.10 (via t-07); compatibility -0.05 (via t-07); specificity -0.02 (via t-06)
> *Key adjudication (t-07):* The defense correctly notes that a1-03 is not solely dependent on LLM capabilities and that human code review provides a backstop. However, the prosecution's structural observation is sound: if the do...

**a1-18** (composite 0.6950) -- D:0.85 S:0.73 R:0.60 C:0.60
> Each golden test should be specific enough that only the correct implementation satisfies it; large tolerances in pixel comparison weaken branch path uniqueness and allow wrong implementations to pass.
> *Deltas:* robustness -0.08 (via t-24,t-25,t-47); specificity -0.05 (via t-24,t-25,t-47)
> *Key adjudication (t-24,t-25,t-47):* The defense makes a valid philosophical point: a principle is not undermined by evidence of its violation. However, a1-18 is not just stating a principle -- it is claiming to be a requirement for the ...

**a1-05** (composite 0.6875) -- D:0.87 S:0.63 R:0.70 C:0.55
> Generated test inputs are essential to differential testing, not merely complementary to hand-written tests; McKeeman argues they are the primary mechanism for discovering unexpected disagreements because hand-written tests share cognitive biases with the code.
> *Deltas:* robustness -0.05 (via t-09); compatibility -0.05 (via t-09); specificity -0.02 (via t-10)
> *Key adjudication (t-09):* The defense's batch generation argument is valid and reduces the severity of the mutual exclusion. However, the defense's resolution still requires a live C++ build environment for each batch, which a...

**a3-25** (composite 0.6750) -- D:0.75 S:0.65 R:0.70 C:0.60
> As more golden tests are added, the coverage gap between tested and untested code paths can widen rather than narrow because the total state space grows combinatorially with features while test count grows linearly.

**a1-22** (composite 0.6575) -- D:0.78 S:0.60 R:0.55 C:0.70
> The six missing requirements (V11 configuration management, V12 quantitative metrics, M9 full automation, M10 input domain characterization, F9 intermediate sensing variables, F10 behavior-not-structure testing) are directly supported by the source texts and are not merely nice-to-haves.

**a1-08** (composite 0.6550) -- D:0.80 S:0.72 R:0.60 C:0.50
> Vouk's stopping criterion is reliability-based (statistical estimation of remaining faults), not coverage-based; simply defining 'all golden tests pass' as a stopping criterion does not satisfy Vouk's requirement because passing tests says nothing about faults in untested paths.
> *Deltas:* compatibility -0.05 (via t-41)
> *Key adjudication (t-41):* The defense's pragmatic resolution is sound: a1-08 is aspirational, a2-09 is practical. However, a1-08 takes a compatibility hit because it requires something that is acknowledged as impractical. a2-0...

**a1-13** (composite 0.6525) -- D:0.70 S:0.88 R:0.55 C:0.48 **Axes below 0.50: compatibility.**
> MEASURE_DIVERGENCE reporting must be always-on for methodology compliance, not opt-in via environment variable, because any tolerance-based comparison introduces a gap and all measured disagreements should be reported regardless of whether they exceed the threshold.
> *Deltas:* defensibility -0.05 (via t-13); robustness -0.05 (via t-13); compatibility -0.02 (via t-18)
> *Key adjudication (t-13):* The defense's distinction between reporting and inspection is valid but thin. a1-13's actual claim is that always-on reporting is needed 'for methodology compliance,' which directly invokes the method...

**a3-09** (composite 0.6525) -- D:0.78 S:0.70 R:0.63 C:0.50
> The Rust port was developed by reading C++ source, violating Vouk's (1990) independence assumption for N-version testing, which means misunderstandings of C++ semantics propagate directly into the Rust implementation and are verified rather than detected by golden tests.
> *Deltas:* robustness -0.02 (via t-08)
> *Key adjudication (t-08):* The defense makes the key point: correlation of understanding implies correlation of correctness, not just of errors. The compound pessimism concern (severity 0.45) is valid but low-impact. a3-09 take...

**a1-14** (composite 0.6050) -- D:0.82 S:0.75 R:0.45 C:0.40 **Axes below 0.50: robustness, compatibility.**
> The full generate-execute-compare cycle must be automated without human intervention; a harness requiring manual steps (manually running the C++ generator, then manually running Rust tests, then manually comparing) violates McKeeman's method.
> *Deltas:* robustness -0.10 (via t-19,t-20); compatibility -0.10 (via t-19,t-20)
> *Key adjudication (t-19,t-20):* The defense's reinterpretation of 'full automation' as 'each step automated separately' is pragmatically reasonable but stretches a1-14's text, which says 'without human intervention.' The current wor...

## Contested (0 propositions)

No propositions remain contested. All 48 tensions were addressed through the prosecution/defense/adjudication process in Round 3.

## Fallen (0 propositions)

No propositions fell below the composite 0.50 threshold or had defensibility below 0.30. Even the most damaged propositions retained core validity.

## Tension Resolution Map

All 48 tensions from Round 2 were addressed in Round 3. Below is a summary of resolution outcomes, grouped by severity.

### High-Severity Tensions (>= 0.75)

**t-12** (severity 0.9, type: undermining)
- Propositions: a1-06, a3-04
- Issue: a1-06 forbids suppressing failures without documentation. a3-04 reveals that require_golden!() silently reports success when golden data is absent, achieving the same suppression effect as #[ignore] b...
- Resolution deltas: a1-06.robustness -0.05, a2-06.robustness -0.08, a3-04.defensibility +0.03
- Adjudication: The defense makes a philosophically compelling point: a1-06 and a2-06 correctly identify the risk pattern that a3-04 instantiates. However, the prosecution's point is narrower and more concrete: right now, the require_golden!() behavior exists and ne...

**t-04** (severity 0.88, type: contradiction)
- Propositions: a1-02, a3-03
- Issue: a1-02 requires every output channel to have a golden comparison. a3-03 identifies that the alpha channel is permanently excluded from image comparison, directly violating the exhaustive output compari...
- Resolution deltas: a1-02.robustness -0.10, a1-02.compatibility -0.10, a1-02.defensibility -0.03, a3-14.specificity +0.03
- Adjudication: The defense makes a valid distinction: a1-02 is about channels per test, not file coverage. However, the alpha exclusion (t-04) is still a significant violation of the exhaustiveness principle, even if documented. The documentation does not restore t...

**t-09** (severity 0.85, type: mutual_exclusion)
- Propositions: a1-05, a2-18
- Issue: a1-05 argues generated test inputs are essential to differential testing. a2-18 describes the oracle as binary snapshots from a pre-executed C++ generator. Generated inputs require re-executing the C+...
- Resolution deltas: a1-05.robustness -0.05, a1-05.compatibility -0.05, a2-18.compatibility -0.05
- Adjudication: The defense's batch generation argument is valid and reduces the severity of the mutual exclusion. However, the defense's resolution still requires a live C++ build environment for each batch, which a1-25 warns may become permanently unavailable. The...

**t-24** (severity 0.85, type: undermining)
- Propositions: a1-18, a3-07
- Issue: a1-18 requires that tolerances be tight enough that only the correct implementation passes. a3-07 documents that tolerances have ratcheted monotonically looser over time (up to ch_tol=3, max_fail=10%)...
- Resolution deltas: a1-18.robustness -0.08, a1-18.specificity -0.05, a3-07.defensibility +0.03, a3-08.defensibility +0.03
- Adjudication: The defense makes a valid philosophical point: a principle is not undermined by evidence of its violation. However, a1-18 is not just stating a principle -- it is claiming to be a requirement for the harness. A requirement that is comprehensively vio...

**t-02** (severity 0.82, type: undermining)
- Propositions: a1-01, a3-01
- Issue: a1-01 requires verifying that C++ and Rust receive the same inputs. a3-01 reveals the C++ generator uses StubClipboard/StubScreen/headless scheduler rather than production Eagle Mode, meaning even if ...
- Resolution deltas: a1-01.robustness -0.05, a1-01.compatibility -0.05, a2-18.robustness -0.05
- Adjudication: The defense successfully argues that input identity verification has independent value even within the bespoke generator context. However, a1-01's robustness takes a small hit because the proposition implicitly assumes input identity is sufficient fo...

**t-14** (severity 0.82, type: contradiction)
- Propositions: a1-10, a2-16
- Issue: a1-10 argues that 'blessing' Rust output as golden data provides zero fault detection power and is methodologically dangerous. a2-16 provides a structured mechanism for exactly this: marking failures ...
- Resolution deltas: a1-10.compatibility -0.05, a1-10.robustness -0.05
- Adjudication: The defense makes a strong case that a1-10 and a2-16 can be complementary. However, a1-10's language ('zero fault detection power,' 'methodologically dangerous') is absolutist in a way that does not accommodate a2-16's mechanism, even when used prope...

**t-07** (severity 0.8, type: undermining)
- Propositions: a1-03, a3-13
- Issue: a1-03 requires thorough investigation of each disagreement with root-cause classification. a3-13 identifies that LLMs are structurally incentivized to widen tolerances rather than investigate disagree...
- Resolution deltas: a1-03.robustness -0.10, a1-03.compatibility -0.05
- Adjudication: The defense correctly notes that a1-03 is not solely dependent on LLM capabilities and that human code review provides a backstop. However, the prosecution's structural observation is sound: if the dominant development methodology is LLM-assisted, th...

**t-25** (severity 0.8, type: undermining)
- Propositions: a1-18, a3-08
- Issue: a1-18 requires tests specific enough that only the correct implementation satisfies them. a3-08 identifies that the comparison function has no cap on how wrong individual failing pixels can be, meanin...
- Resolution deltas: a1-18.robustness -0.08, a1-18.specificity -0.05, a3-07.defensibility +0.03, a3-08.defensibility +0.03
- Adjudication: The defense makes a valid philosophical point: a principle is not undermined by evidence of its violation. However, a1-18 is not just stating a principle -- it is claiming to be a requirement for the harness. A requirement that is comprehensively vio...

**t-05** (severity 0.78, type: undermining)
- Propositions: a1-02, a3-14
- Issue: a1-02 demands exhaustive output comparison. a3-14 shows only ~15% of Rust source files are exercised by golden tests, meaning exhaustive comparison applies only to a small fraction of the codebase, se...
- Resolution deltas: a1-02.robustness -0.10, a1-02.compatibility -0.10, a1-02.defensibility -0.03, a3-14.specificity +0.03
- Adjudication: The defense makes a valid distinction: a1-02 is about channels per test, not file coverage. However, the alpha exclusion (t-04) is still a significant violation of the exhaustiveness principle, even if documented. The documentation does not restore t...

**t-31** (severity 0.78, type: undermining)
- Propositions: a2-02, a3-04
- Issue: a2-02 proposes a contract registry as the single source of truth for all golden tests. a3-04 reveals require_golden!() silently passes when data is absent. The contract registry cannot serve as truth ...
- Resolution deltas: a2-02.robustness -0.03
- Adjudication: The defense is largely correct: a2-02 and a2-03 together would fix the a3-04 problem. The remaining concern is that a2-02's contract registry design does not explicitly account for the require_golden!() silent-skip behavior, meaning the contract coul...

**t-01** (severity 0.75, type: undermining)
- Propositions: a1-01, a2-18
- Issue: a1-01 demands verification of input identity between C++ and Rust sides. a2-18 describes golden data as binary snapshots from gen_golden.cpp without mentioning input provenance metadata, meaning the h...
- Resolution deltas: a1-01.robustness -0.05, a1-01.compatibility -0.05, a2-18.robustness -0.05
- Adjudication: The defense successfully argues that input identity verification has independent value even within the bespoke generator context. However, a1-01's robustness takes a small hit because the proposition implicitly assumes input identity is sufficient fo...

**t-19** (severity 0.75, type: mutual_exclusion)
- Propositions: a1-14, a2-01
- Issue: a1-14 demands full automation of the generate-execute-compare cycle without human intervention. a2-01 proposes shell scripts and Unix pipes as the harness, which can automate the Rust side but cannot ...
- Resolution deltas: a1-14.robustness -0.10, a1-14.compatibility -0.10, a2-01.compatibility -0.03
- Adjudication: The defense's reinterpretation of 'full automation' as 'each step automated separately' is pragmatically reasonable but stretches a1-14's text, which says 'without human intervention.' The current workflow requires a human to decide when to regenerat...

**t-35** (severity 0.75, type: undermining)
- Propositions: a2-05, a3-08
- Issue: a2-05 defines 'suspicious' tests based on proximity to failure thresholds (50% of max_failure_pct, 75% of channel_tolerance). a3-08 identifies that the comparison lacks a magnitude cap on failing pixe...
- Resolution deltas: a2-05.robustness -0.05
- Adjudication: The defense correctly identifies that a2-05 operates on the metrics it receives and cannot compensate for upstream gaps. However, a2-05's design claims to detect suspicious patterns in test results, and the magnitude blindness means it misses a real ...

### Medium-Severity Tensions (0.50-0.74)

| Tension | Severity | Type | Propositions | Net Deltas |
|---------|----------|------|--------------|------------|
| t-26 | 0.73 | undermining | a1-19, a3-12 | a1-19.robustness-0.03 |
| t-17 | 0.72 | undermining | a1-12, a3-15 | a1-12.robustness-0.05, a3-15.defensibility+0.02 |
| t-27 | 0.72 | undermining | a1-20, a3-14 | a1-20.robustness-0.05, a1-20.compatibility-0.02 |
| t-32 | 0.72 | mutual_exclusion | a2-03, a3-04 | a2-02.robustness-0.03 |
| t-41 | 0.72 | contradiction | a1-08, a2-09 | a1-08.compatibility-0.05, a2-09.defensibility-0.02 |
| t-13 | 0.7 | contradiction | a1-07, a1-13 | a1-07.compatibility-0.05, a1-13.defensibility-0.05, a1-13.robustness-0.05 |
| t-20 | 0.7 | undermining | a1-14, a3-21 | a1-14.robustness-0.10, a1-14.compatibility-0.10, a2-01.compatibility-0.03 |
| t-40 | 0.7 | latent_tension | a2-06, a3-04 | a1-06.robustness-0.05, a2-06.robustness-0.08, a3-04.defensibility+0.03 |
| t-34 | 0.68 | latent_tension | a2-04, a3-13 | a2-04.robustness-0.05 |
| t-47 | 0.68 | undermining | a1-18, a3-24 | a1-18.robustness-0.08, a1-18.specificity-0.05, a3-07.defensibility+0.03, a3-08.defensibility+0.03 |
| t-15 | 0.65 | latent_tension | a1-10, a3-17 | a1-10.compatibility-0.02 |
| t-18 | 0.65 | undermining | a1-13, a2-05 | a1-13.compatibility-0.02 |
| t-33 | 0.62 | latent_tension | a2-04, a3-07 | a2-04.robustness-0.05 |
| t-39 | 0.62 | latent_tension | a2-16, a3-17 | a2-16.robustness-0.02 |
| t-03 | 0.6 | latent_tension | a1-02, a2-17 | a1-02.robustness-0.10, a1-02.compatibility-0.10, a1-02.defensibility-0.03, a3-14.specificity+0.03 |
| t-30 | 0.6 | undermining | a1-25, a2-22 | a2-22.robustness-0.02 |
| t-21 | 0.58 | latent_tension | a1-15, a3-11 | a1-15.robustness-0.02, a3-11.specificity+0.02 |
| t-36 | 0.58 | undermining | a2-09, a3-25 | a2-09.robustness-0.02, a2-09.defensibility-0.01 |
| t-06 | 0.55 | latent_tension | a1-03, a2-19 | a1-03.specificity-0.02, a2-19.specificity-0.02 |
| t-16 | 0.55 | latent_tension | a1-11, a3-02 | a1-11.robustness-0.02 |
| t-29 | 0.55 | latent_tension | a1-23, a2-18 | a2-18.specificity-0.02 |
| t-38 | 0.55 | undermining | a2-15, a3-19 | a2-15.robustness-0.02 |
| t-42 | 0.55 | latent_tension | a1-09, a3-21 | none |
| t-44 | 0.55 | undermining | a2-14, a3-20 | a2-14.robustness-0.02 |
| t-37 | 0.52 | latent_tension | a2-10, a3-18 | a2-10.robustness-0.02 |
| t-48 | 0.52 | latent_tension | a2-12, a3-22 | a2-12.robustness-0.02 |
| t-10 | 0.5 | latent_tension | a1-05, a3-10 | a1-05.specificity-0.02 |
| t-22 | 0.5 | latent_tension | a1-16, a2-16 | none |
| t-43 | 0.5 | latent_tension | a1-24, a2-15 | a2-15.specificity-0.01 |

### Low-Severity Tensions (< 0.50)

| Tension | Severity | Type | Propositions | Net Deltas |
|---------|----------|------|--------------|------------|
| t-23 | 0.48 | latent_tension | a1-17, a2-02 | a2-02.specificity-0.01 |
| t-08 | 0.45 | latent_tension | a1-04, a3-09 | a3-09.robustness-0.02 |
| t-28 | 0.45 | latent_tension | a1-21, a2-08 | a2-08.robustness-0.01 |
| t-46 | 0.45 | latent_tension | a2-07, a3-14 | a2-07.robustness-0.01 |
| t-45 | 0.42 | latent_tension | a1-12, a2-05 | none |
| t-11 | 0.3 | latent_tension | a1-06, a2-13 | a2-13.robustness-0.02 |

## Key Takeaways

### What Survived the Dialectic

The strongest survivors cluster around three themes:

1. **Concrete defect identification in the current harness** -- The highest-scoring propositions are observations about specific, verifiable flaws:
   - a3-04 (0.9075): `require_golden!()` silently passes when golden data is absent. This is the single most robust finding: a mechanical defect that defeats CI verification.
   - a3-15 (0.9050): `MEASURE_DIVERGENCE` exists as a diagnostic tool but lacks storage/comparison/alerting infrastructure to function as a monitoring system.
   - a3-22 (0.8575): `translate_cpp_notice_flags` contains 10 manual bit-position mappings that are unverified single points of trust.
   - a3-08 (0.8525): The image comparison has no magnitude cap on failing pixels, allowing catastrophically wrong scanlines to pass.

2. **Structural safeguards with clear implementation paths** -- Well-defined mechanisms that can be built:
   - a2-07 (0.8750): Correspondence auditor enforcing file-to-header mapping. Already partially implemented.
   - a2-13 (0.8700): Self-conformance checks for `#[ignore]` and undocumented tolerance overrides.
   - a2-03 (0.8675): Bidirectional completeness checking between contract registry and golden files.
   - a2-17 (0.8700): 8-domain output verification with appropriate tolerance regimes.

3. **Methodological principles with broad compatibility** -- Foundational requirements that align with practical constraints:
   - a1-19 (0.8675): Document methodology deviations at the point of divergence. The meta-requirement enabling principled pragmatism.
   - a1-15 (0.8575): Golden tests must test observable behavior, not implementation artifacts.
   - a1-11 (0.8500): Record exact compiler versions and source revisions for golden data traceability.

### What Was Wounded

The 17 wounded propositions fall into recognizable patterns:

1. **Methodological requirements that collide with the frozen oracle model:**
   - a1-14 (0.6050, lowest composite): Full automation of generate-execute-compare. Robustness (0.45) and compatibility (0.40) both fell below 0.50. The frozen oracle structurally cannot support McKeeman's live differential cycle.
   - a1-05 (0.6875): Generated inputs as essential. Architecturally incompatible with frozen oracle without major infrastructure.
   - a1-08 (0.6550): Reliability-based stopping criterion. Theoretically correct but practically impossible for this project.

2. **Propositions with internal contradictions or compound pessimism:**
   - a1-13 (0.6525): Always-on divergence reporting. Internally contradicted by a1-07 (sub-tolerance is a design choice, not a requirement).
   - a3-09 (0.6525): Failure independence violation makes golden tests 'verify rather than detect.' Sound observation but framed too pessimistically.
   - a1-07 (0.7300): Sub-tolerance treatment is not addressed by sources. Compatibility (0.50) hit the floor due to tension with monitoring proposals.

3. **Requirements too abstract or aspirational to enforce:**
   - a1-22 (0.6575): Bundle claim about six missing requirements. Too abstract to survive as a single proposition.
   - a1-02 (0.7125): Exhaustive output comparison. Robustness fell to 0.60 under the weight of the alpha exclusion and 85% file coverage gap.
   - a1-18 (0.6950): Tight tolerances for test specificity. Does not define 'tight enough,' which tolerance ratcheting exploits.

### What Fell

No propositions fell entirely, which indicates the initial proposal quality was high across all three agents. Even the most damaged proposition (a1-14 at 0.6050) retains defensibility (0.82) -- its principle is correct even if its implementation demand is impractical.

### Implications for Harness Design

The dialectic reveals a clear priority stack for harness work:

1. **Fix `require_golden!()` immediately** (a3-04). This is the single most impactful defect: the entire golden test suite can silently report green while running zero actual comparisons. Until this is fixed, all other harness improvements are undermined.

2. **Add a max-diff cap to image comparison** (a3-08). The current comparison allows catastrophically wrong pixels to pass as long as they are few. Adding a per-pixel magnitude cap closes a structural blind spot.

3. **Connect MEASURE_DIVERGENCE to CI** (a3-15, a1-12). The divergence measurement infrastructure exists but produces ephemeral output. Storing JSONL results and comparing across runs converts a diagnostic tool into a regression detection system.

4. **Implement the contract registry with bidirectional checking** (a2-02, a2-03). This provides the structural foundation for all other harness components and directly addresses the silent skip problem.

5. **Add tolerance provenance tracking** (a3-07, a3-12, a2-04). Tolerances have ratcheted looser without documentation. A tolerance registry with mandatory reason strings and a one-way ratchet prevents further erosion.

6. **Verify `translate_cpp_notice_flags` independently** (a3-22). The 10 manual bit-position mappings are a single point of unverified trust. Auto-generating from shared definitions or adding independent validation closes this gap.

7. **Accept the frozen oracle's limitations explicitly** (a1-23, a1-09, a3-21). The frozen oracle is a valid practical adaptation but structurally weaker than live differential testing. Document this at the architecture level rather than pretending the gap does not exist.

The harness does not need to be perfect to be useful. The dialectic shows that fixing the top 3 items (silent skip, magnitude cap, divergence tracking) would transform the golden test suite from a suite that *can silently lie* into one that provides *meaningful, tracked, gated verification* -- even within the acknowledged limitations of a frozen oracle.
