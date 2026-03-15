# Harness Specification: Measuring and Closing a C++→Rust Port

## 1. System Overview

This harness measures the completeness of a C++→Rust port and drives the remaining work to completion. It supports both mechanical transliterations (1:1 symbol correspondence) and idiomatic ports (architectural divergence, consolidation, idiom replacement). It operates in four phases: (1) inventory the C++ codebase via isolated map workers and targeted research to produce a coverage report, (2) detect cross-file architectural patterns and produce a traceability ledger that accounts for every C++ symbol, (3) transform the pattern-resolved data into an immutable capability-level feature list that serves as the contract for what "done" means, and (4) run an autonomous closing loop where a maintainer agent implements, tests, and verifies each capability until every feature passes.

The unit of measurement is the **observable capability** — a behavior the C++ system provides that must also exist in the Rust target. Capabilities map naturally to testable claims and accommodate the full spectrum from 1:1 symbol correspondence to many:1 architectural consolidation.

The harness assumes: a C++ source tree exists, a Rust target tree exists (possibly incomplete), a CLI agent tool is available (e.g., Claude Code), git is initialized in the workspace, and the operator has shell access to run commands.

---

## 2. Filesystem Layout

```
workspace/
├── cpp_source/                     # C++ source tree (read-only input)
├── rust_target/                    # Rust target tree (read-write)
├── state/
│   ├── source_manifest.json        # Enumerated C++ files with checksums
│   ├── system_state.json           # Global system state (phase, counters)
│   ├── map_results/
│   │   ├── worker_0001.json        # Individual map worker outputs
│   │   ├── worker_0002.json
│   │   └── ...
│   ├── ambiguous_items.json        # Items needing research classification
│   ├── research_results.json       # Research agent's classifications
│   ├── coverage_report.json        # Merged map + research results
│   ├── confidence_audit.json       # Sampling audit of confident classifications
│   ├── pattern_resolution/
│   │   ├── chunk_0001.json         # Per-directory pattern resolution output
│   │   ├── chunk_0002.json
│   │   └── ...
│   ├── pattern_map.json            # Merged cross-file pattern groups
│   ├── traceability_ledger.jsonl   # Per-symbol disposition (append-only)
│   ├── feature_list.json           # Immutable contract (capability-level)
│   ├── session_context.json        # Initializer→Maintainer handoff
│   ├── task_queue.json             # Current session's task queue
│   ├── iteration_result.json       # Latest iteration's feedback data
│   ├── progress.txt                # Append-only human-readable log
│   └── backups/
│       ├── feature_list.json.bak   # Last known good feature list
│       └── system_state.json.bak   # Last known good system state
└── logs/
    ├── map_reduce.log              # Map-Reduce phase log
    ├── research.log                # Research phase log
    ├── pattern_resolution.log      # Pattern resolution phase log
    ├── closing.log                 # Closing loop log
    └── errors.log                  # All errors across phases
```

**File size limits:**

| File | Maximum Size |
|------|-------------|
| `source_manifest.json` | 10 MB |
| Any single `worker_NNNN.json` | 1 MB |
| `ambiguous_items.json` | 10 MB |
| `coverage_report.json` | 50 MB |
| `confidence_audit.json` | 5 MB |
| Any single `chunk_NNNN.json` | 5 MB |
| `pattern_map.json` | 10 MB |
| `traceability_ledger.jsonl` | 50 MB |
| `feature_list.json` | 50 MB |
| `task_queue.json` | 5 MB |
| `iteration_result.json` | 1 MB |
| `progress.txt` | 100 MB |
| Any log file | 100 MB |

When a file approaches its size limit (≥90%), the system MUST rotate the file (rename to `*.1`, `*.2`, etc.) before continuing. Maximum 10 rotated files per path.

---

## 3. Schemas

### 3.1 Map Worker Output (`state/map_results/worker_NNNN.json`)

```json
{
  "worker_id": "string (format: 'W-NNNN', zero-padded 4 digits)",
  "source_file": "string (path relative to cpp_source/)",
  "started_at": "string (ISO 8601)",
  "completed_at": "string (ISO 8601)",
  "items": [
    {
      "item_id": "string (format: 'ITEM-NNNNNN', zero-padded 6 digits, globally unique)",
      "cpp_symbol": "string (fully qualified symbol name)",
      "kind": "string (enum: 'function' | 'class' | 'method' | 'macro' | 'constant' | 'type_alias' | 'template' | 'module')",
      "line_start": "integer (≥ 1)",
      "line_end": "integer (≥ line_start)",
      "classification": "string (enum: 'ported' | 'not_ported' | 'ambiguous' | 'superseded' | 'not_applicable')",
      "rust_equivalent": "string | null (path relative to rust_target/, null when classification ∉ {'ported', 'superseded'})",
      "superseding_construct": "string | null (description of the Rust/language construct that replaces this symbol; required when classification = 'superseded' or 'not_applicable', null otherwise)",
      "confidence": "number (0.0–1.0, required when classification = 'ambiguous', optional for 'superseded' and 'not_applicable')",
      "ambiguity_reason": "string | null (enum: 'parse_failure' | 'multi_match' | 'no_match' | 'low_confidence'; required when classification = 'ambiguous', null otherwise)"
    }
  ]
}
```

**Constraints:**
- `items` array: 0–10,000 entries per worker.
- Every `item_id` MUST be globally unique across all workers. The orchestrator assigns the ID range `[worker_number * 10000, (worker_number + 1) * 10000)` to each worker. When workers are reused across batches, subsequent batches start from `(max_previous_id + 1)` rounded up to the next 10,000 boundary.
- `confidence` MUST be present and in [0.0, 1.0] when `classification` = `"ambiguous"`. It MUST be absent or null for `"ported"` and `"not_ported"`. It is optional for `"superseded"` and `"not_applicable"` (workers may signal uncertainty about idiom detection).
- `superseding_construct` MUST be present when `classification` = `"superseded"` or `"not_applicable"`. It SHOULD reference a specific Rust type, trait, function, or language feature (e.g., `"Rc<T> from std"`, `"ownership semantics"`, `"std::env::var"`).
- Workers MUST NOT classify a symbol as `"superseded"` unless they can identify the specific Rust construct in the target tree or standard library that replaces it. Uncertain cases MUST use `"ambiguous"` with `ambiguity_reason: "no_match"`.

### 3.2 Source Manifest (`state/source_manifest.json`)

```json
{
  "generated_at": "string (ISO 8601)",
  "cpp_source_root": "string (absolute path)",
  "portignore_present": "boolean",
  "total_files": "integer (≥ 0)",
  "files": [
    {
      "path": "string (path relative to cpp_source/)",
      "sha256": "string (SHA-256 hex digest of file contents)",
      "size_bytes": "integer (≥ 0)",
      "worker_id": "string | null (W-NNNN, assigned during map stage, null initially)"
    }
  ]
}
```

**Constraints:**
- `total_files` MUST equal `files.length`.
- Every `path` MUST be unique.
- Maximum 50,000 entries (same limit as work units).
- The manifest is written once during file enumeration (Section 4.1, step 1) and updated in-place only to fill in `worker_id` assignments. No file entry is ever added or removed after initial generation.
- The `sha256` field enables staleness detection: if the orchestrator is restarted, it can re-hash source files and compare against the manifest to detect changes since enumeration.

### 3.3 Ambiguous Items (`state/ambiguous_items.json`)

```json
{
  "source_run_id": "string (UUID v4)",
  "generated_at": "string (ISO 8601)",
  "total_mapped": "integer (≥ 0)",
  "total_ambiguous": "integer (≥ 0)",
  "items": [
    {
      "item_id": "string (ITEM-NNNNNN)",
      "source_file": "string (path relative to cpp_source/)",
      "cpp_symbol": "string",
      "kind": "string (same enum as map worker output)",
      "confidence": "number (0.0–1.0)",
      "ambiguity_reason": "string (enum: 'parse_failure' | 'multi_match' | 'no_match' | 'low_confidence')",
      "raw_output": "string (map worker's original classification output, max 2000 chars)"
    }
  ]
}
```

**Constraints:**
- `total_ambiguous` MUST equal `items.length`.
- Maximum 50,000 items. If Map-Reduce produces more than 50,000 ambiguous items, the system MUST halt — the map worker prompts need refinement.

### 3.4 Research Results (`state/research_results.json`)

```json
{
  "source_run_id": "string (must match ambiguous_items.source_run_id)",
  "completed_at": "string (ISO 8601)",
  "resolutions": [
    {
      "item_id": "string (ITEM-NNNNNN, must exist in ambiguous_items.json)",
      "resolved_classification": "string (enum: 'ported' | 'not_ported' | 'not_applicable' | 'superseded')",
      "rust_equivalent": "string | null (path, required when resolved_classification = 'ported' or 'superseded')",
      "superseding_construct": "string | null (required when resolved_classification = 'superseded' or 'not_applicable')",
      "reasoning": "string (1–500 chars explaining the resolution)",
      "sources_consulted": "integer (≥ 1)",
      "search_rounds": "integer (≥ 1)"
    }
  ],
  "unresolved": [
    {
      "item_id": "string",
      "reason": "string (enum: 'exhausted_search_budget' | 'conflicting_evidence' | 'requires_human')"
    }
  ]
}
```

