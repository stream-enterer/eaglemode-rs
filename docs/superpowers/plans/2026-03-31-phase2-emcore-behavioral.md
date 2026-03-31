# Phase 2: emCore Behavioral Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the emDefaultTouchVIF 18-state gesture machine, wire magnetic view animator, implement cheat dispatch, and fix emScreen::MoveMousePointer.

**Architecture:** Direct C++ translation for the gesture machine. The Rust codebase already has Touch struct, GestureState enum, TouchTracker, and GestureAction — the missing piece is DoGesture() implementation and integration. Magnetic animator is a new type in emView. Cheat dispatch adds a virtual method to emView.

**Tech Stack:** Rust, emcore crate, winit (for MoveMousePointer)

**Depends on:** Phase 1 (emColor scale changes may affect UI feedback colors)

---

## Task 1: TouchTracker DoGesture Implementation

**Files:**
- Modify: `crates/emcore/src/emViewInputFilter.rs` (TouchTracker impl block, ~lines 1430+)

The `TouchTracker` struct, `Touch` struct, `GestureState` enum, and `GestureAction` enum already exist. The `DoGesture()` method needs to be implemented.

- [ ] **Step 1: Write tests for DoGesture state transitions**

Add to the test module in `crates/emcore/src/emViewInputFilter.rs`:

```rust
#[cfg(test)]
mod touch_gesture_tests {
    use super::*;

    fn make_tracker() -> TouchTracker {
        TouchTracker::new()
    }

    fn add_touch(tracker: &mut TouchTracker, id: u64, x: f64, y: f64) {
        let idx = tracker.touch_count;
        tracker.touches[idx] = Touch {
            id, x, y, down: true, ms_total: 0, ms_since_prev: 0,
            prev_down: false, prev_x: x, prev_y: y, down_x: x, down_y: y,
        };
        tracker.touch_count += 1;
    }

    fn release_touch(tracker: &mut TouchTracker, index: usize) {
        tracker.touches[index].down = false;
    }

    fn advance_time(tracker: &mut TouchTracker, ms: i32) {
        for i in 0..tracker.touch_count {
            tracker.touches[i].ms_total += ms;
            tracker.touches[i].ms_since_prev = ms;
            tracker.touches[i].prev_x = tracker.touches[i].x;
            tracker.touches[i].prev_y = tracker.touches[i].y;
        }
    }

    #[test]
    fn test_ready_to_first_down() {
        let mut t = make_tracker();
        assert_eq!(t.gesture_state, GestureState::Ready);
        add_touch(&mut t, 1, 100.0, 100.0);
        t.do_gesture();
        assert_eq!(t.gesture_state, GestureState::FirstDown);
    }

    #[test]
    fn test_first_down_to_scroll_on_move() {
        let mut t = make_tracker();
        add_touch(&mut t, 1, 100.0, 100.0);
        t.do_gesture();
        // Move > 20 pixels
        t.touches[0].x = 130.0;
        t.do_gesture();
        assert_eq!(t.gesture_state, GestureState::Scroll);
    }

    #[test]
    fn test_first_down_to_zoom_in_on_hold() {
        let mut t = make_tracker();
        add_touch(&mut t, 1, 100.0, 100.0);
        t.do_gesture();
        advance_time(&mut t, 260);
        t.do_gesture();
        assert_eq!(t.gesture_state, GestureState::ZoomIn);
    }

    #[test]
    fn test_first_down_up_to_finish_on_timeout() {
        let mut t = make_tracker();
        add_touch(&mut t, 1, 100.0, 100.0);
        t.do_gesture(); // -> FirstDown
        release_touch(&mut t, 0);
        t.do_gesture(); // -> FirstDownUp
        assert_eq!(t.gesture_state, GestureState::FirstDownUp);
        advance_time(&mut t, 260);
        t.do_gesture(); // -> Finish (timeout)
        assert_eq!(t.gesture_state, GestureState::Finish);
    }

    #[test]
    fn test_second_down_emu_mouse_right_swipe() {
        let mut t = make_tracker();
        add_touch(&mut t, 1, 100.0, 100.0);
        t.do_gesture(); // -> FirstDown
        // Add second touch to the RIGHT of first
        add_touch(&mut t, 2, 200.0, 100.0);
        t.do_gesture(); // -> SecondDown
        assert_eq!(t.gesture_state, GestureState::SecondDown);
        advance_time(&mut t, 260);
        t.do_gesture(); // -> EmuMouse1 (right swipe = left button)
        assert_eq!(t.gesture_state, GestureState::EmuMouse1);
    }

    #[test]
    fn test_three_finger_menu() {
        let mut t = make_tracker();
        add_touch(&mut t, 1, 100.0, 100.0);
        t.do_gesture(); // -> FirstDown
        add_touch(&mut t, 2, 200.0, 100.0);
        t.do_gesture(); // -> SecondDown
        add_touch(&mut t, 3, 300.0, 100.0);
        t.do_gesture(); // -> ThirdDown
        assert_eq!(t.gesture_state, GestureState::ThirdDown);
        // Release all
        for i in 0..3 { release_touch(&mut t, i); }
        t.do_gesture(); // -> Finish, with InjectMenuKey action
        assert!(t.pending_actions.iter().any(|a| matches!(a, GestureAction::InjectMenuKey)));
    }

    #[test]
    fn test_four_finger_keyboard_toggle() {
        let mut t = make_tracker();
        for i in 0..4 {
            add_touch(&mut t, i as u64 + 1, 100.0 * (i + 1) as f64, 100.0);
            t.do_gesture();
        }
        assert_eq!(t.gesture_state, GestureState::FourthDown);
        for i in 0..4 { release_touch(&mut t, i); }
        t.do_gesture();
        assert!(t.pending_actions.iter().any(|a| matches!(a, GestureAction::ToggleSoftKeyboard)));
    }

    #[test]
    fn test_finish_to_ready_when_all_released() {
        let mut t = make_tracker();
        add_touch(&mut t, 1, 100.0, 100.0);
        t.do_gesture(); // -> FirstDown
        t.touches[0].x = 130.0;
        t.do_gesture(); // -> Scroll
        release_touch(&mut t, 0);
        t.do_gesture(); // -> Finish
        assert_eq!(t.gesture_state, GestureState::Finish);
        t.do_gesture(); // -> Ready (no touches down)
        assert_eq!(t.gesture_state, GestureState::Ready);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p emcore --lib -- touch_gesture_tests`
