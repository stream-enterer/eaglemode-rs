# Gap: Deferred Work from Main App Launch

**Filed:** 2026-03-31
**Affects:** Full Eagle Mode feature parity
**Severity:** Low-Medium — app is functional without these, but they're needed for complete parity

## Context

The 2026-03-31 main app launch design intentionally excludes the following work to keep scope focused on getting eaglemode running with the ported apps (emStocks, emFileMan, emDirPanel). This document tracks everything deferred so nothing is lost.

## Deferred Items

### 1. Viewer Plugins (No Ported Plugins for Common File Types)

The C++ Eagle Mode ships 30+ file format plugins. None of these are ported. Navigating the filesystem shows "unsupported format" for any non-directory, non-.emStocks file.

**Plugins not yet ported (grouped by priority):**

**High value (basic browsing):**
- `emText` — plain text viewer (`.txt`, `.log`, `.conf`, etc.)
- `emPng` — PNG image viewer
- `emJpeg` — JPEG image viewer

**Medium value (common formats):**
- `emBmp` — BMP images
- `emGif` — GIF images
- `emSvg` — SVG vector graphics
- `emPdf` — PDF viewer
- `emJson` — JSON viewer
- `emWebp` — WebP images

**Low value (niche formats):**
- `emTga` — TGA images
- `emPcx` — PCX images
- `emPnm` — PNM images
- `emRas` — RAS images
- `emRgb` — RGB images
- `emTiff` — TIFF images
- `emIlbm` — ILBM images
- `emXbm` — XBM images
- `emXpm` — XPM images

**C++ source:** `~/git/eaglemode-0.96.4/include/em{Text,Png,Jpeg,...}/`

### 2. App Plugins (Interactive Applications)

The C++ cosmos includes interactive apps. None are ported.

- `emClock` — analog/digital clock
- `emMines` — Minesweeper game
- `emNetwalk` — network puzzle game
- `SilChess` — chess game
- `emFractal` — fractal explorer
- `emHmiDemo` — HMI demonstration

**C++ source:** `~/git/eaglemode-0.96.4/include/em{Clock,Mines,Netwalk,...}/`

### 3. Dynamic Plugin Loading

The current design uses static linking — all plugin functions are compiled into the binary and registered via a static lookup table. The C++ version loads `.so` libraries at runtime from `.emFpPlugin` config files.

**What's needed later:**
- `emFpPluginList` reads `.emFpPlugin` files (already implemented)
- `emTryResolveSymbol` does dynamic symbol lookup (already implemented)
- Wire these together so plugins can be separate `.so` crates loaded at runtime
- Enables third-party plugins and hot-reloading

### 4. Audio/Video Support

- `emAv` — audio/video player plugin
- `emTmpConv` — temporary file conversion pipeline

These depend on external libraries (ffmpeg, etc.) and are the most complex plugins.

### 5. Platform Ports

- `emWnds` — Windows platform abstraction (currently Linux-only via winit)
- `emX11` — X11-specific features beyond what winit provides

The Rust port uses winit which abstracts most of this, but some C++ features (like specific X11 atom handling) may need attention.

### 6. Cosmos Items for Unported Apps

The C++ default cosmos has ~47 `.emVcItem` files. We only ship items for filesystem + stocks. When more apps are ported, their cosmos items should be added back at their original C++ positions (positions are preserved in the C++ reference files).

**C++ item files:** `~/git/eaglemode-0.96.4/etc/emMain/VcItems/`

### 7. emTreeDump and emOsm

- `emTreeDump` — debug tree visualization
- `emOsm` — OpenStreetMap viewer

Both are specialized and lower priority.

### 8. Golden Tests for emMain

The current golden test infrastructure covers emCore rendering. No golden tests exist for emMain-level panels (starfield, cosmos items, control panel). These should be added once the panels are stable to prevent regressions.

## How to Use This Document

When picking the next work after the main app launch, use this as a menu. Priority order for maximum user impact:

1. `emText` (immediately makes filesystem browsing useful)
2. `emPng` + `emJpeg` (image browsing)
3. `emClock` (visible in cosmos, simple app)
4. Dynamic plugin loading (enables modular builds)
5. Remaining viewers and apps as needed
