#!/usr/bin/env python3
"""Factual comparison of filenames between Eagle Mode's emCore headers
and zuicchini's Rust emCore directory.

Reports every file on each side with the facts: what exists, what markers
are present, and which names appear on both sides. No interpretation of
splits, infrastructure, or intent.
"""

import os
import sys
from pathlib import Path

RUST_DIR = Path(__file__).resolve().parent.parent / "src" / "emCore"
CPP_DIR = Path(os.path.expanduser("~/git/eaglemode-0.96.4/include/emCore"))


def collect_cpp(d: Path) -> set[str]:
    """Return the set of .h basenames (without extension)."""
    return {f.stem for f in d.iterdir() if f.suffix == ".h"}


def collect_rust(d: Path) -> tuple[dict[str, list[str]], list[str]]:
    """Walk the Rust dir and return:
      - stems: {stem: [extensions...]}   e.g. {"emColor": [".rs", ".no_rust_equivalent"]}
      - other: list of entries that don't follow the stem.ext pattern (dirs, etc.)
    """
    stems: dict[str, list[str]] = {}
    other: list[str] = []
    for entry in sorted(d.iterdir()):
        name = entry.name
        if entry.is_dir():
            other.append(name + "/")
            continue
        # Peel off known compound extensions first
        for ext in (".no_rust_equivalent", ".rust_only", ".rs"):
            if name.endswith(ext):
                stem = name.removesuffix(ext)
                stems.setdefault(stem, []).append(ext)
                break
        else:
            other.append(name)
    return stems, other


def main():
    if not CPP_DIR.is_dir():
        sys.exit(f"C++ header dir not found: {CPP_DIR}")
    if not RUST_DIR.is_dir():
        sys.exit(f"Rust emCore dir not found: {RUST_DIR}")

    cpp_stems = collect_cpp(CPP_DIR)
    rust_stems, rust_other = collect_rust(RUST_DIR)

    all_stems = sorted(cpp_stems | rust_stems.keys())

    # --- Per-stem table ---
    rows = []
    for stem in all_stems:
        has_h = stem in cpp_stems
        rust_exts = rust_stems.get(stem, [])
        cpp_col = f"{stem}.h" if has_h else ""
        rust_col = ", ".join(f"{stem}{ext}" for ext in sorted(rust_exts)) if rust_exts else ""
        rows.append((cpp_col, rust_col))

    cpp_w = max((len(r[0]) for r in rows), default=0)
    rust_w = max((len(r[1]) for r in rows), default=0)
    cpp_w = max(cpp_w, len("C++ header"))
    rust_w = max(rust_w, len("Rust files"))

    hdr = f"  {'C++ header':<{cpp_w}}    {'Rust files':<{rust_w}}"
    sep = f"  {'-'*cpp_w}    {'-'*rust_w}"
    print(hdr)
    print(sep)
    for cpp_col, rust_col in rows:
        print(f"  {cpp_col:<{cpp_w}}    {rust_col:<{rust_w}}")

    # --- Other Rust entries (dirs, non-.rs files) ---
    if rust_other:
        print(f"\nRust entries with no stem mapping:")
        for name in rust_other:
            print(f"  {name}")

    # --- Summary counts ---
    print(f"\nC++ headers:             {len(cpp_stems)}")
    print(f"Rust stems:              {len(rust_stems)}")
    rs_count = sum(1 for exts in rust_stems.values() if ".rs" in exts)
    marker_no = sum(1 for exts in rust_stems.values() if ".no_rust_equivalent" in exts)
    marker_only = sum(1 for exts in rust_stems.values() if ".rust_only" in exts)
    print(f"  .rs files:             {rs_count}")
    print(f"  .no_rust_equivalent:   {marker_no}")
    print(f"  .rust_only:            {marker_only}")

    both = sum(1 for s in all_stems if s in cpp_stems and ".rs" in rust_stems.get(s, []))
    cpp_only = sum(1 for s in all_stems if s in cpp_stems and s not in rust_stems)
    cpp_marker = sum(
        1 for s in all_stems
        if s in cpp_stems and ".no_rust_equivalent" in rust_stems.get(s, [])
    )
    rust_no_cpp = sum(1 for s in all_stems if s not in cpp_stems)
    print(f"\nStems with both .h and .rs:        {both}")
    print(f"Stems with .h and .no_rust_equivalent: {cpp_marker}")
    print(f"Stems with .h but nothing in Rust: {cpp_only}")
    print(f"Stems in Rust with no .h:          {rust_no_cpp}")


if __name__ == "__main__":
    main()
