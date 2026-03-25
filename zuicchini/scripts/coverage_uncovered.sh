#!/usr/bin/env bash
set -euo pipefail

# List uncovered lines/functions from the latest coverage run.
#
# Uses llvm-cov show to display annotated source with hit counts,
# filtered to uncovered regions only.
#
# Usage:
#   ./scripts/coverage_uncovered.sh                    # all files, uncovered lines only
#   ./scripts/coverage_uncovered.sh emFilePanel.rs     # one file
#   ./scripts/coverage_uncovered.sh --summary          # missed-function count per file

CRATE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORKSPACE_ROOT="$(cd "$CRATE_ROOT/.." && pwd)"
COVERAGE_ROOT="$WORKSPACE_ROOT/target/coverage"

FILTER=""
SUMMARY=false

for arg in "$@"; do
    case "$arg" in
        --summary) SUMMARY=true ;;
        *)         FILTER="$arg" ;;
    esac
done

# Resolve latest run
if [ ! -L "$COVERAGE_ROOT/latest" ]; then
    echo "No coverage runs found. Run scripts/coverage.sh first."
    exit 1
fi

OUTDIR="$(readlink -f "$COVERAGE_ROOT/latest")"
PROFDATA="$OUTDIR/coverage.profdata"

if [ ! -f "$OUTDIR/binaries.txt" ]; then
    echo "No binaries.txt in $OUTDIR"
    exit 1
fi

# Build object args from saved binary list
BINARIES=()
while IFS= read -r bin; do
    [ -x "$bin" ] && BINARIES+=("$bin")
done < "$OUTDIR/binaries.txt"

if [ ${#BINARIES[@]} -eq 0 ]; then
    echo "No instrumented binaries found."
    exit 1
fi

OBJECT_ARGS=("${BINARIES[0]}")
for ((i = 1; i < ${#BINARIES[@]}; i++)); do
    OBJECT_ARGS+=(--object "${BINARIES[$i]}")
done

COMMON_ARGS=(
    --instr-profile="$PROFDATA"
    "${OBJECT_ARGS[@]}"
    --ignore-filename-regex='/.cargo/registry|/rustc/|tests/'
)

if [ "$SUMMARY" = true ]; then
    # Cache summary JSON per run
    SUMMARY_JSON="$OUTDIR/summary.json"
    if [ ! -f "$SUMMARY_JSON" ]; then
        echo "Exporting summary data..." >&2
        llvm-cov export "${COMMON_ARGS[@]}" \
            --summary-only \
            2>/dev/null > "$SUMMARY_JSON"
    fi

    jq -r '
        .data[0].files[]
        | select(.filename | test("zuicchini/src/"))
        | select(.summary.functions.count > .summary.functions.covered)
        | "\(.summary.functions.count - .summary.functions.covered) missed / \(.summary.functions.count) total  \(.filename | split("/")[-1])"
    ' "$SUMMARY_JSON" \
        | sort -rn
else
    # Resolve source files to show
    SRC_DIR="$CRATE_ROOT/src/emCore"
    if [ -n "$FILTER" ]; then
        SOURCES=($(find "$SRC_DIR" -name "$FILTER" -o -name "${FILTER%.rs}.rs" 2>/dev/null))
        if [ ${#SOURCES[@]} -eq 0 ]; then
            echo "No source file matching '$FILTER' in $SRC_DIR"
            exit 1
        fi
    else
        SOURCES=($(find "$SRC_DIR" -name '*.rs' | sort))
    fi

    # Show annotated source, only functions with <100% line coverage
    llvm-cov show "${COMMON_ARGS[@]}" \
        --show-line-counts-or-regions \
        --line-coverage-lt=1 \
        --sources "${SOURCES[@]}" \
        2>/dev/null
fi
