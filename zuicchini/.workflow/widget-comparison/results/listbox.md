# ListBox Audit Report

**Date**: 2026-03-18
**Agent**: Batch 3
**C++ files**: emListBox.cpp (1075 LOC) + emListBox.h (483 LOC) = 1558 LOC
**Rust file**: list_box.rs (1992 LOC)

## Findings: 14 total

### [MEDIUM] Arrow keys added — not in C++
- **LB-03**: Rust adds explicit ArrowUp/ArrowDown with `focus_index` that C++ doesn't have. C++ uses panel tree zoom-to-focus. Rust auto-selects on arrow in Single mode.
- **Confidence**: high | **Coverage**: uncovered

### [MEDIUM] Hit test vs paint row height mismatch — **FIXED**
- **LB-05**: Input and scroll now use `row_height()` helper that matches paint's `visible_height / items.len()`. Falls back to `ROW_HEIGHT` when empty or before first paint.
- **Confidence**: high | **Coverage**: uncovered

### [LOW] add_item/insert_item don't accept data parameter (LB-01)
### [LOW] sort_items comparator lacks data access (LB-02)
### [LOW] focus_index concept not in C++ (LB-04)
### [LOW] Custom item panels can't intercept input (LB-06)
### [LOW] Inline paint row height may differ from C++ RasterGroup layout (LB-08)
### [LOW] canvasColor for text computed locally vs chained (LB-09)
### [LOW] prev_input_index adjustment correct but fragile (LB-12)
### [LOW] HowTo Multi mode missing keyboard section (LB-14) — **FIXED**
### [LOW] HowTo Toggle mode missing keyboard section (LB-15) — **FIXED**

### [INFO] Scroll model: traditional scrolling vs zoom-to-visit (LB-07)
### [INFO] set_items bulk replacement is Rust addition (LB-13)
### [INFO] prev_input stored as index, manually adjusted on move (LB-11)

## Summary

| Severity | Count |
|----------|-------|
| MEDIUM | 2 |
| LOW | 9 |
| INFO | 3 |

## Most Critical
1. **Row height mismatch (LB-05)** — clicks land on wrong items in non-expanded path
2. **Arrow key addition (LB-03)** — behavioral extension that may conflict with focus navigation
3. **Truncated HowTo text (LB-14, LB-15)** — easy fix, keyboard help text missing

## Overall: Good port. Core selection logic (all 4 modes), keywalk, paint pipeline faithful. Main gaps: hit test geometry, arrow key addition, HowTo truncation.
