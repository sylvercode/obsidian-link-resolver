# Feature Specification: Obsidian Link Resolver MCP Wrapper

**Feature Branch**: `002-mcp-wrapper`

**Created**: 2026-10-01

**Status**: Draft

**Input**: User description: "Prepare new spec to add a mcp that wrap the current tool"

## Clarifications

### Session 2026-10-04

- Q: Which host platforms should the MCP binary release promise support? → A: Current release targets: Linux x86_64/aarch64, macOS x86_64/arm64, and Windows x86_64 (Option A).
- Q: After a vault file changes, when must a later MCP resolution reflect that change? → A: Before the next resolution when detected; otherwise within 60 seconds (Option A).

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

An AI client needs to discover what the tool does, what inputs it requires, and how to interpret the responses before calling it. The MCP wrapper exposes well-defined tool metadata, input schema, and result schema so clients can discover the capability reliably and call it with confidence. The tool description must explain that the resolver can return either a simple target line or a richer structured emplacement: the simple line is sufficient for a single location, while the structured emplacement is preferred when an agent wants to read a precise section or shard of a note.

**Why this priority**: Tool discovery and schema clarity are essential for agent interoperability. Without clear metadata, a client cannot reliably call the capability in a standard MCP ecosystem, and without guidance on when to prefer emplacement, agents may choose the lower-fidelity output even when a section-based read is more efficient.

**Independent Test**: Can be tested by querying the MCP server's tool list and input schema, then verifying that the arguments and result fields are documented and match the underlying resolver semantics, including the explanation of when to use a simple target line versus the richer emplacement data.

**Acceptance Scenarios**:

1. **Given** an MCP client connects to the server, **When** it lists available tools, **Then** it sees a dedicated resolver tool with a clear purpose and description.
2. **Given** the tool is inspected for its schema, **When** the client reads its arguments, **Then** it can determine the required link, context path, and optional settings without guesswork.
3. **Given** the tool description is read, **When** the client considers whether to request the simple line or the emplacement detail, **Then** it sees that emplacement is the better choice for targeted reads of a section or sharded context, while the simple line is enough for a single point target.
4. **Given** the tool returns a result, **When** the client inspects the result structure, **Then** it can distinguish success, unresolved, ambiguous, and error outcomes unambiguously.

---

### User Story 4 - Reuse a cached vault index across repeated MCP calls (Priority: P2)

An AI client or automation workflow often resolves many links in the same vault within a short time. The MCP wrapper reuses a cached vault index for that vault so repeated link resolution avoids re-enumerating the entire vault on each call, keeps the result faster for common agent workflows, and preserves the same semantics as the underlying resolver.

**Why this priority**: Performance matters for agent-driven reads because a vault may contain many notes and the same vault is frequently queried repeatedly within a single session. Avoiding repeated full directory walks improves responsiveness while keeping the resolution rules unchanged.

**Independent Test**: Can be tested by making repeated resolution calls against the same vault in the same MCP session and asserting that the vault index is reused rather than rebuilt; the behavior should be equivalent to the underlying resolver without altering the result contract.

**Acceptance Scenarios**:

1. **Given** an MCP session resolves multiple links in the same vault, **When** the calls repeat against that vault, **Then** the wrapper reuses the cached vault state instead of re-enumerating the whole vault on every request.
2. **Given** a relevant filesystem change occurs in a cached vault, **When** the change is detected, **Then** the cache is refreshed before the next resolution; if changes are not detected sooner, they are reflected within 60 seconds.
3. **Given** a link is resolved against a cached vault, **When** the result is returned, **Then** it matches the current resolver semantics and does not change the observed target behavior.

---

### User Story 5 - Install the MCP server through a standard client registration flow (Priority: P2)

A user wants to enable the resolver in an AI client without manual source builds or unfamiliar setup steps. The delivery mechanism is a locally installed MCP server that a client can register through standard configuration, with clear installation guidance in the user documentation and a straightforward path for connecting the server to the user's vault.

**Why this priority**: Without a clear installation story, the capability is difficult to adopt even if the underlying resolver works correctly. Users need a predictable, low-friction setup path that fits the common MCP client model.

