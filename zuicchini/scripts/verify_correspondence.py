#!/usr/bin/env python3
"""Verify file and method correspondence between C++ emCore headers and Rust.

Reads:
  - scripts/file_mapping.json          (Phase 0 output)
  - stale/state/run_002/feature_list.json (1533 pre-mapped symbols)
  - ~/git/eaglemode-0.96.4/include/emCore/*.h (C++ headers)
  - src/**/*.rs                         (Rust source)

Outputs per-item verdicts:
  FILE_MATCH / FILE_MISSING
  METHOD_MATCH / METHOD_MISSING / METHOD_DIVERGED

Final summary counts.
"""

import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
EAGLEMODE = Path.home() / "git" / "eaglemode-0.96.4"
HEADERS_DIR = EAGLEMODE / "include" / "emCore"
FEATURE_LIST = ZUICCHINI / "stale" / "state" / "run_002" / "feature_list.json"
FILE_MAPPING = ZUICCHINI / "scripts" / "file_mapping.json"
SRC_DIR = ZUICCHINI / "src"


# ── Snake-case conversion ──────────────────────────────────────────────

def to_snake_case(name: str) -> str:
    """Convert CamelCase C++ method name to snake_case.

    GetX1      -> get_x1
    SetToMinMax -> set_to_min_max
    IsOpaque   -> is_opaque
    PaintRect  -> paint_rect
    GetHSVA    -> get_hsva
    IsTotallyTransparent -> is_totally_transparent
    GetRed     -> get_red
    SetHSVA    -> set_hsva
    ToString   -> to_string
    BLACK      -> BLACK (all-caps constants kept as-is for matching)
    """
    # All-uppercase constants (BLACK, WHITE, etc.) — keep as-is
    if name.isupper() or (name.isupper() and "_" in name):
        return name.lower()

    # Insert underscore before transitions:
    #  - lowercase/digit -> uppercase: getX -> get_X
    #  - uppercase -> uppercase+lowercase: getHSVA -> get_HSVA (but HSVa -> HS_Va)
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name)
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", s)
    return s.lower()


# ── C++ header method extraction ───────────────────────────────────────

def extract_methods_from_header(header_path: Path) -> list[dict]:
    """Extract public/protected method names from a C++ header.

    Returns list of {"name": str, "visibility": "public"|"protected", "is_static": bool, "is_virtual": bool}.
    Skips constructors, destructors, operators, friends, typedefs.
    """
    with open(header_path) as f:
        lines = f.readlines()

    methods = []
    visibility = "private"  # C++ class default
    in_class = False
    class_name = None
    brace_depth = 0
    skip_until_semicolon = False

    # Track nested class depth to handle inner classes
    class_stack = []

    for line in lines:
        stripped = line.strip()

        # Track braces for class body / inline method bodies
        for ch in stripped:
            if ch == "{":
                brace_depth += 1
            elif ch == "}":
                brace_depth -= 1
                if class_stack and brace_depth <= class_stack[-1][1]:
                    class_stack.pop()
                    if class_stack:
                        class_name, _, visibility = class_stack[-1][0], class_stack[-1][1], "private"
                    else:
                        in_class = False
                        class_name = None
                        visibility = "private"

        # Detect class declaration
        cls_match = re.match(r"\s*class\s+(em\w+)\b", stripped)
        if cls_match and "{" in stripped:
            new_class = cls_match.group(1)
            class_stack.append((new_class, brace_depth - 1, visibility))
            class_name = new_class
            in_class = True
            visibility = "private"
            continue
        elif cls_match and ";" not in stripped:
            # Forward-looking: class on next line might have brace
            new_class = cls_match.group(1)
            class_stack.append((new_class, brace_depth, visibility))
            class_name = new_class
            in_class = True
            visibility = "private"
            continue

        if not in_class:
            # Free functions (em-prefixed) — skip operators and class-qualified methods
            if "operator" in stripped or "::" in stripped:
                continue
            m = re.match(
                r"(?:inline\s+)?(?:static\s+)?(?:const\s+)?"
                r"(?:[\w*&<> ]+?\s+)?"
                r"(em[A-Z]\w+)\s*\(",
                stripped,
            )
            if m and not stripped.startswith("//") and not stripped.startswith("#"):
                name = m.group(1)
                if not name.startswith("emException"):
                    methods.append({
                        "name": name,
                        "visibility": "public",
                        "is_static": False,
                        "is_virtual": False,
                        "source": "free_function",
                    })
            continue

        # Skip comments, preprocessor
        if stripped.startswith("//") or stripped.startswith("#") or stripped.startswith("/*"):
            continue

        # Track visibility
        if stripped.startswith("public:") or stripped.startswith("public :"):
            visibility = "public"
            continue
        elif stripped.startswith("protected:") or stripped.startswith("protected :"):
            visibility = "protected"
            continue
        elif stripped.startswith("private:") or stripped.startswith("private :"):
            visibility = "private"
            continue

        # Skip private members
        if visibility == "private":
            continue

        # Skip friend, typedef, using, enum, struct declarations
        if any(stripped.startswith(kw) for kw in ("friend ", "typedef ", "using ", "enum ", "struct ")):
            continue

        # Skip destructor
        if "~" in stripped and "(" in stripped:
            continue

        # Skip operator overloads
        if "operator" in stripped and ("(" in stripped or "[" in stripped):
            continue

        # Skip EM_DEPRECATED wrapper — extract inner if present
        dep_match = re.match(r"EM_DEPRECATED\(\s*(.+)", stripped)
        if dep_match:
            stripped = dep_match.group(1)

        # Try to match method declaration
        # Pattern: [virtual] [static] [inline] [const] ReturnType MethodName(
        m = re.match(
            r"(?:virtual\s+)?(?:static\s+)?(?:inline\s+)?(?:explicit\s+)?"
            r"(?:const\s+)?"
            r"(?:[\w:*&<>,\s]+?\s+)"  # return type (greedy but requires space before name)
            r"([A-Z_]\w*)\s*\(",       # method name starting with uppercase or _
            stripped,
        )
        if m:
            name = m.group(1)
            # Skip if name matches class name (constructor)
            if name == class_name or (class_name and name == class_name.replace("em", "", 1)):
                continue
            # Skip common non-method patterns
            if name in ("EM_DEPRECATED", "EM_FUNC_ATTR_PRINTF"):
                continue

            is_static = "static " in stripped[:stripped.index(name)]
            is_virtual = "virtual " in stripped[:stripped.index(name)]

            methods.append({
                "name": name,
                "visibility": visibility,
                "is_static": is_static,
                "is_virtual": is_virtual,
                "source": "header_parse",
                "class": class_name,
            })
            continue

        # Match constant/field declarations: static const Type NAME = ...;
        # or just: Type NAME; (for public members)
        const_match = re.match(
            r"(?:static\s+)?(?:const\s+)?\w+\s+([A-Z][A-Z0-9_]+)\s*[;=]",
            stripped,
        )
        if const_match:
            methods.append({
                "name": const_match.group(1),
                "visibility": visibility,
                "is_static": True,
                "is_virtual": False,
                "source": "header_const",
                "class": class_name,
            })

    return methods


