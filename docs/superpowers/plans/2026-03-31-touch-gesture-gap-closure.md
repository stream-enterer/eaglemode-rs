# Touch Gesture Gap Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close all four gaps in the 18-state touch gesture machine so that hold-to-zoom, tap sequences, two-finger mouse emulation, three-finger menu key, and four-finger soft keyboard all work end-to-end.

**Architecture:** Gap 1 adds `emDefaultTouchVIF` to `ZuiWindow` and routes winit `Touch` events + per-frame `cycle_gesture` calls. Gap 2 adds `ForwardInput` infrastructure as a method on `emDefaultTouchVIF` that dispatches synthetic events through `ZuiWindow::dispatch_input`. Gap 3 wires `InjectMenuKey` through ForwardInput and adds soft keyboard API stubs to `emView`. Gap 5 resolves the dual state machine by gating the old 4-state system behind the gesture machine's state, keeping only Fling.

**Tech Stack:** Rust, emcore crate, winit 0.30 (`WindowEvent::Touch`)

**Depends on:** Phase 2 behavioral plan (Tasks 1-2 already completed — `do_gesture`, `run_gesture_loop`, `drain_gesture_actions`, gesture tracker sync all exist)

---

## File Map

| File | Role | Tasks |
|------|------|-------|
| `crates/emcore/src/emWindow.rs` | Window struct, event dispatch, VIF chain | 1, 2 |
| `crates/emcore/src/emGUIFramework.rs` | Event loop, `window_event`, `about_to_wait` | 1 |
| `crates/emcore/src/emViewInputFilter.rs` | `emDefaultTouchVIF`, `drain_gesture_actions`, `emViewInputFilter` trait | 2, 3, 4, 5 |
| `crates/emcore/src/emView.rs` | `emView` struct — soft keyboard API | 3 |
| `crates/eaglemode/tests/support/pipeline.rs` | Test harness VIF chain | 1 |
| `crates/eaglemode/tests/support/mod.rs` | Test harness VIF chain | 1 |

---

## Task 1: Instantiate emDefaultTouchVIF in Window and Route Touch Events (Gap 1)

**Files:**
- Modify: `crates/emcore/src/emWindow.rs` (struct fields, `create`, `dispatch_input`, `tick_vif_animations`)
- Modify: `crates/emcore/src/emGUIFramework.rs` (lines 155-234, `window_event`)
- Modify: `crates/eaglemode/tests/support/pipeline.rs` (line 42-51)
- Modify: `crates/eaglemode/tests/support/mod.rs` (line 43-52)

### Context

`emDefaultTouchVIF` is currently defined in `emViewInputFilter.rs` but never instantiated in the window. The window's `ZuiWindow` struct has a `vif_chain: Vec<Box<dyn emViewInputFilter>>` containing `emMouseZoomScrollVIF` and `emKeyboardZoomScrollVIF`. Touch events are **not** routed through the VIF chain — `emDefaultTouchVIF` has its own `touch_start`/`touch_move`/`touch_end` methods that need direct calls from winit `WindowEvent::Touch` events.

The C++ `emDefaultTouchVIF::Cycle()` calls `NextTouches()` then `DoGesture()` each frame. The Rust equivalent is `cycle_gesture(view, tree, dt_ms)` which already exists at `emViewInputFilter.rs:2148`. It needs to be called every frame from `about_to_wait`.

- [ ] **Step 1: Add emDefaultTouchVIF import to emWindow.rs**

In `crates/emcore/src/emWindow.rs`, add `emDefaultTouchVIF` to the existing import from `emViewInputFilter`:

```rust
use crate::emViewInputFilter::{CheatAction, emCheatVIF, emDefaultTouchVIF, emKeyboardZoomScrollVIF, emMouseZoomScrollVIF, emViewInputFilter};
```

- [ ] **Step 2: Add touch_vif field to ZuiWindow**

In `crates/emcore/src/emWindow.rs`, add to the `ZuiWindow` struct (after `cheat_vif`):

```rust
    touch_vif: emDefaultTouchVIF,
```

- [ ] **Step 3: Initialize touch_vif in ZuiWindow::create**