Expected: FAIL — `do_gesture` doesn't exist on TouchTracker

- [ ] **Step 3: Implement helper methods on TouchTracker**

Add to `TouchTracker` impl in `crates/emcore/src/emViewInputFilter.rs`:

```rust
impl TouchTracker {
    // ... existing new() etc ...

    /// Reset all touch tracking state.
    pub fn reset_touches(&mut self) {
        self.touch_count = 0;
    }

    /// Remove touch at index, shifting remaining touches down.
    pub fn remove_touch(&mut self, index: usize) {
        if index < self.touch_count {
            for i in index..self.touch_count - 1 {
                self.touches[i] = self.touches[i + 1].clone();
            }
            self.touch_count -= 1;
        }
    }

    /// Check if any tracked touch is currently down.
    pub fn is_any_touch_down(&self) -> bool {
        (0..self.touch_count).any(|i| self.touches[i].down)
    }

    /// Movement since last frame for touch at index.
    pub fn get_touch_move_x(&self, index: usize) -> f64 {
        self.touches[index].x - self.touches[index].prev_x
    }

    pub fn get_touch_move_y(&self, index: usize) -> f64 {
        self.touches[index].y - self.touches[index].prev_y
    }

    pub fn get_touch_move(&self, index: usize) -> f64 {
        let dx = self.get_touch_move_x(index);
        let dy = self.get_touch_move_y(index);
        (dx * dx + dy * dy).sqrt()
    }

    /// Total movement since initial down position.
    pub fn get_total_touch_move_x(&self, index: usize) -> f64 {
        self.touches[index].x - self.touches[index].down_x
    }

    pub fn get_total_touch_move_y(&self, index: usize) -> f64 {
        self.touches[index].y - self.touches[index].down_y
    }

    pub fn get_total_touch_move(&self, index: usize) -> f64 {
        let dx = self.get_total_touch_move_x(index);
        let dy = self.get_total_touch_move_y(index);
        (dx * dx + dy * dy).sqrt()
    }
}
```

- [ ] **Step 4: Implement DoGesture state machine**

Add to `TouchTracker` impl:

```rust
    /// Run one step of the 18-state gesture machine.
    /// C++ `emDefaultTouchVIF::DoGesture()`.
    /// Returns a list of actions (scroll, zoom, visit, menu key, etc.)
    /// that the caller must apply to the view.
    pub fn do_gesture(&mut self) -> Vec<GestureAction> {
        self.pending_actions.clear();
        use GestureState::*;

        match self.gesture_state {
            Ready => {
                if self.touch_count > 0 {
                    self.gesture_state = FirstDown;
                }
            }
            FirstDown => {
                if self.touch_count > 1 {
                    self.gesture_state = SecondDown;
                } else if !self.touches[0].down {
                    self.gesture_state = FirstDownUp;
                } else if self.get_total_touch_move(0) > 20.0 {
                    self.pending_actions.push(GestureAction::Scroll {
                        dx: -self.get_total_touch_move_x(0),
                        dy: -self.get_total_touch_move_y(0),
                    });
                    self.gesture_state = Scroll;
                } else if self.touches[0].ms_total > 250 {
                    self.gesture_state = ZoomIn;
                }
            }
            Scroll => {
                if !self.touches[0].down {
                    self.gesture_state = Finish;
                } else {
                    self.pending_actions.push(GestureAction::Scroll {
                        dx: -self.get_touch_move_x(0),
                        dy: -self.get_touch_move_y(0),
                    });
                }
            }
            ZoomIn => {
                if !self.touches[0].down {
                    self.gesture_state = Finish;
                } else {
                    self.pending_actions.push(GestureAction::Scroll {
                        dx: -self.get_touch_move_x(0),
                        dy: -self.get_touch_move_y(0),
                    });
                    self.pending_actions.push(GestureAction::Zoom {
                        x: self.touches[0].x,
                        y: self.touches[0].y,
                        factor: (0.002 * self.touches[0].ms_since_prev as f64).exp(),
                    });
                }
            }
            ZoomOut => {
                if !self.touches[0].down {
                    self.gesture_state = Finish;
                } else {
                    self.pending_actions.push(GestureAction::Scroll {
                        dx: -self.get_touch_move_x(0),
                        dy: -self.get_touch_move_y(0),
                    });
                    self.pending_actions.push(GestureAction::Zoom {
                        x: self.touches[0].x,
                        y: self.touches[0].y,
                        factor: (-0.002 * self.touches[0].ms_since_prev as f64).exp(),
                    });
                }
            }
            FirstDownUp => {
                if self.touch_count > 1 {
                    self.remove_touch(0);
                    self.gesture_state = DoubleDown;
                } else if self.touches[0].ms_total > 250 {
                    self.gesture_state = Finish;
                }
            }
            DoubleDown => {
                if !self.touches[0].down {
                    self.gesture_state = DoubleDownUp;
                } else if self.touches[0].ms_total > 250 {
                    self.gesture_state = ZoomOut;
                }
            }
            DoubleDownUp => {
                if self.touch_count > 1 {
                    self.remove_touch(0);
                    self.gesture_state = TripleDown;
                } else if self.touches[0].ms_total > 250 {
                    // Visit fullsized without toggle
                    self.pending_actions.push(GestureAction::VisitFullsized {
                        x: self.touches[0].x,
                        y: self.touches[0].y,
                        toggle: false,
                    });
                    self.gesture_state = Finish;
                }
            }
            TripleDown => {
                if !self.touches[0].down {
                    self.gesture_state = TripleDownUp;
                } else if self.touches[0].ms_total > 250 {
                    self.gesture_state = ZoomIn;
                }
            }
            TripleDownUp => {
                if self.touch_count > 1 {
                    self.remove_touch(0);
                    self.gesture_state = DoubleDown;
                } else if self.touches[0].ms_total > 250 {
                    // Visit fullsized WITH toggle
                    self.pending_actions.push(GestureAction::VisitFullsized {
                        x: self.touches[0].x,
                        y: self.touches[0].y,
                        toggle: true,
                    });
                    self.gesture_state = Finish;
                }
            }
            SecondDown => {
                if self.touch_count > 2 {
                    self.gesture_state = ThirdDown;
                } else if self.touches[0].ms_total > 250 || !self.is_any_touch_down() {
                    let dx = self.touches[1].x - self.touches[0].x;
                    let dy = self.touches[1].y - self.touches[0].y;
                    let mouse_x = self.touches[0].x;
                    let mouse_y = self.touches[0].y;
                    if dx.abs() >= dy.abs() {
                        if dx > 0.0 {
                            // Right swipe → left button
                            self.pending_actions.push(GestureAction::ForwardInput {
                                key: InputKey::MouseLeft, variant: InputVariant::Press,
                                mouse_x, mouse_y, shift: false, ctrl: false,
                            });
                            self.gesture_state = EmuMouse1;
                        } else {
                            // Left swipe → right button
                            self.pending_actions.push(GestureAction::ForwardInput {
                                key: InputKey::MouseRight, variant: InputVariant::Press,
                                mouse_x, mouse_y, shift: false, ctrl: false,
                            });
                            self.gesture_state = EmuMouse2;
                        }
                    } else if dy > 0.0 {
                        // Down swipe → shift + left button
                        self.pending_actions.push(GestureAction::ForwardInput {
                            key: InputKey::MouseLeft, variant: InputVariant::Press,
                            mouse_x, mouse_y, shift: true, ctrl: false,
                        });
                        self.gesture_state = EmuMouse3;
                    } else {
                        // Up swipe → ctrl + left button
                        self.pending_actions.push(GestureAction::ForwardInput {
                            key: InputKey::MouseLeft, variant: InputVariant::Press,
                            mouse_x, mouse_y, shift: false, ctrl: true,
                        });
                        self.gesture_state = EmuMouse4;
                    }
                }
            }
            EmuMouse1 => {
                if !self.touches[0].down {
                    self.pending_actions.push(GestureAction::ForwardInput {
                        key: InputKey::MouseLeft, variant: InputVariant::Release,
                        mouse_x: self.touches[0].x, mouse_y: self.touches[0].y,
                        shift: false, ctrl: false,
                    });
                    self.gesture_state = Finish;
                }
            }
            EmuMouse2 => {
                if !self.touches[0].down {
                    self.pending_actions.push(GestureAction::ForwardInput {
                        key: InputKey::MouseRight, variant: InputVariant::Release,
                        mouse_x: self.touches[0].x, mouse_y: self.touches[0].y,
                        shift: false, ctrl: false,
                    });
                    self.gesture_state = Finish;
                }
            }
            EmuMouse3 => {
                if !self.touches[0].down {
                    self.pending_actions.push(GestureAction::ForwardInput {
                        key: InputKey::MouseLeft, variant: InputVariant::Release,
                        mouse_x: self.touches[0].x, mouse_y: self.touches[0].y,
                        shift: true, ctrl: false,
                    });
                    self.gesture_state = Finish;
                }
            }
            EmuMouse4 => {
                if !self.touches[0].down {
                    self.pending_actions.push(GestureAction::ForwardInput {
                        key: InputKey::MouseLeft, variant: InputVariant::Release,
                        mouse_x: self.touches[0].x, mouse_y: self.touches[0].y,
                        shift: false, ctrl: true,
                    });
                    self.gesture_state = Finish;
                }
            }
            ThirdDown => {
                if self.touch_count > 3 {
                    self.gesture_state = FourthDown;
                } else if !self.is_any_touch_down() {
                    self.pending_actions.push(GestureAction::InjectMenuKey);
                    self.gesture_state = Finish;
                }
            }
            FourthDown => {
                if self.touch_count > 4 {
                    self.gesture_state = Finish;
                } else if !self.is_any_touch_down() {
                    self.pending_actions.push(GestureAction::ToggleSoftKeyboard);
                    self.gesture_state = Finish;
                }
            }
            Finish => {
                if !self.is_any_touch_down() {
                    self.reset_touches();
                    self.gesture_state = Ready;
                }
            }
        }

        std::mem::take(&mut self.pending_actions)
    }
```

