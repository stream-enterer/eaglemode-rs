use crate::panel::{PanelBehavior, PanelCtx, PanelState};
use crate::render::Painter;
use crate::widget::{Border, InnerBorderType, Look, OuterBorderType};

use super::raster::RasterLayout;

/// RasterGroup wraps RasterLayout with border painting and focusable support.
pub struct RasterGroup {
    pub layout: RasterLayout,
    pub border: Border,
    pub look: Look,
}

impl RasterGroup {
    pub fn new() -> Self {
        Self {
            layout: RasterLayout::new(),
            border: Border::new(OuterBorderType::Group).with_inner(InnerBorderType::Group),
            look: Look::default(),
        }
    }
}

impl Default for RasterGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl PanelBehavior for RasterGroup {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, state: &PanelState) {
        let pixel_scale = state.viewed_rect.w * state.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.border
            .paint_border(painter, w, h, &self.look, state.is_focused(), state.enabled, pixel_scale);
    }

    fn layout_children(&mut self, ctx: &mut PanelCtx) {
        let aux_id = super::position_aux_panel(ctx, &self.border);
        let r = ctx.layout_rect();
        let cr = self.border.content_rect(r.w, r.h, &self.look);
        self.layout.do_layout_skip(ctx, aux_id, Some(cr));
        let cc = self
            .border
            .content_canvas_color(ctx.canvas_color(), &self.look, ctx.is_enabled());
        ctx.set_all_children_canvas_color(cc);
    }

    fn auto_expand(&self) -> bool {
        true
    }
}

