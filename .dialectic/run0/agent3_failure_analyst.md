# Failure Analysis: zuicchini Golden Verification Harness

## Preamble

This document catalogs failure modes of the verification harness used to prove behavioral equivalence between Eagle Mode's emCore (C++) and zuicchini (Rust). The harness relies on frozen golden data as its oracle, compared against live Rust output. The analysis covers oracle integrity, coverage illusions, tolerance dynamics, self-conformance limits, methodology gaps, LLM-specific risks, completeness illusions, regression blindness, oracle decay, and composition failures.

---

## 1. Oracle Integrity Failures

### 1.1 The Generator Is Itself Unverified Code

The C++ golden data generator (`gen_golden.cpp`) is approximately 1500+ lines of bespoke test infrastructure code that constructs scenes, exercises emCore subsystems, and serializes output. This generator is *not* the production C++ code -- it is a test harness that calls into production code, but the harness itself makes choices about scene setup, initialization order, and parameter values. If `gen_golden.cpp` initializes an `emPainter` differently than production Eagle Mode does (e.g., the `StubClipboard`, the `StubScreen` elided from the build, the headless `emStandardScheduler` with no actual event loop), the golden data faithfully records behavior of a C++ configuration that has never actually run in production.

The compounding mechanism: every Rust test that passes against this golden data is actually proving equivalence with the generator's configuration, not with Eagle Mode itself. If Eagle Mode's actual runtime initialization differs -- a different default canvas color, a different scheduler tick count before settling, a different clipboard interaction -- the golden data is an authoritative record of a system that does not exist in production. This divergence is invisible because nobody runs Eagle Mode in the exact configuration the generator uses, and nobody compares the generator's output against a running Eagle Mode instance.

### 1.2 Binary Format as Implicit Contract

The golden file format (`[u32 width][u32 height][RGBA bytes]` for painter, `[u32 count][N*32 bytes]` for layout, etc.) is an undocumented implicit contract between `gen_golden.cpp` and `common.rs`. The format is defined in `golden_format.h` and independently re-implemented in Rust. There is no schema version, no magic number, and no checksum. If either side's serialization logic has an endianness assumption (both assume little-endian), a platform change silently corrupts every golden file. More insidiously, if the C++ generator is rebuilt with a different compiler that pads a struct differently or reorders fields, the binary output changes without any compile error, and the Rust loader reads garbage that happens to have the right byte count. The `assert_eq!(data.len(), expected_len)` guard catches size mismatches but not content corruption where sizes happen to match.

### 1.3 Semantic Alpha Channel Divergence Baked Into the Oracle

The `compare_images` function explicitly skips channel 3 (alpha) with the comment: "C++ emPainter uses channel 3 to track remaining canvas visibility (not standard compositing alpha), while the Rust painter stores standard alpha." This means the golden data contains C++ alpha values that are semantically different from what the Rust side produces, and the harness simply ignores this channel. But alpha feeds into downstream operations -- compositing, blending, overdraw tracking. If any future code path reads the alpha channel for logic (not just for comparison), the Rust and C++ implementations diverge silently in a dimension the harness has structurally decided not to observe. The harness cannot detect alpha-dependent behavioral divergence because it has permanently excluded alpha from its observation space.

---

## 2. Coverage Theater

### 2.1 The `require_golden!` Silent Skip Pattern

Every golden test begins with `require_golden!()`, which checks if the `tests/golden/data/` directory exists. If it does not, the test prints "SKIP" to stderr and returns successfully. This means a CI pipeline without generated golden data reports 232 golden test invocations as passing. The test output says "SKIP" on stderr, but the test runner reports "ok." A CI system that checks exit codes will see 100% pass rate. A developer who runs `cargo test` without first running `make -C golden_gen run` gets green across the board. There is no mechanism that distinguishes "tested and passed" from "skipped because oracle missing" in the test runner's summary output.

The compounding mechanism: as the project grows, new developers, new CI environments, and new sessions all inherit this ambiguity. A "232 tests passed" report can mean "232 golden comparisons verified" or "232 tests skipped." The confidence derived from the number is identical in both cases.

