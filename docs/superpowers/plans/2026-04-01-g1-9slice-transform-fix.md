# G1 9-Slice Transform Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix `paint_9slice_section`'s transform setup to match C++ `PaintImage` → ScanlineTool::Init, fixing 23 G1 golden tests.

**Architecture:** The C++ `PaintBorderImage` delegates each 9-slice section to `PaintImage`, which creates an `emImageTexture` and routes through `PaintRect` → `ScanlineTool::Init`. The Rust `paint_9slice_section` reimplements this transform setup inline. Compare the two paths parameter-by-parameter, identify divergences, and fix the Rust to match C++. The area sampling inner loop is correct and unchanged.

**Tech Stack:** Rust, C++ reference at `~/git/eaglemode-0.96.4/`

**Key files:**
- Rust: `crates/emcore/src/emPainter.rs` — `paint_9slice_section` (line 2848), `area_sample_transform_24` (line 5946), `PaintBorderImage` (line 2163)
- C++: `~/git/eaglemode-0.96.4/src/emCore/emPainter.cpp:1892-1982` (PaintBorderImage), `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTl.cpp:228-342` (ScanlineTool::Init image setup)
- C++: `~/git/eaglemode-0.96.4/include/emCore/emPainter.h:1026-1037` (PaintImage inline)

---

### Task 1: Record baseline and understand the C++ path

- [ ] **Step 1: Record the current baseline**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | grep 'test result:'
```

Record exact pass/fail count. Expected: 204 passed, 37 failed (after the area sampling inner loop fix).

- [ ] **Step 2: Trace the C++ path for one 9-slice section**

For the upper-left corner of `PaintBorderImage`:

```
C++ PaintBorderImage → PaintImage(x, y, l, t, img, srcX=0, srcY=0, srcW=srcL, srcH=srcT, alpha, canvasColor, EXTEND_EDGE)
  → PaintRect(x, y, l, t, emImageTexture(x, y, l, t, img, 0, 0, srcL, srcT, alpha, EXTEND_EDGE), canvasColor)
    → ScanlineTool::Init:
        sx = max(0, srcX) = 0
        sx2 = min(iw, srcX+srcW) = srcL
        sy = max(0, srcY) = 0
        sy2 = min(ih, srcY+srcH) = srcT
        ImgW = srcL
        ImgH = srcT
        ImgMap = GetMap() + (0*iw+0)*ch = GetMap()   // offset to section start
        ImgDX = ch                                     // pixel stride
        ImgDY = iw*ch                                  // row stride (FULL image width)
        tw = l * ScaleX                                // dest width in pixels
        th = t * ScaleY
        tdx = (ImgW << 24) / tw = (srcL << 24) / (l * ScaleX)
        tdy = (ImgH << 24) / th = (srcT << 24) / (t * ScaleY)
        tx = x * ScaleX + OriginX                     // dest left edge in pixels
        ty = y * ScaleY + OriginY
        // Pre-reduction (if TDX > 0xFFFF00):
        n = (TDX/3 + 0xFFFFFF) >> 24
        reduced_ImgW = (ImgW+n-1)/n
        centering_offset = (ImgW - (reduced_ImgW-1)*n - 1) >> 1
        ImgMap += ImgDX * centering_offset
        ImgDX *= n
        tdx = reduced_ImgW * (1<<24) / tw     // recompute for reduced grid
        TDX = (int64)tdx
        TX = (int64)(tx * tdx)                 // NOTE: tx * tdx, NOT (tx-0.5)*tdx
        TY = (int64)(ty * tdy)
        ODX = ((1<<40)-1)/TDX + 1
        ODY = ((1<<40)-1)/TDY + 1