In `crates/emcore/src/emWindow.rs`, in the `Self { ... }` block of `create()`, add after `cheat_vif: emCheatVIF::new(),`:

```rust
            touch_vif: emDefaultTouchVIF::new(),
```

- [ ] **Step 4: Add touch event handling to ZuiWindow::handle_input**

The existing `handle_input` returns `Option<emInputEvent>` — but touch events don't map to `emInputEvent`. They need separate handling. Add a new method to `ZuiWindow`:

```rust
    /// Handle a winit Touch event by routing to the emDefaultTouchVIF.
    /// Returns true if the event was consumed.
    pub fn handle_touch(
        &mut self,
        touch: &winit::event::Touch,
        tree: &mut PanelTree,
    ) -> bool {
        use winit::event::TouchPhase;
        match touch.phase {
            TouchPhase::Started => {
                self.touch_vif.touch_start(
                    touch.id,
                    touch.location.x,
                    touch.location.y,
                    &mut self.view,
                    tree,
                )
            }
            TouchPhase::Moved => {
                // dt=0.016 is a reasonable default; the real frame delta is
                // applied in cycle_gesture which runs each frame.
                self.touch_vif.touch_move(
                    touch.id,
                    touch.location.x,
                    touch.location.y,
                    0.016,
                    &mut self.view,
                    tree,
                )
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.touch_vif.touch_end(touch.id, &mut self.view, tree)
            }
        }
    }
```

- [ ] **Step 5: Route WindowEvent::Touch in emGUIFramework::window_event**

In `crates/emcore/src/emGUIFramework.rs`, in the `window_event` method, add a new match arm **before** the `ref input_event =>` catch-all (i.e. after the `WindowEvent::Focused` arm):

```rust
            WindowEvent::Touch(touch) => {
                if let Some(win) = self.windows.get_mut(&window_id) {
                    win.handle_touch(&touch, &mut self.tree);
                    win.invalidate();
                    win.request_redraw();
                }
            }
```

- [ ] **Step 6: Call cycle_gesture each frame in tick_vif_animations**

In `crates/emcore/src/emWindow.rs`, modify `tick_vif_animations` to also tick the touch VIF's gesture timer. The `dt` parameter is seconds; `cycle_gesture` needs milliseconds:

```rust
    pub fn tick_vif_animations(&mut self, tree: &mut PanelTree, dt: f64) -> bool {
        let view = &mut self.view;
        let mut active = false;
        for vif in &mut self.vif_chain {
            if vif.animate(view, tree, dt) {
                active = true;
            }
        }
        // Tick touch gesture timer (C++ emDefaultTouchVIF::Cycle)
        let dt_ms = (dt * 1000.0) as i32;
        self.touch_vif.cycle_gesture(view, tree, dt_ms);
        // Tick fling animation
        if self.touch_vif.animate_fling(view, dt) {
            active = true;
        }
        active
    }
```

- [ ] **Step 7: Expose touch_vif for ForwardInput (Task 2 will need it)**

Add a public accessor to `ZuiWindow`:

```rust
    pub fn touch_vif_mut(&mut self) -> &mut emDefaultTouchVIF {
        &mut self.touch_vif
    }
```

- [ ] **Step 8: Add emDefaultTouchVIF to test pipeline VIF chains**

In `crates/eaglemode/tests/support/pipeline.rs`, add the import:

```rust
use emcore::emViewInputFilter::emDefaultTouchVIF;
```

And add a `touch_vif: emDefaultTouchVIF` field to `PipelineTestHarness`, initialized with `emDefaultTouchVIF::new()` in the constructor.

Do the same in `crates/eaglemode/tests/support/mod.rs` for `TestHarness`.

- [ ] **Step 9: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 10: Commit**

```bash
git add crates/emcore/src/emWindow.rs crates/emcore/src/emGUIFramework.rs crates/eaglemode/tests/support/pipeline.rs crates/eaglemode/tests/support/mod.rs
git commit -m "feat(touch): instantiate emDefaultTouchVIF in window, route winit Touch events, call cycle_gesture each frame"
```

---

