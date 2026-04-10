#!/usr/bin/env python3
"""Compare C++ and Rust DrawOp JSONL files parameter-by-parameter.

Usage:
    python3 scripts/diff_draw_ops.py <test_name> [divergence_dir]
    python3 scripts/diff_draw_ops.py cosmos_item_border
    python3 scripts/diff_draw_ops.py testpanel_root crates/eaglemode/target/golden-divergence
"""

import json
import sys
from pathlib import Path

FLOAT_TOL = 1e-10
SKIP_KEYS = {"seq", "_unserialized"}
# State ops that may appear in one side but not the other.
# C++ passes canvas_color per-call; Rust has explicit SetCanvasColor ops.
STATE_OPS = {"SetCanvasColor", "SetAlpha", "PushState", "PopState", "SetOffset", "ClipRect"}


def load_ops(path):
    ops = []
    with open(path) as f:
        buf = ""
        for line in f:
            buf += line.rstrip("\n")
            # A complete JSONL line starts with { and ends with }
            if buf.strip().startswith("{") and buf.strip().endswith("}"):
                try:
                    # Escape control chars in text fields (C++ doesn't escape newlines)
                    cleaned = buf.replace("\t", "\\t")
                    ops.append(json.loads(cleaned))
                except json.JSONDecodeError:
                    # Try escaping embedded newlines in string values
                    import re
                    cleaned = re.sub(r'(?<=:")[^"]*(?=")', lambda m: m.group().replace("\n", "\\n"), buf)
                    try:
                        ops.append(json.loads(cleaned))
                    except json.JSONDecodeError:
                        pass  # skip unparseable lines
                buf = ""
            elif buf.strip().startswith("{"):
                # Incomplete line — append newline escape and continue
                buf += "\\n"
            else:
                buf = ""
    return ops


def fmt(v):
    if isinstance(v, float):
        return f"{v:.15g}"
    if isinstance(v, str) and len(v) > 40:
        return v[:37] + "..."
    return str(v)


def diff_ops(cpp_ops, rust_ops, name):
    divergences = []
    min_len = min(len(cpp_ops), len(rust_ops))

    for i in range(min_len):
        cpp = cpp_ops[i]
        rust = rust_ops[i]

        cpp_op = cpp.get("op", "?")
        rust_op = rust.get("op", "?")

        if cpp_op != rust_op:
            divergences.append(
                (i, f"{cpp_op}/{rust_op}", "op", cpp_op, rust_op, "TYPE MISMATCH")
            )
            break

        all_keys = (set(cpp.keys()) | set(rust.keys())) - SKIP_KEYS
        for key in sorted(all_keys):
            cv = cpp.get(key)
            rv = rust.get(key)
            if cv is None:
                divergences.append((i, cpp_op, key, "(missing)", fmt(rv), "RUST EXTRA"))
                continue
            if rv is None:
                divergences.append((i, cpp_op, key, fmt(cv), "(missing)", "C++ EXTRA"))
                continue
            if isinstance(cv, float) and isinstance(rv, float):
                d = abs(cv - rv)
                if d > FLOAT_TOL:
                    divergences.append((i, cpp_op, key, fmt(cv), fmt(rv), f"{d:.6e}"))
            elif cv != rv:
                divergences.append((i, cpp_op, key, fmt(cv), fmt(rv), "MISMATCH"))

    if len(cpp_ops) != len(rust_ops):
        divergences.append(
            (min_len, "(count)", "op_count",
             str(len(cpp_ops)), str(len(rust_ops)),
             f"C++={len(cpp_ops)} Rust={len(rust_ops)}")
        )

    print(f"\n=== {name}: {len(divergences)} divergence(s) in {min_len} ops ===")
    if not divergences:
        print("  IDENTICAL")
        return 0

    print(f"{'seq':>4}  {'op':<28} {'param':<20} {'C++':<24} {'Rust':<24} {'delta'}")
    print(f"{'---':>4}  {'---':<28} {'---':<20} {'---':<24} {'---':<24} {'---'}")
    for seq, op, param, cv, rv, delta in divergences:
        print(f"{seq:>4}  {op:<28} {param:<20} {str(cv):<24} {str(rv):<24} {delta}")

    return len(divergences)


def main():
    if len(sys.argv) < 2:
        print("Usage: diff_draw_ops.py <test_name> [divergence_dir]")
        sys.exit(1)

    name = sys.argv[1]
    div_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(
        "crates/eaglemode/target/golden-divergence"
    )

    cpp_path = div_dir / f"{name}.cpp_ops.jsonl"
    rust_path = div_dir / f"{name}.rust_ops.jsonl"

    missing = []
    if not cpp_path.exists():
        missing.append(f"  C++:  {cpp_path}  (run: make -C crates/eaglemode/tests/golden/gen run)")
    if not rust_path.exists():
        missing.append(f"  Rust: {rust_path}  (run: DUMP_DRAW_OPS=1 cargo test --test golden {name})")
    if missing:
        print(f"Missing files for '{name}':")
        for m in missing:
            print(m)
        sys.exit(1)

    cpp_ops = load_ops(cpp_path)
    rust_ops = load_ops(rust_path)

    # Full comparison (including state ops)
    n = diff_ops(cpp_ops, rust_ops, name)

    # Paint-only comparison (filter state ops for alignment)
    cpp_paint = [o for o in cpp_ops if o.get("op") not in STATE_OPS]
    rust_paint = [o for o in rust_ops if o.get("op") not in STATE_OPS]
    n2 = diff_ops(cpp_paint, rust_paint, f"{name} (paint ops only)")

    sys.exit(1 if (n > 0 or n2 > 0) else 0)


if __name__ == "__main__":
    main()