### 2.2 Painter Tests Cover Drawing Primitives, Not Drawing Compositions

The painter golden tests exercise individual primitives: `rect_solid`, `ellipse_basic`, `polygon_tri`, `line_basic`, etc. These are the equivalent of testing that addition works, multiplication works, and subtraction works -- individually. But production rendering is *composition* of primitives: a border widget calls `PaintRect` then `PaintEllipse` then `PaintTextLayout` within the same painter context, accumulating state (clip rects, transforms, canvas color). The `multi_compose` test exists but is a single handcrafted scenario, not a systematic exploration of composition. The primitive tests create high coverage numbers (36 painter goldens) while leaving the composition space -- which is combinatorially larger -- almost entirely unobserved.

### 2.3 Widget Tests Are Constrained to a Single Viewport Configuration

Every widget golden test uses the same viewport: 800x600 pixels, tallness 0.75, `NO_ACTIVE_HIGHLIGHT` flag. Widgets in Eagle Mode render differently at different zoom levels (the `ViewConditionType` system controls this). A widget that renders correctly at the golden test's zoom level but clips incorrectly at 2x zoom or renders nothing at 0.1x zoom will pass all golden tests. The harness tests one point in a continuous parameter space and treats it as if it covers the space.

### 2.4 Layout Test Monotonicity

All 31 layout golden tests use `compare_rects` with `eps = 1e-6`. This uniform tolerance obscures the fact that some layout algorithms produce coordinates at vastly different scales. A `pack_extreme` layout might produce rects with widths of `0.001`, where a `1e-6` tolerance means the result can be 0.1% wrong. A `linear_h_equal` layout produces rects with widths of `0.333...`, where `1e-6` is 0.0003% -- three orders of magnitude tighter relative tolerance. The uniform epsilon creates the illusion of uniform precision across tests that have wildly different sensitivity requirements.

---

## 3. Tolerance Creep

### 3.1 The Tolerance Ratchet

Examining the actual tolerance values used across golden tests reveals a clear progression of loosening:

- Basic painter primitives: `(ch_tol=1, max_fail=0.1%)` -- the tightest
- Ellipses, polygons: `(ch_tol=1, max_fail=0.5%)`
- Line ends: `(ch_tol=1, max_fail=1.0%)`
- Compositor tests: `(ch_tol=1, max_fail=0.5%)`
- Widget rendering: `(ch_tol=3, max_fail=3.5%)` to `(ch_tol=3, max_fail=10.0%)` for tunnel
- Trajectory (animator): `1e-6` down to `1e-2` for magnetic animator
- Input filter: `1e-6` down to `1e-4` for keyboard zoom

Each tolerance was set to make the current implementation pass. The magnetic animator tolerance (`1e-2`) is four orders of magnitude looser than the kinetic animator (`1e-6`). This likely reflects a genuine algorithmic divergence in the magnetic animator that was accommodated by loosening the tolerance rather than identified as a bug. But the harness treats `1e-2` and `1e-6` as equivalently valid "passing" thresholds. A `1e-2` tolerance on a velocity trajectory that produces values in the range `[-100, 100]` means the Rust implementation can produce velocities that differ by up to 1% from C++ -- enough to produce visually noticeable scrolling differences.

The compounding mechanism: each time a new test is added and the initial tolerance is too tight, the path of least resistance is to widen it. There is no record of why each tolerance was chosen, no regression test that the tolerance stays constant, and no mechanism to detect if a tolerance was widened after initial creation. The tolerances drift monotonically looser over time because tightening a tolerance requires debugging a regression, while loosening one requires only changing a number.

### 3.2 The `max_failure_pct` Double Gate Creates a False Sense of Tightness

