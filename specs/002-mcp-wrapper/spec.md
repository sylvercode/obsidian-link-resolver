# Feature Specification: Obsidian Link Resolver MCP Wrapper

**Feature Branch**: `002-mcp-wrapper`

**Created**: 2026-10-01

**Status**: Draft

**Input**: User description: "Prepare new spec to add a mcp that wrap the current tool"

## Clarifications

### Session 2026-10-04

- Q: Which host platforms should the MCP binary release promise support? → A: Current release targets are `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-gnu`.
- Q: After a vault file changes, when must a later MCP resolution reflect that change? → A: Before the next resolution when detected; otherwise within 60 seconds (Option A).

### Session 2026-10-05

- Q: Which version baseline should installation guidance and compatibility tests use for GitHub Copilot in VS Code and Claude Desktop? → A: Use each client's latest stable release available when validation is performed. Record each exact version, validation date, operating system, and result; a later client release does not alter the recorded result unless validation is repeated.
- Q: How should users choose the MCP installer component while keeping the existing CLI-only install as the default? → A: Provide separate MCP installer commands alongside the existing CLI installer (Option B).
- Q: How should the MCP wrapper distinguish malformed tool arguments from resolver failures? → A: Missing required arguments or arguments with invalid types are MCP tool/protocol errors and do not produce a resolver result. Once arguments satisfy the MCP input schema, failures reported by the resolver—including invalid link, context, or vault values when the resolver classifies them as errors—are returned as normal tool results with the resolver's `error` outcome and reason.

### Session 2026-10-12

- Q: How should releases version and publish the library, CLI binary, and MCP server when they live in the same repository? → A: Split the repository into three separate crates in a Cargo workspace (library, CLI, and MCP), sharing a synchronized semantic version and released under a single `vX.Y.Z` tag with distinct component-specific assets.

### Session 2026-10-17

- Q: Where should the process-scoped vault cache live after the workspace split? → A: Implement it in the core resolver crate behind an opt-in `vault-cache` feature, enabled by the MCP crate. Cache lifecycle remains scoped to the MCP server process, and default library consumers do not enable the cache dependencies.

### Analysis refinements 2026-10-05

- Deterministic results are required for identical inputs when the vault contents are unchanged; filesystem changes may legitimately change later results.
- Diagnostic redaction excludes request text, paths, note contents, serialized arguments, and unfiltered error messages.
- The stable MCP contract and its major-version compatibility policy must be verifiable against published versions and migration notes.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Resolve links through a standard tool interface (Priority: P1)

An AI assistant or another tool client needs to resolve an Obsidian link without manually invoking a shell or parsing CLI output. The MCP wrapper exposes the existing resolver as a standard tool so the caller can pass the link text, the context file, and the relevant options and receive the same resolution outcome in a structured, machine-readable format.

**Why this priority**: This is the core user value. Agents already use standard tool protocols to work with external capabilities; wrapping the resolver in MCP lets them incorporate vault-aware link resolution into their normal workflows without custom integration code.

**Independent Test**: Can be fully tested by invoking the MCP tool with a known Obsidian link and context file, then asserting that the returned result matches the underlying resolver's expected target file and line.

**Acceptance Scenarios**:

1. **Given** a valid Obsidian wikilink such as `[[Project Plan#Milestones]]` and a context file in the vault, **When** the MCP tool is called, **Then** it returns the target file path and target location in a structured response.
2. **Given** a same-file link such as `[[#Overview]]`, **When** the MCP tool is called, **Then** it resolves the target relative to the context file and returns the correct path and line.
3. **Given** a broken or missing link target, **When** the MCP tool is called, **Then** it returns a non-success outcome with a clear reason rather than silently inventing a result.
4. **Given** a required argument is missing or any argument has a value of the wrong schema type, **When** the MCP tool is called, **Then** it returns an MCP tool/protocol error without a resolver result.
5. **Given** all arguments satisfy the MCP input schema but the resolver rejects a link, context, or vault value as invalid, **When** the MCP tool is called, **Then** it returns the resolver's normal result with `error` status and reason.

---

### User Story 2 - Preserve the current resolver contract while adding MCP access (Priority: P2)

A caller expects the MCP wrapper to behave like the current tool: the same outcomes, the same path semantics, and the same source of truth for target resolution. The wrapper should act as a stable interface over the existing resolver and must not change how links are interpreted, named, or validated.

**Why this priority**: The wrapper is only valuable if it is trustworthy and consistent with the underlying tool. Preserve the semantics of the existing resolver so AI clients can rely on the same results that human users already get.

