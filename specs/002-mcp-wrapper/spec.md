# Feature Specification: Obsidian Link Resolver MCP Wrapper

**Feature Branch**: `002-mcp-wrapper`

**Created**: 2026-10-01

**Status**: Draft

**Input**: User description: "Prepare new spec to add a mcp that wrap the current tool"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Resolve links through a standard tool interface (Priority: P1)

An AI assistant or another tool client needs to resolve an Obsidian link without manually invoking a shell or parsing CLI output. The MCP wrapper exposes the existing resolver as a standard tool so the caller can pass the link text, the context file, and the relevant options and receive the same resolution outcome in a structured, machine-readable format.

**Why this priority**: This is the core user value. Agents already use standard tool protocols to work with external capabilities; wrapping the resolver in MCP lets them incorporate vault-aware link resolution into their normal workflows without custom integration code.

**Independent Test**: Can be fully tested by invoking the MCP tool with a known Obsidian link and context file, then asserting that the returned result matches the underlying resolver's expected target file and line.

**Acceptance Scenarios**:

1. **Given** a valid Obsidian wikilink such as `[[Project Plan#Milestones]]` and a context file in the vault, **When** the MCP tool is called, **Then** it returns the target file path and target location in a structured response.
2. **Given** a same-file link such as `[[#Overview]]`, **When** the MCP tool is called, **Then** it resolves the target relative to the context file and returns the correct path and line.
3. **Given** a broken or missing link target, **When** the MCP tool is called, **Then** it returns a non-success outcome with a clear reason rather than silently inventing a result.

---

### User Story 2 - Preserve the current resolver contract while adding MCP access (Priority: P2)

A caller expects the MCP wrapper to behave like the current tool: the same outcomes, the same path semantics, and the same source of truth for target resolution. The wrapper should act as a stable interface over the existing resolver and must not change how links are interpreted, named, or validated.

**Why this priority**: The wrapper is only valuable if it is trustworthy and consistent with the underlying tool. Preserve the semantics of the existing resolver so AI clients can rely on the same results that human users already get.

**Independent Test**: Can be tested by comparing the outcome of an equivalent direct CLI/library call and an MCP tool call for identical inputs and asserting that the target path, line range, and status match.

**Acceptance Scenarios**:

1. **Given** a link that resolves successfully in the current resolver, **When** the same link is submitted through the MCP tool, **Then** the returned result matches the resolver's result exactly.
2. **Given** an ambiguous note name, **When** the MCP wrapper resolves it, **Then** the returned result includes the candidate list, reason, and outcome defined by the underlying resolver.
3. **Given** a structured emplacement request, **When** the MCP tool is called, **Then** it exposes the same heading stack and section ranges as the underlying resolver without altering the semantics.

---

### User Story 3 - Offer discoverable, agent-friendly tool metadata (Priority: P3)

An AI client needs to discover what the tool does, what inputs it requires, and how to interpret the responses before calling it. The MCP wrapper exposes well-defined tool metadata, input schema, and result schema so clients can discover the capability reliably and call it with confidence.

**Why this priority**: Tool discovery and schema clarity are essential for agent interoperability. Without clear metadata, a client cannot reliably call the capability in a standard MCP ecosystem.

**Independent Test**: Can be tested by querying the MCP server's tool list and input schema, then verifying that the arguments and result fields are documented and match the underlying resolver semantics.

**Acceptance Scenarios**:

1. **Given** an MCP client connects to the server, **When** it lists available tools, **Then** it sees a dedicated resolver tool with a clear purpose and description.
2. **Given** the tool is inspected for its schema, **When** the client reads its arguments, **Then** it can determine the required link, context path, and optional settings without guesswork.
3. **Given** the tool returns a result, **When** the client inspects the result structure, **Then** it can distinguish success, unresolved, ambiguous, and error outcomes unambiguously.

---

### Edge Cases

