# Implementation Plan: MCP Link Generation

**Branch**: `004-mcp-link-generation` | **Date**: 2026-10-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-mcp-link-generation/spec.md`

## Summary

Add a read-only `generate_obsidian_link` MCP tool that produces Obsidian-compatible Wikilink or Markdown text for existing or future note and attachment destinations, including subtargets, same-file shorthand, and embeds. Implement the tool in the existing MCP binary crate, reuse the resolver parser, vault index, cache, and exact-result statuses, and preserve the startup-configured vault boundary. Both output formats use vault-relative paths. Existing targets are reported as validated only when the generated candidate resolves to the exact requested target; future targets remain explicitly unverified.

## Technical Context

**Language/Version**: Rust, workspace edition 2021

**Primary Dependencies**: Existing `rmcp` 3.5.1, `serde`, `serde_json`, `schemars`, `tokio`, and `obsidian-link-resolver`; no new dependencies planned.

**Storage**: Local vault filesystem, read-only; use the existing process-scoped `VaultCache` and indexed `Vault` snapshot.

**Testing**: `cargo test -p obsidian-link-resolver-mcp`; MCP schema/contract tests and subprocess stdio integration tests. Extend the existing `mcp_warm` benchmark with generation and retain its 100 ms p50 gate.

**Target Platform**: Existing native MCP stdio server platforms supported by the workspace.

**Project Type**: Rust MCP server binary crate backed by the existing resolver library.

**Performance Goals**: Preserve the existing warm MCP p50 ceiling of 100 ms on the benchmark vault (up to 5,000 notes). Reuse the existing cache and avoid a second vault walk per request; measure the generation call in the existing benchmark.

**Constraints**: Read-only operation; strict request schema; exact vault-relative target paths; both output formats use vault-relative `/` paths; source and target must stay within startup-configured roots, including symlink and planned-source cases; heading and block selectors are mutually exclusive; unsupported or unfaithful encodings fail clearly. Existing resolver behavior and `resolve_obsidian_link` contract remain unchanged.

**Scale/Scope**: One new MCP tool plus its request/result model, renderer, validation flow, and focused tests; benchmark against the existing synthetic 5,000-note vault. Link discovery, document search, and note editing remain out of scope.

## Constitution Check

*GATE: Evaluated before Phase 0 research and re-checked after Phase 1 design.*

| Principle | Requirement | Plan compliance |
|---|---|---|
| I. Performance & Agent Efficiency | Low latency and minimal per-call overhead | Reuse the cached vault index; perform one focused target validation; add generation to the existing warm p50 benchmark. **Pass.** |
| II. CLI Interface & Consistent Output | Stable, compact, deterministic machine interfaces | This feature adds an MCP JSON tool contract and does not change CLI behavior; use stable structured result fields and no timestamps in primary results. **Pass.** |
| III. Test-First | Tests precede behavior changes | Add schema, renderer, path-boundary, existing/future outcome, and stdio tests before implementation; extend the warm benchmark. **Pass.** |
| IV. Integration Testing | Validate real invocation and error forms | Exercise the new tool over the existing stdio test client against temporary vault fixtures, including errors and exact destination validation. **Pass.** |
| V. Observability, Versioning & Simplicity | Minimal diagnostics and small stable surface | Add one tool with a strict schema; reuse current diagnostics and avoid noisy per-request payload logging. **Pass.** |
| VI. Interoperability & Cross-Technology Integration | Stable language-agnostic interface; no ecosystem lock-in | MCP JSON schema and structured JSON results remain language-neutral; no hard dependency is added. **Pass.** |
| VII. Reproducible Development Environment | Declare new tools/dependencies | No dependency or toolchain additions are planned. **Pass.** |
| VIII. CI & Release Gating | Required checks and performance tests gate changes/releases | Use existing workspace CI and the MCP warm benchmark; required CI remains the merge gate. **Pass.** |
| IX. Native Documentation | Document every new/changed Rust symbol with Rustdoc | New request/result/renderer symbols must have Rustdoc comments in the same implementation changes. **Pass, gated in implementation.** |

**Pre-research result**: PASS. No unresolved product clarifications remain. No constitution exception is requested.

## Phase 0: Research Decisions

Decisions and evidence are recorded in [research.md](research.md). Key choices: use the existing MCP/server/resolver crates with no new dependencies; emit vault-relative paths in both formats per the clarified spec and official Obsidian behavior; validate existing targets by exact resolver result comparison; safely handle planned source paths by canonicalizing the existing path prefix; use only the resolver's currently supported attachment types.

## Phase 1: Design

- [Data model](data-model.md) defines request, destination, result, validation, and state rules.
- [MCP contract](contracts/mcp-link-generation.md) defines the additive tool schema, response, rendering rules, and compatibility guarantees.
- [Quickstart](quickstart.md) defines contract/integration validation and the warm benchmark invocation.

### Design Decisions

1. Register `generate_obsidian_link` alongside `resolve_obsidian_link`; keep resolver behavior and schema unchanged.
2. Use a separate generation request and result. `intent` is required (`existing` or `future`); `format` defaults to `wikilink`; `output` defaults to `link`.
3. Accept an exact vault-relative target path. Preserve the note extension for Markdown output; Wikilinks may omit `.md`. Attachments retain the full filename.
4. For existing destinations, parse/resolve the candidate with the existing library and compare canonical target path and requested subtarget before returning `validated`. For attachments, verify the exact indexed file and no subtarget.
5. For future destinations, do not run destination validation as success; return `unverified` only after path-boundary and syntax checks pass.
6. Reuse the startup allow-list and cached vault. Existing paths are canonicalized before containment checks. A planned source path is accepted only when its canonical existing prefix is inside the selected root and its unresolved tail contains no traversal; reject paths whose symlinks escape the root.
7. Build the candidate using a focused renderer with explicit escaping/percent encoding. Round-trip it through the shared parser; reject unsupported names rather than returning misleading text.
8. Support same-file shorthand only for a heading or block in the contextual note; support embed output only for note/attachment targets and preserve heading/block selectors only for notes.
9. Support the optional `label` request property as caller-supplied visible text for both formats. Render Wikilink aliases as `[[target|label]]` (including embeds); when omitted, Wikilinks retain their normal target-derived display while Markdown labels are derived from the target and requested subtarget.

### Intended Source and Test Structure

```text
crates/obsidian-link-resolver-mcp/
├── src/
│   ├── mcp.rs                         # Register and dispatch the new tool
│   └── link_generation.rs             # Request handling, path checks, rendering, validation
├── tests/
│   ├── contract/
│   │   └── mcp_link_generation.rs      # Tool schema, defaults, invalid inputs, result contract
│   ├── integration/
│   │   └── mcp_link_generation.rs      # End-to-end cases using the stdio client
│   └── support/
│       └── mcp_client.rs              # Existing MCP subprocess harness
└── benches/
    └── mcp_warm.rs                    # Add representative generation request

