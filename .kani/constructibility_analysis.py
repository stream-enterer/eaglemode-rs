#!/usr/bin/env python3
"""Type-system Kani constructibility analysis.

Determines which functions are formally verifiable with Kani by:
1. Parsing all struct definitions in src/emCore/*.rs
2. Recursively determining which types are constructible from primitives
3. For each constructible type, finding all impl block methods
4. For each method, checking if all parameters are constructible
5. Outputting the complete inventory as JSON

A type is "Kani-constructible" if kani::any() can produce it, or if all
its fields are themselves constructible (so we can build it field-by-field).

Output: .kani/provable_functions.json

Usage: python3 .kani/constructibility_analysis.py
"""

import json
import os
import re
import sys
from collections import defaultdict

RUST_SRC = "src/emCore"
OUTPUT = ".kani/provable_functions.json"

# Primitive types that implement kani::Arbitrary
PRIMITIVES = {
    "u8", "u16", "u32", "u64", "u128", "usize",
    "i8", "i16", "i32", "i64", "i128", "isize",
    "f32", "f64", "bool", "char",
}

# Types with private fields that need public constructors.
# Maps type name -> public construction expression using kani::any() inputs.
# Types with all-pub fields don't need an entry here (struct literal works).
PUBLIC_CONSTRUCTORS = {
    "emColor": "crate::emCore::emColor::emColor::rgba(kani::any(), kani::any(), kani::any(), kani::any())",
    "Fixed12": "crate::emCore::fixed::Fixed12::from_raw(kani::any())",
    "StartFlags": "crate::emCore::emProcess::StartFlags::empty()",
    "emColorRec": "crate::emCore::emRecRecTypes::emColorRec::new(crate::emCore::emColor::emColor::rgba(kani::any(), kani::any(), kani::any(), kani::any()), kani::any())",
    "emKineticViewAnimator": "crate::emCore::emViewAnimator::emKineticViewAnimator::new(kani::any(), kani::any(), kani::any(), kani::any())",
    "emSpeedingViewAnimator": "crate::emCore::emViewAnimator::emSpeedingViewAnimator::new(kani::any())",
    "emSwipingViewAnimator": "crate::emCore::emViewAnimator::emSwipingViewAnimator::new(kani::any())",
    "emMagneticViewAnimator": "crate::emCore::emViewAnimator::emMagneticViewAnimator::new(kani::any())",
    "emRenderThreadPool": "crate::emCore::emRenderThreadPool::emRenderThreadPool::new(kani::any())",
    # Private fields or private struct — skip these
    "PackRect": None,
    "SubPixelEdges": None,
    "ClipRect": None,  # private struct in emPainter
    # PanelId is a slotmap key — not constructible without a slotmap
    "PanelId": None,
    "SignalId": None,
    "EngineId": None,
    "TimerId": None,
    "PriSchedAgentId": None,
    "JobId": None,
    "RecListenerId": None,
    "ControlCreator": None,
    "emMiniIpcClient": None,
    "emRecFileReader": None,
    "emRecFileWriter": None,
    "SignalConnection": None,
}


def read_file(path):
    with open(path) as f:
        return f.read()


def parse_structs(src_dir):
    """Parse all struct definitions, returning {name: {fields: [(name, type)], file, line, kind}}."""
    structs = {}

    for fname in sorted(os.listdir(src_dir)):
        if not fname.endswith(".rs"):
            continue
        fpath = os.path.join(src_dir, fname)
        content = read_file(fpath)
        lines = content.split("\n")

        i = 0
        while i < len(lines):
            line = lines[i].strip()

            # Match struct declarations
            # Tuple struct: pub struct Foo(type);
            m = re.match(r'(?:pub(?:\(crate\))?\s+)?struct\s+(\w+)\s*\(([^)]*)\)\s*;', line)
            if m:
                name = m.group(1)
                inner = m.group(2).strip()
                # Parse tuple fields
                fields = []
                if inner:
                    for j, field in enumerate(split_type_list(inner)):
                        field = field.strip()
                        # Remove pub/pub(crate) prefix
                        field = re.sub(r'^pub(\(crate\))?\s+', '', field)
                        fields.append((f"_{j}", field))
                structs[name] = {
                    "fields": fields,
                    "file": fname,
                    "line": i + 1,
                    "kind": "tuple",
                }
                i += 1
                continue

            # Named struct: pub struct Foo { ... }
            m = re.match(r'(?:pub(?:\(crate\))?\s+)?struct\s+(\w+)(?:<[^>]*>)?\s*\{', line)
            if m:
                name = m.group(1)
                fields = []
                i += 1
                brace_depth = 1
                while i < len(lines) and brace_depth > 0:
                    fline = lines[i].strip()
                    brace_depth += fline.count("{") - fline.count("}")
                    if brace_depth <= 0:
                        break
                    # Parse field: [pub] name: Type,
                    fm = re.match(r'(?:pub(?:\(crate\))?\s+)?(\w+)\s*:\s*(.+?)\s*,?\s*$', fline)
                    if fm:
                        field_name = fm.group(1)
                        field_type = fm.group(2).rstrip(",").strip()
                        fields.append((field_name, field_type))
                    i += 1
                structs[name] = {
                    "fields": fields,
                    "file": fname,
                    "line": (m.start() if hasattr(m, 'start') else 0) + 1,
                    "kind": "named",
                }
                i += 1
                continue

            # Unit struct: pub struct Foo;
            m = re.match(r'(?:pub(?:\(crate\))?\s+)?struct\s+(\w+)\s*;', line)
            if m:
                name = m.group(1)
                structs[name] = {
                    "fields": [],
                    "file": fname,
                    "line": i + 1,
                    "kind": "unit",
                }
                i += 1
                continue

            i += 1

    return structs


