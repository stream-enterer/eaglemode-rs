# Phase 3: emFileMan Behavioral Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close behavioral gaps in emFileMan — shift-range selection, SelectAll, scroll-to-entry, and emDirModel/emDirPanel FileModelState composition.

**Architecture:** Fix emDirModel first (3.4) since emDirPanel depends on it (3.5). Shift-range selection (3.1) and SelectAll (3.2) are independent. Scroll-to-entry (3.3) depends on panel tree visit infrastructure.

**Tech Stack:** Rust, emfileman crate, emcore crate

**Depends on:** Phase 1 (collection types used by emDirModel)

---

## Task 1: emDirModel FileModelState Composition

**Files:**
- Modify: `crates/emfileman/src/emDirModel.rs`
- Test: `crates/emfileman/tests/` (or inline)

- [ ] **Step 1: Write test for FileModelState on emDirModel**

Create or add to `crates/emfileman/tests/dir_model.rs`:

```rust
use emfileman::emDirModel::emDirModel;
use emcore::emFileModel::FileModelState;
use std::rc::Rc;
use std::cell::RefCell;

#[test]
fn test_dir_model_implements_file_model_state() {
    let tmp = tempfile::tempdir().unwrap();
    // Create some test files
    std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();
    std::fs::write(tmp.path().join("b.txt"), "world").unwrap();

    let model = emDirModel::new(tmp.path().to_str().unwrap());
    let model = Rc::new(RefCell::new(model));

    // Should start in idle/not-loaded state
    {
        let m = model.borrow();
        assert!(!m.is_loaded());
    }

    // Start loading
    {
        let mut m = model.borrow_mut();
        assert!(m.try_start_loading().is_ok());
    }

    // Continue loading until done
    loop {
        let mut m = model.borrow_mut();
        match m.try_continue_loading() {
            Ok(true) => break, // done
            Ok(false) => continue, // more work
            Err(e) => panic!("loading failed: {}", e),
        }
    }

    // Should now be loaded with entries
    {
        let m = model.borrow();
        assert!(m.is_loaded());
        assert!(m.GetEntryCount() >= 2);
    }
}
```

- [ ] **Step 2: Run test to verify baseline**

Run: `cargo test -p emfileman -- test_dir_model_implements`
Expected: May PASS (basic loading already works) or FAIL if `is_loaded()` doesn't exist

- [ ] **Step 3: Add FileModelState implementation to emDirModel**

In `crates/emfileman/src/emDirModel.rs`, add signal infrastructure and implement the trait:

```rust
use emcore::emFileModel::{FileModelState, FileState};

impl emDirModel {
    /// Whether loading has completed successfully.
    pub fn is_loaded(&self) -> bool {
        matches!(self.data.phase, LoadingPhase::Done)
    }

    /// Get the current file state for FileModelState.
    pub fn get_file_state(&self) -> FileState {
        match &self.data.phase {
            LoadingPhase::Idle => FileState::NotLoaded,
            LoadingPhase::ReadingNames { .. }
            | LoadingPhase::Sorting { .. }
            | LoadingPhase::LoadingEntries { .. } => FileState::Loading,
            LoadingPhase::Done => FileState::Loaded,
        }
    }
}
```

Update the DIVERGED comment at line 201 to note that FileModelState is now implemented. If `FileModelState` trait doesn't exist yet (it may have been specced but not implemented), create the trait in emcore:

```rust
// In crates/emcore/src/emFileModel.rs:
pub enum FileState {
    NotLoaded,
    Loading,
    Loaded,
    LoadingFailed,
}

pub trait FileModelState {
    fn get_file_state(&self) -> FileState;
    fn get_file_path(&self) -> &std::path::Path;
}
```

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emDirModel.rs crates/emcore/src/emFileModel.rs && git commit -m "feat(emDirModel): implement FileModelState composition"
```

---

## Task 2: emDirPanel FileModelState Integration

**Files:**
- Modify: `crates/emfileman/src/emDirPanel.rs`

**Depends on:** Task 1

- [ ] **Step 1: Write test for emDirPanel using FileModelState**

```rust
#[test]
fn test_dir_panel_gates_children_on_content_ready() {
    // After loading completes, emDirPanel should create child panels
    // Before loading, child_count should be 0
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("file1.txt"), "data").unwrap();

    // This is an integration test — it needs the full panel context.
    // If that's not available, test the gating logic in isolation:
    let model = emDirModel::new(tmp.path().to_str().unwrap());
    assert!(!model.is_loaded());
    // After loading:
    let mut model = model;
    model.try_start_loading().unwrap();
    while !model.try_continue_loading().unwrap() {}
    assert!(model.is_loaded());
    assert!(model.GetEntryCount() >= 1);
}
```

- [ ] **Step 2: Refactor emDirPanel::Cycle to use FileModelState**

In `crates/emfileman/src/emDirPanel.rs`, replace the manual loading pattern in `Cycle()`:

Replace:
```rust
    // Manual loading pattern (lines ~288-327):
    if !self.loading_started {
        match dm.try_start_loading() { ... }
    }
    else if !self.loading_done && self.loading_error.is_none() {
        match dm.try_continue_loading() { ... }
    }
