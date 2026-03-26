# Verification Methodology: Behavioral Equivalence of the zuicchini Port

This document defines the governing methodology for verifying that zuicchini
(Rust) is behaviorally equivalent to Eagle Mode's emCore (C++). It is a
composite framework drawn from three published sources, adapted to the
specific constraints of this project.

---

## 1. Governing Sources

| Source | Citation | Role in this methodology |
|--------|----------|--------------------------|
| **Back-to-Back Testing** | Vouk, M.A. "Back-to-back testing." *Information and Software Technology* 32(1), 34–45, 1990. DOI: 10.1016/0950-5849(90)90044-R | Defines the verification method, its detection limits, and its cost model. Explicitly covers interlanguage conversion as a use case. |
| **Differential Testing** | McKeeman, W.M. "Differential Testing for Software." *Digital Technical Journal* 10(1), 100–107, 1998. | Defines the oracle principle (another implementation *is* the oracle), test quality criteria, and test reduction. |
| **Characterization Testing** | Feathers, M.C. *Working Effectively with Legacy Code*, Ch. 13. Prentice Hall, 2004. ISBN 0-13-117705-2. | Defines what golden master tests verify (actual behavior, not specified behavior), and the heuristic for writing them. |

Local copies of all three sources are in `~/doc/`.

---

## 2. Definitions

**Reference implementation**: Eagle Mode emCore 0.96.4, C++. The reference
remains the source of truth. The port does not define correctness; the
reference does.

**Port**: zuicchini, Rust. The system under verification.

**Oracle**: Pre-captured output from the reference implementation, stored as
binary golden files in `tests/golden/data/`. The oracle is a frozen snapshot —
it substitutes for live execution of the C++ reference. Vouk describes this
configuration as the two-version case (N=2) where "a starting version is
always available" and the testing is applied "after [the code has] been
ported between two languages or language dialects" (p. 36, Figure 1b
caption).

**Disagreement**: Any difference between port output and oracle output that
exceeds the defined tolerance for that comparison domain. McKeeman: "If (we
might say when) the results differ or one of the systems loops indefinitely
or crashes, the tester has a candidate for a bug-exposing test" (p. 100).
A disagreement is a defect in the port unless proven otherwise.

**Coincident failure**: A defect present in both the reference and the port,
invisible to back-to-back comparison because both produce the same wrong
output. Vouk: "if there is an identical fault in all versions because
versions are copies of each other, all programmers have made exactly the
same mistake, or the output space is binary... the response of functionally
equivalent programs to similar faults may be identical or different.
Dissimilar faults with different components may cause identical and wrong
answers" (p. 38). This is the *acknowledged unverifiable residual* of the
methodology.

**Characterization test**: Feathers: "A characterization test is a test that
characterizes the actual behavior of a piece of code. There's no 'Well, it
should do this' or 'I think it does that.' The tests document the actual
current behavior of the system" (Ch. 13). Characterization tests "don't
have any moral authority; they just sit there documenting what pieces of the
system really do" (Ch. 13).

**Verification domain**: A category of observable output with its own
comparison function, tolerance parameters, and coverage target.

---

## 3. Verification Domains

Each domain has a comparison function (in `tests/golden/common.rs`), a
tolerance specification, and a pass/fail criterion.

### 3.1 Pixel Output

**What it verifies**: Rendering correctness — the RGB output of painting
operations (rects, ellipses, gradients, lines, text, compositing, transforms).

**Comparison**: `compare_images()` — per-pixel, per-channel absolute
difference on RGB. Alpha excluded (documented divergence: C++ channel-3
semantics differ from standard compositing alpha).

**Tolerance parameters**:
- `channel_tolerance`: max per-channel absolute diff allowed per pixel (u8).
- `max_failure_pct`: max percentage of pixels that may exceed tolerance.

**Pass criterion**: `failing_pixels / total_pixels * 100 <= max_failure_pct`,
where a pixel fails if any RGB channel diff exceeds `channel_tolerance`.

**Coverage target**: Every public `emPainter` draw method, every compositor
blend mode, and every transform variant must have at least one golden test.
Per McKeeman's test quality criterion, tests must "exercise each conversion
along the path" (p. 106) — specifically, the integer-to-u8 truncation
points in the compositing pipeline.

