# Group A+B: PaintImageColored Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix `PaintImageColored` glyph rendering to match C++ at tol=0, fixing 20 Group A+B golden tests.

**Architecture:** The C++ applies Color1/Color2 mapping to interpolated grayscale inside the paint scanline function using the hash table pipeline (`PaintScanlineIntG2`: `a = (g*o2+0x800)>>12; pix = h2R[a]`). The Rust applies color mapping before blending using direct arithmetic (`lum_to_color`: `(lum*alpha+127)/255`). These produce ±1 differences. The fix is to match the C++ arithmetic exactly in the Rust color mapping path.

**Tech Stack:** Rust, C++ reference at `~/git/eaglemode-0.96.4/`

**Key files:**
- Rust color mapping: `crates/emcore/src/emPainter.rs` — `PaintImageColored` (line 1331), `lum_to_color` closure (line 1405), `PaintBorderImageColored` (line 2592)
- Rust blend: `crates/emcore/src/emPainterScanlineTool.rs` — `blend_scanline`, `blend_scanline_canvas`
- Rust hash: `crates/emcore/src/emColor.rs` — `blend_hash_lookup` (line 60)
- C++ color mapping: `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlPSInt.cpp` — `PaintScanlineIntG2` (HAVE_GC2 path, lines 301-312 for CHANNELS>1, lines 338-345 for CHANNELS=1)
- C++ hash setup: `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlPSInt.cpp` — lines 205-226 (h1R/h2R/hR setup)
- C++ scanline init: `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTl.cpp:136-154` — sets Color1, Color2, o1, o2 opacity values

---

### Task 1: Record baseline and understand the C++ color mapping pipeline

- [ ] **Step 1: Record baseline**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | grep 'test result:'
```

Expected: 204 passed, 37 failed.

- [ ] **Step 2: Understand how C++ applies Color1/Color2 to grayscale pixels**

For font glyphs, `PaintText` calls `PaintImageColored` with `color1=TRANSPARENT(0)`, `color2=textColor`. The C++ creates an `emImageColoredTexture` which goes through `ScanlineTool::Init` (emPainter_ScTl.cpp:136-154):

```cpp
case emTexture::IMAGE_COLORED:
    Color1 = texture.GetColor1();  // TRANSPARENT for font glyphs
    Color2 = texture.GetColor2();  // text color
    if (!Color1.GetAlpha()) {
        // HAVE_GC2 path — only Color2 matters
        PaintScanline = psFuncPtr[PSF_INT_G2];
    }
```

Then in the paint scanline function (`PaintScanlineIntG2`), for CHANNELS=1 (font glyphs are 1-channel):

```cpp
// h2R is set up at: (Pixel*)pf.RedHash + Color2.GetRed() * 256
// This points to hash row for Color2's red value
// h2R[a] = hash[Color2.Red * 256 + a] << redShift