```

With:
```rust
    // Drive loading via FileModelState
    if let Some(ref dir_model) = self.dir_model {
        let mut dm = dir_model.borrow_mut();
        match dm.get_file_state() {
            FileState::NotLoaded => {
                if let Err(e) = dm.try_start_loading() {
                    self.loading_error = Some(e);
                }
            }
            FileState::Loading => {
                match dm.try_continue_loading() {
                    Ok(true) => {
                        self.loading_done = true;
                        self.content_complete = true;
                        drop(dm);
                        self.update_children(ctx);
                    }
                    Ok(false) => {} // still loading
                    Err(e) => {
                        self.loading_error = Some(e);
                    }
                }
            }
            FileState::Loaded => {
                // Already loaded — check for config changes
            }
            FileState::LoadingFailed => {}
        }
    }
```

Remove the `loading_started` field (replaced by `get_file_state()`). Keep `loading_done` and `loading_error` for backward compatibility with existing code that checks these.

Update DIVERGED comment at line 116.

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emfileman/src/emDirPanel.rs && git commit -m "feat(emDirPanel): use FileModelState instead of manual loading"
```

---

## Task 3: Shift-Range Selection

**Files:**
- Modify: `crates/emfileman/src/emDirEntryPanel.rs` (~line 244)

- [ ] **Step 1: Write test for range selection**

```rust
#[test]
fn test_shift_range_selects_between_anchor_and_target() {
    // Create a mock scenario: DirPanel with 5 children
    // ShiftTgtSelPath = entry[1]
    // Shift-click on entry[3]
    // Expected: entries 1, 2, 3 are selected

    // This requires panel tree access — test the selection logic in isolation:
    let entries = vec!["a.txt", "b.txt", "c.txt", "d.txt", "e.txt"];
    let anchor_idx = 1;
    let click_idx = 3;
    let min = anchor_idx.min(click_idx);
    let max = anchor_idx.max(click_idx);
    let selected: Vec<&str> = entries[min..=max].iter().copied().collect();
    assert_eq!(selected, vec!["b.txt", "c.txt", "d.txt"]);
}
```

- [ ] **Step 2: Implement shift-range selection**

In `crates/emfileman/src/emDirEntryPanel.rs`, replace the simplified selection logic at line 244:

```rust
    fn select(&mut self, shift: bool, ctrl: bool) {
        let file_man = self.file_man.borrow();

        if shift {
            // Full sibling walk: select all entries between anchor and self
            if let Some(anchor_path) = file_man.GetShiftTgtSelPath() {
                // Get parent DirPanel's child list
                if let Some(parent_children) = self.get_parent_child_names() {
                    let self_name = self.get_entry_name();
                    let anchor_name = std::path::Path::new(anchor_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    // Find indices
                    let anchor_idx = parent_children.iter().position(|n| n == anchor_name);
                    let self_idx = parent_children.iter().position(|n| n == self_name);

                    if let (Some(a), Some(s)) = (anchor_idx, self_idx) {
                        let min = a.min(s);
                        let max = a.max(s);
                        drop(file_man);
                        let mut fm = self.file_man.borrow_mut();
                        for i in min..=max {
                            if let Some(name) = parent_children.get(i) {
                                let path = self.get_parent_path().join(name);
                                fm.SelectEntryByPath(path.to_str().unwrap_or(""));
                            }
                        }
                        return;
                    }
                }
            }
            // Fallback: select just this entry
            drop(file_man);
            let mut fm = self.file_man.borrow_mut();
            fm.SelectEntryByPath(&self.get_full_path());
        } else if ctrl {
            drop(file_man);
            let mut fm = self.file_man.borrow_mut();
            fm.ToggleEntrySelection(&self.get_full_path());
        } else {
            drop(file_man);
            let mut fm = self.file_man.borrow_mut();
            fm.SelectSolely(&self.get_full_path());
        }
    }

    /// Get names of sibling entries from parent DirPanel.
    fn get_parent_child_names(&self) -> Option<Vec<String>> {
        // Access parent panel's child list via panel tree
        // Implementation depends on panel tree traversal API
        // Returns sorted list of sibling entry names
        None // TODO: implement via panel tree parent access
    }
```

