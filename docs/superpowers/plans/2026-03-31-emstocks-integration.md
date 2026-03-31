# emStocks Full Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire all 56 DIVERGED comment blocks and 10 TODOs in the emStocks crate to existing emCore primitives, eliminating deferred integration points.

**Architecture:** Bottom-up by layer. Each task gates on `cargo check` + `cargo clippy -- -D warnings` + `cargo-nextest ntr`. No new emCore primitives needed — all dependencies already exist.

**Tech Stack:** Rust, emcore crate (emClipboard, emEngine, emPainter, emTexture, emPanel/emBorder/emFilePanel, emListBox, widget types, emDialog, emTimer), `arboard` crate (clipboard), `open` crate (browser launching).

**Spec:** `docs/superpowers/specs/2026-03-31-emstocks-integration-design.md`

---

## Task 1: Add clipboard and browser dependencies (Phase 0)

**Files:**
- Modify: `Cargo.toml` (workspace root — add `arboard` and `open` to workspace dependencies)
- Modify: `crates/emstocks/Cargo.toml` (add `arboard` and `open` dependencies)

- [ ] **Step 1: Add workspace dependencies**

In the workspace root `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
arboard = "3"
open = "5"
```

- [ ] **Step 2: Add crate dependencies**

In `crates/emstocks/Cargo.toml`, add to `[dependencies]`:

```toml
arboard = { workspace = true }
open = { workspace = true }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p emstocks`
Expected: PASS (no code changes yet, just dependency additions)

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/emstocks/Cargo.toml
git commit -m "deps(emstocks): add arboard and open crates for clipboard and browser integration"
```

---

## Task 2: Wire clipboard operations in emStocksFilePanel (Phase 0, T1-T4)

**Files:**
- Modify: `crates/emstocks/src/emStocksFilePanel.rs`

- [ ] **Step 1: Add arboard import and wire clipboard in Input method**

At the top of `emStocksFilePanel.rs`, no new import needed — `arboard` is used inline.

Replace the Ctrl+X handler (lines 153-157):

```rust
InputKey::Key('X') => {
    // C++: ListBox->CutStocks()
    let clipboard_text = list_box.CutStocks(&mut self.rec);
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(&clipboard_text);
    }
    return true;
}
```

Replace the Ctrl+C handler (lines 159-163):

```rust
InputKey::Key('C') => {
    // C++: ListBox->CopyStocks()
    let clipboard_text = list_box.CopyStocks(&self.rec);
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(&clipboard_text);
    }
    return true;
}
```

Replace the Ctrl+V handler (lines 165-176):

```rust
InputKey::Key('V') => {
    // C++: ListBox->PasteStocks()
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(clipboard_text) = clipboard.get_text() {
            if !clipboard_text.is_empty() {
                let _result = list_box.PasteStocks(
                    &mut self.rec,
                    &self.config,
                    &clipboard_text,
                );
            }
        }
    }
    return true;
}
```

Replace the Ctrl+H handler (lines 190-199):

```rust
InputKey::Key('H') => {
    // C++: ListBox->FindSelected()
    let text = if let Ok(mut clipboard) = arboard::Clipboard::new() {
        clipboard.get_text().unwrap_or_else(|_| self.config.search_text.clone())
    } else {
        self.config.search_text.clone()
    };
    let _found = list_box.FindSelected(
        &self.rec,
        &mut self.config,
        &text,
    );
    return true;
}
```

- [ ] **Step 2: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/emstocks/src/emStocksFilePanel.rs
git commit -m "feat(emStocksFilePanel): wire system clipboard via arboard (T1-T4)"
```

---

## Task 3: Wire browser launching in emStocksFilePanel (Phase 0, T5-T6)

**Files:**
- Modify: `crates/emstocks/src/emStocksFilePanel.rs`

- [ ] **Step 1: Wire browser launch for Ctrl+W and Shift+Ctrl+W**

Replace the Ctrl+W handler (lines 184-188):

```rust
InputKey::Key('W') => {
    // C++: ListBox->ShowFirstWebPages()
    let pages = list_box.ShowFirstWebPages(&self.rec);
    for url in &pages {
        let _ = open::that(url);
    }
    return true;
}
```

Replace the Shift+Ctrl+W handler (lines 214-218):

```rust
InputKey::Key('W') => {
    // C++: ListBox->ShowAllWebPages()
    let pages = list_box.ShowAllWebPages(&self.rec);
    for url in &pages {
        let _ = open::that(url);
    }
    return true;
}
```

- [ ] **Step 2: Update the fetch dialog TODO (T7 deferred)**

Replace line 181:

```rust
// TODO(Phase 4): create real emStocksFetchPricesDialog
```

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emstocks/src/emStocksFilePanel.rs
git commit -m "feat(emStocksFilePanel): wire browser launching via open crate (T5-T6)"
```

---

## Task 4: Wire PaintTextBoxed in emStocksFilePanel (Phase 1, T8)

**Files:**
- Modify: `crates/emstocks/src/emStocksFilePanel.rs`

- [ ] **Step 1: Add PaintTextBoxed import and wire the call**

Add to imports at top of file:

```rust
use emcore::emPainter::{emPainter, TextAlignment, VAlign};
```

Replace the Paint method body (lines 30-46):

```rust
fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
    if self.vfs_good {
        painter.Clear(self.bg_color);

        if let Some(ref list_box) = self.list_box {
            if let Some(msg) = list_box.GetEmptyMessage() {
                painter.PaintTextBoxed(
                    0.0, 0.0, w, h,
                    msg,
                    h * 0.1,
                    emColor::rgb(255, 255, 255),
                    self.bg_color,
                    TextAlignment::Center,
                    VAlign::Center,
                    TextAlignment::Center,
                    0.0,
                    false,
                    0.0,
                );
            }
        }
    }
    // C++: if (!IsVFSGood()) emFilePanel::Paint(painter,canvasColor);
    // Base class paint for non-good state deferred until Phase 2 emFilePanel integration.
}
```

- [ ] **Step 2: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/emstocks/src/emStocksFilePanel.rs
git commit -m "feat(emStocksFilePanel): wire PaintTextBoxed for empty list message (T8)"
```

