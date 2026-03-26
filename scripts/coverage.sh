#!/usr/bin/env bash
set -euo pipefail

# LLVM source-based code coverage for zuicchini.
#
# Uses rustc's built-in instrumentation (-C instrument-coverage) and the
# llvm-profdata / llvm-cov tools shipped with the system LLVM.
#
# Output goes to <workspace>/target/coverage/<run>/
# where <run> is an auto-incrementing directory (001, 002, ...).
# A "latest" symlink always points to the most recent run.
#
# Usage:
#   ./scripts/coverage.sh              # run all tests, generate report + html
#   ./scripts/coverage.sh --report     # skip test run, regenerate from existing profdata
#   ./scripts/coverage.sh --text-only  # skip html generation

CRATE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$CRATE_ROOT/target"

COVERAGE_ROOT="$TARGET/coverage"

SKIP_RUN=false
TEXT_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --report)    SKIP_RUN=true ;;
        --text-only) TEXT_ONLY=true ;;
        *)           echo "Unknown arg: $arg"; exit 1 ;;
    esac
done

# ── Run directory selection ───────────────────────────────────────

if [ "$SKIP_RUN" = true ]; then
    # --report reuses the latest run
    if [ ! -L "$COVERAGE_ROOT/latest" ]; then
        echo "No previous run found — run without --report first."
        exit 1
    fi
    OUTDIR="$(readlink -f "$COVERAGE_ROOT/latest")"
else
    # Allocate the next run number
    mkdir -p "$COVERAGE_ROOT"
    LAST=$(find "$COVERAGE_ROOT" -maxdepth 1 -type d -name '[0-9][0-9][0-9]' \
        | sort -r | head -1 | xargs -r basename)
    if [ -z "$LAST" ]; then
        NEXT="001"
    else
        NEXT=$(printf '%03d' $((10#$LAST + 1)))
    fi
    OUTDIR="$COVERAGE_ROOT/$NEXT"
    mkdir -p "$OUTDIR"

    # Update latest symlink
    ln -sfn "$NEXT" "$COVERAGE_ROOT/latest"
    echo "Run $NEXT"
fi

PROFRAW_DIR="$OUTDIR/profraw"
PROFDATA="$OUTDIR/coverage.profdata"
REPORT="$OUTDIR/report.txt"
HTMLDIR="$OUTDIR/html"

# ── Step 1: Build and run tests with instrumentation ──────────────

if [ "$SKIP_RUN" = false ]; then
    rm -rf "$PROFRAW_DIR" "$PROFDATA"
    mkdir -p "$PROFRAW_DIR"

    echo "Building tests with coverage instrumentation..."
    cd "$CRATE_ROOT"

    # Build and capture the exact binary paths cargo produces.
    BUILT_BINARIES=()
    while IFS= read -r exe; do
        BUILT_BINARIES+=("$exe")
    done < <(
        RUSTFLAGS="-C instrument-coverage" \
            cargo test --tests --no-run --message-format=json 2>/dev/null \
            | jq -r 'select(.executable != null) | .executable'
    )

    echo "  Built ${#BUILT_BINARIES[@]} test binaries."

    # Save the list so --report can reuse it.
    printf '%s\n' "${BUILT_BINARIES[@]}" > "$OUTDIR/binaries.txt"

    # Record timestamp and git state for provenance.
    {
        echo "date: $(date -Iseconds)"
        echo "commit: $(git -C "$CRATE_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
        echo "dirty: $(git -C "$CRATE_ROOT" diff --quiet 2>/dev/null && echo no || echo yes)"
    } > "$OUTDIR/metadata.txt"

    echo "Running instrumented tests..."
    for bin in "${BUILT_BINARIES[@]}"; do
        name="$(basename "$bin")"
        echo "  Running $name..."
        LLVM_PROFILE_FILE="$PROFRAW_DIR/test-%p-%m.profraw" \
            "$bin" --test-threads=1 2>&1 | tail -3 \
            || echo "  Warning: $name had failures"
    done
fi

# ── Step 2: Merge raw profiles ────────────────────────────────────

profraw_count=$(find "$PROFRAW_DIR" -name '*.profraw' 2>/dev/null | wc -l)
if [ "$profraw_count" -eq 0 ]; then
    echo "No .profraw files found in $PROFRAW_DIR — run tests first."
    exit 1
fi

echo "Merging $profraw_count profiles..."
llvm-profdata merge -sparse "$PROFRAW_DIR"/*.profraw -o "$PROFDATA"

# ── Step 3: Collect test binaries ─────────────────────────────────

if [ ! -f "$OUTDIR/binaries.txt" ]; then
    echo "No binaries.txt found — run tests first (without --report)."
    exit 1
fi

BINARIES=()
while IFS= read -r bin; do
    [ -x "$bin" ] && BINARIES+=("$bin")
done < "$OUTDIR/binaries.txt"

if [ ${#BINARIES[@]} -eq 0 ]; then
    echo "No instrumented test binaries found."
    exit 1
fi

# Build the -object flags: first binary is positional, rest are --object=
OBJECT_ARGS=("${BINARIES[0]}")
for ((i = 1; i < ${#BINARIES[@]}; i++)); do
    OBJECT_ARGS+=(--object "${BINARIES[$i]}")
done

echo "Using ${#BINARIES[@]} instrumented binaries."

# ── Step 4: Generate text report ──────────────────────────────────

echo "Generating text report..."
llvm-cov report \
    --instr-profile="$PROFDATA" \
    "${OBJECT_ARGS[@]}" \
    --ignore-filename-regex='/.cargo/registry|/rustc/|tests/' \
    > "$REPORT"

echo ""
cat "$REPORT"
echo ""
echo "Report written to $REPORT"

# ── Step 5: Generate HTML (optional) ──────────────────────────────

if [ "$TEXT_ONLY" = false ]; then
    echo "Generating HTML report..."
    rm -rf "$HTMLDIR"
    llvm-cov show \
        --instr-profile="$PROFDATA" \
        "${OBJECT_ARGS[@]}" \
        --ignore-filename-regex='/.cargo/registry|/rustc/|tests/' \
        --format=html \
        --output-dir="$HTMLDIR"
    echo "HTML report written to $HTMLDIR/index.html"
fi
