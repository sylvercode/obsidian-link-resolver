---
description: "Task list for MCP Link Guidance"
---

# Tasks: MCP Link Guidance

**Input**: Design documents from `specs/003-mcp-link-guidance/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, and `contracts/mcp-link-guidance.md`

**Tests**: Metadata contract tests are required by the implementation plan and project Test-First principle.

## Phase 1: Setup

**Purpose**: Project initialization and basic structure.

No setup tasks are required; the Rust workspace and MCP test harness already exist.

---

## Phase 2: Foundational

**Purpose**: Shared prerequisites that block user-story work.

No foundational tasks are required; the existing resolver schema, startup access boundary, and MCP transport remain unchanged.

---

## Phase 3: User Story 1 - Pass source links to the resolver (Priority: P1) - MVP

**Goal**: Guide agents to recognize the four specified link forms and send each unchanged with its source Markdown file as context.

**Independent Test**: Discover `resolve_obsidian_link`, verify its description names `[[Note]]`, `[[Note#Heading]]`, `[text](Note.md)`, and `[text](Note.md#Heading)`, requires unchanged source text and source-file context, and states the startup-configured directory boundary. Submit each unchanged form with a configured source file and confirm it reaches the existing resolver.

### Tests for User Story 1

- [ ] T001 [US1] Extend metadata and stdio contract assertions in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_tool_metadata.rs` to check all four exact examples, verbatim-link and context guidance, startup directory restriction, absence of discovery/search claims, and unchanged submission of each example to the existing tool.

### Implementation for User Story 1

- [ ] T002 [US1] Update the `resolve_obsidian_link` description in `crates/obsidian-link-resolver-mcp/src/mcp.rs` with the four required examples, exact-source and source-file-context instructions, Markdown-folder/no-Obsidian clarification, and startup-configured access boundary without changing the tool schema or resolver behavior.

**Checkpoint**: The discovered tool description supports the four requested forms, and unchanged inputs continue through the existing resolver.

---

## Phase 4: User Story 2 - Understand the MCP's purpose at initialization (Priority: P2)

**Goal**: Keep initialization instructions to a concise statement of the MCP's resolver purpose, leaving detailed syntax guidance on the tool.

**Independent Test**: Initialize the MCP server and verify its instructions briefly state the resolver purpose without repeating the link-form list or conflicting with the tool description.

### Tests for User Story 2

- [ ] T003 [US2] Extend the initialization metadata assertion in `crates/obsidian-link-resolver-mcp/tests/contract/mcp_tool_metadata.rs` to require a concise resolver-purpose statement and ensure the detailed link examples are not duplicated there.

### Implementation for User Story 2

- [ ] T004 [US2] Shorten the MCP initialization instructions in `crates/obsidian-link-resolver-mcp/src/mcp.rs` to a brief purpose statement and keep detailed link guidance exclusively in the resolver tool description.

**Checkpoint**: Initialization orients the client concisely while the discovered tool retains the detailed calling guidance.

---

## Phase 5: Polish & Cross-Cutting Validation

**Purpose**: Verify the feature against the documented acceptance flow and unchanged resolver behavior.

- [ ] T005 Run the package validation command documented in `specs/003-mcp-link-guidance/quickstart.md` (`cargo test -p obsidian-link-resolver -p obsidian-link-resolver-mcp`) after both stories are complete.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No tasks; existing workspace is ready.
- **Foundational (Phase 2)**: No tasks; existing infrastructure is retained.
- **User Stories (Phases 3-4)**: Both rely on the existing MCP server and contract test harness. Complete US1 before US2 because both extend the same test and metadata source files; this is an execution-order constraint, not a runtime dependency.
- **Polish (Phase 5)**: Depends on both user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Can start immediately; no other story dependency.
- **US2 (P2)**: No behavior dependency on US1, but follows US1 to avoid concurrent edits to `mcp_tool_metadata.rs` and `mcp.rs`.

### Parallel Opportunities

- No story tasks are marked parallelizable because each story's test and implementation modify the same files, and both stories share those files.
- The resolver and MCP package suites are run together by T005 as a single documented validation command.

## Implementation Strategy

### MVP First

1. Complete T001, then T002 for User Story 1.
2. Validate the P1 independent test: four unchanged source forms are discoverable and submitted with their source-file context.
3. Stop at the P1 checkpoint if only the MVP is needed.

### Incremental Delivery

1. Deliver US1 as the MVP without changing resolver behavior or the input schema.
2. Complete T003, then T004 to deliver concise initialization guidance.
3. Run T005 to validate both Rust packages against the quickstart.
