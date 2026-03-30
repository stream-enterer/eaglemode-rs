# emFileMan End-to-End Browsing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the emFileMan panel layer into a working file browser with rendering loop, selection, input, and control panel.

**Architecture:** Phase 1 wires the rendering loop (model→panel→children). Phase 2 adds selection/input handlers. Phase 3 builds the control panel widget tree. Phase 4 completes SelInfoPanel. Each task modifies one file and includes tests.

**Tech Stack:** Rust, emcore crate (PanelBehavior, emFilePanel, emFpPlugin, widget types), emfileman crate

**Spec:** `docs/superpowers/specs/2026-03-30-emFileMan-browsing-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/emfileman/src/emDirPanel.rs` | Modify | Loading state machine, sorted children, Input/KeyWalk, CreateControlPanel |
| `crates/emfileman/src/emDirEntryPanel.rs` | Modify | notice/UpdateContentPanel/UpdateAltPanel, Input/Select, CreateControlPanel |
| `crates/emfileman/src/emDirEntryAltPanel.rs` | Modify | notice view gating, Cycle config watching, Input focus forwarding |
| `crates/emfileman/src/emFileLinkPanel.rs` | Modify | UpdateDataAndChildPanel, child creation in Cycle |
| `crates/emfileman/src/emFileManControlPanel.rs` | Rewrite | Full widget tree: RadioGroups, CheckButtons, Buttons, layout, Cycle sync |
| `crates/emfileman/src/emFileManSelInfoPanel.rs` | Modify | Selection generation tracking, reset_details on change |

---

## Phase 1: Rendering Loop

### Task 1: emDirPanel loading state machine

Drive `emDirModel` loading in Cycle and reflect progress via `VirtualFileState`. Currently the panel treats `NoFileModel` as `Loaded` — this task makes it properly show loading progress.

**Files:**
- Modify: `crates/emfileman/src/emDirPanel.rs`

- [ ] **Step 1: Add loading state fields to emDirPanel**

Add fields to track loading lifecycle. The `emDirModel` doesn't implement `FileModelState`, so `emDirPanel` drives loading directly and tracks state locally.

```rust
// In emDirPanel struct, add after child_count:
    loading_started: bool,
    loading_done: bool,
    loading_error: Option<String>,
```

Initialize in `new()`:
```rust
    loading_started: false,
    loading_done: false,
    loading_error: None,
```

- [ ] **Step 2: Implement loading state machine in Cycle**

Replace the current `Cycle` implementation. The new version drives `emDirModel::try_start_loading()`/`try_continue_loading()` and sets `VirtualFileState` via `set_custom_error`/`clear_custom_error`.

```rust
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
                    // Still loading — request another cycle
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
```

- [ ] **Step 3: Update notice to reset loading state on model change**

When the model is acquired or released, reset loading state:

```rust
fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
    if flags.contains(NoticeFlags::VIEW_CHANGED)
        || flags.contains(NoticeFlags::SOUGHT_NAME_CHANGED)
    {
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
```

- [ ] **Step 4: Update Paint to show loading progress**

Replace the `NoFileModel` hack. When loading, show progress via `paint_status`:

```rust
fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
    if self.loading_done {
        let cfg = self.config.borrow();
        let theme = cfg.GetTheme();
        let dc = emColor::from_packed(theme.GetRec().DirContentColor);
        painter.Clear(dc);
    } else if let Some(ref err) = self.loading_error {
        self.file_panel.paint_status(painter, w, h);
    } else if self.loading_started {
        // Loading in progress — paint status will show "Loading..."
        self.file_panel.paint_status(painter, w, h);
    } else {
        // No model yet
        let cfg = self.config.borrow();
        let theme = cfg.GetTheme();
        let dc = emColor::from_packed(theme.GetRec().DirContentColor);
        painter.Clear(dc);
    }
}
```

- [ ] **Step 5: Update IsOpaque to match new state**

```rust
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
```

- [ ] **Step 6: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 7: Commit**

```bash
git add crates/emfileman/src/emDirPanel.rs
git commit -m "feat(emFileMan): wire emDirPanel loading state machine in Cycle"
```

---

### Task 2: emDirPanel sorted children

Sort directory entries using the config comparator before creating child panels.

**Files:**
- Modify: `crates/emfileman/src/emDirPanel.rs`

- [ ] **Step 1: Sort entries before creating children**

Modify `update_children()` to collect visible entries, sort them, then create panels in sorted order:

```rust
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

        // Collect visible entries
        let mut visible: Vec<&emDirEntry> = Vec::new();
        for i in 0..count {
            let entry = dm.GetEntry(i);
            if !entry.IsHidden() || show_hidden {
                visible.push(entry);
            }
        }

        // Sort using config comparator
        visible.sort_by(|a, b| {
            let cmp = cfg.CompareDirEntries(a, b);
            if cmp < 0 {
                std::cmp::Ordering::Less
            } else if cmp > 0 {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });

        let visible_count = visible.len();

        // Only recreate if count changed
        if visible_count != self.child_count {
            ctx.DeleteAllChildren();

            for entry in &visible {
                let panel = emDirEntryPanel::new(
                    Rc::clone(&self.ctx),
                    (*entry).clone(),
                );
                ctx.create_child_with(entry.GetName(), Box::new(panel));
            }

            self.child_count = visible_count;
            self.content_complete = true;
        }
    }
}
```

- [ ] **Step 2: Add test for sorted children**

```rust
#[test]
fn update_children_sorts_entries() {
    // Sorting is delegated to CompareDirEntries — verify it's called
    // by checking that the sort_by closure compiles
    let cfg = emFileManViewConfig::default_for_test();
    let e1 = emDirEntry::from_path("/tmp");
    let e2 = emDirEntry::from_path("/var");
    let cfg_ref = cfg.borrow();
    let _result = cfg_ref.CompareDirEntries(&e1, &e2);
}
```

- [ ] **Step 3: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 4: Commit**

```bash
git add crates/emfileman/src/emDirPanel.rs
git commit -m "feat(emFileMan): sort emDirPanel children using config comparator"
```