**Independent Test**: Can be tested by comparing the outcome of an equivalent direct CLI/library call and an MCP tool call for identical inputs and asserting that the target path, line range, and status match.

**Acceptance Scenarios**:

1. **Given** a link that resolves successfully in the current resolver, **When** the same link is submitted through the MCP tool, **Then** the returned result matches the resolver's result exactly.
2. **Given** an ambiguous note name, **When** the MCP wrapper resolves it, **Then** the returned result includes the candidate list, reason, and outcome defined by the underlying resolver.
3. **Given** a structured emplacement request, **When** the MCP tool is called, **Then** it exposes the same heading stack and section ranges as the underlying resolver without altering the semantics.
4. **Given** a client uses a published MCP contract, **When** a later contract version is released, **Then** compatibility is validated according to FR-012 and SC-012.
5. **Given** a path-qualified link or a vault-relative context path, **When** the MCP tool resolves it on a supported platform, **Then** it matches the underlying resolver's target and path interpretation.

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

**Why this priority**: Performance matters for agent-driven reads because a vault may contain many notes and the same vault is frequently queried repeatedly during one server process. Avoiding repeated full directory walks improves responsiveness while keeping the resolution rules unchanged.

**Independent Test**: Can be tested by making repeated resolution calls against the same vault during one MCP server process and asserting that the vault index is reused rather than rebuilt; the behavior should be equivalent to the underlying resolver without altering the result contract.

**Acceptance Scenarios**:

1. **Given** one MCP server process resolves multiple links in the same vault, **When** the calls repeat against that vault, **Then** the wrapper reuses the cached vault state instead of re-enumerating the whole vault on every request.
2. **Given** a relevant filesystem change occurs in a cached vault, **When** the change is detected, **Then** the cache is refreshed before the next resolution; if changes are not detected sooner, they are reflected within 60 seconds.
3. **Given** a link is resolved against a cached vault, **When** the result is returned, **Then** it matches the current resolver semantics and does not change the observed target behavior.

---

### User Story 5 - Install the MCP server in supported clients (Priority: P2)

A user wants to enable the resolver in GitHub Copilot in VS Code or Claude Desktop without manual source builds or unfamiliar setup steps. The delivery mechanism is a locally installed MCP server that the supported client can register through its standard local-stdio configuration, with clear installation guidance and a straightforward path for connecting the server to the user's vault.

**Why this priority**: Without a clear installation story for the supported clients, the capability is difficult to adopt even if the underlying resolver works correctly. Users need a predictable, low-friction setup path that fits each client's standard MCP registration flow.

**Independent Test**: Can be tested by reviewing the installation documentation and verifying that it describes the supported local server delivery model, each client's configuration path, and vault setup steps in a way a non-expert user can follow.

**Acceptance Scenarios**:

1. **Given** a user wants to install the resolver in either supported client, **When** they follow that client's documented steps, **Then** they can register the server in its standard local-stdio configuration flow without custom integration steps.
2. **Given** the user has a vault path available, **When** they configure the server, **Then** the documentation explains how to point the server to the target vault and how the client will launch the local server process.
3. **Given** a user is evaluating the feature, **When** they read the docs for either supported client, **Then** they understand the recommended delivery model and the rationale for using a local server with that client's standard MCP registration configuration.

---

### User Story 6 - Inspect MCP behavior when troubleshooting (Priority: P3)

A user troubleshooting the MCP server needs operational details about its lifecycle, request processing, and failures. The server offers optional diagnostic logging; cache activity is one useful example of the events it reports. Logging does not change normal tool responses.

**Why this priority**: Opt-in diagnostics help users investigate server behavior and errors without adding noise to routine MCP sessions.

**Independent Test**: Can be tested by running the server with diagnostics disabled and enabled, then verifying that enabled diagnostics report operational events and failures, include cache reuse and refresh outcomes, and do not change MCP responses.

**Acceptance Scenarios**:

1. **Given** diagnostic logging is disabled, **When** the server starts and handles requests, **Then** it emits no optional diagnostic logs.
2. **Given** diagnostic logging is enabled, **When** the server starts, handles or completes a request, or encounters a failure, **Then** diagnostics report the relevant lifecycle, request outcome, or failure.
3. **Given** diagnostic logging is enabled, **When** a request reuses or refreshes a vault index, **Then** diagnostics identify the cache outcome.
4. **Given** diagnostic logging is enabled, **When** operational events are logged, **Then** logs exclude link text, context or vault paths, note contents, serialized request arguments, and unfiltered error messages, and do not appear in or alter the MCP protocol response.

---

### Edge Cases

