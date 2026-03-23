#!/usr/bin/env python3
"""Phase 2: Extract code from shared source files.

Each C++ header gets its own Rust file. This script handles the 6 extractions
and 3 thin file creations identified in file_mapping.json.

After each extraction, runs `cargo check` to fail fast.
"""

import os
import re
import subprocess
import sys
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
SRC = ZUICCHINI / "src"


def cargo_check(label: str):
    """Run cargo check and abort if it fails."""
    print(f"  cargo check after: {label}")
    result = subprocess.run(
        ["cargo", "check", "--workspace"],
        cwd=ZUICCHINI.parent,  # workspace root
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"FAIL: cargo check failed after {label}")
        print(result.stderr[-2000:])
        sys.exit(1)
    print(f"  OK: {label}")


def find_type_block(lines: list[str], type_name: str) -> tuple[int, int]:
    """Find the line range for a type definition + all impl blocks.

    Returns (start, end) as 0-indexed line indices where:
    - start is the first line of doc comments/attributes before the type
    - end is one past the last line of the last impl block

    Handles: struct, enum, trait definitions and their impl blocks.
    """
    # Find the type definition line
    type_pattern = re.compile(rf"^pub\s+(?:struct|enum|trait)\s+{re.escape(type_name)}\b")
    type_line = None
    for i, line in enumerate(lines):
        if type_pattern.match(line):
            type_line = i
            break

    if type_line is None:
        return None, None

    # Walk back to find doc comments and attributes
    start = type_line
    while start > 0:
        prev = lines[start - 1].strip()
        if prev.startswith("///") or prev.startswith("#[") or prev == "":
            start -= 1
            # Stop at blank lines that aren't between doc comments
            if prev == "" and start > 0 and not lines[start - 1].strip().startswith("///"):
                start += 1
                break
        else:
            break

    # Find end of type definition (brace matching)
    end = find_brace_end(lines, type_line)

    # Find all impl blocks for this type
    impl_pattern = re.compile(
        rf"^impl(?:<[^>]*>)?\s+(?:\w+\s+for\s+)?{re.escape(type_name)}\b"
    )
    i = end
    while i < len(lines):
        line = lines[i].strip()

        # Check for impl block
        if impl_pattern.match(lines[i]):
            # Walk back to include doc comments/attributes
            impl_start = i
            while impl_start > end:
                prev = lines[impl_start - 1].strip()
                if prev.startswith("///") or prev.startswith("#[") or prev == "":
                    impl_start -= 1
                    if prev == "" and impl_start > end and not lines[impl_start - 1].strip().startswith("///"):
                        impl_start += 1
                        break
                else:
                    break

            impl_end = find_brace_end(lines, i)
            end = impl_end
            i = impl_end
            continue
        i += 1

    return start, end


def find_brace_end(lines: list[str], start_line: int) -> int:
    """Find the line after the closing brace that matches the first opening brace."""
    depth = 0
    found_open = False
    for i in range(start_line, len(lines)):
        for ch in lines[i]:
            if ch == "{":
                depth += 1
                found_open = True
            elif ch == "}":
                depth -= 1
                if found_open and depth == 0:
                    return i + 1
    return len(lines)


def extract_types(
    source_path: Path,
    type_names: list[str],
    new_file_path: Path,
    new_file_imports: list[str],
    source_extra_imports: list[str] | None = None,
):
    """Extract type definitions from source file into a new file.

    Args:
        source_path: File to extract from
        type_names: List of type names to extract
        new_file_path: Path for the new file
        new_file_imports: `use` lines for the new file
        source_extra_imports: Additional `use` lines to add to source after extraction
    """
    lines = source_path.read_text().splitlines(keepends=True)

    # Find all ranges to extract (in reverse order for safe removal)
    ranges = []
    for name in type_names:
        start, end = find_type_block(lines, name)
        if start is None:
            print(f"  WARNING: Could not find type '{name}' in {source_path}")
            continue
        ranges.append((start, end))

    # Sort ranges and merge overlapping
    ranges.sort()

    # Collect extracted lines
    extracted_lines = []
    for start, end in ranges:
        extracted_lines.extend(lines[start:end])
        extracted_lines.append("\n")

    # Build new file
    new_content = "".join(new_file_imports) + "\n" + "".join(extracted_lines)
    new_file_path.write_text(new_content)

    # Remove extracted ranges from source (reverse order to preserve indices)
    for start, end in reversed(ranges):
        del lines[start:end]

    # Add extra imports to source if needed
    if source_extra_imports:
        # Find the last existing `use` line to insert after
        last_use = 0
        for i, line in enumerate(lines):
            if line.startswith("use ") or line.startswith("pub use "):
                last_use = i
            elif not line.strip() and last_use > 0 and i > last_use + 1:
                break

        for imp in reversed(source_extra_imports):
            lines.insert(last_use + 1, imp)

    source_path.write_text("".join(lines))
    print(f"  Extracted {type_names} → {new_file_path.name}")