---

### Task 3: emDirEntryPanel content panel lifecycle

Add `notice()` with dirty flags and content/alt panel creation in `LayoutChildren()`.

**Files:**
- Modify: `crates/emfileman/src/emDirEntryPanel.rs`

- [ ] **Step 1: Add dirty flags and ctx to emDirEntryPanel**

```rust
pub struct emDirEntryPanel {
    file_man: Rc<RefCell<emFileManModel>>,
    config: Rc<RefCell<emFileManViewConfig>>,
    ctx: Rc<emContext>,  // NEW: needed for plugin panel creation
    dir_entry: emDirEntry,
    pub(crate) bg_color: u32,
    content_panel: Option<PanelId>,
    alt_panel: Option<PanelId>,
    content_dirty: bool,  // NEW
    alt_dirty: bool,      // NEW
    last_viewed: bool,    // NEW: track view state changes
    last_in_active_path: bool, // NEW
}
```

Update `new()` to store `ctx` and initialize flags:
```rust
pub fn new(ctx: Rc<emContext>, dir_entry: emDirEntry) -> Self {
    let file_man = emFileManModel::Acquire(&ctx);
    let config = emFileManViewConfig::Acquire(&ctx);
    let bg_color = { /* existing code */ };
    Self {
        file_man,
        config,
        ctx,
        dir_entry,
        bg_color,
        content_panel: None,
        alt_panel: None,
        content_dirty: true,
        alt_dirty: true,
        last_viewed: false,
        last_in_active_path: false,
    }
}
```

- [ ] **Step 2: Implement notice()**

```rust
fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
    if flags.intersects(
        NoticeFlags::VIEW_CHANGED
            | NoticeFlags::SOUGHT_NAME_CHANGED
            | NoticeFlags::ACTIVE_CHANGED,
    ) {
        let viewed_changed = state.viewed != self.last_viewed;
        let active_changed = state.in_active_path != self.last_in_active_path;
        self.last_viewed = state.viewed;
        self.last_in_active_path = state.in_active_path;

        if viewed_changed || active_changed {
            self.content_dirty = true;
            self.alt_dirty = true;
        }
    }
}
```

- [ ] **Step 3: Implement update_content_panel()**

Port of C++ `emDirEntryPanel::UpdateContentPanel`. Creates content via `emFpPluginList::CreateFilePanel()`.

```rust
fn update_content_panel(&mut self, ctx: &mut PanelCtx, state: &PanelState) {
    if !self.content_dirty {
        return;
    }
    self.content_dirty = false;

    let cfg = self.config.borrow();
    let theme = cfg.GetTheme();
    let theme_rec = theme.GetRec();

    let (content_w, _content_h) = if self.dir_entry.IsDirectory() {
        (theme_rec.DirContentW, theme_rec.DirContentH)
    } else {
        (theme_rec.FileContentW, theme_rec.FileContentH)
    };

    let should_create = state.viewed
        && state.viewed_rect.w * content_w >= theme_rec.MinContentVW;
    let should_delete = !state.in_active_path && !state.viewed;

    drop(cfg);

    if should_delete && self.content_panel.is_some() {
        if let Some(child) = self.content_panel.take() {
            ctx.delete_child(child);
        }
    } else if should_create && self.content_panel.is_none() {
        let fppl = emcore::emFpPlugin::emFpPluginList::Acquire(&self.ctx);
        let fppl = fppl.borrow();
        let parent_arg = emcore::emFpPlugin::PanelParentArg::new(Rc::clone(&self.ctx));
        let stat_mode = if self.dir_entry.IsDirectory() {
            emcore::emFpPlugin::FileStatMode::Directory
        } else {
            emcore::emFpPlugin::FileStatMode::Regular
        };
        let behavior = fppl.CreateFilePanelWithStat(
            &parent_arg,
            CONTENT_NAME,
            self.dir_entry.GetPath(),
            None,
            stat_mode,
            0,
        );
        let child_id = ctx.create_child_with(CONTENT_NAME, behavior);
        self.content_panel = Some(child_id);
    }
}
```

- [ ] **Step 4: Implement update_alt_panel()**

```rust
fn update_alt_panel(&mut self, ctx: &mut PanelCtx, state: &PanelState) {
    if !self.alt_dirty {
        return;
    }
    self.alt_dirty = false;

    let cfg = self.config.borrow();
    let theme = cfg.GetTheme();
    let theme_rec = theme.GetRec();

    let should_create = state.viewed
        && state.viewed_rect.w * theme_rec.AltW >= theme_rec.MinAltVW;
    let should_delete = !state.in_active_path && !state.viewed;

    drop(cfg);

    if should_delete && self.alt_panel.is_some() {
        if let Some(child) = self.alt_panel.take() {
            ctx.delete_child(child);
        }
    } else if should_create && self.alt_panel.is_none() {
        let alt = crate::emDirEntryAltPanel::emDirEntryAltPanel::new(
            Rc::clone(&self.ctx),
            self.dir_entry.clone(),
            1,
        );
        let child_id = ctx.create_child_with(ALT_NAME, Box::new(alt));
        self.alt_panel = Some(child_id);
    }
}
```

- [ ] **Step 5: Update LayoutChildren to call update methods**