---

## Task 5: Wire gradient texture in PaintPriceBar (Phase 1, D36)

**Files:**
- Modify: `crates/emstocks/src/emStocksItemChart.rs`

- [ ] **Step 1: Add emTexture import**

Add to imports at top of `emStocksItemChart.rs`:

```rust
use emcore::emTexture::emTexture;
```

- [ ] **Step 2: Replace blended color with LinearGradient**

Replace lines 1001-1007 (the DIVERGED gradient code):

```rust
let bar_y = f64::min(y1, y2);
let bar_h = (y2 - y1).abs();
let gradient = emTexture::LinearGradient {
    color_a: c1.GetTransparented(30.0),
    color_b: c2.GetTransparented(10.0),
    start: (x, y1),
    end: (x, y2),
};
painter.paint_polygon_textured(
    &[
        (x, bar_y),
        (x + w, bar_y),
        (x + w, bar_y + bar_h),
        (x, bar_y + bar_h),
    ],
    &gradient,
    emColor::TRANSPARENT,
);
```

Remove the DIVERGED comment on lines 1001-1003.

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emstocks/src/emStocksItemChart.rs
git commit -m "feat(emStocksItemChart): use LinearGradient texture for PaintPriceBar (D36)"
```

---

## Task 6: Update PaintGraph stroke DIVERGED comment (Phase 1, D37)

**Files:**
- Modify: `crates/emstocks/src/emStocksItemChart.rs`

**Note:** The Rust `PaintLine` method does not accept stroke parameters (only color). `PaintPolyline` with thickness is already the closest equivalent to C++ per-segment `PaintLine` with `emRoundedStroke`. The current implementation is correct. The DIVERGED comment should be updated to explain this is a Rust API limitation, not a deferred integration.

- [ ] **Step 1: Update the DIVERGED comment**

Replace lines 1212-1213:

```rust
// DIVERGED: C++ uses per-segment PaintLine with emRoundedStroke and emStrokeEnd.
// Rust emPainter::PaintLine does not accept stroke parameters; PaintPolyline with
// thickness is the closest equivalent and produces visually similar output.
```

- [ ] **Step 2: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/emstocks/src/emStocksItemChart.rs
git commit -m "docs(emStocksItemChart): update PaintGraph stroke DIVERGED with API rationale (D37)"
```

---

## Task 7: Implement emEngine trait for PricesFetcher (Phase 1, D-hdr1, D-hdr2)

**Files:**
- Modify: `crates/emstocks/src/emStocksPricesFetcher.rs`

- [ ] **Step 1: Add emEngine import**

Add to imports:

```rust
use emcore::emEngine::{emEngine, EngineCtx};
```

- [ ] **Step 2: Implement emEngine trait**

The existing `Cycle` method takes `&mut self, rec: &mut emStocksRec`. The emEngine trait's `Cycle` takes `&mut self, ctx: &mut EngineCtx`. These signatures differ because the current Cycle needs the rec parameter (which will be resolved in Phase 4 when FileModel is integrated). For now, add the trait impl that delegates, and keep the existing method as an internal helper.

Add after the `impl emStocksPricesFetcher` block:

```rust
impl emEngine for emStocksPricesFetcher {
    fn Cycle(&mut self, _ctx: &mut EngineCtx<'_>) -> bool {
        // DIVERGED(Phase 4): FileModel/FileStateSignal/ChangeSignal integration pending.
        // Once FileModel is integrated, this will read rec from the model and check file state.
        // For now, this trait impl cannot call the internal Cycle because it needs
        // a &mut emStocksRec. The caller must use the direct Cycle(&mut rec) method.
        self.current_process_active
    }
}
```

- [ ] **Step 3: Update header DIVERGED comments**

Replace lines 2-3:

```rust
// DIVERGED(Phase 4): FileModel/FileStateSignal/ChangeSignal integration pending.
// emEngine trait is implemented but the trait Cycle cannot drive the fetch loop
// until FileModel integration provides access to the rec.
```

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emstocks/src/emStocksPricesFetcher.rs
git commit -m "feat(emStocksPricesFetcher): implement emEngine trait (D-hdr1), defer FileModel (D-hdr2)"
```

---

## Task 8: Wire IsVFSGood and Cycle in emStocksFilePanel (Phase 2, D28, T10)

**Files:**
- Modify: `crates/emstocks/src/emStocksFilePanel.rs`

- [ ] **Step 1: Add emFilePanel import and field**

Add to imports:

```rust
use emcore::emFilePanel::{emFilePanel, VirtualFileState};
```

Add `file_panel` field to the struct:

```rust
pub struct emStocksFilePanel {
    pub(crate) bg_color: emColor,
    pub(crate) config: emStocksConfig,
    pub(crate) list_box: Option<emStocksListBox>,
    /// DIVERGED(Phase 4): FileModel ownership pending. C++ FileModel is emStocksFileModel*
    /// with full lifecycle. Rust uses emStocksRec directly until Phase 4.
    pub(crate) rec: emStocksRec,
    pub(crate) file_panel: emFilePanel,
}
```

Remove the `vfs_good` field and its DIVERGED comment (lines 23-26).

- [ ] **Step 2: Update new() constructor**

```rust
pub(crate) fn new() -> Self {
    Self {
        bg_color: emColor::from_packed(0x131520FF),
        config: emStocksConfig::default(),
        list_box: None,
        rec: emStocksRec::default(),
        file_panel: emFilePanel::new(),
    }
}
```

- [ ] **Step 3: Replace vfs_good with file_panel.GetVirFileState().is_good()**

In `Paint`:

```rust
fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
    if self.file_panel.GetVirFileState().is_good() {
        // ... existing body unchanged ...
    }
}
```

In `Input`:

```rust
if !self.file_panel.GetVirFileState().is_good() || self.list_box.is_none() {
    return false;
}
```

- [ ] **Step 4: Wire Cycle to check VirFileState**

```rust
fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
    self.file_panel.refresh_vir_file_state();
    let state = self.file_panel.GetVirFileState();
    if state.is_good() && self.list_box.is_none() {
        self.list_box = Some(emStocksListBox::new());
    }
    // TODO(Phase 4): ListBox as real panel child.
    false
}
```

- [ ] **Step 5: Update all tests that reference vfs_good**

Replace `panel.vfs_good = false` with nothing (default state is not-good).
Replace `panel.vfs_good = true` with setting up the file_panel to have a good state. If `emFilePanel::new()` defaults to not-good, the `make_active_panel` helper needs adjustment.

Check what `emFilePanel::new()` defaults to. If it's `NoFileModel`, tests that need vfs_good=true will need to set a mock file model or set a custom state. The simplest approach: add a test helper method.

```rust
fn make_active_panel() -> emStocksFilePanel {
    let mut panel = emStocksFilePanel::new();
    // Set file panel to a good state for testing
    panel.file_panel.SetFileModel(/* test model that reports Loaded */);
    panel.list_box = Some(emStocksListBox::new());
    panel
}
```

The exact approach depends on `emFilePanel`'s API for setting test states. Read the file to determine available methods.

- [ ] **Step 6: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/emstocks/src/emStocksFilePanel.rs
git commit -m "feat(emStocksFilePanel): wire IsVFSGood via emFilePanel, wire Cycle (D28, T10)"
```

