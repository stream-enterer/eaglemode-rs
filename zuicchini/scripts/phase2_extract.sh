#!/bin/bash
# Phase 2: Extract code from shared source files
# Each extraction is followed by cargo check to fail fast.
set -euo pipefail

cd "$(dirname "$0")/.."
echo "=== Phase 2: Code Extraction ==="

check() {
    echo "  cargo check..."
    if ! cargo check --workspace 2>&1 | tail -3; then
        echo "FAIL: cargo check failed after: $1"
        exit 1
    fi
    echo "  OK: $1"
}

# ─── 1. Extract StrokeEnd from render/stroke.rs ─────────────────────

echo "1/8: Extracting StrokeEnd from render/stroke.rs"

# Create render/stroke_end.rs with StrokeEndType + StrokeEnd
python3 -c "
lines = open('src/render/stroke.rs').readlines()

# StrokeEnd types are lines 32-111 (0-indexed: 31-110)
# They need: use crate::foundation::Color;
extracted = ['use crate::foundation::Color;\n', '\n']
extracted += lines[31:111]  # lines 32-111

# Write new file
with open('src/render/stroke_end.rs', 'w') as f:
    f.writelines(extracted)

# Remove extracted lines from source, add import
remaining = [lines[0]]  # use crate::foundation::Color;
remaining.append('\n')
remaining.append('use super::stroke_end::{StrokeEnd, StrokeEndType};\n')
remaining.append('\n')
remaining += lines[2:31]   # LineJoin, LineCap, DashType (lines 3-31)
remaining += ['\n']
remaining += lines[111:]   # Stroke and rest (line 112+)

with open('src/render/stroke.rs', 'w') as f:
    f.writelines(remaining)

print('  Created src/render/stroke_end.rs')
"

# Update render/mod.rs to declare new module
python3 -c "
content = open('src/render/mod.rs').read()
# Add module declaration after the stroke line
content = content.replace('mod stroke;\n', 'mod stroke;\nmod stroke_end;\n')
# Add re-export
content = content.replace(
    'pub use stroke::{DashType, LineCap, LineJoin, Stroke, StrokeEnd, StrokeEndType};',
    'pub use stroke::{DashType, LineCap, LineJoin, Stroke};\npub use stroke_end::{StrokeEnd, StrokeEndType};'
)
with open('src/render/mod.rs', 'w') as f:
    f.write(content)
print('  Updated src/render/mod.rs')
"

check "StrokeEnd extraction"

# ─── 2. Extract LinearGroup from layout/linear.rs ────────────────────

echo "2/8: Extracting LinearGroup from layout/linear.rs"

python3 -c "
lines = open('src/layout/linear.rs').readlines()

# LinearGroup starts at line 492 (0-indexed: 491), PanelBehavior impl ends at 541
# Then tests start at 543
group_start = 491  # '/// LinearGroup: ...'
group_end = 541    # closing brace of PanelBehavior impl

imports = [
    'use crate::foundation::Rect;\n',
    'use crate::panel::{NoticeFlags, PanelBehavior, PanelCtx, PanelId, PanelState};\n',
    'use crate::render::Painter;\n',
    'use crate::widget::{Border, InnerBorderType, Look, OuterBorderType};\n',
    '\n',
    'use super::linear::LinearLayout;\n',
    'use super::position_aux_panel;\n',
    '\n',
]

extracted = imports + lines[group_start:group_end]

# Write new file
with open('src/layout/linear_group.rs', 'w') as f:
    f.writelines(extracted)

# Remove extracted lines from source
remaining = lines[:group_start]
# Keep blank line before tests
remaining += lines[group_end:]

with open('src/layout/linear.rs', 'w') as f:
    f.writelines(remaining)

print('  Created src/layout/linear_group.rs')
"

# Update layout/mod.rs
python3 -c "
content = open('src/layout/mod.rs').read()
content = content.replace('pub mod linear;\n', 'pub mod linear;\npub(crate) mod linear_group;\n')
with open('src/layout/mod.rs', 'w') as f:
    f.write(content)
print('  Updated src/layout/mod.rs')
"

check "LinearGroup extraction"

