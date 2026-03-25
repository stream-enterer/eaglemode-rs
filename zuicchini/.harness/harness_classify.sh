#!/usr/bin/env bash
# harness_classify.sh — Thin wrapper around harness_classify.py
# Pattern: cli-first-skill-design (stable CLI interface)
exec python3 "$(dirname "$0")/harness_classify.py" "$@"
