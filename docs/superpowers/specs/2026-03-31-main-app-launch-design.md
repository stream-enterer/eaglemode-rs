# Design: Eagle Mode Main App Launch

**Date:** 2026-03-31
**Goal:** Get the `eaglemode` binary running with full fidelity to the C++ emMain module, using only the already-ported app plugins (emStocks, emFileMan/emDirPanel).

## Scope

Port the entire C++ `emMain` module (10 files) to Rust, complete the emFileMan rendering layer (4 deferred panel lifecycle methods), ship cosmos items for ported plugins, and produce a running binary that boots like the C++ version — main window, starfield cosmos, control panel sidebar, bookmarks, config persistence, IPC single-instance, startup animation.

**Deferred work** documented separately in `docs/superpowers/gaps/2026-03-31-deferred-from-main-app.md`.

## Architecture

### Boot Sequence

1. Parse CLI args: `-geometry WxH+X+Y`, `-fullscreen`, `-visit <path>`, `-noclient`, `-noserver`, `-cecolor <color>`
2. Try `emMiniIpcClient::TrySend("eaglemode", args)` → if server responds, exit (window opened in existing process)
3. If no server or `-noclient`: create `emGUIFramework`, start event loop
4. `emMain` engine registers `emMiniIpcServer` with name `"eaglemode"`, creates first `emMainWindow`
5. `emMainWindow` creates `emMainPanel` (split layout) + detached control window
6. Content side: `emMainContentPanel` → `emVirtualCosmosPanel` → starfield + cosmos items
7. Control side: `emMainControlPanel` with buttons, bookmarks, config
8. `StartupEngine` runs choreographed zoom-in animation (~2 seconds)

### Panel Tree

```
emMainWindow
├── emMainPanel (split: control | slider | content)
│   ├── ControlViewPanel (emSubViewPanel)
│   │   └── emMainControlPanel
│   │       ├── Window buttons (New, Fullscreen, Reload, Close, Quit)
│   │       ├── Auto-hide toggles
│   │       └── emBookmarksPanel (bookmark buttons + groups)
│   ├── Slider (draggable divider)
│   └── ContentViewPanel (emSubViewPanel)
│       └── emMainContentPanel (logo drawing)
│           └── emVirtualCosmosPanel
│               ├── emStarFieldPanel (fractal background)
│               └── emVirtualCosmosItemPanel × N (one per .emVcItem)
│                   └── content panel (via emFpPluginList::CreateFilePanel)
└── StartupOverlay (fades out after animation)
```

### Detached Control Window

`emMainWindow` creates a second `emWindow` for the floating sidebar. This is the C++ behavior — the control panel can be in a separate OS window. The `emMainControlPanel` is the same panel, just hosted in a different `emSubViewPanel` depending on whether the control window is detached or embedded.

## New Crate: `crates/emmain/`

### Files (1:1 with C++ emMain headers)

| File | C++ Source | Role |
|---|---|---|
| `emMain.rs` | `emMain.cpp` | IPC server engine, `NewWindow()`, arg dispatch |
| `emMainWindow.rs` | `emMainWindow.h` | Window lifecycle, control window, startup engine |
| `emMainPanel.rs` | `emMainPanel.h` | Control/content split with draggable slider |
| `emMainContentPanel.rs` | `emMainContentPanel.h` | Content container, Eagle Mode logo |
| `emMainControlPanel.rs` | `emMainControlPanel.h` | Sidebar buttons, auto-hide, bookmark buttons |
| `emVirtualCosmos.rs` | `emVirtualCosmos.h` | 3 classes: model, panel, item panel |
| `emStarFieldPanel.rs` | `emStarFieldPanel.h` | Fractal starfield with recursive sub-panels |
| `emBookmarks.rs` | `emBookmarks.h` | `emBookmarkRec`, `emBookmarksModel`, `emBookmarksPanel` |
| `emMainConfig.rs` | `emMainConfig.h` | Config record (auto-hide, slider, colors) |
| `emAutoplay.rs` | `emAutoplay.h` | `emAutoplayConfig`, `emAutoplayViewModel`, animator |
| `lib.rs` | — | Module declarations and re-exports |

### Dependencies

```toml
[dependencies]
emcore = { path = "../emcore" }
emstocks = { path = "../emstocks" }
emfileman = { path = "../emfileman" }
```

The `eaglemode` crate (`crates/eaglemode/`) depends on `emmain` and becomes the binary entry point.

