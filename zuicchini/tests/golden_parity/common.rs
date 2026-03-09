use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub struct CompareError {
    pub message: String,
}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden")
}

/// Returns true if golden directory exists (generator has been run).
pub fn golden_available() -> bool {
    golden_dir().is_dir()
}

// ────────────────────── Painter golden files ──────────────────────

/// Load a painter golden file. Returns (width, height, rgba_bytes).
pub fn load_painter_golden(name: &str) -> (u32, u32, Vec<u8>) {
    let path = golden_dir()
        .join("painter")
        .join(format!("{name}.painter.golden"));
    let data =
        std::fs::read(&path).unwrap_or_else(|e| panic!("Cannot read {}: {e}", path.display()));
    assert!(data.len() >= 8, "Golden file too short: {}", path.display());
    let width = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let height = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let expected_len = 8 + (width as usize * height as usize * 4);
    assert_eq!(
        data.len(),
        expected_len,
        "Golden file size mismatch for {name}: got {} expected {expected_len}",
        data.len()
    );
    (width, height, data[8..].to_vec())
}

/// Compare two RGBA images pixel-by-pixel on RGB channels only.
///
/// The alpha channel is **excluded** because C++ emPainter uses channel 3 to
/// track "remaining canvas visibility" (not standard compositing alpha), while
/// the Rust painter stores standard alpha.  The visual output (RGB) is what
/// matters for parity.
///
/// `channel_tolerance`: max per-channel absolute diff allowed per pixel.
/// `max_failure_pct`: max percentage of pixels that may exceed tolerance.
pub fn compare_images(
    actual: &[u8],
    expected: &[u8],
    width: u32,
    height: u32,
    channel_tolerance: u8,
    max_failure_pct: f64,
) -> Result<(), CompareError> {
    let total = (width * height) as usize;
    assert_eq!(actual.len(), total * 4);
    assert_eq!(expected.len(), total * 4);

    let mut fail_count = 0usize;
    let mut max_diff: u8 = 0;
    let mut first_failures: Vec<(usize, usize, usize)> = Vec::new();

    for i in 0..total {
        let off = i * 4;
        let mut pixel_fail = false;
        // Compare RGB only (channels 0-2), skip alpha (channel 3)
        for ch in 0..3 {
            let diff = actual[off + ch].abs_diff(expected[off + ch]);
            if diff > channel_tolerance {
                pixel_fail = true;
                max_diff = max_diff.max(diff);
            }
        }
        if pixel_fail {
            fail_count += 1;
            if first_failures.len() < 10 {
                first_failures.push((i % width as usize, i / width as usize, off));
            }
        }
    }

    let fail_pct = fail_count as f64 / total as f64 * 100.0;
    if fail_pct > max_failure_pct {
        let mut msg = format!(
            "Image mismatch: {fail_count}/{total} pixels ({fail_pct:.2}%) exceed tolerance \
             {channel_tolerance}, max_diff={max_diff}\n"
        );
        for &(x, y, off) in &first_failures {
            msg += &format!(
                "  ({x},{y}): actual=rgb({},{},{}) expected=rgb({},{},{})\n",
                actual[off],
                actual[off + 1],
                actual[off + 2],
                expected[off],
                expected[off + 1],
                expected[off + 2],
            );
        }
        Err(CompareError { message: msg })
    } else {
        Ok(())
    }
}

// ────────────────────── Layout golden files ──────────────────────

#[derive(Debug, Clone)]
pub struct GoldenRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub fn load_layout_golden(name: &str) -> Vec<GoldenRect> {
    let path = golden_dir()
        .join("layout")
        .join(format!("{name}.layout.golden"));
    let data =
        std::fs::read(&path).unwrap_or_else(|e| panic!("Cannot read {}: {e}", path.display()));
    assert!(data.len() >= 4);
    let child_count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let expected_len = 4 + child_count * 32;
    assert_eq!(
        data.len(),
        expected_len,
        "Layout golden size mismatch for {name}"
    );

    let mut rects = Vec::with_capacity(child_count);
    for i in 0..child_count {
        let off = 4 + i * 32;
        let x = f64::from_le_bytes(data[off..off + 8].try_into().unwrap());
        let y = f64::from_le_bytes(data[off + 8..off + 16].try_into().unwrap());
        let w = f64::from_le_bytes(data[off + 16..off + 24].try_into().unwrap());
        let h = f64::from_le_bytes(data[off + 24..off + 32].try_into().unwrap());
        rects.push(GoldenRect { x, y, w, h });
    }
    rects
}

/// Scale golden rects from emCore normalized coords to absolute coords.
/// In emCore, parent width = 1.0 and all four (x,y,w,h) are in that unit space.
pub fn scale_golden_rects(rects: &mut [GoldenRect], parent_width: f64) {
    for r in rects.iter_mut() {
        r.x *= parent_width;
        r.y *= parent_width;
        r.w *= parent_width;
        r.h *= parent_width;
    }
}

pub fn compare_rects(
    actual: &[(f64, f64, f64, f64)],
    expected: &[GoldenRect],
    eps: f64,
) -> Result<(), CompareError> {
    if actual.len() != expected.len() {
        return Err(CompareError {
            message: format!(
                "Rect count mismatch: actual={} expected={}",
                actual.len(),
                expected.len()
            ),
        });
    }
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        let dx = (a.0 - e.x).abs();
        let dy = (a.1 - e.y).abs();
        let dw = (a.2 - e.w).abs();
        let dh = (a.3 - e.h).abs();
        if dx > eps || dy > eps || dw > eps || dh > eps {
            return Err(CompareError {
                message: format!(
                    "Rect {i} mismatch:\n  actual =({:.6},{:.6},{:.6},{:.6})\n  \
                     expected=({:.6},{:.6},{:.6},{:.6})\n  \
                     diffs   =({dx:.2e},{dy:.2e},{dw:.2e},{dh:.2e})",
                    a.0, a.1, a.2, a.3, e.x, e.y, e.w, e.h
                ),
            });
        }
    }
    Ok(())
}