- When the link or context is invalid, the MCP wrapper returns a clear error outcome rather than crashing or producing incomplete data.
- When the underlying resolver reports ambiguous note matches, the wrapper preserves the candidate list and reason in the MCP response.
- When the link references a heading or block that does not exist, the wrapper reports the sub-target-not-found result defined by the resolver.
- When a non-markdown attachment is requested, the wrapper preserves the attachment semantics and does not invent heading or emplacement information.
- When the underlying resolver requires a vault root or vault detection, the MCP wrapper exposes the same behavior to callers through the tool contract.
- When identical inputs are submitted twice, the wrapper produces the same structured result; no non-deterministic fields are included in the primary response.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide an MCP interface that exposes the current Obsidian Link Resolver as a standard tool for AI clients and automation workflows.
- **FR-002**: The MCP tool MUST accept the same minimal inputs required by the resolver: the Obsidian link string and the path of the context file that contains the link.
- **FR-003**: The MCP tool MUST support the same optional configuration the resolver already supports, including explicit vault root selection and structured emplacement requests when applicable.
- **FR-004**: The MCP wrapper MUST return the same semantic outcome categories as the resolver: resolved, unresolved, sub-target-not-found, ambiguous, and error.
- **FR-005**: The MCP tool MUST preserve the current resolver behavior for note-name matching, path-qualified references, same-file links, heading paths, block ids, and attachment handling.
- **FR-006**: The MCP wrapper MUST expose a deterministic machine-readable result schema so clients can parse tool output without scraping human-readable text.
- **FR-007**: The MCP wrapper MUST provide tool metadata that clearly describes what it resolves, which inputs it expects, and how the result should be interpreted.
- **FR-008**: The MCP wrapper MUST allow callers to request structured emplacement information when available, while still respecting the underlying resolver's semantics for note and attachment targets.
- **FR-009**: The wrapper MUST not silently change or reinterpret the current resolver's decisions; it must act as a faithful adapter over the existing capability.
- **FR-010**: The MCP server MUST surface the current resolver's status and reason fields in a way that supports agent branching and retries without custom parsing.
- **FR-011**: The MCP interface MUST be discoverable by standard MCP clients and provide stable argument names and result fields across versions.
- **FR-012**: The system MUST support the same cross-platform and vault-relative path behavior as the underlying resolver so that the wrapper remains portable and consistent across environments.
- **FR-013**: The wrapper MUST be usable for both interactive agent workflows and programmatic automation without requiring the caller to spawn a shell or parse CLI output.

### Key Entities *(include if feature involves data)*

- **Resolver Capability**: The existing Obsidian Link Resolver behavior that determines which note, heading, or block a link targets.
- **MCP Tool Request**: The structured input sent to the MCP server, including the link text, context file path, and optional settings.
- **MCP Tool Result**: The machine-readable output returned to the client, including outcome, reason, target path, and optional emplacement details.
- **Context File**: The note containing the link that is used to resolve same-file and vault-local references.
- **Vault**: The Obsidian vault used to resolve names and targets; the wrapper preserves the same vault detection and matching logic as the current resolver.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An MCP client can resolve a valid Obsidian link through the wrapper and receive the same target path and location as the underlying resolver in 100% of a defined test corpus.
- **SC-002**: The MCP wrapper preserves all five resolver outcomes — resolved, unresolved, sub-target-not-found, ambiguous, and error — in 100% of supported calls.
- **SC-003**: An agent can discover the resolver tool through standard MCP tooling and invoke it without custom text parsing or shell integration in 100% of tested client configurations.
- **SC-004**: Identical inputs submitted through the MCP wrapper produce identical primary result records in 100% of cases, with no non-deterministic fields in the main response.
- **SC-005**: The MCP wrapper adds no functional ambiguity beyond the underlying resolver: callers can interpret the result using the same semantics as the current tool, with no hidden behavior changes.
- **SC-006**: The tool supports the main agent workflows needed for Obsidian link resolution, including same-file, heading, block, and attachment references, across the supported set defined by the underlying resolver.

## Assumptions

- The MCP wrapper is a compatibility layer over the existing Obsidian Link Resolver rather than a second implementation of resolution logic.
- The current resolver's semantics and edge cases remain the authoritative contract for link interpretation.
- MCP clients can pass structured arguments and parse structured results via the standard protocol without custom text scraping.
- Agent workflows rely on deterministic result fields and stable outcome categories rather than human-readable prose alone.
- The wrapper's scope is limited to exposing the current capability through MCP; it does not expand the underlying link-resolution rules beyond the current tool's contract.