# ── Feature list extraction ────────────────────────────────────────────

def extract_feature_methods(features: list) -> dict[str, list[dict]]:
    """Extract method info from feature_list.json, keyed by header name.

    Returns {header_name: [{"class": str, "name": str, "rust_target": str}]}.
    """
    result = defaultdict(list)

    for feat in features:
        rust_target = feat.get("rust_target", "")
        for prov in feat.get("cpp_provenance", []):
            sym = prov["cpp_symbol"]

            # Match clean Class::Method or Class::Method(...)
            m = re.match(r"^(em\w+)::([A-Z_]\w*?)(?:\(|$)", sym)
            if not m:
                continue

            cls, method = m.group(1), m.group(2)

            # Skip constructors (method == class name or without em prefix)
            if method == cls or method == cls.replace("em", "", 1):
                continue
            # Skip destructors
            if method.startswith("~"):
                continue

            header = cls + ".h"
            result[header].append({
                "class": cls,
                "name": method,
                "rust_target": rust_target,
                "source": "feature_list",
            })

    # Deduplicate by (class, name) within each header
    deduped = {}
    for header, items in result.items():
        seen = set()
        unique = []
        for item in items:
            key = (item["class"], item["name"])
            if key not in seen:
                seen.add(key)
                unique.append(item)
        deduped[header] = unique

    return deduped


# ── Rust file searching ────────────────────────────────────────────────

def load_rust_file_content(path: Path) -> str:
    """Load a Rust file's content, or empty string if not found."""
    try:
        return path.read_text()
    except FileNotFoundError:
        return ""


def find_method_in_rust(content: str, snake_name: str) -> bool:
    """Check if a snake_cased method name appears in Rust source content.

    Looks for:
    - fn snake_name(
    - snake_name: (field)
    - const SNAKE_NAME (for constants)
    - SNAKE_NAME (all-caps variant)
    """
    if not content:
        return False

    # Direct fn match
    if re.search(rf"\bfn\s+{re.escape(snake_name)}\b", content):
        return True

    # For constants (all-caps in C++), check both snake_case and UPPER_CASE
    upper_name = snake_name.upper()
    if re.search(rf"\b{re.escape(upper_name)}\b", content):
        return True

    # Field/method name in struct/impl
    if re.search(rf"\b{re.escape(snake_name)}\s*:", content):
        return True

    # Method call (e.g., self.snake_name())
    if re.search(rf"\.{re.escape(snake_name)}\s*\(", content):
        return True

    return False


def find_diverged(content: str, cpp_name: str) -> bool:
    """Check if a DIVERGED: comment mentions this C++ method name."""
    if not content:
        return False
    # Match DIVERGED: comments that reference the C++ name
    return bool(re.search(rf"DIVERGED:.*\b{re.escape(cpp_name)}\b", content))


def find_split(content: str) -> bool:
    """Check if file has a SPLIT: comment."""
    return "SPLIT:" in content


# ── Method-level exclusions ─────────────────────────────────────────────
# Computed mechanically: for every method extracted from emStd1.h/emStd2.h,
# check if `fn {snake_name}` exists anywhere in src/. If not, it's not ported
# (covered by Rust std). This replaces a hand-curated list.

