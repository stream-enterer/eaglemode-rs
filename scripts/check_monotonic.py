#!/usr/bin/env python3
"""Compare two strict baselines and verify monotonic improvement.

Usage: python3 scripts/check_monotonic.py before.jsonl after.jsonl

Exit 0 if no test got worse (max_diff did not increase for any test).
Exit 1 if any test regressed, printing the regressions.
"""
import json, sys

def load(path):
    tests = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or not line.startswith('{'):
                # Extract JSON from mixed output lines
                idx = line.find('{')
                if idx < 0:
                    continue
                line = line[idx:]
            try:
                d = json.loads(line)
                tests[d["test"]] = d
            except (json.JSONDecodeError, KeyError):
                continue
    return tests

before = load(sys.argv[1])
after = load(sys.argv[2])

regressions = []
improvements = []
new_passes = []

for name, b in before.items():
    a = after.get(name)
    if a is None:
        print(f"  MISSING: {name} not in after baseline")
        continue
    b_md = b.get("max_diff", 0)
    a_md = a.get("max_diff", 0)
    b_fail = b.get("fail", b.get("failures", 0))
    a_fail = a.get("fail", a.get("failures", 0))

    if a_md > b_md:
        regressions.append((name, b_md, a_md, b_fail, a_fail))
    elif a_md < b_md or a_fail < b_fail:
        improvements.append((name, b_md, a_md, b_fail, a_fail))
    if not b.get("pass", True) and a.get("pass", True):
        new_passes.append(name)

if improvements:
    print(f"\nIMPROVED ({len(improvements)}):")
    for name, bmd, amd, bf, af in sorted(improvements, key=lambda x: x[1]-x[2], reverse=True):
        print(f"  {name:<45} max_diff {bmd:>3} -> {amd:>3}  fail {bf:>6} -> {af:>6}")

if new_passes:
    print(f"\nNEW PASSES ({len(new_passes)}):")
    for name in sorted(new_passes):
        print(f"  {name}")

if regressions:
    print(f"\nREGRESSIONS ({len(regressions)}):")
    for name, bmd, amd, bf, af in sorted(regressions, key=lambda x: x[2]-x[1], reverse=True):
        print(f"  {name:<45} max_diff {bmd:>3} -> {amd:>3}  fail {bf:>6} -> {af:>6}")
    sys.exit(1)
else:
    print(f"\nNo regressions. {len(improvements)} improved, {len(new_passes)} newly passing.")
    sys.exit(0)
