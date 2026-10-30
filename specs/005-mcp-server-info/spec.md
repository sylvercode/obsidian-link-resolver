# Feature Specification: MCP Server Info Tool

**Feature Branch**: `005-mcp-server-info` (feature directory; no branch was created)

**Created**: 2026-10-29

**Status**: Draft

**Input**: User description: "add a tool to the mcp to know what are the configured vault and the current version"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Discover the configured vaults (Priority: P1)

An agent connected to the MCP needs to know which vault locations the server was started with so it can construct valid paths for the link tools, explain to the user which vaults are reachable, and avoid requests that will be rejected for being outside the allowed vaults. It calls a single info tool and receives the list of configured vault locations.

**Why this priority**: Today a caller can only learn the vault boundary by trial and error (a request fails because the path is outside the allowed vaults). Exposing the configured vaults removes guesswork and is the core value of the feature.

**Independent Test**: Start the server with one vault, call the info tool, and verify the response lists exactly that vault. Repeat with two vaults and verify both appear in configuration order.

**Acceptance Scenarios**:

1. **Given** the server was started with a single vault, **When** the caller invokes the info tool, **Then** the response lists exactly one vault entry whose location is the absolute, resolved path of that vault.
2. **Given** the server was started with multiple vaults, **When** the caller invokes the info tool, **Then** the response lists every configured vault in the order they were supplied at startup, with no omissions or duplicates.
3. **Given** the caller passes no arguments, **When** it invokes the info tool, **Then** the tool succeeds; the tool requires no input.

---

### User Story 2 - Discover the running server version (Priority: P2)

An agent or a human troubleshooting an integration needs to know which version of the MCP server is running so they can match behavior against documentation, confirm an upgrade took effect, or report a bug accurately. The same info tool returns the server's semantic version.

**Why this priority**: Version visibility is essential for support and compatibility checks, but it is secondary to knowing where links can be resolved.

**Independent Test**: Call the info tool and verify the returned version equals the version the server binary was built as (e.g., the value printed by the binary's own version flag or recorded in its release metadata).

**Acceptance Scenarios**:

1. **Given** a running server, **When** the caller invokes the info tool, **Then** the response includes the server's version as a semantic version string (MAJOR.MINOR.PATCH, optionally with pre-release/build metadata).
2. **Given** the server is upgraded and restarted, **When** the caller invokes the info tool again, **Then** the reported version reflects the newly installed build.

---

### User Story 3 - Version visible in the connection handshake (Priority: P3)

An MCP client that lists connected servers (for example, a chat client's server inspector) shows the server name and version taken from the standard MCP handshake, without the agent having to call a tool.

**Why this priority**: Complements the tool for human operators using client UIs; useful but not required for agents, which can call the tool directly.

**Independent Test**: Connect a client, inspect the handshake's server information, and verify it contains the server name and the same version string reported by the info tool.

**Acceptance Scenarios**:

1. **Given** a client completes the MCP handshake, **When** it reads the server information, **Then** it contains a server name and a version identical to the version returned by the info tool.

---

### Edge Cases

- A configured vault directory is removed or becomes inaccessible after startup: the tool still reports the vault as configured; it reports startup configuration, not live health.
- The same vault is supplied more than once at startup (including via different spellings that resolve to the same location): the tool reports each distinct resolved location once.
- The tool is called before any link tool has been used (no vault has been indexed yet): it responds without triggering vault indexing and without noticeable delay.
- Vault paths contain non-ASCII characters or spaces: they are returned faithfully and unaltered apart from being resolved to absolute form.
- The tool is called with unexpected arguments: the call either ignores them or fails with a clear message; it never returns partial or misleading information.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The MCP MUST expose a dedicated tool that returns server information and requires no input parameters.
- **FR-002**: The response MUST include the list of vaults the server was configured with at startup, each as an absolute, resolved location, in the order supplied at startup, with duplicates (after resolution) reported once.
- **FR-003**: The response MUST include the running server's version as a semantic version string.
- **FR-004**: The tool MUST be read-only: it MUST NOT create, modify, or index vault content, and MUST NOT change server state.
- **FR-005**: The response MUST be deterministic for a given server process: repeated calls return identical content, and no timestamps or other non-deterministic fields are included.
- **FR-006**: The response MUST be structured (machine-readable) and compact, with short, stable field names, so that agents can parse it without free-text interpretation.
- **FR-007**: The tool's description MUST tell callers what the tool returns and that it takes no arguments, so agents can select it without trial and error.
- **FR-008**: The server's standard MCP handshake information MUST include the server name and the same version string returned by the tool.
- **FR-009**: Adding the tool MUST NOT change the behavior, input, or output of the existing link resolution and link generation tools.

### Key Entities *(include if feature involves data)*

- **Server Info**: The information returned by the tool: the server version and the list of configured vaults.
- **Configured Vault**: A vault location accepted at startup, represented by its absolute, resolved path.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In acceptance testing, 100% of info tool calls return exactly the set of vaults the server was started with, in startup order, for configurations of one, two, and three vaults.
- **SC-002**: In acceptance testing, 100% of info tool calls return a version string equal to the build's declared version.
- **SC-003**: Two consecutive info tool calls on the same server process return byte-identical content in 100% of tested cases.
- **SC-004**: The info tool responds in under 50 ms on the existing benchmark vault configuration, including when called before any vault has been indexed.
- **SC-005**: Zero vault files are created or modified by any info tool call in acceptance testing.
- **SC-006**: All existing MCP contract tests for link resolution and link generation continue to pass unchanged.

## Assumptions

- The server version reported is the version of the MCP server build; the resolver library shares the same release version, so a single version value is sufficient.
- Reporting the configured vault locations to a connected client is acceptable because the client (or its operator) supplied those locations when configuring the server.
- Live accessibility or health checks of each vault (e.g., "still exists", "readable") are out of scope; the tool reports startup configuration only.
- Additional diagnostics such as cache statistics, indexed note counts, uptime, or build metadata beyond the version are out of scope for this feature.
- The tool is intended to be inexpensive enough to call on every agent session start.