**Constraints:**
- `resolutions.length + unresolved.length` MUST equal `ambiguous_items.total_ambiguous`.
- Every `item_id` in `resolutions` and `unresolved` MUST exist in `ambiguous_items.json`.
- No `item_id` appears in both `resolutions` and `unresolved`.
- Maximum search rounds per item: 10. If the research agent reaches 10 rounds without resolution, the item moves to `unresolved` with reason `"exhausted_search_budget"`.

### 3.5 Coverage Report (`state/coverage_report.json`)

```json
{
  "run_id": "string (UUID v4)",
  "generated_at": "string (ISO 8601)",
  "cpp_source_root": "string (absolute path)",
  "rust_target_root": "string (absolute path)",
  "items": [
    {
      "item_id": "string (ITEM-NNNNNN)",
      "cpp_source": "string (path relative to cpp_source/)",
      "cpp_symbol": "string",
      "kind": "string (same enum as map worker output)",
      "rust_equivalent": "string | null",
      "superseding_construct": "string | null",
      "status": "string (enum: 'ported' | 'not_ported' | 'partial' | 'not_applicable' | 'superseded')",
      "confidence": "number (0.0–1.0)",
      "resolution_source": "string (enum: 'map_worker' | 'research_agent' | 'manual' | 'confidence_audit')",
      "notes": "string (0–500 chars)"
    }
  ],
  "unresolved_items": [
    {
      "item_id": "string",
      "reason": "string"
    }
  ],
  "summary": {
    "total": "integer",
    "ported": "integer",
    "not_ported": "integer",
    "partial": "integer",
    "not_applicable": "integer",
    "superseded": "integer",
    "unresolved": "integer",
    "coverage_pct": "number (0.0–100.0, formula: (ported + not_applicable + superseded) / total * 100)"
  }
}
```

**Constraints:**
- `summary.total` MUST equal `items.length + unresolved_items.length`.
- `summary.ported + summary.not_ported + summary.partial + summary.not_applicable + summary.superseded + summary.unresolved` MUST equal `summary.total`.
- Every item from every map worker output MUST appear exactly once (by `item_id`).
- Maximum 500,000 items. If exceeded, the harness MUST partition into sub-reports by C++ source directory.

### 3.6 Confidence Audit (`state/confidence_audit.json`)

```json
{
  "coverage_run_id": "string (must match coverage_report.run_id)",
  "audited_at": "string (ISO 8601)",
  "sample_size": "integer (≥ 1)",
  "sampling_strategy": "string (enum: 'stratified_by_classification' | 'random' | 'targeted_superseded')",
  "results": [
    {
      "item_id": "string (ITEM-NNNNNN)",
      "original_classification": "string",
      "audited_classification": "string",
      "correct": "boolean",
      "reasoning": "string (1–500 chars)",
      "reclassified": "boolean (true if the coverage report was updated)"
    }
  ],
  "summary": {
    "total_audited": "integer",
    "correct": "integer",
    "incorrect": "integer",
    "accuracy_pct": "number (0.0–100.0)",
    "reclassified_count": "integer",
    "accuracy_by_classification": {
      "ported": { "audited": "integer", "correct": "integer" },
      "not_ported": { "audited": "integer", "correct": "integer" },
      "superseded": { "audited": "integer", "correct": "integer" },
      "not_applicable": { "audited": "integer", "correct": "integer" }
    }
  }
}
```

**Constraints:**
- Every `item_id` MUST exist in `coverage_report.json`.
- `summary.total_audited` MUST equal `results.length`.
- If `accuracy_pct` < 85% for any classification category, the system logs a warning and the pattern resolution phase treats that category's items with reduced trust.

### 3.7 Pattern Map (`state/pattern_map.json`)

```json
{
  "coverage_run_id": "string (must match coverage_report.run_id)",
  "generated_at": "string (ISO 8601)",
  "total_groups": "integer (≥ 0)",
  "total_ungrouped": "integer (≥ 0)",
  "groups": [
    {
      "group_id": "string (format: 'GRP-NNNN')",
      "pattern_type": "string (enum: 'consolidation' | 'idiom_replacement' | 'library_subsumption' | 'language_subsumption' | 'one_to_one')",
      "description": "string (1–300 chars, describes the capability this group represents)",
      "member_items": ["string (ITEM-NNNNNN, from coverage report)"],
      "rust_target": "string | null (primary Rust construct implementing this capability)",
      "superseding_construct": "string | null (for subsumption patterns)",
      "confidence": "number (0.0–1.0)",
      "detection_method": "string (enum: 'deterministic_grouping' | 'directory_chunk_analysis' | 'research_resolution')",
      "source_chunk": "string | null (chunk_NNNN that produced this group, null for deterministic)"
    }
  ]
}
```

**Constraints:**
- `total_groups` MUST equal `groups.length`.
- Every `item_id` in `member_items` MUST exist in `coverage_report.json`.
- No `item_id` appears in more than one group.
- `total_ungrouped` counts coverage report items that appear in no group.
- For `pattern_type` = `"one_to_one"`, `member_items` contains exactly 1 item.
- Groups with `confidence` < 0.5 are flagged for human review and logged to `progress.txt`.

### 3.8 Traceability Ledger (`state/traceability_ledger.jsonl`)

One JSON object per line (JSON Lines format). Every C++ symbol in the coverage report gets exactly one entry.

```json
{
  "item_id": "string (ITEM-NNNNNN)",
  "cpp_symbol": "string",
  "cpp_source": "string (path relative to cpp_source/)",
  "disposition": "string (enum: 'mapped' | 'consolidated' | 'language_subsumed' | 'library_subsumed' | 'superseded' | 'unresolved')",
  "target": "string (CAP-NNNN feature ID when disposition = 'mapped' or 'consolidated'; construct name when subsumed; null when unresolved)",
  "group_id": "string | null (GRP-NNNN from pattern_map.json, null for ungrouped items)",
  "reasoning": "string (1–300 chars)",
  "resolved_by": "string (enum: 'map_worker' | 'research_agent' | 'pattern_resolution' | 'confidence_audit' | 'manual')",
  "resolved_at": "string (ISO 8601)"
}
```

**Constraints:**
- The ledger is **append-only**. Once a line is written, it is never modified or deleted.
- Every `item_id` from the coverage report MUST appear exactly once.
- Every entry with `disposition: "mapped"` or `"consolidated"` MUST have a `target` that references a valid feature ID in `feature_list.json`.
- Completeness check: `wc -l traceability_ledger.jsonl` MUST equal `coverage_report.summary.total`.

### 3.9 Feature List / Immutable Contract (`state/feature_list.json`)

```json
{
  "version": "integer (≥ 1, monotonically increasing)",
  "created_at": "string (ISO 8601)",
  "updated_at": "string (ISO 8601)",
  "source_coverage_run_id": "string (must match coverage_report.run_id)",
  "measurement_unit": "string (literal: 'capability')",
  "features": [
    {
      "id": "string (format: 'CAP-NNNN', zero-padded 4 digits)",
      "category": "string (enum: 'functional_parity' | 'api_compatibility' | 'error_handling' | 'performance' | 'edge_case')",
      "description": "string (1–500 chars, describes the observable capability being verified)",
      "cpp_provenance": [
        {
          "item_id": "string (ITEM-NNNNNN)",
          "cpp_symbol": "string",
          "cpp_source": "string (path relative to cpp_source/)"
        }
      ],
      "rust_target": "string (path relative to rust_target/)",
      "pattern_type": "string (enum: 'one_to_one' | 'consolidation' | 'idiom_replacement' | 'library_subsumption' | 'language_subsumption')",
      "group_id": "string (GRP-NNNN from pattern_map.json)",
      "architectural_note": "string | null (documents known intentional divergences between C++ and Rust implementations; null when implementations are structurally equivalent)",
      "parity_test": {
        "type": "string (enum: 'unit' | 'integration' | 'property' | 'golden_parity' | 'fuzz')",
        "command": "string (shell command, must be non-empty)",
        "expected_exit_code": "integer (default: 0)",
        "timeout_seconds": "integer (default: 60, max: 600)",
        "tolerance": "object | null (for golden_parity: {channel_tolerance: integer, max_failure_pct: number})"
      },
      "passes": "boolean (default: false)",
      "last_verified": "string | null (ISO 8601, null until first verification)",
      "verification_evidence": "string | null (test output, max 1000 chars, null until first verification)"
    }
  ],
  "immutability_hash": "string (SHA-256, computed over features array with passes, last_verified, and verification_evidence set to their defaults/nulls)"
}
```

**Immutability rules:**
- The `immutability_hash` is computed by: (1) deep-copy the `features` array, (2) set every feature's `passes` to `false`, `last_verified` to `null`, `verification_evidence` to `null`, (3) sort each feature's `cpp_provenance` array by `item_id` for canonical ordering, (4) serialize to canonical JSON (sorted keys, no whitespace), (5) compute SHA-256 hex digest.
- Only three fields per feature are mutable: `passes`, `last_verified`, `verification_evidence`.
- All other fields (`id`, `category`, `description`, `cpp_provenance`, `rust_target`, `pattern_type`, `group_id`, `architectural_note`, `parity_test`) are frozen after creation.
- `version` increments by 1 on every write. The `updated_at` timestamp updates on every write.
- Maximum 10,000 features. If pattern resolution yields more than 10,000 capabilities, the operator MUST partition the port into sub-projects, each with its own feature list.

