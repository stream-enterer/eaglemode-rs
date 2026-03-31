# Phase 4: emThread Investigation & Cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the emPainterDrawList/emThread relationship, close all remaining NOT VERIFIED items in marker files, and bring CORRESPONDENCE.md to a completed state.

**Architecture:** Phase 4 is investigation and documentation — no new types are being ported. The work is: (1) verify the emPainterDrawList.rs safety invariant and document its relationship to C++ emThread, (2) close 8 remaining NOT VERIFIED items across 6 marker files by investigating C++ source, (3) final CORRESPONDENCE.md update. No emPainter*.rs files are modified (firewall rule still applies).

**Tech Stack:** C++ source at `~/git/eaglemode-0.96.4/`, Rust source at `src/emCore/`, grep/read investigation only. No new crates or dependencies.

**Spec:** `docs/superpowers/specs/2026-03-28-port-completion-design.md` (Section 2 Phase 4)

**Key rules from spec:**
- emPainter firewall: do NOT touch any `emPainter*.rs` file as blast radius
- No test assumed correct: verify existing test correctness before relying on them
- Every NOT VERIFIED item is resolved by investigation with specific evidence

**Current state:** 8 .no_rs marker files remain, 1 .rust_only marker file remains. 8 NOT VERIFIED items across 6 files. No `docs/empainter-deferred-refactors.log` exists (no deferred refactors were logged during Phases 1–3).

**Dependency order:** Tasks 1–6 are independent investigations (can run in any order). Task 7 depends on all others.

---

## Task 1: emPainterDrawList safety invariant verification

**Files:**
- Modify: `src/emCore/emPainterDrawList.rust_only`

**Context:** The `rust_only` marker has one NOT VERIFIED item: "whether the frame loop structure actually guarantees invariant #2 (no tree modification between record and replay)." The three safety invariants for `unsafe impl Send/Sync for DrawOp` are:
1. Images are owned by panel behaviors in the PanelTree
2. The PanelTree is not modified between recording and replay
3. `std::thread::scope` ensures all replay threads complete before returning

The frame loop is in `src/emCore/emViewRenderer.rs`. The recording phase calls `view.Paint(tree, &mut rec)` with `tree: &PanelTree` (immutable borrow). The replay phase uses `&draw_list` only — it does not reference the tree at all. The question is whether anything could modify the tree between recording and replay within the same frame.

- [ ] **Step 1: Trace the frame loop structure**

Read `src/emCore/emViewRenderer.rs` — specifically the `render_parallel` method. Document:
1. Where `draw_list` is created (Phase 1 start)
2. Where recording happens (`view.Paint(tree, &mut rec)`)
3. Where replay happens (`pool.CallParallel(...)`)
4. Whether any code between recording end and replay start could modify the PanelTree

Read `src/emCore/emWindow.rs` — find where `render_parallel` is called. Document:
1. What happens before and after the render call in the frame loop
2. Whether panel tree mutations are possible during the render call

- [ ] **Step 2: Trace image pointer lifetimes**

The 6 DrawOp variants with `*const emImage` raw pointers are:
- PaintImageFull, PaintImageColored, PaintImageScaled
- PaintBorderImage, PaintImageSimple, PaintBorderImageColored

For each, trace where the `&emImage` reference originates:
1. Read `src/emCore/emPainter.rs` — find the `try_record()` calls for image-painting methods
2. Verify images come from panel behavior fields (owned by PanelTree nodes)
3. Verify no image could be dropped or reallocated between recording and replay

```bash
grep -n "PaintImage\|PaintBorderImage" src/emCore/emPainter.rs | grep "try_record\|DrawOp" | head -20
```

- [ ] **Step 3: Document the finding**

Update `src/emCore/emPainterDrawList.rust_only` — replace the NOT VERIFIED item (lines 75-78) with the verified finding. The finding should include:
- Whether invariant #2 is guaranteed by the frame loop structure
- The specific code path that ensures no tree modification during replay
- Whether the `&PanelTree` borrow in Phase 1 and absence of tree access in Phase 2 provides the guarantee
- Any remaining risk (e.g., interior mutability via RefCell that could bypass the borrow checker)

If the invariant IS guaranteed by the borrow structure, write:
```
Verified: invariant #2 is guaranteed by the frame loop structure.
  emViewRenderer::render_parallel() creates draw_list, records with
  tree: &PanelTree (immutable borrow), then replays without touching
  the tree. The &PanelTree borrow prevents mutation during recording.
  Between recording end and replay start, no code path accesses the
  tree. [Add specific line references.]
```