# ─── 3. Extract PackGroup from layout/pack.rs ────────────────────────

echo "3/8: Extracting PackGroup from layout/pack.rs"

python3 -c "
lines = open('src/layout/pack.rs').readlines()

# PackGroup: lines 178-222 (0-indexed: 177-221)
group_start = 177  # '/// PackGroup wraps...'
group_end = 222    # closing brace of PanelBehavior impl

imports = [
    'use crate::foundation::Rect;\n',
    'use crate::panel::{NoticeFlags, PanelBehavior, PanelCtx, PanelState};\n',
    'use crate::render::Painter;\n',
    'use crate::widget::{Border, InnerBorderType, Look, OuterBorderType};\n',
    '\n',
    'use super::pack::PackLayout;\n',
    'use super::position_aux_panel;\n',
    '\n',
]

extracted = imports + lines[group_start:group_end]

with open('src/layout/pack_group.rs', 'w') as f:
    f.writelines(extracted)

remaining = lines[:group_start]
remaining += lines[group_end:]

with open('src/layout/pack.rs', 'w') as f:
    f.writelines(remaining)

print('  Created src/layout/pack_group.rs')
"

python3 -c "
content = open('src/layout/mod.rs').read()
content = content.replace('pub mod pack;\n', 'pub mod pack;\npub(crate) mod pack_group;\n')
with open('src/layout/mod.rs', 'w') as f:
    f.write(content)
"

check "PackGroup extraction"

# ─── 4. Extract RasterGroup from layout/raster.rs ────────────────────

echo "4/8: Extracting RasterGroup from layout/raster.rs"

python3 -c "
lines = open('src/layout/raster.rs').readlines()

# RasterGroup: lines 344-388 (0-indexed: 343-387)
group_start = 343  # '/// RasterGroup wraps...'
group_end = 388    # closing brace of PanelBehavior impl

imports = [
    'use crate::foundation::Rect;\n',
    'use crate::panel::{NoticeFlags, PanelBehavior, PanelCtx, PanelState};\n',
    'use crate::render::Painter;\n',
    'use crate::widget::{Border, InnerBorderType, Look, OuterBorderType};\n',
    '\n',
    'use super::raster::RasterLayout;\n',
    'use super::position_aux_panel;\n',
    '\n',
]

extracted = imports + lines[group_start:group_end]

with open('src/layout/raster_group.rs', 'w') as f:
    f.writelines(extracted)

remaining = lines[:group_start]
remaining += lines[group_end:]

with open('src/layout/raster.rs', 'w') as f:
    f.writelines(remaining)

print('  Created src/layout/raster_group.rs')
"

python3 -c "
content = open('src/layout/mod.rs').read()
content = content.replace('pub mod raster;\n', 'pub mod raster;\npub(crate) mod raster_group;\n')
with open('src/layout/mod.rs', 'w') as f:
    f.write(content)
"

check "RasterGroup extraction"

# ─── 5. Extract layout/mod.rs types to layout/tiling.rs ──────────────

echo "5/8: Extracting Tiling types from layout/mod.rs"

python3 -c "
lines = open('src/layout/mod.rs').readlines()

# Find the line where real code starts (after module declarations)
# Module declarations and re-exports are at the top
# Real code starts with 'use std::fmt;'
code_start = None
code_end = len(lines)
for i, line in enumerate(lines):
    if line.startswith('use std::') or line.startswith('use crate::'):
        if code_start is None:
            code_start = i
        break

if code_start is None:
    print('ERROR: Could not find code start in layout/mod.rs')
    exit(1)

# Extract everything from code_start onwards
extracted_lines = lines[code_start:]

# Write tiling.rs
with open('src/layout/tiling.rs', 'w') as f:
    f.writelines(extracted_lines)

# Keep only module declarations in mod.rs, add tiling module + re-exports
mod_lines = lines[:code_start]
# Remove any trailing blank lines from mod section
while mod_lines and mod_lines[-1].strip() == '':
    mod_lines.pop()
