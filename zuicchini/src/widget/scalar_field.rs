use std::rc::Rc;

use crate::foundation::{Color, Rect};
use crate::input::{Cursor, InputEvent, InputKey, InputVariant};
use crate::render::Painter;

use super::border::{Border, InnerBorderType, OuterBorderType};
use super::look::Look;

/// Default text formatter: decimal representation of the value.
/// The `mark_interval` parameter is ignored by the default.
fn default_text_of_value(value: i64, _mark_interval: u64) -> String {
    value.to_string()
}

/// Numeric input with scale bar.
///
/// Values are stored as `f64` but keyboard stepping logic uses integer
/// arithmetic internally to match the C++ emScalarField behaviour.
pub struct ScalarField {
    border: Border,
    look: Rc<Look>,
    value: f64,
    min: f64,
    max: f64,
    precision: usize,
    editable: bool,
    dragging: bool,
    drag_start_x: f64,
    drag_start_value: f64,
    /// Cached width from the last paint call.
    last_w: f64,

    // --- Scale mark configuration ---
    scale_mark_intervals: Vec<u64>,
    marks_never_hidden: bool,
    text_of_value_fn: Box<dyn Fn(i64, u64) -> String>,
    text_box_tallness: f64,
    kb_interval: u64,

    pub on_value: Option<Box<dyn FnMut(f64)>>,
}

impl ScalarField {
    pub fn new(min: f64, max: f64, look: Rc<Look>) -> Self {
        let clamped_max = if max < min { min } else { max };
        let value = min;
        Self {
            border: Border::new(OuterBorderType::Instrument)
                .with_inner(InnerBorderType::InputField),
            look,
            value,
            min,
            max: clamped_max,
            precision: 2,
            editable: true,
            dragging: false,
            drag_start_x: 0.0,
            drag_start_value: 0.0,
            last_w: 0.0,
            scale_mark_intervals: vec![1],
            marks_never_hidden: false,
            text_of_value_fn: Box::new(default_text_of_value),
            text_box_tallness: 0.5,
            kb_interval: 0,
            on_value: None,
        }
    }

    pub fn set_caption(&mut self, caption: &str) {
        self.border.caption = caption.to_string();
    }

    // --- Editable ---

    pub fn is_editable(&self) -> bool {
        self.editable
    }

    pub fn set_editable(&mut self, editable: bool) {
        if self.editable == editable {
            return;
        }
        self.editable = editable;
        // Switch inner border type to match editability, but only if it was
        // the "other" standard type (matching C++ SetEditable behaviour).
        if editable && self.border.inner == InnerBorderType::OutputField {
            self.border.inner = InnerBorderType::InputField;
        } else if !editable && self.border.inner == InnerBorderType::InputField {
            self.border.inner = InnerBorderType::OutputField;
        }
    }

    // --- Value ---

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn set_value(&mut self, val: f64) {
        let clamped = val.clamp(self.min, self.max);
        if (clamped - self.value).abs() > f64::EPSILON {
            self.value = clamped;
            self.fire_change();
        }
    }

    pub fn set_precision(&mut self, precision: usize) {
        self.precision = precision;
    }

    // --- Min/Max ---

    pub fn min_value(&self) -> f64 {
        self.min
    }

    pub fn max_value(&self) -> f64 {
        self.max
    }

    pub fn set_min_value(&mut self, min: f64) {
        if (self.min - min).abs() < f64::EPSILON {
            return;
        }
        self.min = min;
        if self.max < self.min {
            self.max = self.min;
        }
        if self.value < self.min {
            self.set_value(self.min);
        }
    }

    pub fn set_max_value(&mut self, max: f64) {
        if (self.max - max).abs() < f64::EPSILON {
            return;
        }
        self.max = max;
        if self.min > self.max {
            self.min = self.max;
        }
        if self.value > self.max {
            self.set_value(self.max);
        }
    }

    pub fn set_min_max_values(&mut self, min: f64, max: f64) {
        self.set_min_value(min);
        self.set_max_value(max);
    }

    // --- Scale mark configuration ---

    /// Returns the current scale mark intervals (descending order, each > 0).
    pub fn scale_mark_intervals(&self) -> &[u64] {
        &self.scale_mark_intervals
    }

    /// Sets scale mark intervals. Each element must be > 0 and the sequence
    /// must be in strictly descending order. Panics on invalid input (matching
    /// the C++ `emFatalError` behaviour).
    pub fn set_scale_mark_intervals(&mut self, intervals: &[u64]) {
        for (i, &iv) in intervals.iter().enumerate() {
            assert!(iv > 0, "scale mark interval must be > 0 (index {i})");
            if i > 0 {
                assert!(
                    iv < intervals[i - 1],
                    "scale mark intervals must be strictly descending \
                     (index {i}: {} >= {})",
                    iv,
                    intervals[i - 1]
                );
            }
        }
        if self.scale_mark_intervals == intervals {
            return;
        }
        self.scale_mark_intervals = intervals.to_vec();
    }