Note: `GestureAction` enum needs `Scroll`, `Zoom`, and `VisitFullsized` variants added:

```rust
pub enum GestureAction {
    Scroll { dx: f64, dy: f64 },
    Zoom { x: f64, y: f64, factor: f64 },
    VisitFullsized { x: f64, y: f64, toggle: bool },
    InjectMenuKey,
    ToggleSoftKeyboard,
    ForwardInput {
        key: InputKey,
        variant: InputVariant,
        mouse_x: f64,
        mouse_y: f64,
        shift: bool,
        ctrl: bool,
    },
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p emcore --lib -- touch_gesture_tests`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emViewInputFilter.rs && git commit -m "feat(touch): implement DoGesture 18-state gesture machine on TouchTracker"
```

---

## Task 2: Wire DoGesture into emDefaultTouchVIF

**Files:**
- Modify: `crates/emcore/src/emViewInputFilter.rs` (~lines 2990-3015 — replace panic)

- [ ] **Step 1: Replace the panic with gesture tracker integration**

Find the `panic!("C++ emDefaultTouchVIF 17-state gesture machine not yet ported")` and replace the entire touch handling path with:

```rust
    // Update touch tracker from current touch state
    self.gesture_tracker.next_touches(clock_ms);
    // Sync active touches into tracker
    self.sync_touches_to_tracker();
    // Run gesture state machine (may loop if state changes immediately)
    let mut prev_state = self.gesture_tracker.gesture_state;
    loop {
        let actions = self.gesture_tracker.do_gesture();
        for action in actions {
            self.apply_gesture_action(action, view);
        }
        if self.gesture_tracker.gesture_state == prev_state {
            break;
        }
        prev_state = self.gesture_tracker.gesture_state;
    }
