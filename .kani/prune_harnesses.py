#!/usr/bin/env python3
"""Remove harnesses from proofs_generated.rs that fail to compile.

Reads cargo kani --only-codegen error output, identifies failing harness
function names, removes them from the generated file, and logs the result.

Usage: cargo kani --only-codegen 2>&1 | python3 .kani/prune_harnesses.py
  Or:  python3 .kani/prune_harnesses.py < .kani/compile_errors.log
"""

import re
import sys

RS_FILE = "src/emCore/proofs_generated.rs"
REPORT = ".kani/prune_report.json"

def main():
    errors = sys.stdin.read()

    # Extract harness names mentioned in errors
    # Errors reference line numbers in proofs_generated.rs
    error_lines = set()
    for m in re.finditer(r'proofs_generated\.rs:(\d+)', errors):
        error_lines.add(int(m.group(1)))

    # Read the generated file
    with open(RS_FILE) as f:
        lines = f.readlines()

    # Find which harnesses contain error lines
    bad_harnesses = set()
    current_harness = None
    harness_start = None
    for i, line in enumerate(lines, 1):
        if line.strip().startswith("fn kani_"):
            m = re.match(r'\s*fn\s+(\w+)', line.strip())
            if m:
                current_harness = m.group(1)
                harness_start = i
        if line.strip() == "}" and current_harness:
            if any(harness_start <= el <= i for el in error_lines):
                bad_harnesses.add(current_harness)
            current_harness = None

    # Also catch harnesses named in duplicate errors
    for m in re.finditer(r'the name `(\w+)` is defined multiple times', errors):
        bad_harnesses.add(m.group(1))

    # Also catch harnesses where the function path doesn't resolve
    for m in re.finditer(r"cannot find function `(\w+)` in module `crate::emCore::(\w+)`", errors):
        fn_name = m.group(1)
        module = m.group(2)
        # Find harnesses that call this function
        for i, line in enumerate(lines, 1):
            if f"::{module}::" in line and f"::{fn_name}(" in line:
                # Find the enclosing harness
                for j in range(i, 0, -1):
                    if lines[j-1].strip().startswith("fn kani_"):
                        hm = re.match(r'\s*fn\s+(\w+)', lines[j-1].strip())
                        if hm:
                            bad_harnesses.add(hm.group(1))
                        break

    if not bad_harnesses:
        print(f"No bad harnesses found in error output.", file=sys.stderr)
        return

    print(f"Found {len(bad_harnesses)} harnesses to remove.", file=sys.stderr)

    # Remove bad harnesses from the file
    new_lines = []
    skip = False
    removed = 0
    kept = 0

    i = 0
    while i < len(lines):
        line = lines[i]

        # Check if this is the start of a bad harness (#[cfg(kani)] block)
        if line.strip() == "#[cfg(kani)]" and i + 2 < len(lines):
            # Look ahead for the fn name
            proof_line = lines[i + 1] if lines[i + 1].strip().startswith("#[kani::proof]") else None
            fn_line = lines[i + 2] if proof_line else lines[i + 1]
            hm = re.match(r'\s*fn\s+(\w+)', fn_line.strip())
            if hm and hm.group(1) in bad_harnesses:
                # Skip this entire harness block
                while i < len(lines) and not (lines[i].strip() == "}" and not skip):
                    if lines[i].strip() == "}":
                        i += 1
                        break
                    i += 1
                else:
                    i += 1
                # Skip trailing blank line
                if i < len(lines) and lines[i].strip() == "":
                    i += 1
                removed += 1
                continue

        # Check if this fn line itself is a bad harness
        if line.strip().startswith("fn kani_"):
            hm = re.match(r'\s*fn\s+(\w+)', line.strip())
            if hm and hm.group(1) in bad_harnesses:
                # Skip until closing brace
                while i < len(lines):
                    if lines[i].strip() == "}":
                        i += 1
                        break
                    i += 1
                if i < len(lines) and lines[i].strip() == "":
                    i += 1
                removed += 1
                continue

        if line.strip().startswith("#[kani::proof]"):
            # Count kept harnesses
            kept += 1

        new_lines.append(line)
        i += 1

    with open(RS_FILE, "w") as f:
        f.writelines(new_lines)

    print(f"Removed {removed} harnesses, ~{kept} remaining.", file=sys.stderr)

    import json
    with open(REPORT, "w") as f:
        json.dump({
            "removed": removed,
            "remaining": kept,
            "bad_harnesses": sorted(bad_harnesses),
        }, f, indent=2)


if __name__ == "__main__":
    main()