    pub fn is_never_hiding_marks(&self) -> bool {
        self.marks_never_hidden
    }

    pub fn set_never_hide_marks(&mut self, never_hide: bool) {
        self.marks_never_hidden = never_hide;
    }

    // --- Text formatting ---

    /// Sets a custom value-to-text formatter. The function receives the value
    /// as `i64` and the current mark interval as `u64`, returning the display
    /// string.
    pub fn set_text_of_value_fn(&mut self, f: Box<dyn Fn(i64, u64) -> String>) {
        self.text_of_value_fn = f;
    }

    pub fn text_box_tallness(&self) -> f64 {
        self.text_box_tallness
    }

    pub fn set_text_box_tallness(&mut self, tallness: f64) {
        self.text_box_tallness = tallness;
    }

    // --- Keyboard interval ---

    pub fn keyboard_interval(&self) -> u64 {
        self.kb_interval
    }

    pub fn set_keyboard_interval(&mut self, interval: u64) {
        self.kb_interval = interval;
    }

    // --- Paint ---

    pub fn paint(&mut self, painter: &mut Painter, w: f64, h: f64) {
        self.last_w = w;
        self.border
            .paint_border(painter, w, h, &self.look, false, true);

        let Rect {
            x: cx,
            y: cy,
            w: cw,
            h: ch,
        } = self.border.content_rect(w, h, &self.look);
        let range = self.max - self.min;

        if range > 0.0 {
            // Fill bar
            let fill_frac = (self.value - self.min) / range;
            let fill_w = cw * fill_frac;
            painter.paint_rect(cx, cy, fill_w, ch, self.look.input_hl_color);

            // Scale marks — use configured intervals, with hiding logic
            let ivals = &self.scale_mark_intervals;
            let mut start = 0;
            if !self.marks_never_hidden && ivals.len() > 1 {
                // Skip intervals that exceed the value range
                let v_range = range as u64;
                while start < ivals.len() - 1 && ivals[start] > v_range {
                    start += 1;
                }
            }

            for (level, &interval_u64) in ivals.iter().enumerate().skip(start) {
                let interval = interval_u64 as f64;
                if interval <= 0.0 {
                    continue;
                }
                let first_mark = (self.min / interval).ceil() as i64;
                let last_mark = (self.max / interval).floor() as i64;
                for k in first_mark..=last_mark {
                    let mark_val = k as f64 * interval;
                    if (mark_val - self.min).abs() < f64::EPSILON
                        || (mark_val - self.max).abs() < f64::EPSILON
                    {
                        continue;
                    }
                    let mx = cx + cw * (mark_val - self.min) / range;
                    // Marks at higher levels (smaller intervals) are shorter
                    let height_frac = 1.0 - (level - start) as f64 * 0.2;
                    let mark_h = ch * height_frac.max(0.3);
                    let mark_y = cy + ch - mark_h;
                    painter.paint_rect(mx, mark_y, 1.0, mark_h, self.look.disabled_fg());
                }
            }
        }

        // Value text — use the text_of_value_fn
        let mark_iv = if self.scale_mark_intervals.is_empty() {
            1u64
        } else {
            *self
                .scale_mark_intervals
                .last()
                .expect("non-empty intervals")
        };
        let text = (self.text_of_value_fn)(self.value as i64, mark_iv);
        let display_text = if self.precision > 0 {
            format!("{:.prec$}", self.value, prec = self.precision)
        } else {
            text
        };
        let text_h = 13.0_f64.min(ch);
        let tw = Painter::measure_text_width(&display_text, text_h);
        let tx = cx + (cw - tw) / 2.0;
        let ty = cy + (ch - text_h) / 2.0;

        let fg = if self.editable {
            self.look.input_fg_color
        } else {
            self.look.output_fg_color
        };

        painter.paint_text(tx, ty, &display_text, text_h, 1.0, fg, Color::TRANSPARENT);
    }

    // --- Input ---