**Backward compatibility:** A feature with a single-element `cpp_provenance` array and `pattern_type: "one_to_one"` is functionally equivalent to the old one-symbol-per-feature model. Mechanical ports produce feature lists that are structurally similar to the old format.

**Backup protocol:**
- Before every write to `feature_list.json`, copy the current file to `state/backups/feature_list.json.bak`.
- Maximum 5 backup copies (rotated as `.bak`, `.bak.1`, `.bak.2`, `.bak.3`, `.bak.4`).

### 3.10 Session Context (`state/session_context.json`)

```json
{
  "project_id": "string (UUID v4)",
  "initialized_at": "string (ISO 8601)",
  "initializer_version": "string (format: 'M.m.p')",
  "cpp_source_root": "string (absolute path)",
  "rust_target_root": "string (absolute path)",
  "feature_list_path": "string (relative path: 'state/feature_list.json')",
  "progress_log_path": "string (relative path: 'state/progress.txt')",
  "bootstrap_command": "string (shell command to run environment setup)",
  "test_command": "string (shell command to run all parity tests)",
  "last_session": {
    "session_id": "string (UUID v4)",
    "completed_at": "string (ISO 8601)",
    "features_completed": ["string (CAP-NNNN IDs)"],
    "features_remaining": "integer (≥ 0)",
    "coverage_at_end": "number (0.0–100.0)"
  } | null
}
```

**Constraints:**
- `last_session` is `null` after the initializer runs and before the first maintainer session completes.
- `feature_list_path` MUST resolve to an existing file when the maintainer reads it.
- `features_completed` MUST contain only IDs that exist in `feature_list.json` and have `passes: true`.

### 3.11 Task Queue (`state/task_queue.json`)

```json
{
  "session_id": "string (UUID v4)",
  "generated_at": "string (ISO 8601)",
  "tasks": [
    {
      "task_id": "string (format: 'TASK-NNNN')",
      "feature_id": "string (CAP-NNNN, must exist in feature_list.json)",
      "priority": "integer (1 = highest)",
      "action": "string (enum: 'implement' | 'fix' | 'verify' | 'research')",
      "description": "string (1–300 chars)",
      "status": "string (enum: 'pending' | 'in_progress' | 'completed' | 'failed' | 'skipped')",
      "max_attempts": "integer (default: 3, max: 5)",
      "attempts": "integer (default: 0)"
    }
  ],
  "loop_config": {
    "max_iterations": "integer (≥ 1, ≤ 200, default: 50)",
    "rate_limit_backoff_seconds": "integer (≥ 60, ≤ 3600, default: 300)",
    "timeout_per_task_seconds": "integer (≥ 30, ≤ 1800, default: 600)"
  }
}
```

**Constraints:**
- Tasks MUST be sorted by `priority` ascending.
- No duplicate `task_id` values.
- `attempts` MUST NOT exceed `max_attempts`. When `attempts` equals `max_attempts`, set `status` to `"skipped"`.
- Maximum 10,000 tasks per session.

### 3.12 Iteration Result (`state/iteration_result.json`)

```json
{
  "iteration_number": "integer (≥ 1)",
  "task_id": "string (TASK-NNNN)",
  "feature_id": "string (CAP-NNNN)",
  "started_at": "string (ISO 8601)",
  "completed_at": "string (ISO 8601)",
  "action": "string (enum: 'implement' | 'fix' | 'verify' | 'research')",
  "outcome": "string (enum: 'success' | 'failure' | 'partial' | 'rate_limited' | 'timeout')",
  "test_results": [
    {
      "test_name": "string",
      "command": "string",
      "exit_code": "integer",
      "stdout_tail": "string (last 500 chars of stdout)",
      "stderr_tail": "string (last 500 chars of stderr)",
      "duration_ms": "integer (≥ 0)"
    }
  ],
  "files_changed": ["string (paths relative to workspace/)"],
  "git_commit_sha": "string | null (null when outcome ≠ 'success')"
}
```

**Constraints:**
- `test_results` MUST contain at least one entry: the parity test defined in the feature's `parity_test.command`.
- When `outcome` = `"success"`, `git_commit_sha` MUST be a 40-character hex string.
- `stdout_tail` and `stderr_tail` MUST NOT exceed 500 characters each. Truncate from the beginning if the full output is longer.

### 3.13 System State (`state/system_state.json`)

```json
{
  "current_phase": "string (enum: 'uninitialized' | 'inventory' | 'pattern_resolution' | 'contract_generation' | 'closing_loop' | 'complete' | 'halted')",
  "current_substate": "string (see State Machine section)",
  "last_updated": "string (ISO 8601)",
  "halt_reason": "string | null (non-null only when current_phase = 'halted')",
  "inventory_status": {
    "map_total_workers": "integer (≥ 0)",
    "map_completed_workers": "integer (≥ 0)",
    "map_failed_workers": "integer (≥ 0)",
    "reduce_complete": "boolean",
    "research_complete": "boolean",
    "ambiguous_item_count": "integer (≥ 0)",
    "resolved_item_count": "integer (≥ 0)"
  },
  "confidence_audit_status": {
    "audit_complete": "boolean",
    "accuracy_pct": "number | null (0.0–100.0)",
    "reclassified_count": "integer (≥ 0)"
  },
  "pattern_resolution_status": {
    "deterministic_grouping_complete": "boolean",
    "chunk_analysis_complete": "boolean",
    "total_chunks": "integer (≥ 0)",
    "completed_chunks": "integer (≥ 0)",
    "total_groups": "integer (≥ 0)",
    "ledger_complete": "boolean"
  },
  "contract_status": {
    "feature_list_generated": "boolean",
    "feature_count": "integer (≥ 0)",
    "immutability_hash": "string | null"
  },
  "closing_status": {
    "total_features": "integer (≥ 0)",
    "features_passing": "integer (≥ 0)",
    "features_failing": "integer (≥ 0)",
    "features_untested": "integer (≥ 0)",
    "coverage_pct": "number (0.0–100.0)",
    "sessions_completed": "integer (≥ 0)",
    "total_iterations": "integer (≥ 0)",
    "total_regressions": "integer (≥ 0)",
    "last_session_id": "string | null (UUID v4)"
  }
}
```

**Constraints:**
- `closing_status.features_passing + closing_status.features_failing + closing_status.features_untested` MUST equal `closing_status.total_features`.
- `closing_status.coverage_pct` = `closing_status.features_passing / closing_status.total_features * 100` (0.0 when `total_features` = 0). This measures **capability coverage**, not symbol coverage.

### 3.14 Progress Log (`state/progress.txt`)

Each line follows this format:
```
ISO8601_TIMESTAMP | ENTITY_ID | ACTION | OUTCOME | DETAIL_STRING
```

- `ENTITY_ID`: one of `SYSTEM`, `W-NNNN` (worker), `TASK-NNNN`, `CAP-NNNN`, `GRP-NNNN`, `AUDIT`.
- `ACTION`: one of `MAP`, `REDUCE`, `RESEARCH`, `AUDIT`, `PATTERN_RESOLVE`, `GENERATE_CONTRACT`, `BOOTSTRAP`, `IMPLEMENT`, `FIX`, `VERIFY`, `COMMIT`, `REGRESS`, `HALT`, `RESUME`.
- `OUTCOME`: one of `OK`, `FAIL`, `SKIP`, `TIMEOUT`, `RATE_LIMITED`, `RECLASSIFIED`.
- `DETAIL_STRING`: free text, max 200 chars. No newlines. No pipe characters.

This file is **append-only**. No line is ever deleted or modified after being written.

---

## 4. Phase 1: Inventory

### 4.1 Map Stage

**Purpose:** Classify every symbol in the C++ source tree as `ported`, `not_ported`, `ambiguous`, `superseded`, or `not_applicable`.

**Procedure:**

1. The orchestrator enumerates all `.cpp`, `.cc`, `.cxx`, `.h`, `.hpp`, `.hxx` files under `cpp_source/`. Files matching patterns in `.portignore` (if present) are excluded. If no `.portignore` exists, all files are included. The orchestrator writes the enumeration to `state/source_manifest.json` conforming to Schema 3.2, computing a SHA-256 checksum for each file. If `source_manifest.json` already exists (recovery scenario), the orchestrator re-hashes source files and compares against the stored checksums. If any checksum differs, the orchestrator MUST halt with error `"Source tree changed since enumeration; re-run with --force or harness reset"`.

2. The orchestrator partitions files into work units. Each work unit contains exactly 1 file. Maximum 50,000 work units. If the C++ source tree contains more than 50,000 files, the orchestrator MUST halt with error `"Source tree exceeds 50,000 files; partition manually"`. As workers are assigned, the orchestrator updates the corresponding `worker_id` field in `source_manifest.json`.

3. For each work unit, the orchestrator spawns a **map worker** — an isolated LLM invocation with:
   - **Input:** the file contents (read-only), the Rust target tree path (read-only), and the worker's assigned item ID range.
   - **Prompt:** instructs the worker to identify every public symbol in the file, search for a corresponding Rust implementation, and emit a classification. The prompt explicitly describes the expanded classification enum: workers SHOULD use `superseded` when they find a Rust construct that replaces the C++ symbol with a different architecture, and `not_applicable` when the symbol exists only due to C++ language limitations (e.g., manual memory management, explicit destructors for RAII).
   - **Output contract:** the worker MUST emit valid JSON conforming to Schema 3.1 (Map Worker Output). No other output is accepted.
   - **Isolation:** the worker has no access to other workers' outputs, the feature list, the task queue, or any mutable state file. It reads only its assigned C++ file and the Rust target tree.

