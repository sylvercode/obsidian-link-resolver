<!--
Sync Impact Report
Version change: [UNSET] -> 1.0.0
Modified principles:
- [PRINCIPLE_1_NAME] -> I. Performance & Agent Efficiency (added)
- [PRINCIPLE_2_NAME] -> II. CLI Interface & Consistent Output (added)
- [PRINCIPLE_3_NAME] -> III. Test-First (NON-NEGOTIABLE) (added)
- [PRINCIPLE_4_NAME] -> IV. Integration Testing (added)
- [PRINCIPLE_5_NAME] -> V. Observability, Versioning & Simplicity (added)
Added sections:
- Performance & Output Constraints
Removed sections: none
Follow-up TODOs: none
-->

# obsidian-link-resolver Constitution

## Core Principles

### I. Performance & Agent Efficiency
The CLI MUST be optimized for low-latency and minimal resource usage because it may be invoked frequently by automated agents. Avoid heavy startup costs, prefer native or precompiled binaries when practical, and ensure common paths execute without unnecessary initialization. Rationale: frequent invocation amplifies per-call latency and resource usage; fast tools reduce overall system load and improve agent throughput.

### II. CLI Interface & Consistent Output
The command-line interface MUST expose a stable, machine-friendly protocol: stdin/args → stdout for primary results, stderr for diagnostics and errors. Output formats MUST be deterministic and compact to reduce token cost for downstream agents. Provide a --format=json (machine) mode with a single-line, compact JSON representation, and a human-readable mode only for interactive use. Rationale: consistent, minimal output lowers token usage and parsing ambiguity across callers.

### III. Test-First (NON-NEGOTIABLE)
Tests MUST be authored before implementation for all new features and for any changes that affect output format, performance, or public behavior. Include unit tests, contract tests for CLI output, and performance/regression tests for critical paths. Rationale: preventing regressions in output shape and performance is essential for agent compatibility and token-cost stability.

### IV. Integration Testing
Integration tests MUST cover real-world invocation patterns (file inputs, path resolution, agent-like callers) and verify end-to-end behavior including exit codes, stdout shape, and error forms. Use lightweight agent mocks where appropriate. Rationale: agent callers rely on both behavior and performance of integrated flows, not only isolated units.

### V. Observability, Versioning & Simplicity
Prefer clear, minimal telemetry: structured logs to stderr with optional verbosity flags; avoid noisy defaults. Follow semantic versioning (MAJOR.MINOR.PATCH). Keep public CLI surface small and stable; prefer adding flags over changing defaults. Rationale: observability aids debugging; semantic versioning communicates breaking changes; simplicity reduces accidental token-cost growth.

## Performance & Output Constraints
- Target: fast cold-start times and minimal per-invocation overhead. Measure and document common-case latency in CI for regressions.
- Output: Machine mode (--format=json) MUST emit compact, single-line JSON with predictable field names and types. DO NOT include non-deterministic fields (timestamps) in primary result objects unless explicitly namespaced (e.g., metadata.timestamp).
- Token-cost guidance: prefer short, precise field names and avoid embedding large free-text blobs in machine output.

## Development Workflow
- All changes that affect CLI output, defaults, or performance MUST include test coverage and a short benchmark in the PR description.
- Code review MUST verify that output shape is unchanged or that a documented migration path exists for consumers.
- CI gates: unit tests, contract tests for CLI output, and a lightweight performance check for critical commands.

## Governance
Amendments to this constitution require a documented proposal in the repository, a pull request referencing the rationale and tests, and approval by at least one maintainer. Versioning policy:
- MAJOR when breaking changes to CLI output or public behavior are introduced.
- MINOR when new, backward-compatible features or principles are added.
- PATCH for clarifications, wording, or non-semantic refinements.
Compliance: PRs that touch CLI behavior must reference this constitution and include tests demonstrating no regressions.

**Version**: 1.0.0 | **Ratified**: 2026-09-17 | **Last Amended**: 2026-09-17
