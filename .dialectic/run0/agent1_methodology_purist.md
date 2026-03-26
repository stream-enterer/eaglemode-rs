# Methodology Purist Analysis: What the Verification Harness Must Do and Must Refuse to Do

## Preamble: Scope and Constraints of This Argument

This analysis examines each derived requirement against the three source texts: Vouk (1990) "Back-to-Back Testing" in *Information and Software Technology* 32(1); McKeeman (1998) "Differential Testing for Software" in *Digital Technical Journal* 10(1); and Feathers (2004) *Working Effectively with Legacy Code*, Chapter 13 and surrounding material on characterization tests. Every claim is traced to these sources. Where a derived requirement extends beyond what the text actually states, I say so. Where the sources demand something the derived requirements omit, I identify it.

---

## Part I: Evaluation of Vouk-Derived Requirements

### V1. Input identity: CORRECTLY DERIVED, but incompletely stated.

Vouk states that in back-to-back testing "all programs are tested with the same input data" (Section 2). This is a foundational axiom of the method. The derived requirement correctly captures this: the harness must guarantee identical inputs for oracle generation and port execution.

However, Vouk's formulation is stronger than "guarantee identical inputs." In his model, both versions are executed with the *same* input in the *same* test run, enabling immediate comparison. The frozen oracle adaptation breaks this temporal coupling. When the C++ is executed once to produce golden files and the Rust is executed later against those files, "same input" must be proven rather than assumed. The harness must therefore not merely guarantee but *verify* input identity -- the golden data file must encode or reference the exact input configuration that produced it, and the Rust test must reconstruct that exact configuration. Any drift in input reconstruction (e.g., floating-point constants that differ between C++ and Rust compilers, or test setup code that evolves independently of the generator) violates V1. The frozen oracle makes this harder, not easier, to satisfy.

### V2. Exhaustive pair comparison: CORRECTLY DERIVED.

Vouk describes comparing "outputs of all possible program pair combinations" (Section 3). In the two-version case (C++ oracle, Rust port), this reduces to: every oracle output must have a corresponding comparison. The derived requirement is faithful.

However, note that Vouk's exhaustive comparison requirement applies to *all output variables*, not just a subset. If the C++ generates output across multiple channels (pixel data, layout rects, behavioral state, notice flags, trajectory data), every channel must be compared. The harness must not generate golden data for one channel and neglect another. A golden file that captures pixel output but not the side effects that produced it (e.g., state changes in the panel tree) would violate V2 by comparing only a subset of the output space.

### V3. Thorough investigation of disagreements: CORRECTLY DERIVED, but the standard is higher than stated.

Vouk writes that "when a difference is observed the problem is thoroughly investigated" (Section 4.1). The derived requirement says "the harness must produce sufficient diagnostics." This is correct but understated. Vouk's "thorough investigation" is not merely diagnostic output -- it is a process requirement. The investigation must determine whether the disagreement is: (a) a fault in the port, (b) a fault in the oracle, (c) a specification ambiguity, or (d) an acceptable implementation difference. The harness must support this classification, not merely emit data. The MEASURE_DIVERGENCE JSONL output and diff visualization are diagnostic aids, but they do not by themselves constitute "thorough investigation" -- a human or automated process must triage every disagreement.

### V4. Regression after correction: CORRECTLY DERIVED.

Vouk states that "regression testing is used whenever possible to check the applied corrections" (Section 4.2). The derived requirement is faithful: the harness must be re-runnable and must distinguish regressions (newly-introduced failures) from pre-existing failures. This implies the harness must maintain a baseline of known failures and detect when the set changes. A pass/fail test suite that re-runs all golden comparisons satisfies the "re-runnable" aspect, but detecting *regressions as distinct from pre-existing failures* requires either a known-failure list or a mechanism to compare current results against previous results.

### V5. Failure independence assumption stated: CORRECTLY DERIVED, with important caveat.

Vouk's detection probability model (Section 3, Equations 1-3) relies on the assumption that failures in different versions are statistically independent. This is the mathematical foundation for the claim that N-version comparison detects faults that single-version testing misses. The derived requirement that "the harness must state this assumption" is faithful to Vouk.