- When a required argument is missing or any argument has the wrong schema type, the MCP wrapper returns an MCP tool/protocol error and does not invoke the resolver or fabricate a resolver result.
- When a request passes configured-root validation and the resolver rejects its link or vault contents, the wrapper returns the resolver's normal `error` outcome and reason. Requests with an unconfigured root or a context path that cannot be verified inside an allowed root are rejected as MCP tool errors before resolver invocation. Other resolver outcomes, such as `unresolved` or `sub-target-not-found`, remain normal results.
- When the underlying resolver reports ambiguous note matches, the wrapper preserves the candidate list and reason in the MCP response.
- When the link references a heading or block that does not exist, the wrapper reports the sub-target-not-found result defined by the resolver.
- When a non-markdown attachment is requested, the wrapper preserves the attachment semantics and does not invent heading or emplacement information.
- The MCP process requires one or more launch-time allowed vault roots. A tool request may select only one of those roots, and its context path must be inside the selected root; omitting `vault_root` selects the configured root containing the context.
- When identical inputs are submitted twice against unchanged vault contents, the wrapper produces the same structured result; no non-deterministic fields are included in the primary response. Results may differ after relevant vault changes are reflected.
- When a vault is queried repeatedly, its cached index is reused; detected filesystem changes are reflected before the next resolution, and changes not detected sooner are reflected within 60 seconds.
- When diagnostic logging is enabled, operational events and failures, including cache activity, are reported separately from tool results and exclude link text, context or vault paths, note contents, serialized request arguments, and unfiltered error messages; when disabled, optional diagnostic logs are not emitted.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide an MCP interface that exposes the current Obsidian Link Resolver as a standard tool for AI clients and automation workflows.
- **FR-002**: The MCP tool MUST accept the same minimal inputs required by the resolver: the Obsidian link string and the path of the context file that contains the link.
- **FR-003**: The MCP tool MUST support optional explicit vault-root selection and structured emplacement requests when applicable. The server MUST require one or more launch-time allowed vault roots, MUST reject request roots outside that allow-list, and MUST reject context paths outside the selected allowed root.
- **FR-004**: For tool calls whose arguments satisfy the MCP input schema, the MCP wrapper MUST preserve the resolver's semantic outcome categories: resolved, unresolved, sub-target-not-found, ambiguous, and error. Contract compatibility across published versions is governed by FR-012.
- **FR-005**: The MCP tool MUST preserve the current resolver behavior for note-name matching, path-qualified references, vault-relative paths, same-file links, heading paths, block ids, and attachment handling.
- **FR-006**: The MCP wrapper MUST expose a deterministic machine-readable result schema so clients can parse tool output without scraping human-readable text.
- **FR-007**: The MCP wrapper MUST provide tool metadata that clearly describes what it resolves, which inputs it expects, and how the result should be interpreted.
- **FR-008**: The tool description MUST explain the choice between a simple target line and a richer structured emplacement: a simple target line is suitable for a single point target, while the structured emplacement is preferable when an agent wants to read the precise section or shard of a note that contains the target.
- **FR-009**: The MCP wrapper MUST allow callers to request structured emplacement information when available, while still respecting the underlying resolver's semantics for note and attachment targets.
- **FR-010**: The wrapper MUST not silently change or reinterpret the current resolver's decisions; it must act as a faithful adapter over the existing capability.
- **FR-011**: The MCP server MUST surface the current resolver's status and reason fields in a way that supports agent branching and retries without custom parsing.
- **FR-012**: The MCP interface MUST be discoverable by standard MCP clients and provide a stable contract of argument names, requiredness, result field names, and field meanings. Within a major version, it MUST NOT remove or rename existing arguments or result fields, change whether an argument is required, or change the meaning of existing fields or outcome categories. Additive optional arguments and result fields are permitted. Any breaking contract change MUST be released under a new major version and accompanied by migration notes. This is the canonical requirement for published-contract compatibility.
- **FR-013**: The system MUST support the same cross-platform and vault-relative path behavior as the underlying resolver so that the wrapper remains portable and consistent across environments.
- **FR-014**: The wrapper MUST be usable for both interactive agent workflows and programmatic automation without requiring the caller to spawn a shell or parse CLI output.
- **FR-015**: The MCP delivery model MUST use a locally installed server process registered through the standard local-stdio MCP configuration mechanism of GitHub Copilot in VS Code or Claude Desktop, rather than requiring a custom application integration or ad hoc installation path.
- **FR-016**: The project MUST publish or otherwise make available prebuilt MCP server binaries for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-gnu` so the installation flow is reliable for ordinary users and does not depend on local source builds.
- **FR-017**: The MCP wrapper MUST cache each vault index by canonical root for the lifetime of the MCP server process so repeated resolutions against the same vault do not require re-enumerating the entire vault on every request.
- **FR-018**: The cached vault state MUST remain logically consistent with the current resolver semantics. Each resolution MUST use one cached index generation selected when that resolution starts. If the root is invalidated while a resolution is in flight, that resolution MAY complete using its selected generation; the root MUST remain dirty and be refreshed before the next resolution for that root. Rapid filesystem events for one root MAY be coalesced, but the next resolution MUST use the latest filesystem state available at refresh time. Relevant changes MUST otherwise be reflected within 60 seconds.
- **FR-019**: The project MUST document the MCP installation and client-configuration flow for GitHub Copilot in VS Code and Claude Desktop, including client-specific local-stdio registration examples and separate MCP installer commands alongside the existing CLI installer. Installation guidance MUST follow the existing README's latest-version install/update, pinned-version, and manual-fallback patterns; it MUST provide an MCP-specific install/update path without changing the existing CLI installer or its CLI-only default.
- **FR-020**: The server MUST provide optional diagnostic logging, disabled by default, for server lifecycle events, request processing and outcomes, and operational failures. Diagnostics MUST include cache reuse, refresh, and refresh-failure events; MUST be written to stderr; MUST NOT alter MCP responses; and MUST NOT include link text, context or vault paths, note contents, serialized request arguments, or unfiltered error messages. The normative diagnostic record schema and allow-lists are defined in the Operational Diagnostics section and `contracts/mcp.md`.
- **FR-021**: The MCP interface MUST reject calls with missing required arguments or argument values of the wrong schema type as MCP tool/protocol errors without producing a resolver result. For calls that satisfy the input schema, it MUST preserve the resolver's result—including an `error` outcome and reason when the resolver rejects a link, context, or vault value—without converting it into an MCP tool/protocol error.
- **FR-022**: The library, CLI binary, and MCP server MUST be organized as three separate crates in a Cargo workspace within one repository (`obsidian-link-resolver` library, `obsidian-link-resolver-cli`, and `obsidian-link-resolver-mcp`). All crates MUST share a synchronized semantic version and release under a single `vX.Y.Z` tag. Each release MUST publish distinct library/FFI, CLI, and MCP assets. The CLI and MCP server MUST have separate installer commands; the existing CLI installer MUST remain CLI-only, and the MCP installer MUST install only the MCP server without requiring a component-selection parameter. Library/FFI assets are published separately and are not selected by either executable installer.

### Key Entities *(include if feature involves data)*

- **Resolver Capability**: The existing Obsidian Link Resolver behavior that determines which note, heading, or block a link targets.
- **MCP Tool Request**: The structured input sent to the MCP server, including the link text, context file path, and optional settings.
- **MCP Tool Result**: The machine-readable output returned to the client, including outcome, reason, target path, and optional emplacement details.
- **Context File**: The note containing the link that is used to resolve same-file and vault-local references.
- **Vault**: The Obsidian vault used to resolve names and targets; the wrapper preserves the same vault detection and matching logic as the current resolver.

### Operational Diagnostics

When enabled, diagnostics are newline-delimited JSON records written only to stderr. Each record uses the fixed logger envelope (`timestamp`, `level`, `target`, `message`) and may include only `event`, process-local `request_seq`, resolver `status`, fixed `failure_kind`, and `duration_ms`. Allowed events are `server_started`, `server_stopped`, `request_started`, `request_completed`, `request_failed`, `cache_hit`, `cache_miss`, `cache_refresh_started`, `cache_refreshed`, `cache_refresh_failed`, and `watcher_unavailable`. Allowed failure kinds are `invalid_arguments`, `root_selection`, `cache_refresh`, `watch_setup`, `protocol`, `transport`, and `internal`. Event values, failure kinds, messages, and targets are fixed allow-listed values and MUST NOT interpolate untrusted data. A resolver result with status `error` is logged as `request_completed`; `request_failed` means no resolver result was produced. No other diagnostic fields are permitted. Detailed protocol and logger behavior is specified in `contracts/mcp.md`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every case in the normative quickstart conformance corpus, the MCP wrapper returns the same outcome category as the underlying resolver, matches every result field the resolver provides, and adds no wrapper-only result fields. Target path and location are compared only for cases where the resolver returns those fields. Path behavior is validated with platform-appropriate fixtures for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-gnu`.
- **SC-002**: The conformance suite MUST separately assert and report outcome-category coverage: at least one normative corpus case for each resolver outcome—resolved, unresolved, sub-target-not-found, ambiguous, and error (5/5 categories). Each outcome remains as defined by the resolver.
- **SC-003**: Tool discovery and a valid tool call pass in both documented local-stdio client configurations—GitHub Copilot in VS Code and Claude Desktop (2/2 configurations)—without custom text parsing or shell integration. Validate against each client's latest stable release available when validation is performed and record each exact version, validation date, operating system, and result in `quickstart.md`. A later client release does not change an existing record unless validation is repeated.
- **SC-004**: The tool description explicitly tells clients when to prefer a simple target line versus a richer structured emplacement, and the guidance is verified against all applicable corpus scenarios: heading, block, and requested emplacement.
- **SC-005**: Identical inputs submitted against unchanged vault contents produce identical primary result records in 100% of repeated calls, with no non-deterministic fields in the main response; results may change when relevant vault contents change.
- **SC-006**: The cache conformance suite passes all four named scenarios in the quickstart—same-root reuse without per-call enumeration, event-triggered or 60-second fallback refresh, isolation of two canonical roots, and invalidation during an in-flight resolution followed by refresh before the next resolution. Detected filesystem changes are reflected before the next resolution, and changes not detected sooner are reflected within 60 seconds.
- **SC-007**: The project publishes prebuilt MCP server binaries for all five supported targets—`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-gnu` (5/5)—and provides complete local-stdio registration guides for both supported clients—GitHub Copilot in VS Code and Claude Desktop (2/2). Installation documentation provides latest-version install/update, pinned-version, and manual-fallback instructions for a dedicated MCP installer command/script, without changing the existing CLI installer or its CLI-only behavior; neither installer requires a component-selection parameter.
- **SC-008**: The tool supports path-qualified and vault-relative references, same-file, heading, block, and attachment references, plus unresolved, missing-sub-target, and ambiguous outcomes, as represented by the normative quickstart conformance corpus and the underlying resolver's behavior.
- **SC-009**: With diagnostic logging enabled, users can identify server startup, request outcomes, operational failures, and cache reuse or refresh outcomes, while MCP protocol responses remain unchanged; with logging disabled, no optional diagnostic logs are emitted.
- **SC-010**: On the designated release benchmark runner, end-to-end latency from receipt of a tool call to completion of its response has a p50 of at most 100 ms across 100 consecutive warm-cache calls against the approximately 5,000-note benchmark vault. Process startup and initial index construction are excluded; the runner and fixture version are recorded with results.
- **SC-011**: Boundary contract tests verify all three response classes: missing required arguments and wrong-type required or optional arguments produce an MCP tool/protocol error; schema-valid arguments rejected by the resolver produce its normal `error` result with reason. Resolver outcomes such as `unresolved` and `sub-target-not-found` remain normal tool results.
- **SC-012**: For each published MCP contract compared under FR-012, the compatibility check passes within the same major version and rejects a breaking change unless it uses a new major version and includes migration notes.
- **SC-013**: Pushing a valid `vX.Y.Z` tag runs the workspace release gates and publishes distinct library/FFI, CLI, and MCP assets built from their respective crates, all identified with the same synchronized version; the CLI installer installs only the CLI, and the separate MCP installer installs only the MCP server without a component-selection parameter.
- **SC-014**: On GitHub-hosted `ubuntu-24.04`, the project measures 100 fresh MCP server processes from process launch through completion of the first resolution against the approximately 5,000-note `bench-vault-v1` corpus, including startup and initial index construction. The resulting machine-readable cold-start report records the runner image, operating system, architecture, fixture version, sample count, and p50 as a CI and release baseline without a failure threshold until a latency budget is established.

## Assumptions

- The MCP wrapper is a compatibility layer over the existing Obsidian Link Resolver rather than a second implementation of resolution logic.
- The current resolver's semantics and edge cases remain the authoritative contract for link interpretation.
- Supported-client compatibility is limited to GitHub Copilot in VS Code and Claude Desktop using the documented local-stdio configurations; each configuration must pass tool discovery and a valid tool call. Validate against each client's latest stable release available when validation is performed and record exact versions, validation date, operating system, and result in `quickstart.md`. A later client release does not change a recorded result unless validation is repeated. No compatibility claim is made for other clients.
- Agent workflows rely on deterministic result fields and stable outcome categories rather than human-readable prose alone.
- The wrapper's scope is limited to exposing the current capability through MCP; it does not expand the underlying link-resolution rules beyond the current tool's contract.
- Diagnostic logging is opt-in and limited to operational server state; request payloads, link text, and note contents are not needed in logs.
