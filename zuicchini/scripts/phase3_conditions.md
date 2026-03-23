# Phase 3: Necessary and Sufficient Conditions

## Goal
Flatten all Rust source files into `src/emCore/` with `emFoo.rs` naming per file_mapping.json.

## Steps

### 1. Create directory structure
- `src/emCore/`
- `src/emCore/shaders/`

### 2. Move files (per file_mapping.json)
- Each `source_file` → `target_rs` via git mv
- Shader: `src/render/shaders/tile_composite.wgsl` → `src/emCore/shaders/tile_composite.wgsl`
- Rust-only files: move per rust_only target_rs entries

### 3. Handle mod.rs real code
- widget/mod.rs functions → separate file (emBorder helpers or standalone)
- foundation/mod.rs functions → emStd1.rs or alongside dlog.rs content

### 4. Create marker files
- 15 `.no_rust_equivalent` files (from file_mapping.json marker_file entries)
- 3 `.rust_only` files (from rust_only marker_file entries)

### 5. Generate src/emCore/mod.rs
- One `mod emFoo;` declaration per .rs file (excluding mod.rs itself)
- Public re-exports matching current public API

### 6. Update src/lib.rs
- Replace all module declarations with single `#[allow(non_snake_case)] pub mod emCore;`
- Remove old mod declarations (foundation, input, layout, model, panel, render, scheduler, widget, window, debug)

### 7. Fix imports (bulk rewrite)
- `use crate::foundation::X` → `use crate::emCore::X`
- `use crate::widget::X` → `use crate::emCore::X`
- `use crate::input::X` → `use crate::emCore::X`
- `use crate::layout::X` → `use crate::emCore::X`
- `use crate::model::X` → `use crate::emCore::X`
- `use crate::panel::X` → `use crate::emCore::X`
- `use crate::render::X` → `use crate::emCore::X`
- `use crate::scheduler::X` → `use crate::emCore::X`
- `use crate::window::X` → `use crate::emCore::X`
- `use crate::debug::X` → `use crate::emCore::X` (if any)
- `use super::X` → rewrite to `use crate::emCore::X` or local module reference
- Cross-crate: update sosumi-7 imports too

### 8. Fix include_str!/include_bytes! paths
- Verify all 19 references resolve from new file locations
- Shader reference in compositor: `include_str!("shaders/tile_composite.wgsl")` works if shaders dir moves with it

### 9. Clean up
- Remove empty old directories
- Remove old mod.rs files
- Ensure no stale files remain in old locations

## Necessary Conditions (all must hold after Phase 3)
1. `cargo clippy --workspace -- -D warnings` passes
2. `cargo-nextest ntr --workspace` passes
3. `python3 scripts/verify_correspondence.py --validate` passes
4. `ls src/emCore/*.rs | wc -l` matches expected file count
5. `ls src/emCore/*.no_rust_equivalent | wc -l` == 15
6. `ls src/emCore/*.rust_only | wc -l` == 3
7. No .rs files remain in src/ subdirectories except src/emCore/ and src/debug/
8. All include_str!/include_bytes! paths resolve (cargo check proves this)
9. src/emCore/shaders/tile_composite.wgsl exists

## Sufficient Conditions
1. Every mapping entry's source_file is moved to its target_rs path
2. All imports rewritten per the module→emCore substitution
3. mod.rs and lib.rs correctly declare all modules
4. Marker files created per mapping
5. The test suite passes (proves no behavioral regressions)

## Validation
- `cargo check` after lib.rs + mod.rs changes (structural correctness)
- `cargo clippy --workspace -- -D warnings` (lint clean)
- `cargo-nextest ntr --workspace` (behavioral correctness)
- `python3 scripts/verify_correspondence.py --validate` (structural integrity)
- `grep -r 'use crate::foundation\|use crate::widget\|use crate::layout\|use crate::model\|use crate::panel\|use crate::render\|use crate::scheduler\|use crate::window\|use crate::input' src/emCore/` returns 0 results (all old imports gone)