However, the deeper point is that in a manual C++-to-Rust port, failure independence is *systematically violated*. The port author reads the C++ code and translates it. Misunderstandings of the C++ algorithm will be faithfully reproduced in Rust. Vouk explicitly acknowledges this threat: the detection model breaks down when failures are correlated. The harness must not merely state the assumption -- it must identify the specific ways in which the porting process violates it. A manual port from reading the source code is the *worst case* for failure independence, far worse than the independent-development N-version programming that Vouk primarily analyzes.

### V6. Coincident failure acknowledgment: CORRECTLY DERIVED.

Vouk states that "when the correlation is 100%... the fault cannot be detected" (Section 3.2). The derived requirement that the harness must define what it cannot detect is directly supported. The harness must explicitly document the class of faults that produce identical wrong output in both C++ and Rust -- for example, algorithmic errors present in the C++ original that are faithfully ported, or numerical formulas that are wrong in the same way in both implementations.

### V7. Combination with inspection: CORRECTLY DERIVED.

Vouk states that "back-to-back testing should not be used in isolation" and "must be combined with... specification and code inspection" (Section 5). The derived requirement that "the harness must identify code not covered by comparison" is a reasonable operationalization. The harness should produce a manifest of which Rust code paths are exercised by golden comparisons and which are not, so that inspection effort can be directed at uncovered code.

### V8. All conflict warnings inspected: OVER-EXTENDED.

The derived requirement states "no disagreement suppressed, even within tolerance." Vouk says "it is necessary to inspect carefully all conflict warnings" (Section 4.1). However, Vouk is writing about *disagreements that the comparison detects*, not about sub-threshold differences. Vouk does not address tolerance-based comparison because his model assumes exact comparison of outputs. The concept of "within tolerance but still a disagreement" is an adaptation not present in Vouk's text.

That said, the spirit of V8 is sound: Vouk's method requires that every detected disagreement be investigated, not suppressed. The over-extension is the claim that sub-tolerance differences must also not be suppressed. Vouk's text does not speak to this. A faithful reading requires: every comparison that the harness flags as a disagreement must be investigated. Whether the harness should flag sub-tolerance differences is a design choice the sources do not address.

**Correction**: V8 should be restated as: "Every disagreement that exceeds the comparison threshold must be inspected; no such disagreement may be silently suppressed." The clause "even within tolerance" is not supported by Vouk.

### V9. Stopping criterion: PARTIALLY DERIVED, with important nuance.

Vouk discusses stopping criteria in Section 4.3: "the testing can be stopped when the target reliability is estimated to have been reached." The derived requirement that a "measurable stopping criterion [be] defined before testing begins" is a reasonable operationalization. However, Vouk's stopping criterion is based on *reliability estimation* -- a statistical measure of the probability that remaining faults exist -- not merely on coverage or test count. The derived requirement should note that Vouk's stopping criterion is reliability-based, which requires a fault detection model and failure rate estimation. Simply defining "all golden tests pass" as a stopping criterion does not satisfy Vouk's requirement, because passing all tests says nothing about faults in untested paths.

### V10. Monitoring multiple output variables: CORRECTLY DERIVED.

Vouk's Figure 6 demonstrates that monitoring multiple output variables increases fault detection probability. The derived requirement is directly supported. The harness must compare all available output channels (pixels, rects, behavioral state, notices, input state, trajectories), not just the most convenient one.

### MISSING Vouk requirement: V11. Version selection and configuration management.

Vouk discusses in Section 2 the importance of configuration management in back-to-back testing: the versions under test must be precisely identified, and the test environment must be reproducible. The derived requirements do not include a requirement that the harness record the exact C++ compiler version, flags, and source revision used to generate golden data, nor the exact Rust compiler version and flags used to execute the port. In the frozen oracle model, this is critical -- if the golden data was generated with one C++ compiler and the data is later found to be wrong, the harness must be able to trace back to the exact build that produced it.

### MISSING Vouk requirement: V12. Quantitative comparison metrics.

Vouk's detection model (Section 3) is inherently quantitative -- it reasons about detection probabilities, not pass/fail outcomes. The derived requirements do not include a requirement that the harness compute and report quantitative disagreement metrics (e.g., percentage of pixels differing, maximum channel difference, distribution of differences). The existing MEASURE_DIVERGENCE mechanism partially addresses this, but the requirement should be explicit: the harness must produce quantitative measures of disagreement, not merely binary pass/fail.

---

## Part II: Evaluation of McKeeman-Derived Requirements

### M1. Oracle relationship defined: CORRECTLY DERIVED, with important clarification.