4. **Parallelism:** up to 20 map workers execute concurrently. The orchestrator maintains a pool and schedules new workers as slots free up.

5. **Per-worker timeout:** 120 seconds. If a worker does not produce valid output within 120 seconds, the orchestrator kills it, logs the timeout to `progress.txt`, increments `map_failed_workers` in `system_state.json`, and moves the file to a retry queue.

6. **Retry policy:** each file is attempted a maximum of 3 times. After 3 failures, the orchestrator classifies all symbols in that file as `ambiguous` with `ambiguity_reason: "parse_failure"` and writes a synthetic worker output file.

7. **Output validation:** the orchestrator validates each worker's output against Schema 3.1 before accepting it. Validation checks:
   - JSON parses without error.
   - `worker_id` matches the assigned worker ID.
   - Every `item_id` falls within the worker's assigned range.
   - Every `classification` value is one of the allowed enum values.
   - `confidence` is present and in [0.0, 1.0] for ambiguous items, absent/null for ported/not_ported.
   - `superseding_construct` is present and non-empty when classification is `superseded` or `not_applicable`.
   - `items` array length ≤ 10,000.
   If validation fails, the output is rejected and the file enters the retry queue.

### 4.2 Reduce Stage

**Purpose:** Merge all map worker outputs into a single ambiguous-items file and a set of confirmed classifications.

**Procedure:**

1. The reducer reads all `state/map_results/worker_*.json` files.

2. For each item across all workers:
   - If `classification` ∈ {`"ported"`, `"not_ported"`, `"superseded"`, `"not_applicable"`}: the item goes directly to the coverage report data (held in memory or a temporary file).
   - If `classification` = `"ambiguous"`: the item goes to `state/ambiguous_items.json`.

3. The reducer verifies completeness against `state/source_manifest.json`: every file in the manifest MUST have a corresponding worker output (matched by `source_file` path). Files without a valid worker output trigger a retry of the map stage.

4. The reducer writes `state/ambiguous_items.json` conforming to Schema 3.3.

5. The reducer updates `system_state.json`: set `reduce_complete: true`, record `ambiguous_item_count`.

6. **Termination condition for reduce:** all worker outputs have been read and sorted. The reducer is a deterministic, non-LLM process — it runs as code, not as an agent.

### 4.3 Research Stage

**Purpose:** Resolve every ambiguous item by conducting targeted investigation.

**Trigger condition:** `ambiguous_items.json` exists AND `total_ambiguous` > 0. If `total_ambiguous` = 0, skip directly to Coverage Report Generation.

**Procedure:**

1. The research agent reads `state/ambiguous_items.json`.

2. For each ambiguous item (processed sequentially to conserve API budget):
   - The agent formulates search queries based on the `cpp_symbol`, `kind`, and `ambiguity_reason`.
   - The agent searches the Rust target tree, documentation, git history, and (if configured) web sources.
   - After each search round, the agent evaluates: does the evidence resolve the ambiguity? If yes, record the resolution. If no, refine the query and search again.
   - The agent MAY resolve items as `superseded` if it finds that the C++ symbol's behavior is subsumed by a Rust construct with different architecture.
   - **Maximum search rounds per item:** 10. If 10 rounds elapse without resolution, the item moves to `unresolved` with reason `"exhausted_search_budget"`.
   - **Maximum total research time:** 4 hours wall-clock. If the research agent exceeds 4 hours, it MUST stop, write results for all items processed so far, and move remaining items to `unresolved` with reason `"exhausted_search_budget"`.

3. The research agent writes `state/research_results.json` conforming to Schema 3.4.

4. The research agent updates `system_state.json`: set `research_complete: true`, record `resolved_item_count`.

5. Log each resolution to `progress.txt`.

### 4.4 Coverage Report Generation

**Purpose:** Merge map results and research results into a single coverage report.

**Trigger condition:** reduce stage complete AND (research stage complete OR zero ambiguous items).

**Procedure:**

1. Read all confirmed classifications from the reduce stage.
2. Read `state/research_results.json` (if it exists).
3. For each resolved item, use the research agent's classification. For each unresolved item, set `status: "not_ported"` and `resolution_source: "manual"` (conservative default).
4. Compute summary statistics (including the new `superseded` count).
5. Write `state/coverage_report.json` conforming to Schema 3.5.
6. Validate: `summary.total` equals `items.length + unresolved_items.length`. All counts sum correctly. If validation fails, recompute from raw data.

### 4.5 Confidence Audit

**Purpose:** Detect confident-but-wrong classifications that bypass the ambiguous→research pipeline. Isolated map workers can make high-confidence errors (especially `superseded` and `ported` classifications) that no downstream process would otherwise catch.

**Trigger condition:** coverage report exists and passes validation.

**Procedure:**

1. The auditor selects a **stratified random sample** from the coverage report. Sampling rates:
   - `superseded` items: audit 20% (or 50 items, whichever is larger, capped at 200).
   - `ported` items with `confidence` < 0.8: audit 15% (capped at 200).
   - `not_applicable` items: audit 20% (capped at 200).
   - `ported` items with `confidence` ≥ 0.8: audit 5% (capped at 100).
   - `not_ported` items: audit 5% (capped at 100).

2. For each sampled item, a research-grade LLM agent re-evaluates the classification by:
   - Reading the original C++ source file and the referenced Rust construct.
   - Independently classifying the symbol.
   - Comparing its classification to the map worker's classification.
   - Recording whether the original classification was correct.

3. If a sampled item is found to be misclassified:
   - Update the item in the coverage report with the corrected classification and `resolution_source: "confidence_audit"`.
   - Log the reclassification to `progress.txt`.

4. Write `state/confidence_audit.json` conforming to Schema 3.6.

5. Update `system_state.json`: set `confidence_audit_status.audit_complete: true`, record accuracy and reclassification count.

6. **Accuracy threshold:** if any classification category has accuracy below 85%, log a warning. The operator may choose to increase audit sample sizes, refine map worker prompts, or accept the accuracy level. The system does NOT halt on low accuracy — it proceeds with the audit corrections applied.

---

## 5. Phase 2: Pattern Resolution

**Purpose:** Detect cross-file architectural patterns (consolidation, idiom replacement, library/language subsumption) that isolated map workers cannot identify, and produce a traceability ledger accounting for every C++ symbol.

**Trigger condition:** coverage report exists, confidence audit complete (or skipped if zero items audited).

This phase has two stages: deterministic grouping (non-LLM) and directory-based chunk analysis (LLM-assisted).

### 5.1 Deterministic Grouping

**Purpose:** Identify trivial cross-file patterns by grouping coverage report items that share the same `rust_equivalent` path.

**Procedure:**

1. Read `state/coverage_report.json`.

2. Group items by `rust_equivalent` (non-null values only):
   - Items sharing the same `rust_equivalent` are placed in a candidate group.
   - Single-item groups are `one_to_one` patterns.
   - Multi-item groups are `consolidation` candidates.

3. For each candidate group:
   - If all members have `status: "ported"` and share the same `rust_equivalent`: create a group with `pattern_type: "consolidation"` and `detection_method: "deterministic_grouping"`.
   - If members have mixed statuses: flag the group for chunk analysis (Stage 2).

4. Items with `status: "not_applicable"` or `"superseded"`: group by `superseding_construct` using the same logic.

5. Items with `status: "not_ported"` and no `rust_equivalent`: leave ungrouped for chunk analysis.

6. Write preliminary groups to `state/pattern_map.json`. Update `system_state.json`: set `pattern_resolution_status.deterministic_grouping_complete: true`.

7. This stage is **deterministic and idempotent**. Given the same coverage report, it produces the same pattern map.

### 5.2 Directory-Based Chunk Analysis

**Purpose:** Detect non-trivial cross-file patterns among items not resolved by deterministic grouping. Operates on compressed symbol-level data (names, classifications, Rust targets — not full source code) chunked by C++ source directory.

**Trigger condition:** deterministic grouping complete AND ungrouped items exist with `status` ∈ {`"not_ported"`, `"partial"`}.

**Procedure:**

1. Partition ungrouped coverage report items by C++ source directory (the directory component of `cpp_source` path). Each directory becomes one chunk.

2. For each chunk, the orchestrator spawns a **pattern resolver agent** with:
   - **Input:** the chunk's items (item_id, cpp_symbol, kind, status, rust_equivalent, superseding_construct), the preliminary pattern map (groups detected so far), and read access to the Rust target tree.
   - **Prompt:** instructs the agent to identify cross-file patterns within this directory. The agent looks for: name similarity clusters pointing to the same Rust module, C++ class hierarchies that map to Rust enums, utility function clusters replaced by standard library calls, and template families replaced by generics.
   - **Output contract:** the agent emits a JSON array of new groups conforming to the `groups` schema in 3.7. Each group references item_ids from its chunk.
   - **Timeout:** 120 seconds per chunk. Timed-out chunks leave their items ungrouped.

3. **Parallelism:** up to 10 chunk resolvers execute concurrently.

4. Each chunk's output is written to `state/pattern_resolution/chunk_NNNN.json`. The orchestrator validates each output before accepting it.