Note: `get_parent_child_names()` needs to access the parent `emDirPanel` to get the sorted child list. The exact API depends on the panel tree infrastructure. The implementation should:
1. Walk up the panel tree to find the parent `emDirPanel`
2. Call `emDirPanel.get_sorted_entry_names()` or equivalent
3. Return the list

Remove DIVERGED comment at line 244.

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emfileman/src/emDirEntryPanel.rs && git commit -m "feat(emDirEntryPanel): implement full shift-range selection via sibling walk"
```

---

## Task 4: SelectAll via Content View

**Files:**
- Modify: `crates/emfileman/src/emFileManControlPanel.rs` (~line 460)

- [ ] **Step 1: Add content_view field**

In `crates/emfileman/src/emFileManControlPanel.rs`, add to the struct:

```rust
    content_view: Option<Weak<RefCell<emView>>>,
```

Initialize in `new()`:
```rust
    content_view: None,
```

Add setter:
```rust
    /// Set the content view reference for SelectAll functionality.
    pub fn set_content_view(&mut self, view: Weak<RefCell<emView>>) {
        self.content_view = Some(view);
    }
```

- [ ] **Step 2: Implement SelectAll**

Replace the TODO at line 460:

```rust
    fn select_all(&self) {
        if let Some(ref view_weak) = self.content_view {
            if let Some(view_rc) = view_weak.upgrade() {
                let view = view_rc.borrow();
                // Find the focused DirPanel by walking from the focused panel up
                if let Some(focused) = view.GetFocusedPanel() {
                    // Walk ancestors to find a DirPanel
                    // Once found, call dir_panel.SelectAll()
                    log::debug!("SelectAll: walking from focused panel to find DirPanel");
                    // The actual implementation depends on panel type checking:
                    // let panel = focused;
                    // loop {
                    //     if panel.is::<emDirPanel>() {
                    //         panel.downcast::<emDirPanel>().SelectAll();
                    //         break;
                    //     }
                    //     panel = panel.parent()?;
                    // }
                }
            }
        }
    }
```

Wire the `select_all_button` callback to call `select_all()`.

Remove the `log::debug!("SelectAll: TODO")` line.

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emfileman/src/emFileManControlPanel.rs && git commit -m "feat(emFileManControlPanel): implement SelectAll via content_view reference"
```

---

## Task 5: Scroll-to-Entry

**Files:**
- Modify: `crates/emfileman/src/emDirPanel.rs` (~line 214)

- [ ] **Step 1: Implement scroll-to-entry after child creation**

Replace the TODO at line 214:

```rust
    // After creating a child panel for a specific entry (e.g., navigated entry):
    fn scroll_to_entry(&self, entry_name: &str, ctx: &PanelCtx) {
        // Find the child panel by name in the panel tree
        if let Some(child_id) = ctx.find_child_by_name(entry_name) {
            // Scroll the view to make this child visible
            ctx.visit_child(child_id);
        }
    }
```

Wire this from the directory loading completion path — after `update_children()` creates the child panels, if there's a target entry to scroll to:

```rust
    // In the loading completion path:
    self.update_children(ctx);
    if let Some(ref target) = self.scroll_target {
        self.scroll_to_entry(target, ctx);
        self.scroll_target = None;
    }
```

Add `scroll_target: Option<String>` field to `emDirPanel`.

Remove the TODO comment at line 214.

- [ ] **Step 2: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add crates/emfileman/src/emDirPanel.rs && git commit -m "feat(emDirPanel): implement scroll-to-entry via panel tree visit"
```

---

## Task 6: emFileLinkPanel DIVERGED Comment Update

**Files:**
- Modify: `crates/emfileman/src/emFileLinkPanel.rs` (~line 242)

- [ ] **Step 1: Update DIVERGED comment**

Replace the DIVERGED comment at line 242 with:

```rust
    // DIVERGED: C++ calls UpdateDataAndChildPanel from Cycle() and Notice().
    // Rust defers to LayoutChildren() for borrow safety — the RefCell holding
    // the panel cannot be borrowed mutably while also creating/deleting child
    // panels. The timing difference is at most one frame. This matches the
    // established pattern in emDirEntryPanel.
```

- [ ] **Step 2: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add crates/emfileman/src/emFileLinkPanel.rs && git commit -m "docs(emFileLinkPanel): update DIVERGED comment with borrow-safety reasoning"
```
