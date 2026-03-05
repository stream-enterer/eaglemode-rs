# Deferred Items

Tracked here so they don't get forgotten. Sourced from EMCORE_FEATURE_CONTRACT.md.

## View Animators

- [ ] `SwipingViewAnimator` — touch-drag with spring physics and momentum (needs touch input infrastructure)
- [ ] `MagneticViewAnimator` — snaps view to "best" panel alignment (needs working UI for tuning)

## Widgets

- [ ] `FileSelectionBox` — file browser (only if game needs file open/save)
- [ ] `FileDialog` — wraps FileSelectionBox in a dialog window
- [ ] `CoreConfigPanel` — core settings editor (needs config system fully working)
- [ ] `ErrorPanel` — simple error text display (small effort, useful for debugging)

## Rendering

- [ ] Multi-threaded tile rasterization — parallelize independent dirty tiles across threads (benchmark-driven, threading boundary is well-defined)

## Font System Follow-ups

- [ ] Hinted rasterization — skrifa's `HintingInstance` requires per-size instances; currently using `DrawSettings::unhinted`. Add hinting for crisper text at small sizes (no API changes needed)
- [ ] Thread FontCache through PanelBehavior/PanelCtx — when widgets start implementing `PanelBehavior::preferred_size` via the trait (not just inherent methods), the trait signature and PanelCtx need `&FontCache`
- [ ] Variable font weight selection — Inter Variable is embedded but always renders at default weight; expose weight axis via `skrifa::instance::Location`
- [ ] Text scroll in TextField — `scroll_x` isn't updated when cursor moves past visible area with proportional fonts
- [ ] i18n shaping verification — rustybuzz handles Arabic/Devanagari/CJK but needs testing with actual multilingual text