    pub fn input(&mut self, event: &InputEvent) -> bool {
        if !self.editable {
            return false;
        }

        let Rect { w: cw, .. } = self.border.content_rect(self.last_w, 0.0, &self.look);
        let range = self.max - self.min;

        match event.key {
            InputKey::MouseLeft => match event.variant {
                InputVariant::Press => {
                    self.dragging = true;
                    self.drag_start_x = event.mouse_x;
                    self.drag_start_value = self.value;
                    true
                }
                InputVariant::Release => {
                    self.dragging = false;
                    true
                }
                InputVariant::Repeat | InputVariant::Move => {
                    if self.dragging && cw > 0.0 {
                        let dx = event.mouse_x - self.drag_start_x;
                        let dv = dx / cw * range;
                        let new_val = (self.drag_start_value + dv).clamp(self.min, self.max);
                        if (new_val - self.value).abs() > f64::EPSILON {
                            self.value = new_val;
                            self.fire_change();
                        }
                    }
                    true
                }
            },
            InputKey::ArrowRight | InputKey::Key('+') if event.variant == InputVariant::Press => {
                self.step_by_keyboard(1);
                true
            }
            InputKey::ArrowLeft | InputKey::Key('-') if event.variant == InputVariant::Press => {
                self.step_by_keyboard(-1);
                true
            }
            _ => false,
        }
    }

    pub fn get_cursor(&self) -> Cursor {
        if self.editable {
            Cursor::ResizeEW
        } else {
            Cursor::Normal
        }
    }

    pub fn preferred_size(&self) -> (f64, f64) {
        let cw = 100.0;
        let ch = 13.0 + 4.0;
        self.border.preferred_size_for_content(cw, ch)
    }

    // --- Keyboard stepping (C++ StepByKeyboard parity) ---

    /// Steps the value by a keyboard increment in the given direction.
    ///
    /// Matches the C++ `StepByKeyboard` logic: if `kb_interval > 0`, uses that
    /// as step. Otherwise computes `range/129` (min 1) and finds the best
    /// matching scale mark interval. Snaps to grid with direction-dependent
    /// rounding using integer division.
    fn step_by_keyboard(&mut self, dir: i32) {
        let range_f = self.max - self.min;
        let range = range_f as i64;

        let dv: i64 = if self.kb_interval > 0 {
            self.kb_interval as i64
        } else {
            // Auto mode: range/129, at least 1
            let mindv = (range / 129).max(1);
            let mut dv = mindv;
            for (i, &iv) in self.scale_mark_intervals.iter().enumerate() {
                let iv = iv as i64;
                if iv >= mindv || i == 0 {
                    dv = iv;
                }
            }
            dv
        };

        if dv <= 0 {
            return;
        }

        let cur = self.value as i64;
        let v = if dir < 0 {
            let v = cur - dv;
            // Snap to grid: direction-dependent rounding
            if v < 0 {
                -((-v) / dv) * dv
            } else {
                (v + dv - 1) / dv * dv
            }
        } else {
            let v = cur + dv;
            if v < 0 {
                -((-v + dv - 1) / dv) * dv
            } else {
                (v / dv) * dv
            }
        };

        self.set_value(v as f64);
    }

    fn fire_change(&mut self) {
        if let Some(cb) = &mut self.on_value {
            cb(self.value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn value_clamping() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);

        sf.set_value(50.0);
        assert!((sf.value() - 50.0).abs() < 0.001);

        sf.set_value(-10.0);
        assert!((sf.value() - 0.0).abs() < 0.001);

        sf.set_value(200.0);
        assert!((sf.value() - 100.0).abs() < 0.001);
    }

    #[test]
    fn arrow_key_stepping() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        sf.set_value(50.0);

        // Cache dimensions (paint would do this in real usage)
        sf.last_w = 200.0;

        sf.input(&InputEvent::press(InputKey::ArrowRight));
        assert!(sf.value() > 50.0);

        sf.input(&InputEvent::press(InputKey::ArrowLeft));
        // Should be roughly back to 50
        assert!((sf.value() - 50.0).abs() < 2.0);
    }

    #[test]
    fn callback_on_change() {
        let look = Look::new();
        let values = Rc::new(RefCell::new(Vec::new()));
        let val_clone = values.clone();

        let mut sf = ScalarField::new(0.0, 10.0, look);
        sf.set_value(5.0);
        sf.last_w = 200.0;
        sf.on_value = Some(Box::new(move |v| {
            val_clone.borrow_mut().push(v);
        }));

        sf.input(&InputEvent::press(InputKey::ArrowRight));
        assert_eq!(values.borrow().len(), 1);
        assert!(values.borrow()[0] > 5.0);
    }

