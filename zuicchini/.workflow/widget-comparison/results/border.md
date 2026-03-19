# Border Audit Report

**Date**: 2026-03-18 (Session 2)
**C++ files**: emBorder.cpp (1460 LOC) + emBorder.h (510 LOC) = 1970 LOC
**Rust file**: border.rs (2676 LOC) — core rendering widget, every other widget renders through it

## Findings: 9 new + 6 verified-OK

### [HIGH] substance_round_rect wrong inset for OBT_RECT and OBT_ROUND_RECT — **FIXED**
- **C++**: emBorder.cpp:634-693 — `d = s * 0.023` for both types
- **Rust**: border.rs:909, 921 — `d = s * 0.006` (~4x too small)
- Substance rect is oversized → affects panel visibility/hit-test logic
- **Confidence**: high | **Coverage**: substance_round_rect not directly golden-tested

### [HIGH] label_space uses post-HowTo width instead of pre-HowTo s value — **FIXED**
- **C++**: emBorder.cpp:901,937 — `s = min(rndW, rndH)` computed ONCE before HowTo shift, reused for labelSpace
- **Rust**: border.rs:1232 — `label_space(rnd_w, rnd_h)` called with post-HowTo `rnd_w` which is smaller
- Label height slightly shorter when has_how_to AND label coexist
- Same bug in content_round_rect (line 1037) and content_rect_unobscured (line 1168)
- **Confidence**: high | **Coverage**: likely uncovered for how_to + label combination

### [HIGH] best_label_tallness ignores icon geometry — **FIXED**
- **C++**: emBorder.cpp:460-464 — returns totalH/totalW including icon contribution
- **Rust**: border.rs:366-395 — only considers caption + description
- Wrong tallness when icon present → Aux panels mis-positioned
- **Confidence**: high | **Coverage**: uncovered for icon + aux combinations

### [MEDIUM] MarginFilled paints inset rect instead of full Clear — **FIXED**
- **C++**: emBorder.cpp:628 — `painter->Clear(color, canvasColor)` fills ENTIRE panel
- **Rust**: border.rs:1631-1638 — `paint_rect(ox, oy, w-2*ox, h-2*oy)` fills only interior
- Margin corners show canvas color instead of bg_color
- **Confidence**: high | **Coverage**: depends on golden test for MarginFilled specifically

### [MEDIUM] OBT_RECT/RoundRect paint fill unconditionally (no transparency check) — **FIXED**
- **C++**: emBorder.cpp:654-662 — skips fill if `IsTotallyTransparent()`, only updates canvasColor on fill
- **Rust**: border.rs:1649-1658 — always paints rect, always sets canvas_color
- Overwrites canvas_color with transparent when bg_color is transparent
- **Confidence**: medium | **Coverage**: covered for opaque, uncovered for transparent bg

### [MEDIUM] Disabled alpha dimming rounding off by 1 — **FIXED**
- **C++**: `alpha * 0.25 + 0.5` (float round). Rust: `alpha * 64 / 255` (int truncation)
- At most 1 alpha unit difference per channel. Sub-pixel.
- **Confidence**: high | **Coverage**: partially covered

### [MEDIUM] label_layout ignores description width for description-only labels — **FIXED**
- **C++**: emBorder.cpp:1252-1274 — uses natural description text width for totalW
- **Rust**: border.rs:596-607 — falls back to `total_w = 1.0` when no caption
- Wrong horizontal positioning for description-only labels (rare)
- Also missing: description width capping to caption width (C++ lines 1264-1267)
- **Confidence**: medium | **Coverage**: uncovered

### [LOW] HowTo pill size check uses panel coords vs view coords — **FIXED**
- Added `pixel_scale: f64` parameter to `paint_border()`. Represents `(viewed_rect.w/w)*(viewed_rect.h/h)`.
- HowTo check changed from `tw * th > 100.0` to `tw * th * pixel_scale > 100.0`.
- All 33 call sites updated: 13 compute scale from PanelState::viewed_rect, 20 pass 1.0 (tests/examples/widgets without view info).

### [LOW] caption_alignment/description_alignment fallback to label_alignment — **CLOSED: Intentional Rust convenience. C++ has three independent fields (LabelAlignment, CaptionAlignment, DescriptionAlignment) all defaulting to EM_ALIGN_LEFT. Rust uses Option<TextAlignment> for caption/description that fall back to label_alignment when None. With all defaults, both produce Left alignment — identical behavior. The divergence only manifests if label_alignment is changed without explicitly setting caption/description alignment. No current consumer does this. The Rust fallback design is more convenient (change one field to affect all) and can always be overridden by setting caption_alignment/description_alignment to Some(value). Not a bug — a deliberate API simplification.**

## Verified-OK (fixes from prior session confirmed)

- **CC-05**: label_alignment defaults to Left — FIXED
- **CC-06**: content_round_rect/content_rect geometry — CORRECT
- **CC-03**: enabled state rendering with dimming — IMPLEMENTED
- Border image 9-slice rendering — ALL slice coordinates match C++
- Look propagation / content_canvas_color tracking — CORRECT
- border_scaling propagation via base_unit — CORRECT
- HowTo text assembly — CORRECT

## Summary

| Severity | Count |
|----------|-------|
| HIGH | 3 |
| MEDIUM | 4 |
| LOW | 2 |
| OK | 6 |

## Most Critical
1. **substance_round_rect coefficient** — 0.006 vs 0.023, affects Rect/RoundRect border types
2. **label_space post-HowTo width** — systemic in content_rect, content_round_rect, content_rect_unobscured
3. **best_label_tallness ignores icons** — affects Aux panel positioning

## Overall: The core DoBorder rendering, 9-slice images, content_rect, and Look system are faithful. The three HIGH findings are in secondary geometry functions (substance_round_rect, label_space with HowTo, icon-aware tallness) that affect layout and hit-testing but not the primary render path.