def compute_std_equivalent_methods() -> set[str]:
    """Scan emStd1.h/emStd2.h for free functions and class methods, then check
    if any `fn {snake_case}` exists in any Rust source file. Functions with
    no match are confirmed not-ported (std-equivalent)."""
    import re
    from pathlib import Path

    # Load all Rust source content
    rust_content = ""
    for p in SRC_DIR.rglob("*.rs"):
        rust_content += p.read_text() + "\n"

    std_methods = set()
    for h in ("emStd1.h", "emStd2.h"):
        h_path = HEADERS_DIR / h
        if not h_path.exists():
            continue
        content = h_path.read_text()
        # Free functions: ReturnType emFooBar(
        for m in re.finditer(
            r"(?:^|\n)\s*(?:inline\s+)?(?:[\w*&<> ]+\s+)(em[A-Z]\w+)\s*\(", content
        ):
            std_methods.add(m.group(1))
        # Class methods (e.g., emException::SetText)
        for m in re.finditer(
            r"(?:^|\n)\s*(?:virtual\s+)?(?:static\s+)?(?:[\w*&<> ]+\s+)"
            r"([A-Z]\w+)\s*\(",
            content,
        ):
            name = m.group(1)
            if name not in ("EM_DEPRECATED", "EM_FUNC_ATTR_PRINTF"):
                std_methods.add(name)

    # Keep only those with NO fn match in Rust source
    not_ported = set()
    for name in std_methods:
        snake = to_snake_case(name)
        if not re.search(rf"\bfn\s+{re.escape(snake)}\b", rust_content):
            not_ported.add(name)

    return not_ported


# Computed at import time; cached for the session
STD_EQUIVALENT_METHODS: set[str] = set()  # populated in main()


# ── Main verification logic ────────────────────────────────────────────

def resolve_source_path(source_file: str) -> Path:
    """Resolve a source file path from the mapping to an absolute path."""
    # source_files in mapping have "src/" prefix
    if source_file.startswith("src/"):
        return ZUICCHINI / source_file
    return SRC_DIR / source_file


def validate_mapping(mapping_data: dict) -> list[str]:
    """Run mechanical checks on file_mapping.json to catch structural errors.

    Returns a list of failure messages. Empty list = all checks pass.
    """
    failures = []
    mappings = mapping_data["mappings"]
    rust_only = mapping_data.get("rust_only", {})

    # ── Check 1: Rust-only files must not appear as source_files in any header ──
    # Prevents false provenance (e.g., rect.rs wrongly mapped to emATMatrix.h).
    rust_only_files = set(rust_only.keys())
    for header, info in mappings.items():
        for sf in info.get("source_files", []):
            clean = sf.replace("src/", "") if sf.startswith("src/") else sf
            if clean in rust_only_files:
                failures.append(
                    f"PROVENANCE: Rust-only file '{clean}' appears as source "
                    f"for {header} — remove from mapping or from rust_only"
                )

    # ── Check 2: Every source_file must exist on disk ──
    for header, info in mappings.items():
        for sf in info.get("source_files", []):
            path = resolve_source_path(sf)
            if not path.exists():
                failures.append(
                    f"EXISTENCE: {header} lists source '{sf}' but file does not exist"
                )

    # ── Check 3: No unexpected double-mapping (same file as source for headers
    #    with different target_rs). "extract" patterns are expected to share
    #    a source file (code will be split out during Phase 2). ──
    file_to_headers = defaultdict(list)
    extractions_needed = []
    for header, info in mappings.items():
        target = info.get("target_rs")
        if not target:
            continue
        for sf in info.get("source_files", []):
            clean = sf.replace("src/", "") if sf.startswith("src/") else sf
            file_to_headers[clean].append((header, info.get("pattern", ""), target))
    for f, entries in file_to_headers.items():
        if len(entries) > 1:
            patterns = {e[1] for e in entries}
            headers = [e[0] for e in entries]
            targets = [e[2] for e in entries]
            if "extract" in patterns:
                extractions_needed.append({
                    "source_file": f,
                    "headers": sorted(headers),
                    "targets": sorted(targets),
                })
                continue
            failures.append(
                f"DOUBLE-TARGET: '{f}' maps to multiple headers: {sorted(headers)}"
            )

    # ── Check 4: Header count matches actual .h files ──
    actual_headers = sorted(p.name for p in HEADERS_DIR.glob("*.h"))
    mapping_headers = sorted(mappings.keys())
    if actual_headers != mapping_headers:
        missing = set(actual_headers) - set(mapping_headers)
        extra = set(mapping_headers) - set(actual_headers)
        if missing:
            failures.append(f"HEADERS: {len(missing)} headers not in mapping: {sorted(missing)}")
        if extra:
            failures.append(f"HEADERS: {len(extra)} mapping entries not on disk: {sorted(extra)}")

    # ── Check 5: Every non-mod.rs non-lib.rs Rust file is accounted for ──
    all_rs = set()
    for p in SRC_DIR.rglob("*.rs"):
        rel = str(p.relative_to(SRC_DIR))
        if p.name not in ("mod.rs", "lib.rs"):
            all_rs.add(rel)

    accounted = set(rust_only.keys())
    for header, info in mappings.items():
        for sf in info.get("source_files", []):
            clean = sf.replace("src/", "") if sf.startswith("src/") else sf
            accounted.add(clean)

    unaccounted = all_rs - accounted
    if unaccounted:
        failures.append(
            f"COVERAGE: {len(unaccounted)} Rust files not in any mapping or rust_only: "
            f"{sorted(unaccounted)}"
        )

    # ── Check 6: Type-name factual correspondence ──
    # Extract class names from C++ headers, struct/enum/trait names from Rust
    # files. Where emFoo.h defines class emFoo and Foo exists in bar.rs,
    # verify bar.rs is in the mapping for emFoo.h.
    # Note: C++ headers include other headers, so a class name found in a
    # header may be from an #include, not defined there. Only flag cases where
    # the factual file is NOT in our mapping AND the class name matches the
    # header name (emFoo.h should define class emFoo).
    type_to_file = {}
    for p in SRC_DIR.rglob("*.rs"):
        if p.name in ("mod.rs", "lib.rs"):
            continue
        rel = str(p.relative_to(SRC_DIR))
        content = p.read_text()
        for t in set(re.findall(r"(?:pub(?:\(crate\))?\s+)?(?:struct|enum|trait)\s+(\w+)", content)):
            type_to_file.setdefault(t, []).append(rel)

    for header, info in mappings.items():
        if info["pattern"] == "no-rust-equivalent":
            continue
        # Only check the primary class (header name without .h, strip em prefix)
        base = header.replace(".h", "")
        rust_name = base[2:] if base.startswith("em") else base
        factual_files = set(type_to_file.get(rust_name, []))
        if not factual_files:
            continue

        our_files = set()
        for sf in info.get("source_files", []):
            our_files.add(sf.replace("src/", "") if sf.startswith("src/") else sf)

        # Flag if the primary type exists in a file we don't map to this header
        missing_from_mapping = factual_files - our_files
        # Exclude files that are mapped to OTHER headers (they're includes, not errors)
        all_mapped = set()
        for h2, i2 in mappings.items():
            if h2 != header:
                for sf in i2.get("source_files", []):
                    all_mapped.add(sf.replace("src/", "") if sf.startswith("src/") else sf)
        truly_missing = missing_from_mapping - all_mapped
        if truly_missing:
            failures.append(
                f"TYPE-NAME: {header} defines {base} but type '{rust_name}' found in "
                f"{sorted(truly_missing)} which is not in this header's mapping"
            )

    return failures, extractions_needed


