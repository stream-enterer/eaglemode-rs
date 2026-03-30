//! Port of C++ emDirPanel grid layout algorithm (LayoutChildren).

use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emFilePanel::emFilePanel;
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPainter::emPainter;

use crate::emDirEntryPanel::emDirEntryPanel;
use crate::emDirModel::emDirModel;
use crate::emFileManViewConfig::emFileManViewConfig;

pub struct LayoutRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Port of C++ emDirPanel::LayoutChildren grid algorithm.
/// theme_height is the theme's Height value.
/// panel_height is GetHeight() (the panel's actual height, typically GetHeight()).
/// pad_l/t/r/b are DirPaddingL/T/R/B from the theme.
pub fn compute_grid_layout(
    count: usize,
    theme_height: f64,
    panel_height: f64,
    pad_l: f64,
    pad_t: f64,
    pad_r: f64,
    pad_b: f64,
) -> Vec<LayoutRect> {
    if count == 0 {
        return Vec::new();
    }

    let t = theme_height;
    let h = panel_height;

    // Find minimum rows such that rows*cols >= count
    let mut rows = 1;
    loop {
        let mut cols = (rows as f64 * t / (h * (1.0 - 0.05 / rows as f64))) as i32;
        if cols <= 0 {
            cols = 1;
        }
        if (rows * cols as usize) >= count {
            break;
        }
        rows += 1;
    }
    let cols = count.div_ceil(rows);

    // Cell dimensions with padding
    let mut cw = 1.0 / (pad_l + cols as f64 + pad_r);
    let mut ch = h / (pad_t / t + rows as f64 + pad_b / t);
    if ch > cw * t {
        ch = cw * t;
    } else {
        cw = ch / t;
    }
    let mut cx = cw * pad_l;
    let cy = cw * pad_t;

    // Gap calculation
    let f = 1.0 - cw * (pad_l + pad_r);
    let n = (f / cw + 0.001) as i32;
    let mut gap = ((pad_t + pad_b) / t - (pad_l + pad_r)) * cw;
    gap = gap.min(f - n as f64 * cw);
    if gap < 0.0 {
        gap = 0.0;
    }
    gap /= (n + 1) as f64;
    cx += gap;

    // Column-major layout
    let mut rects = Vec::with_capacity(count);
    let mut col = 0;
    let mut row = 0;
    for _ in 0..count {
        rects.push(LayoutRect {
            x: cx + (cw + gap) * col as f64,
            y: cy + ch * row as f64,
            w: cw,
            h: ch,
        });
        row += 1;
        if row >= rows {
            col += 1;
            row = 0;
        }
    }
    rects
}

/// Directory grid panel.
/// Port of C++ `emDirPanel` (extends emFilePanel).
///
/// Displays directory entries in a grid layout. Lazily acquires emDirModel
/// when viewed. Creates/updates emDirEntryPanel children from model entries.
pub struct emDirPanel {
    pub(crate) file_panel: emFilePanel,
    ctx: Rc<emContext>,
    pub(crate) path: String,
    config: Rc<RefCell<emFileManViewConfig>>,
    dir_model: Option<Rc<RefCell<emDirModel>>>,
    pub(crate) content_complete: bool,
    child_count: usize,
    loading_started: bool,
    loading_done: bool,
    loading_error: Option<String>,
}

impl emDirPanel {
    pub fn new(ctx: Rc<emContext>, path: String) -> Self {
        let config = emFileManViewConfig::Acquire(&ctx);
        Self {
            file_panel: emFilePanel::new(),
            ctx,
            path,
            config,
            dir_model: None,
            content_complete: false,
            child_count: 0,
            loading_started: false,
            loading_done: false,
            loading_error: None,
        }
    }

    pub fn IsContentComplete(&self) -> bool {
        self.content_complete
    }

    pub fn GetPath(&self) -> &str {
        &self.path
    }

    fn update_children(&mut self, ctx: &mut PanelCtx) {
        if !self.loading_done {
            self.content_complete = false;
            return;
        }
        if let Some(ref dm_rc) = self.dir_model {
            let dm = dm_rc.borrow();
            let cfg = self.config.borrow();
            let show_hidden = cfg.GetShowHiddenFiles();
            let count = dm.GetEntryCount();

            // Count visible entries
            let mut visible_count = 0;
            for i in 0..count {
                let entry = dm.GetEntry(i);
                if !entry.IsHidden() || show_hidden {
                    visible_count += 1;
                }
            }

            // Only recreate if count changed
            if visible_count != self.child_count {
                ctx.DeleteAllChildren();

                for i in 0..count {
                    let entry = dm.GetEntry(i);
                    if !entry.IsHidden() || show_hidden {
                        let panel = emDirEntryPanel::new(
                            Rc::clone(&self.ctx),
                            entry.clone(),
                        );
                        ctx.create_child_with(entry.GetName(), Box::new(panel));
                    }
                }

                self.child_count = visible_count;
                self.content_complete = true;
            }
        }
    }
}