def split_type_list(s):
    """Split a comma-separated type list, respecting angle brackets and parens."""
    parts = []
    depth = 0
    current = ""
    for ch in s:
        if ch in "<([":
            depth += 1
        elif ch in ">)]":
            depth -= 1
        elif ch == "," and depth == 0:
            parts.append(current.strip())
            current = ""
            continue
        current += ch
    if current.strip():
        parts.append(current.strip())
    return parts


def is_type_constructible(type_str, structs, memo, depth=0):
    """Recursively check if a type is Kani-constructible.

    Returns (bool, construction_expr_or_None).
    """
    if depth > 20:
        return False, None

    t = type_str.strip()

    # Check memo
    if t in memo:
        return memo[t]

    # Prevent infinite recursion
    memo[t] = (False, None)

    # Primitives
    if t in PRIMITIVES:
        result = (True, f"kani::any::<{t}>()")
        memo[t] = result
        return result

    # References — strip and check inner
    if t.startswith("&mut "):
        inner = t[5:].strip()
        ok, expr = is_type_constructible(inner, structs, memo, depth + 1)
        if ok:
            result = (True, f"&mut {expr}")
            memo[t] = result
            return result
        return False, None

    if t.startswith("&"):
        inner = t[1:].strip()
        ok, expr = is_type_constructible(inner, structs, memo, depth + 1)
        if ok:
            result = (True, f"&{expr}")
            memo[t] = result
            return result
        return False, None

    # Arrays: [T; N]
    m = re.match(r'\[(.+?);\s*(\d+)\]', t)
    if m:
        elem = m.group(1).strip()
        ok, _ = is_type_constructible(elem, structs, memo, depth + 1)
        if ok:
            result = (True, f"kani::any::<{t}>()")
            memo[t] = result
            return result
        return False, None

    # Tuples: (T1, T2, ...)
    if t.startswith("(") and t.endswith(")"):
        inner = t[1:-1]
        parts = split_type_list(inner)
        if all(is_type_constructible(p.strip(), structs, memo, depth + 1)[0] for p in parts):
            result = (True, f"kani::any::<{t}>()")
            memo[t] = result
            return result
        return False, None

    # Option<T>
    m = re.match(r'Option<(.+)>', t)
    if m:
        inner = m.group(1).strip()
        ok, expr = is_type_constructible(inner, structs, memo, depth + 1)
        if ok:
            # Kani can construct Option<T> if T is Arbitrary
            result = (True, f"Some({expr})")
            memo[t] = result
            return result
        return False, None

    # Check for public constructor override
    if t in PUBLIC_CONSTRUCTORS:
        override = PUBLIC_CONSTRUCTORS[t]
        if override is None:
            # Explicitly marked as not constructible
            return False, None
        result = (True, override)
        memo[t] = result
        return result

    # Known structs from the codebase
    if t in structs:
        info = structs[t]
        fields = info["fields"]

        if not fields:
            # Unit struct
            result = (True, f"{t}")
            memo[t] = result
            return result

        # Check all fields
        field_constructions = []
        all_ok = True
        for fname, ftype in fields:
            ok, expr = is_type_constructible(ftype, structs, memo, depth + 1)
            if not ok:
                all_ok = False
                break
            field_constructions.append((fname, expr))

        if all_ok:
            if info["kind"] == "tuple":
                args = ", ".join(expr for _, expr in field_constructions)
                result = (True, f"{t}({args})")
            else:
                args = ", ".join(f"{fname}: {expr}" for fname, expr in field_constructions)
                result = (True, f"{t} {{ {args} }}")
            memo[t] = result
            return result

    return False, None