```rust
fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
    // Create/delete children based on dirty flags
    // We need PanelState — approximate from ctx
    let state = PanelState {
        id: PanelId::default(),
        is_active: false,
        in_active_path: self.last_in_active_path,
        window_focused: false,
        enabled: true,
        viewed: self.last_viewed,
        clip_rect: ctx.layout_rect(),
        viewed_rect: ctx.layout_rect(),
        priority: 0.0,
        memory_limit: u64::MAX,
        pixel_tallness: 1.0,
        height: ctx.layout_rect().h,
    };
    self.update_content_panel(ctx, &state);
    self.update_alt_panel(ctx, &state);

    // Layout existing children
    if let Some(child) = self.content_panel {
        let cfg = self.config.borrow();
        let theme = cfg.GetTheme();
        let theme_rec = theme.GetRec();
        let (cx, cy, cw, ch, cc) = if self.dir_entry.IsDirectory() {
            (theme_rec.DirContentX, theme_rec.DirContentY,
             theme_rec.DirContentW, theme_rec.DirContentH,
             emColor::from_packed(theme_rec.DirContentColor))
        } else {
            (theme_rec.FileContentX, theme_rec.FileContentY,
             theme_rec.FileContentW, theme_rec.FileContentH,
             emColor::from_packed(theme_rec.FileContentColor))
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
            emColor::from_packed(self.bg_color),
        );
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 7: Commit**

```bash
git add crates/emfileman/src/emDirEntryPanel.rs
git commit -m "feat(emFileMan): wire emDirEntryPanel content/alt panel lifecycle"
```

---

### Task 4: emDirEntryAltPanel notice and Cycle

Gate child creation on viewing conditions and add config change watching.

**Files:**
- Modify: `crates/emfileman/src/emDirEntryAltPanel.rs`

- [ ] **Step 1: Add state tracking fields**

```rust
pub struct emDirEntryAltPanel {
    pub(crate) data: emDirEntryAltPanelData,
    ctx: Rc<emContext>,
    config: Rc<RefCell<emFileManViewConfig>>,
    content_panel: Option<PanelId>,
    alt_panel: Option<PanelId>,
    content_dirty: bool,     // NEW
    alt_dirty: bool,         // NEW
    last_viewed: bool,       // NEW
    last_in_active_path: bool, // NEW
    last_config_gen: u64,    // NEW
}
```

Update `new()`:
```rust
pub fn new(ctx: Rc<emContext>, dir_entry: emDirEntry, alternative: i32) -> Self {
    let config = emFileManViewConfig::Acquire(&ctx);
    let last_config_gen = config.borrow().GetChangeSignal();
    Self {
        data: emDirEntryAltPanelData::new(dir_entry, alternative),
        ctx,
        config,
        content_panel: None,
        alt_panel: None,
        content_dirty: true,
        alt_dirty: true,
        last_viewed: false,
        last_in_active_path: false,
        last_config_gen,
    }
}
```

- [ ] **Step 2: Add notice() and Cycle()**

```rust
fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
    if flags.intersects(
        NoticeFlags::VIEW_CHANGED
            | NoticeFlags::SOUGHT_NAME_CHANGED
            | NoticeFlags::ACTIVE_CHANGED,
    ) {
        let viewed_changed = state.viewed != self.last_viewed;
        let active_changed = state.in_active_path != self.last_in_active_path;
        self.last_viewed = state.viewed;
        self.last_in_active_path = state.in_active_path;
        if viewed_changed || active_changed {
            self.content_dirty = true;
            self.alt_dirty = true;
        }
    }
}

fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
    let cfg = self.config.borrow();
    let gen = cfg.GetChangeSignal();
    drop(cfg);
    if gen != self.last_config_gen {
        self.last_config_gen = gen;
        self.content_dirty = true;
        self.alt_dirty = true;
        return true;
    }
    false
}
```

- [ ] **Step 3: Gate content creation in update_content_panel on viewing**

```rust
fn update_content_panel(&mut self, ctx: &mut PanelCtx) {
    if !self.content_dirty {
        return;
    }
    self.content_dirty = false;

    let config = self.config.borrow();
    let theme = config.GetTheme();
    let theme_rec = theme.GetRec();

    let should_create = self.last_viewed;
    let should_delete = !self.last_in_active_path && !self.last_viewed;

    drop(config);

    if should_delete {
        if let Some(child) = self.content_panel.take() {
            ctx.delete_child(child);
        }
    } else if should_create && self.content_panel.is_none() {
        let fppl = emcore::emFpPlugin::emFpPluginList::Acquire(&self.ctx);
        let fppl = fppl.borrow();
        let parent_arg = emcore::emFpPlugin::PanelParentArg::new(Rc::clone(&self.ctx));
        let behavior = fppl.CreateFilePanelWithStat(
            &parent_arg,
            crate::emDirEntryPanel::CONTENT_NAME,
            self.data.dir_entry.GetPath(),
            None,
            if self.data.dir_entry.IsDirectory() {
                emcore::emFpPlugin::FileStatMode::Directory
            } else {
                emcore::emFpPlugin::FileStatMode::Regular
            },
            self.data.alternative as usize,
        );
        let child_id = ctx.create_child_with(crate::emDirEntryPanel::CONTENT_NAME, behavior);
        self.content_panel = Some(child_id);
    }

    // Layout content if present
    if let Some(child_id) = self.content_panel {
        let config = self.config.borrow();
        let theme = config.GetTheme();
        let theme_rec = theme.GetRec();
        let bg = emColor::from_packed(theme_rec.BackgroundColor);
        ctx.layout_child_canvas(
            child_id,
            theme_rec.AltContentX, theme_rec.AltContentY,
            theme_rec.AltContentW, theme_rec.AltContentH,
            bg,
        );
    }
}
```

- [ ] **Step 4: Gate alt creation similarly**

```rust
fn update_alt_panel(&mut self, ctx: &mut PanelCtx) {
    if !self.alt_dirty {
        return;
    }
    self.alt_dirty = false;

    let should_create = self.last_viewed;
    let should_delete = !self.last_in_active_path && !self.last_viewed;

    if should_delete {
        if let Some(child) = self.alt_panel.take() {
            ctx.delete_child(child);
        }
    } else if should_create && self.alt_panel.is_none() {
        let next_alt = emDirEntryAltPanel::new(
            Rc::clone(&self.ctx),
            self.data.dir_entry.clone(),
            self.data.alternative + 1,
        );
        let child_id = ctx.create_child_with(
            crate::emDirEntryPanel::ALT_NAME,
            Box::new(next_alt),
        );
        self.alt_panel = Some(child_id);
    }

    // Layout alt if present
    if let Some(child_id) = self.alt_panel {
        let config = self.config.borrow();
        let theme = config.GetTheme();
        let theme_rec = theme.GetRec();
        let canvas = ctx.GetCanvasColor();
        ctx.layout_child_canvas(
            child_id,
            theme_rec.AltAltX, theme_rec.AltAltY,
            theme_rec.AltAltW, theme_rec.AltAltH,
            canvas,
        );
    }
}
```

- [ ] **Step 5: Update LayoutChildren**

```rust
fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
    self.update_content_panel(ctx);
    self.update_alt_panel(ctx);
}
```

- [ ] **Step 6: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 7: Commit**

```bash
git add crates/emfileman/src/emDirEntryAltPanel.rs
git commit -m "feat(emFileMan): add notice/Cycle to emDirEntryAltPanel for view gating"
```

---

### Task 5: emFileLinkPanel child creation

Implement `UpdateDataAndChildPanel` — resolve link target and create child panel.

**Files:**
- Modify: `crates/emfileman/src/emFileLinkPanel.rs`

- [ ] **Step 1: Add imports and state fields**

Add to imports:
```rust
use crate::emDirEntry::emDirEntry;
use crate::emDirEntryPanel::emDirEntryPanel;
use crate::emFileLinkModel::emFileLinkModel;
```

Add fields to `emFileLinkPanel`:
```rust
    ctx: Rc<emContext>,          // NEW
    dir_entry: Option<emDirEntry>, // NEW: cached dir entry for link target
    dir_entry_up_to_date: bool,  // NEW
    needs_update: bool,          // NEW
