# emFileMan Panel Layer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the PanelBehavior structs for all 7 emFileMan panel types, wire up the 3 FpPlugin entry points, and close the rendering loop so directory browsing works end-to-end.

**Architecture:** Each panel stub already has utility functions and data structures extracted. This plan adds a `struct` implementing `PanelBehavior` to each file, composing the existing utilities. Panels acquire models via `emContext::acquire()` at construction time (in FpPlugin entry points or parent panels). The panel tree drives lifecycle via `Cycle()`, `notice()`, `Paint()`, `LayoutChildren()`, and `Input()`.

**Tech Stack:** Rust, emcore (PanelBehavior, PanelCtx, PanelState, emPainter, emFilePanel, emFpPlugin), emfileman models (emDirModel, emFileManModel, emFileManViewConfig, emFileLinkModel)

**Spec:** `docs/superpowers/specs/2026-03-30-emFileMan-design.md`

---

## Reference: Key Imports and Signatures

These are used across multiple tasks. Refer back here instead of re-reading files.

```rust
// crates/emcore/src/emPanel.rs
pub trait PanelBehavior: AsAny {
    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {}
    fn Input(&mut self, event: &emInputEvent, state: &PanelState, input_state: &emInputState) -> bool { false }
    fn IsOpaque(&self) -> bool { false }
    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {}
    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {}
    fn Cycle(&mut self, ctx: &mut PanelCtx) -> bool { false }
    fn CreateControlPanel(&mut self, parent_ctx: &mut PanelCtx, name: &str) -> Option<PanelId> { None }
    fn GetIconFileName(&self) -> Option<String> { None }
    fn get_title(&self) -> Option<String> { None }
}

// NoticeFlags (bitflags)
// LAYOUT_CHANGED, FOCUS_CHANGED, VISIBILITY, VIEW_CHANGED, ACTIVE_CHANGED,
// SOUGHT_NAME_CHANGED, CHILDREN_CHANGED, CANVAS_CHANGED, ENABLE_CHANGED,
// UPDATE_PRIORITY_CHANGED, MEMORY_LIMIT_CHANGED, VIEW_FOCUS_CHANGED

// PanelState fields:
//   id: PanelId, is_active: bool, in_active_path: bool, viewed: bool,
//   height: f64, clip_rect: Rect, viewed_rect: Rect, priority: f64,
//   memory_limit: u64, pixel_tallness: f64, window_focused: bool, enabled: bool

// crates/emcore/src/emPanelCtx.rs
impl PanelCtx {
    pub fn create_child_with(&mut self, name: &str, behavior: Box<dyn PanelBehavior>) -> PanelId;
    pub fn delete_child(&mut self, child: PanelId);
    pub fn layout_child_canvas(&mut self, child: PanelId, x: f64, y: f64, w: f64, h: f64, canvas_color: emColor);
    pub fn find_child_by_name(&self, name: &str) -> Option<PanelId>;
    pub fn children(&self) -> Vec<PanelId>;
    pub fn layout_rect(&self) -> Rect;  // normalized: w=1.0, h=tallness
    pub fn GetCanvasColor(&self) -> emColor;
    pub fn set_focusable(&mut self, focusable: bool);
    pub fn DeleteAllChildren(&mut self);
}

// crates/emcore/src/emFilePanel.rs
pub struct emFilePanel { ... }
impl emFilePanel {
    pub fn new() -> Self;
    pub fn SetFileModel(&mut self, model: Option<Rc<RefCell<dyn FileModelState>>>);
    pub fn GetFileModel(&self) -> bool;
    pub fn GetVirFileState(&self) -> VirtualFileState;
    pub fn refresh_vir_file_state(&mut self);
    pub fn paint_status(&self, painter: &mut emPainter, w: f64, h: f64);
}
// VirtualFileState: Waiting, Loading{progress}, Loaded, Unsaved, Saving,
//                   TooCostly, LoadError(String), SaveError(String),
//                   NoFileModel, CustomError(String)

// crates/emcore/src/emFpPlugin.rs
pub type emFpPluginFunc = fn(
    parent: &PanelParentArg, name: &str, path: &str,
    plugin: &emFpPlugin, error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>>;

impl PanelParentArg {
    pub fn root_context(&self) -> &Rc<emContext>;
    pub fn parent_panel(&self) -> Option<PanelId>;
}

impl emFpPluginList {
    pub fn Acquire(root_context: &Rc<emContext>) -> Rc<RefCell<Self>>;
    pub fn CreateFilePanel(&self, parent: &PanelParentArg, name: &str,
        path: &str, alternative: usize) -> Box<dyn PanelBehavior>;
    pub fn CreateFilePanelWithStat(&self, parent: &PanelParentArg, name: &str,
        path: &str, stat_errno: i32, stat_mode: u32, alternative: usize) -> Box<dyn PanelBehavior>;
}

// crates/emcore/src/emPainter.rs (key methods)
impl emPainter {
    pub fn PaintRect(...);
    pub fn PaintRoundRect(...);
    pub fn PaintRoundRectOutline(...);
    pub fn PaintRectOutline(...);
    pub fn PaintTextBoxed(...);
    pub fn PaintText(...);
    pub fn PaintBorderImage(...);
    pub fn PaintPolygon(...);
    pub fn Clear(&mut self, color: emColor);
}
```

---

## Phase 1 — Leaf Panels (no child panel management)

### Task 1: emDirStatPanel — PanelBehavior Implementation

The simplest panel: counts directory entries and paints formatted text. C++ is 143 lines. The existing Rust file has `DirStatistics` with `from_entries()` and `format_text()`.

**Files:**
- Modify: `crates/emfileman/src/emDirStatPanel.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `crates/emfileman/src/emDirStatPanel.rs`:

```rust
    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emDirStatPanel::new(Rc::clone(&ctx));
        // Verify it implements PanelBehavior
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn panel_initial_vfs_is_no_model() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emDirStatPanel::new(Rc::clone(&ctx));
        assert_eq!(panel.file_panel.GetVirFileState(), emcore::emFilePanel::VirtualFileState::NoFileModel);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emDirStatPanel -- --test-threads=1`
Expected: FAIL — `emDirStatPanel` type not found

- [ ] **Step 3: Implement emDirStatPanel**

Add imports at top of `crates/emfileman/src/emDirStatPanel.rs`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emFilePanel::{emFilePanel, VirtualFileState};
use emcore::emPanel::{AsAny, NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};

use crate::emFileManViewConfig::emFileManViewConfig;
```

Add the panel struct after the existing `impl DirStatistics` block but before `#[cfg(test)]`:

```rust
/// Directory statistics panel.
/// Port of C++ `emDirStatPanel` (extends emFilePanel).
///
/// Counts entries by type (files, subdirectories, other, hidden) when the
/// directory model is loaded, and paints formatted statistics text.
pub struct emDirStatPanel {
    pub(crate) file_panel: emFilePanel,
    config: Rc<RefCell<emFileManViewConfig>>,
    stats: DirStatistics,
}

impl emDirStatPanel {
    pub fn new(ctx: Rc<emContext>) -> Self {
        let config = emFileManViewConfig::Acquire(&ctx);
        Self {
            file_panel: emFilePanel::new(),
            config,
            stats: DirStatistics {
                total_count: -1,
                file_count: -1,
                sub_dir_count: -1,
                other_type_count: -1,
                hidden_count: -1,
            },
        }
    }

    fn update_statistics(&mut self) {
        if self.file_panel.GetVirFileState() == VirtualFileState::Loaded {
            // In production, the emDirModel would be queried here.
            // For now, stats are updated via set_entries() which is called
            // by the parent panel (emDirPanel) when the model loads.
        } else {
            self.stats = DirStatistics {
                total_count: -1,
                file_count: -1,
                sub_dir_count: -1,
                other_type_count: -1,
                hidden_count: -1,
            };
        }
    }

    /// Update statistics from a slice of entries (called by parent when model loads).
    pub fn set_entries(&mut self, entries: &[crate::emDirEntry::emDirEntry]) {
        self.stats = DirStatistics::from_entries(entries);
    }
}

impl AsAny for emDirStatPanel {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl PanelBehavior for emDirStatPanel {
    fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
        self.file_panel.refresh_vir_file_state();
        self.update_statistics();
        false
    }

    fn IsOpaque(&self) -> bool {
        if self.file_panel.GetVirFileState() != VirtualFileState::Loaded {
            return false;
        }
        let config = self.config.borrow();
        let theme = config.GetTheme();
        let bg = theme.GetRec().BackgroundColor;
        (bg >> 24) == 0xFF
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        if self.file_panel.GetVirFileState() != VirtualFileState::Loaded {
            self.file_panel.paint_status(painter, w, h);
            return;
        }

        let config = self.config.borrow();
        let theme = config.GetTheme();
        let bg_color = emColor::new(theme.GetRec().BackgroundColor);
        painter.Clear(bg_color);

        let text = self.stats.format_text();
        let name_color = emColor::new(theme.GetRec().DirNameColor);
        painter.PaintTextBoxed(
            0.02, 0.02,
            w - 0.04, state.height - 0.04,
            &text,
            state.height,
            name_color,
            bg_color,
            TextAlignment::Left,
            VAlign::Top,
            TextAlignment::Left,
            0.5,
            false,
            1.0,
        );
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emDirStatPanel -- --test-threads=1`
Expected: All tests pass (4 existing + 2 new)

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p emfileman -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emDirStatPanel.rs
git commit -m "feat(emFileMan): implement emDirStatPanel PanelBehavior with Cycle and Paint"
```

---

### Task 2: emFileManSelInfoPanel — PanelBehavior with State Machine

Selection statistics panel with async directory scanning. C++ is 658 lines. The existing Rust file has `ScanDetails`, `SelInfoState`, `work_on_detail_entry()`, and `work_on_detail_entry_with_stack()`.

**Files:**
- Modify: `crates/emfileman/src/emFileManSelInfoPanel.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let panel = emFileManSelInfoPanel::new(Rc::clone(&ctx));
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn panel_initial_state() {
        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let panel = emFileManSelInfoPanel::new(Rc::clone(&ctx));
        assert_eq!(panel.state.direct.state, ScanState::Costly);
        assert!(!panel.allow_business);
    }

    #[test]
    fn set_rectangles_wide() {
        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let mut panel = emFileManSelInfoPanel::new(Rc::clone(&ctx));
        panel.set_rectangles(0.2); // wide panel (height < 0.3)
        assert!(panel.text_w > 0.0);
        assert!(panel.details_w > 0.0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileManSelInfoPanel -- --test-threads=1`
Expected: FAIL — `emFileManSelInfoPanel` type not found

- [ ] **Step 3: Implement emFileManSelInfoPanel**

Add imports at top of `crates/emfileman/src/emFileManSelInfoPanel.rs`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emPanel::{AsAny, NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};

use crate::emDirEntry::emDirEntry;
use crate::emFileManModel::emFileManModel;
```

Add the panel struct after `work_on_detail_entry_with_stack` but before `#[cfg(test)]`:

```rust
/// Selection statistics panel.
/// Port of C++ `emFileManSelInfoPanel` (extends emPanel).
pub struct emFileManSelInfoPanel {
    file_man: Rc<RefCell<emFileManModel>>,
    pub(crate) state: SelInfoState,
    pub(crate) allow_business: bool,
    dir_stack: Vec<String>,
    initial_dir_stack: Vec<String>,
    sel_list: Vec<String>,
    sel_index: usize,
    dir_path: String,
    dir_handle: Option<std::fs::ReadDir>,
    // Layout rectangles
    pub(crate) text_x: f64,
    pub(crate) text_y: f64,
    pub(crate) text_w: f64,
    pub(crate) text_h: f64,
    details_frame_x: f64,
    details_frame_y: f64,
    details_frame_w: f64,
    details_frame_h: f64,
    pub(crate) details_x: f64,
    details_y: f64,
    pub(crate) details_w: f64,
    details_h: f64,
}

impl emFileManSelInfoPanel {
    pub fn new(ctx: Rc<emContext>) -> Self {
        let file_man = emFileManModel::Acquire(&ctx);
        let mut panel = Self {
            file_man,
            state: SelInfoState::new(),
            allow_business: false,
            dir_stack: Vec::new(),
            initial_dir_stack: Vec::new(),
            sel_list: Vec::new(),
            sel_index: 0,
            dir_path: String::new(),
            dir_handle: None,
            text_x: 0.0, text_y: 0.0, text_w: 0.0, text_h: 0.0,
            details_frame_x: 0.0, details_frame_y: 0.0,
            details_frame_w: 0.0, details_frame_h: 0.0,
            details_x: 0.0, details_y: 0.0, details_w: 0.0, details_h: 0.0,
        };
        panel.set_rectangles(1.0);
        panel
    }

    /// Port of C++ SetRectangles.
    pub(crate) fn set_rectangles(&mut self, h: f64) {
        if h < 0.3 {
            let mut use_w = 1.0_f64;
            let mut use_h = 0.17_f64;
            if use_h > h {
                use_w *= h / use_h;
                use_h = h;
            }
            use_w -= use_h * 0.05;
            use_w -= use_h * 0.05;

            self.text_h = use_h;
            self.text_w = self.text_h / 0.29;
            self.text_x = (1.0 - use_w) * 0.5;
            self.text_y = (h - use_h) * 0.5;

            self.details_frame_h = use_h;
            self.details_frame_w = self.details_frame_h / 0.56;
            self.details_frame_x = self.text_x + use_w - self.details_frame_w;
            self.details_frame_y = self.text_y;
        } else {
            let mut use_w = 1.0_f64;
            let mut use_h = 0.76_f64;
            if use_h > h {
                use_w *= h / use_h;
                use_h = h;
            }
            use_w -= use_w * 0.05;
            use_h -= use_h * 0.05;

            self.text_w = use_w;
            self.text_h = self.text_w * 0.29;
            self.text_x = (1.0 - use_w) * 0.5;
            self.text_y = (h - use_h) * 0.5;

            self.details_frame_w = use_w;
            self.details_frame_h = self.details_frame_w * 0.44;
            self.details_frame_x = self.text_x;
            self.details_frame_y = self.text_y + use_h - self.details_frame_h;
        }

        self.details_w = self.details_frame_w * 0.3;
        self.details_h = self.details_w * 0.4667;
        self.details_x = self.details_frame_x + (self.details_frame_w - self.details_w) * 0.5;
        self.details_y = self.details_frame_y + (self.details_frame_h - self.details_h) * 0.5;
    }

    fn reset_details(&mut self) {
        self.state = SelInfoState::new();
        self.dir_stack.clear();
        self.initial_dir_stack.clear();
        self.sel_list.clear();
        self.dir_path.clear();
        self.dir_handle = None;
    }

    /// Port of C++ WorkOnDetails. Returns true if busy (should be called again).
    fn work_on_details(&mut self) -> bool {
        if !self.allow_business {
            match self.state.direct.state {
                ScanState::Wait => {
                    self.state.direct.state = ScanState::Costly;
                }
                ScanState::Scanning => {
                    self.state.direct.state = ScanState::Costly;
                    self.dir_stack.clear();
                    self.sel_list.clear();
                }
                _ => {}
            }
            match self.state.recursive.state {
                ScanState::Wait => {
                    self.state.recursive.state = ScanState::Costly;
                }
                ScanState::Scanning => {
                    self.state.recursive.state = ScanState::Costly;
                    self.dir_stack.clear();
                    self.dir_path.clear();
                    self.dir_handle = None;
                }
                _ => {}
            }
            return false;
        }

        // Direct scanning
        match self.state.direct.state {
            ScanState::Costly | ScanState::Wait => {
                self.state.direct = ScanDetails::new();
                self.state.direct.state = ScanState::Scanning;
                self.state.recursive.state = ScanState::Wait;
                let fm = self.file_man.borrow();
                let cnt = fm.GetTargetSelectionCount();
                self.sel_list.clear();
                for i in 0..cnt {
                    self.sel_list.push(fm.GetTargetSelection(i).to_string());
                }
                self.dir_stack.clear();
                self.sel_index = 0;
                return true;
            }
            ScanState::Scanning => {
                if self.sel_index >= self.sel_list.len() {
                    self.state.direct.state = ScanState::Success;
                    self.initial_dir_stack = self.dir_stack.clone();
                    self.dir_stack.clear();
                    self.sel_list.clear();
                    return true;
                }
                let path = self.sel_list[self.sel_index].clone();
                let entry = emDirEntry::from_path(&path);
                if entry.GetLStatErrNo() != 0 {
                    self.state.direct.state = ScanState::Error;
                    self.state.direct.error_message = format!(
                        "Failed to lstat \"{}\": errno {}",
                        entry.GetPath(), entry.GetLStatErrNo()
                    );
                    self.state.recursive.state = ScanState::Error;
                    self.state.recursive.error_message = self.state.direct.error_message.clone();
                    self.sel_list.clear();
                    self.dir_stack.clear();
                    return false;
                }
                work_on_detail_entry_with_stack(&mut self.state.direct, &entry, &mut self.dir_stack);
                // Accumulate size from lstat (matching C++)
                self.state.direct.size += entry.GetLStat().st_size as u64;
                #[cfg(target_os = "linux")]
                {
                    self.state.direct.disk_usage += (entry.GetLStat().st_blocks as u64) * 512;
                }
                #[cfg(not(target_os = "linux"))]
                {
                    self.state.direct.disk_usage_unknown = true;
                }
                self.sel_index += 1;
                return true;
            }
            _ => {}
        }

        // Recursive scanning
        match self.state.recursive.state {
            ScanState::Costly | ScanState::Wait => {
                self.state.recursive = self.state.direct.clone();
                self.state.recursive.state = ScanState::Scanning;
                self.dir_stack = self.initial_dir_stack.clone();
                return true;
            }
            ScanState::Scanning => {
                if self.dir_handle.is_none() {
                    if self.dir_stack.is_empty() {
                        self.state.recursive.state = ScanState::Success;
                        self.initial_dir_stack.clear();
                        return false;
                    }
                    self.dir_path = self.dir_stack.pop().unwrap();
                    match std::fs::read_dir(&self.dir_path) {
                        Ok(rd) => { self.dir_handle = Some(rd); }
                        Err(e) => {
                            self.state.recursive.state = ScanState::Error;
                            self.state.recursive.error_message = format!(
                                "Failed to read dir \"{}\": {}", self.dir_path, e
                            );
                            self.dir_stack.clear();
                            self.initial_dir_stack.clear();
                            self.dir_path.clear();
                            return false;
                        }
                    }
                    return true;
                }
                let dir_handle = self.dir_handle.as_mut().unwrap();
                match dir_handle.next() {
                    Some(Ok(de)) => {
                        let name = de.file_name().to_string_lossy().to_string();
                        let entry = emDirEntry::from_parent_and_name(&self.dir_path, &name);
                        if entry.GetLStatErrNo() != 0 {
                            self.state.recursive.state = ScanState::Error;
                            self.state.recursive.error_message = format!(
                                "Failed to lstat \"{}\": errno {}",
                                entry.GetPath(), entry.GetLStatErrNo()
                            );
                            self.dir_stack.clear();
                            self.initial_dir_stack.clear();
                            self.dir_path.clear();
                            self.dir_handle = None;
                            return false;
                        }
                        work_on_detail_entry_with_stack(
                            &mut self.state.recursive, &entry, &mut self.dir_stack
                        );
                        self.state.recursive.size += entry.GetLStat().st_size as u64;
                        #[cfg(target_os = "linux")]
                        {
                            self.state.recursive.disk_usage += (entry.GetLStat().st_blocks as u64) * 512;
                        }
                        #[cfg(not(target_os = "linux"))]
                        {
                            self.state.recursive.disk_usage_unknown = true;
                        }
                        return true;
                    }
                    Some(Err(e)) => {
                        self.state.recursive.state = ScanState::Error;
                        self.state.recursive.error_message = format!(
                            "Error reading dir \"{}\": {}", self.dir_path, e
                        );
                        self.dir_stack.clear();
                        self.initial_dir_stack.clear();
                        self.dir_path.clear();
                        self.dir_handle = None;
                        return false;
                    }
                    None => {
                        // Directory exhausted
                        self.dir_path.clear();
                        self.dir_handle = None;
                        return true;
                    }
                }
            }
            _ => {}
        }

        false
    }

    fn paint_details(
        painter: &mut emPainter,
        x: f64, y: f64, w: f64, h: f64,
        caption: &str,
        details: &ScanDetails,
        color: emColor,
        canvas_color: emColor,
    ) {
        painter.PaintTextBoxed(
            x, y, w, h * 0.3,
            caption, h * 0.3,
            color, canvas_color,
            TextAlignment::Center, VAlign::Center,
            TextAlignment::Left, 1.0, false, 1.0,
        );
        let y = y + h * 0.3;
        let h = h - h * 0.3;

        if details.state != ScanState::Success {
            let (msg, blend_color) = match details.state {
                ScanState::Costly => ("Costly", emColor::new(0x886666FF)),
                ScanState::Wait => ("Wait...", emColor::new(0x888800FF)),
                ScanState::Scanning => ("Scanning...", emColor::new(0x008800FF)),
                _ => {
                    let msg = if details.error_message.is_empty() {
                        "ERROR"
                    } else {
                        &details.error_message
                    };
                    (msg, emColor::new(0xFF0000FF))
                }
            };
            let blended = color.GetBlended(blend_color, 128);
            painter.PaintTextBoxed(
                x, y, w, h, msg, h * 0.1,
                blended, canvas_color,
                TextAlignment::Center, VAlign::Center,
                TextAlignment::Left, 1.0, false, 1.0,
            );
            return;
        }

        let d = h / 32.0;
        let text = format!("Entries: {}", details.entries);
        painter.PaintTextBoxed(x, y, w, d * 8.0, &text, d * 8.0, color, canvas_color,
            TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);

        let text = format!("Hidden Entries: {}", details.hidden_entries);
        painter.PaintTextBoxed(x, y + d * 9.0, w, d * 2.0, &text, d * 2.0, color, canvas_color,
            TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);

        let text = format!("Symbolic Links: {}", details.symbolic_links);
        painter.PaintTextBoxed(x, y + d * 12.0, w, d * 2.0, &text, d * 2.0, color, canvas_color,
            TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);

        let text = format!("Regular Files : {}", details.regular_files);
        painter.PaintTextBoxed(x, y + d * 14.0, w, d * 2.0, &text, d * 2.0, color, canvas_color,
            TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);

        let text = format!("Subdirectories: {}", details.subdirectories);
        painter.PaintTextBoxed(x, y + d * 16.0, w, d * 2.0, &text, d * 2.0, color, canvas_color,
            TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);

        let text = format!("Other Types   : {}", details.other_types);
        painter.PaintTextBoxed(x, y + d * 18.0, w, d * 2.0, &text, d * 2.0, color, canvas_color,
            TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);

        let text = format!("Size: {}", details.size);
        painter.PaintTextBoxed(x, y + d * 21.0, w, d * 8.0, &text, d * 8.0, color, canvas_color,
            TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);

        if details.disk_usage_unknown {
            let text = "Disk Usage: unknown";
            painter.PaintTextBoxed(x, y + d * 30.0, w, d * 2.0, text, d * 2.0, color, canvas_color,
                TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);
        } else {
            let text = format!("Disk Usage: {}", details.disk_usage);
            painter.PaintTextBoxed(x, y + d * 30.0, w, d * 2.0, &text, d * 2.0, color, canvas_color,
                TextAlignment::Left, VAlign::Top, TextAlignment::Left, 1.0, false, 1.0);
        }
    }
}

impl AsAny for emFileManSelInfoPanel {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl PanelBehavior for emFileManSelInfoPanel {
    fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
        // Check if selection changed — reset if so
        // (In production this would use signals; for now poll generation)
        self.work_on_details()
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.contains(NoticeFlags::LAYOUT_CHANGED) {
            self.set_rectangles(state.height);
        }
        if flags.contains(NoticeFlags::VIEW_CHANGED) {
            // Check if details area is visible and large enough
            self.allow_business = state.viewed;
        }
    }

    fn IsOpaque(&self) -> bool {
        false
    }

    fn Paint(&mut self, painter: &mut emPainter, _w: f64, _h: f64, _state: &PanelState) {
        let fm = self.file_man.borrow();
        let fg_src = emColor::new(0x80E080FF);
        let text = format!("Sources:{:4}", fm.GetSourceSelectionCount());
        painter.PaintTextBoxed(
            self.text_x, self.text_y,
            self.text_w, self.text_h * 0.5,
            &text, self.text_h * 0.5,
            fg_src, emColor::TRANSPARENT,
            TextAlignment::Left, VAlign::Center,
            TextAlignment::Left, 1.0, false, 1.0,
        );
        let fg_tgt = emColor::new(0xE08080FF);
        let text = format!("Targets:{:4}", fm.GetTargetSelectionCount());
        painter.PaintTextBoxed(
            self.text_x, self.text_y + self.text_h * 0.5,
            self.text_w, self.text_h * 0.5,
            &text, self.text_h * 0.5,
            fg_tgt, emColor::TRANSPARENT,
            TextAlignment::Left, VAlign::Center,
            TextAlignment::Left, 1.0, false, 1.0,
        );
        drop(fm);

        // 3D frame trapezoids (simplified — paint as colored rects)
        let canvas = emColor::TRANSPARENT;
        painter.PaintRect(
            self.details_frame_x, self.details_frame_y,
            self.details_frame_w, self.details_frame_h,
            emColor::new(0x00000030), canvas,
        );

        // Details area
        let s = self.details_w;
        let x = self.details_x;
        let y = self.details_y;
        let h = s * 0.48;

        let bg1 = emColor::new(0x880000FF);
        let fg1 = emColor::new(0xE0E0E0FF);
        let bg2 = fg1;
        let fg2 = emColor::new(0x000000FF);

        painter.PaintTextBoxed(
            x, y, s, s * 0.1,
            "Target Selection Details", s * 0.1,
            bg1, canvas,
            TextAlignment::Center, VAlign::Center,
            TextAlignment::Left, 1.0, false, 1.0,
        );

        painter.PaintRoundRect(
            x + s * 0.15, y + s * 0.13,
            s * 0.84, s * 0.34,
            s * 0.03, s * 0.03,
            bg2, canvas,
        );

        painter.PaintRoundRect(
            x, y + s * 0.22,
            s * 0.28, s * 0.16,
            s * 0.02, s * 0.02,
            bg1, canvas,
        );

        Self::paint_details(
            painter,
            x + s * 0.01, y + s * 0.23,
            s * 0.26, s * 0.14,
            "Direct",
            &self.state.direct,
            fg1, bg1,
        );
        Self::paint_details(
            painter,
            x + s * 0.33, y + s * 0.15,
            s * 0.52, s * 0.28,
            "Recursive",
            &self.state.recursive,
            fg2, bg2,
        );
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileManSelInfoPanel -- --test-threads=1`
Expected: All tests pass

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p emfileman -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emFileManSelInfoPanel.rs
git commit -m "feat(emFileMan): implement emFileManSelInfoPanel PanelBehavior with scan state machine"
```

---

### Phase 1 Gate

- [ ] **Run full gate check**

Run: `cargo clippy --workspace -- -D warnings && cargo-nextest ntr`
Expected: All pass. No regressions.

---

## Phase 2 — Panels with Child Management

### Task 3: emFileLinkPanel — PanelBehavior with Child Panel

Link file display panel. C++ is 376 lines. Existing Rust has `CalcContentCoords()` and constants. The panel creates either an emDirEntryPanel or a plugin panel as its single child.

**Files:**
- Modify: `crates/emfileman/src/emFileLinkPanel.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emFileLinkPanel::new(Rc::clone(&ctx), true);
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn panel_have_border_flag() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emFileLinkPanel::new(Rc::clone(&ctx), true);
        assert!(panel.have_border);
        let panel2 = emFileLinkPanel::new(Rc::clone(&ctx), false);
        assert!(!panel2.have_border);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileLinkPanel -- --test-threads=1`
Expected: FAIL — `emFileLinkPanel` type not found

- [ ] **Step 3: Implement emFileLinkPanel**

Add imports and the panel struct to `crates/emfileman/src/emFileLinkPanel.rs`, after the existing `CalcContentCoords` function and before `#[cfg(test)]`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emFilePanel::{emFilePanel, VirtualFileState};
use emcore::emPanel::{AsAny, NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};

use crate::emDirEntry::emDirEntry;
use crate::emFileManViewConfig::emFileManViewConfig;
use crate::emFileLinkModel::emFileLinkModel;

/// File link panel.
/// Port of C++ `emFileLinkPanel` (extends emFilePanel).
///
/// Displays a linked file by resolving the target path and creating either
/// an emDirEntryPanel (if link has HaveDirEntry) or a plugin panel as child.
pub struct emFileLinkPanel {
    pub(crate) file_panel: emFilePanel,
    ctx: Rc<emContext>,
    config: Rc<RefCell<emFileManViewConfig>>,
    model: Option<Rc<RefCell<emFileLinkModel>>>,
    pub(crate) have_border: bool,
    have_dir_entry_panel: bool,
    dir_entry_up_to_date: bool,
    full_path: String,
    dir_entry: emDirEntry,
    child_panel: Option<PanelId>,
}

impl emFileLinkPanel {
    pub fn new(ctx: Rc<emContext>, have_border: bool) -> Self {
        let config = emFileManViewConfig::Acquire(&ctx);
        Self {
            file_panel: emFilePanel::new(),
            ctx,
            config,
            model: None,
            have_border,
            have_dir_entry_panel: false,
            dir_entry_up_to_date: false,
            full_path: String::new(),
            dir_entry: emDirEntry::new(),
            child_panel: None,
        }
    }

    pub fn set_link_model(&mut self, model: Rc<RefCell<emFileLinkModel>>) {
        self.model = Some(model);
    }

    fn update_data_and_child_panel(&mut self, ctx: &mut PanelCtx, state: &PanelState) {
        // If not viewed enough, delete child
        if !state.viewed {
            self.delete_child_panel(ctx);
        }

        if self.file_panel.GetVirFileState().is_good() {
            if let Some(ref model_rc) = self.model {
                let model = model_rc.borrow();
                let full_path = model.GetFullPath();
                let have_dep = model.GetHaveDirEntry();
                if self.have_dir_entry_panel != have_dep || self.full_path != full_path {
                    self.delete_child_panel(ctx);
                    self.full_path = full_path;
                    self.have_dir_entry_panel = have_dep;
                    self.dir_entry_up_to_date = false;
                }
            }
        } else {
            if self.child_panel.is_some() {
                self.delete_child_panel(ctx);
            }
            if self.child_panel.is_none() {
                self.full_path.clear();
                self.have_dir_entry_panel = false;
                self.dir_entry_up_to_date = false;
            }
        }

        // Update dir entry if needed
        if self.child_panel.is_some() && !self.dir_entry_up_to_date {
            self.dir_entry = emDirEntry::from_path(&self.full_path);
            self.dir_entry_up_to_date = true;
        }

        // Create child if conditions met
        if self.child_panel.is_none() && self.file_panel.GetVirFileState().is_good() && state.viewed {
            if !self.dir_entry_up_to_date {
                self.dir_entry = emDirEntry::from_path(&self.full_path);
                self.dir_entry_up_to_date = true;
            }
            self.create_child_panel(ctx);
        }
    }

    fn create_child_panel(&mut self, ctx: &mut PanelCtx) {
        if self.child_panel.is_some() {
            return;
        }
        // Create a file panel via the plugin system
        let fppl = emcore::emFpPlugin::emFpPluginList::Acquire(&self.ctx);
        let fppl = fppl.borrow();
        let parent_arg = emcore::emFpPlugin::PanelParentArg::new(Rc::clone(&self.ctx));
        let behavior = fppl.CreateFilePanelWithStat(
            &parent_arg, "",
            self.dir_entry.GetPath(),
            self.dir_entry.GetStatErrNo(),
            self.dir_entry.GetStat().st_mode,
            0,
        );
        let child_id = ctx.create_child_with("", behavior);
        self.child_panel = Some(child_id);
    }

    fn delete_child_panel(&mut self, ctx: &mut PanelCtx) {
        if let Some(child) = self.child_panel.take() {
            ctx.delete_child(child);
        }
    }

    fn layout_child_panel(&self, ctx: &mut PanelCtx, state: &PanelState) {
        if let Some(child) = self.child_panel {
            let config = self.config.borrow();
            let theme = config.GetTheme();
            let theme_rec = theme.GetRec();
            let (x, y, w, h) = CalcContentCoords(
                state.height,
                self.have_border,
                self.have_dir_entry_panel,
                theme_rec.Height,
                theme_rec.LnkPaddingL,
                theme_rec.LnkPaddingT,
                theme_rec.LnkPaddingR,
                theme_rec.LnkPaddingB,
            );
            let canvas = if self.have_dir_entry_panel {
                emColor::new(theme_rec.DirContentColor)
            } else if self.have_border {
                emColor::new(BORDER_BG_COLOR)
            } else {
                ctx.GetCanvasColor()
            };
            ctx.layout_child_canvas(child, x, y, w, h, canvas);
        }
    }
}

impl AsAny for emFileLinkPanel {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl PanelBehavior for emFileLinkPanel {
    fn Cycle(&mut self, ctx: &mut PanelCtx) -> bool {
        self.file_panel.refresh_vir_file_state();
        false
    }

    fn notice(&mut self, flags: NoticeFlags, _state: &PanelState) {
        // Child panel management deferred to Cycle for borrow safety
    }

    fn IsOpaque(&self) -> bool {
        if !self.file_panel.GetVirFileState().is_good() && self.child_panel.is_none() {
            return false;
        }
        if self.have_border {
            return (BORDER_BG_COLOR >> 24) == 0xFF;
        }
        false
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        if !self.file_panel.GetVirFileState().is_good() && self.child_panel.is_none() {
            self.file_panel.paint_status(painter, w, h);
            return;
        }

        let canvas_color = emColor::TRANSPARENT;

        if self.have_border {
            let bg = emColor::new(BORDER_BG_COLOR);
            let fg = emColor::new(BORDER_FG_COLOR);
            painter.Clear(bg);

            let config = self.config.borrow();
            let theme = config.GetTheme();
            let theme_rec = theme.GetRec();
            let (cx, cy, cw, ch) = CalcContentCoords(
                state.height, self.have_border, self.have_dir_entry_panel,
                theme_rec.Height,
                theme_rec.LnkPaddingL, theme_rec.LnkPaddingT,
                theme_rec.LnkPaddingR, theme_rec.LnkPaddingB,
            );

            // Border outline
            let d = cx.min(cy) * 0.15;
            let t = cx.min(cy) * 0.03;
            painter.PaintRectOutline(
                cx - d * 0.5, cy - d * 0.5,
                cw + d, ch + d, t, fg, bg,
            );

            // Label
            let label = format!("emFileLink to {}", self.full_path);
            let ty = cx.min(cy) * 0.2;
            painter.PaintTextBoxed(
                ty, 0.0, 1.0 - ty * 2.0, cy - ty,
                &label, (cy - ty) * 0.9,
                fg, bg,
                TextAlignment::Center, VAlign::Center,
                TextAlignment::Left, 1.0, false, 1.0,
            );

            if self.have_dir_entry_panel {
                painter.PaintRect(
                    cx, cy, cw, ch,
                    emColor::new(theme_rec.DirContentColor), bg,
                );
            }
        } else if self.have_dir_entry_panel {
            let config = self.config.borrow();
            let theme = config.GetTheme();
            painter.Clear(emColor::new(theme.GetRec().DirContentColor));
        }
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();
        let mock_state = PanelState {
            height: rect.h,
            ..PanelState::default()
        };
        self.layout_child_panel(ctx, &mock_state);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileLinkPanel -- --test-threads=1`
Expected: All tests pass

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p emfileman -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emFileLinkPanel.rs
git commit -m "feat(emFileMan): implement emFileLinkPanel PanelBehavior with child panel management"
```

---

### Task 4: emDirEntryAltPanel — PanelBehavior with Recursive Alt Panels

Alternative content view. C++ is 334 lines. Creates content via `CreateFilePanel(..., alternative)` with incrementing index, and recursively nests alt panels.

**Files:**
- Modify: `crates/emfileman/src/emDirEntryAltPanel.rs`

- [ ] **Step 1: Write the failing test**

Add `#[cfg(test)]` section at end of `crates/emfileman/src/emDirEntryAltPanel.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let entry = crate::emDirEntry::emDirEntry::from_path("/tmp");
        let panel = emDirEntryAltPanel::new(Rc::clone(&ctx), entry, 1);
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn panel_has_correct_alternative_index() {
        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let entry = crate::emDirEntry::emDirEntry::from_path("/tmp");
        let panel = emDirEntryAltPanel::new(Rc::clone(&ctx), entry, 3);
        assert_eq!(panel.data.alternative, 3);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emDirEntryAltPanel -- --test-threads=1`
Expected: FAIL — `emDirEntryAltPanel` type not found

- [ ] **Step 3: Implement emDirEntryAltPanel**

Add imports and panel struct to `crates/emfileman/src/emDirEntryAltPanel.rs`, after the existing `emDirEntryAltPanelData` but before the new `#[cfg(test)]`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emPanel::{AsAny, NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};

use crate::emDirEntryPanel::{CONTENT_NAME, ALT_NAME};
use crate::emFileManModel::emFileManModel;
use crate::emFileManViewConfig::emFileManViewConfig;

/// Alternative content view panel.
/// Port of C++ `emDirEntryAltPanel` (extends emPanel).
pub struct emDirEntryAltPanel {
    pub(crate) data: emDirEntryAltPanelData,
    ctx: Rc<emContext>,
    file_man: Rc<RefCell<emFileManModel>>,
    config: Rc<RefCell<emFileManViewConfig>>,
    content_panel: Option<PanelId>,
    alt_panel: Option<PanelId>,
}

impl emDirEntryAltPanel {
    pub fn new(ctx: Rc<emContext>, dir_entry: emDirEntry, alternative: i32) -> Self {
        let file_man = emFileManModel::Acquire(&ctx);
        let config = emFileManViewConfig::Acquire(&ctx);
        Self {
            data: emDirEntryAltPanelData::new(dir_entry, alternative),
            ctx,
            file_man,
            config,
            content_panel: None,
            alt_panel: None,
        }
    }

    pub fn update_dir_entry(&mut self, dir_entry: emDirEntry) {
        if self.data.dir_entry == dir_entry {
            return;
        }
        self.data.dir_entry = dir_entry;
    }

    fn update_content_panel(
        &mut self,
        ctx: &mut PanelCtx,
        state: &PanelState,
        force_recreation: bool,
    ) {
        if force_recreation {
            if let Some(child) = self.content_panel.take() {
                ctx.delete_child(child);
            }
        }

        let config = self.config.borrow();
        let theme = config.GetTheme();
        let theme_rec = theme.GetRec();
        let content_w = theme_rec.AltContentW;
        let min_vw = theme_rec.MinContentVW;

        let should_create = state.viewed
            && state.viewed_rect.w * content_w >= min_vw;

        if should_create && self.content_panel.is_none() {
            let fppl = emcore::emFpPlugin::emFpPluginList::Acquire(&self.ctx);
            let fppl = fppl.borrow();
            let parent_arg = emcore::emFpPlugin::PanelParentArg::new(Rc::clone(&self.ctx));
            let behavior = fppl.CreateFilePanelWithStat(
                &parent_arg,
                CONTENT_NAME,
                self.data.dir_entry.GetPath(),
                self.data.dir_entry.GetStatErrNo(),
                self.data.dir_entry.GetStat().st_mode,
                self.data.alternative as usize,
            );
            let child_id = ctx.create_child_with(CONTENT_NAME, behavior);
            self.content_panel = Some(child_id);

            // Layout content panel
            let bg = emColor::new(theme_rec.BackgroundColor);
            ctx.layout_child_canvas(
                child_id,
                theme_rec.AltContentX, theme_rec.AltContentY,
                theme_rec.AltContentW, theme_rec.AltContentH,
                bg,
            );
        } else if !should_create {
            if let Some(child) = self.content_panel.take() {
                ctx.delete_child(child);
            }
        }
    }

    fn update_alt_panel(
        &mut self,
        ctx: &mut PanelCtx,
        state: &PanelState,
        force_recreation: bool,
    ) {
        if force_recreation {
            if let Some(child) = self.alt_panel.take() {
                ctx.delete_child(child);
            }
        }

        let config = self.config.borrow();
        let theme = config.GetTheme();
        let theme_rec = theme.GetRec();
        let alt_w = theme_rec.AltAltW;
        let min_vw = theme_rec.MinAltVW;

        let should_create = state.viewed
            && state.viewed_rect.w * alt_w >= min_vw;

        if should_create && self.alt_panel.is_none() {
            let next_alt = emDirEntryAltPanel::new(
                Rc::clone(&self.ctx),
                self.data.dir_entry.clone(),
                self.data.alternative + 1,
            );
            let child_id = ctx.create_child_with(ALT_NAME, Box::new(next_alt));
            self.alt_panel = Some(child_id);

            let canvas = ctx.GetCanvasColor();
            ctx.layout_child_canvas(
                child_id,
                theme_rec.AltAltX, theme_rec.AltAltY,
                theme_rec.AltAltW, theme_rec.AltAltH,
                canvas,
            );
        } else if !should_create {
            if let Some(child) = self.alt_panel.take() {
                ctx.delete_child(child);
            }
        }
    }
}

impl AsAny for emDirEntryAltPanel {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl PanelBehavior for emDirEntryAltPanel {
    fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
        false
    }

    fn notice(&mut self, _flags: NoticeFlags, _state: &PanelState) {
        // Content and alt panel updates happen in LayoutChildren
    }

    fn IsOpaque(&self) -> bool {
        false
    }

    fn Paint(&mut self, painter: &mut emPainter, _w: f64, _h: f64, _state: &PanelState) {
        let config = self.config.borrow();
        let theme = config.GetTheme();
        let theme_rec = theme.GetRec();

        let label = format!("Alternative Content Panel #{}", self.data.alternative);
        let label_color = emColor::new(theme_rec.LabelColor);
        let canvas = emColor::TRANSPARENT;

        painter.PaintTextBoxed(
            theme_rec.AltLabelX, theme_rec.AltLabelY,
            theme_rec.AltLabelW, theme_rec.AltLabelH,
            &label, theme_rec.AltLabelH,
            label_color, canvas,
            TextAlignment::Left, VAlign::Center,
            TextAlignment::Left, 0.5, false, 1.0,
        );

        // Content background
        let bg = emColor::new(theme_rec.BackgroundColor);
        painter.PaintRect(
            theme_rec.AltContentX, theme_rec.AltContentY,
            theme_rec.AltContentW, theme_rec.AltContentH,
            bg, canvas,
        );
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();
        let mock_state = PanelState {
            height: rect.h,
            viewed: true,
            viewed_rect: rect,
            ..PanelState::default()
        };
        self.update_content_panel(ctx, &mock_state, false);
        self.update_alt_panel(ctx, &mock_state, false);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emDirEntryAltPanel -- --test-threads=1`
Expected: All tests pass

- [ ] **Step 5: Run clippy + commit**

```bash
cargo clippy -p emfileman -- -D warnings
git add crates/emfileman/src/emDirEntryAltPanel.rs
git commit -m "feat(emFileMan): implement emDirEntryAltPanel PanelBehavior with recursive alt panels"
```

---

### Task 5: emDirEntryPanel — Struct, Cycle, Notice, IsOpaque, UpdateBgColor

The rendering workhorse (995 lines C++). Split across 4 tasks (5-8). This task establishes the struct and lifecycle methods.

**Files:**
- Modify: `crates/emfileman/src/emDirEntryPanel.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let entry = crate::emDirEntry::emDirEntry::from_path("/tmp");
        let panel = emDirEntryPanel::new(Rc::clone(&ctx), entry);
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn panel_initial_bg_color() {
        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let entry = crate::emDirEntry::emDirEntry::from_path("/tmp");
        let panel = emDirEntryPanel::new(Rc::clone(&ctx), entry);
        // Initial bg_color is the theme's BackgroundColor (no selection)
        assert_ne!(panel.bg_color, 0);
    }

    #[test]
    fn panel_get_title() {
        use emcore::emPanel::PanelBehavior;

        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let entry = crate::emDirEntry::emDirEntry::from_path("/tmp");
        let panel = emDirEntryPanel::new(Rc::clone(&ctx), entry);
        assert_eq!(panel.get_title(), Some("/tmp".to_string()));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emDirEntryPanel -- --test-threads=1`
Expected: FAIL — `emDirEntryPanel` type not found

- [ ] **Step 3: Implement emDirEntryPanel struct and lifecycle**

Add imports at top of `crates/emfileman/src/emDirEntryPanel.rs`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emPanel::{AsAny, NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};

use crate::emDirEntry::emDirEntry;
use crate::emFileManModel::emFileManModel;
use crate::emFileManViewConfig::emFileManViewConfig;
```

Add the panel struct after existing functions but before `#[cfg(test)]`:

```rust
/// Directory entry panel — displays a single file or directory.
/// Port of C++ `emDirEntryPanel` (extends emPanel).
///
/// The rendering workhorse of emFileMan. Draws themed background, name,
/// info, borders, and content area. Creates content panels via the plugin
/// system and alt panels for alternative views.
pub struct emDirEntryPanel {
    ctx: Rc<emContext>,
    file_man: Rc<RefCell<emFileManModel>>,
    config: Rc<RefCell<emFileManViewConfig>>,
    dir_entry: emDirEntry,
    pub(crate) bg_color: u32,
    recursive_call: bool,
    content_panel: Option<PanelId>,
    alt_panel: Option<PanelId>,
}

impl emDirEntryPanel {
    pub fn new(ctx: Rc<emContext>, dir_entry: emDirEntry) -> Self {
        let file_man = emFileManModel::Acquire(&ctx);
        let config = emFileManViewConfig::Acquire(&ctx);

        let bg_color = {
            let fm = file_man.borrow();
            let cfg = config.borrow();
            let theme = cfg.GetTheme();
            let theme_rec = theme.GetRec();
            let sel_src = fm.IsSelectedAsSource(dir_entry.GetPath());
            let sel_tgt = fm.IsSelectedAsTarget(dir_entry.GetPath());
            compute_bg_color(
                sel_src, sel_tgt,
                theme_rec.BackgroundColor,
                theme_rec.SourceSelectionColor,
                theme_rec.TargetSelectionColor,
            )
        };

        Self {
            ctx,
            file_man,
            config,
            dir_entry,
            bg_color,
            recursive_call: false,
            content_panel: None,
            alt_panel: None,
        }
    }

    pub fn GetDirEntry(&self) -> &emDirEntry {
        &self.dir_entry
    }

    pub fn UpdateDirEntry(&mut self, dir_entry: emDirEntry) {
        if self.dir_entry == dir_entry {
            return;
        }
        let path_changed = dir_entry.GetPath() != self.dir_entry.GetPath();
        self.dir_entry = dir_entry;
        if path_changed {
            self.update_bg_color();
        }
    }

    fn update_bg_color(&mut self) {
        let fm = self.file_man.borrow();
        let cfg = self.config.borrow();
        let theme = cfg.GetTheme();
        let theme_rec = theme.GetRec();
        let sel_src = fm.IsSelectedAsSource(self.dir_entry.GetPath());
        let sel_tgt = fm.IsSelectedAsTarget(self.dir_entry.GetPath());
        let new_bg = compute_bg_color(
            sel_src, sel_tgt,
            theme_rec.BackgroundColor,
            theme_rec.SourceSelectionColor,
            theme_rec.TargetSelectionColor,
        );
        self.bg_color = new_bg;
    }
}

impl AsAny for emDirEntryPanel {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl PanelBehavior for emDirEntryPanel {
    fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
        self.update_bg_color();
        false
    }

    fn notice(&mut self, _flags: NoticeFlags, _state: &PanelState) {
        // Content/alt panel updates deferred to LayoutChildren
    }

    fn IsOpaque(&self) -> bool {
        let cfg = self.config.borrow();
        let theme = cfg.GetTheme();
        let theme_rec = theme.GetRec();
        let bg_opaque = (self.bg_color >> 24) == 0xFF;
        bg_opaque
            && theme_rec.BackgroundX <= 0.0
            && theme_rec.BackgroundY <= 0.0
            && theme_rec.BackgroundW >= 1.0
            && theme_rec.BackgroundRX <= 0.0
            && theme_rec.BackgroundRY <= 0.0
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        let cfg = self.config.borrow();
        let theme = cfg.GetTheme();
        let theme_rec = theme.GetRec();
        let bg = emColor::new(self.bg_color);
        let canvas = emColor::TRANSPARENT;

        // Background rounded rect
        painter.PaintRoundRect(
            theme_rec.BackgroundX, theme_rec.BackgroundY,
            theme_rec.BackgroundW, theme_rec.BackgroundH,
            theme_rec.BackgroundRX, theme_rec.BackgroundRY,
            bg, canvas,
        );

        // Name color based on file type
        let name_color = if self.dir_entry.IsRegularFile() {
            let mode = self.dir_entry.GetStat().st_mode;
            if mode & (libc::S_IXUSR | libc::S_IXGRP | libc::S_IXOTH) != 0 {
                emColor::new(theme_rec.ExeNameColor)
            } else {
                emColor::new(theme_rec.NormalNameColor)
            }
        } else if self.dir_entry.IsDirectory() {
            emColor::new(theme_rec.DirNameColor)
        } else {
            emColor::new(theme_rec.OtherNameColor)
        };

        let name = self.dir_entry.GetName();
        painter.PaintTextBoxed(
            theme_rec.NameX, theme_rec.NameY,
            theme_rec.NameW, theme_rec.NameH,
            name, theme_rec.NameH,
            name_color, bg,
            TextAlignment::Left, VAlign::Center,
            TextAlignment::Left, 0.5, false, 1.0,
        );

        // Path (shown when content area is visible)
        let content_w = if self.dir_entry.IsDirectory() {
            theme_rec.DirContentW
        } else {
            theme_rec.FileContentW
        };

        if self.content_panel.is_some() || state.viewed_rect.w * content_w >= theme_rec.MinContentVW {
            painter.PaintTextBoxed(
                theme_rec.PathX, theme_rec.PathY,
                theme_rec.PathW, theme_rec.PathH,
                self.dir_entry.GetPath(), theme_rec.PathH,
                emColor::new(theme_rec.PathColor), bg,
                TextAlignment::Left, VAlign::Center,
                TextAlignment::Left, 0.5, false, 1.0,
            );

            // Content area background
            if self.dir_entry.IsDirectory() {
                painter.PaintRect(
                    theme_rec.DirContentX, theme_rec.DirContentY,
                    theme_rec.DirContentW, theme_rec.DirContentH,
                    emColor::new(theme_rec.DirContentColor), bg,
                );
            } else {
                painter.PaintRect(
                    theme_rec.FileContentX, theme_rec.FileContentY,
                    theme_rec.FileContentW, theme_rec.FileContentH,
                    emColor::new(theme_rec.FileContentColor), bg,
                );
            }
        }

        // Info area (permissions, owner, group, size, time)
        let info_color = emColor::new(theme_rec.InfoColor);
        let time_str = FormatTime(self.dir_entry.GetStat().st_mtime, false);
        painter.PaintTextBoxed(
            theme_rec.InfoX, theme_rec.InfoY,
            theme_rec.InfoW, theme_rec.InfoH,
            &time_str, theme_rec.InfoH,
            info_color, bg,
            TextAlignment::Left, VAlign::Center,
            TextAlignment::Left, 0.5, false, 1.0,
        );
    }

    fn get_title(&self) -> Option<String> {
        Some(self.dir_entry.GetPath().to_string())
    }

    fn GetIconFileName(&self) -> Option<String> {
        if self.dir_entry.IsDirectory() {
            Some("directory.tga".to_string())
        } else {
            Some("file.tga".to_string())
        }
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        // Content and alt panel layout
        if let Some(child) = self.content_panel {
            let cfg = self.config.borrow();
            let theme = cfg.GetTheme();
            let theme_rec = theme.GetRec();
            let (cx, cy, cw, ch, cc) = if self.dir_entry.IsDirectory() {
                (theme_rec.DirContentX, theme_rec.DirContentY,
                 theme_rec.DirContentW, theme_rec.DirContentH,
                 emColor::new(theme_rec.DirContentColor))
            } else {
                (theme_rec.FileContentX, theme_rec.FileContentY,
                 theme_rec.FileContentW, theme_rec.FileContentH,
                 emColor::new(theme_rec.FileContentColor))
            };
            ctx.layout_child_canvas(child, cx, cy, cw, ch, cc);
        }
        if let Some(child) = self.alt_panel {
            let cfg = self.config.borrow();
            let theme = cfg.GetTheme();
            let theme_rec = theme.GetRec();
            ctx.layout_child_canvas(
                child,
                theme_rec.AltX, theme_rec.AltY,
                theme_rec.AltW, theme_rec.AltH,
                emColor::new(self.bg_color),
            );
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emDirEntryPanel -- --test-threads=1`
Expected: All tests pass

- [ ] **Step 5: Run clippy + commit**

```bash
cargo clippy -p emfileman -- -D warnings
git add crates/emfileman/src/emDirEntryPanel.rs
git commit -m "feat(emFileMan): implement emDirEntryPanel PanelBehavior with themed rendering"
```

---

### Task 6: emDirPanel — PanelBehavior with Grid Layout and UpdateChildren

Directory grid panel. C++ is 448 lines. Existing Rust has `compute_grid_layout()`.

**Files:**
- Modify: `crates/emfileman/src/emDirPanel.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let panel = emDirPanel::new(Rc::clone(&ctx), "/tmp".to_string());
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn panel_initial_state() {
        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let panel = emDirPanel::new(Rc::clone(&ctx), "/tmp".to_string());
        assert_eq!(panel.path, "/tmp");
        assert!(!panel.content_complete);
    }

    #[test]
    fn panel_icon_filename() {
        use emcore::emPanel::PanelBehavior;

        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let panel = emDirPanel::new(Rc::clone(&ctx), "/tmp".to_string());
        assert_eq!(panel.GetIconFileName(), Some("directory.tga".to_string()));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emDirPanel -- --test-threads=1`
Expected: FAIL — `emDirPanel` type not found

- [ ] **Step 3: Implement emDirPanel**

Add imports and the panel struct to `crates/emfileman/src/emDirPanel.rs`, after `compute_grid_layout` but before `#[cfg(test)]`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emFilePanel::{emFilePanel, VirtualFileState};
use emcore::emPanel::{AsAny, NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};

use crate::emDirEntry::emDirEntry;
use crate::emDirEntryPanel::emDirEntryPanel;
use crate::emDirModel::emDirModel;
use crate::emFileManModel::emFileManModel;
use crate::emFileManViewConfig::emFileManViewConfig;

/// Directory grid panel.
/// Port of C++ `emDirPanel` (extends emFilePanel).
///
/// Displays directory entries in a grid layout. Lazily acquires emDirModel
/// when viewed. Creates/updates emDirEntryPanel children from model entries.
pub struct emDirPanel {
    pub(crate) file_panel: emFilePanel,
    ctx: Rc<emContext>,
    pub(crate) path: String,
    file_man: Rc<RefCell<emFileManModel>>,
    config: Rc<RefCell<emFileManViewConfig>>,
    dir_model: Option<Rc<RefCell<emDirModel>>>,
    pub(crate) content_complete: bool,
    key_walk_string: String,
    key_walk_active: bool,
}

impl emDirPanel {
    pub fn new(ctx: Rc<emContext>, path: String) -> Self {
        let file_man = emFileManModel::Acquire(&ctx);
        let config = emFileManViewConfig::Acquire(&ctx);
        Self {
            file_panel: emFilePanel::new(),
            ctx,
            path,
            file_man,
            config,
            dir_model: None,
            content_complete: false,
            key_walk_string: String::new(),
            key_walk_active: false,
        }
    }

    pub fn IsContentComplete(&self) -> bool {
        self.content_complete
    }

    pub fn GetPath(&self) -> &str {
        &self.path
    }

    fn update_children(&mut self, ctx: &mut PanelCtx) {
        if self.file_panel.GetVirFileState() == VirtualFileState::Loaded {
            if let Some(ref dm_rc) = self.dir_model {
                let dm = dm_rc.borrow();
                let config = self.config.borrow();
                let show_hidden = config.GetShowHiddenFiles();
                let count = dm.GetEntryCount();

                // Collect existing child names
                let existing: Vec<PanelId> = ctx.children();
                let mut existing_names: Vec<(PanelId, String)> = Vec::new();
                for child_id in &existing {
                    if let Some(name) = ctx.tree.GetRec(*child_id).map(|r| r.name.clone()) {
                        existing_names.push((*child_id, name));
                    }
                }

                // Mark which model entries are already covered
                let mut found = vec![false; count];
                for (child_id, child_name) in &existing_names {
                    if let Some(idx) = dm.GetEntryIndex(child_name) {
                        let entry = dm.GetEntry(idx);
                        if !entry.IsHidden() || show_hidden {
                            found[idx] = true;
                        } else {
                            ctx.delete_child(*child_id);
                        }
                    } else {
                        ctx.delete_child(*child_id);
                    }
                }

                // Create panels for new entries
                for i in 0..count {
                    if !found[i] {
                        let entry = dm.GetEntry(i);
                        if !entry.IsHidden() || show_hidden {
                            let panel = emDirEntryPanel::new(
                                Rc::clone(&self.ctx),
                                entry.clone(),
                            );
                            ctx.create_child_with(entry.GetName(), Box::new(panel));
                        }
                    }
                }

                self.content_complete = true;
            }
        } else {
            // Not loaded — remove children that aren't in active/viewed path
            self.content_complete = false;
        }
    }
}

impl AsAny for emDirPanel {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl PanelBehavior for emDirPanel {
    fn Cycle(&mut self, ctx: &mut PanelCtx) -> bool {
        self.file_panel.refresh_vir_file_state();
        self.update_children(ctx);
        false
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.contains(NoticeFlags::VIEW_CHANGED) || flags.contains(NoticeFlags::SOUGHT_NAME_CHANGED) {
            if state.viewed {
                if self.dir_model.is_none() {
                    self.dir_model = Some(emDirModel::Acquire(&self.ctx, &self.path));
                }
            } else if self.dir_model.is_some() {
                self.dir_model = None;
                self.file_panel.SetFileModel(None);
            }
        }
    }

    fn IsOpaque(&self) -> bool {
        match self.file_panel.GetVirFileState() {
            VirtualFileState::Loaded | VirtualFileState::NoFileModel => {
                let cfg = self.config.borrow();
                let theme = cfg.GetTheme();
                let dc = theme.GetRec().DirContentColor;
                (dc >> 24) == 0xFF
            }
            _ => false,
        }
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        match self.file_panel.GetVirFileState() {
            VirtualFileState::Loaded | VirtualFileState::NoFileModel => {
                let cfg = self.config.borrow();
                let theme = cfg.GetTheme();
                let dc = emColor::new(theme.GetRec().DirContentColor);
                painter.Clear(dc);
            }
            _ => {
                self.file_panel.paint_status(painter, w, h);
            }
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

        let canvas_color = match self.file_panel.GetVirFileState() {
            VirtualFileState::Loaded | VirtualFileState::NoFileModel => {
                emColor::new(theme_rec.DirContentColor)
            }
            _ => emColor::TRANSPARENT,
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
                if cw > 1.0 { cw = 1.0; }
                if cw < 0.001 { cw = 0.001; }
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emDirPanel -- --test-threads=1`
Expected: All tests pass

- [ ] **Step 5: Run clippy + commit**

```bash
cargo clippy -p emfileman -- -D warnings
git add crates/emfileman/src/emDirPanel.rs
git commit -m "feat(emFileMan): implement emDirPanel PanelBehavior with grid layout and UpdateChildren"
```

---

### Phase 2 Gate

- [ ] **Run full gate check**

Run: `cargo clippy --workspace -- -D warnings && cargo-nextest ntr`
Expected: All pass.

---

## Phase 3 — Control Panel and Plugin Wiring

### Task 7: emFileManControlPanel — Stub PanelBehavior

The full control panel (595 lines C++) builds a complex widget tree. This task creates a minimal PanelBehavior that paints a placeholder, establishing the type so other panels can reference it. Full widget construction is deferred to a follow-up plan once end-to-end browsing works.

**Files:**
- Modify: `crates/emfileman/src/emFileManControlPanel.rs`

- [ ] **Step 1: Write the failing test**

Add `#[cfg(test)]` section at end of `crates/emfileman/src/emFileManControlPanel.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let panel = emFileManControlPanel::new(Rc::clone(&ctx));
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emfileman --lib emFileManControlPanel -- --test-threads=1`
Expected: FAIL — `emFileManControlPanel` type not found

- [ ] **Step 3: Implement emFileManControlPanel stub**

Replace the contents of `crates/emfileman/src/emFileManControlPanel.rs`:

```rust
//! Sort/filter/theme UI control panel.
//!
//! Port of C++ `emFileManControlPanel`. Extends `emLinearLayout`.
//! Contains sort criterion radio buttons, name sorting style radio buttons,
//! directories-first and show-hidden checkboxes, theme selectors,
//! autosave checkbox, and command group buttons.
//!
//! DIVERGED: Full widget construction deferred. This version paints a
//! placeholder label. The control panel requires emLinearLayout, emPackGroup,
//! emRasterLayout, emRadioButton, emCheckButton, emButton widget integration
//! which will be ported as a follow-up.

use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emPanel::{AsAny, NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};

use crate::emFileManModel::emFileManModel;
use crate::emFileManViewConfig::emFileManViewConfig;

/// Control panel for file manager settings.
/// Port of C++ `emFileManControlPanel` (extends emLinearLayout).
pub struct emFileManControlPanel {
    file_man: Rc<RefCell<emFileManModel>>,
    config: Rc<RefCell<emFileManViewConfig>>,
}

impl emFileManControlPanel {
    pub fn new(ctx: Rc<emContext>) -> Self {
        let file_man = emFileManModel::Acquire(&ctx);
        let config = emFileManViewConfig::Acquire(&ctx);
        Self { file_man, config }
    }
}

impl AsAny for emFileManControlPanel {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl PanelBehavior for emFileManControlPanel {
    fn IsOpaque(&self) -> bool {
        false
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        let fg = emColor::new(0xCCCCCCFF);
        let bg = emColor::TRANSPARENT;
        painter.PaintTextBoxed(
            0.02, 0.02, w - 0.04, h - 0.04,
            "File Manager Control Panel\n(widget construction pending)",
            h * 0.1,
            fg, bg,
            TextAlignment::Center, VAlign::Center,
            TextAlignment::Center, 1.0, false, 1.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let sched = Rc::new(RefCell::new(emcore::emScheduler::EngineScheduler::new()));
        let ctx = emcore::emContext::emContext::NewRootWithScheduler(sched);
        let panel = emFileManControlPanel::new(Rc::clone(&ctx));
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p emfileman --lib emFileManControlPanel -- --test-threads=1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManControlPanel.rs
git commit -m "feat(emFileMan): implement emFileManControlPanel stub PanelBehavior"
```

---

### Task 8: FpPlugin Entry Points — Wire Up Panel Creation

Connect the 3 FpPlugin entry points to create the panel types. Currently they return `None` with TODO comments.

**Files:**
- Modify: `crates/emfileman/src/emDirFpPlugin.rs`
- Modify: `crates/emfileman/src/emDirStatFpPlugin.rs`
- Modify: `crates/emfileman/src/emFileLinkFpPlugin.rs`

- [ ] **Step 1: Update emDirFpPlugin to create emDirPanel**

Replace `crates/emfileman/src/emDirFpPlugin.rs`:

```rust
use std::rc::Rc;

use emcore::emFpPlugin::{emFpPlugin, PanelParentArg};
use emcore::emPanel::PanelBehavior;

use crate::emDirPanel::emDirPanel;

/// Entry point for the directory panel plugin.
/// Loaded via `emDir.emFpPlugin` config file.
#[no_mangle]
pub fn emDirFpPluginFunc(
    parent: &PanelParentArg,
    _name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emDirFpPlugin: No properties allowed.".to_string();
        return None;
    }
    Some(Box::new(emDirPanel::new(
        Rc::clone(parent.root_context()),
        path.to_string(),
    )))
}
```

- [ ] **Step 2: Update emDirStatFpPlugin to create emDirStatPanel**

Replace `crates/emfileman/src/emDirStatFpPlugin.rs`:

```rust
use std::rc::Rc;

use emcore::emFpPlugin::{emFpPlugin, PanelParentArg};
use emcore::emPanel::PanelBehavior;

use crate::emDirStatPanel::emDirStatPanel;

/// Entry point for the directory statistics panel plugin.
/// Loaded via `emDirStat.emFpPlugin` config file.
#[no_mangle]
pub fn emDirStatFpPluginFunc(
    parent: &PanelParentArg,
    _name: &str,
    _path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emDirStatFpPlugin: No properties allowed.".to_string();
        return None;
    }
    Some(Box::new(emDirStatPanel::new(
        Rc::clone(parent.root_context()),
    )))
}
```

- [ ] **Step 3: Update emFileLinkFpPlugin to create emFileLinkPanel**

Replace `crates/emfileman/src/emFileLinkFpPlugin.rs`:

```rust
use std::rc::Rc;

use emcore::emFpPlugin::{emFpPlugin, PanelParentArg};
use emcore::emPanel::PanelBehavior;

use crate::emFileLinkPanel::emFileLinkPanel;

/// Entry point for the file link panel plugin.
/// Loaded via `emFileLink.emFpPlugin` config file.
#[no_mangle]
pub fn emFileLinkFpPluginFunc(
    parent: &PanelParentArg,
    _name: &str,
    _path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emFileLinkFpPlugin: No properties allowed.".to_string();
        return None;
    }
    // Border depends on parent panel type — for now default to true
    // (correct determination requires checking parent's PanelBehavior type)
    Some(Box::new(emFileLinkPanel::new(
        Rc::clone(parent.root_context()),
        true,
    )))
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

- [ ] **Step 5: Run all tests**

Run: `cargo-nextest ntr`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emDirFpPlugin.rs crates/emfileman/src/emDirStatFpPlugin.rs crates/emfileman/src/emFileLinkFpPlugin.rs
git commit -m "feat(emFileMan): wire FpPlugin entry points to create panel types"
```

---

### Phase 3 Gate

- [ ] **Run full gate check**

Run: `cargo clippy --workspace -- -D warnings && cargo-nextest ntr`
Expected: All pass. No regressions. All existing tests still pass, plus new panel tests.

---

## Final Verification

- [ ] **Verify all panel types implement PanelBehavior**

For each file, verify the type exists and implements PanelBehavior:

| File | Type | PanelBehavior methods implemented |
|------|------|-----------------------------------|
| emDirStatPanel.rs | emDirStatPanel | Cycle, IsOpaque, Paint |
| emFileManSelInfoPanel.rs | emFileManSelInfoPanel | Cycle, notice, IsOpaque, Paint |
| emFileLinkPanel.rs | emFileLinkPanel | Cycle, notice, IsOpaque, Paint, LayoutChildren |
| emDirEntryAltPanel.rs | emDirEntryAltPanel | Cycle, notice, IsOpaque, Paint, LayoutChildren |
| emDirEntryPanel.rs | emDirEntryPanel | Cycle, notice, IsOpaque, Paint, get_title, GetIconFileName, LayoutChildren |
| emDirPanel.rs | emDirPanel | Cycle, notice, IsOpaque, Paint, LayoutChildren, GetIconFileName |
| emFileManControlPanel.rs | emFileManControlPanel | IsOpaque, Paint |

- [ ] **Verify all 3 FpPlugin entry points create panels**

| Entry Point | Creates |
|-------------|---------|
| emDirFpPluginFunc | emDirPanel |
| emDirStatFpPluginFunc | emDirStatPanel |
| emFileLinkFpPluginFunc | emFileLinkPanel |

- [ ] **Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore(emFileMan): final cleanup for panel layer completion"
```
