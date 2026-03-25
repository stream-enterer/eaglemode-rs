#!/usr/bin/env python3
"""Generate Kani proof harnesses for every function in src/emCore/.

For each function, generates a #[kani::proof] harness that:
  1. Creates kani::any() for each parameter
  2. Calls the function
  3. Asserts no panic (implicit — Kani catches panics automatically)

Functions whose parameters can't be constructed via kani::any() will fail
to compile — that's intentional. The compiler is the filter.

Outputs:
  - src/emCore/proofs_generated.rs  (generated harnesses)
  - .kani/gen_report.json           (what was generated and why)

Usage: python3 .kani/gen_harnesses.py
"""

import json
import os
import re
import sys

RUST_SRC = "src/emCore"
OUTPUT_RS = os.path.join(RUST_SRC, "proofs_generated.rs")
OUTPUT_JSON = ".kani/gen_report.json"

# Types that implement kani::Arbitrary (can use kani::any())
KANI_ARBITRARY_TYPES = {
    "u8", "u16", "u32", "u64", "u128", "usize",
    "i8", "i16", "i32", "i64", "i128", "isize",
    "f32", "f64", "bool", "char",
}

# Types we know how to construct for Kani (with custom construction)
CONSTRUCTIBLE_TYPES = {
    "emColor": "emColor::new(kani::any(), kani::any(), kani::any(), kani::any())",
}


def parse_rust_functions(src_dir):
    """Extract all fn declarations from Rust source files."""
    functions = []

    for fname in sorted(os.listdir(src_dir)):
        if not fname.endswith(".rs"):
            continue
        if fname in ("mod.rs", "proofs.rs", "proofs_generated.rs"):
            continue

        fpath = os.path.join(src_dir, fname)
        with open(fpath) as f:
            content = f.read()
            lines = content.split("\n")

        module = fname[:-3]  # strip .rs

        # Track impl blocks
        current_impl = None
        brace_depth = 0

        for line_no, line in enumerate(lines, 1):
            stripped = line.strip()

            # Track brace depth for impl blocks
            brace_depth += stripped.count("{") - stripped.count("}")

            # Detect impl blocks
            m = re.match(r"impl(?:<[^>]*>)?\s+(\w+)", stripped)
            if m:
                current_impl = m.group(1)

            if brace_depth <= 0:
                current_impl = None
                brace_depth = 0

            # Match fn declarations — single-line and multi-line start
            m = re.match(
                r"(?:pub(?:\(crate\))?\s+)?"
                r"(?:unsafe\s+)?"
                r"(?:const\s+)?"
                r"fn\s+(\w+)"
                r"\s*(?:<[^>]*>)?"
                r"\s*\(([^)]*)\)",
                stripped
            )
            if not m:
                # Try multiline: fn name(\n  params\n)
                m = re.match(
                    r"(?:pub(?:\(crate\))?\s+)?"
                    r"(?:unsafe\s+)?"
                    r"(?:const\s+)?"
                    r"fn\s+(\w+)"
                    r"\s*(?:<[^>]*>)?"
                    r"\s*\(",
                    stripped
                )
                if m and ")" not in stripped:
                    # Collect params from subsequent lines
                    fn_name = m.group(1)
                    params_lines = [stripped.split("(", 1)[1] if "(" in stripped else ""]
                    for k in range(line_no, min(line_no + 15, len(lines))):
                        params_lines.append(lines[k].strip())
                        if ")" in lines[k]:
                            break
                    params_str = " ".join(params_lines)
                    params_str = params_str.split(")")[0].strip()
                    captured = {1: fn_name, 2: params_str}
                    m = type('Match', (), {
                        'group': lambda self, i, _c=captured: _c[i]
                    })()

            if not m:
                continue

            name = m.group(1)
            try:
                params_raw = m.group(2).strip()
            except (IndexError, KeyError):
                continue

            # Skip test/proof functions
            if name.startswith("test_") or name.startswith("proof_"):
                continue
            # Skip main, new (constructors need special handling)
            if name in ("main", "fmt"):
                continue

            is_unsafe = "unsafe" in stripped.split("fn")[0] if "fn" in stripped else False

            # Parse individual parameters
            params = parse_params(params_raw)

            functions.append({
                "name": name,
                "module": module,
                "impl_type": current_impl,
                "file": fname,
                "line": line_no,
                "params_raw": params_raw,
                "params": params,
                "is_unsafe": is_unsafe,
                "visibility": "pub" if stripped.startswith("pub") else "private",
            })

    return functions


def parse_params(params_raw):
    """Parse a parameter list into [(name, type)] pairs."""
    if not params_raw or params_raw.isspace():
        return []

    params = []
    # Split by comma, but respect angle brackets and parentheses
    depth = 0
    current = ""
    for ch in params_raw:
        if ch in "<(":
            depth += 1
        elif ch in ">)":
            depth -= 1
        elif ch == "," and depth == 0:
            params.append(current.strip())
            current = ""
            continue
        current += ch
    if current.strip():
        params.append(current.strip())

    result = []
    for p in params:
        p = p.strip()
        if not p:
            continue

        # Handle self variants
        if p in ("self", "&self", "&mut self", "mut self"):
            result.append(("self", p))
            continue

        # Handle name: Type
        m = re.match(r"(\w+)\s*:\s*(.+)", p)
        if m:
            result.append((m.group(1).strip(), m.group(2).strip()))
        else:
            result.append(("_", p))

    return result


