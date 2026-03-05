# zuicchini

A zoomable UI framework in Rust, inspired by the architecture of
[Eagle Mode](https://eaglemode.sourceforge.net/) by Oliver Hamann.

## About

zuicchini is a clean-room reimplementation of the core ideas behind Eagle Mode's
`emCore` library — a zoomable, panel-based UI framework where the entire interface
lives in a single infinitely zoomable plane. It is not a fork or derivative of
Eagle Mode; no Eagle Mode source code was used.

## Architecture

The framework is organized into layered modules:

- **foundation** — Core types: `Color` (packed RGBA), `Image` (CPU pixel buffer)
- **scheduler** — Cooperative task scheduler with priority engines, signals, and timers
- **model** — Observable data: watched variables, KDL-based records, context tree, resource cache
- **render** — CPU software `Painter` (rects, ellipses, lines, polygons, rounded rects, bitmap text), `Stroke` styling, wgpu `Compositor`, tile cache
- **input** — `InputEvent`/`InputKey`/`InputVariant`, `Cursor` enum, `Hotkey` parser, `InputState` tracker
- **panel** — `PanelBehavior` trait, `PanelTree` (slotmap-backed), `PanelCtx` scoped API, view/zoom/visit navigation, animator, input filter
- **layout** — Linear, pack (brute-force optimal), and raster layout algorithms with adaptive orientation
- **window** — winit/wgpu application shell, window management, screen info, state persistence
- **widget** — Ready-made UI components: Label, Button, CheckButton, CheckBox, RadioButton, RadioBox, Splitter, ListBox, TextField, ScalarField, ColorField, Dialog; with `Look` theming and `Border` chrome

## Building

```
cargo build
cargo test
```

## Acknowledgments

This project is heavily inspired by [Eagle Mode](https://eaglemode.sourceforge.net/),
created by Oliver Hamann and released under the GNU General Public License v3.
Eagle Mode pioneered the concept of a fully zoomable user interface where all
content — files, images, text, controls — exists on a single infinite canvas
navigated by zooming and panning. zuicchini aims to bring these ideas to the
Rust ecosystem.

## License

MIT — see [LICENSE](LICENSE).