    #[test]
    fn editable_toggle() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);

        assert!(sf.is_editable());
        assert_eq!(sf.border.inner, InnerBorderType::InputField);

        sf.set_editable(false);
        assert!(!sf.is_editable());
        assert_eq!(sf.border.inner, InnerBorderType::OutputField);

        // Input should be disabled when not editable
        sf.set_value(50.0);
        sf.last_w = 200.0;
        let handled = sf.input(&InputEvent::press(InputKey::ArrowRight));
        assert!(!handled);
        assert!((sf.value() - 50.0).abs() < 0.001);

        sf.set_editable(true);
        assert!(sf.is_editable());
        assert_eq!(sf.border.inner, InnerBorderType::InputField);
    }

    #[test]
    fn min_max_getters_setters() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);

        assert!((sf.min_value() - 0.0).abs() < f64::EPSILON);
        assert!((sf.max_value() - 100.0).abs() < f64::EPSILON);

        // Setting min above max clamps max up
        sf.set_min_value(200.0);
        assert!((sf.min_value() - 200.0).abs() < f64::EPSILON);
        assert!((sf.max_value() - 200.0).abs() < f64::EPSILON);
        assert!((sf.value() - 200.0).abs() < f64::EPSILON);

        // Setting max below min clamps min down
        sf.set_max_value(50.0);
        assert!((sf.max_value() - 50.0).abs() < f64::EPSILON);
        assert!((sf.min_value() - 50.0).abs() < f64::EPSILON);

        // set_min_max_values
        sf.set_min_max_values(10.0, 90.0);
        assert!((sf.min_value() - 10.0).abs() < f64::EPSILON);
        assert!((sf.max_value() - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn constructor_clamps_max() {
        let look = Look::new();
        let sf = ScalarField::new(50.0, 10.0, look);
        // max < min => max clamped to min
        assert!((sf.max_value() - 50.0).abs() < f64::EPSILON);
        assert!((sf.min_value() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn scale_mark_intervals() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);

        // Default is [1]
        assert_eq!(sf.scale_mark_intervals(), &[1]);

        sf.set_scale_mark_intervals(&[100, 50, 10, 5, 1]);
        assert_eq!(sf.scale_mark_intervals(), &[100, 50, 10, 5, 1]);
    }

    #[test]
    #[should_panic(expected = "strictly descending")]
    fn scale_mark_intervals_rejects_non_descending() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        sf.set_scale_mark_intervals(&[10, 50]); // ascending — invalid
    }

    #[test]
    #[should_panic(expected = "must be > 0")]
    fn scale_mark_intervals_rejects_zero() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        sf.set_scale_mark_intervals(&[0]);
    }

    #[test]
    fn scale_mark_intervals_empty_is_ok() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        sf.set_scale_mark_intervals(&[]);
        assert_eq!(sf.scale_mark_intervals(), &[] as &[u64]);
    }

    #[test]
    fn never_hide_marks() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        assert!(!sf.is_never_hiding_marks());
        sf.set_never_hide_marks(true);
        assert!(sf.is_never_hiding_marks());
    }

    #[test]
    fn text_box_tallness() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        assert!((sf.text_box_tallness() - 0.5).abs() < f64::EPSILON);
        sf.set_text_box_tallness(0.75);
        assert!((sf.text_box_tallness() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn keyboard_interval() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        assert_eq!(sf.keyboard_interval(), 0);
        sf.set_keyboard_interval(5);
        assert_eq!(sf.keyboard_interval(), 5);
    }

    #[test]
    fn step_by_keyboard_with_explicit_interval() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        sf.set_keyboard_interval(10);
        sf.set_value(50.0);
        sf.last_w = 200.0;

        sf.input(&InputEvent::press(InputKey::Key('+')));
        assert!((sf.value() - 60.0).abs() < 1.0);

        sf.input(&InputEvent::press(InputKey::Key('-')));
        assert!((sf.value() - 50.0).abs() < 1.0);
    }

    #[test]
    fn custom_text_of_value() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        sf.set_text_of_value_fn(Box::new(|val, _iv| format!("{}%", val)));
        // The function is stored and usable
        let text = (sf.text_of_value_fn)(50, 1);
        assert_eq!(text, "50%");
    }

    #[test]
    fn plus_minus_keys_work() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        sf.set_value(50.0);
        sf.last_w = 200.0;

        let handled = sf.input(&InputEvent::press(InputKey::Key('+')));
        assert!(handled);
        assert!(sf.value() > 50.0);
    }

    #[test]
    fn non_editable_cursor_is_default() {
        let look = Look::new();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        assert_eq!(sf.get_cursor(), Cursor::ResizeEW);
        sf.set_editable(false);
        assert_eq!(sf.get_cursor(), Cursor::Normal);
    }

    #[test]
    fn set_value_fires_callback() {
        let look = Look::new();
        let count = Rc::new(RefCell::new(0u32));
        let count_clone = count.clone();
        let mut sf = ScalarField::new(0.0, 100.0, look);
        sf.on_value = Some(Box::new(move |_v| {
            *count_clone.borrow_mut() += 1;
        }));

        sf.set_value(50.0);
        assert_eq!(*count.borrow(), 1);

        // Setting same value should not fire
        sf.set_value(50.0);
        assert_eq!(*count.borrow(), 1);
    }
}