5. After all chunks complete, merge chunk outputs into `state/pattern_map.json`:
   - Append chunk groups to the existing deterministic groups.
   - Verify no item_id appears in multiple groups. If a conflict is detected, the deterministic group takes precedence and the chunk group is logged as a conflict.

6. For remaining ungrouped `not_ported` items: create `one_to_one` groups (one item per group). Every item MUST belong to exactly one group.

7. Update `system_state.json`: set `pattern_resolution_status.chunk_analysis_complete: true`, record `total_groups`.

8. **Recovery:** chunk processing is checkpointed per-chunk. If the orchestrator crashes at chunk 47 of 100, it reads existing chunk output files and resumes from the first chunk without a valid output.

### 5.3 Traceability Ledger Generation

**Purpose:** Write one ledger entry per C++ symbol, documenting its disposition (what happened to it in the Rust port).

**Trigger condition:** pattern map complete (all items grouped).

**Procedure:**

1. Read `state/pattern_map.json` and `state/coverage_report.json`.

2. For each group in the pattern map:
   - For each member item in the group:
     - Determine `disposition` from the group's `pattern_type`:
       - `one_to_one` with `status: "not_ported"` → `disposition: "mapped"` (will become a feature)
       - `one_to_one` with `status: "ported"` → `disposition: "mapped"` (will become a verify feature)
       - `consolidation` → `disposition: "consolidated"`
       - `idiom_replacement` → `disposition: "superseded"`
       - `library_subsumption` → `disposition: "library_subsumed"`
       - `language_subsumption` → `disposition: "language_subsumed"`
     - Write a JSONL entry to `state/traceability_ledger.jsonl` conforming to Schema 3.8.

3. Validate: line count of `traceability_ledger.jsonl` equals `coverage_report.summary.total`.

4. Update `system_state.json`: set `pattern_resolution_status.ledger_complete: true`.

5. This stage is **deterministic given the pattern map**. It is idempotent — re-running truncates and rewrites the ledger.

---

## 6. Phase 3: Contract Generation

**Purpose:** Transform the pattern map into an immutable capability-level feature list.

**Trigger condition:** pattern map and traceability ledger are complete and validated.

**Procedure:**

1. Read `state/pattern_map.json` and `state/coverage_report.json`.

2. For each group in the pattern map:
   - If the group's `pattern_type` ∈ {`"library_subsumption"`, `"language_subsumption"`} AND all members have `status` ∈ {`"superseded"`, `"not_applicable"`}:
     - Do NOT create a feature. These are subsumed by language/library constructs and have no Rust user code to verify. Log the exclusion to `progress.txt`.
   - Otherwise, create a feature entry conforming to Schema 3.9:
     - Assign a sequential `CAP-NNNN` ID.
     - Set `description` to the group's `description` field (or generate from the primary member's `cpp_symbol` for `one_to_one` groups).
     - Set `cpp_provenance` to an array of all member items (item_id, cpp_symbol, cpp_source).
     - Set `rust_target` to the group's `rust_target`.
     - Set `pattern_type` and `group_id` from the group.
     - Set `category` based on the group's dominant `kind`:
       - `function`, `method` → `"functional_parity"`
       - `class`, `template`, `module` → `"api_compatibility"`
       - `macro` → `"functional_parity"`
       - `constant`, `type_alias` → `"api_compatibility"`
     - Override `category` to `"error_handling"` if any member symbol name contains `error`, `except`, `throw`, `catch`, `fail`, `abort` (case-insensitive).
     - Override `category` to `"edge_case"` if any member came from `unresolved` in the research results.
     - Set `architectural_note` from the group if the `pattern_type` indicates divergence (consolidation, idiom_replacement).
     - Generate `parity_test`:
       - `type`: `"golden_parity"` for consolidation groups with known visual output, `"unit"` for functions/methods/constants, `"integration"` for classes/templates/modules, `"property"` for macros.
       - `command`: `"cargo test --test parity_CAP_NNNN"` (the maintainer agent creates the actual test file during implementation).
       - `expected_exit_code`: `0`.
       - `timeout_seconds`: `60` for unit, `120` for integration/golden_parity, `180` for property.
     - Set `passes: false`, `last_verified: null`, `verification_evidence: null`.

3. For groups where all members have `status: "ported"`:
   - Create a feature with a verify-only parity test and `passes: false` (the closing loop MUST verify even pre-existing ports).

4. Compute the `immutability_hash` per the algorithm in Schema 3.9.

5. Write `state/feature_list.json` conforming to Schema 3.9.

6. Update the traceability ledger: for each feature created, update the `target` field of its member items' ledger entries to the feature's `CAP-NNNN` ID. (This is the one exception to append-only: ledger entries written during Phase 2 with `target: null` are updated in-place to reference their feature ID.)

7. Copy `state/feature_list.json` to `state/backups/feature_list.json.bak`.

8. Update `system_state.json`: set `current_phase: "contract_generation"`, `contract_status.feature_list_generated: true`, record `feature_count` and `immutability_hash`.

**Validation after generation:**
- Parse the written file back from disk and recompute `immutability_hash`. It MUST match the hash stored in the file. If it does not match, halt with error `"Feature list hash verification failed after write"`.
- Verify all `CAP-NNNN` IDs are unique and sequential.
- Verify all `parity_test.command` fields are non-empty.
- Verify every feature's `cpp_provenance` references valid item_ids from the coverage report.

---

## 7. Phase 4: Closing Loop

### 7.1 Initializer Agent (runs exactly once)

**Trigger condition:** `feature_list.json` exists, `session_context.json` does NOT exist.

**Procedure:**

1. Read `state/feature_list.json` and `state/system_state.json`.

2. Create `state/session_context.json` conforming to Schema 3.10:
   - Generate a new `project_id` (UUID v4).
   - Set `initialized_at` to current timestamp.
   - Set `initializer_version` to `"2.0.0"`.
   - Set paths and commands.
   - Set `last_session` to `null`.

3. Create or verify the bootstrap script referenced by `bootstrap_command`:
   - The script MUST be idempotent (safe to run multiple times).
   - The script MUST exit 0 on success, non-zero on failure.
   - The script sets up the Rust build environment, installs dependencies, and verifies the Rust target compiles.

4. Verify the test command works by running it. If it fails (and there are no pre-existing passing tests), that is acceptable — the test infrastructure just needs to execute without crashing.

5. Create initial `state/progress.txt` with a header line:
   ```
   ISO8601 | SYSTEM | BOOTSTRAP | OK | Initializer agent completed setup
   ```

6. Commit all state files to git with message: `"harness: initializer complete, N capabilities queued"`.

7. Update `system_state.json`: set `current_phase: "closing_loop"`, `current_substate: "CLOSING_PAUSED"`.

**Partial failure recovery:** if the initializer crashes after creating some but not all artifacts, the next invocation MUST detect existing artifacts, validate them, and only create missing ones. The initializer MUST NOT overwrite valid existing artifacts.

### 7.2 Maintainer Agent Bootstrap Ritual (runs at the start of every session)

**Trigger condition:** `session_context.json` exists.

The maintainer MUST execute these steps in order before performing any implementation work:

1. **Verify working directory:** confirm `pwd` is the workspace root.
2. **Read system state:** load `state/system_state.json`. If `current_phase` = `"complete"`, print summary and exit. If `current_phase` = `"halted"`, print halt reason and exit.
3. **Read git log:** examine the last 20 commits for context.
4. **Read progress log:** load the last 50 lines of `state/progress.txt`.
5. **Read session context:** load `state/session_context.json`.
6. **Read feature list:** load `state/feature_list.json`. Recompute `immutability_hash` and compare. If mismatch: halt with error `"Feature list integrity check failed"`.
7. **Run bootstrap command:** execute `session_context.bootstrap_command`. If it exits non-zero, halt with error `"Bootstrap failed"`.
8. **Run existing tests:** execute `session_context.test_command`. Record which features pass and which fail. Update `feature_list.json` for any features whose pass status changed (in either direction).
9. **Detect regressions:** if any feature that had `passes: true` now fails its test, log a regression to `progress.txt` and create a `fix` task with priority 1 (highest).
10. **Generate task queue:** create `state/task_queue.json` from the feature list:
    - For each feature with `passes: false`: create a task with `action: "implement"` (or `action: "fix"` if the feature previously had `passes: true`).
    - Sort tasks by priority. Priority assignment: `fix` tasks get priority 1, `implement` tasks are ordered by feature ID (lower IDs = higher priority).
    - Set `loop_config` defaults or use values from `session_context.json` if overridden.

### 7.3 Task Loop Execution

**Trigger condition:** `task_queue.json` exists with at least one task where `status` = `"pending"`.

**Loop invariant:** iteration counter starts at 1 and increments by 1 per iteration. The loop terminates when ANY of these conditions is true:
- Iteration counter exceeds `loop_config.max_iterations`.
- All tasks in `task_queue.json` have `status` ∈ {`"completed"`, `"skipped"`}.
- All features in `feature_list.json` have `passes: true`.
- The operator sends SIGINT (manual stop).

**Each iteration:**

1. **Select task:** pick the first task in `task_queue.json` with `status: "pending"`. If none remain, terminate loop. Set `status: "in_progress"`, increment `attempts`.