---

## Task 9: Wire emBorder and panel lifecycle for ItemChart (Phase 2, D29-D31)

**Files:**
- Modify: `crates/emstocks/src/emStocksItemChart.rs`

**Note:** This task makes `emStocksItemChart` implement `PanelBehavior` and use `emBorder` for content rect calculation. The existing `PaintContent` method becomes the body of the `Paint` trait method.

- [ ] **Step 1: Add panel/border imports**

```rust
use emcore::emPanel::{PanelBehavior, PanelState};
use emcore::emBorder::emBorder;
```

- [ ] **Step 2: Add border field to struct**

Add to `emStocksItemChart`:

```rust
pub(crate) border: emBorder,
```

Initialize in constructor with appropriate border type.

- [ ] **Step 3: Implement PanelBehavior**

```rust
impl PanelBehavior for emStocksItemChart {
    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        if !state.viewed {
            return;
        }
        // Use border's GetContentRect for layout
        let content = self.border.GetContentRect(w, h, /* look */);
        self.UpdateTransformationFromContentRect(content.x, content.y, content.w, content.h);
        self.PaintContent(painter, content.x, content.y, content.w, content.h, state);
    }
}
```

- [ ] **Step 4: Remove D29 DIVERGED comment** (line 34-35)

Remove: `// DIVERGED: No emBorder/emPanel inheritance.`

- [ ] **Step 5: Wire IsViewed check — remove D30** (line 216-218)

In `CalculateDaysPerPrice`, replace the DIVERGED comment and use `PanelState::viewed`:

The current method doesn't take state. This method is called from `UpdateData`, not from `Paint`. The `IsViewed()` check in C++ affects the `days_per_price` calculation — when viewed, it uses a power-of-2/256 division; when not viewed, it uses TotalDays. Since we now have panel state, thread the `viewed` flag through or store it on the struct.

```rust
pub(crate) fn CalculateDaysPerPrice(&self) -> i32 {
    if !self.viewed {
        return self.total_days;
    }
    // ... existing power-of-2/256 logic ...
}
```

Add `pub(crate) viewed: bool` field, set from `PanelState::viewed` in `Paint`.

- [ ] **Step 6: Wire GetContentRect — remove D31** (line 423-424)

`UpdateTransformation` currently uses unit rect `(0, 0, 1, 1)`. Replace with a method `UpdateTransformationFromContentRect` that takes the actual content rect from `emBorder::GetContentRect`.

- [ ] **Step 7: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/emstocks/src/emStocksItemChart.rs
git commit -m "feat(emStocksItemChart): implement PanelBehavior, wire emBorder (D29-D31)"
```

---

## Task 10: Eliminate PaintParams, wire view context (Phase 2, D32-D35, D38)

**Files:**
- Modify: `crates/emstocks/src/emStocksItemChart.rs`

**Note:** This is the largest single change in Phase 2. PaintParams gets replaced by actual panel view context queries. The panel state provides the transform data.

- [ ] **Step 1: Store view context values on struct**

Add fields to `emStocksItemChart`:

```rust
/// Cached view context values, updated each Paint call from PanelState.
pub(crate) pixels_per_unit_x: f64,
pub(crate) pixels_per_unit_y: f64,
pub(crate) clip_x1: f64,
pub(crate) clip_y1: f64,
pub(crate) clip_x2: f64,
pub(crate) clip_y2: f64,
pub(crate) max_label_height: f64,
```

In the `Paint` method, compute these from `PanelState`:

```rust
// Cache view context values from panel state
self.pixels_per_unit_x = state.clip_rect.w; // panel width in pixels
self.pixels_per_unit_y = state.clip_rect.h * state.pixel_tallness;
// Clip rect in panel coordinates
self.clip_x1 = state.clip_rect.x;
self.clip_y1 = state.clip_rect.y;
self.clip_x2 = state.clip_rect.x + state.clip_rect.w;
self.clip_y2 = state.clip_rect.y + state.clip_rect.h;
self.max_label_height = 0.032; // or compute from view
```

- [ ] **Step 2: Add view context helper methods**

Replace the PaintParams methods with methods on the struct:

```rust
impl emStocksItemChart {
    /// C++ ViewToPanelDeltaX: converts pixel distance to panel units.
    pub(crate) fn ViewToPanelDeltaX(&self, pixels: f64) -> f64 {
        pixels / self.pixels_per_unit_x
    }

    /// C++ ViewToPanelDeltaY: converts pixel distance to panel units.
    pub(crate) fn ViewToPanelDeltaY(&self, pixels: f64) -> f64 {
        pixels / self.pixels_per_unit_y
    }

    /// C++ PanelToViewDeltaY: converts panel units to pixel distance.
    pub(crate) fn PanelToViewDeltaY(&self, panel_dist: f64) -> f64 {
        panel_dist * self.pixels_per_unit_y
    }

