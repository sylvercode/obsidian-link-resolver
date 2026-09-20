<!--
Sync Impact Report
Version change: 1.2.0 -> 1.3.0
Modified principles: none renamed
Added principles:
- IX. Native Documentation for Named Symbols (added)
Modified sections:
- Development Workflow (expanded to require native-language documentation review for named symbols)
Added sections: none
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

### VI. Interoperability & Cross-Technology Integration
The project MUST be designed for interoperability so it can be consumed by, or integrated with, other technology stacks regardless of the implementation language. Whatever language is chosen for a component (e.g., Rust), the design MUST preserve a clear path to interact with or be embedded in other ecosystems, at minimum .NET libraries and Node.js projects. To satisfy this, components MUST expose stable, language-agnostic integration surfaces — such as the machine-friendly CLI protocol (see Principle II), a C-compatible ABI / FFI boundary, or a documented data contract (JSON schema) — rather than assuming a single-language, in-process consumer. New features MUST NOT introduce hard dependencies that preclude .NET or Node.js integration without a documented justification and migration path. Rationale: guaranteeing cross-technology integration keeps the tool reusable across diverse agent and application environments and prevents lock-in that would force costly rewrites when embedding the tool elsewhere.

### VII. Reproducible Development Environment
The `.devcontainer/devcontainer.json` MUST always be complete and MUST reflect the full toolchain required to build, test, and run the project. Every language runtime, package manager, and tool the project depends on MUST be declared there so that a fresh container yields a working environment without manual setup. When a change introduces a new dependency, tool, or version requirement, the devcontainer configuration MUST be updated in the same change. Rationale: a complete, reproducible dev container guarantees consistent onboarding, keeps local and CI environments in parity, and eliminates "works on my machine" drift.

### VIII. Continuous Integration & Release Gating
A GitHub Actions CI workflow MUST be maintained as the authoritative source of build and test automation. Releases MUST be reproducible and test-gated: creating a semantic version tag of the form `vX.Y.Z` MUST trigger a release pipeline that first runs the full test suite (the test gate) and only produces release artifacts when all tests pass. The CI and release configuration MUST stay in sync with the devcontainer toolchain (see Principle VII) so that local, CI, and release builds are equivalent. Rationale: an automated, test-gated release process prevents shipping broken or unverified artifacts and ensures every tagged release is reproducible.

### IX. Native Documentation for Named Symbols
Every named symbol in the project — including modules, types, enums and enum variants, functions, methods, members, parameters, return values, and public or private constants — MUST be documented using the documentation mechanism that is native and industry-standard for that language and ecosystem. For Rust, use Rustdoc comments (`///` and `//!`) and module-level docs; for Python, use docstrings; for TypeScript or JavaScript, prefer JSDoc or the ecosystem's standard API documentation; and for C-family code, use the language's accepted comment or documentation style. Custom documentation formats are not allowed when a standard mechanism exists. Documentation MUST explain purpose, constraints, semantics, and side effects in a way that supports both humans and tools; undocumented or ambiguously documented symbols are treated as incomplete work. Rationale: clear, idiomatic documentation makes APIs discoverable, reviewable, and maintainable across ecosystems, and it keeps the project aligned with the conventions expected by the languages it integrates with.

## Performance & Output Constraints
- Target: fast cold-start times and minimal per-invocation overhead. Measure and document common-case latency in CI for regressions.
- Output: Machine mode (--format=json) MUST emit compact, single-line JSON with predictable field names and types. DO NOT include non-deterministic fields (timestamps) in primary result objects unless explicitly namespaced (e.g., metadata.timestamp).
- Token-cost guidance: prefer short, precise field names and avoid embedding large free-text blobs in machine output.

## Development Workflow
- All changes that affect CLI output, defaults, or performance MUST include test coverage and a short benchmark in the PR description.
- Code review MUST verify that output shape is unchanged or that a documented migration path exists for consumers.
- Every change that adds or modifies a named symbol MUST include native-language documentation in the same patch, using the standard mechanism for the language and ecosystem.
- CI gates: unit tests, contract tests for CLI output, and a lightweight performance check for critical commands.
- Any change that adds or updates a dependency, tool, or runtime version MUST update `.devcontainer/devcontainer.json` in the same change so the container remains complete (see Principle VII).
- Releases are cut by pushing a `vX.Y.Z` tag, which MUST run the full test suite as a gate before publishing artifacts; a failed test gate MUST block the release (see Principle VIII).

## Governance
Amendments to this constitution require a documented proposal in the repository, a pull request referencing the rationale and tests, and approval by at least one maintainer. Versioning policy:
- MAJOR when breaking changes to CLI output or public behavior are introduced.
- MINOR when new, backward-compatible features or principles are added.
- PATCH for clarifications, wording, or non-semantic refinements.
Compliance: PRs that touch CLI behavior or API surface must reference this constitution, include tests demonstrating no regressions, and verify that all named symbols use the native documentation standard appropriate to their language.

**Version**: 1.3.0 | **Ratified**: 2026-09-17 | **Last Amended**: 2026-09-20