crates/obsidian-link-resolver/src/
├── link.rs                            # Existing parser; no behavior changes planned
├── resolve.rs                         # Existing exact-resolution authority
└── vault.rs                           # Existing attachment index and validation rules
```

Feature artifacts live beside this plan: `research.md`, `data-model.md`, `contracts/mcp-link-generation.md`, and `quickstart.md`. `tasks.md` remains the output of the separate task-generation phase.

## Post-Design Constitution Check

| Principle | Result |
|---|---|
| I. Performance & Agent Efficiency | **Pass**: cache reuse and benchmark coverage are explicit; warm 100 ms p50 gate remains. |
| II. CLI Interface & Consistent Output | **Pass**: CLI is unaffected; new MCP result has stable structured fields. |
| III. Test-First | **Pass**: contract and stdio scenarios are identified before implementation. |
| IV. Integration Testing | **Pass**: end-to-end calls cover supported outputs and failure outcomes. |
| V. Observability, Versioning & Simplicity | **Pass**: one additive tool; use existing redacted diagnostics. |
| VI. Interoperability & Cross-Technology Integration | **Pass**: tool schema and JSON result remain language-neutral. |
| VII. Reproducible Development Environment | **Pass**: no new dependency/tool required. |
| VIII. CI & Release Gating | **Pass**: existing required CI plus warm benchmark remains authoritative. |
| IX. Native Documentation | **Pass with implementation gate**: all added Rust symbols must include Rustdoc documentation. |

No violations or unjustified complexity. The implementation must not change the existing resolver contract or write to vault files.

## Complexity Tracking

No constitution violations require justification.
