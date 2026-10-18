# Feature Specification: MCP Link Guidance

**Feature Branch**: `003-obsidian-reference`

**Created**: 2026-10-18

**Status**: Draft

**Input**: User description: "Update the existing MCP metadata so an agent can recognize supported Obsidian-style and Markdown link forms in a Markdown file and pass the link to the resolver exactly as written. Keep server initialization instructions brief and focused on the MCP's purpose. A folder of Markdown files does not require the Obsidian app, but the server can access only directories configured at startup. Link discovery, document search, new tools, and new skills are out of scope."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pass source links to the resolver (Priority: P1)

An agent reading a Markdown file encounters a link and needs to resolve its target. The agent can recognize the supported link forms from the resolver's tool description, preserve the source link exactly, and submit it with the file containing the link as context. This works for a folder of Markdown files without requiring the Obsidian app, provided the directory is configured for server access at startup.

**Why this priority**: Recognizing the link forms and preserving them avoids needless conversion and lets agents use the resolver with ordinary Markdown documentation.

**Independent Test**: Inspect the resolver tool's discoverable description and verify that it identifies the four supported example forms and instructs callers not to rewrite them. Submit each form unchanged with its source file as context and verify it reaches the existing resolver.

**Acceptance Scenarios**:

1. **Given** a Markdown file containing `[[Note]]` or `[[Note#Heading]]`, **When** an agent chooses a link to resolve, **Then** the tool guidance identifies the form and the agent passes the exact source text as the link.
2. **Given** a Markdown file containing `[text](Note.md)` or `[text](Note.md#Heading)`, **When** an agent chooses a link to resolve, **Then** the tool guidance identifies the form and the agent passes the exact source text as the link.
3. **Given** a directory of Markdown files configured for server access at startup and no Obsidian app, **When** an agent submits a supported link with its source file as context, **Then** the resolver can resolve it using its existing behavior.
4. **Given** a link is present in a Markdown file, **When** the agent reads the tool description, **Then** it is told not to convert the link to another supported form before submitting it.

### User Story 2 - Understand the MCP's purpose at initialization (Priority: P2)

A client or agent initializes the MCP server and receives a concise explanation of its purpose, without a second, lengthy list of link syntax competing with the resolver tool description.

**Why this priority**: Initialization should orient the client while the tool description supplies the detailed calling guidance at the point it is needed.

**Independent Test**: Inspect the server's initialization instructions and confirm they state the MCP's purpose concisely without duplicating the detailed link-form guidance.

**Acceptance Scenarios**:

1. **Given** an MCP client initializes the server, **When** it receives the server instructions, **Then** they briefly describe the resolver's purpose and do not replace the tool's link-form guidance.

### Edge Cases

- A directory not configured for server access at startup remains inaccessible even if the agent can read a Markdown file from it.
- A link syntax other than the four documented examples is not newly promised by this guidance; existing resolver behavior remains authoritative.
- The guidance must not imply that the server discovers links or searches documents on the agent's behalf.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The resolver tool's discoverable description MUST identify Wikilinks in the forms `[[Note]]` and `[[Note#Heading]]` and Markdown links in the forms `[text](Note.md)` and `[text](Note.md#Heading)` as supported examples.
- **FR-002**: The tool description MUST instruct agents to pass a link exactly as written in its Markdown source and MUST NOT encourage conversion between link forms.
- **FR-003**: The tool description MUST explain that the source Markdown file is supplied as context for resolving the link.
- **FR-004**: The server's initialization instructions MUST remain brief and focused on the MCP's purpose; detailed syntax guidance belongs in the resolver tool description.
- **FR-005**: The guidance MUST make clear that a directory of Markdown files can be used without the Obsidian app, while server access remains limited to directories configured at startup.
- **FR-006**: The feature MUST NOT add link discovery, document search, a new MCP tool, or a new skill.
- **FR-007**: The guidance MUST leave link parsing and resolution outcomes to the existing resolver behavior.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All four representative link forms named in FR-001 are identifiable in the resolver tool description, and each can be submitted unchanged as a resolver input in acceptance testing (4/4 forms).
- **SC-002**: In acceptance testing, an agent can resolve a supported link from a configured Markdown directory without the Obsidian app, while a directory not configured at startup remains outside the server's access scope.
- **SC-003**: Reviewers can identify the detailed link-form guidance in the resolver tool description and the concise purpose statement in initialization instructions without finding conflicting guidance.
- **SC-004**: The feature adds zero link-discovery or document-search capabilities, zero MCP tools, and zero skills.

## Assumptions

- The existing resolver remains the authority for which link syntaxes resolve and how their targets are interpreted.
- A Markdown folder is sufficient input context when its directory is configured for server access at startup; an Obsidian application or vault metadata is not additionally required by this guidance.
- Tool descriptions are the model-facing source for calling guidance, while server initialization instructions remain a brief statement of purpose.
