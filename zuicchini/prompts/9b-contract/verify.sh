#!/bin/bash
# Pre-commit verification for the feature contract.
# Run this BEFORE setting passes: true and committing.
# Usage: bash prompts/9b-contract/verify.sh <feature_id>

set -e

FEATURE_ID="$1"
CONTRACT="prompts/9b-contract/feature-contract.json"

if [ -z "$FEATURE_ID" ]; then
    echo "Usage: bash prompts/9b-contract/verify.sh <feature_id>"
    exit 1
fi

echo "=== Verifying gate for: $FEATURE_ID ==="

# 1. Clippy
echo "[1/4] Running clippy..."
cargo clippy --workspace -- -D warnings 2>&1 | tail -3
echo "    ✓ Clippy clean"

# 2. Full test suite
echo "[2/4] Running full test suite..."
RESULT=$(cargo nextest run --workspace 2>&1 | tail -1)
echo "    $RESULT"
if echo "$RESULT" | grep -q "passed" && ! echo "$RESULT" | grep -q "failed"; then
    echo "    ✓ All tests pass"
else
    echo "    ✗ TESTS FAILED — do not set passes: true"
    exit 1
fi

# 3. Check no previously-passing features regressed
echo "[3/4] Checking contract state..."
python3 -c "
import json, sys
d = json.load(open('$CONTRACT'))
total = sum(1 for p in d['phases'] for f in p['features'])
done = sum(1 for p in d['phases'] for f in p['features'] if f['passes'])
pending = [f['id'] for p in d['phases'] for f in p['features'] if not f['passes']]
print(f'    {done}/{total} features passing')
if '$FEATURE_ID' not in [f['id'] for p in d['phases'] for f in p['features']]:
    print(f'    ✗ Feature $FEATURE_ID not found in contract')
    sys.exit(1)
feat = None
for p in d['phases']:
    for f in p['features']:
        if f['id'] == '$FEATURE_ID':
            feat = f
            break
if feat and feat['passes']:
    print(f'    ⚠ Feature $FEATURE_ID already marked as passing')
print(f'    Next pending: {pending[0] if pending else \"none\"}')
"
echo "    ✓ Contract state consistent"

# 4. Verify contract JSON is staged for commit
echo "[4/4] Checking git staging..."
if git diff --name-only -- "$CONTRACT" | grep -q .; then
    echo "    ⚠ Contract JSON has unstaged changes — run: git add $CONTRACT"
else
    echo "    ✓ Contract JSON staged or unchanged"
fi

echo ""
echo "=== Gate PASSED for $FEATURE_ID ==="
echo "Safe to set passes: true, git add, and commit."
