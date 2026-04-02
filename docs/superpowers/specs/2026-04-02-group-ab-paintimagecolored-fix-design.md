# Group A+B: PaintImageColored Glyph Rendering Fix

## Objective

Fix `PaintImageColored` glyph rendering to match C++ for HowTo pill text scale. Fix 20 Group A+B tests to pass at tol=0.

## Background

Both C++ and Rust use the same font rendering approach: look up a 128x224 glyph from a pre-rendered TGA atlas, then paint it via `PaintImageColored`. The atlas files are identical. The `PaintText` call parameters match. The divergence is downstream in `PaintImageColored` — how it scales the glyph image to the target pixel size and blends the result.

For HowTo pill text (very small, ~6.5% opacity), the glyphs are heavily downscaled. The residual max_diff is 13-54 after the HowTo text wiring fix.

## Approach

Trace a single divergent glyph pixel through both C++ and Rust `PaintImageColored` paths. Compare parameters at each stage: texture setup, interpolation method selection (area sampling vs adaptive), section bounds, color mapping (grayscale→color), blend mode. Wherever Rust diverges from C++, match the C++ exactly.

### C++ Reference

- `~/git/eaglemode-0.96.4/src/emCore/emPainter.cpp:2121-2126` — `PaintText` calls `PaintImageColored` per glyph
- `~/git/eaglemode-0.96.4/include/emCore/emPainter.h:1052-1065` — `PaintImageColored` inline → `PaintRect` with `emImageColoredTexture`
- `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTl.cpp:136-154` — ScanlineTool::Init for IMAGE_COLORED texture type (sets Color1, Color2, selects PSF_INT_G1/G2/G1G2)
- `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlPSInt.cpp` — PaintScanlineIntG1G2 applies Color1/Color2 to interpolated grayscale

### Rust Files

- `crates/emcore/src/emPainter.rs:1660-1674` — `PaintText` calls `PaintImageColored` per glyph
- `crates/emcore/src/emPainter.rs` — `PaintImageColored` implementation, `paint_image_full`, `paint_9slice_section`
- `crates/emcore/src/emPainterInterpolation.rs` — interpolation functions
- `crates/emcore/src/emPainterScanlineTool.rs` — blend functions

## The 20 Group A+B Tests

**Group A (15):** colorfield_expanded, listbox_expanded, widget_button_normal, widget_radiobutton, widget_textfield_content, widget_textfield_empty, widget_textfield_single_char_square, widget_listbox_single, widget_listbox_empty, widget_listbox, widget_colorfield, widget_colorfield_alpha_near, widget_colorfield_alpha_opaque, widget_colorfield_alpha_zero, widget_checkbox_unchecked

**Group B (5):** testpanel_expanded, composition_tktest_1x, composition_tktest_2x, widget_file_selection_box, composed_border_nest

## Verification

- `cargo test --test golden -- --test-threads=1` — 20 tests must pass, no regressions
- `cargo clippy -- -D warnings` and `cargo-nextest ntr` must pass
- `parallel_benchmark` must pass

## Constraint

Full golden suite after every change. Pass count must never decrease.