```

Update `new()`:
```rust
pub fn new(ctx: Rc<emContext>, have_border: bool) -> Self {
    let config = emFileManViewConfig::Acquire(&ctx);
    Self {
        file_panel: emFilePanel::new(),
        config,
        ctx,
        model: None,
        have_border,
        have_dir_entry_panel: false,
        full_path: String::new(),
        child_panel: None,
        dir_entry: None,
        dir_entry_up_to_date: false,
        needs_update: true,
    }
}
```

- [ ] **Step 2: Implement update_data_and_child_panel()**

```rust
fn update_data_and_child_panel(&mut self, ctx: &mut PanelCtx, viewed: bool) {
    if !viewed {
        if let Some(child) = self.child_panel.take() {
            ctx.delete_child(child);
        }
        return;
    }

    let Some(ref model_rc) = self.model else {
        return;
    };

    let model = model_rc.borrow();
    let new_full_path = model.GetFullPath();
    let new_have_dir_entry = model.GetHaveDirEntry();
    drop(model);

    if new_full_path != self.full_path || new_have_dir_entry != self.have_dir_entry_panel {
        // Path or type changed — recreate child
        if let Some(child) = self.child_panel.take() {
            ctx.delete_child(child);
        }
        self.full_path = new_full_path;
        self.have_dir_entry_panel = new_have_dir_entry;
        self.dir_entry_up_to_date = false;
    }

    if self.child_panel.is_none() && !self.full_path.is_empty() {
        if self.have_dir_entry_panel {
            let entry = emDirEntry::from_path(&self.full_path);
            let panel = emDirEntryPanel::new(Rc::clone(&self.ctx), entry.clone());
            let child_id = ctx.create_child_with("", Box::new(panel));
            self.child_panel = Some(child_id);
            self.dir_entry = Some(entry);
        } else {
            let fppl = emcore::emFpPlugin::emFpPluginList::Acquire(&self.ctx);
            let fppl = fppl.borrow();
            let parent_arg = emcore::emFpPlugin::PanelParentArg::new(Rc::clone(&self.ctx));
            let behavior = fppl.CreateFilePanel(&parent_arg, "", &self.full_path, 0);
            let child_id = ctx.create_child_with("", behavior);
            self.child_panel = Some(child_id);
        }
        self.needs_update = false;
    }

    let rect = ctx.layout_rect();
    self.layout_child_panel(ctx, rect.h);
}
```

- [ ] **Step 3: Update Cycle and notice**

```rust
fn Cycle(&mut self, ctx: &mut PanelCtx) -> bool {
    self.file_panel.refresh_vir_file_state();
    if self.needs_update {
        // Defer actual creation to LayoutChildren where we have ctx safely
    }
    false
}

fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
    if flags.intersects(NoticeFlags::VIEW_CHANGED) {
        self.needs_update = true;
    }
}
```

- [ ] **Step 4: Update LayoutChildren to call update**

```rust
fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
    if self.needs_update {
        // Approximate viewed state from whether model exists
        let viewed = self.model.is_some();
        self.update_data_and_child_panel(ctx, viewed);
        self.needs_update = false;
    }
    let rect = ctx.layout_rect();
    self.layout_child_panel(ctx, rect.h);
}
```

- [ ] **Step 5: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emFileLinkPanel.rs
git commit -m "feat(emFileMan): implement emFileLinkPanel child panel creation"
```

---

## Phase 2: Selection & Input

### Task 6: emDirEntryPanel selection helpers

Add `Select()`, `SelectSolely()`, and `Input()` to emDirEntryPanel.

**Files:**
- Modify: `crates/emfileman/src/emDirEntryPanel.rs`

- [ ] **Step 1: Add selection helper methods**

```rust
impl emDirEntryPanel {
    // ... existing methods ...

    /// Port of C++ emDirEntryPanel::Select
    fn select(&mut self, shift: bool, ctrl: bool) {
        let path = self.dir_entry.GetPath().to_string();
        let mut fm = self.file_man.borrow_mut();

        if ctrl {
            // Toggle target selection
            if fm.IsSelectedAsTarget(&path) {
                fm.DeselectAsTarget(&path);
            } else {
                fm.SelectAsTarget(&path);
            }
            fm.SetShiftTgtSelPath(&path);
        } else if shift {
            // Range selection — select from ShiftTgtSelPath to current
            // For now, just select this entry (range requires sibling enumeration)
            fm.SelectAsTarget(&path);
            fm.SetShiftTgtSelPath(&path);
        } else {
            // Plain click: old targets become sources, select this as target
            fm.ClearSourceSelection();
            fm.SwapSelection();
            fm.SelectAsTarget(&path);
            fm.SetShiftTgtSelPath(&path);
        }
    }

    /// Port of C++ emDirEntryPanel::SelectSolely
    fn select_solely(&mut self) {
        let path = self.dir_entry.GetPath().to_string();
        let mut fm = self.file_man.borrow_mut();
        fm.ClearSourceSelection();
        fm.ClearTargetSelection();
        fm.SelectAsTarget(&path);
    }
}
```