The image comparison has two parameters: `channel_tolerance` (per-pixel) and `max_failure_pct` (population). This looks rigorous -- both a per-sample and a statistical constraint. But the interaction between them is subtle. A `(ch_tol=3, max_fail=10.0%)` test allows up to 10% of pixels to have per-channel differences of up to 255 (there is no cap on *how wrong* failing pixels are, only on how many can exceed the threshold). A single systematically wrong scanline in a 256x256 image (256 pixels out of 65536 = 0.4%) passes comfortably at `max_fail=10.0%` even if those pixels are completely wrong (diff=255). The double gate looks tighter than a single gate but is actually structurally incapable of detecting localized catastrophic errors.

---

## 4. Self-Conformance Circularity

### 4.1 The Harness Tests What It Was Built To Test

The golden tests were written by examining C++ behavior, constructing Rust equivalents, and iterating until the comparison passes. This means the test *parameters* (scene setup, viewport configuration, settle count, tolerance) were tuned to match C++ output. The harness cannot detect behaviors it was not designed to observe, and it was designed specifically to observe the behaviors that were already ported. Any C++ behavior that was misunderstood during porting produces a golden test that faithfully verifies the misunderstanding.

Concretely: the `settle()` function in every compositor/notice/widget test runs `for _ in 0..5 { tree.HandleNotice(...); view.Update(tree); }`. If the C++ equivalent needs 6 iterations to stabilize for a particular configuration, the golden data was generated with the C++ scheduler (which runs to actual convergence), but the Rust test hardcodes 5 iterations. The Rust test can still pass if the 5th iteration's output happens to match the C++ converged output -- but for a different initial state, 5 iterations might not suffice. The settle count was chosen empirically for the existing tests, not derived from a convergence proof.

### 4.2 The Comparator Cannot Detect Its Own Blindness

The `compare_images` function compares RGB channels only, the `compare_behavioral` function compares `(is_active, in_active_path)` tuples, and the `compare_notices` function compares translated flag bitmasks. Each comparator defines what "equivalent" means for its domain. But no comparator can verify that its definition of equivalence is the correct one. The notice flag translation table (`translate_cpp_notice_flags`) manually maps C++ bit positions to Rust bit positions -- if a mapping is wrong, every notice test that exercises that flag will pass (because both sides agree on the wrong translation) while the actual behavioral semantics diverge. The comparator's correctness is assumed, not tested.

### 4.3 The Flag Translation Table Is a Single Point of Unverified Trust

`translate_cpp_notice_flags` contains 10 manual bit-position mappings. If any single mapping is wrong, every golden test that triggers that notice flag will produce a systematic false positive. The mapping is not generated from a shared definition -- it is manually written in Rust by reading the C++ header. A transposition error (e.g., swapping `NF_ENABLE_CHANGED` and `NF_ACTIVE_CHANGED`) would cause the harness to verify that enable-change notices arrive when active-change notices should, and vice versa, and every such test would pass because the error is symmetric: the expectation is translated through the same wrong mapping as the check.

---

## 5. Methodology Mismatch

### 5.1 Vouk (1990): N-Version Assumes Independent Development

Vouk's back-to-back testing framework assumes the two versions were developed independently, so correlated faults are unlikely. Here, the Rust version was developed by *reading the C++ source*. Every Rust implementation decision was informed by the C++ code. Misunderstandings of C++ semantics propagate directly into the Rust port -- they are positively correlated, not independent. If a developer reads `emPanel::IsActive()` and misunderstands what "active" means in the C++ widget tree, the Rust `is_active` field will be wrong in the same way the developer understood it, and the golden tests will verify that understanding because the test setup reflects the same misunderstanding.

### 5.2 McKeeman (1998): Differential Testing Assumes Random Input

McKeeman's differential testing generates random inputs and feeds them to both implementations, looking for output disagreement. The zuicchini harness uses *fixed, handcrafted inputs* -- the exact scenes constructed in `gen_golden.cpp`. This eliminates the key strength of McKeeman's approach: the ability to find failures in the space between the test author's imagination. A handcrafted test exercises the paths the test author thought of. Random input generation exercises paths nobody thought of. The harness claims the differential testing methodology but captures none of its exploratory power.

### 5.3 Feathers (2004): Characterization Testing Assumes Same Codebase

