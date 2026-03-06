# Blocked Methods — Porting Harness Round 3

> 0 of 106 methods blocked (2 resolved 2026-03-06).

## BLOCK-001: `emPanel::IsActivatedAdherent` (WS-1, P0) — RESOLVED

Added `activation_adherent: bool` to `View`, extended `set_active_panel` with `adherent` parameter,
updated all call sites, added `is_activation_adherent()` and `is_panel_activated_adherent()` accessors,
and adherent preservation logic in `set_active_panel_best_possible`.

## BLOCK-002: `emPanel::GetInputClockMS` (WS-2, P1) — RESOLVED

Added `View::get_input_clock_ms()` returning wall-clock milliseconds via `SystemTime`.
The C++ base implementation is just `emGetClockMS()` (wall-clock); per-cycle freezing
is a ViewPort subclass concern not yet needed.