def validate_split_justification(mapping_data: dict) -> list[dict]:
    """For each split, check if each file defines a pub struct/enum/trait.

    Files with a primary type are justified by the Modules rule ("one primary
    type per file"). Files without a primary type need a different justification.

    Returns list of {"header", "file", "has_primary_type", "types"} dicts.
    """
    results = []
    mappings = mapping_data["mappings"]

    for header, info in sorted(mappings.items()):
        if info["pattern"] != "split":
            continue
        source_files = info.get("source_files", [])
        if len(source_files) < 2:
            continue

        for sf in source_files:
            path = resolve_source_path(sf)
            content = load_rust_file_content(path)
            if not content:
                continue

            # Find pub struct/enum/trait definitions
            types = re.findall(
                r"pub(?:\(crate\))?\s+(?:struct|enum|trait)\s+(\w+)", content
            )

            results.append({
                "header": header,
                "file": sf,
                "has_primary_type": len(types) > 0,
                "types": types,
            })

    return results


def validate_provenance(mapping_data: dict, merged_methods: dict) -> list[str]:
    """Detect suspicious provenance: split files with 0% method match.

    If a file is listed as a split of header X but no methods from header X
    match in that file, the mapping is likely wrong.
    """
    failures = []
    mappings = mapping_data["mappings"]

    for header, info in mappings.items():
        if info["pattern"] != "split":
            continue

        source_files = info.get("source_files", [])
        methods = merged_methods.get(header, [])

        if not methods or not source_files:
            continue

        # Check each source file individually
        for sf in source_files:
            path = resolve_source_path(sf)
            content = load_rust_file_content(path)
            if not content:
                continue

            # Count how many methods from this header match in this specific file
            match_count = 0
            total_applicable = 0
            for m in methods:
                cpp_name = m["name"]
                if cpp_name in STD_EQUIVALENT_METHODS:
                    continue
                total_applicable += 1
                snake = to_snake_case(cpp_name)
                if find_method_in_rust(content, snake):
                    match_count += 1

            if total_applicable > 0 and match_count == 0:
                failures.append(
                    f"SUSPICIOUS: {header} split file '{sf}' has 0/{total_applicable} "
                    f"method matches — possible false provenance"
                )

    return failures


def detect_getter_setter_fields(mapping_data: dict, merged_methods: dict) -> list[dict]:
    """Detect Get*/Set* pairs where the Rust code uses a pub field instead.

    Returns list of {"header", "getter", "setter", "field", "rust_file"} dicts.
    """
    detections = []
    mappings = mapping_data["mappings"]

    for header, info in mappings.items():
        if info["pattern"] == "no-rust-equivalent":
            continue

        methods = merged_methods.get(header, [])
        if not methods:
            continue

        # Find Get*/Set* pairs
        getters = {}
        setters = {}
        for m in methods:
            name = m["name"]
            if name.startswith("Get") and len(name) > 3:
                field = name[3:]  # GetSpaceL -> SpaceL
                getters[field] = name
            elif name.startswith("Set") and len(name) > 3:
                field = name[3:]
                setters[field] = name

        # Only look at fields that have BOTH getter and setter
        pairs = set(getters.keys()) & set(setters.keys())
        if not pairs:
            continue

        # Load Rust content
        source_files = info.get("source_files", [])
        contents = {}
        for sf in source_files:
            path = resolve_source_path(sf)
            contents[sf] = load_rust_file_content(path)
        all_content = "\n".join(contents.values())

        for field in sorted(pairs):
            snake_field = to_snake_case(field)
            getter_snake = to_snake_case(getters[field])
            setter_snake = to_snake_case(setters[field])

            # Check if getter/setter are already matched
            getter_found = find_method_in_rust(all_content, getter_snake)
            setter_found = find_method_in_rust(all_content, setter_snake)

            if getter_found or setter_found:
                continue  # Already has method-level match, not a field replacement

            # Check if the field name appears as a pub field
            if re.search(rf"pub\s+{re.escape(snake_field)}\s*:", all_content):
                detections.append({
                    "header": header,
                    "getter": getters[field],
                    "setter": setters[field],
                    "field": snake_field,
                })

    return detections