McKeeman states that "differential testing requires that two or more comparable systems be available" (Section 1). The derived requirement that the "asymmetric oracle/subject relationship [be] documented" is a valid extension. McKeeman's original formulation is symmetric -- he treats all systems as peers and flags *any* disagreement for investigation. The asymmetric adaptation (C++ is oracle, Rust is subject) is specific to this project. The derived requirement correctly notes that this asymmetry must be documented, but it should also note that McKeeman's method does not inherently privilege one system over another. The decision to treat C++ as the oracle is a project decision, not a methodological requirement of differential testing.

### M2. Crash and hang detection: CORRECTLY DERIVED.

McKeeman explicitly discusses the case where "one of the systems loops indefinitely or crashes" (Section 3) as a form of disagreement that must be detected. The derived requirement that panics, aborts, and timeouts are disagreements is directly supported. The harness must detect Rust panics, process aborts, and execution timeouts as test failures, even when the C++ golden data shows successful completion.

### M3. Statement coverage measurement: CORRECTLY DERIVED.

McKeeman writes that "the simplest measure of completeness is statement coverage" (Section 4). The derived requirement to measure which port code is exercised is directly supported. McKeeman views coverage as a completeness metric for the test suite: if large portions of the Rust port are never exercised by any golden test, the differential testing provides no assurance about those portions.

### M4. Generated test inputs: CORRECTLY DERIVED, with important emphasis.

McKeeman's central contribution is the use of random/generated inputs: "Random testing is a way to make testing more complete" (Section 2). The derived requirement to "support generated inputs as complement to hand-written" is directly supported. However, McKeeman's argument is stronger: he argues that generated inputs are *essential*, not merely complementary. Hand-written tests suffer from the same cognitive biases as the code they test. Generated inputs explore the input space in ways that human test authors do not anticipate. For a manual port, where the same developer who wrote the code also writes the tests, generated inputs are the primary defense against shared blind spots. The derived requirement understates McKeeman's emphasis.

### M5. Test reduction: CORRECTLY DERIVED.

McKeeman describes test reduction as a key step: "the first step is to reduce the test to the shortest version that qualifies" (Section 5). The derived requirement is directly supported. When a generated input reveals a disagreement, the harness should support minimizing the input to the smallest case that still triggers the disagreement. This aids diagnosis (V3) and produces more useful regression tests (V4).

### M6. Distributed execution: CORRECTLY DERIVED.

McKeeman's architecture separates test generation, execution, and comparison into distinct phases (Section 3, Figure 1). The derived requirement that "oracle generator and port tests don't need to run in same process" and that "comparison is on serialized outputs" is directly supported by McKeeman's architecture. The frozen oracle adaptation is actually well-aligned with McKeeman's model, which already assumes serialized comparison.

### M7. Result classification: CORRECTLY DERIVED.

McKeeman describes a "test analyzer" component that classifies results by failure category (Section 3). The derived requirement is directly supported. The harness must classify disagreements into categories (pixel mismatch, structural mismatch, crash, timeout, etc.) to enable prioritized investigation.

### M8. Regression as quality metric: PARTIALLY DERIVED.

McKeeman does discuss recording outputs as expected results for future regression detection. However, this is more Feathers's territory than McKeeman's. McKeeman's primary concern is *discovery* of disagreements, not *prevention of regressions*. The derived requirement is not wrong, but its attribution to McKeeman is weak. It would be better attributed to Feathers (see F1/F3) or to Vouk (V4).

### MISSING McKeeman requirement: M9. Automation of the comparison loop.

McKeeman's method is predicated on *automation* -- the entire generate-execute-compare cycle must run without human intervention (Section 3). The derived requirements do not explicitly require that the harness be fully automated. A harness that requires manual steps (e.g., manually running the C++ generator, then manually running Rust tests, then manually comparing results) violates McKeeman's method. The harness must automate the full cycle: input generation, oracle execution (or golden data loading), port execution, comparison, and result classification.

### MISSING McKeeman requirement: M10. Input domain characterization.

McKeeman discusses the importance of characterizing the input domain for the systems under test (Section 2). The generated inputs must cover the relevant input space -- not merely random bytes, but structured inputs that exercise the system's functionality. The derived requirements do not address input domain definition or the quality of generated inputs.

---

## Part III: Evaluation of Feathers-Derived Requirements

### F1. Tests document actual behavior, not specified behavior: CORRECTLY DERIVED.

