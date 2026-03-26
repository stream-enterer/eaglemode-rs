#!/usr/bin/env python3
"""Generate contract.json from golden data files and test source code.

Pattern: feature-list-as-immutable-contract (bootstrap)

Scans tests/golden/data/ for all .golden files and tests/golden/*.rs for
the actual tolerance parameters used in compare_* calls. Produces a complete
contract.json with per-case tolerances matching what the code enforces.

Usage: python3 .harness/harness_gen_contract.py > .harness/contract.json
"""

import json
import os
import re
import sys

GOLDEN_DATA = "tests/golden/data"
GOLDEN_SRC = "tests/golden"

CATEGORIES = {
    "painter":      {"oracle": "pixel_comparison",       "comparison_fn": "compare_images",       "golden_ext": ".painter.golden"},
    "compositor":   {"oracle": "pixel_comparison",       "comparison_fn": "compare_images",       "golden_ext": ".compositor.golden"},
    "layout":       {"oracle": "rect_comparison",        "comparison_fn": "compare_rects",        "golden_ext": ".layout.golden"},
    "behavioral":   {"oracle": "exact_match",            "comparison_fn": "compare_behavioral",   "golden_ext": ".behavioral.golden"},
    "trajectory":   {"oracle": "f64_tolerance",          "comparison_fn": "compare_trajectory",   "golden_ext": ".trajectory.golden"},
    "widget_state": {"oracle": "structured_comparison",  "comparison_fn": "compare_widget_state", "golden_ext": ".widget_state.golden"},
    "notice":       {"oracle": "flag_translation_exact",  "comparison_fn": "compare_notices",      "golden_ext": ".notice.golden"},
    "input":        {"oracle": "exact_match",            "comparison_fn": "compare_input",        "golden_ext": ".input.golden"},
}

# Default tolerances when source parsing fails
DEFAULT_TOLERANCES = {
    "painter":      {"channel_tolerance": 1, "max_failure_pct": 0.5},
    "compositor":   {"channel_tolerance": 2, "max_failure_pct": 0.5},
    "layout":       {"eps": 1e-6},
    "trajectory":   {"tolerance": 1e-6},
    "notice":       {"mask": "0x0FFF"},
    "behavioral":   {},
    "input":        {},
    "widget_state": {},
}


def load_all_rs_source():
    """Load all .rs files in tests/golden/ into a dict keyed by filename."""
    sources = {}
    for f in os.listdir(GOLDEN_SRC):
        if f.endswith(".rs"):
            path = os.path.join(GOLDEN_SRC, f)
            with open(path) as fh:
                sources[f] = fh.read()
    return sources


def find_image_tolerance(name, sources):
    """Find compare_images("name", ..., ch_tol, max_fail_pct) in source."""
    pattern = re.compile(
        r'compare_images\(\s*"' + re.escape(name) + r'"'
        r'[\s\S]*?,\s*(\d+)\s*,\s*([0-9.]+)\s*\)',
        re.MULTILINE
    )
    for src in sources.values():
        m = pattern.search(src)
        if m:
            return {
                "channel_tolerance": int(m.group(1)),
                "max_failure_pct": float(m.group(2)),
            }
    return None


def find_rect_tolerance(name, sources):
    """Find compare_rects(..., eps) near a test using this golden name."""
    for src in sources.values():
        if f'"{name}"' not in src:
            continue
        # Find compare_rects calls and extract the last float argument
        # Pattern: compare_rects(&actual, &expected, eps)
        # The eps is typically on its own line or as the last arg
        idx = src.find(f'"{name}"')
        # Search within ~1000 chars after the name reference
        region = src[idx:idx+1500]
        m = re.search(r'compare_rects\([\s\S]*?,\s*([0-9]+\.?[0-9]*(?:e[+-]?[0-9]+)?)\s*\)', region)
        if m:
            return {"eps": float(m.group(1))}
    return None


def find_trajectory_tolerance(name, sources):
    """Find compare_trajectory(..., tolerance) near a test using this golden name."""
    for src in sources.values():
        if f'"{name}"' not in src:
            continue
        idx = src.find(f'"{name}"')
        region = src[idx:idx+1500]
        m = re.search(r'compare_trajectory\([\s\S]*?,\s*([0-9]+\.?[0-9]*(?:e[+-]?[0-9]+)?)\s*\)', region)
        if m:
            return {"tolerance": float(m.group(1))}
    return None


def find_tolerance(name, category, sources):
    """Find the actual tolerance for a test case from source code."""
    if category in ("painter", "compositor"):
        result = find_image_tolerance(name, sources)
        if result:
            return result
    elif category == "layout":
        result = find_rect_tolerance(name, sources)
        if result:
            return result
    elif category == "trajectory":
        result = find_trajectory_tolerance(name, sources)
        if result:
            return result
    elif category == "notice":
        return {"mask": "0x0FFF"}
    elif category in ("behavioral", "input", "widget_state"):
        return {}

    return DEFAULT_TOLERANCES.get(category, {})


def main():
    sources = load_all_rs_source()
    cases = []
    missing_tolerance = []

    for category in sorted(CATEGORIES.keys()):
        cat_dir = os.path.join(GOLDEN_DATA, category)
        if not os.path.isdir(cat_dir):
            continue
        ext = CATEGORIES[category]["golden_ext"]
        for f in sorted(os.listdir(cat_dir)):
            if not f.endswith(".golden"):
                continue
            # Strip .<category>.golden
            name = f
            if name.endswith(ext):
                name = name[:-len(ext)]
            else:
                # Fallback: strip last two dot-separated segments
                parts = name.rsplit(".", 2)
                name = parts[0]

            tolerance = find_tolerance(name, category, sources)
            if tolerance == DEFAULT_TOLERANCES.get(category, {}) and category in ("painter", "compositor", "layout", "trajectory"):
                missing_tolerance.append(f"{category}/{name}")

            cases.append({
                "name": name,
                "category": category,
                "tolerance": tolerance,
            })

    contract = {
        "schema_version": 1,
        "generated_by": "harness_gen_contract.py",
        "categories": CATEGORIES,
        "cases": cases,
    }

    json.dump(contract, sys.stdout, indent=2)
    print()  # trailing newline

    if missing_tolerance:
        print(f"\n# WARNING: {len(missing_tolerance)} cases used default tolerance (source parse failed):", file=sys.stderr)
        for m in missing_tolerance:
            print(f"#   {m}", file=sys.stderr)


if __name__ == "__main__":
    main()
