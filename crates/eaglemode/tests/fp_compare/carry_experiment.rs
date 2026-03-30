//! Carry-over weight experiment: tests whether a closed-form O(1) formula
//! can reproduce C++ sequential carry behavior without sequential state.
//!
//! Three oracles:
//!   A) sequential_carry  — faithful C++ carry loop (ground truth)
//!   B) independent_rust  — current Rust per-pixel ox computation
//!   C) closed_form       — O(1) carry-equivalent (hypothesis)
//!
//! Run: rustc --edition 2021 tests/fp_compare/carry_experiment.rs -o target/carry_experiment && ./target/carry_experiment

use std::fmt;

// ============================================================================
// Fixed-point helpers matching C++ emPainter_ScTlIntImg.cpp
// ============================================================================

fn rational_inv(span: i64) -> u32 {
    if span <= 0x200 {
        0x7FFF_FFFF
    } else {
        (((1i64 << 40) - 1) / span + 1) as u32
    }
}

/// Derive (tdx, odx) from source and dest widths, matching the real pipeline.
/// C++: tdx = ImgW * (1<<24) / tw, odx = rational_inv(tdx)
fn derive_transform(src_w: i32, dest_w: i32) -> (i64, u32) {
    let tdx = (src_w as i64) << 24 / dest_w as i64;
    // More precise: use f64 like the real code
    let tdx_f64 = ((src_w as i64) << 24) as f64 / dest_w as f64;
    let tdx = tdx_f64 as i64;
    let odx = rational_inv(tdx);
    (tdx, odx)
}

// ============================================================================
// Source data patterns
// ============================================================================

#[derive(Clone, Copy, Debug)]
enum Pattern {
    Gradient,        // column_index % 256
    Checkerboard,    // alternating 0 and 255
    WorstCase,       // 0 at even cols, 255 at odd cols near carry boundaries
}

fn source_pixel(col: i32, pattern: Pattern) -> [u32; 4] {
    match pattern {
        Pattern::Gradient => {
            let v = (col as u32) & 0xFF;
            [v, v, v, 255]
        }
        Pattern::Checkerboard => {
            let v = if col & 1 == 0 { 0u32 } else { 255 };
            [v, v, v, 255]
        }
        Pattern::WorstCase => {
            // Maximum contrast at every column boundary
            let v = if col & 1 == 0 { 0u32 } else { 255 };
            // Different per channel to detect channel-specific errors
            let r = v;
            let g = 255 - v;
            let b = if col % 3 == 0 { 255 } else { 0 };
            [r, g, b, 255]
        }
    }
}

/// Simulate Y-accumulation for a single source column.
/// For simplicity, use 1 source row (oy1 = 0x10000, no multi-row accumulation).
/// This is the minimum needed to test X-carry behavior.
/// Returns (cy_r, cy_g, cy_b, cy_a) in the same 24fp scale as C++.
fn y_accumulate_single_row(col: i32, pattern: Pattern) -> [u64; 4] {
    let p = source_pixel(col, pattern);
    // C++ READ_PREMUL_MUL_COLOR with oy1=0x10000 (full weight, single row):
    // cy = pixel * 0x10000, then FINPREMUL_SHR_COLOR(cy, 8) => cy >>= 8
    // Net: cy = pixel * 0x10000 >> 8 = pixel * 0x100 = pixel << 8
    [
        (p[0] as u64) << 8,
        (p[1] as u64) << 8,
        (p[2] as u64) << 8,
        (p[3] as u64) << 8,
    ]
}

// ============================================================================
// Pixel output
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl fmt::Debug for Pixel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({},{},{},{})", self.r, self.g, self.b, self.a)
    }
}

fn write_pixel(cyx: [u64; 4]) -> Pixel {
    Pixel {
        r: (cyx[0] >> 24) as u8,
        g: (cyx[1] >> 24) as u8,
        b: (cyx[2] >> 24) as u8,
        a: (cyx[3] >> 24) as u8,
    }
}

// ============================================================================
// Oracle A: Sequential carry (faithful C++ port)
// ============================================================================