```

Key points:
- **ImgDY uses FULL image width**, not section width
- **ImgMap** is pointer-offset to the section start pixel
- **Pre-reduction** uses `ImgW` (section width), not full image width
- **TX** is `(int64)(tx * tdx)` where `tx` is dest left edge in screen pixels
- **No `-0.5` offset** for area sampling (contrast with bilinear/adaptive which use `(tx-0.5)*tdx`)

- [ ] **Step 3: Trace the Rust path for the same section**

```
Rust PaintBorderImage → paint_9slice_section(proof, x, y, l, t, image, 0.0, 0.0, sl, st, quality, ext)
  → sw_u = sl as u32 = src_l
    sh_u = st as u32 = src_t
    dw_px = l * scale_x        // dest width in pixels
    dh_px = t * scale_y
    tdx_init = (sw_u << 24) as f64 / dw_px
    tdx_i = tdx_init as i64
    stride_x = ((tdx_i / 3 + 0xFFFFFF) >> 24) as u32
    red_w = sw_u.div_ceil(stride_x)
    off_x = (sw_u - (red_w-1)*stride_x - 1) / 2
    xfm = area_sample_transform_24(red_w, red_h, x, y, l, t)
      → tw = l * scale_x
        tdx_f64 = (red_w << 24) as f64 / tw       // reduced grid
        tx_sub = x * scale_x + offset_x
        TX = (tx_sub * tdx_f64) as i64
    xfm.stride_x = stride_x
    xfm.off_x = off_x
    sec = SectionBounds { ox: 0, oy: 0, w: sl as i32, h: st as i32 }
```

- [ ] **Step 4: Identify the divergences**

Compare each parameter from Step 2 and Step 3. The most likely divergences:

1. **TX computation**: C++ computes `TX = (int64)(tx * tdx)` where `tx = dest_x * ScaleX + OriginX` and `tdx` is the **post-reduction** value. Rust computes `TX = (tx_sub * tdx_f64) as i64` where `tdx_f64 = (red_w << 24) / tw`. These SHOULD match but may differ due to f64 intermediate precision.

2. **Pre-reduction TDX**: C++ computes `tdx = reduced_ImgW * ((emInt64)1<<24) / tw` — this is `int64 / double → double`. Rust computes `tdx_f64 = ((src_w as i64) << 24) as f64 / tw` — this uses `red_w` as `src_w`. Verify `red_w` matches `reduced_ImgW`.

3. **Section bounds vs ImgMap offset**: C++ uses `ImgMap` pointer offset to position within the image, with `ImgDY = full_image_width * channels`. Rust uses `SectionBounds { ox, oy, w, h }` and `read_area_pixel` which computes pixel addresses as `image.GetPixel((sec.ox + rx) as u32, (sec.oy + ry) as u32)`. The Rust `ry` advancement uses `row + 1` (section-relative), while C++ uses `p += imgDY` which advances by full-image-row. **These should be equivalent IF `read_area_pixel` correctly maps reduced grid coordinates through section bounds to image pixels.**

4. **Centering offset**: C++ `ImgMap += ImgDX * (t>>1)` vs Rust `off_x = (sw_u - (red_w-1)*stride_x - 1) / 2`. The C++ `t = ImgW - (reduced_ImgW-1)*n - 1`, so `t>>1 = (ImgW - (reduced_ImgW-1)*n - 1) >> 1`. Rust uses integer `/2`. For non-negative values, `/2` and `>>1` are the same.

5. **`div_ceil` vs C++ rounding**: Rust `red_w = sw_u.div_ceil(stride_x)` = `(sw_u + stride_x - 1) / stride_x`. C++ `reduced_ImgW = (ImgW + n - 1) / n`. These are the same.

6. **Stride computation on unreduced TDX**: The Rust computes `tdx_init` from `sw_u` (unreduced), then `stride_x` from `tdx_init`. C++ computes `TDX` from `ImgW` (unreduced), then `n` from `TDX`. These should produce the same stride. But if the float-to-int64 cast differs by 1 LSB, the stride could differ, changing the entire reduced grid.

---

### Task 2: Generate diff images and locate the divergent pixels

- [ ] **Step 1: Pick a simple failing G1 test**

Start with `widget_checkbox_unchecked` (max_diff=22, 0.04%) — it has few divergent pixels and renders a single widget (simpler than composite tests).

```bash
DUMP_GOLDEN=1 cargo test --test golden widget_checkbox_unchecked -- --test-threads=1 2>&1
```

Examine `target/golden-debug/diff_widget_checkbox_unchecked.ppm`.

- [ ] **Step 2: From the error output, note the divergent pixel coordinates**

The test output shows the first 10 divergent pixels with actual vs expected RGB values and (x,y) coordinates. Record these.

- [ ] **Step 3: Determine which 9-slice section produces those pixels**

The widget is rendered at (0,0)-(800,600) with known border insets. Using the insets and the divergent pixel coordinates, determine which of the 9 sections (UL, U, UR, L, C, R, LL, B, LR) contains the divergent pixels.

- [ ] **Step 4: Add debug prints to `paint_9slice_section`**

Temporarily add `eprintln!` to `paint_9slice_section` at the point where `xfm` and `sec` are computed. Print the key transform parameters: `tdx`, `tdy`, `tx`, `ty`, `odx`, `ody`, `img_w`, `img_h`, `stride_x`, `stride_y`, `off_x`, `off_y`, `sec.ox`, `sec.oy`, `sec.w`, `sec.h`, `start_x`, `end_x`, `start_y`, `end_y`.

Run the failing test and capture the parameters for the divergent section.

- [ ] **Step 5: Manually compute what C++ would produce**

Using the C++ formulas from Task 1 Step 2, compute the C++ transform parameters for the same section. Compare against the Rust output from Step 4.

Any parameter that differs is a candidate bug.

---

### Task 3: Fix the divergence

Based on the findings from Task 2, fix the Rust code.

**Files:**
- Modify: `crates/emcore/src/emPainter.rs` — `paint_9slice_section` and/or `area_sample_transform_24`

- [ ] **Step 1: Fix the identified parameter divergence**

Apply the fix. Common fixes might include:
- Adjusting the TDX/TDY computation to match C++ exactly (e.g., computing from unreduced width first, then recomputing after reduction — matching the C++ two-step)
- Fixing TX/TY computation order (C++ computes `tx * tdx` where `tdx` is the final post-reduction value; Rust may be using a different `tdx` value)
- Fixing section bounds (if `sec.w/sec.h` don't match `ImgW/ImgH` after clipping)

- [ ] **Step 2: Run the full golden suite**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | grep 'test result:'
```