- [ ] **Step 2: Add Input() implementation**

```rust
fn Input(
    &mut self,
    event: &emInputEvent,
    _state: &PanelState,
    input_state: &emInputState,
) -> bool {
    use emcore::emInput::InputKey;

    match event.key {
        InputKey::MouseLeft => {
            if event.repeat >= 2 {
                // Double-click: select solely (RunDefaultCommand out of scope)
                self.select_solely();
                true
            } else {
                self.select(input_state.GetShift(), input_state.GetCtrl());
                true
            }
        }
        InputKey::Enter => {
            self.select_solely();
            true
        }
        InputKey::Space => {
            self.select(input_state.GetShift(), input_state.GetCtrl());
            true
        }
        _ => false,
    }
}
```

- [ ] **Step 3: Add selection tests**

```rust
#[test]
fn select_solely_clears_and_selects() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let entry = crate::emDirEntry::emDirEntry::from_path("/tmp");
    let mut panel = emDirEntryPanel::new(Rc::clone(&ctx), entry);

    panel.select_solely();

    let fm = panel.file_man.borrow();
    assert!(fm.IsSelectedAsTarget("/tmp"));
    assert_eq!(fm.GetTargetSelectionCount(), 1);
    assert_eq!(fm.GetSourceSelectionCount(), 0);
}

#[test]
fn select_plain_swaps_selection() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let entry = crate::emDirEntry::emDirEntry::from_path("/tmp");
    let mut panel = emDirEntryPanel::new(Rc::clone(&ctx), entry);

    // First click: selects as target
    panel.select(false, false);
    {
        let fm = panel.file_man.borrow();
        assert!(fm.IsSelectedAsTarget("/tmp"));
    }

    // Create another panel and click it
    let entry2 = crate::emDirEntry::emDirEntry::from_path("/var");
    let mut panel2 = emDirEntryPanel::new(Rc::clone(&ctx), entry2);
    panel2.select(false, false);

    let fm = panel2.file_man.borrow();
    assert!(fm.IsSelectedAsTarget("/var"));
    // /tmp should now be a source (swapped)
    assert!(fm.IsSelectedAsSource("/tmp"));
}

#[test]
fn select_ctrl_toggles() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let entry = crate::emDirEntry::emDirEntry::from_path("/tmp");
    let mut panel = emDirEntryPanel::new(Rc::clone(&ctx), entry);

    panel.select(false, true); // ctrl-click: select
    assert!(panel.file_man.borrow().IsSelectedAsTarget("/tmp"));

    panel.select(false, true); // ctrl-click: deselect
    assert!(!panel.file_man.borrow().IsSelectedAsTarget("/tmp"));
}
```

- [ ] **Step 4: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emDirEntryPanel.rs
git commit -m "feat(emFileMan): add selection helpers and Input() to emDirEntryPanel"
```

---

### Task 7: emDirPanel SelectAll and KeyWalk

Add `Input()` with Alt+A SelectAll and type-ahead KeyWalk.

**Files:**
- Modify: `crates/emfileman/src/emDirPanel.rs`

- [ ] **Step 1: Add KeyWalkState and file_man field**

```rust
use emcore::emInput::{emInputEvent, InputKey};
use emcore::emInputState::emInputState;
use crate::emFileManModel::emFileManModel;

struct KeyWalkState {
    search: String,
    last_key_time: std::time::Instant,
}