## Task 2: ForwardInput Infrastructure (Gap 2)

**Files:**
- Modify: `crates/emcore/src/emViewInputFilter.rs` (`drain_gesture_actions`)
- Modify: `crates/emcore/src/emWindow.rs` (`dispatch_input` or new method)

### Context

The C++ `ForwardInput(event, state)` is an inline method on `emViewInputFilter` that calls either `Next->Input(event, state)` or `View.Input(event, state)`. In Rust, the touch VIF is **not** in the VIF chain — it's a separate field. The correct approach is:

1. `drain_gesture_actions` collects `ForwardInput` actions into a returned `Vec` instead of dropping them.
2. The window dispatches these synthetic events through `dispatch_input`, the same path as real events.

This matches the C++ behavior: the synthetic event enters the filter chain at the point after the touch VIF, which means the mouse/keyboard VIFs and panel dispatch all see it.

- [ ] **Step 1: Write test for ForwardInput action collection**

Add to the test module in `crates/emcore/src/emViewInputFilter.rs`:

```rust
#[test]
fn test_drain_gesture_actions_returns_forward_input() {
    use super::*;
    let mut vif = emDefaultTouchVIF::new();
    // Manually push a ForwardInput action to simulate gesture machine output
    vif.gesture_tracker.pending_actions.push(GestureAction::ForwardInput {
        key: InputKey::MouseLeft,
        variant: InputVariant::Press,
        mouse_x: 100.0,
        mouse_y: 200.0,
        shift: false,
        ctrl: false,
    });
    let mut view = emView::new(PanelId::from_raw(0), 800.0, 600.0);
    let forward_events = vif.drain_gesture_actions(&mut view);
    assert_eq!(forward_events.len(), 1);
    assert_eq!(forward_events[0].key, InputKey::MouseLeft);
    assert_eq!(forward_events[0].variant, InputVariant::Press);
    assert!((forward_events[0].mouse_x - 100.0).abs() < 1e-6);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib -- test_drain_gesture_actions_returns_forward_input`
Expected: FAIL — `drain_gesture_actions` returns `()` not `Vec<emInputEvent>`

- [ ] **Step 3: Change drain_gesture_actions to return synthetic events**

In `crates/emcore/src/emViewInputFilter.rs`, replace the `drain_gesture_actions` method:

```rust
    /// Process pending gesture actions that aren't handled inline by do_gesture.
    /// Returns synthetic input events that must be dispatched through the
    /// window's input pipeline (ForwardInput and InjectMenuKey actions).
    fn drain_gesture_actions(&mut self, _view: &mut emView) -> Vec<emInputEvent> {
        let mut forward_events = Vec::new();
        for action in self.gesture_tracker.pending_actions.drain(..) {
            match action {
                GestureAction::InjectMenuKey => {
                    dlog!("Touch gesture: inject menu key");
                    // Menu key press
                    forward_events.push(emInputEvent {
                        key: InputKey::Menu,
                        variant: InputVariant::Press,
                        chars: String::new(),
                        repeat: 0,
                        source_variant: 0,
                        mouse_x: 0.0,
                        mouse_y: 0.0,
                        shift: false,
                        ctrl: false,
                        alt: false,
                        meta: false,
                        eaten: false,
                    });
                    // Menu key release
                    forward_events.push(emInputEvent {
                        key: InputKey::Menu,
                        variant: InputVariant::Release,
                        chars: String::new(),
                        repeat: 0,
                        source_variant: 0,
                        mouse_x: 0.0,
                        mouse_y: 0.0,
                        shift: false,
                        ctrl: false,
                        alt: false,
                        meta: false,
                        eaten: false,
                    });
                }
                GestureAction::ToggleSoftKeyboard => {
                    dlog!("Touch gesture: toggle soft keyboard");
                    // TODO: _view.show_soft_keyboard(!_view.is_soft_keyboard_shown())
                }
                GestureAction::ForwardInput {
                    key,
                    variant,
                    mouse_x,
                    mouse_y,
                    shift,
                    ctrl,
                } => {
                    dlog!(
                        "Touch gesture: forward input {:?} {:?} at ({:.0}, {:.0})",
                        key,
                        variant,
                        mouse_x,
                        mouse_y
                    );
                    forward_events.push(emInputEvent {
                        key,
                        variant,
                        chars: String::new(),
                        repeat: 0,
                        source_variant: 0,
                        mouse_x,
                        mouse_y,
                        shift,
                        ctrl,
                        alt: false,
                        meta: false,
                        eaten: false,
                    });
                }
            }
        }
        forward_events
    }
```

