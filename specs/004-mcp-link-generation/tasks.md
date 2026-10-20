---
description: "Task list for MCP link generation"
---

# Tasks: MCP Link Generation

**Input**: Design documents from `specs/004-mcp-link-generation/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/mcp-link-generation.md`, `quickstart.md`

**Tests**: Required before implementation by Constitution Principles III and IV.

**Organization**: Tasks are grouped by user story. Shared MCP schema and vault-boundary prerequisites are foundational.

## Phase 1: Setup

**Purpose**: Project initialization

The existing Rust workspace, MCP crate, test harness, and dependencies already satisfy this feature's setup needs. No setup tasks are required.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish the strict tool contract and common vault boundary before story behavior.

- [x] T001 [P] Add tool discovery, strict input schema, required fields, and invalid-argument contract tests in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_link_generation.rs` and register the module in `crates/obsidian-link-resolver-mcp/tests/contract.rs`.
- [x] T002 [P] Add source/target containment integration tests, including planned source paths and symlink escape rejection, in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_link_generation.rs` and register the module in `crates/obsidian-link-resolver-mcp/tests/integration.rs`.
- [x] T003 Implement documented generation request/result types and configured-root selection, including safe canonical-prefix validation for planned source paths, in `crates/obsidian-link-resolver-mcp/src/link_generation.rs`.
- [x] T004 Register the generation module and `generate_obsidian_link` tool adapter using the existing allowed roots and shared `VaultCache` in `crates/obsidian-link-resolver-mcp/src/mcp.rs`.

**Checkpoint**: The additive tool schema is discoverable, strict, and cannot widen the startup-configured vault boundary.

---

## Phase 3: User Story 1 - Generate a link to an existing destination (Priority: P1) - MVP

**Goal**: Return a candidate only as validated when it resolves to the exact existing note and requested heading or block.

**Independent Test**: In the stdio MCP test vault, request whole-note, heading, and block links in Wikilink and Markdown forms; verify exact destination matches validate and missing, ambiguous, or mismatched targets are rejected.

### Tests for User Story 1

- [x] T005 [US1] Add stdio integration cases for existing whole-note, heading, and block destinations, both link formats, and missing/ambiguous/mismatched destinations in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_link_generation.rs`.

### Implementation for User Story 1

- [x] T006 [US1] Implement representable note/subtarget rendering, parser round-tripping, and exact canonical path plus subtarget comparison against the cached resolver result in `crates/obsidian-link-resolver-mcp/src/link_generation.rs`.
- [x] T007 [US1] Return structured validated or rejected generation results without changing `resolve_obsidian_link` behavior in `crates/obsidian-link-resolver-mcp/src/mcp.rs`.

**Checkpoint**: Existing note and subtarget requests report validated only on an exact resolver match.

---

## Phase 4: User Story 2 - Prepare a link to future content (Priority: P2)

**Goal**: Return safely representable future links as explicitly unverified, without implying destination existence or writing vault files.

**Independent Test**: Request future note, attachment, and subtarget links, including a planned source file; verify syntax/path outcomes are separate from destination validation and the vault remains unchanged.

### Tests for User Story 2

- [x] T008 [US2] Add stdio cases for future notes, attachments, and subtargets plus explicit existing requests for missing targets in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_link_generation.rs`; assert unverified/rejected outcomes and no vault mutations.

### Implementation for User Story 2

- [x] T009 [US2] Implement explicit `future` intent handling so allowed, representable candidates return `unverified`, existing-target failures never fall back to future, and generation performs no writes in `crates/obsidian-link-resolver-mcp/src/link_generation.rs`.

**Checkpoint**: Future candidates are clearly distinguished from destination-validated links and no files are created or modified.

---

## Phase 5: User Story 3 - Choose link format and subtarget (Priority: P3)

**Goal**: Support per-request format/output selection, display labels in both formats, attachments, same-file shorthand, and embeds.

**Independent Test**: Exercise omitted and explicit formats, custom and derived labels, note/attachment links and embeds, same-file subtargets, and invalid selector combinations; verify emitted syntax and destination status.

### Tests for User Story 3

- [x] T010 [US3] Add contract assertions for Wikilink/link defaults, format/output enums, Markdown label, and mutually exclusive subtarget inputs in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_link_generation.rs`.
- [x] T011 [US3] Add stdio cases for both formats, attachment targets, embeds, same-file shorthand, custom/derived Markdown labels, and simultaneous heading/block rejection in `crates/obsidian-link-resolver-mcp/tests/integration/mcp_link_generation.rs`.

### Implementation for User Story 3

- [x] T012 [US3] Implement format and output selection, vault-relative destinations, attachment handling, label derivation, same-file shorthand, and embed rendering in `crates/obsidian-link-resolver-mcp/src/link_generation.rs`.

**Checkpoint**: All requested link forms are emitted faithfully, or rejected without claiming validation.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Verify performance and the complete MCP feature contract.

- [x] T013 Add a representative `generate_obsidian_link` warm-cache sample to `crates/obsidian-link-resolver-mcp/benches/mcp_warm.rs` and retain the existing 100 ms p50 gate.
- [x] T014 Run the MCP crate tests and warm benchmark from `specs/004-mcp-link-generation/quickstart.md`; confirm the existing resolver contract remains unchanged in `crates/obsidian-link-resolver-mcp/src/mcp.rs`.

---

## Phase 7: Convergence

- [x] T015 Make caller-supplied display labels work for Wikilink as well as Markdown output, render Wikilink aliases, add contract and integration coverage for both formats, and update the plan/contract/data-model wording per FR-009 and US3/AC6 (partial)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No tasks; the workspace and dependencies are already in place.
- **Foundational (Phase 2)**: Must complete before story implementation. T001 and T002 can be authored in parallel; T003 and T004 follow the contract tests.
- **User Stories (Phases 3-5)**: Proceed in priority order because each later story extends the shared request, rendering, and result path established by US1.
- **Polish (Phase 6)**: Run after the desired stories are complete.

### User Story Dependencies

- **US1 (P1)**: Depends on Phase 2; delivers the MVP existing-note validation path.
- **US2 (P2)**: Depends on the shared renderer/result path from US1; adds explicit future intent without weakening existing-target validation.
- **US3 (P3)**: Depends on the shared renderer from US1 and future/result distinctions from US2; adds the remaining output forms and selection behavior.

### Parallel Opportunities

- T001 and T002 touch separate contract/integration test modules and can be authored in parallel.
- User-story implementation tasks are intentionally sequential because they extend the same `link_generation.rs` module and MCP contract.
- The warm benchmark task T013 touches only `benches/mcp_warm.rs` and can be prepared independently once the generation request shape is stable.

## Parallel Example: Foundational Contract Coverage

```text
Task: T001 Add strict tool schema and argument contract tests in tests/contract/mcp_link_generation.rs
Task: T002 Add configured-root and planned-source integration tests in tests/integration/mcp_link_generation.rs
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 2 shared schema and vault-boundary prerequisites.
2. Complete Phase 3 existing note and subtarget generation.
3. Validate US1 independently with `cargo test -p obsidian-link-resolver-mcp`.

### Incremental Delivery

1. Deliver US1 for exact existing-note links.
2. Add US2 so future links remain explicitly unverified and read-only.
3. Add US3 for format, attachment, same-file, and embed options.
4. Complete the warm benchmark and quickstart validation.

## Notes

- Every task has a sequential ID and an explicit implementation/test path.
- Tests precede the behavior they verify, consistent with the project constitution and plan.
- No new dependency or resolver-library behavior change is planned.
