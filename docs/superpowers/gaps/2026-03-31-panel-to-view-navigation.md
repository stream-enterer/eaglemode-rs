# Gap: Panel-to-View Navigation Channel

**Filed:** 2026-03-31
**Affects:** emDirPanel scroll-to-entry, any panel that needs to trigger view navigation
**Severity:** Medium — feature works except for the final scroll/navigate step

## Problem

Panels cannot trigger view navigation. In C++, panels hold a pointer to their owning `emView` and call `View.Visit(child_panel)` directly. In Rust, `PanelBehavior` methods receive `PanelCtx` (wrapping `&mut PanelTree`) but NOT the `emView`. There is no communication channel from panel → view.

## Where This Surfaces

1. **emDirPanel key_walk** (`crates/emfileman/src/emDirPanel.rs`): User types characters to search entries. The matching entry is found and `scroll_target` is stored. After children are created, `find_child_by_name()` locates the child `PanelId`. But there's no way to tell the view to navigate to it. Currently logs a debug message.

2. **Any future panel** that needs to programmatically navigate (e.g., "go to item", search-and-scroll, auto-expand).

## What Exists

- `emView` has navigation: `Visit()`, `VisitFullsized()`, `SetSeekPos()`, `VisitNext/Prev/In/Out()`
- `PanelTree` has `find_child_by_name()`, child enumeration, parent field (`pub(crate)`)
- `PanelCtx` wraps `&mut PanelTree` — no view access

## Proposed Fix

Add a navigation request queue to `PanelTree` that the `emView` drains each frame.

### In `crates/emcore/src/emPanelTree.rs`:

```rust
// Add to PanelTree struct:
    navigation_requests: Vec<PanelId>,

// Add methods:
    /// Request that the view navigate to show this panel.
    /// Called by panel behaviors; drained by emView each frame.
    pub fn request_visit(&mut self, target: PanelId) {
        self.navigation_requests.push(target);
    }

    /// Drain pending navigation requests. Called by emView::Update.
    pub fn drain_navigation_requests(&mut self) -> Vec<PanelId> {
        std::mem::take(&mut self.navigation_requests)
    }
```

### In `crates/emcore/src/emPanelCtx.rs`:

```rust
    /// Request view navigation to a child panel.
    pub fn request_visit(&mut self, child: PanelId) {
        self.tree.request_visit(child);
    }
```

### In `crates/emcore/src/emView.rs` (in `Update` or equivalent frame method):

```rust
    // After processing panels, drain navigation requests
    for target in tree.drain_navigation_requests() {
        self.VisitFullsized(tree, target);
    }
```

### Then in `crates/emfileman/src/emDirPanel.rs` (update_children):

Replace the log::debug with:
```rust
if let Some(target) = self.scroll_target.take() {
    if let Some(child_id) = ctx.find_child_by_name(&target) {
        ctx.request_visit(child_id);
    }
}
```

## Estimated Scope

~20 lines across 3 files in emcore, plus changes in emDirPanel. No architectural changes needed.

## Changes Required in This Plan's Implementation Once Gap Is Filled

These are specific locations in code committed by phase 3 that contain placeholder
logic gated on this gap. Search for the markers to find current line numbers.

### 1. `crates/emfileman/src/emDirPanel.rs` — update_children scroll target block

**Search for:** `scroll_to_entry: found child`

Current code (placeholder):
```rust
if let Some(target) = self.scroll_target.take() {
    if let Some(_child_id) = ctx.find_child_by_name(&target) {
        log::debug!(
            "scroll_to_entry: found child '{}', navigation pending",
            target
        );
    }
}
```

Replace with:
```rust
if let Some(target) = self.scroll_target.take() {
    if let Some(child_id) = ctx.find_child_by_name(&target) {
        ctx.request_visit(child_id);
    }
}
```

### 2. `crates/emfileman/src/emDirPanel.rs` — DIVERGED comment above scroll target block

**Search for:** `cannot trigger view navigation without view access`

Update the DIVERGED comment to remove the "cannot trigger" caveat once the gap is closed.

## New Code Required (emcore infrastructure)

### 3. `crates/emcore/src/emPanelTree.rs` — PanelTree struct

Add `navigation_requests: Vec<PanelId>` field and `request_visit()` / `drain_navigation_requests()` methods (see Proposed Fix above).

### 4. `crates/emcore/src/emPanelCtx.rs` — PanelCtx

Add `request_visit(&mut self, child: PanelId)` that delegates to `self.tree.request_visit(child)`.

### 5. `crates/emcore/src/emView.rs` — Update loop

Drain `tree.drain_navigation_requests()` and call `self.VisitFullsized(tree, target)` for each.

## Related Code Pointers

- `PanelTree` struct: `crates/emcore/src/emPanelTree.rs` (search for `pub struct PanelTree`)
- `PanelCtx`: `crates/emcore/src/emPanelCtx.rs`
- `emView::Update`: `crates/emcore/src/emView.rs` (search for `pub fn Update`)
- `emDirPanel::update_children`: `crates/emfileman/src/emDirPanel.rs` (search for `scroll_target.take()`)
- `emDirPanel::key_walk`: `crates/emfileman/src/emDirPanel.rs` (search for `scroll_target = Some`)
- `emView::VisitFullsized`: `crates/emcore/src/emView.rs` (search for `fn VisitFullsized`)