If the invariant is NOT guaranteed (e.g., RefCell interior mutability), write:
```
RISK: invariant #2 is NOT enforced by the type system. [Explain why.]
  The safety of unsafe impl Send/Sync for DrawOp depends on a runtime
  invariant. [Describe the specific risk and what would break.]
```

- [ ] **Step 4: Commit**

```bash
git add src/emCore/emPainterDrawList.rust_only
git commit -m "docs: verify DrawList safety invariant #2

[Summary of finding — whether invariant is guaranteed or at risk.]"
```

---

## Task 2: emThread outside-emCore investigation

**Files:**
- Modify: `src/emCore/emThread.no_rs`

**Context:** NOT VERIFIED (line 218): "whether any of the 19 outside-emCore files use emThread primitives in ways that are not covered by std::thread / std::sync." The 8 specific files flagged are:
- `include/emAv/emAvImageConverter.h`
- `include/emFractal/emFractalFilePanel.h`
- `include/emWnds/emWndsViewRenderer.h`
- `include/emX11/emX11ViewRenderer.h`
- `include/SilChess/SilChessPanel.h`
- `src/emAv/emAvServerModel.cpp`
- `src/emSvg/emSvgServerModel.cpp`
- `src/emTiff/emTiffImageFileModel.cpp`

The question: do these files use emThread/emThreadMutex/emThreadEvent in ways that std::thread/std::sync doesn't cover?

- [ ] **Step 1: Check each flagged file for emThread usage patterns**

For each of the 8 files, grep for emThread usage and categorize:

```bash
for f in \
  ~/git/eaglemode-0.96.4/include/emAv/emAvImageConverter.h \
  ~/git/eaglemode-0.96.4/include/emFractal/emFractalFilePanel.h \
  ~/git/eaglemode-0.96.4/include/emWnds/emWndsViewRenderer.h \
  ~/git/eaglemode-0.96.4/include/emX11/emX11ViewRenderer.h \
  ~/git/eaglemode-0.96.4/include/SilChess/SilChessPanel.h \
  ~/git/eaglemode-0.96.4/src/emAv/emAvServerModel.cpp \
  ~/git/eaglemode-0.96.4/src/emSvg/emSvgServerModel.cpp \
  ~/git/eaglemode-0.96.4/src/emTiff/emTiffImageFileModel.cpp; do
  echo "=== $(basename $f) ==="
  grep -n "emThread\|emMutex\|emEvent\|UserSpaceMutex" "$f" 2>/dev/null | head -10
done
```

For each usage found, categorize:
- `emThread` (start/join) → maps to `std::thread::spawn` / `std::thread::scope`
- `emThreadMiniMutex` → maps to `std::sync::Mutex`
- `emThreadMutex` (reader/writer) → maps to `std::sync::RwLock`
- `emThreadEvent` (counting semaphore) → maps to `std::sync::Condvar` + `Mutex`
- `emThreadRecursiveMutex` → maps to custom or `parking_lot::ReentrantMutex`
- Anything else → flag as potential gap

- [ ] **Step 2: Check the remaining 11 outside-emCore files**

The marker mentions 19 total files. 8 are flagged above. Find the other 11:

```bash
grep -rl "emThread" ~/git/eaglemode-0.96.4/include/ ~/git/eaglemode-0.96.4/src/ \
  --include='*.h' --include='*.cpp' 2>/dev/null | grep -v "/emCore/" | sort
```

For each additional file, do the same categorization.

- [ ] **Step 3: Update emThread.no_rs**

Replace the NOT VERIFIED item (lines 218-228) with verified findings. For each file:
```
  <filename>: uses <primitive> for <purpose>. Maps to <Rust equivalent>.
```

If all usage maps cleanly to std::thread/std::sync, add:
```
Conclusion: All 19 outside-emCore files use emThread primitives in
standard patterns (spawn/join, mutex lock/unlock, event signal/wait).
All map to std::thread / std::sync equivalents. No gap found.
```

If any file has a pattern that doesn't map cleanly, document the specific gap.

- [ ] **Step 4: Commit**

```bash
git add src/emCore/emThread.no_rs
git commit -m "docs: close emThread outside-emCore audit

Verified 19 outside-emCore files use standard threading patterns.
All map to std::thread / std::sync equivalents."
```

---

## Task 3: emAvlTree three NOT VERIFIED items

**Files:**
- Modify: `src/emCore/emAvlTree.no_rs`