def parse_impl_methods(src_dir):
    """Parse all impl blocks, extracting method signatures.

    Returns list of {name, impl_type, file, line, params: [(name, type)],
                     is_unsafe, has_self, self_kind, return_type}
    """
    methods = []

    for fname in sorted(os.listdir(src_dir)):
        if not fname.endswith(".rs"):
            continue
        if fname in ("mod.rs", "proofs.rs", "proofs_generated.rs"):
            continue

        fpath = os.path.join(src_dir, fname)
        content = read_file(fpath)
        lines = content.split("\n")
        module = fname[:-3]

        # Track impl blocks with brace counting
        i = 0
        impl_stack = []  # stack of (impl_type, brace_depth_at_start)
        brace_depth = 0

        while i < len(lines):
            line = lines[i]
            stripped = line.strip()

            # Skip comments
            if stripped.startswith("//"):
                i += 1
                continue

            # Skip #[cfg(test)] mod tests blocks
            if stripped == "#[cfg(test)]":
                i += 1
                continue

            # Detect impl blocks
            # impl Foo { ... }
            # impl<T> Foo<T> { ... }
            # impl Trait for Foo { ... }
            impl_match = re.match(
                r'impl(?:<[^>]*>)?\s+'
                r'(?:\w+(?:<[^>]*>)?\s+for\s+)?'
                r'(\w+)',
                stripped
            )
            if impl_match and "{" in stripped:
                impl_type = impl_match.group(1)
                impl_stack.append((impl_type, brace_depth))

            # Count braces
            # Simple approach: count { and } on this line
            for ch in stripped:
                if ch == '{':
                    brace_depth += 1
                elif ch == '}':
                    brace_depth -= 1
                    # Check if we exited an impl block
                    if impl_stack and brace_depth <= impl_stack[-1][1]:
                        impl_stack.pop()

            # Match function declarations
            fn_match = re.match(
                r'(?:pub(?:\(crate\))?\s+)?'
                r'(unsafe\s+)?'
                r'(?:const\s+)?'
                r'fn\s+(\w+)'
                r'\s*(?:<[^>]*>)?'
                r'\s*\(([^)]*)\)',
                stripped
            )

            if not fn_match:
                # Try multiline: fn name(\n  params\n)
                fn_start = re.match(
                    r'(?:pub(?:\(crate\))?\s+)?'
                    r'(unsafe\s+)?'
                    r'(?:const\s+)?'
                    r'fn\s+(\w+)'
                    r'\s*(?:<[^>]*>)?'
                    r'\s*\(',
                    stripped
                )
                if fn_start and ")" not in stripped:
                    fn_name = fn_start.group(2)
                    is_unsafe = fn_start.group(1) is not None
                    # Collect params from subsequent lines
                    param_text = stripped.split("(", 1)[1] if "(" in stripped else ""
                    for k in range(i + 1, min(i + 20, len(lines))):
                        param_text += " " + lines[k].strip()
                        if ")" in lines[k]:
                            break
                    param_text = param_text.split(")")[0].strip()

                    # Extract return type
                    full_sig = stripped
                    for k in range(i + 1, min(i + 20, len(lines))):
                        full_sig += " " + lines[k].strip()
                        if "{" in lines[k] or ";" in lines[k]:
                            break
                    ret_match = re.search(r'\)\s*->\s*([^{;]+)', full_sig)
                    ret_type = ret_match.group(1).strip() if ret_match else None

                    params = parse_params(param_text)
                    impl_type = impl_stack[-1][0] if impl_stack else None

                    # Skip test/proof functions
                    if fn_name.startswith("test_") or fn_name.startswith("proof_") or fn_name.startswith("kani_"):
                        i += 1
                        continue

                    has_self = any(pn == "self" for pn, _ in params)
                    self_kind = None
                    if has_self:
                        self_kind = next(pt for pn, pt in params if pn == "self")

                    methods.append({
                        "name": fn_name,
                        "impl_type": impl_type,
                        "module": module,
                        "file": fname,
                        "line": i + 1,
                        "params": params,
                        "is_unsafe": is_unsafe,
                        "has_self": has_self,
                        "self_kind": self_kind,
                        "return_type": ret_type,
                    })
                    i += 1
                    continue

            if fn_match:
                is_unsafe = fn_match.group(1) is not None
                fn_name = fn_match.group(2)
                params_raw = fn_match.group(3).strip()

                # Extract return type
                ret_match = re.search(r'\)\s*->\s*([^{;]+)', stripped)
                ret_type = ret_match.group(1).strip() if ret_match else None

                params = parse_params(params_raw)
                impl_type = impl_stack[-1][0] if impl_stack else None

                # Skip test/proof/kani functions
                if fn_name.startswith("test_") or fn_name.startswith("proof_") or fn_name.startswith("kani_"):
                    i += 1
                    continue

                has_self = any(pn == "self" for pn, _ in params)
                self_kind = None
                if has_self:
                    self_kind = next(pt for pn, pt in params if pn == "self")

                methods.append({
                    "name": fn_name,
                    "impl_type": impl_type,
                    "module": module,
                    "file": fname,
                    "line": i + 1,
                    "params": params,
                    "is_unsafe": is_unsafe,
                    "has_self": has_self,
                    "self_kind": self_kind,
                    "return_type": ret_type,
                })

            i += 1

    return methods