fn sequential_carry(
    tdx: i64,
    tx_offset: i64,    // C++ TX value
    img_w: i32,
    odx0: u32,
    dest_x_start: i32,
    count: usize,
    pattern: Pattern,
) -> Vec<Pixel> {
    let tx_end = (img_w as i64) << 24;
    let mut result = Vec::with_capacity(count);

    // C++ carry state (persists across pixels within a single call)
    let mut cy: [u64; 4] = [0; 4];
    let mut pcy_col: i32 = i32::MIN; // C++ pCy = NULL initially
    let mut carried_ox: u32 = 0;

    // C++ outer loop: do { ... } while (buf < bufEnd)
    // We process all pixels in one call (no chunking) to match single-threaded C++
    let mut pixel_idx = 0usize;
    while pixel_idx < count {
        let dest_x = dest_x_start + pixel_idx as i32;
        let mut tx1 = dest_x as i64 * tdx - tx_offset;
        let tx2_raw = tx1 + tdx;
        let mut tx2 = tx2_raw;
        let odx: u32;

        // Edge handling (C++ lines 740-776)
        if tx1 < 0 {
            tx1 = 0;
            if tx2 <= 0 {
                // EXTEND_EDGE: tx2 = 1<<24
                tx2 = 1 << 24;
            } else if tx2 > tx_end {
                tx2 = tx_end;
            }
            odx = rational_inv(tx2);
        } else if tx2 > tx_end {
            if tx1 >= tx_end {
                // EXTEND_EDGE
                tx1 = tx_end - (1 << 24);
            }
            odx = rational_inv(tx_end - tx1);
        } else {
            odx = odx0;
        }

        // C++ line 777: ox = ((0x1000000-(tx1&0xffffff))*(emInt64)odx+0xffffff)>>24
        let mut ox_computed: u32 =
            (((0x100_0000i64 - (tx1 & 0xFF_FFFF)) as u64 * odx as u64 + 0xFF_FFFF) >> 24) as u32;
        if odx == 0x7FFF_FFFF {
            ox_computed = 0x7FFF_FFFF;
        }

        // C++ line 779: p0 = row0 + (tx1>>24)*imgDX
        let col0 = (tx1 >> 24) as i32;

        // C++ lines 781-788: pCy coupling
        let ox: u32;
        let mut ox1: u32;
        let mut current_col: i32;

        if pcy_col != col0 {
            // pCy != p0: stale carry, start fresh
            ox1 = ox_computed;
            ox = 0;
            current_col = col0;
        } else {
            // pCy == p0: carry is valid, advance past the carried column
            ox1 = odx;
            ox = carried_ox;
            current_col = col0 + 1; // p0 += imgDX
        }

        // C++ inner loop (lines 790-824): process one output pixel
        let mut cyx: [u64; 4] = [0x7F_FFFF, 0x7F_FFFF, 0x7F_FFFF, 0x7F_FFFF];
        let mut oxs: u32 = 0x10000;
        let mut current_ox = ox;

        // while (ox < oxs)
        while current_ox < oxs {
            // ADD_MUL_COLOR(cyx, cy, ox)
            cyx[0] += cy[0] * current_ox as u64;
            cyx[1] += cy[1] * current_ox as u64;
            cyx[2] += cy[2] * current_ox as u64;
            cyx[3] += cy[3] * current_ox as u64;

            oxs -= current_ox;

            // pCy = p0; READ + Y_ACCUMULATE for column at current_col
            pcy_col = current_col;
            cy = y_accumulate_single_row(current_col.min(img_w - 1).max(0), pattern);

            current_col += 1; // p0 += imgDX
            current_ox = ox1;
            ox1 = odx;
        }

        // ADD_MUL_COLOR(cyx, cy, oxs)
        cyx[0] += cy[0] * oxs as u64;
        cyx[1] += cy[1] * oxs as u64;
        cyx[2] += cy[2] * oxs as u64;
        cyx[3] += cy[3] * oxs as u64;

        result.push(write_pixel(cyx));

        // ox -= oxs (carry to next pixel)
        carried_ox = current_ox - oxs;

        pixel_idx += 1;
    }

    result
}

// ============================================================================
// Oracle B: Independent (current Rust production code)
// ============================================================================