## Component Details

### emMain (IPC server + window factory)

An `emEngine` that:
- Owns `emMiniIpcServer` with server name `"eaglemode"`
- On IPC message: parses args, calls `NewWindow()` with geometry/fullscreen/visit overrides
- `NewWindow()` creates `emMainWindow`, adds to window list
- Owns `emSigModel` for reload signaling (external tools trigger config reload)

### emMainWindow (window + startup animation)

- Creates OS window via `emWindow::new()` with `emWindowStateSaver` for geometry persistence
- Creates `emMainPanel` as root panel
- Creates separate `emWindow` for detached control panel
- `StartupEngine`: `emEngine` that runs ~2 seconds after launch, drives choreographed zoom from overview to default visit target. Phases: fade-in → zoom → settle.
- Owns shared `emBookmarksModel` and `emAutoplayViewModel`

### emMainPanel (split layout)

- Three children: `ControlViewPanel` (left), `Slider` (middle), `ContentViewPanel` (right)
- `ControlViewPanel` and `ContentViewPanel` are `emSubViewPanel`s with independent zoom/pan
- Slider responds to mouse drag, resizes control/content split
- Reads `emMainConfig` for initial split ratio and auto-hide state
- Auto-hide: collapses control view when mouse leaves and auto-hide is enabled

### emMainContentPanel (content container)

- Draws Eagle Mode logo at root zoom level
- Creates `emVirtualCosmosPanel` as single child
- Pure layout/decoration

### emVirtualCosmos (3 classes in one file)

**emVirtualCosmosModel:**
- Loads `.emVcItem` files from `emGetConfigDirOverloadable("emMain", "VcItems")`
- Parses each via `emRec` into `emVirtualCosmosItemRec` (PosX, PosY, Width, ContentTallness, BorderScaling, BackgroundColor, BorderColor, TitleColor, FileName, CopyToUser, Alternative, Focusable)
- Watches directory for changes, reloads on modification
- Change signal for panels

**emVirtualCosmosPanel:**
- Creates/destroys `emVirtualCosmosItemPanel` children as model changes
- Creates `emStarFieldPanel` as background child
- Maps item positions to panel layout coordinates

**emVirtualCosmosItemPanel:**
- Renders border, title, background color per item record
- On auto-expand (viewport large enough): calls `emFpPluginList::CreateFilePanel()` with item's filename
- Content panel created lazily, destroyed when zoomed out

### emStarFieldPanel (fractal starfield)

- Deterministic pseudo-random star generation based on grid position
- Recursive: creates 4 child `emStarFieldPanel`s (quadrants) when zoomed in
- Each depth level has smaller, dimmer stars
- Paints via `emPainter::PaintRect` with computed colors/positions
- Contains tic-tac-toe Easter egg from C++

### emBookmarks

**emBookmarkRec:** `emStructRec` with fields: name, description, icon, hotkey, location identity (panel path + relative coordinates), background/text colors. Groups contain children recursively via `emArrayRec<emBookmarkRec>`.

**emBookmarksModel:** `emConfigModel` loading `~/.eaglemode/emMain/Bookmarks.emBookmarks`. Provides bookmark tree.

**emBookmarksPanel:** Renders bookmark buttons in sidebar. Each button triggers `emVisitingViewAnimator` to navigate. Groups render as expandable sections.

### emMainControlPanel (sidebar)

`emLinearGroup` containing:
- Window control buttons (New Window, Fullscreen, Reload, Close, Quit)
- Auto-hide toggles (control view, slider)
- `emBookmarksPanel` section
- Buttons wire to `emMainWindow` methods or config toggles

### emMainConfig

`emConfigModel` with `emStructRec` fields:
- `AutoHideControlView` (bool)
- `AutoHideSlider` (bool)
- `ControlViewSize` (double — split ratio)

### emAutoplay

**emAutoplayConfig:** Record with duration, recursive flag, loop flag.
**emAutoplayViewModel:** `emViewAnimator` that traverses panels automatically — visits child panels in sequence with configurable timing.

## emFileMan Completion

4 deferred panel lifecycle methods must be wired before the main app works:

| Method | Purpose | Depends On |
|---|---|---|
| `emDirPanel` model wiring | Call `emDirModel::try_continue_loading()` from `Cycle()`, set `VirtualFileState` | emDirModel (done) |
| `emDirEntryPanel::UpdateContentPanel` | When zoomed past threshold, call `emFpPluginList::CreateFilePanel()` for child panel | emFpPluginList (done) |
| `emDirEntryPanel::UpdateAltPanel` | Create `emDirEntryAltPanel` for alternative view when content panel exists | UpdateContentPanel |
| `emFileLinkPanel::UpdateDataAndChildPanel` | Load `emFileLinkModel`, resolve target path, create child panel | emFileLinkModel (done) |

Plus: Shift-click range selection in `emDirEntryPanel` (iterate sibling panels from panel tree).

**Execution order:** Model wiring → content panel creation → alt panel → link panel → range selection.

## Config Files

### Cosmos Items (`etc/emMain/VcItems/`)

Only items whose plugins are ported. Positions preserved from C++ defaults:

| Item File | Content | Plugin |
|---|---|---|
| `Home.emVcItem` | `~` (directory) | emDirFpPlugin |
| `Root.emVcItem` | `/` (directory) | emDirFpPlugin |
| `Stocks1.emVcItem` | `Stocks1.emStocks` | emStocksFpPlugin |

Additional directory-type items from C++ defaults included if they reference directories.

### Plugin Registration (`etc/emCore/FpPlugins/`)

`.emFpPlugin` files for:
- `emDir.emFpPlugin` — directories → `emDirFpPluginFunc`
- `emStocks.emFpPlugin` — `.emStocks` → `emStocksFpPluginFunc`
- `emFileMan.emFpPlugin` — file manager → `emFileManFpPluginFunc`
- `emDirStat.emFpPlugin` — directory stats → `emDirStatFpPluginFunc`
- `emFileLink.emFpPlugin` — symlinks → `emFileLinkFpPluginFunc`

### Default Bookmarks

Ship initial `Bookmarks.emBookmarks` with at minimum a "Home" bookmark pointing at the home directory cosmos item.

### User Config Locations

Per `emInstallInfo::emGetConfigDirOverloadable()`:
- System defaults: `<install>/etc/emMain/`, `<install>/etc/emCore/`
- User overrides: `~/.eaglemode/emMain/`, `~/.eaglemode/emCore/`
- Persisted state: `~/.eaglemode/emMain/MainConfig.rec`, `~/.eaglemode/emMain/Bookmarks.emBookmarks`

## Static Plugin Linking

For this initial launch, plugins are statically linked. `emFpPluginList` gets a static registry fallback:

```rust
// In emmain or eaglemode crate
fn static_plugin_registry(name: &str) -> Option<emFpPluginFunc> {
    match name {
        "emDirFpPluginFunc" => Some(emfileman::emDirFpPluginFunc),
        "emStocksFpPluginFunc" => Some(emstocks::emStocksFpPluginFunc),
        "emFileManFpPluginFunc" => Some(emfileman::emFileManFpPluginFunc),
        "emDirStatFpPluginFunc" => Some(emfileman::emDirStatFpPluginFunc),
        "emFileLinkFpPluginFunc" => Some(emfileman::emFileLinkFpPluginFunc),
        _ => None,
    }
}
```

`emFpPlugin::TryCreateFilePanel()` checks the static registry before falling back to dynamic `emTryResolveSymbol`.

## Testing Strategy

### Unit Tests (per module)
- `emVirtualCosmosModel`: parse `.emVcItem` files, verify item records
- `emBookmarkRec`: round-trip serialization
- `emMainConfig`: defaults, load/save
- `emStarFieldPanel`: deterministic stars at known positions
- `emAutoplay`: state machine transitions

### Integration Tests (headless TestHarness)
- `emMainPanel`: panel tree structure, control/content split, slider drag
- `emDirPanel` lifecycle: model loading → child panel creation → content panel via plugin
- Plugin resolution: `emFpPluginList` finds correct plugin for `.emStocks` and directories
- IPC: server ↔ client round-trip

### Manual Smoke Test
- `cargo run` → window with cosmos, starfield, sidebar
- Browse filesystem via directory items, open stocks
- Bookmarks work, window state persists across restarts
- Second `cargo run` → IPC to existing instance opens new window

## What's NOT in Scope

See `docs/superpowers/gaps/2026-03-31-deferred-from-main-app.md` for full list:
- No new viewer plugins (text, image, etc.)
- No new golden tests for emMain panels
- No dynamic plugin `.so` loading
- No touch/mobile-specific work
- No unported app plugins (clock, games, fractal, etc.)
