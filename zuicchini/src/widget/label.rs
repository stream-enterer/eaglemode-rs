use std::rc::Rc;

use crate::render::font_cache::FontCache;
use crate::render::Painter;

use super::border::{Border, OuterBorderType};
use super::look::Look;

/// Non-focusable text display widget.
pub struct Label {
    border: Border,
    look: Rc<Look>,
}

impl Label {
    pub fn new(caption: &str, look: Rc<Look>) -> Self {
        Self {
            border: Border::new(OuterBorderType::None).with_caption(caption),
            look,
        }
    }

    pub fn set_caption(&mut self, text: &str) {
        self.border.caption = text.to_string();
    }

    pub fn caption(&self) -> &str {
        &self.border.caption
    }

    pub fn paint(&self, painter: &mut Painter, w: f64, h: f64) {
        self.border
            .paint_border(painter, w, h, &self.look, false, true);
    }

    pub fn preferred_size(&self, font_cache: &FontCache) -> (f64, f64) {
        let size_px = FontCache::quantize_size(FontCache::DEFAULT_SIZE_PX);
        let tw = font_cache.measure_text(&self.border.caption, 0, size_px).0;
        // The caption row height: font size / 0.8 (label_layout uses 80% of
        // row height for font), so row height ≈ font_size / 0.8.
        // Label uses OuterBorderType::None which has zero insets.
        let row_h = FontCache::DEFAULT_SIZE_PX / 0.8;
        (tw + 4.0, row_h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_caption() {
        let look = Look::new();
        let mut label = Label::new("Hello", look);
        assert_eq!(label.caption(), "Hello");
        label.set_caption("World");
        assert_eq!(label.caption(), "World");
    }

    #[test]
    fn label_preferred_size() {
        let look = Look::new();
        let fc = FontCache::new();
        let label = Label::new("Test", look);
        let (w, h) = label.preferred_size(&fc);
        // Width = measured text width + 4px padding
        // Height = font_size / 0.8 (label_layout allocates 80% of row for font)
        assert!(w > 4.0, "Label should have positive width");
        let expected_h = FontCache::DEFAULT_SIZE_PX / 0.8;
        assert!(
            (h - expected_h).abs() < 0.01,
            "h={h}, expected={expected_h}"
        );
    }
}