Feathers' characterization tests capture existing behavior before refactoring *the same codebase*. The captured behavior is definitionally correct because it is the behavior the system already has. In a cross-language port, the captured behavior comes from the C++ codebase, but it is being used to validate a *different codebase* (Rust). Feathers' key assumption -- that the system under test is the same system that generated the characterization data -- is violated. In Feathers' framework, a characterization test failure means "the refactoring changed behavior." In this harness, a golden test failure could mean "the port has a bug" or "the port is correct but the golden data assumed C++-specific behavior" (like the alpha channel semantics), and the harness cannot distinguish between these.

### 5.4 The Gap Between All Three: State Space Coverage

None of the three methodologies address the problem of *state space coverage* in a stateful system. Eagle Mode's UI tree is deeply stateful: panels have focus state, active state, viewing state, layout state, notice queues, and engine priorities. The golden tests exercise specific state transitions (activate, focus, tab) from specific initial states. But the state space is combinatorial. The number of possible (focus, active, viewing, notice) state combinations for a 5-panel tree is enormous. The harness tests perhaps 50 specific state transitions. There is no methodology for reasoning about what fraction of the state space is covered, because none of the three source methodologies were designed for stateful UI systems.

---

## 6. LLM-Specific Execution Failures

### 6.1 Session Amnesia and Tolerance Archaeology

Each LLM session that modifies tolerance values does so based on the immediate test failure, without access to the history of why previous tolerances were set. A session that sees `(ch_tol=3, max_fail=3.5%)` for `widget_colorfield` has no way to know whether this tolerance was:
- The tightest value that passes (determined by bisection)
- A rough estimate that was never tightened
- A value that was loosened from `(1, 0.5%)` due to a now-fixed bug and should be re-tightened
- A value that masks an unresolved divergence

The tolerance value carries no provenance. Each session treats it as authoritative. Over many sessions, tolerances accumulate the maximum looseness ever needed, because no session has the context to tighten them.

### 6.2 Force Multiplier on Test Volume Without Verification Depth

An LLM can produce 50 golden tests in a session. Each test follows the pattern: construct scene, load golden, compare with tolerance. The volume of tests creates confidence ("we have 232 golden tests") without revealing whether those 232 tests cover 232 independent behavioral dimensions or 232 variations of the same 10 code paths. The LLM optimizes for the observable metric (test count) because test count is legible and checkable, while behavioral coverage is not. The force multiplier amplifies the appearance of progress without a corresponding amplification of actual verification depth.

### 6.3 Hallucinated Generator Parity

The C++ generator (`gen_golden.cpp`) and the Rust test files must exercise *exactly the same operations* in *exactly the same order* with *exactly the same parameters*. An LLM writing a Rust test reads the C++ generator code and translates it. But subtle C++ semantics can be lost in translation: C++ `emPainter::PaintEllipse` takes `(x, y, w, h)` as a bounding box, while the Rust equivalent might take `(cx, cy, rx, ry)` as center and radii (as the comment on line 98 of `painter.rs` reveals: "C++ PaintEllipse(28,28,200,150) -> cx=128 cy=103 rx=100 ry=75"). If the coordinate transformation is wrong, the golden test compares Rust output of a *different scene* against C++ output, and the tolerance band absorbs the difference. The test passes, the divergence compounds, and no session has enough context to realize the scenes are not the same.

### 6.4 Sycophantic Tolerance Setting

When a golden test fails with "3.7% pixels exceed tolerance" and the current tolerance is 3.5%, an LLM is structurally incentivized to widen the tolerance to 4.0% to make the test pass. The alternative -- investigating why 3.7% of pixels differ -- requires deep diving into pixel arithmetic, which is time-consuming and may require cross-referencing C++ source. Widening the tolerance is a one-character change that produces a green test. Over many sessions, this sycophantic pattern means tolerances converge toward the maximum divergence the implementation has ever exhibited, rather than toward the minimum divergence achievable.

---

## 7. Completeness Illusion

### 7.1 The 100-File, 232-Test, 13-Category Mirage