Feathers writes: "A characterization test is a test that characterizes the actual behavior of a piece of code" (Chapter 13, p. 186). The derived requirement is directly supported. The golden data must capture what the C++ *actually does*, not what a specification says it should do. If the C++ has a bug that produces specific pixel output, that buggy output is the golden data.

### F2. Suspicious values must be flaggable: CORRECTLY DERIVED.

Feathers writes about marking suspicious behavior: "mark it as suspicious and find out what the effect would be of fixing it" (Chapter 13, p. 188). The derived requirement is directly supported. The harness must support marking specific golden test results as "suspicious but accepted" -- values that appear to be bugs in the C++ original but which the port must reproduce for behavioral equivalence. This connects to V6 (coincident failures) and F8 (deployed-system bug policy).

### F3. Characterization algorithm: CORRECTLY DERIVED.

Feathers describes the characterization test algorithm (Chapter 13, p. 186): (1) write a test that calls the code, (2) write an assertion that you know will fail, (3) run the test and let the failure tell you what the actual behavior is, (4) change the assertion to match the actual behavior. The derived requirement that "oracle data must be captured from execution, not hand-constructed" is a faithful restatement of this algorithm. The golden data generator that executes C++ code and serializes the output follows Feathers's algorithm exactly.

### F4. Coverage guided by change, not completeness: CORRECTLY DERIVED, with important nuance.

Feathers writes: "we think about the changes that we want to make... and try to figure out whether the tests that we have will sense any problems" (Chapter 13, p. 189). The derived requirement is directly supported. However, the nuance is critical: Feathers is writing about *legacy code modification*, where you add characterization tests around the specific code you plan to change. In the porting context, *all* code is being changed (from C++ to Rust), so "guided by change" means "everywhere." This doesn't eliminate the requirement -- it means the harness must prioritize golden test coverage for code paths that are most likely to diverge in translation (e.g., integer arithmetic, pointer semantics, memory layout differences), not merely aim for uniform coverage.

### F5. Branch path uniqueness: CORRECTLY DERIVED.

Feathers writes: "ask yourself whether there is any other way that the test could pass, aside from executing that branch" (Chapter 13, p. 190). The derived requirement is directly supported. A golden test that passes for multiple possible implementations (because it only tests a subset of the output or uses excessive tolerance) is weak. Each test should be specific enough that only the correct implementation satisfies it. This argues *against* large tolerances in pixel comparison and *for* exact comparison where the methodology permits.

### F6. Conversion exercise: CORRECTLY DERIVED.

Feathers discusses the importance of testing conversions -- transformations of data from one form to another -- because these are common sites of bugs (Chapter 13, p. 191). The derived requirement that "the most valuable characterization tests exercise a specific path and exercise each conversion along the path" is directly supported. In the porting context, conversions are everywhere: integer widths, signed/unsigned conversions, floating-point rounding, coordinate transformations. The harness must have tests that specifically target these conversion boundaries.

### F7. Behavioral existence and connection: CORRECTLY DERIVED.

Feathers writes about verifying "the existence and connection of those behaviors on a case-by-case basis" (Chapter 11, p. 157). The derived requirement is directly supported. The harness must verify not just that individual functions produce correct output, but that behaviors are connected -- that the output of one stage correctly feeds the input of the next. End-to-end golden tests (compositor tests that exercise the full painter-layout-composition pipeline) serve this purpose.

### F8. Deployed-system bug policy: CORRECTLY DERIVED.

Feathers writes: "If the system has been deployed, you need to examine the possibility that someone is depending on that behavior" (Chapter 13, p. 188). The derived requirement is directly supported. If the C++ original has been deployed and users depend on its specific behavior (including its bugs), the Rust port must reproduce that behavior. The harness must not "fix" C++ bugs in the port without explicitly documenting the divergence and its impact.

### MISSING Feathers requirement: F9. Sensing variables.

Feathers discusses the concept of "sensing variables" (Chapter 13, p. 187) -- values that you can observe to determine whether the code is behaving correctly. The harness must identify and capture the right sensing variables. If the golden data captures only final pixel output but not intermediate state (e.g., the layout rectangles that produced the pixel output), the harness may miss divergences that happen to cancel out in the final output. The derived requirements do not explicitly require identification and capture of intermediate sensing variables, though V10 partially addresses this through "multiple output variables."

### MISSING Feathers requirement: F10. Preservation under refactoring.

