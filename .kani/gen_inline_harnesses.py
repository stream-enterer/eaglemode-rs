#!/usr/bin/env python3
"""Generate #[cfg(kani)] inline harness blocks for private functions.

Reads provable_functions.json, identifies functions that were skipped
because they're private, and generates harness code to append to each
source file. The harnesses go inside the file so they can access
private items.

Usage: python3 .kani/gen_inline_harnesses.py
  Prints the code blocks to append to each file.
  Use --apply to actually write them.
"""

import json
import os
import re
import sys

INPUT = ".kani/provable_functions.json"
RUST_SRC = "src/emCore"

PRIMITIVES = {
    "u8", "u16", "u32", "u64", "u128", "usize",
    "i8", "i16", "i32", "i64", "i128", "isize",
    "f32", "f64", "bool", "char",
}

# Functions to skip entirely (as_any returns dyn trait, kill sends signals,
# uninhibit_screensaver is platform stub, to_rec returns String)
EXCLUDE = {
    ("emViewAnimator", "as_any"),
    ("emProcess", "kill"),
    ("emWindowPlatform", "uninhibit_screensaver"),
    ("emCoreConfig", "to_rec"),
    ("emWindowStateSaver", "to_rec"),
    # These return String/Vec which Kani can't handle well
    ("emRec", "format_double"),
    ("emCoreConfigPanel", "factor_text_of_value"),
    ("emCoreConfigPanel", "mem_text_of_value"),
    ("emCoreConfigPanel", "downscale_text"),
    ("emCoreConfigPanel", "upscale_text"),
    ("emScalarField", "default_text_of_value"),
    ("emTextField", "char_press"),
    ("emTextField", "ctrl_char"),
    # make_dest allocates Vec
    ("emPainterScanlineTool", "make_dest"),
    # rect_vertices returns Vec
    ("emPainterScanline", "rect_vertices"),
    # setup functions create complex test fixtures
    ("emLinearLayout", "setup_tree"),
    ("emPackLayout", "setup"),
    ("emRasterLayout", "setup"),
    # setup_scrolled/input_state_at/setup_two_finger_tracker need complex test fixtures
    ("emViewAnimator", "setup_scrolled"),
    ("emViewInputFilter", "input_state_at"),
    ("emViewInputFilter", "setup_two_finger_tracker"),
    # compute_arrow_vertices returns fixed-size array but via complex geometry
    ("emView", "compute_arrow_vertices"),
    # make_area_xfm returns struct with many fields, complex construction
    ("emPainterInterpolation", "make_area_xfm"),
    # lanczos_sinc uses sin which Kani can't handle
    ("emPainterInterpolation", "lanczos_sinc"),
    # GetChar accesses static atlas
    ("emFontCache", "GetChar"),
}

# The private functions to generate harnesses for
SKIP_FUNCTIONS = {
    ('emATMatrix', 'approx_eq'),
    ('emClipRects', 'new'),
    ('emCoreConfig', 'clamp_f64'),
    ('emCoreConfig', 'clamp_i32'),
    ('emCoreConfig', 'IsSetToDefault'),
    ('emCoreConfig', 'SetToDefault'),
    ('emCoreConfig', 'to_rec'),
    ('emCoreConfigPanel', 'factor_val_to_cfg'),
    ('emCoreConfigPanel', 'factor_cfg_to_val'),
    ('emCoreConfigPanel', 'factor_text_of_value'),
    ('emCoreConfigPanel', 'mem_cfg_to_val'),
    ('emCoreConfigPanel', 'mem_val_to_cfg'),
    ('emCoreConfigPanel', 'mem_text_of_value'),
    ('emCoreConfigPanel', 'downscale_text'),
    ('emCoreConfigPanel', 'upscale_text'),
    ('emFontCache', 'GetChar'),
    ('emLinearLayout', 'setup_tree'),
    ('emPackLayout', 'setup'),
    ('emPackLayout', 'rate_cell'),
    ('emRasterLayout', 'setup'),
    ('emPainterInterpolation', 'make_area_xfm'),
    ('emPainterInterpolation', 'full_sec'),
    ('emPainterInterpolation', 'channel_diff'),
    ('emPainterInterpolation', 'rational_inv'),
    ('emPainterInterpolation', 'interpolate_four_values_adaptive'),
    ('emPainterInterpolation', 'lanczos_sinc'),
    ('emPainterScanline', 'make_poly_span'),
    ('emPainterScanline', 'make_edge_span'),
    ('emPainterScanline', 'rect_vertices'),
    ('emPainterScanline', 'round_abs'),
    ('emPainterScanlineTool', 'make_dest'),
    ('emPainterScanlineTool', 'from_state'),
    ('emPainterScanlineTool', 'new'),
    ('emViewAnimator', 'accelerate_dim'),
    ('emViewAnimator', 'get_curve_point'),
    ('emViewAnimator', 'get_curve_pos_dist'),
    ('emViewAnimator', 'get_direct_dist'),
    ('emViewAnimator', 'get_direct_point'),
    ('emViewAnimator', 'setup_scrolled'),
    ('emViewAnimator', 'as_any'),
    ('emViewAnimator', 'update_busy_state'),
    ('emViewAnimator', 'is_active'),
    ('emViewAnimator', 'stop'),
    ('emViewInputFilter', 'speeding_step'),
    ('emViewInputFilter', 'input_state_at'),
    ('emViewInputFilter', 'setup_two_finger_tracker'),
    ('emView', 'compute_arrow_count'),
    ('emView', 'compute_arrow_vertices'),
    ('emPainter', 'new'),
    ('emPainter', 'cut_arrow'),
    ('emPainter', 'cut_triangle'),
    ('emPainter', 'cut_square'),
    ('emPainter', 'cut_circle'),
    ('emPainter', 'cut_diamond'),
    ('emPainter', 'coverage'),
    ('emPainter', 'IsEmpty'),
    ('emPainter', 'adaptive_circle_segments'),
    ('emPainter', 'to_scanline_clip'),
    ('emProcess', 'kill'),
    ('emResTga', 'make_tga_header'),
    ('emTextField', 'char_press'),
    ('emTextField', 'ctrl_char'),
    ('emTextField', 'is_word_char'),
    ('emRec', 'format_double'),
    ('emScalarField', 'default_text_of_value'),
    ('emRenderThreadPool', 'compute_count'),
    ('emWindowPlatform', 'uninhibit_screensaver'),
    ('emWindowStateSaver', 'IsSetToDefault'),
    ('emWindowStateSaver', 'SetToDefault'),
    ('emWindowStateSaver', 'to_rec'),
}


