# emFileMan End-to-End Browsing Integration

## Objective

Wire the emFileMan panel layer into a working file browser: directories display entries, entries display content via plugins, users can select files and control sort/filter/theme settings. This closes the rendering loop and makes emFileMan functional.

## Scope

All 6 panel types in `crates/emfileman/src/` receive their missing PanelBehavior methods to reach full C++ API parity. The work is ordered by criticality: rendering loop first, then interaction, then control UI.

### In scope

1. **Rendering loop**: model-to-panel wiring, child panel lifecycle (create/delete based on view state)
2. **Selection & input**: Input() handlers for click/shift/ctrl selection, KeyWalk type-ahead
3. **Control panel**: full widget tree with radio buttons, checkboxes, command group buttons
4. **SelInfoPanel**: selection signal watching, detail reset on selection change
5. **CreateControlPanel**: emDirPanel and emDirEntryPanel return emFileManControlPanel when active

### Out of scope

- Command execution (`RunDefaultCommand`) — requires emProcess/shell integration not yet ported. `Input()` handlers will call `SelectSolely()` on double-click/Enter but will not invoke `RunDefaultCommand()`.
- IPC server integration in emFileManModel
- emFileManControlPanel's dynamic command `Group` tree — the recursive `Group : emRasterGroup` pattern (where each CommandNode spawns child groups/buttons via `AutoExpand`) requires runtime-dynamic widget creation from the CommandNode tree. The static controls (sort/filter/theme/selection buttons) are in scope; the dynamic command tree is not.

## Phase 1: Rendering Loop

### emDirPanel — SetFileModel connection

Current state: acquires `emDirModel` in `notice()` when viewed, but never connects it as a `FileModelState` to `file_panel`. The `VirtualFileState` therefore never progresses from `NoFileModel`. The panel currently treats `NoFileModel` as equivalent to `Loaded` for painting, which is a hack — it should show loading progress.

Change: `emDirPanel::Cycle()` must drive the `emDirModel` loading state machine and set `VirtualFileState` on `file_panel` accordingly. The current `emDirModel` is DIVERGED from C++ — it does not compose `emFileModel<T>` and does not implement `FileModelState` (it wraps `emDirModelData` directly and notes "SignalId and update_signal from the scheduler are not needed for the data-layer-only port").

Two options:
- (a) Make `emDirModel` implement `FileModelState` by adding the required fields (`FileState`, `SignalId`, progress). This is closer to C++ but requires modifying `emDirModel` to carry scheduler state it currently doesn't need.
- (b) Drive loading manually in `emDirPanel::Cycle()`: call `try_start_loading()`/`try_continue_loading()` each cycle, track loading state locally, and set `file_panel`'s `VirtualFileState` directly via a setter.

Use option (b): it avoids changing the existing `emDirModel` contract and keeps the panel in control of its loading lifecycle, which matches the current architecture where `emDirPanel` already calls `refresh_vir_file_state()` each cycle. Add a `set_vir_file_state()` method to `emFilePanel` if one doesn't exist, or track loading state via a field on `emDirPanel` (e.g., `loading_started: bool`).

The Cycle logic:
1. If `dir_model` is Some and not yet loaded: call `try_continue_loading()`, update `VirtualFileState` to `Loading { progress }` or `Loaded`
2. If loaded: call `update_children()`
3. If error: set `VirtualFileState` to `LoadError`

### emDirPanel — SortChildren

Current state: `update_children()` creates children but does not sort them.

Change: After creating children, sort using `emFileManViewConfig::CompareDirEntries()`. The C++ `SortChildren()` method uses the config's sort comparator. In Rust, this means collecting entries, sorting with the config comparator, and creating children in sorted order (rather than sorting after creation).

### emDirEntryPanel — UpdateContentPanel

Current state: `content_panel` and `alt_panel` fields exist but are never populated.

Change: Add `notice()` that triggers content/alt panel creation:

```
fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
    if flags intersects VIEW_CHANGED | SOUGHT_NAME_CHANGED | ACTIVE_CHANGED {
        self.update_content_panel(ctx, false, false);
        self.update_alt_panel(ctx, false, false);
    }
}
```

`UpdateContentPanel` logic (from C++):
- Create content panel if: viewed AND content area is clipped AND viewport width >= MinContentVW
- Delete content panel if: not in active path AND not viewed
- Content panel is created via `emFpPluginList::CreateFilePanel()` with the entry's path
- For directories: uses `FileStatMode::Directory`
- For regular files: uses `FileStatMode::Regular`
- Panel name is `CONTENT_NAME` ("")

`UpdateAltPanel` logic:
- Create alt panel if: sought by `ALT_NAME` AND viewed with sufficient viewport
- Delete alt panel if: not in active path AND not viewed
- Creates `emDirEntryAltPanel` with `alternative=1`

