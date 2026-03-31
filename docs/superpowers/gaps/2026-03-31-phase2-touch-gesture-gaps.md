# Phase 2 Touch Gesture Gaps

> **Audience:** LLM agents working on eaglemode-rs. This document identifies
> specific code locations, missing infrastructure, and untracked work items
> that prevent the 18-state touch gesture machine from functioning end-to-end.
> None of these gaps are covered by Phases 3 or 4 of the gap-closure spec.

**Date identified:** 2026-03-31, during Phase 2 execution.
**Root cause:** The Phase 2 plan assumed the gesture machine would be built
from scratch. In reality, prior convergence passes (`1191c37`, `f85e975`,
`88b315c`) had already ported `do_gesture()` and the `TouchTracker`. The
Phase 2 wiring work connected `run_gesture_loop` to the touch event handlers
but left three categories of gaps.

---

## Gap 1: `cycle_gesture` Has No Caller — Time-Based Transitions Are Dead

### What's broken

The gesture state machine has 250ms timeout transitions that only fire when
`cycle_gesture()` is called each frame. Without a frame-loop caller, these
states stall forever:

| Transition | State | Condition | Effect |
|-----------|-------|-----------|--------|
| Hold-to-zoom | `FirstDown` → `ZoomIn` | `ms_total > 250` | Continuous zoom at touch point |
| Two-finger direction | `SecondDown` → `EmuMouse1-4` | `ms_total > 250` | Synthetic mouse emulation |
| Double-tap visit | `DoubleDownUp` → `Finish` | `ms_total > 250` | `VisitFullsized` navigation |
| Triple-tap toggle | `TripleDownUp` → `Finish` | `ms_total > 250` | `VisitFullsized` with toggle |
| Zoom-out hold | `DoubleDown` → `ZoomOut` | `ms_total > 250` | Continuous zoom out |
| Triple-down hold | `TripleDown` → `ZoomIn` | `ms_total > 250` | Re-enters zoom-in |
| Single-tap timeout | `FirstDownUp` → `Finish` | `ms_total > 250` | Tap chain expires |

### Code locations

- **Method that needs calling:**
  `emDefaultTouchVIF::cycle_gesture(view, tree, dt_ms)` at
  `crates/emcore/src/emViewInputFilter.rs:2148`

- **What it does:** Calls `self.gesture_tracker.next_touches(dt_ms)` to
  advance `ms_total` and `ms_since_prev` on all tracked touches, then
  `run_gesture_loop` to evaluate time-based transitions, then
  `drain_gesture_actions` to process any resulting actions.

- **Where it should be called:** The window event loop in
  `crates/emcore/src/emWindow.rs`. Currently the window creates a VIF chain
  (line 52: `vif_chain: Vec<Box<dyn emViewInputFilter>>`) containing
  `emMouseZoomScrollVIF`, `emKeyboardZoomScrollVIF`, and `emCheatVIF`
  (line 8 imports). `emDefaultTouchVIF` is **not instantiated** in the
  window at all. The window would need to:
  1. Add `emDefaultTouchVIF` to the VIF chain or as a separate field
  2. Call `cycle_gesture(view, tree, dt_ms)` each frame (in the same
     location where `animate_fling` would be called)
  3. Route touch events from winit (`WindowEvent::Touch`) to
     `touch_start`/`touch_move`/`touch_end`

- **C++ reference:** `emViewInputFilter.cpp:928` — C++ `emDefaultTouchVIF::Cycle()`
  calls `NextTouches()` then loops `DoGesture()`. The `Cycle()` method is
  called by the engine's frame scheduler (`emEngine::Cycle` override).

- **Test infrastructure also missing:** The test support pipelines at
  `crates/eaglemode/tests/support/pipeline.rs:42` and
  `crates/eaglemode/tests/support/mod.rs:43` create VIF chains without
  `emDefaultTouchVIF`.

### What's needed

- Add `emDefaultTouchVIF` as a field on the window (or in the VIF chain)
- Route winit `Touch` events to `touch_start`/`touch_move`/`touch_end`
- Call `cycle_gesture` each frame with actual frame delta in ms
- Add `emDefaultTouchVIF` to test pipeline VIF chains

---

## Gap 2: `ForwardInput` Action Is Not Implemented — Two-Finger Mouse Emulation Is Inert

### What's broken

When the gesture machine detects a two-finger directional gesture, it
transitions to `EmuMouse1-4` and pushes `GestureAction::ForwardInput`
actions with synthetic mouse button/modifier combinations. These actions
are drained by `drain_gesture_actions` but only logged — no input event
is actually forwarded through the VIF chain.