def parse_params(params_raw):
    """Parse parameter list into [(name, type)]."""
    if not params_raw or params_raw.isspace():
        return []

    parts = split_type_list(params_raw)
    result = []
    for p in parts:
        p = p.strip()
        if not p:
            continue
        if p in ("self", "&self", "&mut self", "mut self"):
            result.append(("self", p))
            continue
        m = re.match(r'(?:mut\s+)?(\w+)\s*:\s*(.+)', p)
        if m:
            result.append((m.group(1).strip(), m.group(2).strip()))
        else:
            result.append(("_", p))
    return result


def classify_method(method, structs, memo):
    """Classify a method as provable or not.

    Returns (provable: bool, reason: str, construction_template: dict).
    """
    params = method["params"]
    impl_type = method["impl_type"]

    constructions = {}

    for pname, ptype in params:
        if pname == "self":
            # Check if the impl type is constructible
            if impl_type and impl_type in structs:
                ok, expr = is_type_constructible(impl_type, structs, memo)
                if not ok:
                    return False, f"self type '{impl_type}' not constructible", None
                constructions["self"] = {"type": ptype, "construction": expr}
            elif impl_type:
                return False, f"self type '{impl_type}' unknown", None
            else:
                return False, "self without impl_type", None
            continue

        # Strip reference layers for constructibility check
        clean = ptype
        is_ref = False
        is_mut_ref = False
        if clean.startswith("&mut "):
            clean = clean[5:].strip()
            is_mut_ref = True
        elif clean.startswith("&"):
            clean = clean[1:].strip()
            is_ref = True

        ok, expr = is_type_constructible(clean, structs, memo)
        if not ok:
            return False, f"param '{pname}' type '{ptype}' not constructible", None
        constructions[pname] = {
            "type": ptype,
            "clean_type": clean,
            "construction": expr,
            "is_ref": is_ref,
            "is_mut_ref": is_mut_ref,
        }

    return True, "all params constructible", constructions