Feathers's characterization tests are explicitly designed to survive refactoring -- they test behavior, not implementation structure (Chapter 13, p. 186). The derived requirements do not address the requirement that golden tests must be robust to implementation changes in the Rust port that preserve behavior. A golden test that depends on Rust-specific implementation details (e.g., the order of HashMap iteration) rather than behavioral output would violate this principle. The harness must ensure that golden comparisons test *observable behavior*, not implementation artifacts.

---

## Part IV: The Frozen Oracle -- Methodological Consequences

The project's adaptation of a frozen oracle (C++ executed once to produce golden files, Rust compared against frozen files) creates specific methodological gaps that must be explicitly acknowledged.

### Gap 1: Oracle bug discovery is asymmetric and delayed.

In Vouk's live back-to-back model, a disagreement equally questions both versions. In the frozen oracle model, disagreements are assumed to be Rust port bugs. If the C++ golden data contains a bug, the harness will force the Rust port to reproduce that bug (per F8). Discovering that the golden data itself is wrong requires external evidence (specification review, user reports, or inspection as per V7). The harness must provide a mechanism to challenge and re-generate golden data when oracle bugs are suspected.

### Gap 2: Input-output coupling is severed.

In Vouk's model, both versions process the same input in the same test run, guaranteeing input identity (V1) by construction. In the frozen oracle model, the input is reconstructed from the test code -- the C++ generator and the Rust test independently set up their inputs. Any drift between these setups is invisible to the harness. The harness must either: (a) encode the input in the golden file alongside the output, so the Rust test reads the input from the same source as the output, or (b) use a shared input specification that both the generator and the test consume. The current architecture (C++ generator hardcodes inputs; Rust tests hardcode inputs) relies on developer discipline to maintain V1, which is fragile.

### Gap 3: Re-execution of the oracle is a manual, external step.

Vouk's V4 (regression after correction) and McKeeman's automated comparison loop (M9) both assume that the oracle can be re-executed. When the frozen oracle is regenerated (e.g., after discovering an oracle bug), this is a manual step outside the normal test cycle (`make -C zuicchini/tests/golden/gen && make -C zuicchini/tests/golden/gen run`). The harness must document when oracle regeneration is required and must detect staleness -- if the golden data was generated from a different version of the C++ source than what is currently checked in, the comparison is unsound.

### Gap 4: The oracle cannot be extended without C++ infrastructure.

McKeeman's M4 (generated test inputs) requires the ability to generate new inputs and observe both systems' outputs. With a frozen oracle, generating new golden data requires the C++ build environment. If the C++ build environment becomes unavailable (as is common over time), the ability to extend the golden test suite is lost. The harness must document this dependency and the consequences of losing it.

### Gap 5: Coincident failure masking is permanent.

Vouk's V6 acknowledges that coincident failures cannot be detected. In a live back-to-back model, new test inputs might eventually expose different manifestations of coincident failures. In the frozen oracle model, the golden data is fixed -- if a coincident failure exists in the captured data, no amount of Rust-side testing will find it. The frozen oracle *permanently bakes in* any coincident failures present at capture time.

---

## Part V: What the Harness Must Refuse to Do

Drawing from all three sources, the harness must explicitly refuse the following:

1. **Refuse to suppress disagreements.** Vouk (V8) requires inspection of all conflict warnings. The harness must not have a mechanism to mark a failing golden test as "expected failure" without explicit documentation of *why* it fails and *what investigation was performed*. A blanket `#[ignore]` annotation on a failing golden test violates V3 and V8.

2. **Refuse to claim coverage it does not have.** Vouk (V7) and McKeeman (M3) require that the harness not represent itself as providing assurance about code it does not exercise. If 40% of Rust code paths have no golden test coverage, the harness must report this, not silently pass.

3. **Refuse to treat tolerance as absence of disagreement.** Vouk's model assumes exact comparison. Any tolerance (channel_tolerance, eps, f64 tolerance) is an adaptation that introduces a gap. The harness must report *all* comparisons and their measured disagreement, not just those that exceed tolerance. The MEASURE_DIVERGENCE mechanism partially addresses this, but it must be always-on for methodology compliance, not opt-in via environment variable.

