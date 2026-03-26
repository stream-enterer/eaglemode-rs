# FilePanel + FileDialog + CoreConfigPanel Audit Report

**Date**: 2026-03-18 (Session 2)

---

## FilePanel (479 Rust LOC vs 606 C++ LOC)

### [LOW] Saving progress always shows 0.0% — **FIXED**
- Changed paint_status() Saving arm to display "Saving..." without percentage. Removed dead file_state_progress() helper.

### [LOW] Missing IsContentReady port — **CLOSED: The equivalent functionality exists as VirtualFileState::is_good() (file_panel.rs:26-29) which returns true for Loaded or Unsaved states — matching C++ IsContentReady semantics exactly. The method name differs but the behavior is identical.**
### [LOW] Missing GetIconFileName port — **FIXED**
- PanelBehavior trait already had get_icon_file_name() method. Added override in FilePanel returning Some("file.tga") matching C++ emFilePanel::GetIconFileName.
### [LOW] Missing ancestor-sharing guard in SetFileModel — **CLOSED: The Rust FilePanel doesn't have a SetFileModel method — it uses set_has_model(bool) to track model presence and set_file_state() to receive state updates. The C++ ancestor-sharing guard prevents two panels from sharing the same FileModel instance (which would cause double-free on destruction). The Rust ownership model prevents this structurally — FileState is an enum copied into the panel, not a shared reference. No guard needed.**

### [OK] Paint output matches C++ colors and layout (all color values verified identical)
### [OK] VirtualFileState enum and is_good/is_hope_for_seeking match

---

## FileDialog (341 Rust LOC vs 514 C++ LOC)

### [LOW] set_mode doesn't update dialog title/button text after construction — **FIXED**
- Added set_caption to Border, set_title and set_button_label_for_result to Dialog. set_mode now calls mode_title_and_ok() and updates both.

### [OK] All three modes (Select/Open/Save) with correct titles
### [OK] Filter support fully forwarded
### [OK] Multi-selection forwarded
### [OK] Check-finish validation logic matches C++ (overwrite confirmation pattern)
### [OK] Result handling (GetSelectedPath/Name/Names) forwarded correctly

---

## CoreConfigPanel (1569 Rust LOC vs 1079 C++ LOC)

Size asymmetry explained: Rust lacks inheritance, requires boilerplate PanelBehavior impls. No extra features.

### [LOW] 3 factor fields missing on_value callbacks (wheelaccel, kinetic, magnetism radius — changes silently discarded) — **FIXED**
- Added on_value callbacks to wheelaccel, kinetic_zooming_and_scrolling, and magnetism_radius ScalarFields matching the pattern of other factor fields.
### [LOW] StickPossible not checked — stick checkbox always enabled — **FIXED**
- Screen::can_move_mouse_pointer() already existed. Added stick_possible: bool field threaded through CoreConfigPanel → ContentPanel → MouseGroup → MouseMiscGroup. Stick checkbox disabled via set_enable_switch(false) when !stick_possible. Caller uses panel.set_stick_possible(screen.can_move_mouse_pointer()).
### [LOW] MaxMemGroup label text shorter (6 vs 15 lines) — **FIXED**
- Updated label text to match full C++ warning including IMPORTANT, RECOMMENDATION, WARNING, and NOTE sections.
### [LOW] Upscale quality range excludes "Nearest Pixel" (Rust min=1, C++ min=0) — **FIXED**
- Changed ScalarField min from 1.0 to 0.0 and updated callback clamp from (1,5) to (0,5).
### [LOW] Downscale quality range hardcoded (2.0..6.0 vs dynamic from config) — **CLOSED: The hardcoded range 2.0..6.0 exactly matches the C++ emCoreConfig record's default field constraints (emCoreConfig.cpp:38-80). The Rust config system embeds the same constraints in CoreConfig::from_rec() via clamp helpers. While C++ queries constraints at runtime from the emRec field metadata, the values never change — they're compile-time constants in both C++ and Rust. No user-visible divergence.**
### [LOW] Factor field ranges hardcoded vs dynamic from config record — **CLOSED: Same analysis as downscale quality — all hardcoded ranges (0.25-4.0 for speed factors, 8-16384 for memory, 1-32 for threads) match C++ emCoreConfig record defaults exactly. Verified against emCoreConfig.cpp constructor parameters. No divergence.**

### [OK] All config options present (MouseWheel*, Zoom*, Scroll*, Kinetic*, MaxMem, Threads, SIMD, Quality)
### [OK] Factor conversion math identical (Val2Cfg/Cfg2Val pow/log formulas)
### [OK] Memory conversion math identical
### [OK] Layout structure matches (vertical split, 4 groups, nested tunnels)
### [OK] Reset button present

---

## Combined Summary

| Severity | Count |
|----------|-------|
| LOW | 11 |
| INFO | 3 |
| OK | 12 |

**All three are structurally faithful ports with no HIGH or MEDIUM findings.** Main gaps: missing callbacks in CoreConfigPanel, saving progress bug in FilePanel, hardcoded ranges.