**Context:** Three NOT VERIFIED items:
1. (line 105) "whether the Rust emContext.rs iteration depends on any particular ordering of its HashMap"
2. (line 108) "whether the 8 outside-emCore files use emAvlTree raw macros directly or only through emAvlTreeMap/emAvlTreeSet"
3. (line 113) "whether any code calls emAvlCheck() outside emCore"

- [ ] **Step 1: Check emContext.rs HashMap iteration ordering**

Read `src/emCore/emContext.rs` and find all HashMap iteration sites (`.iter()`, `.values()`, `.keys()`, `for ... in`). For each site, determine if the iteration result depends on ordering.

```bash
grep -n "\.iter()\|\.values()\|\.keys()\|for.*in.*map\|for.*in.*hash" \
  src/emCore/emContext.rs | head -20
```

Also read the C++ `emContext.cpp` to see how the AVL tree was iterated:
```bash
grep -n "EM_AVL_LOOP\|EM_AVL_REV_LOOP" \
  ~/git/eaglemode-0.96.4/src/emCore/emContext.cpp | head -10
```

If the Rust code produces output (display, logging) or feeds a deterministic comparison, ordering matters. If it's just "visit all entries," ordering doesn't matter.

- [ ] **Step 2: Check outside-emCore emAvlTree usage — raw macros vs wrappers**

```bash
for f in $(grep -rl "emAvlTree\|EM_AVL" ~/git/eaglemode-0.96.4/include/ \
  ~/git/eaglemode-0.96.4/src/ --include='*.h' --include='*.cpp' 2>/dev/null \
  | grep -v "/emCore/"); do
  echo "=== $(basename $f) ==="
  grep -n "EM_AVL_\|emAvlTree" "$f" | head -5
done
```

Categorize each file:
- Uses `EM_AVL_*` macros directly → uses raw emAvlTree
- Uses `emAvlTreeMap` or `emAvlTreeSet` only → uses wrappers (already ported)

- [ ] **Step 3: Check emAvlCheck() outside emCore**

```bash
grep -rn "emAvlCheck" ~/git/eaglemode-0.96.4/include/ ~/git/eaglemode-0.96.4/src/ \
  --include='*.h' --include='*.cpp' 2>/dev/null | grep -v "/emCore/"
```