    /// C++ GetClipX1 in panel coordinates.
    pub(crate) fn GetClipX1(&self) -> f64 { self.clip_x1 }
    pub(crate) fn GetClipY1(&self) -> f64 { self.clip_y1 }
    pub(crate) fn GetClipX2(&self) -> f64 { self.clip_x2 }
    pub(crate) fn GetClipY2(&self) -> f64 { self.clip_y2 }
    pub(crate) fn GetMaxLabelHeight(&self) -> f64 { self.max_label_height }
}
```

- [ ] **Step 3: Update PaintContent signature — remove D32**

Change `PaintContent` from taking `PaintParams` to taking `(painter, x, y, w, h, state)` matching C++:

```rust
pub fn PaintContent(
    &self,
    painter: &mut emPainter,
    x: f64, y: f64, w: f64, h: f64,
    state: &PanelState,
) {
    self.PaintXScaleLines(painter);
    self.PaintYScaleLines(painter);
    self.PaintXScaleLabels(painter);
    self.PaintYScaleLabels(painter);
    self.PaintPriceBar(painter);
    self.PaintDesiredPrice(painter);
    self.PaintGraph(painter);
}
```

Remove `params: &PaintParams` from all 7 sub-methods. They now use `self.ViewToPanelDeltaX(...)` etc.

- [ ] **Step 4: Update PaintXScaleLabels — remove D33** (line 661-662)

Replace `params.view_to_panel_delta_y(...)` with `self.ViewToPanelDeltaY(...)`.
Replace clip calculations with `self.GetClipY2()` etc.

- [ ] **Step 5: Update PaintYScaleLabels — remove D34** (line 868)

Replace `params.view_to_panel_delta_x(...)` with `self.ViewToPanelDeltaX(...)`.
Replace clip calculations with `self.GetClipX1()`.

- [ ] **Step 6: Update CalculateYScaleLevelRange — remove PaintParams half of D35**

Replace: `params.view_to_panel_delta_y(14.0)` with `self.ViewToPanelDeltaY(14.0)`.

Update the DIVERGED comment to only mention the tuple return (idiom I8):

```rust
/// DIVERGED: Returns (min_level, min_dist, max_level) tuple instead of C++ output
/// pointers — Rust has no out-parameters; tuples are the idiomatic equivalent.
```

- [ ] **Step 7: Delete PaintParams struct and impl — remove D38** (lines 1296-1334)

Remove the entire `PaintParams` struct, its `Default` impl, and its method impl block.

- [ ] **Step 8: Update all tests that use PaintParams**

Replace `PaintParams::default()` with direct calls using the struct's view context methods, or set the fields on the struct directly in tests.

- [ ] **Step 9: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 10: Commit**

```bash
git add crates/emstocks/src/emStocksItemChart.rs
git commit -m "feat(emStocksItemChart): eliminate PaintParams, wire view context methods (D32-D35, D38)"
```

---

## Task 11: Wire emListBox selection API in emStocksListBox (Phase 3, D1-D8)

**Files:**
- Modify: `crates/emstocks/src/emStocksListBox.rs`

**Note:** This replaces local `selected_indices`/`active_index` tracking with delegation to `emListBox`. The `emStocksListBox` struct wraps an `emListBox` instance.

- [ ] **Step 1: Add emListBox import and field**

```rust
use emcore::emListBox::emListBox;
```

Add to struct:

```rust
pub struct emStocksListBox {
    selected_date: String,
    pub visible_items: Vec<usize>,
    pub(crate) list_box: emListBox,
}
```

Remove `selected_indices: Vec<usize>` and `active_index: Option<usize>` fields.
Remove DIVERGED comments D1 (line 12-13), D2 (line 21-22), D3 (line 26).

- [ ] **Step 2: Update constructor**

```rust
pub fn new() -> Self {
    Self {
        selected_date: String::new(),
        visible_items: Vec::new(),
        list_box: emListBox::new(/* appropriate params */),
    }
}
```

- [ ] **Step 3: Delegate selection methods — remove D4-D8**

```rust
pub fn GetSelectionCount(&self) -> usize {
    self.list_box.GetSelectedIndices().len()
}

pub fn IsSelected(&self, visible_index: usize) -> bool {
    self.list_box.IsSelected(visible_index)
}

pub fn Select(&mut self, visible_index: usize) {
    self.list_box.Select(visible_index, false);
}

pub fn ClearSelection(&mut self) {
    self.list_box.ClearSelection();
}

pub fn SetSelectedIndex(&mut self, visible_index: usize) {
    self.list_box.Select(visible_index, true);
}
```

Remove DIVERGED comments D4-D8.

- [ ] **Step 4: Update all callers of the removed fields**

Search for `self.selected_indices` and `self.active_index` throughout the file and replace with `self.list_box` API calls.

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emstocks/src/emStocksListBox.rs
git commit -m "feat(emStocksListBox): delegate selection to emListBox (D1-D8)"
```

---

## Task 12: Wire PaintTextBoxed and clipboard/browser in ListBox (Phase 3, D11, D13, D15, D16, D19)

**Files:**
- Modify: `crates/emstocks/src/emStocksListBox.rs`

- [ ] **Step 1: Wire GetEmptyMessage to call PaintTextBoxed directly — remove D11**

Change `GetEmptyMessage` to take a painter and paint directly instead of returning text:

```rust
pub fn PaintEmptyMessage(&self, painter: &mut emPainter, w: f64, h: f64, bg_color: emColor) {
    if self.visible_items.is_empty() {
        let msg = "The stock list is empty.";
        painter.PaintTextBoxed(
            0.0, 0.0, w, h,
            msg,
            h * 0.1,
            emColor::rgb(255, 255, 255),
            bg_color,
            TextAlignment::Center,
            VAlign::Center,
            TextAlignment::Center,
            0.0,
            false,
            0.0,
        );
    }
}
```

Update `emStocksFilePanel::Paint` to call `list_box.PaintEmptyMessage(...)` instead of `list_box.GetEmptyMessage()`.

- [ ] **Step 2: Wire CopyStocks to use clipboard directly — remove D13**

Change `CopyStocks` to copy to system clipboard instead of returning string:

```rust
pub fn CopyStocks(&self, rec: &emStocksRec) {
    let text = self.SerializeSelectedStocks(rec);
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(&text);
    }
}
```