def main():
    print("=== Kani Constructibility Analysis ===", file=sys.stderr)
    print(f"Source: {RUST_SRC}", file=sys.stderr)

    # Step 1: Parse all structs
    print("\n[1/4] Parsing struct definitions...", file=sys.stderr)
    structs = parse_structs(RUST_SRC)
    print(f"  Found {len(structs)} structs", file=sys.stderr)

    # Step 2: Determine constructibility
    print("\n[2/4] Computing type constructibility...", file=sys.stderr)
    memo = {}
    constructible = {}
    non_constructible = {}

    for name, info in sorted(structs.items()):
        ok, expr = is_type_constructible(name, structs, memo)
        if ok:
            constructible[name] = {
                "construction": expr,
                "fields": info["fields"],
                "file": info["file"],
                "line": info["line"],
                "kind": info["kind"],
            }
        else:
            # Find which field blocked it
            blockers = []
            for fname, ftype in info["fields"]:
                fok, _ = is_type_constructible(ftype, structs, memo)
                if not fok:
                    blockers.append(f"{fname}: {ftype}")
            non_constructible[name] = {
                "file": info["file"],
                "line": info["line"],
                "blockers": blockers,
            }

    print(f"  Constructible: {len(constructible)}", file=sys.stderr)
    print(f"  Non-constructible: {len(non_constructible)}", file=sys.stderr)

    # Show constructible types
    for name in sorted(constructible):
        c = constructible[name]
        field_types = ", ".join(f"{fn}: {ft}" for fn, ft in c["fields"]) if c["fields"] else "(unit)"
        print(f"    + {name} [{c['file']}:{c['line']}] — {field_types}", file=sys.stderr)

    # Step 3: Parse all methods
    print("\n[3/4] Parsing impl block methods...", file=sys.stderr)
    methods = parse_impl_methods(RUST_SRC)
    print(f"  Found {len(methods)} methods/functions", file=sys.stderr)

    # Step 4: Classify each method
    print("\n[4/4] Classifying methods by constructibility...", file=sys.stderr)
    provable = []
    not_provable = []

    for method in methods:
        ok, reason, constructions = classify_method(method, structs, memo)
        entry = {
            "name": method["name"],
            "impl_type": method["impl_type"],
            "module": method["module"],
            "file": method["file"],
            "line": method["line"],
            "params": [(pn, pt) for pn, pt in method["params"]],
            "is_unsafe": method["is_unsafe"],
            "has_self": method["has_self"],
            "self_kind": method["self_kind"],
            "return_type": method["return_type"],
        }
        if ok:
            entry["constructions"] = constructions
            provable.append(entry)
        else:
            entry["reason"] = reason
            not_provable.append(entry)

    print(f"  Provable: {len(provable)}", file=sys.stderr)
    print(f"  Not provable: {len(not_provable)}", file=sys.stderr)

    # Group provable by impl_type
    by_type = defaultdict(list)
    for p in provable:
        key = p["impl_type"] or "(free function)"
        by_type[key].append(p)

    print("\n" + "=" * 60, file=sys.stderr)
    print("  PROVABLE FUNCTIONS BY TYPE", file=sys.stderr)
    print("=" * 60, file=sys.stderr)
    for type_name in sorted(by_type):
        fns = by_type[type_name]
        print(f"\n  {type_name} ({len(fns)} functions):", file=sys.stderr)
        for fn in sorted(fns, key=lambda f: f["name"]):
            params_str = ", ".join(f"{pn}: {pt}" for pn, pt in fn["params"])
            ret = f" -> {fn['return_type']}" if fn["return_type"] else ""
            print(f"    {fn['name']}({params_str}){ret}  [{fn['file']}:{fn['line']}]", file=sys.stderr)

    # Exclusion summary
    print("\n" + "=" * 60, file=sys.stderr)
    print("  EXCLUSION REASONS (top 20)", file=sys.stderr)
    print("=" * 60, file=sys.stderr)
    reason_counts = defaultdict(int)
    for np in not_provable:
        reason_counts[np["reason"]] += 1
    for reason, count in sorted(reason_counts.items(), key=lambda x: -x[1])[:20]:
        print(f"  [{count:4d}] {reason}", file=sys.stderr)

    # Write output
    output = {
        "constructible_types": {
            name: {
                "construction": info["construction"],
                "fields": info["fields"],
                "file": info["file"],
                "line": info["line"],
            }
            for name, info in sorted(constructible.items())
        },
        "non_constructible_types": {
            name: info
            for name, info in sorted(non_constructible.items())
        },
        "provable_functions": provable,
        "not_provable_functions": not_provable,
        "summary": {
            "total_structs": len(structs),
            "constructible_structs": len(constructible),
            "non_constructible_structs": len(non_constructible),
            "total_methods": len(methods),
            "provable_methods": len(provable),
            "not_provable_methods": len(not_provable),
        },
    }

    with open(OUTPUT, "w") as f:
        json.dump(output, f, indent=2)

    print(f"\nOutput: {OUTPUT}", file=sys.stderr)
    print(f"\n{'=' * 60}", file=sys.stderr)
    print(f"  SUMMARY", file=sys.stderr)
    print(f"{'=' * 60}", file=sys.stderr)
    print(f"  Structs:     {len(structs)} total, {len(constructible)} constructible", file=sys.stderr)
    print(f"  Methods:     {len(methods)} total, {len(provable)} provable", file=sys.stderr)
    print(f"  Hit rate:    {len(provable)/max(len(methods),1)*100:.1f}%", file=sys.stderr)
    print(f"{'=' * 60}", file=sys.stderr)


if __name__ == "__main__":
    main()