4. **Refuse to generate golden data from Rust.** Feathers (F3) requires that characterization data be captured from the *existing system* (C++), not the new system (Rust). If the harness allows "blessing" Rust output as golden data, it is no longer characterizing the C++ behavior -- it is characterizing Rust behavior and testing Rust against itself, which provides zero fault detection power. The `save_trajectory_golden` function, which can write golden data from Rust execution, is methodologically dangerous and must be restricted to cases where the C++ generator is demonstrably unavailable and the Rust output has been independently verified.

5. **Refuse to operate without stating its limitations.** Vouk (V5, V6, V7) collectively require that the harness document: what it assumes (failure independence), what it cannot detect (coincident failures), and what it does not cover (uninstrumented code paths). A harness that silently passes all tests without these disclosures gives false confidence.

---

## Part VI: Summary of Requirement Disposition

| Req | Status | Notes |
|-----|--------|-------|
| V1 | Correct, understated | Must verify input identity, not merely assume it |
| V2 | Correct | All output channels must be compared |
| V3 | Correct, understated | "Sufficient diagnostics" is necessary but not sufficient; investigation process required |
| V4 | Correct | Must distinguish regressions from pre-existing failures |
| V5 | Correct, with caveat | Manual port systematically violates the assumption |
| V6 | Correct | Must define undetectable fault classes |
| V7 | Correct | Must identify uninstrumented code |
| V8 | Over-extended | "Even within tolerance" not supported by Vouk; threshold disagreements must be inspected |
| V9 | Partially correct | Vouk requires reliability-based stopping, not just "all tests pass" |
| V10 | Correct | Multiple output channels increase detection |
| M1 | Correct, with clarification | Asymmetry is project choice, not methodological requirement |
| M2 | Correct | Panics, aborts, timeouts are disagreements |
| M3 | Correct | Statement coverage of the port |
| M4 | Correct, understated | McKeeman argues generated inputs are essential, not merely complementary |
| M5 | Correct | Minimize disagreement-exposing inputs |
| M6 | Correct | Frozen oracle aligns with distributed comparison |
| M7 | Correct | Classify disagreements by category |
| M8 | Weakly attributed | Better attributed to Feathers F3 or Vouk V4 |
| F1 | Correct | Actual behavior, not specified behavior |
| F2 | Correct | Suspicious values flaggable |
| F3 | Correct | Oracle data from execution |
| F4 | Correct, with nuance | "All code is changing" in a port |
| F5 | Correct | Tests must be specific enough to reject wrong implementations |
| F6 | Correct | Target conversion boundaries |
| F7 | Correct | End-to-end behavioral connection |
| F8 | Correct | Deployed bugs must be examined, not silently fixed |

**Missing requirements identified:**
- V11: Configuration management (compiler versions, build flags, source revisions)
- V12: Quantitative comparison metrics (not just pass/fail)
- M9: Full automation of the generate-execute-compare cycle
- M10: Input domain characterization for generated inputs
- F9: Identification of intermediate sensing variables
- F10: Golden tests must test observable behavior, not implementation artifacts

**Frozen oracle gaps identified:**
- Oracle bug discovery is asymmetric and delayed
- Input-output coupling is severed
- Oracle re-execution is manual and external
- Oracle extension requires C++ infrastructure
- Coincident failure masking is permanent

---

## Part VII: Final Verdict

The derived requirements are substantially correct in their reading of the three sources. The primary deficiencies are:

1. **Under-emphasis of McKeeman's insistence on generated inputs (M4)**: This is not a "complement" to hand-written tests; McKeeman argues it is the primary mechanism for discovering unexpected disagreements.

2. **Over-extension of V8**: The "even within tolerance" clause is not supported by Vouk's text, which assumes exact comparison.

3. **Weak attribution of M8**: Regression detection is better grounded in Vouk (V4) and Feathers (F3) than in McKeeman.

4. **Six missing requirements (V11, V12, M9, M10, F9, F10)**: These are directly supported by the source texts and are not mere "nice to haves."

5. **Insufficient analysis of the frozen oracle adaptation**: The five gaps identified in Part IV are methodologically significant. The frozen oracle is a valid practical adaptation, but it introduces specific failure modes that must be explicitly acknowledged and mitigated. None of the three sources describe or endorse a frozen oracle model, so every gap it introduces is a deviation from the published methodology that must be justified on its own terms.

The harness, to conform to the published methodologies, must do all of the above. It may not soften these requirements for practicality. Where practicality forces a deviation, the deviation must be documented at the point where it occurs, with the specific methodological requirement it violates and the rationale for accepting the gap.