def compute_type_renames(mapping_data: dict) -> list[dict]:
    """Compute type renames needed: Foo → emFoo for every C++ class.

    Extracts class names from C++ headers, finds the corresponding Rust type
    (with em prefix stripped), and produces rename items.

    Returns list of {"header", "cpp_type", "current_rust_type", "target_rust_type",
                      "files_containing_type", "needs_rename"} dicts.
    """
    mappings = mapping_data["mappings"]
    results = []

    # Build type → file index from Rust source
    type_to_files = defaultdict(list)
    for p in SRC_DIR.rglob("*.rs"):
        rel = str(p.relative_to(SRC_DIR))
        content = p.read_text()
        for t in set(re.findall(
            r"(?:pub(?:\(crate\))?\s+)?(?:struct|enum|trait)\s+(\w+)", content
        )):
            type_to_files[t].append(rel)

    # Also index all files that REFERENCE each type (for call-site tracking)
    type_references = defaultdict(set)
    all_rust_content = {}
    for p in SRC_DIR.rglob("*.rs"):
        rel = str(p.relative_to(SRC_DIR))
        all_rust_content[rel] = p.read_text()

    # Extract class names from C++ headers
    for header, info in sorted(mappings.items()):
        if info["pattern"] == "no-rust-equivalent":
            continue

        h_path = HEADERS_DIR / header
        if not h_path.exists():
            continue

        content = h_path.read_text()
        classes = set(re.findall(r"^\s*class\s+(em\w+)\b", content, re.MULTILINE))

        for cpp_type in sorted(classes):
            # Expected current Rust name (em prefix stripped)
            stripped = cpp_type[2:] if cpp_type.startswith("em") else cpp_type
            target = cpp_type  # Keep the em prefix

            # Check if stripped name exists in Rust
            defining_files = type_to_files.get(stripped, [])

            if not defining_files:
                # Type might already have em prefix, or might not exist
                em_files = type_to_files.get(cpp_type, [])
                if em_files:
                    results.append({
                        "header": header,
                        "cpp_type": cpp_type,
                        "current_rust_type": cpp_type,
                        "target_rust_type": cpp_type,
                        "defining_files": em_files,
                        "needs_rename": False,
                    })
                # else: type not found in Rust at all (not ported)
                continue

            # Count files that reference this type (for scope estimation)
            ref_files = set()
            for rel, file_content in all_rust_content.items():
                if re.search(rf"\b{re.escape(stripped)}\b", file_content):
                    ref_files.add(rel)

            results.append({
                "header": header,
                "cpp_type": cpp_type,
                "current_rust_type": stripped,
                "target_rust_type": target,
                "defining_files": defining_files,
                "reference_count": len(ref_files),
                "needs_rename": stripped != target,
            })

    return results


def detect_enum_restructuring(mapping_data: dict, merged_methods: dict) -> list[dict]:
    """Detect class hierarchies replaced by Rust enums.

    If a header has many missing methods and the Rust file defines an enum
    whose variants correspond to C++ subclass names, it's a restructuring.
    """
    detections = []
    mappings = mapping_data["mappings"]

    for header, info in mappings.items():
        if info["pattern"] == "no-rust-equivalent":
            continue

        methods = merged_methods.get(header, [])
        if len(methods) < 20:  # Only flag headers with many methods
            continue

        source_files = info.get("source_files", [])
        all_content = ""
        for sf in source_files:
            path = resolve_source_path(sf)
            all_content += load_rust_file_content(path) + "\n"

        if not all_content:
            continue

        # Count missing methods
        missing = 0
        total = 0
        for m in methods:
            if m["name"] in STD_EQUIVALENT_METHODS:
                continue
            total += 1
            snake = to_snake_case(m["name"])
            if not find_method_in_rust(all_content, snake):
                missing += 1

        if total == 0 or missing / total < 0.5:
            continue  # Less than 50% missing, not a bulk restructuring

        # Check if Rust uses enums
        enum_matches = re.findall(r"pub\s+enum\s+(\w+)", all_content)
        if enum_matches:
            # Find classes from the methods
            classes = set()
            for m in methods:
                cls = m.get("class", "")
                if cls:
                    classes.add(cls)

            detections.append({
                "header": header,
                "missing": missing,
                "total": total,
                "rust_enums": enum_matches,
                "cpp_classes": sorted(classes),
            })

    return detections


