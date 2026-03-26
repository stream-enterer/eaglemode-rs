#!/usr/bin/env python3
"""Extract C++ and Rust function signatures, match by name, produce inventory JSON.

Stage 1 of the Kani pipeline. Parses C++ headers and Rust source files to build
a function-level correspondence map. Outputs .kani/kani_inventory.json.

Usage: python3 .kani/harness_kani_inventory.py
"""

import json
import os
import re
import sys

CPP_INCLUDE = os.path.expanduser("~/git/eaglemode-0.96.4/include/emCore")
CPP_SRC = os.path.expanduser("~/git/eaglemode-0.96.4/src/emCore")
RUST_SRC = "src/emCore"
OUTPUT = ".kani/kani_inventory.json"


def extract_cpp_functions(header_dir, src_dir):
    """Extract function/method names from C++ headers and source files."""
    functions = []  # list of {class, name, file, line, signature, return_type, params}

    for d in [header_dir, src_dir]:
        if not os.path.isdir(d):
            continue
        for fname in sorted(os.listdir(d)):
            if not (fname.endswith(".h") or fname.endswith(".cpp")):
                continue
            fpath = os.path.join(d, fname)
            with open(fpath, errors="replace") as f:
                content = f.read()

            # Track current class context
            current_class = None
            for line_no, line in enumerate(content.split("\n"), 1):
                stripped = line.strip()

                # Detect class declarations
                m = re.match(r"class\s+(\w+)\s*[:{]", stripped)
                if m:
                    current_class = m.group(1)
                    continue

                # Detect end of class (heuristic: closing brace at indent 0)
                if stripped == "};":
                    current_class = None
                    continue

                # Match method/function declarations
                # Pattern: [return_type] name ( params ) [const] [;]
                m = re.match(
                    r"(?:static\s+)?(?:virtual\s+)?(?:inline\s+)?"
                    r"([\w:*&<> ]+?)\s+"  # return type
                    r"(\w+)"              # function name
                    r"\s*\(([^)]*)\)"     # params
                    r"(.*?)"              # const, override, etc.
                    r"\s*[;{]",           # end
                    stripped
                )
                if m:
                    ret_type = m.group(1).strip()
                    name = m.group(2).strip()
                    params = m.group(3).strip()

                    # Skip constructors, destructors, operators, macros
                    if name.startswith("~") or name.startswith("operator"):
                        continue
                    if name.isupper() and "_" in name:  # likely a macro
                        continue
                    if ret_type in ("if", "for", "while", "switch", "return", "case", "else"):
                        continue

                    # Classify purity heuristically
                    is_const = "const" in m.group(4)
                    is_static = "static" in stripped[:stripped.find(ret_type)]
                    has_simple_params = all(
                        t in params or params == ""
                        for t in []  # we'll check types below
                    )

                    # Check if params are all primitive types
                    primitive_types = {"int", "unsigned", "float", "double", "bool",
                                       "emByte", "emUInt8", "emUInt16", "emUInt32",
                                       "emUInt64", "emInt8", "emInt16", "emInt32",
                                       "emInt64", "char", "size_t", "const"}
                    param_words = set(re.findall(r'\b\w+\b', params)) - {"const", "unsigned"}
                    param_types_simple = all(
                        w in primitive_types or w.startswith("em") or w[0].islower()
                        for w in param_words
                    ) if param_words else True

                    functions.append({
                        "class": current_class or "",
                        "name": name,
                        "qualified": f"{current_class}::{name}" if current_class else name,
                        "file": fname,
                        "line": line_no,
                        "return_type": ret_type,
                        "params": params,
                        "is_const": is_const,
                        "primitive_params": param_types_simple and is_const,
                    })

    return functions


def extract_rust_functions(src_dir):
    """Extract function signatures from Rust source files."""
    functions = []

    for fname in sorted(os.listdir(src_dir)):
        if not fname.endswith(".rs"):
            continue
        fpath = os.path.join(src_dir, fname)
        with open(fpath) as f:
            content = f.read()

        # Track current impl block
        current_impl = None
        for line_no, line in enumerate(content.split("\n"), 1):
            stripped = line.strip()

            # Detect impl blocks
            m = re.match(r"impl(?:<[^>]*>)?\s+(\w+)", stripped)
            if m:
                current_impl = m.group(1)

            # Match function declarations
            m = re.match(
                r"(?:pub(?:\(crate\))?\s+)?"
                r"(?:unsafe\s+)?"
                r"fn\s+(\w+)"    # function name
                r"\s*(?:<[^>]*>)?"  # generics
                r"\s*\(([^)]*)\)"   # params (may be incomplete for multiline)
                r"(?:\s*->\s*(.+?))?"  # return type
                r"\s*\{?",
                stripped
            )
            if m:
                name = m.group(1)
                params = m.group(2).strip()
                ret = (m.group(3) or "()").strip()

                # Skip test functions
                if name.startswith("test_") or name.startswith("proof_"):
                    continue

                # Check if params are primitive
                primitive_rs = {"u8", "u16", "u32", "u64", "i8", "i16", "i32",
                                "i64", "f32", "f64", "bool", "usize", "isize"}
                param_tokens = set(re.findall(r'\b\w+\b', params))
                # Remove self, mut, ref keywords
                param_tokens -= {"self", "mut", "ref", "const", "unsafe"}
                is_pure_candidate = (
                    "&mut self" not in params
                    and "&self" not in params or "& self" not in params  # allow &self for getters
                )
                has_primitive_params = all(
                    t in primitive_rs or t.startswith("em") or t[0].islower()
                    for t in param_tokens
                ) if param_tokens else True

                is_unsafe = "unsafe" in stripped

                functions.append({
                    "impl": current_impl or "",
                    "name": name,
                    "qualified": f"{current_impl}::{name}" if current_impl else name,
                    "file": fname,
                    "line": line_no,
                    "return_type": ret,
                    "params": params,
                    "is_unsafe": is_unsafe,
                    "pure_candidate": is_pure_candidate and has_primitive_params,
                    "visibility": "pub" if "pub" in stripped else "private",
                })

    return functions