**Borrow safety note**: `notice()` does not receive `PanelCtx`, so child creation must be deferred. Options:
- (a) Use `drain_parent_invalidation()` to signal that children need updating, then create in `Cycle()` or `LayoutChildren()`
- (b) Track dirty flags and create children in `LayoutChildren()` (which receives `PanelCtx`)
- (c) Move creation logic into `Cycle()` gated by state flags set in `notice()`

Option (c) matches the C++ pattern most closely — C++ `Notice` sets flags, `Cycle` acts on them. However, Cycle does not currently receive view state. Option (b) is what `emDirEntryAltPanel` already does. Use option (b) for consistency: `notice()` sets dirty flags, `LayoutChildren()` creates/deletes children.

### emDirEntryAltPanel — notice() and Cycle()

Current state: creates children unconditionally in `LayoutChildren()`.

Change: Add `notice()` to set dirty flags based on viewing state. Add `Cycle()` to watch config changes and invalidate painting. Gate child creation in `LayoutChildren()` on viewing conditions (C++ checks `IsSoughtByName(ContentName)` and viewport width thresholds).

### emFileLinkPanel — UpdateDataAndChildPanel

Current state: `model` field exists but child panel is never created.

Change: Add `UpdateDataAndChildPanel()` method (called from `Cycle()` and `notice()`):

1. If not viewed (ViewCondition < 60.0): delete child panel, return
2. Read `model` to get `FullPath` via `GetFullPath()`
3. Determine `HaveDirEntryPanel` from model's `HaveDirEntry` flag
4. If child panel exists and type matches: update dir entry if needed
5. If child panel doesn't exist or type changed: delete old, create new
   - If `HaveDirEntryPanel`: create `emDirEntryPanel` with `emDirEntry::from_path(full_path)`
   - Else: create via `emFpPluginList::CreateFilePanel()`

## Phase 2: Selection & Input

### emDirEntryPanel.Input()

Add `Input()` implementation:

```rust
fn Input(&mut self, event: &emInputEvent, state: &PanelState, input_state: &emInputState) -> bool {
    match event.key {
        LEFT_BUTTON if event.is_double_click => {
            self.select_solely();
            // RunDefaultCommand deferred (out of scope)
            true
        }
        LEFT_BUTTON => {
            self.select(input_state.shift, input_state.ctrl);
            true
        }
        ENTER => {
            self.select_solely();
            true
        }
        SPACE => {
            self.select(input_state.shift, input_state.ctrl);
            true
        }
        _ => false
    }
}
```

### Selection helpers

Add to `emDirEntryPanel`:

```
fn select(&mut self, shift: bool, ctrl: bool)
```
- No modifiers: `ClearSourceSelection()`, `SwapSelection()`, `SelectAsTarget(path)`, `SetShiftTgtSelPath(path)`
- Ctrl: toggle `IsSelectedAsTarget` ↔ `DeselectAsTarget`/`SelectAsTarget`, set `ShiftTgtSelPath`
- Shift: range selection from `ShiftTgtSelPath` to current (requires knowing sibling panel order — walk parent's children between the two paths and select all as targets)

```
fn select_solely(&mut self)
```
- `ClearSourceSelection()`, `ClearTargetSelection()`, `SelectAsTarget(path)`

### emDirPanel.Input() — SelectAll and KeyWalk

Add `Input()`:
- `Alt+A`: calls `SelectAll()` which iterates visible children and calls `file_man.SelectAsTarget(path)` for each
- Other keys: `KeyWalk()` — type-ahead search

`KeyWalk` state machine:
- Accumulates typed characters with 1-second timeout
- On timeout or new character after timeout: resets search string
- Matches entry names case-insensitively
- `*` prefix enables wildcard/substring matching
- Scrolls to matching entry via `seek_child_by_name()`

Add `KeyWalkState` struct:
```rust
struct KeyWalkState {
    search: String,
    last_key_time: std::time::Instant,
}
```

### emDirEntryAltPanel.Input()

Mouse-only: if mouse is in `AltContentArea`, focus child content panel. Otherwise pass to parent.

## Phase 3: Control Panel

### emFileManControlPanel — full widget tree

Replace the placeholder stub with actual widget construction.

Constructor builds:
- `emLinearLayout` (vertical) as the layout manager
- `GrView` group containing:
  - Sort criterion `RadioGroup` with 6 `emRadioButton`: "Name", "Ending", "Class", "Version", "Date", "Size"
  - Name sorting style `RadioGroup` with 3 `emRadioButton`: "Per Locale", "Case Sensitive", "Case Insensitive"
  - `emCheckButton` "Sort Directories First"
  - `emCheckButton` "Show Hidden Files"
  - Theme style `RadioGroup` (built from `emFileManThemeNames::GetThemeStyleCount()`)
  - Theme aspect ratio `RadioGroup` (built from `emFileManThemeNames::GetThemeAspectRatioCount()`)
  - `emCheckButton` "Autosave"
  - `emButton` "Save"
- `GrSelection` group containing:
  - `emButton` "Select All"
  - `emButton` "Clear Selection"
  - `emButton` "Swap Selection"
  - `emButton` "Paths to Clipboard"
  - `emButton` "Names to Clipboard"

Struct fields:
```rust
pub struct emFileManControlPanel {
    ctx: Rc<emContext>,
    config: Rc<RefCell<emFileManViewConfig>>,
    file_man: Rc<RefCell<emFileManModel>>,
    theme_names: Rc<RefCell<emFileManThemeNames>>,
    layout: emLinearLayout,
    // Radio groups
    sort_group: Rc<RefCell<RadioGroup>>,
    name_sort_group: Rc<RefCell<RadioGroup>>,
    theme_style_group: Rc<RefCell<RadioGroup>>,
    theme_ar_group: Rc<RefCell<RadioGroup>>,
    // Check buttons (tracked for state sync)
    dirs_first_checked: bool,
    show_hidden_checked: bool,
    autosave_checked: bool,
}
```

`Cycle()` watches config change signal and syncs radio/check state. Widget callbacks update config:
- Sort radio → `config.SetSortCriterion()`
- Name sort radio → `config.SetNameSortingStyle()`
- DirFirst checkbox → `config.SetSortDirectoriesFirst()`
- ShowHidden checkbox → `config.SetShowHiddenFiles()`
- Theme radios → `config.SetThemeName(theme_names.GetThemeName(style, ar))`
- Autosave → `config.SetAutosave()`
- Save button → `config.SaveAsDefault()`
- SelectAll → find active emDirPanel, call `SelectAll()`
- Clear/Swap/Clipboard → `file_man.ClearTargetSelection()` / `SwapSelection()` / `SelectionToClipboard()`

Constructor takes `content_view` reference (for finding active DirPanel for SelectAll).

### CreateControlPanel integration

Add to `emDirPanel` and `emDirEntryPanel`:
```rust
fn CreateControlPanel(&mut self, parent_ctx: &mut PanelCtx, name: &str) -> Option<PanelId> {
    let panel = emFileManControlPanel::new(Rc::clone(&self.ctx), /* content_view */);
    Some(parent_ctx.create_child_with(name, Box::new(panel)))
}
```

## Phase 4: SelInfoPanel Completion

### Selection signal watching

Current `Cycle()` just calls `work_on_details()`. Change: check selection generation counter (or signal). When selection changes, call `reset_details()` (rename from `_reset_details()`).

`emFileManModel` already has `selection_generation: Rc<Cell<u64>>` with `GetSelectionSignal() -> u64` and `bump_selection_generation()` called in all mutating methods.

### Reset on change

Add `last_selection_gen: u64` field to `emFileManSelInfoPanel`. In `Cycle()`:

```rust
fn Cycle(&mut self, _ctx: &mut PanelCtx) -> bool {
    let fm = self.file_man.borrow();
    let gen = fm.GetSelectionSignal();
    drop(fm);
    if gen != self.last_selection_gen {
        self.last_selection_gen = gen;
        self.reset_details();
    }
    self.work_on_details()
}
```

## File Changes Summary

| File | Changes |
|------|---------|
| `emDirPanel.rs` | SetFileModel wiring, SortChildren, Input() with SelectAll + KeyWalk, CreateControlPanel |
| `emDirEntryPanel.rs` | notice() with dirty flags, UpdateContentPanel, UpdateAltPanel, Input() with Select/SelectSolely, CreateControlPanel |
| `emDirEntryAltPanel.rs` | notice() view gating, Cycle() config watching, Input() focus forwarding |
| `emFileLinkPanel.rs` | UpdateDataAndChildPanel, child creation/deletion in Cycle |
| `emFileManControlPanel.rs` | Full rewrite: widget tree with RadioGroups, CheckButtons, Buttons, Cycle state sync |
| `emFileManSelInfoPanel.rs` | Selection signal detection, reset_details rename/call, generation counter |
| `emFileManModel.rs` | No changes needed — `selection_generation` and `GetSelectionSignal()` already exist |

## Dependencies

- `emFpPluginList::CreateFilePanel()` — already ported
- `emLinearLayout`, `emRadioButton`, `emCheckButton`, `emButton` — all ported in emCore
- `emRasterGroup`, `emPackGroup` — ported in emCore
- `emFileManThemeNames` — ported, provides theme style/AR enumeration
- `PanelBehavior::Input()`, `CreateControlPanel()` — defined in trait

## Testing Strategy

- Unit tests for selection logic (Select, SelectSolely, range selection)
- Unit tests for KeyWalk state machine (timeout, wildcard, matching)
- Integration test: create emDirPanel with a temp directory containing known files, verify child count matches after Cycle
- Control panel: verify RadioGroup callbacks update config state
- SelInfoPanel: verify reset_details clears state and generation tracking works
