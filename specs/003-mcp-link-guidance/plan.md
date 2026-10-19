# Implementation Plan: MCP Link Guidance

**Branch**: `003-obsidian-reference` (Spec Kit feature key: `003-mcp-link-guidance`) | **Date**: 2026-10-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-mcp-link-guidance/spec.md`

## Summary

Update the existing `resolve_obsidian_link` tool description to identify the five requested link examples, direct callers to pass source text unchanged with its Markdown file as context, and bound access to directories configured at startup. Keep initialization instructions to a short statement of purpose. No resolver behavior, tool schema, tools, or skills change.

## Technical Context

**Language/Version**: Rust, workspace edition

**Primary Dependencies**: Existing `rmcp` 3.5.1 MCP server integration; no dependency changes

**Storage**: N/A; the change is static MCP metadata

**Testing**: Rust `cargo test`; existing MCP contract and stdio integration tests

**Target Platform**: Existing MCP server platforms using local stdio transport

**Project Type**: Rust MCP server binary crate

**Performance Goals**: No runtime or performance change; metadata only

**Constraints**: Preserve resolver semantics and input schema; directories remain restricted to startup-configured roots; add no discovery/search capability, MCP tool, or skill

**Scale/Scope**: One tool description, one initialization instruction, and focused metadata/protocol contract coverage

## Constitution Check

*GATE: Evaluated before Phase 0 and re-checked after Phase 1.*

- **III. Test-First**: Pass. Add metadata assertions and stdio integration coverage before changing the strings; cover all five examples and both configured and unconfigured directories.
- **IV. Integration Testing**: Pass. The MCP contract and stdio integration suites validate discovery, real tool calls, and the startup-configured directory boundary; unchanged resolver tests cover parsing and resolution.
- **VI. Interoperability**: Pass. Keep the existing MCP tool name, schema, and result contract stable.
- **VII. Reproducible Environment**: Pass. No new toolchain or dependency is introduced.
- **VIII. CI & Release Gating**: Pass. Run the existing Rust test suites; required CI remains the merge gate.
- **IX. Native Documentation**: Pass. No Rust symbol or API is added or changed; only existing metadata strings and their contract tests change.
- Performance, CLI output, and storage principles: unaffected.

No constitution violations or unresolved technical clarifications were identified.

## Project Structure

### Documentation (this feature)

```text
specs/003-mcp-link-guidance/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── mcp-link-guidance.md
└── spec.md
```

### Source and Validation

```text
crates/obsidian-link-resolver-mcp/
├── src/mcp.rs                                 # Existing tool description and initialization instructions
└── tests/
    ├── contract/mcp_tool_metadata.rs          # Tool and initialization metadata assertions
    └── integration/mcp_stdio.rs               # End-to-end MCP initialization and tool invocation

crates/obsidian-link-resolver/
├── src/link.rs                                # Existing Wikilink and Markdown link parsing
└── tests/                                     # Existing parser/resolution coverage
```

**Structure Decision**: Keep implementation and contract tests in the existing MCP binary crate. The resolver crate already parses and resolves the documented link forms; no resolver source changes are planned. Feature design artifacts live alongside this spec.

## Complexity Tracking

No constitution violations require justification.

## Post-Design Constitution Check

The design remains metadata-only, preserves the existing MCP schema and resolver authority, adds no dependencies or runtime state, and validates the public metadata through contract/integration tests. All applicable gates pass; no exception is requested.