fn independent_rust(
    tdx: i64,
    tx_offset: i64,
    img_w: i32,
    odx0: u32,
    dest_x_start: i32,
    count: usize,
    pattern: Pattern,
) -> Vec<Pixel> {
    let tx_end = (img_w as i64) << 24;
    let mut result = Vec::with_capacity(count);

    // pCy cache (persists across pixels within a call, like Rust production code)
    let mut prev_cy_col: i32 = i32::MIN;
    let mut cached_cy: [u64; 4] = [0; 4];

    for pixel_idx in 0..count {
        let dest_x = dest_x_start + pixel_idx as i32;
        let mut tx1 = dest_x as i64 * tdx - tx_offset;
        let tx2_raw = tx1 + tdx;
        let mut tx2 = tx2_raw;
        let odx: u32;

        // Edge handling (same as Oracle A)
        if tx1 < 0 {
            tx1 = 0;
            if tx2 <= 0 {
                tx2 = 1 << 24;
            } else if tx2 > tx_end {
                tx2 = tx_end;
            }
            odx = rational_inv(tx2);
        } else if tx2 > tx_end {
            if tx1 >= tx_end {
                tx1 = tx_end - (1 << 24);
            }
            odx = rational_inv(tx_end - tx1);
        } else {
            odx = odx0;
        }

        // Independent ox computation (Rust lines 1347-1355)
        let ox: u32 = {
            let w =
                ((0x100_0000i64 - (tx1 & 0xFF_FFFF)) as u64 * odx as u64 + 0xFF_FFFF) >> 24;
            if odx == 0x7FFF_FFFF {
                0x7FFF_FFFF
            } else {
                w as u32
            }
        };

        let col0 = (tx1 >> 24) as i32;
        let col_bound = ((tx2 - 1).max(tx1) >> 24) as i32 + 1;

        // Column accumulation with pCy reuse (Rust lines 1359-1400)
        let mut cyx: [u64; 4] = [0x7F_FFFF, 0x7F_FFFF, 0x7F_FFFF, 0x7F_FFFF];
        let mut remaining: u32 = 0x10000;
        let mut col = col0;
        let mut col_weight = ox;

        while remaining > 0 && col <= col_bound {
            let w = if col_weight >= remaining {
                remaining
            } else {
                col_weight
            };

            let cy = if col == prev_cy_col {
                cached_cy
            } else {
                let c = y_accumulate_single_row(col.min(img_w - 1).max(0), pattern);
                prev_cy_col = col;
                cached_cy = c;
                c
            };

            cyx[0] += cy[0] * w as u64;
            cyx[1] += cy[1] * w as u64;
            cyx[2] += cy[2] * w as u64;
            cyx[3] += cy[3] * w as u64;

            remaining -= w;
            col += 1;
            col_weight = odx;
        }

        result.push(write_pixel(cyx));
    }

    result
}

// ============================================================================
// Oracle C: Closed-form carry equivalent (O(1) per pixel)
// ============================================================================

/// Compute the independent ox for a pixel (same as Oracle B's per-pixel formula).
fn independent_ox(dest_x: i32, tdx: i64, tx_offset: i64, odx: u32, img_w: i32) -> u32 {
    let tx_end = (img_w as i64) << 24;
    let mut tx1 = dest_x as i64 * tdx - tx_offset;
    if tx1 < 0 { tx1 = 0; }
    else if tx1 >= tx_end { tx1 = tx_end - (1 << 24); }
    let w = ((0x100_0000i64 - (tx1 & 0xFF_FFFF)) as u64 * odx as u64 + 0xFF_FFFF) >> 24;
    if odx == 0x7FFF_FFFF { 0x7FFF_FFFF } else { w as u32 }
}

/// Check if tx1 for this pixel lands exactly on a source column boundary.
/// When it does, pCy != p0 and the C++ carry chain resets.
fn is_column_boundary(dest_x: i32, tdx: i64, tx_offset: i64, img_w: i32) -> bool {
    let tx_end = (img_w as i64) << 24;
    let tx1 = dest_x as i64 * tdx - tx_offset;
    if tx1 <= 0 || tx1 >= tx_end { return true; }
    (tx1 & 0xFF_FFFF) == 0
}

/// O(1) closed-form carry-equivalent ox for each pixel.
///
/// Derivation: The carry recurrence for interior pixels (constant odx) is:
///   carry_out = (odx - ((0x10000 - carry_in) % odx)) % odx
///
/// Define r[j] = (0x10000 - carry_out[j]) % odx. Then:
///   r[j] = (r[j-1] + S) % odx    where S = 0x10000 % odx
///
/// This is a constant-step modular recurrence with closed form:
///   r[j] = (r_seed + j * S) % odx
///
/// Seed from reset pixel R (pCy mismatch):
///   p = (0x10000 - ox_R) % odx
///
/// carry_in for pixel R+m (m >= 1):
///   carry_in = (odx - ((p + (m-1)*S) % odx)) % odx
///
/// The chain resets when frac(tx1) == 0 (pCy mismatch) or at pixel 0.
/// The outer `% odx` is critical: when remainder is 0, carry is 0 not odx.
fn closed_form_carry_ox(dest_x: i32, tdx: i64, tx_offset: i64, odx: u32, img_w: i32) -> u32 {
    if odx == 0 || odx == 0x7FFF_FFFF {
        return independent_ox(dest_x, tdx, tx_offset, odx, img_w);
    }

    // Find the most recent carry-chain reset point at or before dest_x.
    // Reset occurs at pixel 0 (pCy=NULL) or when frac(tx1)==0 (column boundary).
    let mut reset_at = 0i32;
    for k in 1..=dest_x {
        if is_column_boundary(k, tdx, tx_offset, img_w) {
            reset_at = k;
        }
    }

    if dest_x <= reset_at {
        return independent_ox(dest_x, tdx, tx_offset, odx, img_w);
    }

    // Seed from the reset pixel
    let reset_ox = independent_ox(reset_at, tdx, tx_offset, odx, img_w);
    let p_seed = (0x10000u64 - reset_ox as u64) % odx as u64;
    let step = 0x10000u64 % odx as u64;

    // carry_in for pixel at offset m from reset (m >= 1)
    let m = (dest_x - reset_at) as u64;
    let p_prev = (p_seed + (m - 1) * step) % odx as u64;
    let carry_in = (odx as u64 - p_prev) % odx as u64;

    carry_in as u32
}