**Current inventory**: 42 painter goldens, 61 compositor goldens.

### 3.2 Layout Geometry

**What it verifies**: Layout algorithms produce identical child rectangles
given identical inputs (panel count, weights, tallness, spacing, alignment).

**Comparison**: `compare_rects()` — per-rect `(x, y, w, h)` with f64 epsilon.

**Tolerance**: Configurable `eps` (typically 1e-9).

**Pass criterion**: All four coordinates of every rect within epsilon.

**Coverage target**: Every layout type (linear, raster, pack) × every
configuration axis (weights, tallness, spacing, alignment, min/max cell
count, adaptive orientation).

**Current inventory**: 31 layout goldens.

### 3.3 Behavioral State

**What it verifies**: Panel activation, active-path propagation, and focus
traversal produce identical state trees after identical input sequences.

**Comparison**: `compare_behavioral()` — exact match of `(is_active,
in_active_path)` per panel in DFS order.

**Tolerance**: None. Exact match required.

**Coverage target**: Every activation trigger (click, keyboard, programmatic),
focus traversal (tab forward/backward), and edge case (remove active panel,
non-focusable panels).

**Current inventory**: 25 behavioral goldens.

### 3.4 Notice Propagation

**What it verifies**: The notice/signal system delivers identical flag sets
to identical panel trees after identical mutations.

**Comparison**: `compare_notices()` — bitwise comparison of translated C++
flags against Rust NoticeFlags, with configurable bit mask.

**Tolerance**: None. Exact match on masked bits.

**Coverage target**: Every notice flag type, propagation through parent
chains, coalescing of multiple notices.

**Current inventory**: 13 notice goldens.

### 3.5 Input Routing

**What it verifies**: Mouse/keyboard input reaches the correct panel and
produces correct activation side effects.

**Comparison**: `compare_input()` — exact match of `(received_input,
is_active, in_active_path)` per panel.

**Tolerance**: None. Exact match.

**Current inventory**: 6 input goldens.

### 3.6 Trajectory / Animation

**What it verifies**: View animation and input filter velocity calculations
produce equivalent motion trajectories.

**Comparison**: `compare_trajectory()` — per-step `(vel_x, vel_y, vel_z)`
with f64 tolerance.

**Tolerance**: Configurable (typically 1e-6 to 1e-2, depending on whether
the path feeds further integration).

**Pass criterion**: All three velocity components of every step within
tolerance.

**Current inventory**: 20 trajectory goldens.

### 3.7 Widget State

**What it verifies**: Widget-level state machines (e.g., button press/release,
checkbox toggle) produce identical observable state after identical input
sequences.

**Current inventory**: 16 widget state goldens.

---

## 4. Verification Boundary

The methodology explicitly partitions all code into three zones based on
what each source says about the limits of its technique.

### 4.1 Zone A — Verifiable by Back-to-Back Testing

Code whose output feeds a golden test oracle. Equivalence is *demonstrated*
by disagreement count = 0 within defined tolerances.

Vouk: "All programs are tested with the same input data, and outputs of all
possible program pair combinations are compared. When a difference is
observed the problem is thoroughly investigated" (p. 34).

Includes:
- All pixel rendering paths (painter, compositor, scanline)
- All layout algorithms (linear, raster, pack)
- All behavioral state transitions (activation, focus)
- All notice flag propagation
- All input routing decisions
- All animation/trajectory calculations

**Acceptance**: Zero disagreements across all golden tests. Measured by
`MEASURE_DIVERGENCE=1 cargo test --test golden -- --test-threads=1`.

### 4.2 Zone B — Verifiable by Structural Inspection

Code whose correctness cannot be observed via output comparison, but whose
structure can be verified against the reference by code review.

Vouk explicitly states that back-to-back testing should be "combined with
other development and testing techniques such as specification and code
inspection" (p. 44). Feathers distinguishes *characterization* (documenting
what code does) from *targeted testing* (verifying specific change paths),
and notes that characterization tests are complementary to manual review,
not a replacement for it.

Includes:
- Memory management patterns (Rc/RefCell lifecycle, drop ordering)
- Error handling paths (error enums, propagation)
- API surface compatibility (method signatures, visibility)
- File and name correspondence (enforced by CLAUDE.md rules)