def main():
    # Load inputs
    with open(FILE_MAPPING) as f:
        mapping_data = json.load(f)
    with open(FEATURE_LIST) as f:
        features = json.load(f)["features"]

    mappings = mapping_data["mappings"]
    rust_only = mapping_data.get("rust_only", {})

    validate_mode = "--validate" in sys.argv

    # Compute std-equivalent methods mechanically
    global STD_EQUIVALENT_METHODS
    STD_EQUIVALENT_METHODS = compute_std_equivalent_methods()

    # ── Mapping validation (always runs) ──────────────────────────
    mapping_failures, extractions_needed = validate_mapping(mapping_data)
    if mapping_failures:
        print("=" * 72)
        print("MAPPING VALIDATION FAILURES")
        print("=" * 72)
        for f in mapping_failures:
            print(f"  FAIL: {f}")
        if validate_mode:
            print(f"\n  {len(mapping_failures)} mapping validation failure(s).")
            return 2

    # Extract methods from both sources
    feature_methods = extract_feature_methods(features)
    header_methods = {}
    for h_path in sorted(HEADERS_DIR.glob("*.h")):
        header_methods[h_path.name] = extract_methods_from_header(h_path)

    # Merge: feature_list methods take priority, header methods supplement
    merged_methods = {}
    for header in sorted(set(list(feature_methods.keys()) + list(header_methods.keys()))):
        seen = set()
        combined = []

        # Feature list first (higher confidence)
        for item in feature_methods.get(header, []):
            key = (item.get("class", ""), item["name"])
            if key not in seen:
                seen.add(key)
                combined.append(item)

        # Header parsing supplements
        for item in header_methods.get(header, []):
            key = (item.get("class", ""), item["name"])
            if key not in seen:
                seen.add(key)
                combined.append(item)

        if combined:
            merged_methods[header] = combined

    # ── Verification ───────────────────────────────────────────────

    file_match = 0
    file_missing = 0
    method_match = 0
    method_missing = 0
    method_diverged = 0
    skipped_no_equiv = 0
    skipped_rust_only = 0

    file_details = []
    method_details = []

    for header in sorted(mappings.keys()):
        info = mappings[header]

        # Skip no-rust-equivalent
        if info["pattern"] == "no-rust-equivalent":
            skipped_no_equiv += 1
            continue

        source_files = info.get("source_files", [])

        # ── File-level check ───────────────────────────────────────
        if not source_files:
            # Headers with only mod.rs code (emTiling.h, emGroup.h)
            # These will have files created in Phase 2c; for now mark as expected-missing
            mod_code = info.get("mod_rs_code")
            if mod_code:
                file_details.append(("FILE_PENDING", header, f"mod.rs code: {mod_code}"))
            elif info["pattern"] == "merge" and not source_files:
                file_details.append(("FILE_MERGE_TARGET", header, info.get("note", "")))
            continue

        for sf in source_files:
            path = resolve_source_path(sf)
            if path.exists():
                file_match += 1
                file_details.append(("FILE_MATCH", header, sf))
            else:
                file_missing += 1
                file_details.append(("FILE_MISSING", header, sf))

        # ── Method-level check ─────────────────────────────────────
        methods = merged_methods.get(header, [])
        if not methods:
            continue

        # Load all source file contents
        contents = {}
        for sf in source_files:
            path = resolve_source_path(sf)
            contents[sf] = load_rust_file_content(path)

        # Also load mod.rs code location if present
        mod_code = info.get("mod_rs_code")
        if mod_code:
            # Extract the file part (e.g., "layout/mod.rs" from "layout/mod.rs (Orientation, ...)")
            mod_file = mod_code.split("(")[0].strip()
            mod_path = SRC_DIR / mod_file
            contents["mod:" + mod_code] = load_rust_file_content(mod_path)

        # Combine all content for searching
        all_content = "\n".join(contents.values())

        for method_info in methods:
            cpp_name = method_info["name"]
            snake = to_snake_case(cpp_name)

            # Skip methods that map to Rust std library
            if cpp_name in STD_EQUIVALENT_METHODS:
                method_details.append(("METHOD_STD_EQUIV", header, cpp_name, snake))
                continue

            if find_method_in_rust(all_content, snake):
                method_match += 1
                method_details.append(("METHOD_MATCH", header, cpp_name, snake))
            elif find_diverged(all_content, cpp_name):
                method_diverged += 1
                method_details.append(("METHOD_DIVERGED", header, cpp_name, ""))
            else:
                method_missing += 1
                method_details.append(("METHOD_MISSING", header, cpp_name, snake))

    # ── Rust-only files ────────────────────────────────────────────
    for rf in rust_only:
        skipped_rust_only += 1

    # ── Structural detections ──────────────────────────────────────
    provenance_failures = validate_provenance(mapping_data, merged_methods)
    split_justifications = validate_split_justification(mapping_data)
    getter_setter_fields = detect_getter_setter_fields(mapping_data, merged_methods)
    enum_restructurings = detect_enum_restructuring(mapping_data, merged_methods)
    type_renames = compute_type_renames(mapping_data)

    # Count methods explained by detections (for adjusted missing count)
    explained_by_fields = 0
    for det in getter_setter_fields:
        explained_by_fields += 2  # getter + setter pair

    explained_by_enum = 0
    for det in enum_restructurings:
        explained_by_enum += det["missing"]

    # ── Output ─────────────────────────────────────────────────────

    verbose = "--verbose" in sys.argv or "-v" in sys.argv
    header_filter = None
    for arg in sys.argv[1:]:
        if not arg.startswith("-"):
            header_filter = arg
            break

    if verbose or header_filter:
        print("=" * 72)
        print("FILE CORRESPONDENCE")
        print("=" * 72)
        for verdict, header, detail in file_details:
            if header_filter and header_filter not in header:
                continue
            print(f"  {verdict:20s} {header:30s} {detail}")

        print()
        print("=" * 72)
        print("METHOD CORRESPONDENCE")
        print("=" * 72)
        for entry in method_details:
            verdict = entry[0]
            header = entry[1]
            cpp_name = entry[2]
            snake = entry[3] if len(entry) > 3 else ""
            if header_filter and header_filter not in header:
                continue
            if verdict == "METHOD_MATCH" and not verbose:
                continue
            print(f"  {verdict:20s} {header:30s} {cpp_name:30s} {snake}")

    # ── Provenance warnings ───────────────────────────────────────
    if provenance_failures:
        print()
        print("=" * 72)
        print("PROVENANCE WARNINGS")
        print("=" * 72)
        for f in provenance_failures:
            print(f"  {f}")

    # ── Split justification ────────────────────────────────────────
    splits_needing_justification = [s for s in split_justifications if not s["has_primary_type"]]
    if splits_needing_justification:
        print()
        print("=" * 72)
        print("SPLIT JUSTIFICATION")
        print("=" * 72)
        modules_justified = sum(1 for s in split_justifications if s["has_primary_type"])
        print(f"  {modules_justified} split files justified by Modules rule (has primary type)")
        print(f"  {len(splits_needing_justification)} split files need non-Modules justification:")
        for s in splits_needing_justification:
            print(f"    {s['header']:30s} {s['file']} — no pub struct/enum/trait")

    # ── Summary ────────────────────────────────────────────────────
    print()
    print("=" * 72)
    print("SUMMARY")
    print("=" * 72)

    total_files = file_match + file_missing
    method_std_equiv = sum(1 for e in method_details if e[0] == "METHOD_STD_EQUIV")
    total_methods = method_match + method_missing + method_diverged

    print(f"  Files:   {file_match} matched, {file_missing} missing (of {total_files})")
    print(f"  Methods: {method_match} matched, {method_missing} missing, {method_diverged} diverged (of {total_methods})")
    print(f"  Skipped: {skipped_no_equiv} no-rust-equivalent headers, {skipped_rust_only} rust-only files, {method_std_equiv} std-equivalent methods")

    if total_methods > 0:
        pct = method_match / total_methods * 100
        print(f"  Method match rate: {pct:.1f}%")

    # ── Structural analysis ────────────────────────────────────────
    if getter_setter_fields or enum_restructurings:
        print()
        print("  Structural patterns detected (explains some METHOD_MISSING):")

    if getter_setter_fields:
        print(f"    Getter/setter with pub field (needs decision): {len(getter_setter_fields)} pairs ({explained_by_fields} methods)")
        if verbose:
            for det in getter_setter_fields:
                print(f"      {det['header']}: {det['getter']}/{det['setter']} → pub {det['field']}")

    if enum_restructurings:
        print(f"    Class hierarchy → enum: {len(enum_restructurings)} headers ({explained_by_enum} methods)")
        if verbose:
            for det in enum_restructurings:
                print(f"      {det['header']}: {det['missing']}/{det['total']} missing, "
                      f"enums: {det['rust_enums']}, classes: {det['cpp_classes'][:5]}...")

    # Type renames
    renames_needed = [r for r in type_renames if r["needs_rename"]]
    already_correct = [r for r in type_renames if not r["needs_rename"]]
    if type_renames:
        print(f"    Type renames needed: {len(renames_needed)} (already correct: {len(already_correct)})")

    if explained_by_fields + explained_by_enum > 0:
        unexplained = method_missing - explained_by_fields - explained_by_enum
        # Don't go below zero (some detections may overlap with non-missing)
        unexplained = max(0, unexplained)
        print(f"    Unexplained missing: ~{unexplained} (of {method_missing})")

    # ── Per-header breakdown for missing ───────────────────────────
    if method_missing > 0:
        print()
        print("  Missing methods by header:")
        by_header = defaultdict(list)
        for entry in method_details:
            if entry[0] == "METHOD_MISSING":
                by_header[entry[1]].append(entry[2])
        for header in sorted(by_header.keys()):
            methods = by_header[header]
            print(f"    {header}: {len(methods)} missing")
            if verbose or header_filter:
                for m in methods:
                    print(f"      - {m} (expected: {to_snake_case(m)})")

    # ── Validate mode: hard failures vs advisory warnings ──────────
    if validate_mode:
        # mapping_failures are hard failures (structural errors)
        # provenance_failures are advisory (may be expected for implementation splits)
        if mapping_failures:
            print()
            print(f"  VALIDATE: {len(mapping_failures)} hard failure(s) — see MAPPING VALIDATION above")
            return 2
        else:
            advisory = len(provenance_failures)
            print()
            msg = "  VALIDATE: All structural checks passed"
            if advisory:
                msg += f" ({advisory} advisory provenance warning(s))"
            print(msg)
            return 0

    # ── Inventory (JSON output) ─────────────────────────────────
    if "--inventory" in sys.argv or "--phase4-plan" in sys.argv:
        inv_path = ZUICCHINI / "scripts" / "inventory.json"

        # ── Method items ──
        gs_index = set()
        for det in getter_setter_fields:
            gs_index.add((det["header"], det["getter"]))
            gs_index.add((det["header"], det["setter"]))
        enum_headers = {det["header"] for det in enum_restructurings}

        method_items = []
        for entry in method_details:
            verdict = entry[0]
            header = entry[1]
            cpp_name = entry[2]
            snake = entry[3] if len(entry) > 3 else ""

            mid = f"method:{header}:{cpp_name}"

            if verdict == "METHOD_MATCH":
                method_items.append({
                    "id": mid,
                    "kind": "method",
                    "header": header,
                    "cpp_name": cpp_name,
                    "rust_name": snake,
                    "status": "matched",
                    "category": "matched",
                })
            elif verdict == "METHOD_MISSING":
                if (header, cpp_name) in gs_index:
                    category = "getter_setter_needs_decision"
                    field = to_snake_case(cpp_name[3:])
                    note = f"pub field '{field}' exists — add fn {snake}() wrapper to match C++, or DIVERGED with reason"
                elif header in enum_headers:
                    category = "enum_restructuring"
                    note = "class hierarchy replaced by enum variants"
                else:
                    category = "needs_review"
                    note = f"rename to '{snake}' or add DIVERGED with reason"

                method_items.append({
                    "id": mid,
                    "kind": "method",
                    "header": header,
                    "cpp_name": cpp_name,
                    "rust_name": None,
                    "status": "missing",
                    "category": category,
                    "note": note,
                })
            elif verdict == "METHOD_DIVERGED":
                method_items.append({
                    "id": mid,
                    "kind": "method",
                    "header": header,
                    "cpp_name": cpp_name,
                    "rust_name": None,
                    "status": "diverged",
                    "category": "diverged",
                })
            elif verdict == "METHOD_STD_EQUIV":
                method_items.append({
                    "id": mid,
                    "kind": "method",
                    "header": header,
                    "cpp_name": cpp_name,
                    "rust_name": None,
                    "status": "std_equivalent",
                    "category": "std_equivalent",
                })

        # ── Type rename items ──
        type_items = []
        for r in type_renames:
            type_items.append({
                "id": f"type_rename:{r['header']}:{r['cpp_type']}",
                "kind": "type_rename",
                "header": r["header"],
                "cpp_type": r["cpp_type"],
                "current_rust_type": r["current_rust_type"],
                "target_rust_type": r["target_rust_type"],
                "defining_files": r["defining_files"],
                "reference_count": r.get("reference_count", 0),
                "status": "done" if not r["needs_rename"] else "pending",
            })

        # ── Split justification items ──
        split_items = []
        for s in split_justifications:
            split_items.append({
                "id": f"split_justification:{s['header']}:{s['file']}",
                "kind": "split_justification",
                "header": s["header"],
                "file": s["file"],
                "has_primary_type": s["has_primary_type"],
                "types": s["types"],
                "status": "modules_rule" if s["has_primary_type"] else "needs_justification",
            })

        # ── Provenance warning items ──
        provenance_items = []
        for warning in provenance_failures:
            # Extract header and file from warning message
            m = re.match(r"SUSPICIOUS: (\S+) split file '(\S+)'", warning)
            pid = f"provenance:{m.group(1)}:{m.group(2)}" if m else f"provenance:{warning[:50]}"
            provenance_items.append({
                "id": pid,
                "kind": "provenance_warning",
                "message": warning,
                "status": "advisory",
            })

        # ── No-type-name-data items ──
        no_type_data_items = []
        for header, info in sorted(mappings.items()):
            if info["pattern"] == "no-rust-equivalent":
                continue
            base = header.replace(".h", "")
            rust_name = base[2:] if base.startswith("em") else base
            has_type = any(
                t["header"] == header for t in type_renames
            )
            if not has_type and info.get("source_files"):
                no_type_data_items.append({
                    "id": f"no_type_data:{header}:{rust_name}",
                    "kind": "no_type_data",
                    "header": header,
                    "expected_rust_type": rust_name,
                    "source_files": info["source_files"],
                    "status": "unconfirmed",
                })

        # ── Extraction items (shared source files needing Phase 2 split) ──
        extraction_items = []
        for ext in extractions_needed:
            extraction_items.append({
                "id": f"extraction:{ext['source_file']}",
                "kind": "extraction_needed",
                "source_file": ext["source_file"],
                "headers": ext["headers"],
                "targets": ext["targets"],
                "status": "pending",
            })

        # ── Summary ──
        from collections import Counter
        method_cats = Counter(m["category"] for m in method_items if m["status"] == "missing")
        type_pending = sum(1 for t in type_items if t["status"] == "pending")
        type_done = sum(1 for t in type_items if t["status"] == "done")
        splits_needing = sum(1 for s in split_items if s["status"] == "needs_justification")

        inventory = {
            "summary": {
                "methods": {
                    "matched": sum(1 for m in method_items if m["status"] == "matched"),
                    "missing": sum(1 for m in method_items if m["status"] == "missing"),
                    "diverged": sum(1 for m in method_items if m["status"] == "diverged"),
                    "std_equivalent": sum(1 for m in method_items if m["status"] == "std_equivalent"),
                    "missing_by_category": dict(method_cats),
                },
                "type_renames": {
                    "pending": type_pending,
                    "done": type_done,
                },
                "split_justifications": {
                    "modules_rule": sum(1 for s in split_items if s["status"] == "modules_rule"),
                    "needs_justification": splits_needing,
                },
                "extractions_needed": len(extraction_items),
                "provenance_warnings": len(provenance_items),
                "no_type_data": len(no_type_data_items),
            },
            "methods": method_items,
            "type_renames": type_items,
            "split_justifications": split_items,
            "extractions_needed": extraction_items,
            "provenance_warnings": provenance_items,
            "no_type_data": no_type_data_items,
        }

        with open(inv_path, "w") as f:
            json.dump(inventory, f, indent=2)

        s = inventory["summary"]
        print(f"\n  Inventory written to {inv_path}")
        print(f"    Methods: {s['methods']['matched']} matched, {s['methods']['missing']} missing, "
              f"{s['methods']['diverged']} diverged, {s['methods']['std_equivalent']} std-equiv")
        print(f"    Missing by category: {s['methods']['missing_by_category']}")
        print(f"    Type renames: {s['type_renames']['pending']} pending, {s['type_renames']['done']} done")
        print(f"    Extractions needed: {s['extractions_needed']} shared source files to split")
        print(f"    Split justifications: {s['split_justifications']['needs_justification']} need justification")
        print(f"    Provenance warnings: {s['provenance_warnings']}")
        print(f"    No type data: {s['no_type_data']}")

    return 0 if file_missing == 0 and method_missing == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
