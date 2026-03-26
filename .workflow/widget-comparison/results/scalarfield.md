# ScalarField Audit Report

**Date**: 2026-03-18
**Agent**: Batch 2
**C++ files**: emScalarField.cpp (527 LOC) + emScalarField.h (236 LOC) = 763 LOC
**Rust file**: scalar_field.rs (982 LOC)

## Findings: 10 total (+ systemic CC refs)

### [HIGH] Value type is f64 instead of i64 — **INTENTIONAL DIVERGENCE 2026-03-18**
- **C++**: `emInt64` (i64) for value/min/max. Values are always integers; fractional display is achieved via TextOfValueFunc formatters that divide by a scale factor (e.g., value 5000 displays as "50.00%").
- **Rust**: `f64` for value/min/max. Values are direct floating-point; no scale factor needed.
- **Justification**: The Rust codebase uses ScalarField with direct fractional ranges (`1.0..32.0`, `-200.0..200.0`) in core_config_panel and ColorField expansion. Converting to i64 would require every call site to scale values to integer ranges and add custom formatters — a pervasive change for no behavioral benefit. The f64 approach is simpler and supports the same display precision.
- **What's lost**: C++ integer snapping (value always lands exactly on an integer). Rust values can land on non-integer f64 values during drag. StepByKeyboard partially compensates by rounding to mark intervals. For the current Rust usage patterns (configuration sliders with small ranges), this difference is invisible.
- **If i64 is needed later**: the change touches value/min/max fields, all comparisons (use == not epsilon), StepByKeyboard (pure integer division), check_mouse return type, mark iteration, golden test data, and ~15 call sites in core_config_panel + color_field.

### [HIGH] Drag behavior completely different — absolute vs relative — **FIXED**
- **Fix**: Drag now uses absolute positioning via `check_mouse`, converting mouse position to value on every frame, matching C++ `CheckMouse` behavior.

### [MEDIUM] hit_test uses normalized space, input uses panel-space coords — **FIXED**
- **Fix**: `hit_test` removed; `check_mouse` now handles both hit detection and value computation in panel-space coords, matching C++.

### [MEDIUM] check_mouse doesn't apply marks_never_hidden culling — **NOTE**
- `MarksNeverHidden` is not used in C++ `DoScalarField`; Rust layout matches C++ in this regard. Not an actionable divergence.

### [MEDIUM] Arrow keys (Left/Right) accepted as increment/decrement — **FIXED**
- **Fix**: Removed ArrowLeft/ArrowRight, only +/- character keys matching C++.

### [MEDIUM] Missing IsEnabled() check on input (only checks editable) — **FIXED**
- **Fix**: Input gating now checks both `is_editable()` and `is_enabled()` matching C++.
- See CC-03
- **Confidence**: high | **Coverage**: uncovered

### [LOW] VCT_MIN_EXT missing (see CC-04) — **FIXED**

### [LOW] set_* methods don't fire signals (see CC-02) — **NOTE**
- C++ `SetMinValue`/`SetMaxValue` call `SetValue` for clamping, which fires the value signal. Rust `set_min_value`/`set_max_value` already call `set_value` internally, which fires `on_value_changed`. Behavior matches C++.

### [LOW] HowTo text built at paint-time (string alloc per frame) — **NOTE**
- C++ `GetHowTo()` also builds the HowTo string dynamically on every call (no caching). The allocation pattern is therefore the same as C++; this is not a divergence.
- **Confidence**: medium | **Coverage**: uncovered

### [LOW] preferred_size uses hardcoded dims vs C++ tallness-based — **FIXED**
- **Fix**: `preferred_size` now uses `best_label_tallness()` to derive height matching C++ tallness-based computation.
- **Confidence**: low | **Coverage**: N/A

## Summary

| Severity | Count |
|----------|-------|
| HIGH | 2 |
| MEDIUM | 4 |
| LOW | 4 |

## Most Critical
1. **f64 vs i64** — fundamental type change affects snapping behavior
2. **Drag is relative, not absolute** — user-facing interaction change. C++ click-on-scale positions the needle there. Rust only drags from current position.

## Recommended Tests
- Drag-to-position, decrement key, StepByKeyboard with intervals, disabled input blocking, custom formatter rendering, mark culling