/// Oracle C: full scanline using closed-form carry.
fn closed_form_carry(
    tdx: i64,
    tx_offset: i64,
    img_w: i32,
    odx0: u32,
    dest_x_start: i32,
    count: usize,
    pattern: Pattern,
) -> Vec<Pixel> {
    let tx_end = (img_w as i64) << 24;
    let mut result = Vec::with_capacity(count);

    let mut prev_cy_col: i32 = i32::MIN;
    let mut cached_cy: [u64; 4] = [0; 4];

    for pixel_idx in 0..count {
        let dest_x = dest_x_start + pixel_idx as i32;

        let mut tx1 = dest_x as i64 * tdx - tx_offset;
        let tx2_raw = tx1 + tdx;
        let mut tx2 = tx2_raw;
        let odx: u32;

        if tx1 < 0 {
            tx1 = 0;
            if tx2 <= 0 { tx2 = 1 << 24; }
            else if tx2 > tx_end { tx2 = tx_end; }
            odx = rational_inv(tx2);
        } else if tx2 > tx_end {
            if tx1 >= tx_end { tx1 = tx_end - (1 << 24); }
            odx = rational_inv(tx_end - tx1);
        } else {
            odx = odx0;
        }

        // Use closed-form carry to compute first-column weight
        let ox = closed_form_carry_ox(dest_x, tdx, tx_offset, odx, img_w);

        let col0 = (tx1 >> 24) as i32;
        let col_bound = ((tx2 - 1).max(tx1) >> 24) as i32 + 1;

        let mut cyx: [u64; 4] = [0x7F_FFFF, 0x7F_FFFF, 0x7F_FFFF, 0x7F_FFFF];
        let mut remaining: u32 = 0x10000;
        let mut col = col0;
        let mut col_weight = ox;

        while remaining > 0 && col <= col_bound {
            let w = if col_weight >= remaining { remaining } else { col_weight };

            let cy = if col == prev_cy_col {
                cached_cy
            } else {
                let c = y_accumulate_single_row(col.min(img_w - 1).max(0), pattern);
                prev_cy_col = col;
                cached_cy = c;
                c
            };

            cyx[0] += cy[0] * w as u64;
            cyx[1] += cy[1] * w as u64;
            cyx[2] += cy[2] * w as u64;
            cyx[3] += cy[3] * w as u64;

            remaining -= w;
            col += 1;
            col_weight = odx;
        }

        result.push(write_pixel(cyx));
    }

    result
}



// ============================================================================
// Oracle D: Sequential carry precomputation + independent rendering
// ============================================================================
//
// Approach: Precompute the carry chain (ox, pCy_col) sequentially — just
// integer arithmetic, no pixel data. Then render each pixel using the
// precomputed first-column weight. This enables parallel rendering: the
// precomputation is O(N) but cheap, and the rendering per pixel is independent.