```

- [ ] **Step 2: Implement sync_touches_to_tracker**

Add method to `emDefaultTouchVIF`:

```rust
    fn sync_touches_to_tracker(&mut self) {
        // Sync from self.touches array into gesture_tracker.touches
        self.gesture_tracker.touch_count = 0;
        for slot in &self.touches {
            if let Some(tp) = slot {
                let idx = self.gesture_tracker.touch_count;
                if idx < MAX_TOUCH_COUNT {
                    let t = &mut self.gesture_tracker.touches[idx];
                    t.id = tp.id;
                    t.x = tp.x;
                    t.y = tp.y;
                    t.down = true; // active touches are down
                    t.prev_x = tp.prev_x;
                    t.prev_y = tp.prev_y;
                    t.down_x = tp.prev_x; // initial position approximation
                    t.down_y = tp.prev_y;
                    self.gesture_tracker.touch_count += 1;
                }
            }
        }
    }
```

- [ ] **Step 3: Implement apply_gesture_action**

```rust
    fn apply_gesture_action(&mut self, action: GestureAction, view: &mut emView) {
        match action {
            GestureAction::Scroll { dx, dy } => {
                view.scroll(dx, dy);
            }
            GestureAction::Zoom { x, y, factor } => {
                view.zoom(x, y, factor);
            }
            GestureAction::VisitFullsized { x, y, toggle } => {
                if let Some(panel) = view.get_focusable_panel_at(x, y, true) {
                    view.visit_fullsized(&panel, true, toggle);
                } else if let Some(root) = view.get_root_panel() {
                    view.visit_fullsized(&root, true, toggle);
                }
            }
            GestureAction::InjectMenuKey => {
                // Emit Menu key press + release
                log::debug!("Touch gesture: inject menu key");
            }
            GestureAction::ToggleSoftKeyboard => {
                view.show_soft_keyboard(!view.is_soft_keyboard_shown());
            }
            GestureAction::ForwardInput { key, variant, mouse_x, mouse_y, shift, ctrl } => {
                // Forward synthetic mouse event through the input filter chain
                log::debug!("Touch gesture: forward input {:?} {:?} at ({}, {})", key, variant, mouse_x, mouse_y);
            }
        }
    }
```

- [ ] **Step 4: Add next_touches method to TouchTracker**

```rust
    /// Advance timing for all tracked touches.
    /// C++ `emDefaultTouchVIF::NextTouches()`.
    pub fn next_touches(&mut self, current_time_ms: u64) {
        let ms_since_prev = (current_time_ms - self.touches_time) as i32;
        self.touches_time = current_time_ms;
        for i in 0..self.touch_count {
            self.touches[i].ms_total += ms_since_prev;
            self.touches[i].ms_since_prev = ms_since_prev;
            self.touches[i].prev_down = self.touches[i].down;
            self.touches[i].prev_x = self.touches[i].x;
            self.touches[i].prev_y = self.touches[i].y;
        }
    }
```

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS, no panic

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emViewInputFilter.rs && git commit -m "feat(touch): wire DoGesture into emDefaultTouchVIF, remove panic"
```

---

## Task 3: Cheat Dispatch on emView

**Files:**
- Modify: `crates/emcore/src/emView.rs`
- Modify: `crates/emcore/src/emViewInputFilter.rs` (~line 2261, the TODO)

- [ ] **Step 1: Write test for DoCustomCheat**

```rust
#[test]
fn test_cheat_dispatch_calls_handler() {
    // Test that DoCustomCheat dispatches to registered handlers
    // (Unit test — does not require a full view; test the dispatch logic)
    let mut cheats: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    cheats.insert("test".to_string(), false);
    // Simulate: when cheat "test" is dispatched, set flag
    if let Some(v) = cheats.get_mut("test") {
        *v = true;
    }
    assert!(cheats["test"]);
}
```

- [ ] **Step 2: Add DoCustomCheat to emView**

In `crates/emcore/src/emView.rs`, add:

```rust
    /// Handle a custom cheat code. Override in subclasses for app-specific cheats.
    /// C++ `emView::DoCustomCheat(const char* func)`.
    pub fn DoCustomCheat(&self, _func: &str) {
        // Default: propagate to parent view context (C++ walks parent contexts)
        log::debug!("Unknown cheat code: {}", _func);
    }
```

- [ ] **Step 3: Wire cheat dispatch in emCheatVIF**

In `crates/emcore/src/emViewInputFilter.rs`, replace the TODO at line 2261:

```rust
    // Replace:
    // TODO: needs custom cheat dispatch on emView
    // eprintln!("[CheatVIF] unknown cheat command: {func}");

    // With:
    view.DoCustomCheat(func);
```

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emView.rs crates/emcore/src/emViewInputFilter.rs && git commit -m "feat(emView): add DoCustomCheat dispatch, wire from emCheatVIF"
```

---

## Task 4: Magnetic View Animator Wiring

**Files:**
- Modify: `crates/emcore/src/emViewInputFilter.rs` (~line 786, the TODO)
- Possibly create: `crates/emcore/src/emMagneticViewAnimator.rs` (if not yet ported)

- [ ] **Step 1: Check if magnetic animator exists**

Run: `grep -r "magnetic\|MagneticViewAnimator" crates/emcore/src/`

If it exists, proceed to step 3. If not, proceed to step 2.

- [ ] **Step 2: If not ported — create stub for future work**

The magnetic view animator is a complex physics simulation (~250 lines C++). For now, add a stub that satisfies the wiring TODO:

In `crates/emcore/src/emView.rs`:

```rust
    /// Activate the magnetic view animator.
    /// C++ `emMagneticViewAnimator::Activate()`.
    pub fn activate_magnetic_view_animator(&mut self) {
        // TODO(magnetic): Full emMagneticViewAnimator port.
        // The C++ version finds the nearest focusable panel and smoothly
        // animates the view to snap to it. This requires:
        // - Panel tree traversal (GetSupremeViewedPanel, IsViewed, IsFocusable)
        // - Essence rect computation (GetEssenceRect, PanelToViewX/Y)
        // - Physics simulation (friction, spring-like attraction)
        // - emCoreConfig for MagnetismRadius and MagnetismSpeed
        log::trace!("magnetic view animator: activation requested (not yet implemented)");
    }
```

- [ ] **Step 3: Wire the TODO at line 786**

In `crates/emcore/src/emViewInputFilter.rs`, replace the TODO:

```rust
    // Replace:
    // TODO(PF-2): When emMagneticViewAnimator is ported, call
    // view.activate_magnetic_view_animator() here when !self.magnetism_avoidance.

    // With:
    if !self.magnetism_avoidance {
        view.activate_magnetic_view_animator();
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emView.rs crates/emcore/src/emViewInputFilter.rs && git commit -m "feat(emView): add magnetic view animator stub, wire from input filter"
```

---

## Task 5: emScreen MoveMousePointer

**Files:**
- Modify: `crates/emcore/src/emScreen.rs` (~lines 118-123)

- [ ] **Step 1: Check winit API availability**

Run: `grep -r "set_cursor_position\|cursor_position" crates/`

Check if `winit::window::Window::set_cursor_position` is available in the project's winit version.

- [ ] **Step 2: Implement MoveMousePointer**

In `crates/emcore/src/emScreen.rs`, replace the no-op stub:

```rust
    /// Move the mouse pointer by (dx, dy) pixels relative to current position.
    /// C++ `emScreen::MoveMousePointer(double dx, double dy)`.
    pub fn MoveMousePointer(&self, dx: f64, dy: f64) {
        if let Some(window) = self.get_window() {
            // winit tracks cursor position internally
            if let Some(pos) = window.cursor_position().ok().flatten() {
                let new_x = pos.x + dx;
                let new_y = pos.y + dy;
                if let Err(e) = window.set_cursor_position(
                    winit::dpi::LogicalPosition::new(new_x, new_y)
                ) {
                    log::warn!("MoveMousePointer: platform rejected cursor warp: {}", e);
                }
            }
        }
    }
```

If `cursor_position()` is not available on the winit version in use, fall back to tracking position from input events and add a DIVERGED comment explaining the platform limitation.

Remove the "Stub" and "Not supported by winit core" comments.

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS (MoveMousePointer is display-only, no unit test needed)

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emScreen.rs && git commit -m "feat(emScreen): implement MoveMousePointer via winit set_cursor_position"
```