Keep the serialization logic in a separate `SerializeSelectedStocks` helper (extract from current `CopyStocks`).

- [ ] **Step 3: Wire CutStocks clipboard half — update D15**

```rust
pub fn CutStocks(&mut self, rec: &mut emStocksRec) {
    // DIVERGED(Phase 4): dialog ask parameter pending.
    let text = self.SerializeSelectedStocks(rec);
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(&text);
    }
    self.DeleteSelectedStocks(rec);
}
```

- [ ] **Step 4: Wire PasteStocks to read clipboard — update D16**

```rust
pub fn PasteStocks(&mut self, rec: &mut emStocksRec, config: &emStocksConfig) {
    // DIVERGED(Phase 4): dialog ask parameter pending.
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                self.DeserializeAndInsertStocks(rec, config, &text);
            }
        }
    }
}
```

- [ ] **Step 5: Wire ShowFirstWebPages to launch browser — remove D19**

```rust
pub fn ShowFirstWebPages(&self, rec: &emStocksRec) {
    for idx in &self.visible_items {
        if let Some(stock) = rec.stocks.get(*idx) {
            if let Some(url) = stock.web_pages.first() {
                if !url.is_empty() {
                    let _ = open::that(url);
                }
            }
        }
    }
}
```

Do the same for `ShowAllWebPages`.

- [ ] **Step 6: Update emStocksFilePanel callers**

Update `emStocksFilePanel::Input` to match the new signatures (methods no longer return values).

- [ ] **Step 7: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/emstocks/src/emStocksListBox.rs crates/emstocks/src/emStocksFilePanel.rs
git commit -m "feat(emStocksListBox): wire PaintTextBoxed, clipboard, browser (D11, D13, D15, D16, D19)"
```

---

## Task 13: Wire ControlPanel widget tree (Phase 3, D20-D24)

**Files:**
- Modify: `crates/emstocks/src/emStocksControlPanel.rs`

**Note:** This is the largest Phase 3 task. Replace data-model-only widget placeholders with real widget panel children. The exact widget construction depends on the emCore widget APIs (emButton, emTextField, emCheckBox, emScalarField, emRadioButton, emFileSelectionBox).

- [ ] **Step 1: Add widget imports**

```rust
use emcore::emButton::emButton;
use emcore::emTextField::emTextField;
use emcore::emCheckBox::emCheckBox;
use emcore::emScalarField::emScalarField;
use emcore::emRadioButton::{emRadioButton, RadioGroup};
use emcore::emFileSelectionBox::emFileSelectionBox;
```

- [ ] **Step 2: Replace FileFieldPanel data model with real emFileSelectionBox — remove D20**

Replace the `FileFieldPanel` struct with a wrapper around `emFileSelectionBox`:

```rust
pub(crate) struct FileFieldPanel {
    pub(crate) field_type: FileFieldType,
    pub(crate) widget: emFileSelectionBox,
}
```

Wire `UpdateControls` to read/write the widget's text value.

- [ ] **Step 3: Replace ControlCategoryPanel stub with real widget creation — remove D21**

Create actual list/filter widgets in the category panel instead of just storing sorted items.

- [ ] **Step 4: Replace ControlWidgets data fields with real widgets — remove D22**

Replace `Option<T>` data fields with widget references:

```rust
pub(crate) struct ControlWidgets {
    pub(crate) api_script: FileFieldPanel,
    pub(crate) api_script_interpreter: FileFieldPanel,
    pub(crate) api_key: emTextField,
    pub(crate) web_browser: FileFieldPanel,
    pub(crate) auto_update_dates: emCheckBox,
    pub(crate) triggering_opens_web_page: emCheckBox,
    pub(crate) chart_period: emScalarField,
    pub(crate) min_visible_interest: emRadioButton,
    // ... category panels ...
}
```

- [ ] **Step 5: Wire AutoExpand/AutoShrink to create/destroy widgets — remove D23**

Update `emStocksControlPanel`'s AutoExpand to instantiate the `ControlWidgets` with real widgets, and AutoShrink to drop them.

- [ ] **Step 6: Update UpdateControls to read/write widget values — remove D24**

Replace the method that takes explicit parameters with one that reads from owned widget references and the FileModel.

- [ ] **Step 7: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/emstocks/src/emStocksControlPanel.rs
git commit -m "feat(emStocksControlPanel): wire real widget tree (D20-D24)"
```

---

## Task 14: Wire ItemPanel widget tree (Phase 3, D39-D45)

**Files:**
- Modify: `crates/emstocks/src/emStocksItemPanel.rs`

- [ ] **Step 1: Add widget imports**

```rust
use emcore::emTextField::emTextField;
use emcore::emCheckBox::emCheckBox;
use emcore::emButton::emButton;
use emcore::emRadioButton::{emRadioButton, RadioGroup};
```

- [ ] **Step 2: Replace ItemWidgets data fields with real widgets — remove D39, D41, D43**

Replace the `ItemWidgets` struct that stores plain text values with one that holds real widget references:

```rust
pub(crate) struct ItemWidgets {
    pub(crate) name_label: emTextField,
    pub(crate) name: emTextField,
    pub(crate) symbol: emTextField,
    pub(crate) wkn: emTextField,
    pub(crate) isin: emTextField,
    pub(crate) owning_shares: emCheckBox,
    pub(crate) own_shares: emTextField,
    pub(crate) trade_price: emTextField,
    pub(crate) trade_date: emTextField,
    pub(crate) fetch_share_price: emButton,
    pub(crate) price: emTextField,
    pub(crate) price_date: emTextField,
    pub(crate) expected_dividend: emTextField,
    pub(crate) desired_price: emTextField,
    pub(crate) inquiry_date: emTextField,
    pub(crate) interest: emRadioButton,
    pub(crate) web_pages: [emTextField; 4],
    pub(crate) show_web_page: [emButton; 4],
    pub(crate) show_all_web_pages: emButton,
    pub(crate) comment: emTextField,
    pub(crate) trade_value: emTextField,
    pub(crate) current_value: emTextField,
    pub(crate) difference_value: emTextField,
}
```