The codebase has 100 Rust source files in `emCore/`. The golden test suite has 232 test invocations across 13 test modules (painter, layout, compositor, interaction, notice, input, input_filter, animator, scheduler, widget, widget_interaction, composition, test_panel, parallel). This sounds comprehensive. But the test modules map onto perhaps 15 of the 100 source files. The remaining 85 source files -- `emConfigModel`, `emDialog`, `emFileModel`, `emFpPlugin`, `emGUIFramework`, `emMiniIpc`, `emModel`, `emProcess`, `emRecRecord`, `emScreen`, `emTimer`, `emWindow`, `emWindowStateSaver`, and dozens more -- have no golden equivalence tests at all. The file-level coverage of the golden harness is approximately 15%. The 232-test count creates a completeness illusion that does not survive simple enumeration.

### 7.2 Behavioral Categories With Zero Golden Coverage

The golden harness covers: pixel output (painter/compositor), coordinate output (layout), boolean state (behavioral/input), bitflag state (notice), and numeric trajectories (animator/input_filter). It does not cover:
- Error handling paths (what does `emErrorPanel` render when given malformed input?)
- File I/O behavior (does `emFileModel` handle the same edge cases as C++?)
- Configuration serialization (does `emRecRecord` round-trip the same data?)
- Process management (does `emProcess` spawn and signal correctly?)
- Timer behavior (does `emTimer` fire at the same intervals?)
- Model observation (does `emSigModel` notify the same observers in the same order?)

Each of these is a behavioral domain where C++ and Rust can diverge without any golden test detecting it.

### 7.3 The Composition Test Exists But Tests One Composition

There is a `composition.rs` test module and a `test_panel.rs` module that render complex multi-widget scenes. These are the closest thing to end-to-end tests. But they render a *specific* widget tree at a *specific* zoom level with *specific* initial state. The actual Eagle Mode UI involves dynamic widget creation, zoom-dependent panel expansion/collapse, user interaction triggering layout recalculations, and animation-driven view changes -- none of which are exercised in the composition tests. The composition tests verify that a static snapshot matches, not that the dynamic system behaves correctly.

---

## 8. Regression Blindness

### 8.1 Tolerance Absorbs Regressions

If a code change introduces a 1-channel-value regression across 0.3% of pixels in the `rect_solid` test, this regression is invisible because the test tolerance is `(ch_tol=1, max_fail=0.1%)` and the change falls within `ch_tol=1`. The existing tolerance was set to accommodate implementation-inherent differences, but it equally accommodates regressions. There is no mechanism to distinguish "this pixel differs because Rust rounding differs from C++ rounding" (expected, permanent) from "this pixel differs because someone introduced a bug" (unexpected, recent). Both are absorbed by the same tolerance band.

The compounding mechanism: each regression that falls within tolerance makes the implementation's actual divergence from C++ slightly worse. After 20 such regressions, the total divergence approaches the tolerance boundary. The 21st regression trips the tolerance, and the developer sees a test failure that appears to be caused by one change but is actually the accumulation of 20 undetected regressions. Debugging the 21st change reveals it is not the cause, the tolerance is widened, and the cycle continues.

### 8.2 The `MEASURE_DIVERGENCE` Metric Is Opt-In and Untracked

The harness has a `MEASURE_DIVERGENCE=1` mode that emits JSONL metrics (fail count, max diff, pass/fail). This is powerful -- it could detect tolerance-absorbed regressions by tracking the divergence trend over time. But it is opt-in (`MEASURE_DIVERGENCE=1` must be explicitly set), its output goes to stderr (not captured by test runners), and there is no mechanism to store, compare, or alert on trends. The infrastructure for regression detection exists but is not connected to anything that would actually detect regressions. It is a diagnostic tool, not a monitoring system.

### 8.3 Golden Data Is Not Versioned Independently

The golden data files are checked into the same repository as the Rust code. A single commit can modify both the Rust implementation and the golden data. If a developer "regenerates golden data" to match a changed implementation, the golden test still passes, but it is now testing equivalence with the *new* C++ generator output, which may differ from the *original* C++ generator output. There is no mechanism to detect whether golden data was regenerated as part of a Rust code change. The golden data's purpose is to be a *fixed* reference point, but the repository structure allows it to move.