- [ ] **Step 4: Update all callers of drain_gesture_actions**

There are three call sites in `emDefaultTouchVIF`:
- `touch_start` (line ~1973): `self.drain_gesture_actions(view);`
- `touch_move` (line ~2044): `self.drain_gesture_actions(view);`
- `touch_end` (line ~2097): `self.drain_gesture_actions(view);`
- `cycle_gesture` (line ~2151): `self.drain_gesture_actions(view);`

All four need to collect the returned events. But `touch_start`/`touch_move`/`touch_end` are called from `ZuiWindow::handle_touch` which doesn't have access to the input state. The simplest approach: buffer the forward events on `emDefaultTouchVIF` and let the window drain them.

Add a field to `emDefaultTouchVIF`:

```rust
    /// Synthetic input events from ForwardInput/InjectMenuKey that the window
    /// must dispatch through its input pipeline.
    pending_forward_events: Vec<emInputEvent>,
```

Initialize in `new()`:

```rust
            pending_forward_events: Vec::new(),
```

Change `drain_gesture_actions` callers to buffer:

```rust
    // In touch_start, touch_move, touch_end, cycle_gesture:
    let events = self.drain_gesture_actions(view);
    self.pending_forward_events.extend(events);
```

Add a drain method:

```rust
    /// Drain buffered synthetic input events for the window to dispatch.
    pub fn drain_forward_events(&mut self) -> Vec<emInputEvent> {
        std::mem::take(&mut self.pending_forward_events)
    }
```

- [ ] **Step 5: Dispatch forward events from the window**

`handle_touch` doesn't have access to `emInputState`. The dispatch happens in `emGUIFramework` which owns both the window and the input state.

**C++ modifier lifetime semantics:** In C++, `ForwardInput` is only called from `Input()`, never from `Cycle()`. Modifier keys (Shift, Ctrl) set during EmuMouse entry **persist on `emInputState` across frames** until explicitly cleared on release. The press event is forwarded once on entry; the release event is forwarded once on exit. Between frames, the modifier state persists so that real events passing through `dispatch_input` also carry the synthetic modifier.

The Rust approach must match: press events set modifiers on `input_state` and **leave them set**; release events clear them. Do NOT release modifiers after each dispatch call.

In `crates/emcore/src/emGUIFramework.rs`, add a helper to dispatch forward events:

```rust
    /// Dispatch synthetic input events from the touch gesture machine.
    /// Modifier keys are set/cleared on input_state to match C++ InputState
    /// persistence: press events set modifiers, release events clear them.
    fn dispatch_forward_events(
        win: &mut ZuiWindow,
        tree: &mut PanelTree,
        input_state: &mut emInputState,
    ) {
        let forward_events = win.touch_vif_mut().drain_forward_events();
        if forward_events.is_empty() {
            return;
        }
        for event in &forward_events {
            // C++ parity: modifiers are SET on press and CLEARED on release.
            // They persist across frames so real events also see them.
            match event.variant {
                InputVariant::Press => {
                    if event.shift {
                        input_state.press(InputKey::Shift);
                    }
                    if event.ctrl {
                        input_state.press(InputKey::Ctrl);
                    }
                }
                InputVariant::Release => {
                    if event.shift {
                        input_state.release(InputKey::Shift);
                    }
                    if event.ctrl {
                        input_state.release(InputKey::Ctrl);
                    }
                }
                _ => {}
            }
            input_state.set_mouse(event.mouse_x, event.mouse_y);
            let mut ev = event.clone();
            ev.mouse_x = input_state.mouse_x;
            ev.mouse_y = input_state.mouse_y;
            win.dispatch_input(tree, &ev, input_state);
        }
        win.invalidate();
        win.request_redraw();
    }
```