def match_functions(cpp_fns, rust_fns):
    """Match C++ and Rust functions by name."""
    # Build lookup by unqualified name
    cpp_by_name = {}
    for fn in cpp_fns:
        key = fn["name"]
        if key not in cpp_by_name:
            cpp_by_name[key] = []
        cpp_by_name[key].append(fn)

    rust_by_name = {}
    for fn in rust_fns:
        key = fn["name"]
        if key not in rust_by_name:
            rust_by_name[key] = []
        rust_by_name[key].append(fn)

    matched = []
    unmatched_cpp = []
    unmatched_rust = []

    all_names = set(cpp_by_name.keys()) | set(rust_by_name.keys())

    for name in sorted(all_names):
        cpp_list = cpp_by_name.get(name, [])
        rust_list = rust_by_name.get(name, [])

        if cpp_list and rust_list:
            # Match — take first of each for now
            for c in cpp_list:
                best_rust = None
                # Try to match by class/impl name
                for r in rust_list:
                    if c["class"] and r["impl"] and c["class"] == r["impl"]:
                        best_rust = r
                        break
                if not best_rust:
                    best_rust = rust_list[0]

                matched.append({
                    "name": name,
                    "cpp_qualified": c["qualified"],
                    "rust_qualified": best_rust["qualified"],
                    "cpp_file": c["file"],
                    "rust_file": best_rust["file"],
                    "cpp_line": c["line"],
                    "rust_line": best_rust["line"],
                    "cpp_return": c["return_type"],
                    "rust_return": best_rust["return_type"],
                    "cpp_params": c["params"],
                    "rust_params": best_rust["params"],
                    "cpp_primitive": c.get("primitive_params", False),
                    "rust_pure_candidate": best_rust.get("pure_candidate", False),
                    "kani_candidate": (
                        best_rust.get("pure_candidate", False)
                        and c.get("primitive_params", False)
                    ),
                })
        elif cpp_list and not rust_list:
            for c in cpp_list:
                unmatched_cpp.append({
                    "name": name,
                    "qualified": c["qualified"],
                    "file": c["file"],
                    "line": c["line"],
                    "return_type": c["return_type"],
                    "params": c["params"],
                })
        elif rust_list and not cpp_list:
            for r in rust_list:
                unmatched_rust.append({
                    "name": name,
                    "qualified": r["qualified"],
                    "file": r["file"],
                    "line": r["line"],
                    "return_type": r["return_type"],
                    "params": r["params"],
                    "pure_candidate": r.get("pure_candidate", False),
                })

    return matched, unmatched_cpp, unmatched_rust


def main():
    print("Extracting C++ functions...", file=sys.stderr)
    cpp_fns = extract_cpp_functions(CPP_INCLUDE, CPP_SRC)
    print(f"  Found {len(cpp_fns)} C++ functions/methods", file=sys.stderr)

    print("Extracting Rust functions...", file=sys.stderr)
    rust_fns = extract_rust_functions(RUST_SRC)
    print(f"  Found {len(rust_fns)} Rust functions/methods", file=sys.stderr)

    print("Matching by name...", file=sys.stderr)
    matched, unmatched_cpp, unmatched_rust = match_functions(cpp_fns, rust_fns)

    kani_candidates = [m for m in matched if m.get("kani_candidate")]
    rust_pure = [r for r in rust_fns if r.get("pure_candidate")]

    inventory = {
        "summary": {
            "cpp_total": len(cpp_fns),
            "rust_total": len(rust_fns),
            "matched": len(matched),
            "unmatched_cpp": len(unmatched_cpp),
            "unmatched_rust": len(unmatched_rust),
            "kani_candidates": len(kani_candidates),
            "rust_pure_candidates": len(rust_pure),
        },
        "matched": matched,
        "unmatched_cpp": unmatched_cpp,
        "unmatched_rust": unmatched_rust,
        "kani_candidates": kani_candidates,
    }

    os.makedirs(os.path.dirname(OUTPUT), exist_ok=True)
    with open(OUTPUT, "w") as f:
        json.dump(inventory, f, indent=2)
    print(f"\nInventory written to {OUTPUT}", file=sys.stderr)

    # Print summary
    print(f"\n{'='*60}", file=sys.stderr)
    print(f"  C++ functions:       {len(cpp_fns)}", file=sys.stderr)
    print(f"  Rust functions:      {len(rust_fns)}", file=sys.stderr)
    print(f"  Matched by name:     {len(matched)}", file=sys.stderr)
    print(f"  Unmatched C++:       {len(unmatched_cpp)}", file=sys.stderr)
    print(f"  Unmatched Rust:      {len(unmatched_rust)}", file=sys.stderr)
    print(f"  Kani candidates:     {len(kani_candidates)}", file=sys.stderr)
    print(f"  Rust pure functions: {len(rust_pure)}", file=sys.stderr)
    print(f"{'='*60}", file=sys.stderr)


if __name__ == "__main__":
    main()