2. **Execute task in fresh context:** spawn a new agent invocation (isolated context) with:
   - The task description, the target feature's entry from `feature_list.json` (including `cpp_provenance` for traceability to C++ source), and read access to the Rust target tree.
   - For multi-provenance features (consolidation patterns), the agent receives all referenced C++ source locations so it can understand the full scope of the capability.
   - The agent implements the required Rust code and/or writes the parity test.
   - **Timeout:** `loop_config.timeout_per_task_seconds`. If exceeded, kill the agent, set task outcome to `"timeout"`.

3. **Run parity test:** execute the feature's `parity_test.command` with timeout `parity_test.timeout_seconds`.
   - If exit code matches `expected_exit_code`: outcome = `"success"`.
   - If exit code differs: outcome = `"failure"`.
   - If command times out: outcome = `"timeout"`.

4. **Record iteration result:** write `state/iteration_result.json` conforming to Schema 3.12.

5. **Update state based on outcome:**
   - **Success:**
     - Set task `status: "completed"`.
     - Update feature in `feature_list.json`: set `passes: true`, `last_verified` to current timestamp, `verification_evidence` to test stdout tail (max 1000 chars). Increment `version`. Recompute `immutability_hash`. Back up before writing.
     - Commit changed files to git with message: `"port: CAP-NNNN passes — <description>"`.
     - Update `system_state.json` counters.
   - **Failure:**
     - If `attempts < max_attempts`: set task `status: "pending"` (will retry next iteration).
     - If `attempts >= max_attempts`: set task `status: "skipped"`, log to `progress.txt`.
   - **Timeout:**
     - Same as failure handling.
   - **Rate limited** (detected by specific error patterns in agent output):
     - Do NOT increment `attempts`.
     - Set task `status: "pending"`.
     - Wait `loop_config.rate_limit_backoff_seconds` before next iteration.
     - If 3 consecutive rate limits occur, terminate the loop. Log to `progress.txt`.

6. **Append to progress log:** write a line to `state/progress.txt`.

7. **Check termination conditions** (see loop invariant above).

### 7.4 Session Completion

When the task loop terminates:

1. Update `state/session_context.json`:
   - Set `last_session.session_id` to a new UUID.
   - Set `last_session.completed_at` to current timestamp.
   - Set `last_session.features_completed` to the list of CAP-NNNN IDs completed this session.
   - Recount `features_remaining` from `feature_list.json`.
   - Compute `coverage_at_end`.

2. Update `system_state.json`:
   - If all features pass: set `current_phase: "complete"`, `current_substate: "COMPLETE"`.
   - Otherwise: set `current_substate: "CLOSING_PAUSED"`, increment `sessions_completed`.

3. Commit state files to git: `"harness: session N complete, X/Y capabilities passing (Z%)"`.

4. Print a session summary to stdout:
   ```
   Session complete.
   Capabilities passing: X / Y (Z%)
   Capabilities completed this session: N
   Regressions detected: R
   Tasks skipped (max attempts): S
   ```

---

## 8. State Machine

### 8.1 States

| State | Phase | Description |
|-------|-------|-------------|
| `UNINITIALIZED` | — | No state files exist. System has not started. |
| `INVENTORY_MAP` | Inventory | Map workers are processing C++ source files. |
| `INVENTORY_REDUCE` | Inventory | Reducer is merging map worker outputs. |
| `INVENTORY_RESEARCH` | Inventory | Research agent is resolving ambiguous items. |
| `INVENTORY_MERGE` | Inventory | Coverage report is being generated from map + research results. |
| `CONFIDENCE_AUDIT` | Inventory | Sampling audit of confident classifications. |
| `PATTERN_DETERMINISTIC` | Pattern Resolution | Deterministic grouping by shared rust_equivalent. |
| `PATTERN_CHUNKS` | Pattern Resolution | Directory-based chunk analysis by LLM agents. |
| `PATTERN_LEDGER` | Pattern Resolution | Traceability ledger generation. |
| `CONTRACT_GENERATING` | Contract | Feature list is being generated from pattern map. |
| `CONTRACT_VALIDATED` | Contract | Feature list generated, hash verified. Waiting for initializer. |
| `CLOSING_INIT` | Closing | Initializer agent is setting up session infrastructure. |
| `CLOSING_BOOTSTRAP` | Closing | Maintainer is executing bootstrap ritual. |
| `CLOSING_ACTIVE` | Closing | Task loop is running. |
| `CLOSING_PAUSED` | Closing | Session ended. Waiting for next maintainer session. |
| `REGRESSION_DETECTED` | Closing | A previously passing feature now fails. Fix tasks queued. |
| `COMPLETE` | — | All features pass. Port is verified. |
| `HALTED` | — | Unrecoverable error. Manual intervention required. |

### 8.2 Transitions

```
UNINITIALIZED → INVENTORY_MAP
    trigger: operator runs "harness start"
    action:  create system_state.json, enumerate source files, spawn map workers

INVENTORY_MAP → INVENTORY_REDUCE
    trigger: all map workers completed or exhausted retries
    action:  launch reducer

INVENTORY_REDUCE → INVENTORY_RESEARCH
    trigger: reduce complete AND ambiguous_item_count > 0
    action:  launch research agent

INVENTORY_REDUCE → INVENTORY_MERGE
    trigger: reduce complete AND ambiguous_item_count = 0
    action:  generate coverage report directly from map results

INVENTORY_RESEARCH → INVENTORY_MERGE
    trigger: research complete (all items resolved or budget exhausted)
    action:  merge map results + research results into coverage report

INVENTORY_MERGE → CONFIDENCE_AUDIT
    trigger: coverage report written and validated
    action:  begin confidence audit sampling

CONFIDENCE_AUDIT → PATTERN_DETERMINISTIC
    trigger: audit complete (or skipped if zero items to audit)
    action:  begin deterministic grouping

PATTERN_DETERMINISTIC → PATTERN_CHUNKS
    trigger: deterministic grouping complete AND ungrouped not_ported items exist
    action:  begin directory-based chunk analysis

PATTERN_DETERMINISTIC → PATTERN_LEDGER
    trigger: deterministic grouping complete AND no ungrouped not_ported items
    action:  generate traceability ledger

PATTERN_CHUNKS → PATTERN_LEDGER
    trigger: all chunks processed and merged into pattern map
    action:  generate traceability ledger

PATTERN_LEDGER → CONTRACT_GENERATING
    trigger: traceability ledger complete and validated
    action:  begin feature list generation

CONTRACT_GENERATING → CONTRACT_VALIDATED
    trigger: feature list written, hash verified, backup created
    action:  log completion, await initializer

CONTRACT_VALIDATED → CLOSING_INIT
    trigger: operator runs "harness init" OR automatic after CONTRACT_VALIDATED
    action:  launch initializer agent

CLOSING_INIT → CLOSING_PAUSED
    trigger: initializer completes all setup artifacts
    action:  commit artifacts, log completion

CLOSING_PAUSED → CLOSING_BOOTSTRAP
    trigger: operator runs "harness session" OR maintainer agent starts
    action:  begin bootstrap ritual

CLOSING_BOOTSTRAP → CLOSING_ACTIVE
    trigger: bootstrap ritual completes, task queue generated
    action:  start task loop

CLOSING_BOOTSTRAP → REGRESSION_DETECTED
    trigger: bootstrap test run reveals a previously passing feature now fails
    action:  create fix tasks with priority 1

REGRESSION_DETECTED → CLOSING_ACTIVE
    trigger: fix tasks created and queued
    action:  start task loop (fix tasks run first due to priority 1)

CLOSING_ACTIVE → CLOSING_PAUSED
    trigger: loop terminates (max iterations, all tasks done, or manual stop) AND features remain
    action:  write session summary, update state

CLOSING_ACTIVE → REGRESSION_DETECTED
    trigger: a parity test that previously passed now fails during the loop
    action:  create fix task with priority 1, insert at front of queue

CLOSING_ACTIVE → COMPLETE
    trigger: all features in feature_list.json have passes = true
    action:  write final summary, update system_state.json

Any state → HALTED
    trigger: unrecoverable error (see Failure Catalog)
    action:  write halt_reason to system_state.json, log to errors.log
```

### 8.3 Recovery from Each State

| State | Recovery Procedure |
|-------|-------------------|
| `UNINITIALIZED` | Run `harness start`. |
| `INVENTORY_MAP` | Restart: the orchestrator reads `source_manifest.json`, re-hashes source files to verify no changes, reads existing worker outputs, and only re-spawns workers for manifest entries without valid outputs. |
| `INVENTORY_REDUCE` | Re-run reducer. It is deterministic and idempotent. |
| `INVENTORY_RESEARCH` | The research agent reads `research_results.json` (if partial) and continues from the first unresolved item. |
| `INVENTORY_MERGE` | Re-run merge. It is deterministic and idempotent. |
| `CONFIDENCE_AUDIT` | Re-run audit. Already-audited items are detected from existing `confidence_audit.json` and skipped. |
| `PATTERN_DETERMINISTIC` | Re-run. It is deterministic and idempotent. |
| `PATTERN_CHUNKS` | Read existing chunk output files. Re-spawn resolvers only for chunks without valid outputs. |
| `PATTERN_LEDGER` | Re-run. Truncate and rewrite the ledger (deterministic given the pattern map). |
| `CONTRACT_GENERATING` | Re-run generation. If `feature_list.json` already exists and passes hash validation, skip. |
| `CONTRACT_VALIDATED` | Run `harness init`. |
| `CLOSING_INIT` | Re-run initializer. It detects existing artifacts and only creates missing ones. |
| `CLOSING_BOOTSTRAP` | Re-run maintainer agent. Bootstrap ritual is idempotent. |
| `CLOSING_ACTIVE` | Re-run maintainer agent. Bootstrap ritual detects task queue, resumes from first pending task. |
| `CLOSING_PAUSED` | Run `harness session`. |
| `REGRESSION_DETECTED` | Re-run maintainer agent. Bootstrap detects regressions and creates fix tasks. |
| `COMPLETE` | No action needed. Run `harness status` to view summary. |
| `HALTED` | Read `system_state.json` → `halt_reason`. Fix the root cause. Run `harness resume`. |

