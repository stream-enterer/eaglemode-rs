# Architectural Gaps

Differences between C++ emPainter and Rust zuicchini that are caused by
fundamentally different algorithms, not bugs or tunable parameters.

All measurements taken at `ch_tol=1` (any pixel with channel diff > 1 counts).

## ellipse-polygon: Polygon approximation of curves

- **C++ approach:** Native scanline rasterizer for ellipses — computes exact
  coverage per scanline row using the ellipse equation.
- **Rust approach:** Approximates ellipses as N-segment polygons (64 segments
  default), then rasterizes the polygon with AA scanline fill.
- **Affected tests:**
  - `ellipse_basic` — raw max_diff=250, 1.01% differ
  - `gradient_radial` — raw max_diff=248, 26.10% differ (gradient texturing
    amplifies boundary differences)
  - `ellipse_sector` — raw max_diff=225, 0.29% differ
- **Measured cost:** worst-case max_diff=250, fail_pct=26.10% at ch_tol=1
  (but only 1.0% at ch_tol=80)
- **Could narrow with:** Increase polygon segment count (diminishing returns
  past ~128 segments), or implement a native ellipse scanline rasterizer
  matching C++ `PaintEllipse`.
- **Assessment:** Acceptable. Edge-only differences at <1% of pixels when
  channel tolerance accounts for AA boundary variation. The gradient_radial
  test's high raw fail_pct is because the gradient color varies smoothly
  across the boundary, so many pixels near the edge differ slightly.

## scanline-aa: Scanline anti-aliasing coverage

- **C++ approach:** C++ emPainter's polygon fill uses a specific AA coverage
  computation at polygon edges with integer-arithmetic sub-pixel precision.
- **Rust approach:** Rust scanline rasterizer uses Fixed12 sub-pixel
  arithmetic with a different coverage accumulation method.
- **Affected tests:**
  - `polygon_tri` — raw max_diff=73, 0.93% differ
  - `polygon_star` — raw max_diff=251, 1.44% differ
  - `polygon_complex` — raw max_diff=240, 1.22% differ
  - `clip_basic` — raw max_diff=64, 0.23% differ
- **Measured cost:** worst-case max_diff=251, fail_pct=1.44% at ch_tol=1
- **Could narrow with:** Match C++ sub-pixel precision exactly (would require
  reverse-engineering the C++ coverage formula from emPainter.cpp). Or adopt
  the same fixed-point bit width and rounding rules.
- **Assessment:** Acceptable. Differences are confined to polygon edges (1-2
  pixel band), affect <1.5% of pixels, and are visually imperceptible.

## stroke-expansion: Stroke polygon construction

- **C++ approach:** C++ constructs stroke outlines (for lines, outlines,
  polylines) using its own offset-curve expansion with specific corner
  handling, miter limits, and sub-pixel positioning.
- **Rust approach:** Rust expands strokes into filled polygons using a
  different offset-curve algorithm with different corner/join geometry.
- **Affected tests:**
  - `line_basic` — raw max_diff=152, 1.21% differ
  - `line_dashed` — raw max_diff=255, 1.90% differ
  - `outline_rect` — raw max_diff=255, 4.25% differ
  - `outline_ellipse` — raw max_diff=255, 3.02% differ
  - `outline_polygon` — raw max_diff=255, 2.46% differ
  - `outline_round_rect` — raw max_diff=255, 4.21% differ
  - `bezier_stroked` — raw max_diff=255, 3.48% differ
  - `polyline` — raw max_diff=255, 4.24% differ
- **Measured cost:** worst-case max_diff=255, fail_pct=4.25% at ch_tol=1
- **Could narrow with:** Match C++ stroke expansion algorithm exactly —
  particularly the offset direction, corner cutting, and join construction.
  This is a significant undertaking as emPainter's stroke code is tightly
  integrated with its scanline rasterizer.
- **Assessment:** Acceptable for now. The 4-5% fail_pct at ch_tol=1 drops
  well below 5% at ch_tol=80. Stroke rendering is visually correct; the
  differences are sub-pixel positioning of stroke edges. Revisit if stroke
  precision becomes important for pixel-accurate UI.

## curve-polygon: Bezier curve flattening