Pass count must be >= 204. Check how many G1 tests now pass.

- [ ] **Step 3: If G1 tests still fail, repeat Task 2 for the remaining failures**

The fix might resolve all 23 tests (if they share the same transform divergence) or only some (if different sections have different issues).

- [ ] **Step 4: Remove debug prints**

Remove any `eprintln!` added in Task 2 Step 4.

---

### Task 4: Final verification and commit

- [ ] **Step 1: Run the full golden suite**

```bash
cargo test --test golden -- --test-threads=1
```

Expected: >= 227 passed (204 + 23 G1), <= 14 failed.

- [ ] **Step 2: Run clippy, nextest, parallel_benchmark**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
cargo test --test golden parallel_benchmark -- --test-threads=1
```

All must pass.

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emPainter.rs
git commit -m "fix(9slice): match C++ PaintImage transform setup in paint_9slice_section

[describe the specific divergence found and fixed]

C++ reference: emPainter_ScTl.cpp:228-342 (ScanlineTool::Init)
Fixes: 23 G1 tests (PaintBorderImage section boundary computation)"
```

---

## Critical Rules

1. **Full suite after every code change.** Pass count must never decrease below 204.
2. **Read the actual C++ source.** `emPainter_ScTl.cpp:228-342` is the transform setup. `emPainter.cpp:1892-1982` is the 9-slice dispatch.
3. **The inner loop is correct.** Do NOT modify `emPainterInterpolation.rs`. The bug is in the transform parameters feeding the inner loop.
4. **Debug with data, not theory.** Generate diff images, print transform parameters, compare against C++ manual computation. Don't guess which parameter is wrong.