/// Precompute carry state for all pixels. Returns per-pixel first-column weight
/// and whether pCy matched (for diagnostics).
fn precompute_carry_chain(
    tdx: i64,
    tx_offset: i64,
    img_w: i32,
    odx0: u32,
    dest_x_start: i32,
    count: usize,
) -> Vec<(u32, bool)> {
    let tx_end = (img_w as i64) << 24;
    let mut result = Vec::with_capacity(count);

    // Carry state: same as Oracle A's (carried_ox, pcy_col)
    let mut pcy_col: i32 = i32::MIN;
    let mut carried_ox: u32 = 0;

    for pixel_idx in 0..count {
        let dest_x = dest_x_start + pixel_idx as i32;
        let mut tx1 = dest_x as i64 * tdx - tx_offset;
        let tx2_raw = tx1 + tdx;
        let mut tx2 = tx2_raw;
        let odx: u32;

        // Edge handling (identical to Oracle A)
        if tx1 < 0 {
            tx1 = 0;
            if tx2 <= 0 { tx2 = 1 << 24; }
            else if tx2 > tx_end { tx2 = tx_end; }
            odx = rational_inv(tx2);
        } else if tx2 > tx_end {
            if tx1 >= tx_end { tx1 = tx_end - (1 << 24); }
            odx = rational_inv(tx_end - tx1);
        } else {
            odx = odx0;
        }

        let ox_computed: u32 =
            (((0x100_0000i64 - (tx1 & 0xFF_FFFF)) as u64 * odx as u64 + 0xFF_FFFF) >> 24) as u32;
        let ox_computed = if odx == 0x7FFF_FFFF { 0x7FFF_FFFF } else { ox_computed };

        let col0 = (tx1 >> 24) as i32;

        // pCy coupling: determine first-column weight
        let (first_weight, pcy_matched);
        if pcy_col != col0 {
            // Mismatch: discard carry, use independent weight
            first_weight = ox_computed;
            pcy_matched = false;
        } else {
            // Match: use carried weight
            first_weight = carried_ox;
            pcy_matched = true;
        }

        result.push((first_weight, pcy_matched));

        // Now simulate the while loop to determine carry_out and new pcy_col.
        // This is the key: we DON'T need pixel data, just the weight arithmetic.
        let mut ox: u32;
        let mut ox1: u32;
        let mut current_col: i32;

        if !pcy_matched {
            ox = 0;
            ox1 = ox_computed;
            current_col = col0;
        } else {
            ox = carried_ox;
            ox1 = odx;
            current_col = col0 + 1;
        }

        // Simulate while (ox < oxs) loop — just tracking column advances and ox
        let mut oxs: u32 = 0x10000;
        while ox < oxs {
            oxs -= ox;
            pcy_col = current_col;  // pCy = p0
            current_col += 1;       // p0 += imgDX
            ox = ox1;
            ox1 = odx;
        }
        // After while loop: carry_out = ox - oxs
        carried_ox = ox - oxs;
    }

    result
}

/// Oracle D: precomputed carry + independent rendering.
fn precomputed_carry(
    tdx: i64,
    tx_offset: i64,
    img_w: i32,
    odx0: u32,
    dest_x_start: i32,
    count: usize,
    pattern: Pattern,
) -> Vec<Pixel> {
    let tx_end = (img_w as i64) << 24;

    // Step 1: precompute carry chain (cheap, no pixel data)
    let carry_state = precompute_carry_chain(
        tdx, tx_offset, img_w, odx0, dest_x_start, count,
    );

    // Step 2: render each pixel using precomputed first-column weight
    let mut result = Vec::with_capacity(count);
    let mut prev_cy_col: i32 = i32::MIN;
    let mut cached_cy: [u64; 4] = [0; 4];

    for pixel_idx in 0..count {
        let dest_x = dest_x_start + pixel_idx as i32;
        let mut tx1 = dest_x as i64 * tdx - tx_offset;
        let tx2_raw = tx1 + tdx;
        let mut tx2 = tx2_raw;
        let odx: u32;

        if tx1 < 0 {
            tx1 = 0;
            if tx2 <= 0 { tx2 = 1 << 24; }
            else if tx2 > tx_end { tx2 = tx_end; }
            odx = rational_inv(tx2);
        } else if tx2 > tx_end {
            if tx1 >= tx_end { tx1 = tx_end - (1 << 24); }
            odx = rational_inv(tx_end - tx1);
        } else {
            odx = odx0;
        }

        let (first_weight, pcy_matched) = carry_state[pixel_idx];

        let col0 = (tx1 >> 24) as i32;

        // Render using the C++ algorithm structure, matching Oracle A exactly.
        // The key difference from Oracle B: first_weight may differ from ox_computed.
        let mut cyx: [u64; 4] = [0x7F_FFFF, 0x7F_FFFF, 0x7F_FFFF, 0x7F_FFFF];
        let mut oxs: u32 = 0x10000;

        let mut ox: u32;
        let mut ox1: u32;
        let mut current_col: i32;
        let mut cy: [u64; 4];

        if !pcy_matched {
            // Mismatch path: ox=0, ox1=first_weight, start at col0
            ox = 0;
            ox1 = first_weight;
            current_col = col0;
            // cy starts as whatever was cached (but first iter adds cy*0 = nothing)
            cy = if col0 == prev_cy_col { cached_cy } else { [0; 4] };
        } else {
            // Match path: ox=first_weight, ox1=odx, start at col0+1
            ox = first_weight;
            ox1 = odx;
            current_col = col0 + 1;
            // cy is the cached value from previous pixel's last column
            cy = cached_cy; // This must be the same cy the previous pixel left
        }

        // Simulate while (ox < oxs) loop with actual pixel data
        while ox < oxs {
            cyx[0] += cy[0] * ox as u64;
            cyx[1] += cy[1] * ox as u64;
            cyx[2] += cy[2] * ox as u64;
            cyx[3] += cy[3] * ox as u64;
            oxs -= ox;
            prev_cy_col = current_col;
            cy = y_accumulate_single_row(current_col.min(img_w - 1).max(0), pattern);
            cached_cy = cy;
            current_col += 1;
            ox = ox1;
            ox1 = odx;
        }

        cyx[0] += cy[0] * oxs as u64;
        cyx[1] += cy[1] * oxs as u64;
        cyx[2] += cy[2] * oxs as u64;
        cyx[3] += cy[3] * oxs as u64;

        result.push(write_pixel(cyx));
    }

    result
}

