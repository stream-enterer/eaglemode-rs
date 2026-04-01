// Port of C++ emAutoplayControlPanel (emAutoplay.h:333-398).
//
// DIVERGED: C++ uses emPackGroup with emCheckButton, emButton, emScalarField.
// Rust uses simplified stub buttons until the full toolkit is ported.

use emcore::emColor::emColor;
use emcore::emPanel::{PanelBehavior, PanelState};
use emcore::emPainter::emPainter;
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;

use crate::emMainControlPanel::ControlButton;

// ── Duration table and conversion ───────────────────────────────────────────

/// Port of C++ DurationTable in emAutoplay.cpp.
const DURATION_TABLE_MS: &[i32] = &[
    500, 1000, 2000, 3000, 5000, 10000, 15000, 30000, 60000, 120000,
];

/// Convert a scalar-field value (0..900) to milliseconds by interpolating in
/// `DURATION_TABLE_MS`.
///
/// Port of C++ `emAutoplayControlPanel::DurationValueToMS`.
pub fn DurationValueToMS(value: i64) -> i32 {
    let n = DURATION_TABLE_MS.len();
    let step = 900.0 / (n as f64 - 1.0);
    let pos = value as f64 / step;
    let idx = pos.floor() as usize;
    if idx >= n - 1 {
        return DURATION_TABLE_MS[n - 1];
    }
    let frac = pos - idx as f64;
    let a = DURATION_TABLE_MS[idx] as f64;
    let b = DURATION_TABLE_MS[idx + 1] as f64;
    (a + frac * (b - a)).round() as i32
}

/// Convert milliseconds back to a scalar-field value (0..900) by binary search.
///
/// Port of C++ `emAutoplayControlPanel::DurationMSToValue`.
pub fn DurationMSToValue(ms: i32) -> i64 {
    // Binary search over the value domain [0, 900].
    let mut lo: i64 = 0;
    let mut hi: i64 = 900;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if DurationValueToMS(mid) < ms {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

// ── emAutoplayControlPanel ──────────────────────────────────────────────────

/// Button labels for autoplay controls.
const AUTOPLAY_BUTTON_LABELS: &[&str] = &[
    "Autoplay",
    "Previous",
    "Next",
    "Continue Last",
];

/// Control panel for autoplay settings.
///
/// Port of C++ `emAutoplayControlPanel` (extends `emPackGroup`).
/// DIVERGED: C++ uses emPackGroup layout with emCheckButton, emButton,
/// emScalarField widget trees. Rust uses simplified stub buttons until the full
/// toolkit widget hierarchy is ported.
#[derive(Default)]
pub struct emAutoplayControlPanel {
    children_created: bool,
    btn_autoplay: Option<PanelId>,
    btn_prev: Option<PanelId>,
    btn_next: Option<PanelId>,
    btn_continue_last: Option<PanelId>,
}

impl emAutoplayControlPanel {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_children(&mut self, ctx: &mut PanelCtx) {
        let labels = AUTOPLAY_BUTTON_LABELS;
        let ids: Vec<PanelId> = labels
            .iter()
            .enumerate()
            .map(|(i, &label)| {
                let name = format!("btn_{i}");
                let btn = Box::new(ControlButton {
                    label: label.to_string(),
                });
                ctx.create_child_with(&name, btn)
            })
            .collect();

        self.btn_autoplay = Some(ids[0]);
        self.btn_prev = Some(ids[1]);
        self.btn_next = Some(ids[2]);
        self.btn_continue_last = Some(ids[3]);

        self.children_created = true;
    }
}

impl PanelBehavior for emAutoplayControlPanel {
    fn get_title(&self) -> Option<String> {
        Some("Autoplay".to_string())
    }

    fn IsOpaque(&self) -> bool {
        true
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        let bg = emColor::from_packed(0x515E84FF);
        let canvas = emColor::TRANSPARENT;
        painter.PaintRect(0.0, 0.0, w, h, bg, canvas);
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        if !self.children_created {
            self.create_children(ctx);
        }

        let n = AUTOPLAY_BUTTON_LABELS.len() as f64;
        let gap = 0.005_f64;
        let pad_x = 0.01_f64;
        let child_w = (1.0 - 2.0 * pad_x).max(0.0);
        let total_gaps = (n + 1.0) * gap;
        let usable_h = (1.0 - total_gaps).max(0.0);
        let ch = usable_h / n;

        let canvas = emColor::from_packed(0x515E84FF);

        let all_ids = [
            self.btn_autoplay,
            self.btn_prev,
            self.btn_next,
            self.btn_continue_last,
        ];

        let mut y = gap;
        for id_opt in &all_ids {
            if let Some(id) = *id_opt {
                ctx.layout_child_canvas(id, pad_x, y, child_w, ch, canvas);
                y += ch + gap;
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_panel_new() {
        let panel = emAutoplayControlPanel::new();
        assert_eq!(panel.get_title(), Some("Autoplay".to_string()));
    }

    #[test]
    fn test_duration_value_to_ms() {
        // Value 0 → first table entry
        assert_eq!(DurationValueToMS(0), 500);
        // Value 900 → last table entry
        assert_eq!(DurationValueToMS(900), 120000);
        // Value 100 → second entry (index 1)
        assert_eq!(DurationValueToMS(100), 1000);
        // Value 450 → midpoint of table
        assert_eq!(DurationValueToMS(450), 7500);
    }

    #[test]
    fn test_duration_ms_to_value() {
        // Inverse of known values
        assert_eq!(DurationMSToValue(500), 0);
        assert_eq!(DurationMSToValue(120000), 900);
        assert_eq!(DurationMSToValue(1000), 100);
    }

    #[test]
    fn test_duration_roundtrip() {
        for v in (0..=900).step_by(100) {
            let ms = DurationValueToMS(v);
            let v2 = DurationMSToValue(ms);
            assert_eq!(
                DurationValueToMS(v2), ms,
                "round-trip failed for value {v}: ms={ms}, v2={v2}"
            );
        }
    }
}
