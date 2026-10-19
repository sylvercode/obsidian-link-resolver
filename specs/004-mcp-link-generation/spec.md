# Feature Specification: MCP Link Generation

**Feature Branch**: `004-mcp-link-generation` (feature directory; no branch was created)

**Created**: 2026-10-19

**Status**: Draft

**Input**: User description: `docs/mcp-link-generation-brainstorm.md`

## Clarifications

### Session 2026-10-18
- Q: Which path style should generated links use for their destination? → A: Use vault-relative paths for both Wikilinks and Markdown links, matching Obsidian's documented internal-link behavior.

- Q: How should the generated Markdown link's visible text be chosen? → A: Callers may supply an optional custom label; when omitted, derive it from the target and requested subtarget.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Generate a link to an existing destination (Priority: P1)

An agent has chosen a note, heading, or block to reference and asks the MCP for link text using a source file for context. The MCP formats the requested link and checks that it resolves to the exact destination the caller named. The agent can insert the returned text only when that destination check succeeds.

**Why this priority**: Reliable links to existing content are the main value of generation: callers get consistent syntax and evidence that the link points where intended.

**Independent Test**: For whole-note, heading, and block targets in a test vault, request both supported formats and verify the returned text resolves to the requested target. Include missing, ambiguous, and incorrect subtargets and verify they are not reported as validated.

**Acceptance Scenarios**:

1. **Given** an existing target note and a contextual source file, **When** the caller requests a Wikilink, **Then** the response contains link text and confirms validation only if that text resolves to the requested note.
2. **Given** an existing target heading or block ID, **When** the caller requests a link to it, **Then** the response identifies whether the generated link resolves to that exact subtarget.
3. **Given** a missing, ambiguous, or mismatched destination, **When** the caller requests an existing-target link, **Then** the response explains the validation failure and does not present the candidate as validated.
4. **Given** an existing target and a request for Markdown format, **When** the link is generated, **Then** the response uses Obsidian-compatible Markdown link syntax and reports the destination-validation result.

### User Story 2 - Prepare a link to future content (Priority: P2)

An agent is preparing a note that will refer to a note, attachment, heading, or block that does not exist yet. It asks for representable link text without implying that the destination exists or will be created.

**Why this priority**: Callers often draft links before all referenced content is written, and must be able to distinguish a formatted candidate from a verified destination.

**Independent Test**: Request links for a missing future note or attachment, and for a future heading or block in an existing note. Confirm syntax and allowed-path checks are reported separately from destination validation, and confirm no vault files change.

**Acceptance Scenarios**:

1. **Given** a target explicitly identified as future content, **When** the caller requests a link, **Then** the response labels the result as unverified and does not claim that the destination exists.
2. **Given** an existing note with a heading or block planned for later creation, **When** the caller requests a future-target link, **Then** the response distinguishes accepted syntax from the unverified subtarget.
3. **Given** an explicit existing-target request whose destination is missing, **When** the request is processed, **Then** it fails validation rather than being silently treated as a future-target request.
4. **Given** a target explicitly identified as a future attachment, **When** the caller requests a link or embed, **Then** the response reports it as unverified without implying that the file exists or will be created.

### User Story 3 - Choose link format and subtarget (Priority: P3)

A caller chooses Wikilink or Markdown for each request and may request a whole-note, heading, block, or attachment reference. The caller may request same-file shorthand for a subtarget or embedded output. The default is Wikilink when no format is supplied; the MCP does not infer the caller's preference from Obsidian settings.

**Why this priority**: Per-request format choice supports different authoring preferences without tying generation to an external settings file.

**Independent Test**: Repeat requests with omitted, Wikilink, and Markdown format selections for note, subtarget, same-file shorthand, and attachment references, including linked and embedded output; confirm syntax, destination, and validation status.

**Acceptance Scenarios**:

1. **Given** a valid request without a format, **When** the caller generates a link, **Then** the result uses Wikilink format.
2. **Given** a valid request with either supported format selected, **When** the caller generates a link, **Then** the result uses that format regardless of Obsidian settings.
3. **Given** a request containing both a heading and block selector, **When** the caller requests generation, **Then** the request is rejected as invalid rather than choosing one selector silently.
4. **Given** a heading or block in the contextual source note, **When** the caller requests same-file shorthand, **Then** the generated link omits the note path and identifies the requested subtarget.
5. **Given** a note or attachment target, **When** the caller requests embedded output, **Then** the generated text uses the embed syntax for the selected format and reports the applicable destination-validation result.