---

## 9. Failure Catalog

| # | Failure | Detection | Resolution | Affected State |
|---|---------|-----------|------------|----------------|
| F01 | Map worker timeout | Worker produces no output within 120s | Kill worker, retry (max 3). After 3 failures, synthesize ambiguous output. | `INVENTORY_MAP` |
| F02 | Map worker invalid output | JSON parse error or schema validation failure | Reject output, retry (max 3). After 3 failures, synthesize ambiguous output. | `INVENTORY_MAP` |
| F03 | Map worker crash | Worker process exits non-zero without valid JSON | Same as F01. | `INVENTORY_MAP` |
| F04 | Source tree too large | File count > 50,000 | HALT. Operator must partition source tree. `halt_reason: "Source tree exceeds 50,000 files"`. | `INVENTORY_MAP` |
| F05 | Source tree changed since enumeration | Re-hashed file checksum differs from `source_manifest.json` | HALT. `halt_reason: "Source tree changed since enumeration"`. Operator must `harness reset` or `harness start --force` to re-enumerate. | `INVENTORY_MAP` |
| F06 | All map workers fail for a partition | 3 consecutive retries fail for a file | Synthesize all symbols as ambiguous. Research agent investigates. | `INVENTORY_MAP` |
| F07 | Confident-wrong supersession | Map worker classifies symbol as `superseded` but the superseding construct does not cover the symbol's behavior | Detected by confidence audit (Section 4.5). Reclassified in coverage report. If audit accuracy for `superseded` < 85%, operator warned. | `INVENTORY_MAP`, `CONFIDENCE_AUDIT` |
| F08 | Multiple workers point to same rust_equivalent | N workers independently classify N symbols as `superseded_by` the same Rust construct, but that construct only partially covers each | Detected during deterministic grouping (Section 5.1). The group is flagged if member count exceeds expected consolidation ratio (>10 members triggers review). Chunk analysis validates behavioral coverage. | `PATTERN_DETERMINISTIC`, `PATTERN_CHUNKS` |
| F09 | Ambiguous item count exceeds limit | `total_ambiguous` > 50,000 | HALT. Map worker prompts need refinement. `halt_reason: "Too many ambiguous items (>50,000)"`. | `INVENTORY_REDUCE` |
| F10 | Research agent infinite loop | 10 search rounds for a single item without resolution | Move item to `unresolved` with reason `"exhausted_search_budget"`. | `INVENTORY_RESEARCH` |
| F11 | Research time budget exceeded | Wall-clock > 4 hours | Stop research. Write partial results. Remaining items → `unresolved`. | `INVENTORY_RESEARCH` |
| F12 | Coverage report item count mismatch | `summary.total` ≠ sum of status counts | Recompute summary from items array. | `INVENTORY_MERGE` |
| F13 | Confidence audit accuracy below threshold | Any classification category < 85% accuracy | Log warning. Operator may increase audit sample size, refine map worker prompts, or accept. System continues. | `CONFIDENCE_AUDIT` |
| F14 | Pattern resolver chunk timeout | Resolver does not complete within 120s | Kill resolver. Items in that chunk remain ungrouped → become `one_to_one` features. | `PATTERN_CHUNKS` |
| F15 | Pattern resolver invalid output | JSON parse error or schema validation | Reject output, retry (max 2). After failures, items in chunk become `one_to_one` features. | `PATTERN_CHUNKS` |
| F16 | Pattern resolver creates conflicting groups | Two chunks assign the same item_id to different groups | Deterministic groups take precedence. Chunk groups are discarded for conflicting items. Logged to `progress.txt`. | `PATTERN_CHUNKS` |
| F17 | Traceability ledger incomplete | Line count ≠ coverage report total | Re-run ledger generation from pattern map. | `PATTERN_LEDGER` |
| F18 | Feature list hash mismatch after write | Recomputed hash ≠ stored hash | HALT. Possible serialization bug. `halt_reason: "Hash verification failed"`. | `CONTRACT_GENERATING` |
| F19 | Feature count exceeds limit | > 10,000 features generated | HALT. Operator must partition into sub-projects. `halt_reason: "Feature count exceeds 10,000"`. | `CONTRACT_GENERATING` |
| F20 | Initializer partial failure | `session_context.json` exists but is incomplete | Re-run initializer. It detects existing artifacts and creates missing ones. | `CLOSING_INIT` |
| F21 | Bootstrap command fails | Exit code ≠ 0 | HALT. `halt_reason: "Bootstrap command failed: <stderr>"`. Operator fixes environment. | `CLOSING_BOOTSTRAP` |
| F22 | Feature list integrity failure | `immutability_hash` mismatch during bootstrap | Restore from `state/backups/feature_list.json.bak`. If backup also fails, HALT. | `CLOSING_BOOTSTRAP` |
| F23 | Parity test flaky | Same test passes and fails on consecutive runs without code changes | Mark feature with `verification_evidence: "FLAKY"`. Create `verify` task (not `implement`). The verify task runs the test 5 times: passes only if 5/5 succeed. | `CLOSING_ACTIVE` |
| F24 | Task exceeds max attempts | `attempts` reaches `max_attempts` (default 3, max 5) | Set task `status: "skipped"`. Log to progress. Feature remains `passes: false`. | `CLOSING_ACTIVE` |
| F25 | Rate limit during task loop | Agent output contains rate-limit error pattern | Pause for `rate_limit_backoff_seconds`. Do not count as attempt. After 3 consecutive rate limits, terminate loop. | `CLOSING_ACTIVE` |
| F26 | Task timeout | Agent does not complete within `timeout_per_task_seconds` | Kill agent. Set outcome to `"timeout"`. Count as attempt. | `CLOSING_ACTIVE` |
| F27 | Git commit failure | `git commit` exits non-zero | Log error. Do NOT set task to `"completed"` — code changes exist but are uncommitted. Retry commit once. If second failure, HALT. | `CLOSING_ACTIVE` |
| F28 | Regression detected | Feature with `passes: true` now fails its parity test | Set `passes: false` in feature list. Create `fix` task with priority 1. Increment `total_regressions`. | `CLOSING_ACTIVE`, `CLOSING_BOOTSTRAP` |
| F29 | Filesystem corruption | JSON parse error on any state file | Attempt restore from backup. If no backup or backup also corrupt, HALT. `halt_reason: "State file corrupt: <path>"`. | Any |
| F30 | Progress log write failure | `progress.txt` write returns I/O error | Write to `state/progress_fallback.txt` instead. Log warning to stderr. | Any |
| F31 | C++ preprocessor metaprogramming | Map worker cannot parse complex macros, SFINAE, or template metaprogramming | Worker classifies symbols as `ambiguous` with `ambiguity_reason: "parse_failure"`. Research agent investigates. If research also fails, item enters feature list as `edge_case`. | `INVENTORY_MAP`, `INVENTORY_RESEARCH` |
| F32 | Rust equivalent structurally unrecognizable | Semantic parity exists but code structure differs significantly | Map worker classifies as `ambiguous` with `ambiguity_reason: "no_match"`. Research agent uses semantic analysis. If confirmed ported, set `resolution_source: "research_agent"`. | `INVENTORY_MAP`, `INVENTORY_RESEARCH` |
| F33 | Feature list backup rotation full | 5 backups already exist | Delete oldest backup (`.bak.4`) and rotate. | Any write to `feature_list.json` |
| F34 | All tasks skipped, features remain | Every task hit `max_attempts` but features still fail | HALT. `halt_reason: "All tasks exhausted without resolution; N features remain"`. Operator must investigate and either increase `max_attempts`, improve prompts, or manually fix. | `CLOSING_ACTIVE` |
| F35 | Concurrent agent access | Two maintainer sessions attempt to run simultaneously | The second session detects `current_substate: "CLOSING_ACTIVE"` in system_state.json and refuses to start. Print error: `"Another session is active"`. | `CLOSING_BOOTSTRAP` |
| F36 | Coverage at 100% before closing loop | All items classified as `ported` or `not_applicable` or `superseded` | Feature list contains only verify tasks. The closing loop runs in verify-only mode. If all verify tasks pass, system transitions to COMPLETE. | `CONTRACT_GENERATING` |
| F37 | Stale rust_equivalent after refactoring | Closing loop refactors Rust code, moving a function to a new path; feature's `rust_target` now points to a nonexistent location | Detected when the maintainer agent reads the feature and cannot find the target. The agent SHOULD update the test to reference the new location. The `rust_target` field is frozen, but the parity test command can be written to find the current location dynamically. If the agent cannot resolve the stale reference, the task fails normally via retry/skip. | `CLOSING_ACTIVE` |