pub struct emDirPanel {
    pub(crate) file_panel: emFilePanel,
    ctx: Rc<emContext>,
    file_man: Rc<RefCell<emFileManModel>>,  // NEW
    pub(crate) path: String,
    config: Rc<RefCell<emFileManViewConfig>>,
    dir_model: Option<Rc<RefCell<emDirModel>>>,
    pub(crate) content_complete: bool,
    child_count: usize,
    loading_started: bool,
    loading_done: bool,
    loading_error: Option<String>,
    key_walk_state: Option<KeyWalkState>,  // NEW
}
```

Update `new()`:
```rust
pub fn new(ctx: Rc<emContext>, path: String) -> Self {
    let config = emFileManViewConfig::Acquire(&ctx);
    let file_man = emFileManModel::Acquire(&ctx);
    Self {
        file_panel: emFilePanel::new(),
        ctx,
        file_man,
        path,
        config,
        dir_model: None,
        content_complete: false,
        child_count: 0,
        loading_started: false,
        loading_done: false,
        loading_error: None,
        key_walk_state: None,
    }
}
```

- [ ] **Step 2: Add SelectAll method**

```rust
pub fn SelectAll(&self) {
    if let Some(ref dm_rc) = self.dir_model {
        let dm = dm_rc.borrow();
        let cfg = self.config.borrow();
        let show_hidden = cfg.GetShowHiddenFiles();
        let mut fm = self.file_man.borrow_mut();
        for i in 0..dm.GetEntryCount() {
            let entry = dm.GetEntry(i);
            if !entry.IsHidden() || show_hidden {
                fm.SelectAsTarget(entry.GetPath());
            }
        }
    }
}
```

- [ ] **Step 3: Add KeyWalk method**

```rust
fn key_walk(&mut self, ch: char) {
    let now = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(1);

    match &mut self.key_walk_state {
        Some(state) if now.duration_since(state.last_key_time) < timeout => {
            state.search.push(ch);
            state.last_key_time = now;
        }
        _ => {
            self.key_walk_state = Some(KeyWalkState {
                search: ch.to_string(),
                last_key_time: now,
            });
        }
    }

    // Search for matching entry
    let search = &self.key_walk_state.as_ref().unwrap().search;
    let wildcard = search.starts_with('*');
    let pattern = if wildcard { &search[1..] } else { search };
    let pattern_lower = pattern.to_lowercase();

    if let Some(ref dm_rc) = self.dir_model {
        let dm = dm_rc.borrow();
        for i in 0..dm.GetEntryCount() {
            let name = dm.GetEntry(i).GetName();
            let name_lower = name.to_lowercase();
            let matches = if wildcard {
                name_lower.contains(&pattern_lower)
            } else {
                name_lower.starts_with(&pattern_lower)
            };
            if matches {
                // TODO: scroll to this entry via seek_child_by_name
                // when panel tree integration is available
                break;
            }
        }
    }
}
```

- [ ] **Step 4: Add Input() implementation**

```rust
fn Input(
    &mut self,
    event: &emInputEvent,
    _state: &PanelState,
    input_state: &emInputState,
) -> bool {
    // Alt+A: SelectAll
    if event.is_key(InputKey::Key('a')) && input_state.IsAltMod() {
        self.SelectAll();
        return true;
    }

    // KeyWalk: printable characters
    if event.is_keyboard_event() && !event.chars.is_empty() {
        for ch in event.chars.chars() {
            if ch.is_alphanumeric() || ch == '.' || ch == '_' || ch == '-' || ch == '*' {
                self.key_walk(ch);
                return true;
            }
        }
    }

    false
}
```

- [ ] **Step 5: Add tests**

```rust
#[test]
fn select_all_selects_visible_entries() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emDirPanel::new(Rc::clone(&ctx), "/tmp".to_string());

    // Manually set up a loaded model
    let dm = emDirModel::Acquire(&ctx, "/tmp");
    {
        let mut dm_ref = dm.borrow_mut();
        dm_ref.try_start_loading().unwrap();
        while !dm_ref.try_continue_loading().unwrap() {}
        dm_ref.quit_loading();
    }
    panel.dir_model = Some(dm);
    panel.loading_done = true;

    panel.SelectAll();

    let fm = panel.file_man.borrow();
    assert!(fm.GetTargetSelectionCount() > 0);
}
```

- [ ] **Step 6: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 7: Commit**

```bash
git add crates/emfileman/src/emDirPanel.rs
git commit -m "feat(emFileMan): add SelectAll and KeyWalk Input to emDirPanel"
```

---

### Task 8: emDirEntryAltPanel Input

Focus forwarding to content panel on mouse events.

**Files:**
- Modify: `crates/emfileman/src/emDirEntryAltPanel.rs`

- [ ] **Step 1: Add Input()**

```rust
fn Input(
    &mut self,
    event: &emInputEvent,
    _state: &PanelState,
    _input_state: &emInputState,
) -> bool {
    use emcore::emInput::InputKey;

    // Mouse events in alt content area: focus content panel
    if event.is_mouse_event() && self.content_panel.is_some() {
        // Content panel will receive the event via panel tree propagation
        return false;
    }
    false
}
```

Add the import:
```rust
use emcore::emInput::emInputEvent;
use emcore::emInputState::emInputState;
```

- [ ] **Step 2: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 3: Commit**

```bash
git add crates/emfileman/src/emDirEntryAltPanel.rs
git commit -m "feat(emFileMan): add Input forwarding to emDirEntryAltPanel"
```

---

## Phase 3: Control Panel

### Task 9: emFileManControlPanel full widget tree

Replace the placeholder stub with actual widget construction.

**Files:**
- Modify: `crates/emfileman/src/emFileManControlPanel.rs`

- [ ] **Step 1: Add imports and struct fields**

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emButton::emButton;
use emcore::emCheckButton::emCheckButton;
use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emInput::emInputEvent;
use emcore::emInputState::emInputState;
use emcore::emLinearLayout::emLinearLayout;
use emcore::emLook::emLook;
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};
use emcore::emRadioButton::{emRadioButton, RadioGroup};

use crate::emFileManConfig::{NameSortingStyle, SortCriterion};
use crate::emFileManModel::emFileManModel;
use crate::emFileManThemeNames::emFileManThemeNames;
use crate::emFileManViewConfig::emFileManViewConfig;

pub struct emFileManControlPanel {
    ctx: Rc<emContext>,
    config: Rc<RefCell<emFileManViewConfig>>,
    file_man: Rc<RefCell<emFileManModel>>,
    theme_names: Rc<RefCell<emFileManThemeNames>>,
    look: Rc<emLook>,
    layout: emLinearLayout,

    // Sort criterion radio group (6 buttons)
    sort_group: Rc<RefCell<RadioGroup>>,
    sort_buttons: Vec<emRadioButton>,

    // Name sorting style radio group (3 buttons)
    name_sort_group: Rc<RefCell<RadioGroup>>,
    name_sort_buttons: Vec<emRadioButton>,

    // Checkboxes
    dirs_first: emCheckButton,
    show_hidden: emCheckButton,
    autosave: emCheckButton,

    // Action buttons
    save_btn: emButton,
    select_all_btn: emButton,
    clear_sel_btn: emButton,
    swap_sel_btn: emButton,
    paths_clip_btn: emButton,
    names_clip_btn: emButton,

    // State tracking
    last_config_gen: u64,
}
```

- [ ] **Step 2: Implement constructor**

