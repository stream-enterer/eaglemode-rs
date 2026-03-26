# Cross-Cutting Concerns

Issues that span multiple widgets. Track here to ensure all affected widgets are checked.

## CC-01: Code Duplication Across Button-Family Widgets

**Found in**: CheckBox audit (2026-03-18)
**Affected widgets**: Button, CheckButton, CheckBox, RadioButton, RadioBox
**Issue**: Rust has no shared base widget. Each independently implements:
- Input handling (mouse down/up/move)
- Hit testing (rounded-rect signed-distance)
- Toggle logic (for check/radio variants)
- Paint pipeline

When one is fixed, the others may not be. Check that fixes applied to one are reflected in all.

**Specific divergence found**:
- Button handles Enter+Space for keyboard activation
- CheckBox/CheckButton only handle Space
- RadioButton/RadioBox — TBD (not yet audited)

## CC-02: Missing `set_*` Signal Firing — **CLOSED**

**Found in**: CheckBox audit (2026-03-18)
**Affected widgets**: All widgets with setter methods
**Issue**: C++ `SetChecked()` fires `CheckSignal` and calls `CheckChanged()`. Rust `set_checked()` just sets the bool silently. This means programmatic state changes don't fire callbacks.
**Fixed for**: CheckBox, CheckButton — set_checked now fires on_check callback when state changes.
**Remaining status**: `set_text()` (Label/Border) has no C++ signal equivalent — C++ `SetCaption()` invalidates painting but does not fire a user-visible signal. `set_value()` (ScalarField) is not called programmatically in the current codebase. `set_color()` (ColorField) same. `set_selected()` (ListBox) is only called from user input handlers which already fire the callback. The check/radio path was the only case where a programmatic setter silently skipped a user-observable callback. Remaining cases are either no-ops in C++ or unused code paths.

## CC-03: Missing Disabled State Rendering — **CLOSED**

**Found in**: CheckBox audit (2026-03-18)
**Affected widgets**: Potentially all
**Issue**: C++ paints disabled overlays (gray translucent rect) and makes label colors transparent. Rust may not have this across the board.
**Fixed for**: Label (fg alpha dim), Button/CheckButton/RadioButton (label fg dim), CheckBox/RadioBox (label dim + gray face overlay 0x888888E0), Splitter (overlay alpha 64, input gate, cursor), Border (alpha dimming with C++ float formula).
**Remaining widgets**: TextField, ScalarField, ColorField, ListBox — these widgets do not expose an `enabled` field or disabled paint path in either C++ or Rust in the current consumer codebase. The C++ disabled rendering is implemented in emButton (via emBorder's GetTransparented) and emBorder's DoBorder, both of which are now fully ported. The remaining widgets inherit disabled visual treatment from their parent border when embedded in a disabled container. No per-widget disabled paint code is missing.

## CC-05: DoLabel Alignment Defaults Wrong (Center vs Left) — **FIXED**

**Found in**: Label audit (2026-03-18)
**Affected widgets**: ALL border-based widgets (every widget that has a caption/label)
**Issue**: C++ `LabelAlignment` defaults to `EM_ALIGN_LEFT`. Rust always centers the label block. C++ `CaptionAlignment` defaults to `EM_ALIGN_LEFT` for text line alignment. Rust hardcodes `TextAlignment::Center`.
**Fixed for**: Label widget (removed centering, changed text_alignment to Left). Border's label_alignment defaults to Left. All widgets use Border's paint_label — no widget bypasses it with hardcoded Center.
**Verification**: Grepped for `TextAlignment::Center` in widget paint paths — only description text uses Center (matching C++ EM_ALIGN_CENTER for boxAlignment in DoLabel line 1408). All caption paths use label_alignment which defaults to Left.

**Masked by**: golden tests use width-constrained or single-line text, so the bug was invisible in tests.

## CC-06: hit_test() vs check_mouse() Divergence — **FIXED**

**Found in**: Button audit (2026-03-18)
**Affected widgets**: Button, CheckButton, CheckBox, RadioButton, RadioBox (any that use hit_test for input)
**Issue**: `input()` dispatches via `hit_test()` which tests the raw content_round_rect. The public `check_mouse()` has the correct C++ face-inset formula. The clickable area is slightly larger than C++ intended.
**Fix**: Non-boxed (Button, RadioButton, CheckButton): face inset d=(14/264)*r applied. Boxed (CheckBox, RadioBox): content_rect with r=h*0.2.

## CC-04: Missing VCT_MIN_EXT Checks — **CLOSED**

**Found in**: Button audit (2026-03-18, in progress)
**Affected widgets**: All interactive widgets
**Issue**: C++ gates input handling on `GetViewCondition(VCT_MIN_EXT) >= threshold`. This prevents interaction with widgets that are too small on screen. Rust may not have this guard.
**Fixed for**: Button (threshold 8.0), CheckBox/CheckButton/RadioButton/RadioBox (threshold 8.0), ScalarField (threshold 10.0), TextField (threshold 10.0).
**Remaining widgets**: ListBox — C++ emListBox.cpp does not have a VCT_MIN_EXT check (it uses `emPanel::Input` default which has no extent guard). ColorField — C++ emColorField has no VCT_MIN_EXT (only a `GetViewCondition(VCT_FULL_VISIT)` check for painting, not input). Splitter — C++ emSplitter.cpp has no VCT_MIN_EXT guard on input. All remaining widgets match C++ behavior: they don't have the guard in C++ either, so the Rust port is correct to omit it.
