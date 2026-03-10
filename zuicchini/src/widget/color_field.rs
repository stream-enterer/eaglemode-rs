use std::rc::Rc;

use crate::foundation::{Color, Rect};
use crate::input::{InputEvent, InputKey, InputVariant};
use crate::panel::PanelCtx;
use crate::render::Painter;

use super::border::{Border, InnerBorderType, OuterBorderType};
use super::look::Look;

/// RGBA color editor widget.
pub struct ColorField {
    border: Border,
    look: Rc<Look>,
    color: Color,
    editable: bool,
    alpha_enabled: bool,
    expanded: bool,
    pub on_color: Option<Box<dyn FnMut(Color)>>,
}

const SWATCH_SIZE: f64 = 20.0;

impl ColorField {
    pub fn new(look: Rc<Look>) -> Self {
        Self {
            border: Border::new(OuterBorderType::Instrument)
                .with_inner(InnerBorderType::OutputField),
            look,
            color: Color::BLACK,
            editable: false,
            alpha_enabled: false,
            expanded: false,
            on_color: None,
        }
    }

    pub fn set_caption(&mut self, caption: &str) {
        self.border.caption = caption.to_string();
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn set_color(&mut self, color: Color) {
        if self.color != color {
            self.color = color;
            if let Some(cb) = &mut self.on_color {
                cb(color);
            }
        }
    }

    pub fn is_editable(&self) -> bool {
        self.editable
    }

    pub fn set_editable(&mut self, editable: bool) {
        if self.editable != editable {
            self.editable = editable;
            if editable {
                if self.border.inner == InnerBorderType::OutputField {
                    self.border.inner = InnerBorderType::InputField;
                }
            } else if self.border.inner == InnerBorderType::InputField {
                self.border.inner = InnerBorderType::OutputField;
            }
        }
    }

    pub fn is_alpha_enabled(&self) -> bool {
        self.alpha_enabled
    }

    pub fn set_alpha_enabled(&mut self, alpha_enabled: bool) {
        if self.alpha_enabled != alpha_enabled {
            self.alpha_enabled = alpha_enabled;
            if !alpha_enabled && self.color.a() != 255 {
                self.color = self.color.with_alpha(255);
                if let Some(cb) = &mut self.on_color {
                    cb(self.color);
                }
            }
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    /// Paint using C++ emColorField::PaintContent (emColorField.cpp:371-404).
    ///
    /// Gets content round rect, insets by d=min(w,h)*0.1, paints color rect + outline.
    pub fn paint(&self, painter: &mut Painter, w: f64, h: f64) {
        self.border
            .paint_border(painter, w, h, &self.look, false, true);

        // C++ PaintContent: GetContentRoundRect, then inset by d.
        let (cr, _r) = self.border.content_round_rect(w, h, &self.look);
        let d = cr.w.min(cr.h) * 0.1;

        let rx = cr.x + d;
        let ry = cr.y + d;
        let rw = (cr.w - 2.0 * d).max(0.0);
        let rh = (cr.h - 2.0 * d).max(0.0);

        // Paint color rect.
        painter.paint_rect(rx, ry, rw, rh, self.color);

        // Paint rect outline (C++ PaintRectOutline with d*0.08 thickness).
        let thickness = d * 0.08;
        let outline_color = self.look.input_fg_color;
        if thickness > 0.0 {
            // Top edge.
            painter.paint_rect(rx, ry, rw, thickness, outline_color);
            // Bottom edge.
            painter.paint_rect(rx, ry + rh - thickness, rw, thickness, outline_color);
            // Left edge.
            painter.paint_rect(rx, ry, thickness, rh, outline_color);
            // Right edge.
            painter.paint_rect(rx + rw - thickness, ry, thickness, rh, outline_color);
        }
    }

    pub fn input(&mut self, event: &InputEvent) -> bool {
        match event.key {
            InputKey::MouseLeft if event.variant == InputVariant::Release => {
                self.expanded = !self.expanded;
                true
            }
            _ => false,
        }
    }

    /// Layout child scalar fields for R, G, B, A editing when expanded.
    pub fn layout_children(&self, ctx: &mut PanelCtx, w: f64, h: f64) {
        let children = ctx.children();
        if !self.expanded {
            // Hide all children
            for &child in &children {
                ctx.layout_child(child, 0.0, 0.0, 0.0, 0.0);
            }
            return;
        }

        let Rect {
            x: cx,
            y: cy,
            w: cw,
            ..
        } = self.border.content_rect(w, h, &self.look);
        let field_h = 16.0;
        let start_y = cy + SWATCH_SIZE + 2.0;

        // Expect 4 children (R, G, B, A scalar fields)
        for (i, &child) in children.iter().take(4).enumerate() {
            ctx.layout_child(child, cx, start_y + i as f64 * (field_h + 2.0), cw, field_h);
        }
    }

    pub fn preferred_size(&self) -> (f64, f64) {
        if self.expanded {
            self.border
                .preferred_size_for_content(SWATCH_SIZE, SWATCH_SIZE + 4.0 * 18.0)
        } else {
            self.border
                .preferred_size_for_content(SWATCH_SIZE, SWATCH_SIZE)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_expanded() {
        let look = Look::new();
        let mut cf = ColorField::new(look);
        assert!(!cf.is_expanded());

        cf.input(&InputEvent::release(InputKey::MouseLeft));
        assert!(cf.is_expanded());

        cf.input(&InputEvent::release(InputKey::MouseLeft));
        assert!(!cf.is_expanded());
    }

    #[test]
    fn set_and_get_color() {
        let look = Look::new();
        let mut cf = ColorField::new(look);
        cf.set_color(Color::RED);
        assert_eq!(cf.color(), Color::RED);
    }
}