In the `WindowEvent::Touch` arm:

```rust
            WindowEvent::Touch(touch) => {
                if let Some(win) = self.windows.get_mut(&window_id) {
                    win.handle_touch(&touch, &mut self.tree);
                    Self::dispatch_forward_events(
                        win, &mut self.tree, &mut self.input_state,
                    );
                }
            }
```

For `about_to_wait`: the existing per-window loop extracts `let tree = &mut self.tree;` before iterating `self.windows.values_mut()`. Extract `input_state` the same way. Add forward event dispatch at the end of the per-window loop body, after `tick_vif_animations`:

```rust
        let tree = &mut self.tree;
        let state = &mut self.input_state;
        for win in self.windows.values_mut() {
            // ... existing per-window code ...

            // Dispatch synthetic events from gesture timer transitions
            // (cycle_gesture may have fired 250ms timeouts → EmuMouse/Visit/Menu)
            let forward_events = win.touch_vif_mut().drain_forward_events();
            if !forward_events.is_empty() {
                for event in &forward_events {
                    match event.variant {
                        InputVariant::Press => {
                            if event.shift { state.press(InputKey::Shift); }
                            if event.ctrl { state.press(InputKey::Ctrl); }
                        }
                        InputVariant::Release => {
                            if event.shift { state.release(InputKey::Shift); }
                            if event.ctrl { state.release(InputKey::Ctrl); }
                        }
                        _ => {}
                    }
                    state.set_mouse(event.mouse_x, event.mouse_y);
                    let mut ev = event.clone();
                    ev.mouse_x = state.mouse_x;
                    ev.mouse_y = state.mouse_y;
                    win.dispatch_input(tree, &ev, state);
                }
                win.invalidate();
                win.request_redraw();
            }
        }
```

This works because `tree` and `state` are extracted before the loop, matching the existing `about_to_wait` borrow pattern (see line 277 where `let tree = &mut self.tree;` is already done). The implementor must add `state` extraction alongside `tree` and thread it into the existing loop body.

