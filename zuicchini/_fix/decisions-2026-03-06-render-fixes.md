# Render Pipeline Decisions — 2026-03-06

Context: After porting Eagle Mode's emTestPackLayout as a zuicchini example
(commit 81d4574), we surfaced and fixed several rendering bugs. This session
addressed 4 remaining issues identified during visual comparison with Eagle
Mode. Each issue was researched via subagents reading both the zuicchini and
Eagle Mode (~/.local/git/eaglemode-0.96.4) source code before any fix was
attempted.

Commits prior to this session:
- 81d4574 — Port emTestPackLayout example + fix 3 rendering bugs
- 55ca841 — Fix additive color blending and vertically flipped glyph rendering
- 0318f45 — Fix text anti-aliasing: use source-over blend instead of canvas_blend

---

## 1. Font Text Scaling (border labels)

### Problem

Border labels (caption and description text) were rendered at a hardcoded
`FontCache::DEFAULT_SIZE_PX` (13.0 pixels) regardless of the panel's
dimensions. In Eagle Mode, text scales proportionally with zoom level — large
panels show large text, small panels show small text.

### Research findings

Eagle Mode's `emBorder::DoLabel` (emBorder.cpp:1194) works entirely in
proportional coordinates:

1. Measures text at charHeight=1.0 to get natural width/height ratios.
2. Builds a bounding box in unit-1 space.
3. Computes a uniform scale factor `f = available_h / totalH` to fit the
   bounding box into the available label area.
4. Passes `capH * f` as the font size to `PaintTextBoxed`.

The available label height comes from DoBorder:
`label_space = s * 0.17` where `s = min(rnd_w, rnd_h) * BorderScaling`,
with padding `d = label_space * 0.1`, giving `label_h = label_space - 2*d`.

The zuicchini painter already supports arbitrary font sizes via
`paint_text_boxed(size_px)` and the font cache's `quantize_size()` handles
variable sizes efficiently (integer quantization <= 48px, even-number
quantization above, 16MB LRU cache with eviction).

### Decisions

**Decision 1.1: Proportional label height from panel dimensions.**
Added `Border::label_height(rnd_w, rnd_h)` that computes
`s * 0.17 * 0.8` where `s = min(rnd_w, rnd_h) * border_scaling`. This
matches Eagle Mode's DoBorder label space allocation. The 0.8 factor
accounts for `label_space - 2*d` where `d = label_space * 0.1`.

**Decision 1.2: Font size = 80% of row height.**
Within `label_layout`, the caption font size is `cap_h * 0.8`, leaving
20% as vertical padding within the row. Eagle Mode's DoLabel computes this
differently (measures text at unit size then scales to fit), but the visual
result is similar. The 80% ratio was chosen as a simple approximation that
avoids the complexity of measuring text at a reference size and then
re-scaling — that would require a FontCache reference in label_layout,
which we don't currently have (see TODO: "Thread FontCache through
PanelBehavior/PanelCtx").

Alternative considered: Passing FontCache into label_layout to measure text
and compute exact fit like Eagle Mode's DoLabel. Rejected because it would
require threading FontCache through content_rect, content_round_rect,
get_aux_rect, and preferred_size_for_content — all of which call
label_layout but don't have access to FontCache. This is the right
long-term fix but premature now.

**Decision 1.3: Description height = 15% of caption height.**
Matched Eagle Mode's `descH = capH * 0.15` ratio exactly. In the
proportional unit system, caption gets 1.0 units and description gets 0.15
units.

**Decision 1.4: Removed the +2.0 y-offset hack.**
The old code painted text at `cap_rect.y + 2.0`, a pixel-level offset that
made sense with fixed 17px rows but is wrong with proportional sizing.
With dynamic row heights, the text is positioned at the top of its
proportional row and paint_text_boxed handles centering within the row
height.

**Decision 1.5: MIN_FONT_SIZE = 4.0 pixels.**
Added a floor to prevent text from becoming invisibly small at extreme
zoom-out. Below 4px, individual glyphs are indistinguishable. Eagle Mode
doesn't need this because its DoLabel formula naturally avoids degenerate
sizes, but our simplified 80%-of-row-height formula can produce tiny values
when panels are very small.

**Decision 1.6: Label.preferred_size simplified.**
Label always uses `OuterBorderType::None` (zero insets), so calling
`preferred_size_for_content` was unnecessary overhead. The preferred height
is now `DEFAULT_SIZE_PX / 0.8` (the row height that would produce a 13px
font at the 80% ratio). This is a minor simplification, not a behavior
change — `outer_insets` returns (0,0,0,0) for OuterBorderType::None.

### Files changed

- `zuicchini/src/widget/border.rs` — Replaced TEXT_ROW_HEIGHT with
  label_height(), rewrote label_layout() for proportional sizing, added
  caption_font_size/description_font_size to LabelLayout, updated all
  callers (content_rect, content_round_rect, get_aux_rect,
  preferred_size_for_content, paint_border), updated 4 tests.