impl PanelBehavior for emDirPanel {
    fn Cycle(&mut self, ctx: &mut PanelCtx) -> bool {
        let mut changed = false;

        if let Some(ref dm_rc) = self.dir_model {
            let mut dm = dm_rc.borrow_mut();
            if !self.loading_started {
                match dm.try_start_loading() {
                    Ok(()) => {
                        self.loading_started = true;
                        self.loading_done = false;
                        self.loading_error = None;
                        self.file_panel.clear_custom_error();
                    }
                    Err(e) => {
                        self.loading_error = Some(e.clone());
                        self.file_panel.set_custom_error(&e);
                    }
                }
                changed = true;
            } else if !self.loading_done && self.loading_error.is_none() {
                match dm.try_continue_loading() {
                    Ok(true) => {
                        dm.quit_loading();
                        self.loading_done = true;
                        self.file_panel.clear_custom_error();
                        drop(dm);
                        self.update_children(ctx);
                        changed = true;
                    }
                    Ok(false) => {
                        changed = true;
                    }
                    Err(e) => {
                        self.loading_error = Some(e.clone());
                        self.file_panel.set_custom_error(&e);
                        changed = true;
                    }
                }
            } else if self.loading_done {
                drop(dm);
                self.update_children(ctx);
            }
        }

        self.file_panel.refresh_vir_file_state();
        changed
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.contains(NoticeFlags::VIEW_CHANGED) || flags.contains(NoticeFlags::SOUGHT_NAME_CHANGED) {
            if state.viewed {
                if self.dir_model.is_none() {
                    self.dir_model = Some(emDirModel::Acquire(&self.ctx, &self.path));
                    self.loading_started = false;
                    self.loading_done = false;
                    self.loading_error = None;
                    self.child_count = 0;
                    self.content_complete = false;
                }
            } else if self.dir_model.is_some() {
                self.dir_model = None;
                self.file_panel.SetFileModel(None);
                self.loading_started = false;
                self.loading_done = false;
                self.loading_error = None;
            }
        }
    }

    fn IsOpaque(&self) -> bool {
        if self.loading_done {
            let cfg = self.config.borrow();
            let theme = cfg.GetTheme();
            let dc = theme.GetRec().DirContentColor;
            (dc >> 24) == 0xFF
        } else {
            false
        }
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        if self.loading_done {
            let cfg = self.config.borrow();
            let theme = cfg.GetTheme();
            let dc = emColor::from_packed(theme.GetRec().DirContentColor);
            painter.Clear(dc);
        } else if self.loading_error.is_some() || self.loading_started {
            self.file_panel.paint_status(painter, w, h);
        } else {
            let cfg = self.config.borrow();
            let theme = cfg.GetTheme();
            let dc = emColor::from_packed(theme.GetRec().DirContentColor);
            painter.Clear(dc);
        }
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let children = ctx.children();
        let cnt = children.len();
        if cnt == 0 {
            return;
        }

        let cfg = self.config.borrow();
        let theme = cfg.GetTheme();
        let theme_rec = theme.GetRec();
        let rect = ctx.layout_rect();

        let canvas_color = if self.loading_done {
            emColor::from_packed(theme_rec.DirContentColor)
        } else {
            emColor::TRANSPARENT
        };

        if self.content_complete {
            let rects = compute_grid_layout(
                cnt,
                theme_rec.Height,
                rect.h,
                theme_rec.DirPaddingL,
                theme_rec.DirPaddingT,
                theme_rec.DirPaddingR,
                theme_rec.DirPaddingB,
            );
            for (i, child) in children.iter().enumerate() {
                if i < rects.len() {
                    ctx.layout_child_canvas(
                        *child,
                        rects[i].x, rects[i].y,
                        rects[i].w, rects[i].h,
                        canvas_color,
                    );
                }
            }
        } else {
            // Incomplete: clamp existing positions
            let t = theme_rec.Height;
            for child in &children {
                let mut cw = 0.5_f64;
                cw = cw.clamp(0.001, 1.0);
                let mut ch = cw * t;
                if ch > rect.h { ch = rect.h; cw = ch / t; }
                ctx.layout_child_canvas(*child, 0.0, 0.0, cw, ch, canvas_color);
            }
        }
    }

    fn GetIconFileName(&self) -> Option<String> {
        Some("directory.tga".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_layout_single_entry() {
        let rects = compute_grid_layout(1, 1.5, 1.0, 0.02, 0.02, 0.02, 0.02);
        assert_eq!(rects.len(), 1);
        assert!(rects[0].x >= 0.0);
        assert!(rects[0].y >= 0.0);
        assert!(rects[0].w > 0.0);
        assert!(rects[0].h > 0.0);
    }

    #[test]
    fn grid_layout_many_entries() {
        let rects = compute_grid_layout(20, 1.5, 1.0, 0.02, 0.02, 0.02, 0.02);
        assert_eq!(rects.len(), 20);
        for r in &rects {
            assert!(r.x >= 0.0);
            assert!(r.x + r.w <= 1.0 + 1e-9);
        }
    }

    #[test]
    fn grid_layout_column_major() {
        let rects = compute_grid_layout(4, 1.5, 1.5, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(rects.len(), 4);
        assert!((rects[0].x - rects[1].x).abs() < 1e-9);
    }

    #[test]
    fn grid_layout_empty() {
        let rects = compute_grid_layout(0, 1.5, 1.0, 0.02, 0.02, 0.02, 0.02);
        assert!(rects.is_empty());
    }

    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emDirPanel::new(Rc::clone(&ctx), "/tmp".to_string());
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn panel_initial_state() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emDirPanel::new(Rc::clone(&ctx), "/tmp".to_string());
        assert_eq!(panel.path, "/tmp");
        assert!(!panel.content_complete);
    }

    #[test]
    fn panel_icon_filename() {
        use emcore::emPanel::PanelBehavior;

        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emDirPanel::new(Rc::clone(&ctx), "/tmp".to_string());
        assert_eq!(panel.GetIconFileName(), Some("directory.tga".to_string()));
    }
}