unsigned g = s[0];           // interpolated grayscale byte (0-255)
unsigned a = (g * o2 + 0x800) >> 12;   // scale by opacity, 12-bit rounding
if (!a) continue;            // skip transparent
Pixel pix = h2R[a] + h2G[a] + h2B[a]; // hash lookup per channel
```

Where `o2` is the Color2 opacity, computed from `Alpha * Color2.GetAlpha()` (the texture alpha combined with Color2 alpha). The hash lookup `h2R[a]` is `BLEND_HASH[Color2.Red][a] << redShift` — i.e., `blend_hash_lookup(Color2.Red, a)` in the Rust BLEND_HASH terms.

Then the pixel is composited:
- Full opacity (CHANNELS=1, HAVE_GC2 only): `if (a >= 255) *p = pix; else { blend with existing }`
- Canvas blend (HAVE_CVC): `pix -= hcR[a]+hcG[a]+hcB[a]; *p += pix;`

- [ ] **Step 3: Understand how Rust applies Color1/Color2**

In Rust `PaintImageColored` (emPainter.rs:1405-1417), `lum_to_color`:

```rust
let lum_to_color = |lum: u8| -> emColor {
    if color1.IsTotallyTransparent() {
        // Font glyph case: color1=TRANSPARENT, color2=textColor
        let a = (lum as u32 * color2.GetAlpha() as u32 + 127) / 255;
        emColor::rgba(color2.GetRed(), color2.GetGreen(), color2.GetBlue(), a as u8)
    } else if color2.IsTotallyTransparent() {
        let inv = 255 - lum;
        let a = (inv as u32 * color1.GetAlpha() as u32 + 127) / 255;
        emColor::rgba(color1.GetRed(), color1.GetGreen(), color1.GetBlue(), a as u8)
    } else {
        let t = lum as f64 / 255.0;
        color1.GetBlended(color2, t * 100.0)
    }
};
```

The result is a straight-alpha RGBA color passed through `blend_scanline` (NOT `blend_scanline_premul` for this path — check which is called at line 1486).

- [ ] **Step 4: Identify the arithmetic differences**

Key differences between C++ and Rust for the `color1=TRANSPARENT, color2=textColor` case:

1. **Alpha computation:** C++ uses `(g * o2 + 0x800) >> 12` where `o2` is a 12-bit scaled opacity. Rust uses `(lum * color2.GetAlpha() + 127) / 255`. These are different scaling/rounding.

2. **Color channel output:** C++ uses hash table lookup `h2R[a] = blend_hash_lookup(Color2.Red, a)`. Rust passes Color2.Red directly as the color channel, with `a` as the alpha. The hash table round-trip changes the effective color value.

3. **Blend path:** C++ composites via `pix = h2R[a]+h2G[a]+h2B[a]` (premultiplied packed pixel) then canvas-blend or direct write. Rust composites via `blend_scanline` with straight-alpha RGBA.

The `o2` value setup needs investigation. Read `emPainter_ScTlPSInt.cpp` lines 205-226 to find how `o1`, `o2`, `o` are computed from `Color1.GetAlpha()`, `Color2.GetAlpha()`, and the texture `Alpha`.

---

### Task 2: Trace a specific divergent pixel

- [ ] **Step 1: Pick a simple failing test**

```bash
DUMP_GOLDEN=1 cargo test --test golden widget_checkbox_unchecked -- --test-threads=1 2>&1
```

From the error output, get the first divergent pixel coordinates and actual vs expected RGB values.

- [ ] **Step 2: Determine which glyph produces the divergent pixel**

The divergent pixels should be in the HowTo pill region (bottom of widget border). Using the pixel coordinates, determine which character in the HowTo text string produces that pixel. The HowTo text position can be computed from `emBorder::paint_border` → `PaintTextBoxed` → `PaintText` → per-character `PaintImageColored`.

- [ ] **Step 3: Compute what C++ would produce for that pixel**

Using the C++ formulas from Task 1 Step 2, manually compute:
1. The interpolated grayscale value `g` (from the font atlas at the glyph position)
2. `o2` (from the texture Alpha and Color2 alpha)
3. `a = (g * o2 + 0x800) >> 12`
4. `pix_R = blend_hash_lookup(Color2.Red, a)`
5. `pix_G = blend_hash_lookup(Color2.Green, a)`
6. `pix_B = blend_hash_lookup(Color2.Blue, a)`
7. Final composited pixel (canvas blend or direct write)

Compare against the Rust actual value from the error output. The difference should be ±1-2 per channel.

- [ ] **Step 4: Compute what Rust produces for the same pixel**

Using the Rust `lum_to_color` formula:
1. Same `g` (same atlas, same interpolation after area sampling fix)
2. `a = (g * Color2.GetAlpha() + 127) / 255`
3. Color = `rgba(Color2.Red, Color2.Green, Color2.Blue, a)`
4. Through `blend_scanline` → `canvas_blend` or source-over

Compare the intermediate values (g, a, color channels) between C++ and Rust.

---

### Task 3: Fix the color mapping

Based on findings from Task 2, fix the Rust `lum_to_color` and/or the blend path to match C++.

**Files:**
- Modify: `crates/emcore/src/emPainter.rs` — `lum_to_color` closure in `PaintImageColored`
- Possibly modify: `crates/emcore/src/emPainterScanlineTool.rs` — blend functions

- [ ] **Step 1: Fix the identified divergence**

The fix depends on what Task 2 finds, but the most likely needed changes:

- Replace `(lum * alpha + 127) / 255` with the C++ `(g * o2 + 0x800) >> 12` formula (requires computing `o2` the same way C++ does)
- Replace direct color channel pass-through with `blend_hash_lookup(color_ch, a)` to match the hash table round-trip
- Or restructure to emit premultiplied pixel values matching C++'s `h2R[a]+h2G[a]+h2B[a]` and blend via the premul path

- [ ] **Step 2: Run the full golden suite**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | grep 'test result:'
```

Pass count must be >= 204. Check how many Group A+B tests now pass.

- [ ] **Step 3: If tests still fail, investigate remaining divergences**

The fix might resolve all 20 tests if they all diverge at the same color mapping step, or only some if there are additional divergence points (e.g., the two-color case `color1.GetBlended(color2, t*100.0)` vs C++ `HAVE_GC1 && HAVE_GC2` hash formula).

Repeat Task 2 for any remaining failures.

---

### Task 4: Handle PaintBorderImageColored

`PaintBorderImageColored` (emPainter.rs:2592) also uses the same color mapping pipeline. Verify it's fixed by the same change, or apply the fix there too.

- [ ] **Step 1: Check if PaintBorderImageColored uses the same lum_to_color**

```bash
grep -n 'lum_to_color\|PaintBorderImageColored' crates/emcore/src/emPainter.rs | head -20
```

If it has its own color mapping code, apply the same fix.

- [ ] **Step 2: Run golden suite to verify**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | grep 'test result:'
```

---

### Task 5: Final verification and commit

- [ ] **Step 1: Run full golden suite**

```bash
cargo test --test golden -- --test-threads=1
```

Expected: >= 224 passed (204 + 20 Group A+B), <= 17 failed.

- [ ] **Step 2: Run clippy, nextest, parallel_benchmark**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
cargo test --test golden parallel_benchmark -- --test-threads=1
```

All must pass.

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emPainter.rs crates/emcore/src/emPainterScanlineTool.rs
git commit -m "fix(text): match C++ PaintScanlineIntG2 color mapping in PaintImageColored

[describe the specific fix applied]

C++ reference: emPainter_ScTlPSInt.cpp PaintScanlineIntG2 (HAVE_GC2 path)
Fixes: 20 Group A+B tests (HowTo pill text glyph rendering)"
```

---

## Critical Rules

1. **Full suite after every code change.** Pass count must never decrease below 204.
2. **Read the actual C++ source.** The C++ color mapping pipeline (`PaintScanlineIntG2`) is the truth. The macro expansions and hash table setup in `emPainter_ScTlPSInt.cpp` lines 205-226 define the exact arithmetic.
3. **The font atlas is identical.** Don't investigate font files — the divergence is in color mapping and blending, not in glyph data.
4. **The interpolation inner loop is correct.** Don't modify `emPainterInterpolation.rs`. The grayscale `g` value from interpolation is correct — the divergence is in what happens to `g` afterwards.
5. **Watch for the two-color case.** `lum_to_color` has three branches: color1-only, color2-only, and both. Font glyphs use color2-only. Other callers of `PaintImageColored` / `PaintBorderImageColored` may use both colors. Fix all branches.
