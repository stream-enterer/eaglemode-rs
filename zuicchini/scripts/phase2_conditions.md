# Phase 2: Necessary and Sufficient Conditions

## Goal
Extract code from shared source files so each C++ header's code lives in its own Rust file.

## Extractions Required (6 shared sources)

### 1. render/stroke.rs → render/stroke.rs + render/stroke_end.rs
- **Extract**: StrokeEndType (enum), StrokeEnd (struct + impl) — lines 32-111
- **Keep**: LineJoin, LineCap, DashType, Stroke (struct + impls) — lines 1-30, 113-173
- **Import fix**: stroke.rs must `use super::stroke_end::{StrokeEnd, StrokeEndType};`
- **New file needs**: `use crate::foundation::Color;`

### 2. layout/linear.rs → layout/linear.rs + layout/linear_group.rs
- **Extract**: LinearGroup (struct + impl + PanelBehavior) — lines 492-541
- **Keep**: LinearLayout (struct + impls + PanelBehavior + tests) — lines 1-490, 543+
- **Import fix**: linear_group.rs needs layout and widget imports
- **New file needs**: `use super::linear::LinearLayout;` + border/look/painter imports

### 3. layout/pack.rs → layout/pack.rs + layout/pack_group.rs
- **Extract**: PackGroup (struct + impls + Default + PanelBehavior) — lines 178-222
- **Keep**: PackLayout + internal types (PackRect, PackItem, Packer) — rest
- **New file needs**: `use super::pack::PackLayout;` + border/look/painter imports

### 4. layout/raster.rs → layout/raster.rs + layout/raster_group.rs
- **Extract**: RasterGroup (struct + impls + Default + PanelBehavior) — lines 344-388
- **Keep**: RasterLayout + tests — rest
- **New file needs**: `use super::raster::RasterLayout;` + border/look/painter imports

### 5. model/context.rs — emModel.h extraction
- **No code to extract**: emModel is a C++ abstract base class; in Rust, its functionality is absorbed into Context.
- **Create**: model/model.rs as thin file documenting this.

### 6. model/watched_var.rs — emSigModel.h + emVarSigModel.h extraction
- **No code to extract**: WatchedVar<T> IS the combined implementation of all three C++ templates.
- **Create**: model/sig_model.rs and model/var_sig_model.rs as thin files documenting this.

## mod.rs Code Extraction (3 files)

### 7. layout/mod.rs → layout/tiling.rs
- **Move**: Orientation, ResolvedOrientation, Alignment, AlignmentH, AlignmentV, Spacing, ChildConstraint, get_constraint(), position_aux_panel() — lines 5-273
- **Keep**: module declarations + re-exports only

### 8. layout/mod.rs → layout/group.rs
- **Create**: thin file for emGroup.h (deprecated C++ class)

### 9. widget/mod.rs — functions stay until Phase 3
- trace_input_enabled() and check_mouse_round_rect() stay in mod.rs for now
- Will be handled as part of Phase 3 flatten

### 10. foundation/mod.rs — functions stay until Phase 3
- set_fatal_error_graphical() and is_fatal_error_graphical() stay for now
- Will be handled as part of Phase 3 flatten

## Necessary Conditions (all must hold after Phase 2)
1. `cargo check` passes
2. Every type that existed before exists after (in exactly one file)
3. No duplicate type definitions
4. All `use` imports resolve
5. mod.rs files declare all new modules and re-export public types

## Sufficient Conditions (these guarantee correctness)
1. Each extraction creates a new file containing the exact extracted code
2. The source file is updated to remove the extracted code
3. Both files have correct `use` imports
4. Parent mod.rs declares the new module and re-exports its public types
5. `cargo check` passes after each individual extraction (not just at the end)

## Validation
- `cargo check` after each extraction (fail-fast)
- `cargo clippy --workspace -- -D warnings` after all extractions
- `cargo-nextest ntr --workspace` after all extractions