// ============================================================================
// Test configurations derived from real rendering
// ============================================================================

struct Config {
    name: &'static str,
    src_w: i32,
    dest_w: i32,
    tx_offset_frac: i64, // Added to the computed tx as a phase offset
    scanline_width: usize,
}

fn configs() -> Vec<Config> {
    let mut cfgs = Vec::new();

    // Real-ish (src_w, dest_w) pairs from golden test widget rendering
    let pairs: &[(i32, i32, &str)] = &[
        (286, 78, "border_corner_3.7x"),     // GroupBorder 9-slice corner
        (286, 143, "border_corner_2x"),       // 2x downscale
        (592, 200, "border_full_3x"),         // Full border image
        (128, 50, "font_glyph_2.56x"),        // Font atlas glyph
        (224, 121, "font_glyph_1.85x"),       // Font atlas at checkbox scale
        (64, 30, "reduced_img_2.1x"),         // Post-reduction image
        (512, 100, "texture_5.12x"),          // Large downscale
        (100, 99, "near_unity"),              // Near 1:1 (minimal downscale)
        (1000, 300, "large_3.3x"),            // Large source
        (7, 3, "tiny_2.3x"),                  // Very small source (edge-heavy)
    ];

    let offsets: &[(i64, &str)] = &[
        (0, "aligned"),
        (0x80_0000, "half_pixel"),
        (0xFFF_000, "near_boundary"),          // Close to column boundary
        (0x123_4567, "arbitrary"),
        (0xFF_FFFF, "max_frac"),               // Maximum fractional offset
    ];

    let widths: &[usize] = &[
        8, 32, 78, 128, 200,
        255, 256, 257,                         // Chunk boundary at 256
        300, 512,
    ];

    // Generate representative configs
    for (src_w, dest_w, pair_name) in pairs {
        for (offset, off_name) in offsets {
            for width in widths {
                // Only generate a subset — full Cartesian product is 500 configs
                // Use a hash-like selection to get diverse coverage
                let sel = (*src_w as usize + *width + *offset as usize) % 7;
                if sel < 2 {
                    let w = (*width).min(*dest_w as usize); // Can't render more pixels than dest_w
                    cfgs.push(Config {
                        name: Box::leak(format!("{pair_name}_{off_name}_w{w}").into_boxed_str()),
                        src_w: *src_w,
                        dest_w: *dest_w,
                        tx_offset_frac: *offset,
                        scanline_width: w,
                    });
                }
            }
        }
    }

    // Always include these critical configs regardless of selection
    cfgs.push(Config {
        name: "border_corner_3.7x_aligned_w78",
        src_w: 286,
        dest_w: 78,
        tx_offset_frac: 0,
        scanline_width: 78,
    });
    cfgs.push(Config {
        name: "checkerboard_stress_256boundary",
        src_w: 512,
        dest_w: 300,
        tx_offset_frac: 0xFFF_000,
        scanline_width: 300,
    });
    cfgs.push(Config {
        name: "tiny_all_edge",
        src_w: 7,
        dest_w: 3,
        tx_offset_frac: 0,
        scanline_width: 3,
    });
    cfgs.push(Config {
        name: "near_unity_full",
        src_w: 100,
        dest_w: 99,
        tx_offset_frac: 0x80_0000,
        scanline_width: 99,
    });

    cfgs
}