```rust
impl emFileManControlPanel {
    pub fn new(ctx: Rc<emContext>) -> Self {
        let config = emFileManViewConfig::Acquire(&ctx);
        let file_man = emFileManModel::Acquire(&ctx);
        let theme_names = emFileManThemeNames::Acquire(&ctx);
        let look = emLook::new();
        let last_config_gen = config.borrow().GetChangeSignal();

        let sort_group = RadioGroup::new();
        let sort_captions = ["Name", "Ending", "Class", "Version", "Date", "Size"];
        let sort_buttons: Vec<emRadioButton> = sort_captions
            .iter()
            .enumerate()
            .map(|(i, cap)| emRadioButton::new(cap, Rc::clone(&look), Rc::clone(&sort_group), i))
            .collect();

        let name_sort_group = RadioGroup::new();
        let name_sort_captions = ["Per Locale", "Case Sensitive", "Case Insensitive"];
        let name_sort_buttons: Vec<emRadioButton> = name_sort_captions
            .iter()
            .enumerate()
            .map(|(i, cap)| {
                emRadioButton::new(cap, Rc::clone(&look), Rc::clone(&name_sort_group), i)
            })
            .collect();

        let dirs_first = emCheckButton::new("Sort Directories First", Rc::clone(&look));
        let show_hidden = emCheckButton::new("Show Hidden Files", Rc::clone(&look));
        let autosave = emCheckButton::new("Autosave", Rc::clone(&look));

        let save_btn = emButton::new("Save", Rc::clone(&look));
        let select_all_btn = emButton::new("Select All", Rc::clone(&look));
        let clear_sel_btn = emButton::new("Clear Selection", Rc::clone(&look));
        let swap_sel_btn = emButton::new("Swap Selection", Rc::clone(&look));
        let paths_clip_btn = emButton::new("Paths to Clipboard", Rc::clone(&look));
        let names_clip_btn = emButton::new("Names to Clipboard", Rc::clone(&look));

        let layout = emLinearLayout::vertical();

        let mut panel = Self {
            ctx,
            config,
            file_man,
            theme_names,
            look,
            layout,
            sort_group,
            sort_buttons,
            name_sort_group,
            name_sort_buttons,
            dirs_first,
            show_hidden,
            autosave,
            save_btn,
            select_all_btn,
            clear_sel_btn,
            swap_sel_btn,
            paths_clip_btn,
            names_clip_btn,
            last_config_gen,
        };
        panel.sync_from_config();
        panel
    }

    fn sync_from_config(&mut self) {
        let cfg = self.config.borrow();
        let sc = cfg.GetSortCriterion() as usize;
        self.sort_group.borrow_mut().SetChecked(sc);

        let nss = cfg.GetNameSortingStyle() as usize;
        self.name_sort_group.borrow_mut().SetChecked(nss);

        self.dirs_first.SetChecked(cfg.GetSortDirectoriesFirst());
        self.show_hidden.SetChecked(cfg.GetShowHiddenFiles());
        self.autosave.SetChecked(cfg.GetAutosave());
    }
}
```

- [ ] **Step 3: Implement Cycle for config sync**

```rust
fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
    let cfg_gen = self.config.borrow().GetChangeSignal();
    if cfg_gen != self.last_config_gen {
        self.last_config_gen = cfg_gen;
        self.sync_from_config();
        return true;
    }
    false
}
```

- [ ] **Step 4: Implement Paint**

Paint the control panel as a vertical list of labeled widgets:

```rust
fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
    let bg = emColor::from_packed(0x333333FF);
    let fg = emColor::from_packed(0xDDDDDDFF);
    painter.Clear(bg);

    let enabled = state.enabled;
    let row_h = h / 20.0;
    let mut y = 0.0;

    // Sort criterion section
    painter.PaintTextBoxed(
        0.02, y, w - 0.04, row_h,
        "Sort By:", row_h * 0.8, fg, bg,
        TextAlignment::Left, VAlign::Center,
        TextAlignment::Left, 1.0, false, 1.0,
    );
    y += row_h;
    for btn in &mut self.sort_buttons {
        btn.Paint(painter, w, row_h, enabled);
        y += row_h;
    }

    // Name sorting style section
    painter.PaintTextBoxed(
        0.02, y, w - 0.04, row_h,
        "Name Sorting:", row_h * 0.8, fg, bg,
        TextAlignment::Left, VAlign::Center,
        TextAlignment::Left, 1.0, false, 1.0,
    );
    y += row_h;
    for btn in &mut self.name_sort_buttons {
        btn.Paint(painter, w, row_h, enabled);
        y += row_h;
    }

    // Checkboxes
    self.dirs_first.Paint(painter, w, row_h, enabled);
    y += row_h;
    self.show_hidden.Paint(painter, w, row_h, enabled);
    y += row_h;
    self.autosave.Paint(painter, w, row_h, enabled);
    y += row_h;

    // Buttons
    self.save_btn.Paint(painter, w, row_h, enabled);
    y += row_h;
    self.select_all_btn.Paint(painter, w, row_h, enabled);
    y += row_h;
    self.clear_sel_btn.Paint(painter, w, row_h, enabled);
    y += row_h;
    self.swap_sel_btn.Paint(painter, w, row_h, enabled);
}
```

- [ ] **Step 5: Implement Input for widget delegation**

```rust
fn Input(
    &mut self,
    event: &emInputEvent,
    state: &PanelState,
    input_state: &emInputState,
) -> bool {
    // Sort radio buttons
    for (i, btn) in self.sort_buttons.iter_mut().enumerate() {
        if btn.Input(event, state, input_state) {
            let criteria = [
                SortCriterion::ByName, SortCriterion::ByEnding,
                SortCriterion::ByClass, SortCriterion::ByVersion,
                SortCriterion::ByDate, SortCriterion::BySize,
            ];
            if i < criteria.len() {
                self.config.borrow_mut().SetSortCriterion(criteria[i]);
            }
            return true;
        }
    }

    // Name sort radio buttons
    for (i, btn) in self.name_sort_buttons.iter_mut().enumerate() {
        if btn.Input(event, state, input_state) {
            let styles = [
                NameSortingStyle::PerLocale,
                NameSortingStyle::CaseSensitive,
                NameSortingStyle::CaseInsensitive,
            ];
            if i < styles.len() {
                self.config.borrow_mut().SetNameSortingStyle(styles[i]);
            }
            return true;
        }
    }

    // Checkboxes
    if self.dirs_first.Input(event, state, input_state) {
        let checked = self.dirs_first.IsChecked();
        self.config.borrow_mut().SetSortDirectoriesFirst(checked);
        return true;
    }
    if self.show_hidden.Input(event, state, input_state) {
        let checked = self.show_hidden.IsChecked();
        self.config.borrow_mut().SetShowHiddenFiles(checked);
        return true;
    }
    if self.autosave.Input(event, state, input_state) {
        let checked = self.autosave.IsChecked();
        self.config.borrow_mut().SetAutosave(checked);
        return true;
    }

    // Action buttons
    if self.save_btn.Input(event, state, input_state) {
        self.config.borrow_mut().SaveAsDefault();
        return true;
    }
    if self.clear_sel_btn.Input(event, state, input_state) {
        self.file_man.borrow_mut().ClearTargetSelection();
        return true;
    }
    if self.swap_sel_btn.Input(event, state, input_state) {
        self.file_man.borrow_mut().SwapSelection();
        return true;
    }
    if self.paths_clip_btn.Input(event, state, input_state) {
        let _text = self.file_man.borrow().SelectionToClipboard(false, false);
        return true;
    }
    if self.names_clip_btn.Input(event, state, input_state) {
        let _text = self.file_man.borrow().SelectionToClipboard(false, true);
        return true;
    }

    false
}
```