**Independent Test**: Can be tested by reviewing the installation documentation and verifying that it describes the supported local server delivery model, configuration path, and vault setup steps in a way a non-expert user can follow.

**Acceptance Scenarios**:

1. **Given** a user wants to install the resolver in a local MCP client, **When** they follow the documentation, **Then** they can register the server in the standard client configuration flow without custom integration steps.
2. **Given** the user has a vault path available, **When** they configure the server, **Then** the documentation explains how to point the server to the target vault and how the client will launch the local server process.
3. **Given** a user is evaluating the feature, **When** they read the docs, **Then** they understand the recommended delivery model and the rationale for using a local server with the client’s standard MCP registration configuration.

---

### User Story 6 - Inspect MCP behavior when troubleshooting (Priority: P3)

A user troubleshooting the MCP server needs operational details about its lifecycle, request processing, and failures. The server offers optional diagnostic logging; cache activity is one useful example of the events it reports. Logging does not change normal tool responses.

**Why this priority**: Opt-in diagnostics help users investigate server behavior and errors without adding noise to routine MCP sessions.

**Independent Test**: Can be tested by running the server with diagnostics disabled and enabled, then verifying that enabled diagnostics report operational events and failures, include cache reuse and refresh outcomes, and do not change MCP responses.

**Acceptance Scenarios**:

1. **Given** diagnostic logging is disabled, **When** the server starts and handles requests, **Then** it emits no optional diagnostic logs.
2. **Given** diagnostic logging is enabled, **When** the server starts, handles or completes a request, or encounters a failure, **Then** diagnostics report the relevant lifecycle, request outcome, or failure.
3. **Given** diagnostic logging is enabled, **When** a request reuses or refreshes a vault index, **Then** diagnostics identify the cache outcome.
4. **Given** diagnostic logging is enabled, **When** operational events are logged, **Then** logs do not include link text or note contents and do not appear in or alter the MCP protocol response.

---

### Edge Cases