mod_lines.append('\n')
mod_lines.append('pub mod tiling;\n')
mod_lines.append('\n')
mod_lines.append('pub use tiling::{\n')
mod_lines.append('    Alignment, AlignmentH, AlignmentV, ChildConstraint, Orientation,\n')
mod_lines.append('    ResolvedOrientation, Spacing, get_constraint, position_aux_panel,\n')
mod_lines.append('};\n')

with open('src/layout/mod.rs', 'w') as f:
    f.writelines(mod_lines)

print('  Created src/layout/tiling.rs')
"

check "Tiling extraction from mod.rs"

# ─── 6. Create thin model.rs for emModel.h ───────────────────────────

echo "6/8: Creating thin model/model.rs for emModel.h"

cat > src/model/model.rs << 'RUST'
// emModel.h: abstract base class for named/registered models.
//
// In C++, emModel inherits emEngine and provides:
//   - Name-based registration in emContext
//   - Common lifetime management
//   - Type-erased lookup by (TypeId, name)
//
// In Rust, this functionality is absorbed into Context (model/context.rs):
//   - Context::register() / Context::lookup() handle registration
//   - Rc<RefCell<T>> replaces the C++ ref-counting model
//   - No separate Model trait needed; any T: Any can be a model
//
// This file exists for 1:1 header correspondence.
// See context.rs for the actual implementation.
RUST

python3 -c "
content = open('src/model/mod.rs').read()
content = content.replace('mod clipboard;\n', 'mod clipboard;\nmod model;\n')
with open('src/model/mod.rs', 'w') as f:
    f.write(content)
"

check "thin model.rs"

# ─── 7. Create thin sig_model.rs + var_sig_model.rs ──────────────────

echo "7/8: Creating thin sig_model.rs and var_sig_model.rs"

cat > src/model/sig_model.rs << 'RUST'
// emSigModel.h: a model that holds only a signal (no data).
//
// In C++, emSigModel inherits emModel and adds a signal that can be
// polled by engines. In Rust, signal-only models use the scheduler's
// SignalId directly — no wrapper type needed.
//
// This file exists for 1:1 header correspondence.
// See scheduler/signal.rs for SignalId.
RUST

cat > src/model/var_sig_model.rs << 'RUST'
// emVarSigModel.h: a model holding a value + signal.
//
// In C++, emVarSigModel<T> inherits emModel and provides Get()/Set()
// with automatic signal emission on change. In Rust, WatchedVar<T>
// (model/watched_var.rs) provides equivalent functionality:
//   - WatchedVar::get() / WatchedVar::set() with change detection
//   - WatchedVar::signal_id() for scheduler integration
//
// This file exists for 1:1 header correspondence.
// See watched_var.rs for the actual implementation.
RUST

python3 -c "
content = open('src/model/mod.rs').read()
content = content.replace('mod model;\n', 'mod model;\nmod sig_model;\nmod var_sig_model;\n')
with open('src/model/mod.rs', 'w') as f:
    f.write(content)
"

check "thin sig_model.rs + var_sig_model.rs"

# ─── 8. Create thin layout/group.rs for emGroup.h ────────────────────

echo "8/8: Creating thin layout/group.rs for emGroup.h"

cat > src/layout/group.rs << 'RUST'
// emGroup.h: deprecated C++ class.
//
// In C++, emGroup was a generic tiling panel with a group border.
// It was deprecated in favor of emLinearGroup, emRasterGroup, emPackGroup.
//
// In Rust, the specific Group types (LinearGroup, RasterGroup, PackGroup)
// are used directly. There is no generic Group equivalent.
//
// This file exists for 1:1 header correspondence.
RUST

python3 -c "
content = open('src/layout/mod.rs').read()
content = content.replace('pub mod tiling;\n', 'pub mod tiling;\nmod group;\n')
with open('src/layout/mod.rs', 'w') as f:
    f.write(content)
"

check "thin group.rs"

# ─── Final validation ────────────────────────────────────────────────

echo ""
echo "=== Phase 2 Final Validation ==="
echo "Running clippy..."
cargo clippy --workspace -- -D warnings 2>&1 | tail -5
echo "Running tests..."
cargo-nextest ntr --workspace 2>&1 | tail -10
echo ""
echo "=== Phase 2 Complete ==="
