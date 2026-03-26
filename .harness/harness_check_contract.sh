#!/usr/bin/env bash
# harness_check_contract.sh — Verify bidirectional completeness of contract.json.
#
# Pattern: feature-list-as-immutable-contract
# Requirements: V1 (input identity), V2 (exhaustive comparison), V8 (no suppression)
#
# Fails if:
#   - Any .golden file on disk has no entry in contract.json
#   - Any contract entry has no corresponding .golden file
#   - contract.json is missing or invalid
#
# Usage: .harness/harness_check_contract.sh [--self-test]

set -euo pipefail

CONTRACT=".harness/contract.json"
GOLDEN_DATA="tests/golden/data"
ERRORS=0

fail() { echo "FAIL: $1" >&2; ERRORS=$((ERRORS + 1)); }

if [ "${1:-}" = "--self-test" ]; then
  echo "Running contract checker self-tests..."
  TD=$(mktemp -d)
  trap "rm -rf $TD" EXIT

  # Setup mock structure
  mkdir -p "$TD/data/painter" "$TD/harness"
  touch "$TD/data/painter/foo.painter.golden"
  touch "$TD/data/painter/bar.painter.golden"
  cat > "$TD/harness/contract.json" << 'EOF'
{"schema_version":1,"categories":{"painter":{"golden_ext":".painter.golden"}},"cases":[{"name":"foo","category":"painter","tolerance":{}}]}
EOF

  # Test: missing contract entry for bar.golden should fail
  out=$(CONTRACT="$TD/harness/contract.json" GOLDEN_DATA="$TD/data" bash "$0" 2>&1) && {
    echo "SELF-TEST FAIL: should have rejected missing entry for bar" >&2; exit 1
  }
  echo "$out" | grep -q "bar" || { echo "SELF-TEST FAIL: error should mention 'bar'" >&2; exit 1; }

  # Test: phantom entry should fail
  cat > "$TD/harness/contract.json" << 'EOF'
{"schema_version":1,"categories":{"painter":{"golden_ext":".painter.golden"}},"cases":[{"name":"foo","category":"painter","tolerance":{}},{"name":"bar","category":"painter","tolerance":{}},{"name":"phantom","category":"painter","tolerance":{}}]}
EOF
  out=$(CONTRACT="$TD/harness/contract.json" GOLDEN_DATA="$TD/data" bash "$0" 2>&1) && {
    echo "SELF-TEST FAIL: should have rejected phantom entry" >&2; exit 1
  }
  echo "$out" | grep -q "phantom" || { echo "SELF-TEST FAIL: error should mention 'phantom'" >&2; exit 1; }

  # Test: correct contract should pass
  cat > "$TD/harness/contract.json" << 'EOF'
{"schema_version":1,"categories":{"painter":{"golden_ext":".painter.golden"}},"cases":[{"name":"foo","category":"painter","tolerance":{}},{"name":"bar","category":"painter","tolerance":{}}]}
EOF
  CONTRACT="$TD/harness/contract.json" GOLDEN_DATA="$TD/data" bash "$0" 2>&1 || {
    echo "SELF-TEST FAIL: correct contract should pass" >&2; exit 1
  }

  echo "All contract checker self-tests passed."
  exit 0
fi

# ── Main logic ────────────────────────────────────────────────────────────────

[ -f "$CONTRACT" ] || { fail "contract.json not found at $CONTRACT"; exit 1; }
jq . "$CONTRACT" > /dev/null 2>&1 || { fail "contract.json is not valid JSON"; exit 1; }

# Build set of contract entries: "category/name"
contract_set=$(jq -r '.cases[] | "\(.category)/\(.name)"' "$CONTRACT" | sort)

# Build set of golden files on disk: "category/name"
disk_set=""
for dir in "$GOLDEN_DATA"/*/; do
  [ -d "$dir" ] || continue
  category=$(basename "$dir")
  ext=$(jq -r ".categories.\"$category\".golden_ext // \".$category.golden\"" "$CONTRACT")
  for f in "$dir"*.golden; do
    [ -f "$f" ] || continue
    name=$(basename "$f")
    # Strip the extension
    name="${name%$ext}"
    [ -z "$name" ] && name=$(basename "$f" | sed "s/\\.$category\\.golden\$//")
    disk_set="${disk_set}${category}/${name}
"
  done
done
disk_set=$(echo "$disk_set" | sort | sed '/^$/d')

# Check: every disk file has a contract entry
while IFS= read -r entry; do
  [ -z "$entry" ] && continue
  if ! echo "$contract_set" | grep -qxF "$entry"; then
    fail "Golden file $entry has no contract entry (untracked oracle)"
  fi
done <<< "$disk_set"

# Check: every contract entry has a disk file
while IFS= read -r entry; do
  [ -z "$entry" ] && continue
  if ! echo "$disk_set" | grep -qxF "$entry"; then
    fail "Contract entry $entry has no golden file (phantom entry)"
  fi
done <<< "$contract_set"

if [ $ERRORS -gt 0 ]; then
  echo "Contract check failed with $ERRORS error(s)." >&2
  exit 1
fi

count=$(echo "$contract_set" | wc -l | tr -d ' ')
echo "Contract check passed: $count cases, bidirectional match."