- When the link or context is invalid, the MCP wrapper returns a clear error outcome rather than crashing or producing incomplete data.
- When the underlying resolver reports ambiguous note matches, the wrapper preserves the candidate list and reason in the MCP response.
- When the link references a heading or block that does not exist, the wrapper reports the sub-target-not-found result defined by the resolver.
- When a non-markdown attachment is requested, the wrapper preserves the attachment semantics and does not invent heading or emplacement information.
- When the underlying resolver requires a vault root or vault detection, the MCP wrapper exposes the same behavior to callers through the tool contract.
- When identical inputs are submitted twice, the wrapper produces the same structured result; no non-deterministic fields are included in the primary response.
- When a vault is queried repeatedly, its cached index is reused; detected filesystem changes are reflected before the next resolution, and changes not detected sooner are reflected within 60 seconds.
- When diagnostic logging is enabled, operational events and failures, including cache activity, are reported separately from tool results; when disabled, optional diagnostic logs are not emitted.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide an MCP interface that exposes the current Obsidian Link Resolver as a standard tool for AI clients and automation workflows.
- **FR-002**: The MCP tool MUST accept the same minimal inputs required by the resolver: the Obsidian link string and the path of the context file that contains the link.
- **FR-003**: The MCP tool MUST support the same optional configuration the resolver already supports, including explicit vault root selection and structured emplacement requests when applicable.
- **FR-004**: The MCP wrapper MUST return the same semantic outcome categories as the resolver: resolved, unresolved, sub-target-not-found, ambiguous, and error.
- **FR-005**: The MCP tool MUST preserve the current resolver behavior for note-name matching, path-qualified references, same-file links, heading paths, block ids, and attachment handling.
- **FR-006**: The MCP wrapper MUST expose a deterministic machine-readable result schema so clients can parse tool output without scraping human-readable text.
- **FR-007**: The MCP wrapper MUST provide tool metadata that clearly describes what it resolves, which inputs it expects, and how the result should be interpreted.
- **FR-008**: The tool description MUST explain the choice between a simple target line and a richer structured emplacement: a simple target line is suitable for a single point target, while the structured emplacement is preferable when an agent wants to read the precise section or shard of a note that contains the target.
- **FR-009**: The MCP wrapper MUST allow callers to request structured emplacement information when available, while still respecting the underlying resolver's semantics for note and attachment targets.
- **FR-010**: The wrapper MUST not silently change or reinterpret the current resolver's decisions; it must act as a faithful adapter over the existing capability.
- **FR-011**: The MCP server MUST surface the current resolver's status and reason fields in a way that supports agent branching and retries without custom parsing.
- **FR-012**: The MCP interface MUST be discoverable by standard MCP clients and provide stable argument names and result fields across versions.
- **FR-013**: The system MUST support the same cross-platform and vault-relative path behavior as the underlying resolver so that the wrapper remains portable and consistent across environments.
- **FR-014**: The wrapper MUST be usable for both interactive agent workflows and programmatic automation without requiring the caller to spawn a shell or parse CLI output.
- **FR-015**: The MCP delivery model MUST use a locally installed server process registered through the client’s standard MCP configuration mechanism, rather than requiring a custom application integration or ad hoc installation path.
- **FR-016**: The project MUST publish or otherwise make available prebuilt MCP server binaries for Linux x86_64 and aarch64, macOS x86_64 and arm64, and Windows x86_64 so the installation flow is reliable for ordinary users and does not depend on local source builds.
- **FR-017**: The MCP wrapper MUST cache the vault index for repeated lookups in the same active session so multiple resolutions against the same vault do not require re-enumerating the entire vault on every request.
- **FR-018**: The cached vault state MUST remain logically consistent with the current resolver semantics. Relevant filesystem changes MUST be reflected before the next resolution when detected, and otherwise within 60 seconds.
- **FR-019**: The project MUST document the MCP installation and client-configuration flow in user-facing documentation so a user can install and register the server without consulting source code or unspecified setup steps.
- **FR-020**: The server MUST provide optional diagnostic logging, disabled by default, for server lifecycle events, request processing and outcomes, and operational failures. Diagnostics MUST include cache reuse, refresh, and refresh-failure events; MUST be written to stderr; MUST NOT alter MCP responses; and MUST NOT include link text or note contents.

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
- **SC-004**: The tool description explicitly tells clients when to prefer a simple target line versus a richer emplacement result, and that guidance aligns with the supported agent workflow for sharded section reads in 100% of the documented scenarios.
- **SC-005**: Identical inputs submitted through the MCP wrapper produce identical primary result records in 100% of cases, with no non-deterministic fields in the main response.
- **SC-006**: The MCP wrapper reuses a cached vault index for repeated lookups in the same active session and avoids unnecessary vault re-enumeration for repeated calls in 100% of supported scenarios. Detected filesystem changes are reflected before the next resolution, and changes not detected sooner are reflected within 60 seconds.
- **SC-007**: The MCP wrapper adds no functional ambiguity beyond the underlying resolver: callers can interpret the result using the same semantics as the current tool, with no hidden behavior changes.
- **SC-008**: The project publishes prebuilt MCP server binaries for Linux x86_64 and aarch64, macOS x86_64 and arm64, and Windows x86_64, and documents the installation and client registration flow in user-facing documentation so a user can install and configure it without source-level setup instructions.
- **SC-009**: The tool supports the main agent workflows needed for Obsidian link resolution, including same-file, heading, block, and attachment references, across the supported set defined by the underlying resolver.
- **SC-010**: With diagnostic logging enabled, users can identify server startup, request outcomes, operational failures, and cache reuse or refresh outcomes, while MCP protocol responses remain unchanged; with logging disabled, no optional diagnostic logs are emitted.

## Assumptions

- The MCP wrapper is a compatibility layer over the existing Obsidian Link Resolver rather than a second implementation of resolution logic.
- The current resolver's semantics and edge cases remain the authoritative contract for link interpretation.
- MCP clients can pass structured arguments and parse structured results via the standard protocol without custom text scraping.
- Agent workflows rely on deterministic result fields and stable outcome categories rather than human-readable prose alone.
- The wrapper's scope is limited to exposing the current capability through MCP; it does not expand the underlying link-resolution rules beyond the current tool's contract.
- Diagnostic logging is opt-in and limited to operational server state; request payloads, link text, and note contents are not needed in logs.