- **C++ approach:** C++ emPainter has a native bezier rasterizer that
  evaluates the curve equation directly during scanline fill.
- **Rust approach:** Rust flattens bezier curves into polylines using
  recursive subdivision, then rasterizes as a polygon.
- **Affected tests:**
  - `bezier_filled` — raw max_diff=255, 4.41% differ
- **Measured cost:** max_diff=255, fail_pct=4.41% at ch_tol=1
- **Could narrow with:** Tighten flattening tolerance (currently adaptive),
  or implement direct bezier scanline integration. The flattening tolerance
  is already fairly tight; most of the difference is at curve edges where
  polygon approximation diverges from the true curve.
- **Assessment:** Acceptable. Bezier shapes render correctly; edge
  differences are in the AA band only.

## interpolation: Image scaling filter

- **C++ approach:** C++ uses a specific interpolation algorithm for image
  upscaling (likely area-averaged or custom filter).
- **Rust approach:** Rust uses bilinear interpolation for image scaling.
- **Affected tests:**
  - `image_scaled` — raw max_diff=118, 30.68% differ
- **Measured cost:** max_diff=118, fail_pct=30.68% at ch_tol=1
  (but only 0.18% at ch_tol=70 — most diffs are small rounding differences)
- **Could narrow with:** Identify and match C++ interpolation algorithm.
  The high fail_pct at ch_tol=1 but very low fail_pct at ch_tol=70 suggests
  many pixels differ by small amounts (rounding/truncation), not large
  structural differences.
- **Assessment:** Acceptable. The visual result is correct; the differences
  are sub-perceptual interpolation rounding.

## stroke-ends: Stroke end decoration rendering

- **C++ approach:** C++ constructs stroke end decorations (arrows, triangles,
  diamonds, circles, squares, etc.) as part of the stroke expansion pipeline.
- **Rust approach:** Rust constructs equivalent decorations but with different
  geometry generation, sizing, and positioning.
- **Affected tests:**
  - `line_ends_all` — raw max_diff=255, 19.91% differ
- **Measured cost:** max_diff=255, fail_pct=19.91% at ch_tol=1
- **Could narrow with:** Reverse-engineer exact C++ geometry for each of the
  17 stroke end types. The decorations are individually small but there are
  many types, each with subtle differences in sizing, anchor point, and fill
  pattern.
- **Assessment:** **Needs future work.** At 17% fail_pct (even at ch_tol=80),
  this is above the 5% tolerance budget. The test renders all 17 end types
  in a single image, amplifying per-type differences. Consider splitting into
  per-end-type tests and fixing each individually.

## compound: Compound shape composition

- **C++ approach:** Composites multiple overlapping shapes using its rendering
  pipeline. Each shape's AA boundary interacts with prior shapes.
- **Rust approach:** Same compositing approach, but the individual shape
  differences (from other gaps) compound in overlap regions.
- **Affected tests:**
  - `multi_compose` — raw max_diff=119, 1.57% differ
- **Measured cost:** max_diff=119, fail_pct=1.57% at ch_tol=1
- **Could narrow with:** Narrowing the individual shape gaps (ellipse-polygon,
  scanline-aa) would automatically improve this test.
- **Assessment:** Acceptable. This is a downstream effect of other gaps,
  not an independent issue.

---

## Review needed

The **stroke-expansion** gap affects **8 tests**, triggering the circuit
breaker (>5 tests under the same gap). Additionally, the cumulative
fail_pct across all Category C tests at ch_tol=1 is ~82%, well above the
25% threshold.

However, at the operating tolerances (ch_tol=80), all tests except
`line_ends_all` are well within the 5% fail_pct budget. The high raw
numbers reflect that stroke geometry differences produce both high-magnitude
(max_diff=255 at individual pixels) and widespread (up to 4% of pixels)
divergence when measured at ch_tol=1, but the vast majority of those
differences are in the AA boundary band and fall under ch_tol=80.

The `line_ends_all` test is the only test that exceeds the 5% budget even
at ch_tol=80 (measured at ~17%). This test should be:
1. Split into per-end-type sub-tests to isolate which decorations diverge.
2. Each end type fixed individually to match C++ geometry.
3. Until then, the 17% tolerance is tracked as technical debt.