### Edge Cases

- The source file is planned but does not yet exist; it is accepted only when its intended location can be shown to remain within the allowed vault boundary.
- A target path or source path attempts to escape the allowed vault, including through a symlink; generation is refused.
- A heading or block identifier occurs more than once; the response does not claim exact validation unless the intended subtarget is unambiguous.
- A path, heading, block ID, or label contains characters that cannot be represented faithfully in the requested format; the request fails clearly rather than returning misleading text.
- A target exists but the requested heading or block does not; an existing-target request reports subtarget validation failure.
- Link text must not cause a file to be created or changed, including by inserting a block ID.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The MCP MUST accept a contextual source path and a target path, and optionally one heading or one block ID; it MUST support whole-note and attachment references, including same-file shorthand for a subtarget in the source note.
- **FR-002**: The MCP MUST accept a per-request format choice of Wikilink or Obsidian-compatible Markdown and MUST default to Wikilink when no format is supplied. It MUST NOT read Obsidian settings to choose a format.
- **FR-003**: The caller MUST explicitly identify a request as targeting existing or future content. A missing existing destination MUST NOT be silently accepted as future content.
- **FR-004**: For existing-target requests, the MCP MUST report validation as successful only when the generated link resolves to the requested note or attachment and requested subtarget, when applicable. It MUST report a clear failure when the destination is missing, ambiguous, or mismatched.
- **FR-005**: For future-target requests, the MCP MUST distinguish syntax and path-boundary checks from destination validation and MUST NOT imply that the destination exists or will be created.
- **FR-006**: The MCP MUST keep generation read-only: it returns link text and applicable validation details, and MUST NOT create or modify files, insert block IDs, or perform note edits.
- **FR-007**: Source and target paths MUST remain within the vault locations allowed to the MCP at startup, including when a path does not yet exist or is affected by symlinks.
- **FR-008**: Generated Wikilink destinations MUST use vault-relative paths. Generated Markdown destinations MUST use paths relative to the source note.
- **FR-008**: Generated Wikilink and Markdown destinations MUST use vault-relative paths.
- **FR-009**: For Markdown output, the MCP MUST provide link text suitable for insertion. Callers MAY supply an optional display label; when omitted, the MCP MUST derive the label from the target and requested subtarget.
- **FR-010**: Inputs that cannot be represented faithfully in the chosen format MUST produce a clear failure and MUST NOT be presented as validated links.
- **FR-011**: The MCP MUST accept a per-request link-or-embed choice and support embedded output for note and attachment targets. Link discovery, document search, and note editing are outside the initial feature scope.

### Key Entities *(include if data involved)*

- **Link Request**: The caller's source context, note or attachment target path, optional heading or block ID, requested format and link-or-embed choice, and existing-or-future intention.
- **Destination**: A vault-relative note or attachment, optionally identified by a heading or block ID when the target is a note, that the caller wants the link to reference.
- **Generation Result**: The returned link text and a clear distinction between destination validation, syntax checks, and validation failures.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In acceptance testing, 100% of supported existing-note, attachment, heading, and block requests reported as validated resolve to the exact requested destination.
- **SC-002**: In acceptance testing, 100% of future-target results are explicitly distinguished from destination-validated results.
- **SC-003**: Callers can request either link format, and an omitted format produces Wikilink output in 100% of tested requests.
- **SC-004**: In acceptance testing, zero generation requests create or modify vault files or insert block IDs.
- **SC-005**: Users can identify from each response whether its destination is verified, unverified, or rejected without interpreting ambiguous success wording.
- **SC-006**: In acceptance testing, 100% of same-file shorthand requests identify the requested heading or block without including the source note path.
- **SC-007**: In acceptance testing, 100% of embed requests use the embed syntax for the selected link format.

## Assumptions

- The existing resolver remains the authority for note, heading, block, ambiguity, and path-resolution behavior; attachment validation confirms the exact in-vault file exists, and generation does not change existing resolution behavior.
- Existing-target versus future-target intention is explicit, and validation failures never fall back to future-target acceptance.
- A planned source file may be identified by its intended path, subject to the same allowed-vault boundary as existing files.
- Duplicate subtargets are not claimed as exactly validated unless the resolver can distinguish the requested occurrence.
- Callers may supply a custom Markdown label; when omitted, the label is derived from the target and requested subtarget. Other display and escaping details can be settled during planning without changing the read-only scope.
- Same-file shorthand, embedded output, and attachment targets are in scope; link discovery, document search, and note editing remain out of scope.