Remove DIVERGED comments D39, D41, D43.

- [ ] **Step 3: Replace CategoryPanel stub with real widget creation — remove D40**

Wire CategoryPanel to create real child widgets.

- [ ] **Step 4: Update emStocksItemPanel struct — remove D42**

```rust
pub struct emStocksItemPanel {
    stock_rec_index: Option<usize>,
    pub(crate) update_controls_needed: bool,
    pub country: CategoryPanel,
    pub sector: CategoryPanel,
    pub collection: CategoryPanel,
    pub(crate) widgets: Option<ItemWidgets>,
    // ... previous values for toggle ...
}
```

Remove DIVERGED comment D42.

- [ ] **Step 5: Wire AutoExpand to create real widget children — remove D44**

```rust
fn AutoExpand(&mut self) {
    self.widgets = Some(ItemWidgets::new(/* params */));
    self.update_controls_needed = true;
}
```

Remove DIVERGED comment D44.

- [ ] **Step 6: Wire UpdateControls to use widget API — remove D45**

Replace the method that takes `stock` and `selected_date` parameters with one that reads from widgets and updates widget display values.

Note: D45 also requires Phase 4 (FileModel integration) for the parameter removal. For now, keep the parameters but wire the widget updates. The DIVERGED comment for "takes parameters" stays until Phase 4.

- [ ] **Step 7: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/emstocks/src/emStocksItemPanel.rs
git commit -m "feat(emStocksItemPanel): wire real widget tree (D39-D44)"
```

---

## Task 15: Integrate emStocksFileModel fully (Phase 4, D25-D27)

**Files:**
- Modify: `crates/emstocks/src/emStocksFileModel.rs`
- Modify: `crates/emstocks/src/emStocksFilePanel.rs`

- [ ] **Step 1: Replace placeholder with real dialog reference — remove D25**

In `emStocksFileModel.rs`, replace:

```rust
pub struct emStocksFetchPricesDialogPlaceholder;
```

with a reference to the real dialog type:

```rust
use super::emStocksFetchPricesDialog::emStocksFetchPricesDialog;
```

Update the field type in `emStocksFileModel`.

- [ ] **Step 2: Replace Instant with emTimer — update D26**

```rust
use emcore::emTimer::{TimerCentral, TimerId};
```

Replace `save_timer_deadline: Option<Instant>` with:

```rust
pub(crate) save_timer: Option<TimerId>,
pub(crate) timer_central: TimerCentral,
```

Update `OnRecChanged` to start the timer:

```rust
pub fn OnRecChanged(&mut self) {
    if self.save_timer.is_none() {
        let timer_id = self.timer_central.create_timer(/* signal */);
        self.timer_central.start_timer(timer_id, 15000, false);
        self.save_timer = Some(timer_id);
    }
}
```

Update the DIVERGED comment to the idiom I9 text:

```rust
/// DIVERGED: Composition instead of C++ multiple inheritance — Rust has no MI;
/// composition with delegation is the idiomatic equivalent.
```

- [ ] **Step 3: Wire FilePanel to own FileModel — remove D27**

In `emStocksFilePanel.rs`, replace `rec: emStocksRec` with `model: emStocksFileModel`:

```rust
pub struct emStocksFilePanel {
    pub(crate) bg_color: emColor,
    pub(crate) config: emStocksConfig,
    pub(crate) list_box: Option<emStocksListBox>,
    pub(crate) model: emStocksFileModel,
    pub(crate) file_panel: emFilePanel,
}
```

Update all `self.rec` references to `self.model.GetRec()` (read) or `self.model.GetWritableRec()` (write).

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emstocks/src/emStocksFileModel.rs crates/emstocks/src/emStocksFilePanel.rs
git commit -m "feat(emStocksFileModel): full integration with emTimer, wire FilePanel ownership (D25-D27)"
```

---

## Task 16: Collapse "takes rec parameter" divergences (Phase 4, D9-D10, D12, D14, D17-D18, D24, D45)

**Files:**
- Modify: `crates/emstocks/src/emStocksListBox.rs`
- Modify: `crates/emstocks/src/emStocksControlPanel.rs`
- Modify: `crates/emstocks/src/emStocksItemPanel.rs`

**Note:** Now that FilePanel owns FileModel, child components access rec via the model reference instead of taking it as a parameter.

- [ ] **Step 1: Give emStocksListBox a reference to the FileModel**

Add a model reference field (or accept it via method on the parent). The exact pattern depends on ownership — likely `emStocksListBox` gets a method `set_model_ref` or the FilePanel passes the model when calling methods.

The simplest approach matching C++ (where ListBox holds a pointer to the FileModel): store an `Rc<RefCell<emStocksFileModel>>` or have the parent pass `&emStocksFileModel` at call sites.

- [ ] **Step 2: Update ListBox methods to read from model — remove D9, D10, D12, D17**

```rust
pub fn GoBackInHistory(&mut self) {
    let rec = self.model.GetRec();
    // ... existing logic using rec ...
}

pub fn GoForwardInHistory(&mut self) {
    let rec = self.model.GetRec();
    // ... existing logic ...
}

pub fn NewStock(&mut self) {
    let rec = self.model.GetWritableRec();
    let config = self.config();
    // ... existing logic ...
}

pub fn DeleteSharePrices(&self) {
    // ... read from model ...
}
```

Remove DIVERGED comments D9, D10, D12, D17.

- [ ] **Step 3: Update ControlPanel.UpdateControls — remove D24**

```rust
pub fn UpdateControls(&mut self) {
    let config = self.model.config();
    let rec = self.model.GetRec();
    let list_box = self.list_box();
    // ... existing logic ...
}
```

- [ ] **Step 4: Update ItemPanel.UpdateControls — remove D45**

```rust
pub fn UpdateControls(&mut self) {
    let stock = self.model.GetRec().stocks.get(self.stock_rec_index.unwrap());
    let selected_date = self.list_box().GetSelectedDate();
    // ... existing logic ...
}
```

- [ ] **Step 5: Update FilePanel callers**

Update all call sites in `emStocksFilePanel::Input` that previously passed `&self.rec` or `&mut self.rec` to use the new parameterless signatures.