---

## 10. Operational Procedures

### 10.1 Starting the System

```bash
harness start --cpp-source <path> --rust-target <path> [--parallelism 20] [--worker-timeout 120]
```

**Preconditions:**
- `cpp_source` path exists and contains C++ source files.
- `rust_target` path exists (may be empty or partial).
- No `state/system_state.json` exists (or it shows `UNINITIALIZED`).

**What happens:**
1. Creates `state/` directory structure.
2. Writes initial `system_state.json` with `current_phase: "inventory"`, `current_substate: "INVENTORY_MAP"`.
3. Enumerates source files, spawns map workers, runs through Phase 1 automatically.
4. Runs confidence audit.
5. Runs Phase 2 (Pattern Resolution) automatically.
6. Runs Phase 3 (Contract Generation) automatically.
7. Stops after Phase 3 (`CONTRACT_VALIDATED`). Prints feature count and capability coverage percentage.

### 10.2 Initializing the Closing Loop

```bash
harness init [--bootstrap-cmd <cmd>] [--test-cmd <cmd>]
```

**Preconditions:**
- `system_state.json` shows `CONTRACT_VALIDATED`.
- `feature_list.json` exists and passes hash validation.

**What happens:**
1. Runs the initializer agent (Section 7.1).
2. Transitions to `CLOSING_PAUSED`.

### 10.3 Running a Closing Session

```bash
harness session [--max-iterations 50] [--timeout-per-task 600] [--rate-limit-backoff 300]
```

**Preconditions:**
- `system_state.json` shows `CLOSING_PAUSED` or `REGRESSION_DETECTED`.
- `session_context.json` exists.
- No other session is active.

**What happens:**
1. Runs maintainer bootstrap ritual (Section 7.2).
2. Runs task loop (Section 7.3).
3. Runs session completion (Section 7.4).
4. Prints session summary.

### 10.4 Checking Status

```bash
harness status
```

**No preconditions.** Reads `system_state.json` and `feature_list.json` (if they exist) and prints:

```
Phase:           closing_loop
State:           CLOSING_PAUSED
Capabilities:    847 / 1203 passing (70.4%)
Sessions:        12 completed
Iterations:      487 total
Regressions:     3 total
Last session:    2026-03-09T14:22:00Z
```

### 10.5 Resuming from HALTED

```bash
harness resume
```

**Preconditions:**
- `system_state.json` shows `HALTED`.
- The operator has fixed the root cause described in `halt_reason`.

**What happens:**
1. Reads `halt_reason` from `system_state.json`.
2. Prints the halt reason and asks the operator to confirm the fix.
3. Determines the last valid state before the halt by reading `progress.txt`.
4. Transitions to that state.
5. Proceeds with normal execution.

### 10.6 Stopping a Running Session

Send `SIGINT` (Ctrl+C) to the harness process.

**What happens:**
1. The current task iteration is allowed to complete (up to `timeout_per_task_seconds`).
2. If the current task does not complete within the timeout, it is killed.
3. Session completion (Section 7.4) runs.
4. System transitions to `CLOSING_PAUSED`.
5. No data is lost — all completed work is committed.

### 10.7 Inspecting Intermediate State

All state files are human-readable JSON. The operator inspects them directly:

```bash
# View overall status
cat state/system_state.json | jq .

# View feature list summary
cat state/feature_list.json | jq '.features | group_by(.passes) | map({passes: .[0].passes, count: length})'

# View traceability ledger summary by disposition
cat state/traceability_ledger.jsonl | jq -s 'group_by(.disposition) | map({disposition: .[0].disposition, count: length})'

# View pattern groups
cat state/pattern_map.json | jq '.groups | group_by(.pattern_type) | map({type: .[0].pattern_type, count: length})'

# View recent progress
tail -20 state/progress.txt

# View last iteration result
cat state/iteration_result.json | jq .

# Check feature list integrity
# (recompute hash and compare — exact procedure in Schema 3.9)
```

### 10.8 Resetting the System

```bash
harness reset [--keep-coverage] [--keep-patterns] [--confirm]
```

**What happens:**
- Without flags: deletes all files under `state/`. System returns to `UNINITIALIZED`.
- With `--keep-coverage`: deletes everything except `coverage_report.json` and `confidence_audit.json`. System returns to `PATTERN_DETERMINISTIC` (ready to re-run pattern resolution).
- With `--keep-patterns`: deletes everything except coverage report, audit, pattern map, and ledger. System returns to `CONTRACT_GENERATING` (ready to regenerate the feature list).
- Requires `--confirm` flag to execute. Without it, prints what would be deleted and exits.

---

## 11. Glossary

| Term | Definition |
|------|-----------|
| **Observable capability** | A behavior that the C++ system provides to its users (internal or external) which must also exist in the Rust target. The unit of measurement for the feature list. Capabilities are defined by their contracts — inputs, outputs, and invariants — not by the symbols that implement them. |
| **Source manifest** | A persisted enumeration of all C++ source files with SHA-256 checksums, written at the start of the map stage. Serves as the authoritative input for worker scheduling, completeness verification in the reduce stage, and staleness detection on recovery. |
| **Map worker** | An isolated LLM invocation that processes exactly one C++ source file and emits a structured classification of each symbol found in that file. Map workers have no access to other workers' outputs or mutable system state. |
| **Reducer** | A deterministic (non-LLM) process that merges all map worker outputs, separating confirmed classifications from ambiguous items. The reducer's determinism is a load-bearing invariant for crash recovery. |
| **Research agent** | An LLM agent that resolves ambiguous item classifications through multi-round search and analysis. It operates sequentially on one item at a time. |
| **Confidence audit** | A sampling-based validation pass that re-evaluates a subset of confident (non-ambiguous) classifications to detect systematic errors from map workers, especially in `superseded` and `not_applicable` categories. |
| **Coverage report** | The merged output of the map-reduce and research phases. Contains every C++ symbol with its port status and resolution source. |
| **Pattern resolution** | A two-stage phase (deterministic grouping + directory-based chunk analysis) that detects cross-file architectural patterns invisible to isolated map workers. Produces a pattern map grouping symbols into capabilities. |
| **Pattern map** | The output of pattern resolution. Groups coverage report items into capability-level clusters by pattern type (consolidation, idiom replacement, library/language subsumption, one-to-one). |
| **Traceability ledger** | An append-only JSONL file containing one entry per C++ symbol, documenting its disposition (mapped to a capability, consolidated, subsumed by language/library, etc.). Provides auditability: an auditor can verify that every C++ symbol has been accounted for. |
| **Feature list** | The immutable contract defining every capability that must pass for the port to be considered complete. Each feature references one or more C++ symbols via `cpp_provenance` and has exactly one parity test. Only three fields per feature are mutable: `passes`, `last_verified`, `verification_evidence`. |
| **Immutability hash** | A SHA-256 digest computed over the feature list with mutable fields zeroed and provenance arrays canonically sorted. Detects unauthorized modification of capability descriptions, test commands, provenance, or other frozen fields. |
| **Parity test** | A test that verifies a specific Rust implementation is functionally equivalent to its C++ counterpart. Each feature has exactly one parity test. For consolidation features, the test covers the consolidated capability rather than individual symbols. |
| **Initializer agent** | An LLM agent that runs exactly once to create session infrastructure: session context, bootstrap script, progress log, and initial git commit. |
| **Maintainer agent** | An LLM agent that runs in each closing session. It performs a bootstrap ritual, generates a task queue, and executes the task loop. |
| **Bootstrap ritual** | The ordered sequence of checks the maintainer agent performs before any implementation work: verify directory, read state, read git log, read progress, read feature list, validate hash, run bootstrap command, run tests, detect regressions, generate task queue. |
| **Task loop** | The continuous execution cycle where the maintainer agent picks a task, spawns an isolated agent to execute it, runs the parity test, records the result, and updates state. |
| **Task queue** | The ordered list of tasks for the current session, sorted by priority. Generated from the feature list during bootstrap. |
| **Iteration** | One cycle of the task loop: select task → execute → test → record → update state. |
| **Regression** | A feature that previously had `passes: true` but now fails its parity test. Regressions generate fix tasks with priority 1. |
| **Session** | One invocation of the maintainer agent, from bootstrap through task loop to session completion. Multiple sessions are required to close a port. |
| **Closing loop** | The entire Phase 4 process: repeated sessions of the maintainer agent working through the task queue until all features pass. |
| **HALT** | An unrecoverable error state requiring manual intervention. The system records the reason in `system_state.json` and stops all processing. |
| **Work unit** | One C++ source file assigned to one map worker. Each work unit produces exactly one worker output file. |
| **Orchestrator** | The top-level process that manages the harness lifecycle: spawning workers, launching agents, monitoring state transitions, and enforcing bounds. |
| **Phantom item** | A feature list entry representing a C++ symbol that has no meaningful Rust equivalent — typically because the behavior is subsumed by a language feature, standard library, or architectural consolidation. The capability-level measurement model eliminates phantom items by not creating features for subsumed symbols. |
| **Architectural note** | A first-class field in the feature list documenting known intentional divergences between C++ and Rust implementations (e.g., "C++ uses native scanline solver; Rust uses polygon approximation"). Signals to the closing loop that the divergence is accepted. |