**Acceptance**: Code review confirms 1:1 structural correspondence with
the reference. Deviations are marked with `DIVERGED:` comments per the
project's File and Name Correspondence rules.

### 4.3 Zone C — Unverifiable (Coincident Failure Residual)

Behaviors where both implementations could be wrong in the same way,
invisible to differential comparison.

Vouk: "First, understand that only a small fraction of the faults that
occur among functionally equivalent programs appear to be highly correlated.
The percentage reported ranges from 0 to about 15%" (p. 38). However,
"when the correlation is 100% and the underlying fault is common to all
versions, the fault cannot be detected by back-to-back testing at all"
(p. 38).

Includes:
- Shared algorithmic assumptions inherited from the C++ design
- Numeric edge cases where both C++ and Rust overflow/underflow identically
- Behaviors that depend on unspecified/undefined C++ behavior that happens
  to work on the reference platform but is not guaranteed
- Platform-specific behavior (endianness, floating-point mode)

**Acceptance**: Explicitly documented. No testing methodology eliminates
coincident failures. The residual risk is accepted and recorded.

---

## 5. Coverage Metrics

### 5.1 Detection Efficiency (Back-to-Back)

Vouk models the probability of detecting a failure with T random test
cases across N functionally equivalent versions, assuming failure
independence:

> P_D(T) = 1 − (1 − p)^(N·T)    (equation 1, p. 37)

where p is the per-version failure probability. For N=2 (our case — C++
oracle + Rust port), the detection efficiency is bounded: "the contribution
of each additional version to the failure detection capability of a
N-version system is considerably reduced for N-tuple sizes larger than 4
or 5" (p. 37). With only two versions, maximizing T (test case count) is
the primary lever.

Vouk also models **diminishing returns**: "reliability growth shifts the
operational efficiency profile to smaller p values so that, as reliability
grows, more test cases are needed to achieve the same probability of
detecting a failure" (p. 37). This means early golden tests have the
highest detection value; additional tests for already-well-tested code
have declining marginal value.

**Practical implication**: Prioritize golden test coverage breadth (covering
more code paths) over depth (more inputs to already-covered paths).

### 5.2 Oracle Coverage

**Definition**: The percentage of the port's source files that have at least
one verification domain exercised by golden test data.

**Measurement**: For each `.rs` file in `src/emCore/`, determine whether any
golden test exercises code in that file. Files marked `.no_rust_equivalent`
are excluded (they map to C++ headers replaced by Rust stdlib types). Files
marked `.rust_only` are excluded from oracle coverage (they have no C++
reference to compare against) but must have unit tests.

**Target**: 100% of files that implement C++ reference functionality must be
exercised by at least one golden test *or* be classified as Zone B
(structural inspection only).

### 5.3 Structural Coverage

**Definition**: The percentage of the port's Rust code (statements, branches,
functions) that is exercised by the combined golden + unit test suite.

**Measurement**: `cargo-llvm-cov` or equivalent, reporting:
- Function coverage %
- Line coverage %
- Branch coverage % (where tooling supports it)

McKeeman notes that "the simplest measure of completeness is statement
coverage... Obviously, code that is not executed was not tested" (p. 100).
Statement coverage is necessary but not sufficient — it identifies untested
code but does not guarantee that tested code is correct.

**Target**: Line coverage ≥ 85% across the `src/emCore/` tree. Functions
below 50% individual line coverage are flagged for review.

### 5.4 Correspondence Coverage

**Definition**: The percentage of C++ header files that have a corresponding
Rust file (either `.rs` implementation or `.no_rust_equivalent` marker).

**Measurement**: For each `.h` file in the C++ `include/emCore/`, verify
that `src/emCore/` contains either `emFoo.rs` or `emFoo.no_rust_equivalent`.

**Target**: 100%. Any C++ header without a Rust counterpart or marker is
a gap in the port.

**Current state**: 90 C++ headers, 100 Rust files, 15 `.no_rust_equivalent`
markers, 5 `.rust_only` markers.

### 5.5 When to Stop

Vouk: "the testing can be stopped when the target reliability... is
estimated to have been reached. The target is usually either some failure
intensity or some number of observed failures" (p. 36).