- [ ] **Step 6: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/emstocks/src/emStocksListBox.rs crates/emstocks/src/emStocksControlPanel.rs crates/emstocks/src/emStocksItemPanel.rs crates/emstocks/src/emStocksFilePanel.rs
git commit -m "feat(emStocks): collapse 'takes rec parameter' divergences (D9-D10, D12, D17, D24, D45)"
```

---

## Task 17: Wire dialog confirmation flow (Phase 4, D14, D15, D16, D18)

**Files:**
- Modify: `crates/emstocks/src/emStocksListBox.rs`

- [ ] **Step 1: Add emDialog import**

```rust
use emcore::emDialog::{emDialog, DialogResult};
```

- [ ] **Step 2: Add `ask` parameter to DeleteStocks — remove D14**

```rust
pub fn DeleteStocks(&mut self, ask: bool) {
    if ask {
        // Create confirmation dialog
        // C++: "Really delete N stock(s)?"
        let count = self.GetSelectionCount();
        if count == 0 { return; }
        let msg = format!("Really delete {} stock(s)?", count);
        // Create and show dialog, wire on_finish callback to perform deletion
        let mut dialog = emDialog::new(&msg, self.look.clone());
        dialog.AddCustomButton("Delete", DialogResult::Ok);
        dialog.AddCustomButton("Cancel", DialogResult::Cancel);
        dialog.on_finish = Some(Box::new(move |result| {
            if *result == DialogResult::Ok {
                // Perform deletion via callback
            }
        }));
        // Store dialog reference for lifecycle management
        return;
    }
    // Direct deletion (no dialog)
    self.DeleteSelectedStocks();
}
```

- [ ] **Step 3: Add `ask` parameter to CutStocks — remove D15**

Similar pattern: if `ask`, show confirmation first, then cut on Ok.

- [ ] **Step 4: Add `ask` parameter to PasteStocks — remove D16**

Similar pattern: if `ask`, show confirmation first, then paste on Ok.

- [ ] **Step 5: Add `ask` parameter to SetInterest — remove D18**

Similar pattern: if `ask` and selection count > threshold, show confirmation.

- [ ] **Step 6: Update FilePanel callers to pass `ask: true`**

```rust
InputKey::Delete => {
    let list_box = self.list_box.as_mut().unwrap();
    list_box.DeleteStocks(true);
    return true;
}
```

- [ ] **Step 7: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/emstocks/src/emStocksListBox.rs crates/emstocks/src/emStocksFilePanel.rs
git commit -m "feat(emStocksListBox): wire dialog confirmation for destructive operations (D14-D16, D18)"
```

---

## Task 18: Wire PricesFetcher FileModel integration (Phase 4, D-hdr2, D46, D47)

**Files:**
- Modify: `crates/emstocks/src/emStocksPricesFetcher.rs`

- [ ] **Step 1: Wire emEngine Cycle to use FileModel — remove D-hdr2**

Update the `emEngine` trait impl to access rec through the FileModel:

```rust
impl emEngine for emStocksPricesFetcher {
    fn Cycle(&mut self, _ctx: &mut EngineCtx<'_>) -> bool {
        let model = self.model.as_ref().expect("FileModel must be set");
        let file_state = model.borrow().GetFileState();
        if !file_state.is_loaded_or_unsaved() {
            return false;
        }
        let rec = &mut model.borrow_mut().GetWritableRec();
        // ... existing Cycle logic ...
        self.current_process_active
    }
}
```

Remove the Phase 4 DIVERGED comment from the header.

- [ ] **Step 2: Wire ListBox date-selection update in AddPrice — remove D46**

```rust
fn AddPrice(&mut self, date: &str, price: &str, rec: &mut emStocksRec) {
    let idx = match self.GetCurrentStockRecIndex(rec) {
        Some(i) => i,
        None => return,
    };

    // Wire date-selection update through ListBox
    if let Some(list_box) = &mut self.list_box {
        let stock = &rec.stocks[idx];
        if CompareDates(date, &stock.last_price_date) > 0 {
            list_box.SetSelectedDate(date);
        }
    }

    rec.stocks[idx].AddPrice(date, price);
    self.current_stock_updated = true;
}
```

- [ ] **Step 3: Add file-state guard in Cycle — remove D47**

In the standalone `Cycle` method (which will be internal once emEngine is fully wired):

```rust
pub fn Cycle(&mut self, rec: &mut emStocksRec) -> bool {
    // File-state guard: only fetch when model is in loaded/unsaved state
    // (This check moves to emEngine::Cycle in the trait impl)
    if self.current_process_active {
        self.PollProcess(rec);
    }
    if !self.current_process_active {
        self.StartProcess(rec);
    }
    self.current_process_active
}
```

- [ ] **Step 4: Update header comments**

Replace lines 2-6 with:

```rust
// DIVERGED: Uses BTreeMap<String, Option<usize>> instead of C++ emAvlTreeMap<String,
// emCrossPtr<StockRec>> — BTreeMap is Rust's idiomatic ordered map; cross-pointers
// don't apply when StockRecs live in a Vec.
```

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emstocks/src/emStocksPricesFetcher.rs
git commit -m "feat(emStocksPricesFetcher): wire FileModel integration (D-hdr2, D46, D47)"
```

---

## Task 19: Wire fetch dialog and ListBox as panel child (Phase 4, T7, T9)

**Files:**
- Modify: `crates/emstocks/src/emStocksFilePanel.rs`
- Modify: `crates/emstocks/src/emStocksFetchPricesDialog.rs`

- [ ] **Step 1: Create real emStocksFetchPricesDialog — remove T7**

The `emStocksFetchPricesDialog` already has a file. Wire it as a real `emDialog` child panel:

```rust
impl emStocksFetchPricesDialog {
    pub fn new(stock_ids: Vec<String>, config: &emStocksConfig, look: Rc<emLook>) -> Self {
        let mut dialog = emDialog::new("Fetch Share Prices", look);
        dialog.AddCustomButton("Start", DialogResult::Ok);
        dialog.AddCustomButton("Cancel", DialogResult::Cancel);
        Self {
            dialog,
            stock_ids,
            fetcher: None,
        }
    }
}
```

In `emStocksFilePanel::Input`, replace the Ctrl+P TODO:

```rust
InputKey::Key('P') => {
    let ids = list_box.GetVisibleStockIds();
    self.fetch_dialog = Some(emStocksFetchPricesDialog::new(
        ids,
        &self.config,
        self.look.clone(),
    ));
    return true;
}
```

- [ ] **Step 2: Wire ListBox as real panel child — remove T9**

In `LayoutChildren`:

```rust
fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
    if let Some(ref mut list_box) = self.list_box {
        // C++: ListBox->Layout(0.0, 0.0, 1.0, GetHeight(), BgColor);
        ctx.layout_child(list_box, 0.0, 0.0, 1.0, ctx.height());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr -p emstocks`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emstocks/src/emStocksFilePanel.rs crates/emstocks/src/emStocksFetchPricesDialog.rs
git commit -m "feat(emStocksFilePanel): wire fetch dialog and ListBox as panel child (T7, T9)"
```

---

## Task 20: Update idiom DIVERGED comments (Final Pass, I1-I9)

**Files:**
- Modify: `crates/emstocks/src/emStocksRec.rs`
- Modify: `crates/emstocks/src/emStocksPricesFetcher.rs`
- Modify: `crates/emstocks/src/emStocksItemChart.rs`
- Modify: `crates/emstocks/src/emStocksFileModel.rs`

**No code changes — comment text only.**

- [ ] **Step 1: Update I1 in emStocksRec.rs (line 13-14)**

Replace:
```rust
/// DIVERGED: Rust enum replaces C++ int enum + emEnumRec subclass.
```
With:
```rust
/// DIVERGED: Rust enum replaces C++ int enum + emEnumRec subclass — Rust enums are
/// the idiomatic equivalent of C++ int enums with associated string tables.
```

- [ ] **Step 2: Update I2 in emStocksRec.rs (line 231)**

Replace:
```rust
/// DIVERGED: Returns (days, dates_valid) tuple instead of C++ bool* out-param.
```
With:
```rust
/// DIVERGED: Returns (i32, bool) tuple instead of C++ bool* out-param — Rust has no
/// out-parameters; tuples are the idiomatic equivalent.
```

- [ ] **Step 3: Update I3 in emStocksRec.rs (line 289)**

Replace:
```rust
/// DIVERGED: Rust struct fields use snake_case. Method names preserve C++ names.
```
With:
```rust
/// DIVERGED: Rust struct fields use snake_case — required by Rust naming conventions
/// (clippy::non_snake_case). Method names preserve C++ names per File and Name Correspondence.
```

- [ ] **Step 4: Update I4 in emStocksRec.rs (line 630)**

Replace:
```rust
/// DIVERGED: Returns Option<f64> instead of bool + *pResult.
```
With:
```rust
/// DIVERGED: Returns Option<f64> instead of C++ bool + *pResult — Option is Rust's
/// idiomatic replacement for success bool + out-pointer.
```

- [ ] **Step 5: Update I5 in emStocksRec.rs (line 927)**

Replace:
```rust
/// DIVERGED: Returns Option<usize> instead of -1.
```
With:
```rust
/// DIVERGED: Returns Option<usize> instead of C++ -1 sentinel — Option<usize> is
/// Rust's idiomatic replacement for signed-int sentinel values.
```

- [ ] **Step 6: Update I6 in emStocksRec.rs (line 936-937)**

Same text as I5.

- [ ] **Step 7: Update I7 in emStocksPricesFetcher.rs (line 4-5)**

Replace:
```rust
// DIVERGED: Uses BTreeMap<String, Option<usize>> mapping stock ID to index in emStocksRec.stocks,
// instead of emAvlTreeMap<String, emCrossPtr<StockRec>>. The cross-pointer approach
// doesn't work well when StockRecs are stored in a Vec.
```
With:
```rust
// DIVERGED: Uses BTreeMap<String, Option<usize>> instead of C++ emAvlTreeMap<String,
// emCrossPtr<StockRec>> — BTreeMap is Rust's idiomatic ordered map; cross-pointers
// don't apply when StockRecs live in a Vec.
```

- [ ] **Step 8: Update I8 in emStocksItemChart.rs (line 920-922)**

This should already be updated to the tuple-only DIVERGED from Task 10. Verify it reads:

```rust
/// DIVERGED: Returns (min_level, min_dist, max_level) tuple instead of C++ output
/// pointers — Rust has no out-parameters; tuples are the idiomatic equivalent.
```

- [ ] **Step 9: Verify I9 in emStocksFileModel.rs**

Should already be updated from Task 15. Verify it reads:

```rust
/// DIVERGED: Composition instead of C++ multiple inheritance — Rust has no MI;
/// composition with delegation is the idiomatic equivalent.
```

- [ ] **Step 10: Run final gate**

Run: `cargo check && cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: ALL PASS

- [ ] **Step 11: Commit**

```bash
git add crates/emstocks/src/emStocksRec.rs crates/emstocks/src/emStocksPricesFetcher.rs crates/emstocks/src/emStocksItemChart.rs crates/emstocks/src/emStocksFileModel.rs
git commit -m "docs(emStocks): update all 9 idiom DIVERGED comments with explicit justifications (I1-I9)"
```

---

## Task 21: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Verify zero TODOs remain in emStocks**

Run: `grep -r "TODO" crates/emstocks/src/ --include="*.rs"`
Expected: No output (zero TODOs)

- [ ] **Step 2: Verify DIVERGED count matches expected**

Run: `grep -c "DIVERGED" crates/emstocks/src/*.rs | awk -F: '{s+=$2} END {print s}'`
Expected: Approximately 10-12 occurrences (9 idiom DIVERGED + the PaintGraph API limitation DIVERGED from Task 6)

- [ ] **Step 3: Run full test suite**

Run: `cargo check && cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: ALL PASS

- [ ] **Step 4: Run golden tests**

Run: `cargo test --test golden -- --test-threads=1`
Expected: PASS (emStocks changes should not affect golden tests which are emCore rendering tests)