// ============================================================================
// Comparison and reporting
// ============================================================================

struct CompResult {
    config_name: String,
    pattern: Pattern,
    tdx: i64,
    odx: u32,
    a_vs_b_diffs: usize,
    a_vs_b_max_diff: u8,
    a_vs_b_first_diff_idx: Option<usize>,
    a_vs_c_diffs: usize,
    a_vs_c_max_diff: u8,
    a_vs_c_first_diff_idx: Option<usize>,
    a_vs_d_diffs: usize,
    a_vs_d_max_diff: u8,
    a_vs_d_first_diff_idx: Option<usize>,
    total_pixels: usize,
}

fn max_channel_diff(a: &Pixel, b: &Pixel) -> u8 {
    let dr = (a.r as i16 - b.r as i16).unsigned_abs() as u8;
    let dg = (a.g as i16 - b.g as i16).unsigned_abs() as u8;
    let db = (a.b as i16 - b.b as i16).unsigned_abs() as u8;
    let da = (a.a as i16 - b.a as i16).unsigned_abs() as u8;
    dr.max(dg).max(db).max(da)
}

fn compare_oracles(
    config: &Config,
    pattern: Pattern,
) -> CompResult {
    let (tdx, odx) = derive_transform(config.src_w, config.dest_w);

    let tx_offset = config.tx_offset_frac;

    let a = sequential_carry(tdx, tx_offset, config.src_w, odx, 0, config.scanline_width, pattern);
    let b = independent_rust(tdx, tx_offset, config.src_w, odx, 0, config.scanline_width, pattern);
    let c = closed_form_carry(tdx, tx_offset, config.src_w, odx, 0, config.scanline_width, pattern);
    let d = precomputed_carry(tdx, tx_offset, config.src_w, odx, 0, config.scanline_width, pattern);

    let mut ab_diffs = 0usize;
    let mut ab_max = 0u8;
    let mut ab_first = None;
    let mut ac_diffs = 0usize;
    let mut ac_max = 0u8;
    let mut ac_first = None;
    let mut ad_diffs = 0usize;
    let mut ad_max = 0u8;
    let mut ad_first = None;

    let n = config.scanline_width.min(a.len()).min(b.len()).min(c.len()).min(d.len());
    for i in 0..n {
        let dab = max_channel_diff(&a[i], &b[i]);
        if dab > 0 {
            ab_diffs += 1;
            ab_max = ab_max.max(dab);
            if ab_first.is_none() { ab_first = Some(i); }
        }
        let dac = max_channel_diff(&a[i], &c[i]);
        if dac > 0 {
            ac_diffs += 1;
            ac_max = ac_max.max(dac);
            if ac_first.is_none() { ac_first = Some(i); }
        }
        let dad = max_channel_diff(&a[i], &d[i]);
        if dad > 0 {
            ad_diffs += 1;
            ad_max = ad_max.max(dad);
            if ad_first.is_none() { ad_first = Some(i); }
        }
    }

    CompResult {
        config_name: config.name.to_string(),
        pattern,
        tdx,
        odx,
        a_vs_b_diffs: ab_diffs,
        a_vs_b_max_diff: ab_max,
        a_vs_b_first_diff_idx: ab_first,
        a_vs_c_diffs: ac_diffs,
        a_vs_c_max_diff: ac_max,
        a_vs_c_first_diff_idx: ac_first,
        a_vs_d_diffs: ad_diffs,
        a_vs_d_max_diff: ad_max,
        a_vs_d_first_diff_idx: ad_first,
        total_pixels: config.scanline_width,
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    let cfgs = configs();
    let patterns = [Pattern::Gradient, Pattern::Checkerboard, Pattern::WorstCase];

    println!("=== Area Sampling Carry-Over Experiment ===");
    println!("Configs: {}", cfgs.len());
    println!("Patterns: {:?}", patterns.len());
    println!();

    let mut total_tests = 0;
    let mut ab_total_with_diffs = 0;
    let mut ab_overall_max: u8 = 0;
    let mut ab_total_diff_px = 0;
    let mut ac_total_with_diffs = 0;
    let mut ac_overall_max: u8 = 0;
    let mut ac_total_diff_px = 0;
    let mut ad_total_with_diffs = 0;
    let mut ad_overall_max: u8 = 0;
    let mut ad_total_diff_px = 0;
    let mut total_pixels = 0;

    let mut diff_results: Vec<CompResult> = Vec::new();

    for cfg in &cfgs {
        for pattern in &patterns {
            let result = compare_oracles(cfg, *pattern);
            total_tests += 1;
            total_pixels += result.total_pixels;

            if result.a_vs_b_diffs > 0 {
                ab_total_with_diffs += 1;
                ab_total_diff_px += result.a_vs_b_diffs;
                ab_overall_max = ab_overall_max.max(result.a_vs_b_max_diff);
            }
            if result.a_vs_c_diffs > 0 {
                ac_total_with_diffs += 1;
                ac_total_diff_px += result.a_vs_c_diffs;
                ac_overall_max = ac_overall_max.max(result.a_vs_c_max_diff);
            }
            if result.a_vs_d_diffs > 0 {
                ad_total_with_diffs += 1;
                ad_total_diff_px += result.a_vs_d_diffs;
                ad_overall_max = ad_overall_max.max(result.a_vs_d_max_diff);
            }
            if result.a_vs_b_diffs > 0 || result.a_vs_c_diffs > 0 || result.a_vs_d_diffs > 0 {
                diff_results.push(result);
            }
        }
    }

    println!("=== A vs B (Sequential Carry vs Independent Rust) ===");
    println!("Total configs tested: {total_tests}");
    println!("Configs with diffs:   {ab_total_with_diffs}");
    println!("Total pixels:         {total_pixels}");
    println!("Differing pixels:     {ab_total_diff_px}");
    println!("Overall max diff:     {ab_overall_max}");
    println!();

    println!("=== A vs C (Sequential Carry vs Closed-Form O(1)) ===");
    println!("Configs with diffs:   {ac_total_with_diffs}");
    println!("Differing pixels:     {ac_total_diff_px}");
    println!("Overall max diff:     {ac_overall_max}");
    println!();

    println!("=== A vs D (Sequential Carry vs Precomputed Carry) ===");
    println!("Configs with diffs:   {ad_total_with_diffs}");
    println!("Differing pixels:     {ad_total_diff_px}");
    println!("Overall max diff:     {ad_overall_max}");
    if ad_total_with_diffs == 0 {
        println!("*** ORACLE D MATCHES A PERFECTLY — precomputed carry approach is viable ***");
    }
    println!();

    if diff_results.is_empty() {
        println!("NO DIFFERENCES FOUND between sequential carry and independent computation.");
        println!("This would mean the carry has no effect — investigate test sensitivity.");
    } else {
        println!("--- Configs with differences (A != B) ---");
        for r in &diff_results {
            println!(
                "  {:<45} pattern={:<12} tdx=0x{:X} odx=0x{:X} diffs={}/{} max_diff={} first_at={}",
                r.config_name,
                format!("{:?}", r.pattern),
                r.tdx,
                r.odx,
                r.a_vs_b_diffs,
                r.total_pixels,
                r.a_vs_b_max_diff,
                r.a_vs_b_first_diff_idx.map_or("none".to_string(), |i| i.to_string()),
            );
        }
        println!();

        // Show first 5 detailed pixel diffs
        println!("--- First 5 detailed pixel diffs ---");
        let mut shown = 0;
        for cfg in &cfgs {
            if shown >= 5 { break; }
            for pattern in &patterns {
                if shown >= 5 { break; }
                let (tdx, odx) = derive_transform(cfg.src_w, cfg.dest_w);
                let tx_offset = cfg.tx_offset_frac;
                let a = sequential_carry(tdx, tx_offset, cfg.src_w, odx, 0, cfg.scanline_width, *pattern);
                let b = independent_rust(tdx, tx_offset, cfg.src_w, odx, 0, cfg.scanline_width, *pattern);

                for i in 0..a.len().min(b.len()) {
                    if a[i] != b[i] && shown < 5 {
                        println!(
                            "  {} pattern={:?} pixel[{}]: A={:?} B={:?} diff={}",
                            cfg.name, pattern, i, a[i], b[i], max_channel_diff(&a[i], &b[i])
                        );
                        shown += 1;
                    }
                }
            }
        }
    }

    // Sensitivity check: verify A and B actually produce non-trivial output
    println!();
    println!("--- Sensitivity check ---");
    let (tdx, odx) = derive_transform(286, 78);
    let a = sequential_carry(tdx, 0, 286, odx, 0, 10, Pattern::Checkerboard);
    let b = independent_rust(tdx, 0, 286, odx, 0, 10, Pattern::Checkerboard);
    println!("border_corner_3.7x checkerboard, first 10 pixels:");
    for i in 0..10 {
        let match_str = if a[i] == b[i] { "==" } else { "!=" };
        println!("  [{i}] A={:?} {match_str} B={:?}", a[i], b[i]);
    }
}