- `zuicchini/src/widget/label.rs` — Updated preferred_size() and its test.

---

## 2. Canvas Blend with Transparent Canvas Color

### Problem

`canvas_blend` uses the formula:
`result = target + ((source - canvas) * alpha) / 255`

When canvas_color is TRANSPARENT (rgba(0,0,0,0), the default), this becomes:
`result = target + (source * alpha) / 255` — additive blending.

Semi-transparent fills, alpha overlays, and disabled-state dimming all go
through `blend_pixel` which calls `canvas_blend`, producing brightening
artifacts instead of correct compositing. Two hot paths (blend_pixel for
fully opaque colors, and paint_text's inner loop) were already fixed in
prior commits with direct source-over blending, but all other semi-transparent
painting was still broken.

### Research findings

Eagle Mode's emPainter.h (lines 61-121) documents the two-formula approach:
- When `canvasColor` is **opaque**: uses the additive canvas_blend formula
  for better anti-aliasing at shape edges (shared pixels between adjacent
  shapes get correct coverage).
- When `canvasColor` is **non-opaque**: falls back to standard source-over
  alpha blend `result = target * (1-alpha) + source * alpha`.

In Eagle Mode, canvas_color is passed as an argument to every paint method.
Each panel gets its canvas_color from its parent during Layout(). The key
insight is that Eagle Mode explicitly handles the non-opaque case — it does
NOT blindly apply canvas_blend when canvas is transparent.

In zuicchini, the panel tree already stores and propagates canvas_color
(`PanelData.canvas_color`, `PanelCtx::set_canvas_color`, `view.rs:1396`
sets `painter.set_canvas_color`). Most panels never set it, so it defaults
to TRANSPARENT.

### Options considered

**Option A: Full Eagle Mode API — canvas_color parameter on every paint method.**
Every `paint_rect`, `paint_text`, `paint_round_rect`, etc. would take an
additional `canvas_color: Color` parameter. Every PanelBehavior::paint()
implementation would need to pass the correct canvas color for each draw
call.

Rejected: Enormous API surface change (~30 method signatures, all callers)
for minimal practical benefit. The zuicchini panel tree already propagates
canvas_color to the painter as global state; per-call canvas_color is only
useful for intra-panel edge anti-aliasing (e.g., a panel that draws a white
rect then a blue rect sharing an edge). This is a refinement, not a
correctness issue.

**Option B: Replace canvas_blend with source-over everywhere.**
Remove canvas_blend from blend_pixel entirely, always use source-over.

Rejected: Loses the edge anti-aliasing optimization for panels that DO set
an opaque canvas_color. When two shapes share an edge pixel over a known
background, canvas_blend produces mathematically correct coverage. Throwing
this away for all panels would be a regression for panels that properly
configure their canvas_color.

**Option C (chosen): Hybrid — check canvas_color opacity in blend_pixel.**
When `canvas_color.is_opaque()`, use canvas_blend (correct edge AA).
When `canvas_color` is non-opaque, use standard source-over (correct
compositing without additive artifacts).

This matches Eagle Mode's documented behavior exactly, requires changing
only one function (~20 lines), and is backward-compatible.

### Decision

**Decision 2.1: Hybrid canvas_blend/source-over in blend_pixel.**
Added a three-way branch in `blend_pixel`:
1. Fully opaque color + alpha=255: direct write (existing fast path).
2. Opaque canvas_color: canvas_blend (correct edge anti-aliasing).
3. Non-opaque canvas_color: standard source-over alpha compositing.

This is the minimum correct fix. All semi-transparent painting through
blend_pixel is now correct regardless of whether the panel has set a
canvas_color.

### Files changed

- `zuicchini/src/render/painter.rs` — blend_pixel gains source-over
  fallback branch when canvas_color is non-opaque.

---

## 3. Root Viewport Height Distortion (ROOT_SAME_TALLNESS)

### Problem

With ROOT_SAME_TALLNESS enabled, zuicchini sets `root_vh = vw`, making the
root panel's viewport rect taller than the actual viewport. The concern was
that vertical proportions might be slightly stretched.

### Research findings

The research agent traced through a concrete example (1920x1080 window) in
both Eagle Mode and zuicchini, computing exact child panel positions and
sizes.

Eagle Mode uses an **anisotropic** viewport coordinate system:
- `child.ViewedY = parent.ViewedY + child.LayoutY * parent.ViewedWidth`
- Both X and Y scale by `ViewedWidth`, not ViewedHeight.

Zuicchini uses an **isotropic** viewport coordinate system:
- `child.abs.y = parent.abs.y + child.lr.y * parent.abs.h`
- Y scales by the parent's absolute height.

For child placement to match Eagle Mode (where LayoutY scales by
ViewedWidth), zuicchini must set `root.abs.h = ViewedWidth`. Since the
root's ViewedWidth equals the viewport pixel width, `root_vh = vw`.

Concrete verification for a child at layout_rect (0, 0, 0.5, 0.5):
- Eagle Mode: ViewedWidth = 0.5 * 1920 = 960, ViewedHeight = 0.5 * 1920 = 960
- Zuicchini: abs.w = 0.5 * 1920 = 960, abs.h = 0.5 * 1920 = 960
- **Identical.** The child is a 960x960 square in both systems.

### Decision

**Decision 3.1: No fix needed — behavior is correct.**
`root_vh = vw` is mathematically correct for zuicchini's coordinate model.
Children are positioned and sized identically to Eagle Mode. The root
panel's `viewed_height` (1920) being larger than the viewport (1080) is an
artifact — the bottom portion is clipped away, and all visible children
render at correct proportions.

Minor caveat documented: code that reads `root.viewed_height` for
area-based decisions (SVP selection thresholds, view-condition checks) will
see a larger area than Eagle Mode computes (1920*1920 vs 1920*1080). This
is a minor second-order effect, not a geometric distortion, and can be
addressed if/when those systems are implemented.

### Files changed

None.

---

## 4. Unconditional Repaint in Event Loop

### Problem

`win.invalidate()` in `about_to_wait` called `tile_cache.mark_all_dirty()`
every frame, and `win.request_redraw()` always requested a new frame. With
`ControlFlow::Poll`, this meant the CPU repainted and re-uploaded every tile
to the GPU on every event loop iteration, even when the window was idle.

### Research findings

Eagle Mode uses a demand-driven model:
- `emPanel::InvalidatePainting()` is called explicitly when visual state
  changes. It pushes dirty rects to the viewport.
- The backend accumulates dirty rects and only repaints their union.
- `emView::UpdateEngine` only cycles when explicitly woken by a signal.

Zuicchini already has the infrastructure:
- `View::dirty_rects` accumulates rects from `invalidate_painting()` calls.
- `View::take_dirty_rects()` drains them.
- `PanelTree::deliver_notices()` dispatches layout/canvas/enable changes.
- `ViewAnimator::animate()` returns bool indicating whether animation is
  active.

What was missing: connecting these signals to the repaint decision.

### Options considered

**Full dirty-rect tile intersection:** Map each dirty rect to the specific
tiles it overlaps, only repaint those tiles.

Deferred: Requires non-trivial rect-to-tile intersection math and would add
complexity. The immediate win is eliminating idle-frame waste. Per-tile
dirtying is a future optimization.

**Switch ControlFlow::Poll to ControlFlow::Wait:** Would eliminate CPU
spinning entirely when idle.

Deferred: Requires ensuring all state-change paths (timers, signals, engine
callbacks) call `EventLoopProxy::wake_up()` to break out of Wait. The
scheduler currently assumes Poll mode. This is a separate, larger change.

### Decisions

**Decision 4.1: Gate repaint behind needs_repaint condition.**
`needs_repaint = had_notices || animator_active || has_dirty_rects`.
Only calls `invalidate()` + `request_redraw()` when at least one condition
is true. This eliminates all wasted GPU work when the window is idle (no
input, no timers, no animations).

**Decision 4.2: deliver_notices() returns bool.**
Changed from `-> ()` to `-> bool`, returning whether any notices were
actually dispatched. This serves as a reliable proxy for "panel tree state
may have changed" without adding a separate tracking flag.

**Decision 4.3: Added View::has_dirty_rects().**
Simple pub method returning `!self.dirty_rects.is_empty()`. Avoids
exposing the private `dirty_rects` field while letting App check if any
invalidations are pending.

**Decision 4.4: request_redraw() on Resized.**
`tile_cache.resize()` creates fresh tiles (dirty by default), but
previously nothing triggered a redraw after resize. Added
`win.request_redraw()` in the Resized handler so the new tiles get painted.

**Decision 4.5: Still uses mark_all_dirty() when repainting.**
When `needs_repaint` is true, we still mark ALL tiles dirty rather than
intersecting dirty_rects with tiles. This is conservative — we repaint
everything or nothing. Per-tile selective dirtying is left as a future
optimization since the immediate goal (no idle waste) is achieved.

**Decision 4.6: Kept ControlFlow::Poll.**
The event loop still polls continuously, but now does no GPU work when idle.
Switching to ControlFlow::Wait would save CPU cycles too, but requires
ensuring wake_up() is called from all scheduler/timer paths. Left for a
future pass.

### Files changed

- `zuicchini/src/panel/tree.rs` — deliver_notices() returns bool.
- `zuicchini/src/panel/view.rs` — Added has_dirty_rects().
- `zuicchini/src/window/app.rs` — Gated invalidate()/request_redraw()
  behind needs_repaint; added request_redraw() on Resized.