Expected: No outside-emCore usage (it's a debug/test function).

- [ ] **Step 4: Update emAvlTree.no_rs**

Replace all three NOT VERIFIED items (lines 105-113) with verified findings. Include specific evidence for each.

- [ ] **Step 5: Commit**

```bash
git add src/emCore/emAvlTree.no_rs
git commit -m "docs: close three emAvlTree NOT VERIFIED items

Verified HashMap iteration ordering in emContext.rs.
Verified outside-emCore emAvlTree usage patterns.
Verified emAvlCheck() has no outside-emCore callers."
```

---

## Task 4: emOwnPtr outside-emCore investigation

**Files:**
- Modify: `src/emCore/emOwnPtr.no_rs`

**Context:** NOT VERIFIED (line 138): "whether all 17 outside-emCore files use emOwnPtr in ways that map cleanly to Option<Box<T>>."

- [ ] **Step 1: Check outside-emCore emOwnPtr usage**

```bash
grep -rl "emOwnPtr\|emOwnArrayPtr" ~/git/eaglemode-0.96.4/include/ \
  ~/git/eaglemode-0.96.4/src/ --include='*.h' --include='*.cpp' 2>/dev/null \
  | grep -v "/emCore/" | sort
```

For each file, check how emOwnPtr is used:

```bash
for f in $(grep -rl "emOwnPtr\|emOwnArrayPtr" ~/git/eaglemode-0.96.4/include/ \
  ~/git/eaglemode-0.96.4/src/ --include='*.h' --include='*.cpp' 2>/dev/null \
  | grep -v "/emCore/" | sort); do
  echo "=== $(basename $f) ==="
  grep -n "emOwnPtr\|emOwnArrayPtr" "$f" | head -5
done
```

Categorize each usage:
- `emOwnPtr<T> member` → `Option<Box<T>>` field
- `emOwnPtr<T> local` → `Box<T>` local variable
- `emOwnArrayPtr<T>` → `Vec<T>`
- Any transfer-of-ownership pattern → check if Rust move semantics cover it

- [ ] **Step 2: Update emOwnPtr.no_rs**

Replace the NOT VERIFIED item (lines 138+) with per-file verified findings.

If all usage maps cleanly:
```
Conclusion: All 17 outside-emCore files use emOwnPtr as owned
single-object pointers (member fields, local variables, function
parameters). All map to Option<Box<T>> or Box<T>. No gap found.
```

- [ ] **Step 3: Commit**

```bash
git add src/emCore/emOwnPtr.no_rs
git commit -m "docs: close emOwnPtr outside-emCore audit

Verified 17 outside-emCore files use standard ownership patterns.
All map to Option<Box<T>> / Box<T>."
```

---

## Task 5: emRef ownership change investigation

**Files:**
- Modify: `src/emCore/emRef.no_rs`

**Context:** NOT VERIFIED (line 134): "whether this ownership change affects any code path that shares an emVarModel across multiple consumers via emRef." The ownership change: C++ uses `emRef<emVarModel<T>>` (refcounted shared model), Rust uses `WatchedVar<T>` as a direct struct field (no refcounting).

- [ ] **Step 1: Find all C++ emVarModel usage**

```bash
grep -rn "emVarModel\|emRef.*emVarModel" ~/git/eaglemode-0.96.4/include/ \
  ~/git/eaglemode-0.96.4/src/ --include='*.h' --include='*.cpp' 2>/dev/null | head -30
```

For each usage, determine:
- Is the emVarModel shared across multiple consumers via emRef? (If so, Rust's direct ownership breaks sharing.)
- Or is it owned by a single object? (Rust's direct ownership is fine.)

- [ ] **Step 2: Check Rust WatchedVar usage**

```bash
grep -rn "WatchedVar" src/emCore/ --include='*.rs' | head -20
```

Verify that no Rust code shares a WatchedVar across multiple owners (which would indicate the ownership change has a gap).

- [ ] **Step 3: Update emRef.no_rs**

Replace the NOT VERIFIED item (lines 134-135) with verified findings.

- [ ] **Step 4: Commit**

```bash
git add src/emCore/emRef.no_rs
git commit -m "docs: close emRef ownership change investigation

Verified emVarModel/WatchedVar ownership pattern.
[Summary of whether sharing gap exists.]"
```

---

## Task 6: emString COW semantics investigation

**Files:**
- Modify: `src/emCore/emString.no_rs`

**Context:** NOT VERIFIED (line 267): "whether any of the 300 outside-emCore files rely on cheap copy semantics." C++ `emString` has COW (copy is O(1) refcount bump). Rust `String::clone()` is always O(n) deep copy.

- [ ] **Step 1: Assess the risk**

This is a performance question, not a correctness question. Rust `String::clone()` produces the same result as C++ COW copy — the only difference is cost. The question is whether any C++ code path relies on O(1) string copy being cheap enough to call in a hot loop.

Check the most likely hot paths:

```bash
# Find emString copy in tight loops in outside-emCore code
grep -rn "emString.*=.*emString\|emString.*copy\|emString.*clone" \
  ~/git/eaglemode-0.96.4/src/ --include='*.cpp' 2>/dev/null \
  | grep -v "/emCore/" | head -20
```

Also check:
```bash
# Find emString passed by value (triggers copy) in frequently-called functions
grep -rn "const emString &\|emString " \
  ~/git/eaglemode-0.96.4/include/ --include='*.h' 2>/dev/null \
  | grep -v "/emCore/" | head -30
```

Note: C++ `const emString &` is pass-by-reference (no copy). Only pass-by-value triggers a copy. C++ convention is overwhelmingly const-ref for strings.

- [ ] **Step 2: Update emString.no_rs**

Replace the NOT VERIFIED item (lines 267-268) with the finding. Expected conclusion:

```
Verified: C++ code overwhelmingly passes emString by const reference
(no copy). String copies occur at construction and assignment, which
are O(n) in Rust but happen at the same code points. No hot-loop
O(1)-copy-dependent pattern found. Performance difference is
negligible for the string sizes used in Eagle Mode (file paths,
UI labels, config keys — all short strings).
```

- [ ] **Step 3: Commit**

```bash
git add src/emCore/emString.no_rs
git commit -m "docs: close emString COW semantics investigation

Verified no hot-loop dependence on O(1) string copy.
C++ overwhelmingly uses const-ref passing."
```

---

## Task 7: Final CORRESPONDENCE.md update and emPainterDrawList resolution

**Files:**
- Modify: `src/emCore/CORRESPONDENCE.md`
- Possibly modify: `src/emCore/emPainterDrawList.rust_only`

**Context:** This task finalizes CORRESPONDENCE.md after all NOT VERIFIED items are closed. It also resolves the spec question: "Does emPainterDrawList.rs represent the Rust replacement for emThread's threading role?" Based on investigation in Tasks 1-6, the answer is: emPainterDrawList.rs replaces the *rendering pipeline pattern* that C++ emThread enabled, not emThread itself. emThread is replaced by std::thread/std::sync. emPainterDrawList is the *consequence* of Rust's ownership model preventing the C++ threading approach.

The spec says: "If yes, rename emPainterDrawList.rs -> emThread.rs with DIVERGED: comment." However, renaming would violate the emPainter firewall (emPainterDrawList.rs is referenced by emPainter.rs, and changing its module name would require modifying `use` statements in emPainter.rs). Instead, this task documents the relationship in CORRESPONDENCE.md and the rust_only marker.

- [ ] **Step 1: Verify no empainter-deferred-refactors.log exists**

```bash
ls docs/empainter-deferred-refactors.log 2>&1
```

Expected: "No such file or directory" — no deferred refactors were logged during Phases 1-3.

- [ ] **Step 2: Verify all NOT VERIFIED items are closed**

```bash
grep -rn "NOT VERIFIED" src/emCore/*.no_rs src/emCore/*.rust_only
```

Expected: Only the section header "NOT VERIFIED" in reviewed summary headers (not actual open items). If any remain, they must be closed before proceeding.

- [ ] **Step 3: Count files for state-of-the-port update**

```bash
# Count .rs files
find src/emCore/ -name '*.rs' | wc -l

# Count .no_rs files
find src/emCore/ -name '*.no_rs' | wc -l

# Count .rust_only files
find src/emCore/ -name '*.rust_only' | wc -l
```

- [ ] **Step 4: Update CORRESPONDENCE.md**

Add Phase 4 section after the Phase 3 section:

```markdown
### Phase 4 changes (2026-03-28)

NOT VERIFIED items closed — 6 marker files updated:
- emPainterDrawList.rust_only — DrawList safety invariant #2 verified
  (or risk documented). [Use actual finding from Task 1.]
- emThread.no_rs — 19 outside-emCore files verified, all use standard
  threading patterns mapping to std::thread/std::sync.
- emAvlTree.no_rs — HashMap ordering in emContext.rs verified.
  Outside-emCore files categorized (raw macros vs wrappers).
  emAvlCheck() has no outside-emCore callers.
- emOwnPtr.no_rs — 17 outside-emCore files verified, all map to
  Option<Box<T>> / Box<T>.
- emRef.no_rs — emVarModel/WatchedVar ownership change verified.
- emString.no_rs — COW performance impact verified as negligible.

emPainterDrawList.rs resolution:
- emPainterDrawList.rs is NOT a rename of emThread. It is an
  architectural divergence caused by Rust's ownership model (Rc is
  not Send). C++ emThread is replaced by std::thread/std::sync.
  emPainterDrawList.rs replaces the rendering pipeline pattern that
  C++ emThread enabled. The .rust_only marker remains — this is
  genuinely Rust-only code with no C++ equivalent.
- Rename to emThread.rs rejected: would violate emPainter firewall
  (emPainter.rs imports emPainterDrawList) and would be misleading
  (DrawList is not a thread abstraction).

No empainter-deferred-refactors.log entries: no deferred refactors
were logged during Phases 1-3.
```

Update the "State of the port" section at the top — the .rs and .no_rs counts should not change (no new ports in Phase 4). Update the "All 11 marker files" line to reflect that all marker files now have fully verified reviewed summaries with no remaining NOT VERIFIED items.

Update the "BreakCrossPtrs timing" section — if the NOT VERIFIED item there was already closed in a previous phase, mark it as resolved. If not, it should have been addressed in Tasks 1-6.

Strike out resolved items in cross-cutting sections:
- In "Architectural divergence chain": add resolution note about emPainterDrawList
- In "Encoding risk": already resolved in Phase 3

- [ ] **Step 5: Run full test suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr && \
  cargo test --test golden -- --test-threads=1
```

- [ ] **Step 6: Commit**

```bash
git add src/emCore/CORRESPONDENCE.md
git commit -m "docs: update CORRESPONDENCE.md for Phase 4 completion

All NOT VERIFIED items closed across all marker files.
emPainterDrawList.rs relationship to emThread documented.
Port completion state: [N] .rs files, [N] .no_rs, 1 .rust_only."
```

- [ ] **Step 7: Report findings**

Summarize:
- All NOT VERIFIED items closed (list each with one-line finding)
- emPainterDrawList resolution (not a rename of emThread)
- Final marker file counts
- Any risks or open items discovered during investigation
- Whether the port is complete (all .no_rs verified, all .rust_only justified)