| Gesture | State | ForwardInput key | Modifiers |
|---------|-------|-----------------|-----------|
| Swipe right | `EmuMouse1` | `MouseLeft` | none |
| Swipe left | `EmuMouse2` | `MouseRight` | none |
| Swipe down | `EmuMouse3` | `MouseLeft` | shift |
| Swipe up | `EmuMouse4` | `MouseLeft` | ctrl |

Each EmuMouse state pushes Press on entry, Move while held, Release on
finger-up. All three are logged and discarded.

### Code locations

- **Action push sites** (in `TouchTracker::do_gesture`):
  - `emViewInputFilter.rs:1672` — SecondDown → EmuMouse press
  - `emViewInputFilter.rs:1703` — EmuMouse held → move
  - `emViewInputFilter.rs:1714` — EmuMouse released → release

- **Drain site** (log-only stub):
  `emViewInputFilter.rs:2166-2183` — `drain_gesture_actions` match arm for
  `GestureAction::ForwardInput`. Currently:
  ```rust
  dlog!("Touch gesture: forward input {:?} {:?} at ({:.0}, {:.0})", ...);
  let _ = (key, variant, mouse_x, mouse_y, shift, ctrl);
  ```

- **Missing infrastructure — `ForwardInput` method:**
  C++ `emViewInputFilter` has `ForwardInput(event, state)` (declared at
  `include/emCore/emViewInputFilter.h:90`) which passes a synthetic
  `emInputEvent` + `emInputState` down the VIF chain to the next filter.
  The Rust `emViewInputFilter` trait (`emViewInputFilter.rs`) has no
  equivalent. The trait's `filter` method receives events but there is no
  method to *inject* a synthetic event back into the chain.

- **C++ reference:** `emViewInputFilter.cpp:1091-1171` — The EmuMouse
  states construct `emInputEvent` objects with the synthetic key, set
  modifier state on `InputState`, and call `ForwardInput(InputEvent, InputState)`.

### What's needed

- Add a `ForwardInput` method (or equivalent) to the VIF trait or to
  `emDefaultTouchVIF` that can inject a synthetic `emInputEvent` +
  `emInputState` into the filter chain
- In `drain_gesture_actions`, construct the synthetic event from the
  `ForwardInput` action fields and call the injection method
- This requires access to the VIF chain or the next filter in the chain,
  which `drain_gesture_actions` does not currently have

---

## Gap 3: `InjectMenuKey` and `ToggleSoftKeyboard` Actions Are Not Implemented

### What's broken

Three-finger release pushes `GestureAction::InjectMenuKey`. Four-finger
release pushes `GestureAction::ToggleSoftKeyboard`. Both are logged and
discarded.

### Code locations

- **Action push sites** (in `TouchTracker::do_gesture`):
  - `emViewInputFilter.rs:1732` — ThirdDown all-up → `InjectMenuKey`
  - `emViewInputFilter.rs:1742` — FourthDown all-up → `ToggleSoftKeyboard`

- **Drain site** (log-only stubs):
  - `emViewInputFilter.rs:2158-2161` — InjectMenuKey:
    ```rust
    dlog!("Touch gesture: inject menu key");
    // TODO: emit Menu key press+release through input filter chain
    ```
  - `emViewInputFilter.rs:2162-2165` — ToggleSoftKeyboard:
    ```rust
    dlog!("Touch gesture: toggle soft keyboard");
    // TODO: view.show_soft_keyboard(!view.is_soft_keyboard_shown())
    ```

### Missing infrastructure

**InjectMenuKey** requires the same `ForwardInput` infrastructure as Gap 2.
C++ implementation (`emViewInputFilter.cpp:1184-1189`):
```cpp
InputState.Set(EM_KEY_MENU, true);
InputEvent.Setup(EM_KEY_MENU, emString(), 0, 0);
ForwardInput(InputEvent, InputState);
InputState.Set(EM_KEY_MENU, false);
ForwardInput(InputEvent, InputState);
```
This sends a Menu key press followed by release through the VIF chain.
`EM_KEY_MENU` maps to `InputKey::Menu` in Rust.

**ToggleSoftKeyboard** requires two methods on `emView` that do not exist:
- `IsSoftKeyboardShown() -> bool`
- `ShowSoftKeyboard(show: bool)`

C++ declares these at `include/emCore/emView.h:451-452` as virtual methods
that delegate to `CurrentViewPort`. The Rust `emView` struct
(`crates/emcore/src/emView.rs`) has no soft keyboard API. No file in
`crates/emcore/src/` contains "SoftKeyboard" or "soft_keyboard".

C++ implementation (`emViewInputFilter.cpp:1200`):
```cpp
GetView().ShowSoftKeyboard(!GetView().IsSoftKeyboardShown());
```

### What's needed

- **InjectMenuKey:** Same `ForwardInput` infrastructure as Gap 2, then
  construct press+release for `InputKey::Menu`
