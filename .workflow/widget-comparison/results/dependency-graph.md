# Dependency Graph — All Audit Findings

## Legend

```
[FIXED]    = code change landed, tests pass
[DEFERRED] = blocked on missing infrastructure
[CLOSED]   = intentional divergence or already handled
 ──>       = "depends on" / "blocked by"
```

---

## Infrastructure dependency tree

```
Cycle() Engine (~100 LOC: frame-tick callback registration in scheduler)
│
├──> Signal Routing (~80 LOC: typed signal emission + subscription)
│    │
│    ├──> Dialog FinishSignal [DEFERRED #11, LOW]
│    │    └── synchronous on_finish callback is sufficient for current consumers
│    │
│    ├──> Dialog window-close handling [DEFERRED #12, LOW]
│    │    └── CloseSignal from OS window → Finish(NEGATIVE)
│    │
│    └──> FileSelectionBox reactive layer [DEFERRED #6, HIGH]
│         │   ~330 LOC total (Cycle + signals + fs watch + input)
│         │
│         ├──> FSB directory navigation [DEFERRED #8, MEDIUM]
│         │    └── ListBox trigger signal → EnterSubDir() → reload listing
│         │
│         ├──> FSB name field sync [DEFERRED #9, MEDIUM]
│         │    └── bidirectional ListBox↔TextField signal wiring
│         │
│         └──> FSB FileItemPanel [DEFERRED #7, HIGH]
│              │   ~280 LOC (panel + icon + highlight + preview)
│              │   visual rendering is independent; interaction needs signals
│              │
│              └──(optional) emFpPlugin file preview system
│                   └── out of scope — entire plugin infrastructure

EOI / ZoomView Infrastructure
│   (zoom-out-on-interact behavior, not implemented in Rust port)
│
└──> Button Click() shift param + EOI signal [DEFERRED #14, NOTE]
     └── dead code without a ZoomView consumer

Config Record Field Metadata
│   (~60 LOC: add min/max/step to config field definitions)
│
├──> CoreConfigPanel downscale quality range [DEFERRED #4, LOW]
│    └── hardcoded 2.0..6.0 matches C++ defaults
│
└──> CoreConfigPanel factor field ranges [DEFERRED #5, LOW]
     └── hardcoded 0.25..4.0 matches C++ defaults

Platform Capability Queries
│   (OS-specific: X11 XWarpPointer, Wayland limitations, Win32 SetCursorPos)
│
└──> CoreConfigPanel StickPossible [DEFERRED #3, LOW]
     └── stick checkbox always enabled; harmless if stick actually works

View Transform in Paint Context
│   (threading PanelToView through ~20 paint_border call sites)
│
└──> Border HowTo pill view-space size check [DEFERRED #1, LOW]
     └── only affects when informational help text appears/hides

Trait-based Icon Provider + Asset Loading
│   (~80 LOC: PanelBehavior trait method + icon file resolution)
│
└──> FilePanel GetIconFileName [DEFERRED #2, LOW]
     └── file panels show without type-specific header icons

Unicode Collation (icu_collator crate)
│   (new dependency for locale-aware string ordering)
│
└──> FileSelectionBox locale-aware sort [DEFERRED #10, LOW]
     └── non-ASCII filenames may sort differently than C++

RadioGroup Back-References
│   (~50-100 LOC: Weak<RefCell<RadioButton>> in group, or handle indirection)
│
└──> RadioButton Drop re-index [DEFERRED #13, MEDIUM]
     └── only matters for dynamic mid-group button removal (not used currently)
```

---

## Impact-ordered implementation plan

If the deferred items were to be tackled, this is the order that maximizes unblocked work:

### Phase 1: Cycle() Engine + Signals (unblocks 5 items)
```
Cycle() engine              ──> Signal routing
                                 ├──> Dialog FinishSignal        [#11]
                                 ├──> Dialog window-close        [#12]
                                 └──> FSB reactive layer         [#6]
                                      ├──> FSB navigation        [#8]
                                      └──> FSB name field sync   [#9]
```
**Effort**: ~180 LOC (Cycle + signals). Then ~150 LOC for FSB wiring.
**Value**: Makes FileSelectionBox interactive — largest single gap in the port.

### Phase 2: FileItemPanel (unblocks 1 item, completes FSB)
```
FSB FileItemPanel [#7]
```
**Effort**: ~280 LOC. Depends on Phase 1 for selection interaction.
**Value**: Rich file entries with icons and previews instead of plain text.

### Phase 3: Independent small items (6 items, no dependencies on each other)
```
Config field metadata           [#4, #5]  ~60 LOC
RadioGroup back-references      [#13]     ~50-100 LOC
Platform CanMoveMousePointer    [#3]      ~30 LOC per platform
View transform in paint         [#1]      ~20 call sites
Icon provider trait             [#2]      ~80 LOC
```

### Phase 4: External dependency (1 item)
```
icu_collator for locale sort    [#10]     new crate dependency
```

### Not planned
```
EOI/ZoomView                    [#14]     no consumer exists in the Rust port
```

---

## Completed work (for context)

```
Sessions 1-4: 31 code fixes across 20 widgets

Pixel pipeline ─────────────── ALL CORRECT (no findings)
Border rendering ────────────── 7 FIXED (substance coeff, label_space, icon tallness,
│                                        MarginFilled, transparency, alpha rounding,
│                                        desc-only width)
├── content_rect geometry ──── CORRECT
├── 9-slice images ─────────── CORRECT
└── Look propagation ───────── CORRECT

Button family (5 widgets) ──── 19 FIXED across sessions 1-3
├── hit_test face inset ────── CC-06 FIXED (all 5)
├── modifier key checks ────── FIXED (all 5)
├── Enter key support ──────── FIXED (all 5)
├── VCT_MIN_EXT guard ──────── CC-04 FIXED (Button, CB, ChkBtn, RB, RBox)
├── enabled state ──────────── CC-03 FIXED
├── label alignment ────────── CC-05 FIXED
└── set_checked signals ────── CC-02 FIXED (CheckBox, CheckButton)

TextField ──────────────────── 6 FIXED (word boundary, backspace, double-click,
│                                       overwrite cols, Ctrl+A, selection)
├── selection model ────────── CLOSED (architectural — snapshot approach valid)
└── undo architecture ──────── CLOSED (architectural — snapshot approach valid)

Splitter ───────────────────── 3 FIXED (grip size, 2D hit test, hover cursor)
ListBox ────────────────────── 1 FIXED (row height)
ColorField ─────────────────── 1 FIXED (transparent text underlay)
RadioButton/Box ────────────── 4 FIXED (group lifecycle, select guard, face color)
ScalarField ────────────────── 1 FIXED (arrow keys removed)

Tunnel ─────────────────────── 2 FIXED (setter invalidation, child canvas color)
Dialog ─────────────────────── 2 FIXED (keyboard Enter/Escape, CheckFinish gate)
CheckButton ────────────────── 1 FIXED (HowTo chain)
FilePanel ──────────────────── 1 FIXED (saving progress display)
FileDialog ─────────────────── 1 FIXED (set_mode propagation)
CoreConfigPanel ────────────── 3 FIXED (callbacks, label text, upscale min)
FileSelectionBox ───────────── 1 FIXED (setter propagation / child rebuild)
```