For this project, "target reliability" is operationalized as:
1. Zero disagreements across all golden tests (Zone A gate).
2. Correspondence coverage = 100% (no unmapped C++ headers).
3. Line coverage ≥ 85% (structural coverage gate).
4. All Zone B files reviewed and signed off.
5. Zone C risks documented.

When all five conditions hold simultaneously, the port is *sufficiently
verified* under this methodology.

---

## 6. Oracle Management

### 6.1 Generation

Golden data is generated by compiling and running the C++ reference
implementation via `make -C tests/golden/gen && make -C tests/golden/gen run`.
The generator links against emCore and produces binary golden files in a
defined format (see `tests/golden/gen/golden_format.h`).

### 6.2 Frozen Oracle Principle

Once generated, golden data is committed to the repository and treated as
immutable. The oracle is *not* regenerated unless:

1. A bug is found in the C++ generator itself (not in the port), or
2. A new verification scenario is added, requiring new golden data, or
3. The C++ reference version is updated.

Regeneration requires re-running the C++ generator and committing the new
data with a commit message explaining why.

Feathers, on discovering unexpected behavior during characterization: "it
pays to get some clarification. It could be a bug. That doesn't mean that
we don't include the test in our test suite; instead, we should mark it as
suspicious and find out what the effect would be of fixing it" (Ch. 13).
The same principle applies to golden data — if the oracle output looks
wrong, investigate the C++ reference before assuming the oracle is correct.

### 6.3 Tolerance Rationale

Every tolerance parameter must have a documented rationale:

- `channel_tolerance = 0`: C++ integer arithmetic reproduced exactly.
- `channel_tolerance = 1`: Rounding difference in a single `>> 8` vs
  `div255_round()` step, verified to be ≤ 1 by analysis.
- `eps = 1e-9`: f64 accumulation difference over N operations, verified
  to be within machine epsilon bounds.
- `trajectory_tolerance = 1e-2`: Integration step accumulates floating-point
  drift over 100+ steps; tolerance bounds the maximum per-step drift.

Tolerances must not be widened to make a failing test pass without root-cause
analysis. If a test fails, the first response is to fix the port, not widen
the tolerance.

McKeeman's test quality principle applies here: tests should "exercise each
conversion along the path" (p. 106). A widened tolerance may mask a type
conversion error (e.g., silent truncation from f64 to i32) that would
otherwise surface as a disagreement.

---

## 7. Verification Process

### Phase 1: Correspondence Audit

For each C++ header in `include/emCore/`:
1. Verify a corresponding Rust file or `.no_rust_equivalent` marker exists.
2. For each public type and method in the header, verify the Rust file
   contains a correspondingly named item (or a `DIVERGED:` annotation).
3. Record any gaps.

**Gate**: Correspondence coverage = 100%.

### Phase 2: Golden Test Execution

Run the full golden test suite:
```bash
MEASURE_DIVERGENCE=1 cargo test --test golden -- --test-threads=1
```

**Gate**: Zero disagreements. Every JSONL record has `"pass":true`.

### Phase 3: Structural Coverage Measurement

Run the test suite under coverage instrumentation:
```bash
cargo llvm-cov --test golden --test '*' --html
```

**Gate**: Line coverage ≥ 85% for `src/emCore/`. Functions below 50%
are reviewed and either:
- Additional golden tests are written, or
- The function is classified as Zone B (structural inspection), or
- A justification is recorded (e.g., platform-specific code path not
  exercisable in test environment).

### Phase 4: Zone B Review

For each file not fully covered by golden tests:
1. Manual review confirms structural correspondence with C++ reference.
2. `DIVERGED:` annotations are verified to be accurate and justified.
3. Error handling paths are reviewed for behavioral compatibility.

Feathers' targeted testing heuristic (Ch. 13) applies here:
1. Write tests for the area where you will make your changes.
2. Take a look at the specific things you are going to change, and
   attempt to write tests for those.
3. If extracting or moving functionality, verify the existence and
   connection of those behaviors on a case-by-case basis.

### Phase 5: Zone C Documentation

Document known coincident failure risks:
- List shared algorithmic assumptions.
- List any C++ undefined behavior relied upon by the reference.
- List platform-specific assumptions (endianness, float mode, integer size).