---

## 9. Frozen Oracle Decay

### 9.1 The C++ Reference Version Is Fixed at 0.96.4

The golden data was generated from Eagle Mode 0.96.4. Eagle Mode continues to evolve. If a bug is fixed in Eagle Mode 0.97, the golden data still reflects the 0.96.4 behavior (including the bug). The Rust port, following the golden data, faithfully reproduces the bug. If someone later compares zuicchini against Eagle Mode 0.97, they will find a divergence that is actually *intentional C++ behavior change*, not a Rust bug. But without provenance on the golden data ("this reflects 0.96.4 behavior, which has known issue X"), the divergence is ambiguous.

### 9.2 Compiler-Dependent Floating-Point Output Drift

The C++ generator's floating-point output depends on the compiler's floating-point code generation. If `gen_golden.cpp` is rebuilt with a different GCC version, different optimization flags, or on a different architecture, the golden data changes at the LSB level. Layout coordinates that were `0.333333333333333370` become `0.333333333333333315`. These differences are within the `1e-6` tolerance, so they are invisible. But they mean the golden data is not actually a fixed reference -- it is a snapshot of one compilation's numeric behavior. Regenerating golden data is not idempotent across compiler versions.

### 9.3 Platform-Specific C++ Behavior Embedded in Golden Data

The C++ `emPainter` may use platform-specific SIMD intrinsics, platform-specific rounding modes, or platform-specific font rendering. The golden data captures the output of one platform. If the Rust implementation is run on a different platform (e.g., the developer is on x86-64 but CI is on aarch64), floating-point rounding differences in the Rust code may cause test failures that are not bugs but platform differences. The golden data embeds one platform's behavior as universal truth.

---

## 10. Composition Failures

### 10.1 The Parallel Test Proves Determinism, Not Correctness

The `parallel.rs` tests render scenes through both single-threaded and multi-threaded tiled paths and assert byte-identical output. This proves the parallel path is deterministic and consistent with the single-threaded path. It does not prove either path produces correct output. If both paths have the same bug (which is likely, since they share the same scanline rendering code), the parallel tests pass while both outputs are wrong relative to C++. The parallel tests look like they add verification ("we test this 9 more ways") but actually verify an orthogonal property (determinism) while providing zero additional correctness confidence.

### 10.2 The Notice-Behavioral-Input Triple Has Unverified Interactions

The harness tests notices separately (`notice.rs`), behavioral state separately (`interaction.rs`), and input separately (`input.rs`). Each module verifies one aspect of the panel tree. But in production, these interact: an input event changes active state, which triggers notice delivery, which triggers behavior callbacks, which may further change state. The golden tests for each module construct isolated scenarios that exercise one mechanism at a time. Cross-cutting interactions -- where the correctness of input handling depends on notice delivery ordering -- are not tested by any golden comparison. The `widget_interaction.rs` module comes closest but tests specific widget-level interactions, not the underlying panel tree interaction mechanisms.

### 10.3 The `settle()` Function Hides Temporal Behavior