def generate_harness_code(fn_info):
    """Generate a single harness function body."""
    name = fn_info["name"]
    module = fn_info["module"]
    impl_type = fn_info.get("impl_type")
    params = fn_info["params"]
    is_unsafe = fn_info.get("is_unsafe", False)
    has_self = fn_info.get("has_self", False)
    self_kind = fn_info.get("self_kind")
    return_type = fn_info.get("return_type")
    constructions = fn_info.get("constructions", {})

    if impl_type:
        harness_name = f"kani_private_{impl_type}_{name}"
    else:
        harness_name = f"kani_private_{name}"
    harness_name = re.sub(r'[^a-zA-Z0-9_]', '_', harness_name)

    lines = []
    lines.append(f"    #[kani::proof]")
    lines.append(f"    fn {harness_name}() {{")

    # Handle self
    if has_self and impl_type:
        self_info = constructions.get("self", {})
        construction = self_info.get("construction", "")
        if not construction:
            return None
        # For types constructed via public API (crate:: paths), strip the crate prefix
        # since we're inside the module
        construction = construction.replace(f"crate::emCore::{module}::", "")
        lines.append(f"        let mut self_val = {construction};")

    # Handle other params
    call_args = []
    for pname, ptype in params:
        if pname == "self":
            continue
        param_info = constructions.get(pname, {})
        construction = param_info.get("construction", "")
        clean_type = param_info.get("clean_type", ptype)
        is_ref = param_info.get("is_ref", False)
        is_mut_ref = param_info.get("is_mut_ref", False)

        if not construction:
            return None

        var = f"p_{pname}"
        # Strip crate prefix for local types
        construction = construction.replace(f"crate::emCore::{module}::", "")

        if clean_type in PRIMITIVES:
            lines.append(f"        let mut {var}: {clean_type} = {construction};")
        else:
            lines.append(f"        let mut {var} = {construction};")

        # Add finite assumes for f64/f32
        if clean_type in ("f64", "f32"):
            lines.append(f"        kani::assume({var}.is_finite());")

        if is_mut_ref:
            call_args.append(f"&mut {var}")
        elif is_ref:
            call_args.append(f"&{var}")
        else:
            call_args.append(var)

    args_str = ", ".join(call_args)

    # Build call
    if has_self and impl_type:
        call = f"self_val.{name}({args_str})"
    elif impl_type:
        call = f"{impl_type}::{name}({args_str})"
    else:
        call = f"{name}({args_str})"

    if is_unsafe:
        lines.append(f"        unsafe {{ let _r = {call}; }}")
    else:
        lines.append(f"        let _r = {call};")

    # Layer 2: finite output check for f64 returns
    if return_type and not is_unsafe:
        rt = return_type.strip()
        if rt == "f64":
            lines.append(f"        assert!(_r.is_finite());")
        elif rt == "f32":
            lines.append(f"        assert!(_r.is_finite());")

    lines.append(f"    }}")
    return "\n".join(lines)


def main():
    apply = "--apply" in sys.argv

    with open(INPUT) as f:
        data = json.load(f)

    provable = data["provable_functions"]

    # Find private functions (in SKIP_FUNCTIONS but not in EXCLUDE)
    private_fns = []
    for fn in provable:
        key = (fn["module"], fn["name"])
        if fn["params"] and key in SKIP_FUNCTIONS and key not in EXCLUDE:
            private_fns.append(fn)

    # Group by file
    from collections import defaultdict
    by_file = defaultdict(list)
    for fn in private_fns:
        by_file[fn["file"]].append(fn)

    total_generated = 0
    total_skipped = 0

    for fname in sorted(by_file):
        fns = by_file[fname]
        fpath = os.path.join(RUST_SRC, fname)

        harness_lines = []
        for fn in sorted(fns, key=lambda f: f["name"]):
            code = generate_harness_code(fn)
            if code:
                harness_lines.append(code)
                total_generated += 1
            else:
                print(f"  SKIP {fname}::{fn['name']} (no construction)", file=sys.stderr)
                total_skipped += 1

        if not harness_lines:
            continue

        block = "\n\n#[cfg(kani)]\nmod kani_private_proofs {\n    use super::*;\n\n"
        block += "\n\n".join(harness_lines)
        block += "\n}\n"

        if apply:
            with open(fpath, "a") as f:
                f.write(block)
            print(f"  WROTE {fname}: {len(harness_lines)} harnesses", file=sys.stderr)
        else:
            print(f"// === {fname} ({len(harness_lines)} harnesses) ===")
            print(block)

    print(f"\nTotal: {total_generated} generated, {total_skipped} skipped", file=sys.stderr)


if __name__ == "__main__":
    main()
