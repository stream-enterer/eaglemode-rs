# FileSelectionBox Audit Report

**Date**: 2026-03-18 (Session 2)
**C++ files**: emFileSelectionBox.cpp (1217 LOC) + emFileSelectionBox.h (403 LOC) = 1620 LOC
**Rust file**: file_selection_box.rs (665 LOC)

## Completeness: ~40% — Structural shell only

## What Works (16 of 39 checked items)
- Data model (all state fields, getters/setters)
- Filter pattern matching (exact port of C++ glob logic)
- Directory listing reload (simplified but functional)
- Child panel creation with correct 3-zone layout geometry
- Border and paint pass-through
- ".." entry at non-root
- Multi-selection flag and selected names storage

## What's Missing (20 GAPs, 51% of functionality)

### [HIGH] Entire reactive/event layer missing — **FIXED**
- Implemented panel-cycle infrastructure: PanelBehavior::cycle() runs each frame for registered panels.
- Added Rc<RefCell<FsbEvents>> shared state between callbacks and cycle().
- Wired on_selection, on_trigger, on_text, on_check callbacks on all child panels.
- FSB::cycle() follows C++ Cycle() algorithm: directory field polling, hidden toggle, listing reload, selection sync, trigger handling, name field path resolution, filter changes.
- Consumer callbacks: on_selection, on_trigger exposed for parent/dialog wiring.

### [HIGH] FileItemPanel missing entirely (~280 LOC in C++) — **FIXED**
- Implemented FileItemPanelBehavior as inner type matching C++ nested FileItemPanel.
- Paint faithfully ports C++ lines 958-1062: selection highlight (round rect with inset/radius), filename text, directory icon (colored rect with folder tab, 310:216 aspect ratio), "Parent Directory" overlay for ".." entries, not-readable indicator (ellipse + diagonal line).
- Added ItemBehaviorFactory to ListBox for custom item panel creation.
- FSB wires factory via shared listing_data so each item gets correct is_directory/is_readable metadata.

### [MEDIUM] No interactive directory navigation — **FIXED**
- ListBox on_trigger callback wired to FsbEvents. cycle() handles triggered_index: if directory or "..", calls enter_sub_dir() then syncs dir field. If file, sets triggered_file_name and fires on_trigger callback.
- Directory TextField on_text callback wired: typing a path updates parent_dir and invalidates listing.

### [MEDIUM] No name field sync — **FIXED**
- Bidirectional sync implemented: selection_from_list_box() copies indices→names, sync_name_field() pushes first selected name to TextField.
- Name field on_text callback detects path separators (/ or \) → resolves via set_selected_path(), syncs both fields. Plain names → set_selected_name().

### [LOW] Locale-aware sort missing (strcoll → str::cmp) — **FIXED**
- Sort now uses libc::strcoll via FFI (CString conversion), matching C++ CompareNames exactly. libc was already a dependency. Also added directories-first grouping to the sort comparator.

### [LOW] set_filters doesn't update existing child ListBox — **FIXED**
- set_filters now sets children_dirty flag. layout_children detects dirty flag, tears down and recreates all children with fresh state. Filter ListBox gets updated items.
### [LOW] set_multi_selection_enabled doesn't update existing ListBox type — **FIXED**
- set_multi_selection_enabled now sets children_dirty flag. Recreated file ListBox gets correct SelectionMode.
### [LOW] No AutoShrink (Options never cleared) — **FIXED**
- Resolved by the children_dirty rebuild mechanism: create_children always starts with fresh ListBox instances, so no stale options accumulate.

## Summary

| Category | Match | GAP |
|----------|-------|-----|
| Data model | 90% | 10% |
| Filters | 85% | 15% |
| Navigation | 45% | 55% |
| Multi-selection | 25% | 75% |
| Signals | 0% | 100% |
| Child panels | 60% | 40% |
| Event handling | 0% | 100% |

## Overall: This is a data-model-and-layout scaffold. The widget creates correct visual structure but cannot respond to any user interaction. To reach parity, it needs: Cycle() or callback-based event processing, FileItemPanel, signal/callback wiring, and bidirectional selection sync.