def update_mod_rs(mod_path: Path, new_module: str, visibility: str = ""):
    """Add a module declaration to a mod.rs file.

    Inserts `[visibility] mod new_module;` after existing mod declarations.
    """
    content = mod_path.read_text()
    lines = content.splitlines(keepends=True)

    # Find last mod declaration
    last_mod = 0
    for i, line in enumerate(lines):
        if re.match(r"^(?:pub(?:\(crate\))?\s+)?mod\s+\w+;", line):
            last_mod = i

    prefix = f"{visibility} " if visibility else ""
    new_line = f"{prefix}mod {new_module};\n"
    lines.insert(last_mod + 1, new_line)

    mod_path.write_text("".join(lines))


def main():
    os.chdir(ZUICCHINI)

    print("=== Phase 2: Code Extraction ===")
    print()

    # ─── 1. StrokeEnd from render/stroke.rs ──────────────────────
    print("1/8: StrokeEnd from render/stroke.rs")
    extract_types(
        source_path=SRC / "render" / "stroke.rs",
        type_names=["StrokeEndType", "StrokeEnd"],
        new_file_path=SRC / "render" / "stroke_end.rs",
        new_file_imports=["use crate::foundation::Color;\n", "\n"],
        source_extra_imports=["pub use super::stroke_end::{StrokeEnd, StrokeEndType};\n"],
    )

    # Update render/mod.rs
    mod_rs = SRC / "render" / "mod.rs"
    content = mod_rs.read_text()
    content = content.replace("mod stroke;\n", "mod stroke;\nmod stroke_end;\n")
    content = content.replace(
        "pub use stroke::{DashType, LineCap, LineJoin, Stroke, StrokeEnd, StrokeEndType};",
        "pub use stroke::{DashType, LineCap, LineJoin, Stroke};\n"
        "pub use stroke_end::{StrokeEnd, StrokeEndType};",
    )
    mod_rs.write_text(content)
    cargo_check("StrokeEnd extraction")

    # ─── 2. LinearGroup from layout/linear.rs ────────────────────
    print("\n2/8: LinearGroup from layout/linear.rs")
    extract_types(
        source_path=SRC / "layout" / "linear.rs",
        type_names=["LinearGroup"],
        new_file_path=SRC / "layout" / "linear_group.rs",
        new_file_imports=[
            "use crate::panel::{PanelBehavior, PanelCtx, PanelState};\n",
            "use crate::render::Painter;\n",
            "use crate::widget::{Border, InnerBorderType, Look, OuterBorderType};\n",
            "\n",
            "use super::linear::LinearLayout;\n",
            "use super::position_aux_panel;\n",
        ],
    )
    update_mod_rs(SRC / "layout" / "mod.rs", "linear_group", "pub(crate)")
    cargo_check("LinearGroup extraction")

    # ─── 3. PackGroup from layout/pack.rs ────────────────────────
    print("\n3/8: PackGroup from layout/pack.rs")
    extract_types(
        source_path=SRC / "layout" / "pack.rs",
        type_names=["PackGroup"],
        new_file_path=SRC / "layout" / "pack_group.rs",
        new_file_imports=[
            "use crate::panel::{PanelBehavior, PanelCtx, PanelState};\n",
            "use crate::render::Painter;\n",
            "use crate::widget::{Border, InnerBorderType, Look, OuterBorderType};\n",
            "\n",
            "use super::pack::PackLayout;\n",
            "use super::position_aux_panel;\n",
        ],
    )
    update_mod_rs(SRC / "layout" / "mod.rs", "pack_group", "pub(crate)")
    cargo_check("PackGroup extraction")

    # ─── 4. RasterGroup from layout/raster.rs ────────────────────
    print("\n4/8: RasterGroup from layout/raster.rs")
    extract_types(
        source_path=SRC / "layout" / "raster.rs",
        type_names=["RasterGroup"],
        new_file_path=SRC / "layout" / "raster_group.rs",
        new_file_imports=[
            "use crate::panel::{PanelBehavior, PanelCtx, PanelState};\n",
            "use crate::render::Painter;\n",
            "use crate::widget::{Border, InnerBorderType, Look, OuterBorderType};\n",
            "\n",
            "use super::raster::RasterLayout;\n",
            "use super::position_aux_panel;\n",
        ],
    )
    update_mod_rs(SRC / "layout" / "mod.rs", "raster_group", "pub(crate)")
    cargo_check("RasterGroup extraction")

    # ─── 5. Create thin group.rs + extract tiling from layout/mod.rs ─
    # group.rs must exist before mod.rs references it
    print("\n5/8: Thin group.rs + Tiling types from layout/mod.rs")

    (SRC / "layout" / "group.rs").write_text(
        "// emGroup.h: deprecated C++ class.\n"
        "//\n"
        "// In C++, emGroup was a generic tiling panel with a group border,\n"
        "// deprecated in favor of emLinearGroup, emRasterGroup, emPackGroup.\n"
        "//\n"
        "// In Rust, the specific Group types are used directly.\n"
        "//\n"
        "// This file exists for 1:1 header correspondence.\n"
    )

    mod_path = SRC / "layout" / "mod.rs"
    lines = mod_path.read_text().splitlines(keepends=True)

    # Find where real code starts (first `use std::` or `use crate::`)
    code_start = None
    for i, line in enumerate(lines):
        if line.startswith("use std::") or line.startswith("use crate::"):
            code_start = i
            break

    if code_start is None:
        print("  ERROR: Could not find code start in layout/mod.rs")
        sys.exit(1)

    # Everything from code_start onwards is tiling code
    tiling_lines = lines[code_start:]
    mod_lines = lines[:code_start]

    # Clean up mod_lines: remove trailing blanks
    while mod_lines and mod_lines[-1].strip() == "":
        mod_lines.pop()
    mod_lines.append("\n")

    # Add tiling + group module declarations and re-exports
    # get_constraint and position_aux_panel are pub(crate), use pub(crate) use
    mod_lines.append("pub mod tiling;\n")
    mod_lines.append("mod group;\n")
    mod_lines.append("\n")
    mod_lines.append("pub use tiling::{\n")
    mod_lines.append("    Alignment, AlignmentH, AlignmentV, ChildConstraint, Orientation,\n")
    mod_lines.append("    ResolvedOrientation, Spacing,\n")
    mod_lines.append("};\n")
    mod_lines.append("pub(crate) use tiling::{get_constraint, position_aux_panel};\n")

    # Write tiling.rs
    (SRC / "layout" / "tiling.rs").write_text("".join(tiling_lines))

    # Write updated mod.rs
    mod_path.write_text("".join(mod_lines))
    print("  Created src/layout/tiling.rs + group.rs")

    cargo_check("Tiling + group extraction from mod.rs")

    # ─── 6. Thin model.rs ────────────────────────────────────────
    print("\n6/8: Thin model.rs for emModel.h")
    (SRC / "model" / "model.rs").write_text(
        "// emModel.h: abstract base class for named/registered models.\n"
        "//\n"
        "// In C++, emModel inherits emEngine and provides name-based registration\n"
        "// in emContext, common lifetime management, and type-erased lookup.\n"
        "//\n"
        "// In Rust, this functionality is absorbed into Context (context.rs):\n"
        "// Context::register() / Context::lookup() handle registration,\n"
        "// Rc<RefCell<T>> replaces C++ ref-counting.\n"
        "//\n"
        "// This file exists for 1:1 header correspondence.\n"
    )
    update_mod_rs(SRC / "model" / "mod.rs", "model")
    cargo_check("thin model.rs")

    # ─── 7. Thin sig_model.rs + var_sig_model.rs ─────────────────
    print("\n7/8: Thin sig_model.rs + var_sig_model.rs")
    (SRC / "model" / "sig_model.rs").write_text(
        "// emSigModel.h: a model that holds only a signal (no data).\n"
        "//\n"
        "// In C++, emSigModel inherits emModel and adds a signal.\n"
        "// In Rust, signal-only models use SignalId directly.\n"
        "//\n"
        "// This file exists for 1:1 header correspondence.\n"
    )
    (SRC / "model" / "var_sig_model.rs").write_text(
        "// emVarSigModel.h: a model holding a value + signal.\n"
        "//\n"
        "// In C++, emVarSigModel<T> provides Get()/Set() with signal emission.\n"
        "// In Rust, WatchedVar<T> (watched_var.rs) provides this functionality.\n"
        "//\n"
        "// This file exists for 1:1 header correspondence.\n"
    )
    mod_path = SRC / "model" / "mod.rs"
    update_mod_rs(mod_path, "sig_model")
    update_mod_rs(mod_path, "var_sig_model")
    cargo_check("thin sig_model.rs + var_sig_model.rs")

    # (Step 8 merged into step 5 — group.rs created alongside tiling.rs)

    # ─── Final validation ────────────────────────────────────────
    print("\n=== Phase 2 Final Validation ===")
    print("Running clippy...")
    result = subprocess.run(
        ["cargo", "clippy", "--workspace", "--", "-D", "warnings"],
        cwd=ZUICCHINI.parent,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("FAIL: clippy")
        print(result.stderr[-2000:])
        sys.exit(1)
    print("  clippy OK")

    print("Running tests...")
    result = subprocess.run(
        ["cargo-nextest", "ntr", "--workspace"],
        cwd=ZUICCHINI.parent,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("FAIL: tests")
        print(result.stdout[-2000:])
        print(result.stderr[-2000:])
        sys.exit(1)
    print("  tests OK")

    print("\n=== Phase 2 Complete ===")


if __name__ == "__main__":
    main()