- [ ] **Step 6: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emViewInputFilter.rs crates/emcore/src/emWindow.rs crates/emcore/src/emGUIFramework.rs
git commit -m "feat(touch): implement ForwardInput infrastructure — synthetic events dispatched through window input pipeline"
```

---

## Task 3: Soft Keyboard API and ToggleSoftKeyboard (Gap 3 — soft keyboard part)

**Files:**
- Modify: `crates/emcore/src/emView.rs`
- Modify: `crates/emcore/src/emViewInputFilter.rs` (`drain_gesture_actions`)

### Context

C++ `emView` has `IsSoftKeyboardShown() -> bool` and `ShowSoftKeyboard(bool)` as virtual methods delegating to `CurrentViewPort`. On desktop Linux, these are no-ops (only meaningful on Android/touch platforms). We add the API surface with stub implementations for desktop, matching C++ parity.

- [ ] **Step 1: Write test for soft keyboard API**

Add to the test module in `crates/emcore/src/emView.rs` (or `emViewInputFilter.rs` if view tests live there):

```rust
#[test]
fn test_soft_keyboard_toggle() {
    let root = PanelId::from_raw(0);
    let mut view = emView::new(root, 800.0, 600.0);
    assert!(!view.IsSoftKeyboardShown());
    view.ShowSoftKeyboard(true);
    assert!(view.IsSoftKeyboardShown());
    view.ShowSoftKeyboard(false);
    assert!(!view.IsSoftKeyboardShown());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib -- test_soft_keyboard_toggle`
Expected: FAIL — methods don't exist

- [ ] **Step 3: Add soft keyboard API to emView**

In `crates/emcore/src/emView.rs`, add a field to `emView`:

```rust
    /// Whether the soft keyboard is shown (touch platforms only).
    /// C++ emView::IsSoftKeyboardShown / ShowSoftKeyboard.
    soft_keyboard_shown: bool,
```

Initialize to `false` in `new()`.

Add methods:

```rust
    /// Whether the soft keyboard is currently shown.
    /// C++ `emView::IsSoftKeyboardShown()`.
    pub fn IsSoftKeyboardShown(&self) -> bool {
        self.soft_keyboard_shown
    }

    /// Show or hide the soft keyboard.
    /// C++ `emView::ShowSoftKeyboard(bool show)`.
    /// DIVERGED: C++ delegates to CurrentViewPort which delegates to the
    /// platform window. Desktop stub stores flag only — no actual keyboard
    /// is shown until a platform-specific viewport implements this.
    pub fn ShowSoftKeyboard(&mut self, show: bool) {
        self.soft_keyboard_shown = show;
    }
```

- [ ] **Step 4: Wire ToggleSoftKeyboard in drain_gesture_actions**

In `crates/emcore/src/emViewInputFilter.rs`, in `drain_gesture_actions`, change the `ToggleSoftKeyboard` arm from the TODO stub to:

```rust
                GestureAction::ToggleSoftKeyboard => {
                    dlog!("Touch gesture: toggle soft keyboard");
                    _view.ShowSoftKeyboard(!_view.IsSoftKeyboardShown());
                }
```

(Change `_view` parameter name to `view` since it's now used.)

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emView.rs crates/emcore/src/emViewInputFilter.rs
git commit -m "feat(emView): add IsSoftKeyboardShown/ShowSoftKeyboard API, wire ToggleSoftKeyboard gesture action"
```

---

## Task 4: InjectMenuKey via ForwardInput (Gap 3 — menu key part)

**Files:**
- Modify: `crates/emcore/src/emViewInputFilter.rs` (test only — implementation done in Task 2)

### Context

`InjectMenuKey` was already wired in Task 2's `drain_gesture_actions` rewrite — it pushes `InputKey::Menu` press+release into `forward_events`. This task adds a targeted integration test to verify the full chain works.

- [ ] **Step 1: Write test for InjectMenuKey forward events**

Add to the test module in `crates/emcore/src/emViewInputFilter.rs`:

```rust
#[test]
fn test_inject_menu_key_produces_press_release() {
    use super::*;
    let mut vif = emDefaultTouchVIF::new();
    vif.gesture_tracker.pending_actions.push(GestureAction::InjectMenuKey);
    let mut view = emView::new(PanelId::from_raw(0), 800.0, 600.0);
    let events = vif.drain_gesture_actions(&mut view);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].key, InputKey::Menu);
    assert_eq!(events[0].variant, InputVariant::Press);
    assert_eq!(events[1].key, InputKey::Menu);
    assert_eq!(events[1].variant, InputVariant::Release);
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p emcore --lib -- test_inject_menu_key_produces_press_release`
Expected: PASS (implementation from Task 2)

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emViewInputFilter.rs
git commit -m "test(touch): add InjectMenuKey forward event test"
```

---

## Task 5: Resolve Dual State Machine Conflict (Gap 4)

**Files:**
- Modify: `crates/emcore/src/emViewInputFilter.rs` (`touch_start`, `touch_move`, `touch_end`)

### Context

The old 4-state system (`TouchState`: `Idle` → `SingleTouch` → `PinchZoom` → `Fling`) runs simultaneously with the 18-state gesture machine. Now that `cycle_gesture` is called each frame (Task 1), the following conflicts arise:

1. **First 20px of drag:** Old system scrolls immediately (no dead zone). Gesture machine waits for 20px before transitioning `FirstDown` → `Scroll`. During the first 20px, the old system moves the view, then the gesture machine takes over. C++ has the 20px dead zone from the start.

2. **Pinch zoom:** Old system enters `PinchZoom` on second finger. Gesture machine enters `SecondDown` and waits for 250ms timeout. With `cycle_gesture` working, the timeout fires and transitions to `EmuMouse`, conflicting with the old pinch zoom.

3. **Hold-to-zoom:** Old system's `SingleTouch` scrolls during the 250ms `FirstDown` wait. When `cycle_gesture` fires the timeout, `ZoomIn` starts but the view has already moved.

**Resolution:** Suppress the old system's `SingleTouch` and `PinchZoom` states when the gesture machine is active (past `Ready`/`Finish`). Only `Fling` is kept from the old system — the gesture machine has no fling/inertia concept.

- [ ] **Step 1: Write test for dead zone behavior**

```rust
#[test]
fn test_gesture_dead_zone_no_scroll_under_20px() {
    use super::*;
    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_focusable(root, true);
    tree.Layout(root, 0.0, 0.0, 1.0, 1.0);
    let mut view = emView::new(root, 800.0, 600.0);
    view.Update(&mut tree);
    // Scroll position is stored in the visit state's rel_x/rel_y.
    // (There is no GetScrollX — use current_visit().rel_x.)
    let rx_before = view.current_visit().rel_x;
    let ry_before = view.current_visit().rel_y;

    let mut vif = emDefaultTouchVIF::new();
    vif.touch_start(1, 100.0, 100.0, &mut view, &mut tree);
    // Move 10px — under the gesture machine's 20px dead zone
    vif.touch_move(1, 110.0, 100.0, 0.016, &mut view, &mut tree);

    // With old system suppressed, view should NOT have scrolled
    let rx_after = view.current_visit().rel_x;
    let ry_after = view.current_visit().rel_y;
    assert!(
        (rx_after - rx_before).abs() < 1e-12
            && (ry_after - ry_before).abs() < 1e-12,
        "View scrolled during dead zone — old SingleTouch not suppressed \
         (dx={:.6}, dy={:.6})",
        rx_after - rx_before,
        ry_after - ry_before,
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib -- test_gesture_dead_zone_no_scroll_under_20px`
Expected: FAIL — old system's SingleTouch scrolls immediately

- [ ] **Step 3: Broaden gesture_handles_move guard in touch_move**

The key insight: the old system's `SingleTouch`/`PinchZoom` states are still entered in `touch_start` (for fling velocity tracking), but their **effects** in `touch_move` are blocked by the `gesture_handles_move` guard. The gesture machine transitions from `Ready` to `FirstDown` inside `run_gesture_loop` at the end of `touch_start`, so by the time `touch_move` is called, the guard sees the correct state.

Currently `gesture_handles_move` only covers `Scroll | ZoomIn | ZoomOut`. Expand it to cover all active gesture states. In `touch_move`, replace the `gesture_handles_move` computation:

```rust
        // The gesture machine handles input in all active states.
        // Only Ready and Finish allow the old 4-state system to handle moves.
        let gesture_handles_move = !matches!(
            self.gesture_tracker.gesture_state,
            GestureState::Ready | GestureState::Finish
        );
```

This already guards SingleTouch scroll and PinchZoom in touch_move — it just needs to be broadened from the current `Scroll | ZoomIn | ZoomOut` check.

- [ ] **Step 4: Suppress PinchZoom entry in touch_start when gesture machine is active**

In `touch_start`, the `match self.active_count` block (lines ~1927-1953) enters `PinchZoom` when a second finger arrives. By this point, `run_gesture_loop` has already run for the first finger (the gesture machine is in `FirstDown`). Guard the `PinchZoom` entry:

```rust
            2 => {
                // Don't enter PinchZoom if gesture machine is handling two-finger input.
                // The gesture machine transitions FirstDown→SecondDown on second touch,
                // and SecondDown handles swipe direction detection and EmuMouse.
                let gesture_active = !matches!(
                    self.gesture_tracker.gesture_state,
                    GestureState::Ready | GestureState::Finish
                );
                if !gesture_active {
                    let mut ids = Vec::new();
                    for tp in self.touches.iter().flatten() {
                        ids.push(tp.id);
                        if ids.len() == 2 {
                            break;
                        }
                    }
                    if ids.len() == 2 {
                        self.state = TouchState::PinchZoom {
                            id1: ids[0],
                            id2: ids[1],
                        };
                        self.last_pinch_distance = self.pinch_distance(ids[0], ids[1]);
                    }
                }
            }
```

Note: the `active_count == 1` arm still enters `SingleTouch` unconditionally — this is needed for fling velocity tracking. Its scroll effect is blocked by `gesture_handles_move` in `touch_move`.

Step 3 broadened the guard and Step 4 blocked PinchZoom entry. Now verify the combined effect:

- [ ] **Step 5: Verify touch_move guard covers all states**

The existing `touch_move` code at lines 2007-2040 already has `gesture_handles_move` guarding both `SingleTouch` scroll (line 2013) and `PinchZoom` zoom (line 2028). The change is:

```rust
        // OLD:
        let gesture_handles_move = matches!(
            self.gesture_tracker.gesture_state,
            GestureState::Scroll | GestureState::ZoomIn | GestureState::ZoomOut
        );

        // NEW:
        let gesture_handles_move = !matches!(
            self.gesture_tracker.gesture_state,
            GestureState::Ready | GestureState::Finish
        );
```

This means:
- `FirstDown` (waiting for 20px or 250ms): old system scroll is suppressed → dead zone works
- `SecondDown` (waiting for swipe direction): old system pinch zoom is suppressed
- `EmuMouse1-4`: old system suppressed → emulated mouse works exclusively
- `Scroll`/`ZoomIn`/`ZoomOut`: same as before (suppressed)

- [ ] **Step 6: Guard touch_end PinchZoom→SingleTouch fallback**

In `touch_end`, the `PinchZoom` arm (line 2070-2080) falls back to `SingleTouch` when one finger lifts. When the gesture machine is active, this should remain `Idle` instead:

```rust
            TouchState::PinchZoom { id1, id2 } => {
                let remaining_id = if id == id1 { id2 } else { id1 };
                let gesture_active = !matches!(
                    self.gesture_tracker.gesture_state,
                    GestureState::Ready | GestureState::Finish
                );
                if !gesture_active && self.get_touch(remaining_id).is_some() {
                    self.state = TouchState::SingleTouch { id: remaining_id };
                    self.smoothed_vx = 0.0;
                    self.smoothed_vy = 0.0;
                } else {
                    self.state = TouchState::Idle;
                }
            }
```

- [ ] **Step 7: Preserve fling from gesture machine's Scroll→Finish**

When the gesture machine goes Scroll→Finish (finger lifted during scroll), the old system should transition to Fling if velocity is sufficient. The `touch_end` SingleTouch arm already handles this — but the SingleTouch state must be entered for scroll gestures.

In `touch_start`, SingleTouch is always entered for `active_count == 1` — this is correct. The velocity tracking in `touch_move` (lines 2016-2023) runs inside the `dx/dy > 0.001` block but outside the `!gesture_handles_move` guard, so fling velocity is tracked regardless of which system does the scrolling. When `touch_end` fires and the old system is in `SingleTouch`, it checks velocity and enters `Fling`. No changes needed here — fling works correctly with the broadened guard.

- [ ] **Step 8: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS, including the dead zone test

- [ ] **Step 9: Commit**

```bash
git add crates/emcore/src/emViewInputFilter.rs
git commit -m "fix(touch): resolve dual state machine conflict — suppress old SingleTouch/PinchZoom when gesture machine is active, keep Fling"
```

---

## Verification Checklist

After all tasks are complete, verify each gesture from the gap document summary:

| Gesture | Expected | How to verify |
|---------|----------|---------------|
| Single-finger scroll (20px dead zone) | Works via gesture machine | Unit test: no scroll under 20px, scroll after 20px |
| Hold-to-zoom (250ms) | Works via gesture machine + cycle_gesture | Unit test: advance 260ms, check ZoomIn state |
| Double-tap visit | Works via gesture machine + cycle_gesture | Unit test: tap-release-tap-release + 260ms timeout → VisitFullsized |
| Two-finger mouse emulation | Works via ForwardInput dispatch | Unit test: ForwardInput events produced, integration via dispatch_input |
| Three-finger menu key | Works via InjectMenuKey → ForwardInput | Unit test: Menu press+release events produced |
| Four-finger soft keyboard | Works via ToggleSoftKeyboard → ShowSoftKeyboard | Unit test: IsSoftKeyboardShown toggles |
| Fling after scroll | Works via old system's Fling state | Existing tests pass |
| Pinch zoom during gesture | Suppressed — gesture machine handles two-finger | Unit test: no PinchZoom state entry when gesture active |