---

## 8. Ongoing Verification

### When Adding New Port Code

1. Identify which verification domain(s) the new code falls in.
2. If Zone A: write golden test(s) before or alongside the port. Generate
   C++ golden data if new scenarios are needed.
3. If Zone B: ensure structural correspondence and add `DIVERGED:`
   annotations for any deviations.
4. Run full golden suite. Zero disagreements required.

### When a Golden Test Fails

1. **Do not widen tolerance.** Investigate the disagreement.
2. Determine whether the failure is in the port (fix the Rust code) or
   in the oracle (regenerate golden data with justification).
3. If the failure reveals a coincident failure risk, document it in Zone C.

Feathers: "When you find bugs... if the system has been deployed, you need
to examine the possibility that someone is depending on that behavior,
even though you see it as a bug" (Ch. 13). For this project, the C++
reference *is* deployed — its behavior is the specification. A golden test
failure means the port diverges, not that the reference is wrong, unless
independently confirmed.

### Divergence Measurement

The `MEASURE_DIVERGENCE=1` and `DIVERGENCE_LOG=<path>` environment variables
produce JSONL records for every golden comparison. These records enable
tracking of divergence trends over time:

```json
{"test":"rect_solid","tol":0,"fail":0,"total":65536,"pct":0.0000,"max_diff":0,"pass":true}
```

Aggregate metrics:
- Total golden tests passing / total golden tests
- Maximum `max_diff` across all pixel tests
- Maximum `pct` across all pixel tests
- Any test with `"pass":false`

---

## 9. Relationship to CLAUDE.md Rules

This methodology is the formal basis for several rules in `CLAUDE.md`:

| CLAUDE.md Rule | Methodology Basis |
|----------------|-------------------|
| "Reproduce C++ integer formulas exactly" in pixel arithmetic | Zone A pixel verification requires exact match (tolerance=0); McKeeman's conversion-exercise principle |
| "Same algorithm and operation order on golden-tested paths" | Back-to-back testing detects any algorithmic deviation (Vouk) |
| "Check if the function's output feeds a golden test" | Zone A/B boundary determination |
| File and Name Correspondence | Correspondence coverage (§5.4) |
| `DIVERGED:` / `SPLIT:` annotations | Zone B structural inspection |
| "Fix the cause, not `#[allow]`" | Tolerance discipline (§6.3) |

---

## 10. Limitations

1. **No live cross-execution**: The C++ reference is not executed during
   Rust test runs. The oracle is frozen golden data. If the C++ reference
   has bugs, they are frozen into the oracle. (Mitigated by: the reference
   is a shipped, tested product.) Vouk: "back-to-back testing should not
   be used in isolation. To compensate for the possibility that a fault
   induces identical and wrong responses from all versions, it must be
   combined with other development and testing techniques" (p. 44).

2. **Coverage of internal state**: Back-to-back testing verifies observable
   output, not internal state. Two implementations can have different
   internal representations and still pass. This is acceptable — behavioral
   equivalence does not require structural identity.

3. **Coincident failures**: Undetectable by definition. Vouk estimates that
   "only a small fraction of the faults that occur among functionally
   equivalent programs appear to be highly correlated. The percentage
   reported ranges from 0 to about 15%" (p. 38). Accepted as residual risk.

4. **Input space coverage**: The golden tests cover a finite set of inputs.
   Untested inputs may reveal disagreements. McKeeman: "Testing is always
   incomplete. The simplest measure of completeness is statement coverage
   ... Obviously, code that is not executed was not tested" (p. 100).
   Mitigated by: structural coverage measurement identifies code paths not
   exercised by golden tests.

5. **Non-determinism**: Any source of non-determinism (uninitialized memory,
   hash map ordering, thread scheduling) in either implementation can cause
   false disagreements. Mitigated by: both implementations are single-threaded
   UI trees with deterministic execution.

6. **Two-version detection ceiling**: With N=2, the failure detection
   probability is P_D(T) = 1 − (1−p)^(2T) (Vouk eq. 1). This is
   substantially less effective than N≥3 systems at detecting faults with
   low p values. Mitigated by: maximizing T (golden test count) and
   supplementing with Zone B structural inspection.