- **ToggleSoftKeyboard:** Add `IsSoftKeyboardShown() -> bool` and
  `ShowSoftKeyboard(show: bool)` to `emView` (delegating to a viewport
  or window flag), then call from `drain_gesture_actions`

---

## Gap 4: Dual State Machine Coexistence

### Current state

`emDefaultTouchVIF` runs two state machines simultaneously:

1. **Old 4-state** (`TouchState`): `Idle` → `SingleTouch` → `PinchZoom` → `Fling`
   - Handles: immediate single-finger pan, two-finger pinch zoom, fling
   - Located: `touch_start` (line 1919), `touch_move` (line 2009),
     `touch_end` (line 2050), `animate_fling` (line 2100)

2. **New 18-state** (`GestureState` via `gesture_tracker`):
   - Handles: scroll (after 20px dead zone), hold-to-zoom, tap sequences,
     two-finger emulation, multi-finger shortcuts
   - Wired: `run_gesture_loop` called from `touch_start`/`touch_move`/`touch_end`

A `gesture_handles_move` guard (`emViewInputFilter.rs:2002-2006`) prevents
double-scrolling when the gesture machine is in `Scroll`/`ZoomIn`/`ZoomOut`:
```rust
let gesture_handles_move = matches!(
    self.gesture_tracker.gesture_state,
    GestureState::Scroll | GestureState::ZoomIn | GestureState::ZoomOut
);
```

### Known behavioral quirks

- **First 20px of a drag:** The old system's `SingleTouch` scroll runs
  (immediate, no dead zone). After 20px the gesture machine transitions
  `FirstDown` → `Scroll` and takes over. The handoff is seamless because
  `prev_x`/`prev_y` sync ensures frame deltas are correct, but the
  behavior differs from C++ which has the 20px dead zone from the start.

- **Pinch zoom:** Always handled by the old system's `PinchZoom` state.
  The gesture machine's `SecondDown` state waits for 250ms timeout (which
  never fires — Gap 1) before transitioning to EmuMouse. So two-finger
  pinch works via the old system, not the gesture machine.

- **Fling:** Entirely old system. The gesture machine has no fling/inertia
  concept. `animate_fling` (line 2100) is independent of the gesture tracker.

### End state

When all gaps are closed, the old 4-state system should be evaluated for
removal. The gesture machine's `Scroll` state replaces `SingleTouch` pan,
and the 250ms-based states replace `PinchZoom`. Only `Fling` has no gesture
machine equivalent and may need to be kept or ported as a post-`Finish`
behavior.

---

## Summary: What Gestures Work vs Don't

| Gesture | Works? | Via | Blocker |
|---------|--------|-----|---------|
| Single-finger pan | Yes (immediate, no dead zone) | Old 4-state | — |
| Single-finger scroll (20px dead zone) | Yes (after 20px) | Gesture machine | — |
| Two-finger pinch zoom | Yes | Old 4-state | — |
| Fling after pan | Yes | Old 4-state | — |
| Hold-to-zoom (250ms) | **No** | Gesture machine | Gap 1: no `cycle_gesture` caller |
| Double-tap visit | **No** | Gesture machine | Gap 1: no `cycle_gesture` caller |
| Triple-tap toggle-visit | **No** | Gesture machine | Gap 1: no `cycle_gesture` caller |
| Two-finger mouse emulation | **No** | Gesture machine | Gap 1 + Gap 2: no caller + no ForwardInput |
| Three-finger menu key | **No** | Gesture machine | Gap 1 + Gap 3: no caller + no ForwardInput |
| Four-finger soft keyboard | **No** | Gesture machine | Gap 1 + Gap 3: no caller + no ShowSoftKeyboard API |

---

## Dependency Graph

```
Gap 1 (cycle_gesture caller)
  ├── emDefaultTouchVIF instantiation in emWindow
  ├── winit Touch event routing
  └── frame-loop integration

Gap 2 (ForwardInput infrastructure)
  ├── ForwardInput method on VIF trait or emDefaultTouchVIF
  ├── Synthetic emInputEvent construction
  └── VIF chain access from drain_gesture_actions

Gap 3 (InjectMenuKey + ToggleSoftKeyboard)
  ├── InjectMenuKey depends on Gap 2 (ForwardInput)
  └── ToggleSoftKeyboard depends on:
      ├── emView::IsSoftKeyboardShown() — does not exist
      └── emView::ShowSoftKeyboard(bool) — does not exist
```

All three gaps are prerequisites for the gesture machine to be fully
functional. Gap 1 is the most impactful (unblocks all time-based gestures).
Gap 2 unblocks two-finger emulation and menu key. Gap 3's soft keyboard
part requires new emView API surface.