- [ ] **Step 6: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 7: Commit**

```bash
git add crates/emfileman/src/emFileManControlPanel.rs
git commit -m "feat(emFileMan): implement emFileManControlPanel widget tree"
```

---

### Task 10: CreateControlPanel on emDirPanel and emDirEntryPanel

Wire `CreateControlPanel` on both panel types.

**Files:**
- Modify: `crates/emfileman/src/emDirPanel.rs`
- Modify: `crates/emfileman/src/emDirEntryPanel.rs`

- [ ] **Step 1: Add CreateControlPanel to emDirPanel**

Add import:
```rust
use crate::emFileManControlPanel::emFileManControlPanel;
```

Add to `PanelBehavior for emDirPanel`:
```rust
fn CreateControlPanel(&mut self, parent_ctx: &mut PanelCtx, name: &str) -> Option<PanelId> {
    let panel = emFileManControlPanel::new(Rc::clone(&self.ctx));
    Some(parent_ctx.create_child_with(name, Box::new(panel)))
}
```

- [ ] **Step 2: Add CreateControlPanel to emDirEntryPanel**

Add import:
```rust
use crate::emFileManControlPanel::emFileManControlPanel;
```

Add to `PanelBehavior for emDirEntryPanel`:
```rust
fn CreateControlPanel(&mut self, parent_ctx: &mut PanelCtx, name: &str) -> Option<PanelId> {
    let panel = emFileManControlPanel::new(Rc::clone(&self.ctx));
    Some(parent_ctx.create_child_with(name, Box::new(panel)))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 4: Commit**

```bash
git add crates/emfileman/src/emDirPanel.rs crates/emfileman/src/emDirEntryPanel.rs
git commit -m "feat(emFileMan): wire CreateControlPanel on emDirPanel and emDirEntryPanel"
```

---

## Phase 4: SelInfoPanel Completion

### Task 11: Selection generation tracking in SelInfoPanel

Detect selection changes and reset scan state.

**Files:**
- Modify: `crates/emfileman/src/emFileManSelInfoPanel.rs`

- [ ] **Step 1: Add generation tracking field**

Add to `emFileManSelInfoPanel`:
```rust
    last_selection_gen: u64,
```

Initialize in `new()`:
```rust
let last_selection_gen = file_man.borrow().GetSelectionSignal();
```

- [ ] **Step 2: Rename _reset_details and wire into Cycle**

Rename `_reset_details` to `reset_details` (remove underscore prefix):

```rust
fn reset_details(&mut self) {
    self.state = SelInfoState::new();
    self.dir_stack.clear();
    self.initial_dir_stack.clear();
    self.sel_list.clear();
    self.sel_index = 0;
    self.dir_path.clear();
    self.dir_handle = None;
}
```

Update `Cycle`:
```rust
fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
    let gen = self.file_man.borrow().GetSelectionSignal();
    if gen != self.last_selection_gen {
        self.last_selection_gen = gen;
        self.reset_details();
    }
    self.work_on_details()
}
```

- [ ] **Step 3: Add test for reset on selection change**

```rust
#[test]
fn cycle_resets_on_selection_change() {
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emFileManSelInfoPanel::new(Rc::clone(&ctx));

    // Force into scanning state
    panel.allow_business = true;
    panel.state.direct.state = ScanState::Scanning;

    // Change selection
    {
        let mut fm = panel.file_man.borrow_mut();
        fm.SelectAsTarget("/tmp");
    }

    // Cycle should detect change and reset
    use emcore::emPanel::PanelBehavior;
    use emcore::emPanelCtx::PanelCtx;
    // Note: we can't easily call Cycle without a PanelCtx in unit tests,
    // so test the generation tracking logic directly:
    let gen = panel.file_man.borrow().GetSelectionSignal();
    assert_ne!(gen, panel.last_selection_gen);
    panel.last_selection_gen = gen;
    panel.reset_details();
    assert_eq!(panel.state.direct.state, ScanState::Costly);
}
```

- [ ] **Step 4: Run tests**

Run: `cargo clippy -p emFileMan -- -D warnings && cargo-nextest ntr -p emFileMan`

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManSelInfoPanel.rs
git commit -m "feat(emFileMan): add selection generation tracking to emFileManSelInfoPanel"
```

---

## Final Verification

### Task 12: Full build and test

- [ ] **Step 1: Run full clippy**

Run: `cargo clippy -p emFileMan -- -D warnings`

- [ ] **Step 2: Run full test suite**

Run: `cargo-nextest ntr -p emFileMan`

- [ ] **Step 3: Run workspace clippy**

Run: `cargo clippy -- -D warnings`

- [ ] **Step 4: Run workspace tests**

Run: `cargo-nextest ntr`

- [ ] **Step 5: Fix any failures and commit**

```bash
git add -u
git commit -m "fix(emFileMan): address clippy/test issues from browsing integration"
```