def can_construct(type_str):
    """Check if we can construct a value of this type for Kani."""
    # Clean up the type
    t = type_str.strip()

    # Direct kani::any() types
    if t in KANI_ARBITRARY_TYPES:
        return True, f"kani::any::<{t}>()"

    # Known constructible types
    if t in CONSTRUCTIBLE_TYPES:
        return True, CONSTRUCTIBLE_TYPES[t]

    # Arrays of primitives: [u8; N]
    m = re.match(r"\[(\w+);\s*(\d+)\]", t)
    if m and m.group(1) in KANI_ARBITRARY_TYPES:
        return True, f"kani::any::<{t}>()"

    # Tuples of primitives
    if t.startswith("(") and t.endswith(")"):
        inner = t[1:-1]
        parts = [p.strip() for p in inner.split(",")]
        if all(p in KANI_ARBITRARY_TYPES for p in parts):
            return True, f"kani::any::<{t}>()"

    return False, None


def generate_harness(fn_info):
    """Generate a Kani proof harness for a function. Returns (code, skip_reason)."""
    params = fn_info["params"]

    # Skip if it takes self (we can't construct arbitrary instances of most types)
    for pname, ptype in params:
        if pname == "self":
            return None, f"takes {ptype}"

    # Check all param types are constructible
    param_constructions = []
    for pname, ptype in params:
        # Strip mutability
        clean_type = ptype.lstrip("&").strip()
        if clean_type.startswith("mut "):
            clean_type = clean_type[4:].strip()

        constructible, construction = can_construct(clean_type)
        if not constructible:
            return None, f"param '{pname}' has non-constructible type '{ptype}'"

        # If the function takes a reference, we need to create the value first
        if ptype.startswith("&mut "):
            param_constructions.append((pname, clean_type, construction, "ref_mut"))
        elif ptype.startswith("&"):
            param_constructions.append((pname, clean_type, construction, "ref"))
        else:
            param_constructions.append((pname, clean_type, construction, "value"))

    # Build the harness
    module = fn_info["module"]
    name = fn_info["name"]
    impl_type = fn_info["impl_type"]

    harness_name = f"kani_{module}_{name}"
    if impl_type:
        harness_name = f"kani_{module}_{impl_type}_{name}"
    # Sanitize
    harness_name = re.sub(r'[^a-zA-Z0-9_]', '_', harness_name)

    lines = []
    lines.append(f"#[cfg(kani)]")
    lines.append(f"#[kani::proof]")
    if fn_info["is_unsafe"]:
        lines.append(f"fn {harness_name}() {{")
    else:
        lines.append(f"fn {harness_name}() {{")

    # Create parameters
    call_args = []
    for pname, clean_type, construction, mode in param_constructions:
        var = f"p_{pname}"
        lines.append(f"    let mut {var}: {clean_type} = {construction};")
        if mode == "ref_mut":
            call_args.append(f"&mut {var}")
        elif mode == "ref":
            call_args.append(f"&{var}")
        else:
            call_args.append(var)

    # Build call
    args_str = ", ".join(call_args)
    if impl_type:
        # It's a method — but we already filtered out &self, so it must be an associated fn
        call = f"crate::emCore::{module}::{impl_type}::{name}({args_str})"
    else:
        call = f"crate::emCore::{module}::{name}({args_str})"

    if fn_info["is_unsafe"]:
        lines.append(f"    unsafe {{ let _ = {call}; }}")
    else:
        lines.append(f"    let _ = {call};")

    lines.append("}")
    lines.append("")

    return "\n".join(lines), None


def main():
    print("Parsing Rust source...", file=sys.stderr)
    functions = parse_rust_functions(RUST_SRC)
    print(f"  Found {len(functions)} functions", file=sys.stderr)

    generated = []
    skipped = []
    harness_code = []

    harness_code.append("// AUTO-GENERATED by .kani/gen_harnesses.py")
    harness_code.append("// Do not edit manually. Regenerate with: python3 .kani/gen_harnesses.py")
    harness_code.append("//")
    harness_code.append("// Each harness calls one function with kani::any() inputs.")
    harness_code.append("// Functions that panic for any input will be flagged by Kani.")
    harness_code.append("")

    for fn_info in functions:
        code, skip_reason = generate_harness(fn_info)
        if code:
            harness_code.append(code)
            generated.append({
                "harness": f"kani_{fn_info['module']}_{fn_info.get('impl_type', '')}_{fn_info['name']}".replace("__", "_"),
                "function": fn_info["name"],
                "module": fn_info["module"],
                "impl_type": fn_info.get("impl_type", ""),
                "file": fn_info["file"],
                "line": fn_info["line"],
            })
        else:
            skipped.append({
                "function": fn_info["name"],
                "module": fn_info["module"],
                "impl_type": fn_info.get("impl_type", ""),
                "file": fn_info["file"],
                "line": fn_info["line"],
                "reason": skip_reason,
            })

    # Write harness source
    with open(OUTPUT_RS, "w") as f:
        f.write("\n".join(harness_code))
    print(f"  Wrote {len(generated)} harnesses to {OUTPUT_RS}", file=sys.stderr)

    # Write report
    report = {
        "total_functions": len(functions),
        "harnesses_generated": len(generated),
        "skipped": len(skipped),
        "generated": generated,
        "skipped_detail": skipped,
    }
    with open(OUTPUT_JSON, "w") as f:
        json.dump(report, f, indent=2)
    print(f"  Report: {OUTPUT_JSON}", file=sys.stderr)

    print(f"\n{'='*50}", file=sys.stderr)
    print(f"  Total functions:  {len(functions)}", file=sys.stderr)
    print(f"  Harnesses:        {len(generated)}", file=sys.stderr)
    print(f"  Skipped:          {len(skipped)}", file=sys.stderr)
    print(f"{'='*50}", file=sys.stderr)
    print(f"\nNext: cargo kani 2>&1 | tee .kani/kani_output.log", file=sys.stderr)


if __name__ == "__main__":
    main()