Every stateful golden test uses a `settle()` function that runs a fixed number of notice-delivery/view-update cycles (typically 5). This converts a temporal process (the C++ scheduler's actual convergence loop) into a spatial snapshot (the state after N iterations). If the C++ and Rust implementations converge to the same final state but via different intermediate states, the harness cannot detect this. If they converge at different rates (C++ in 3 iterations, Rust in 7), and the golden test hardcodes 5, the test passes for C++ (converged) and may fail or pass for Rust depending on whether 5 is enough. The `settle()` function is a temporal compression that discards information about convergence dynamics. More dangerously, if the settle count of 5 is sufficient today but a future code change makes convergence require 6 iterations, the golden test breaks in a way that looks like a regression but is actually a convergence rate change that is semantically benign.

### 10.4 Test Infrastructure Shared Between Tests Creates Correlated Failures

The `support/mod.rs` module provides `TestHarness`, `NoticeBehavior`, `InputTrackingBehavior` and other shared test infrastructure. A bug in `TestHarness` (e.g., incorrect `tick()` implementation) would cause every test using it to fail -- but because all those tests were *written* using the same harness, and all passed when written, the harness's behavior is baked into the expectations. If `TestHarness::tick()` has a subtly wrong time step, every test that uses it was tuned to match that wrong time step. A bug in shared infrastructure is amplified across all tests but becomes invisible because all tests were calibrated against it.

---

## 11. Emergent Cascade Risks

### 11.1 The Silent Skip + Tolerance Ratchet + Session Amnesia Cascade

Consider this scenario across multiple LLM sessions:

1. Session A: Golden data is missing. All 232 tests silently skip. CI reports green. Nobody notices.
2. Session B: Golden data is regenerated with a new C++ compiler version. Some floating-point values shift at the LSB level. All tests pass (within tolerance).
3. Session C: A Rust code change introduces a small regression. The regression plus the LSB shift from step 2 exceeds tolerance for 3 tests. The developer widens tolerances.
4. Session D: Another Rust change, another tolerance widening. The pattern is established.
5. Session E: The developer checks "all 232 golden tests pass" and concludes the port is verified.

At no point in this cascade did any session have the full picture. Each session made a locally reasonable decision. The aggregate result is a test suite with loose tolerances comparing against stale golden data that may not even exist in some environments, all reporting green.

### 11.2 The Coverage-Completeness Inversion

As more golden tests are added, the *coverage gap* between tested and untested code paths widens, not narrows. Each new golden test exercises a known code path with a known setup. The total space of possible code paths grows combinatorially with the number of features. Adding 10 more widget rendering tests to the current 50 moves from 50/N to 60/N coverage -- but if the feature set grew by 20 widgets in the same period, coverage actually decreased from 50/(N-20) to 60/N. The project structure (100 Rust source files, ~15 exercised by goldens) means that completing the golden test suite requires roughly 5x the current effort just for file-level coverage, before considering state-space coverage within each file. The current 232 tests are not the middle of the journey; they are near the beginning.

### 11.3 The Self-Referential Quality Signal

The harness's quality signal is "all tests pass." But the harness was constructed to make all tests pass (by setting tolerances, by choosing scenes, by defining comparators). The harness cannot distinguish between "the port is correct" and "the harness is well-calibrated to the port's current behavior." Adding more tests of the same kind (same tolerance-setting pattern, same scene-construction pattern, same comparison function) reinforces the calibration without improving the correctness signal. The quality metric is self-referential: it measures the harness's agreement with the implementation, not the implementation's agreement with the C++ reference.

---

## 12. What the Frozen Oracle Makes Invisible

Vouk's methodology assumes live execution of both versions on the same inputs. The frozen oracle eliminates:

- **Input-dependent branching in C++**: If the C++ code has a branch that fires only for specific input timing (e.g., a debounce threshold), the golden data captures one timing. The Rust test replays that timing. But the Rust debounce implementation might handle *different* timings incorrectly, and the harness will never present those timings.

- **Non-deterministic C++ behavior**: If C++ output depends on hash map iteration order, uninitialized memory, or scheduler timing, the golden data freezes one arbitrary outcome. The Rust implementation may deterministically produce a different outcome that is equally valid, but the golden test calls it a failure.

- **C++ error recovery paths**: If the C++ code has error handling that produces fallback output, and the Rust code has different error handling, the golden data from the C++ error path becomes the oracle for the Rust non-error path (or vice versa). The frozen oracle cannot adapt to path divergence.

- **Resource-dependent behavior**: If C++ behavior depends on available memory, file descriptors, or thread count, the golden data captures behavior under the generator's resource conditions. The Rust tests run under different resource conditions, and any resource-dependent divergence is invisible.

The frozen oracle converts a *relational* property (C++ and Rust produce the same output for the same input) into a *unary* property (Rust produces this specific output). The relational property implies the unary property, but not vice versa: the Rust implementation could produce the frozen output for the golden inputs while diverging for all other inputs.
