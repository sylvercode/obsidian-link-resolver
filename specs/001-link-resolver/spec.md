# Feature Specification: Obsidian Link Resolver CLI

**Feature Branch**: `001-link-resolver`

**Created**: 2026-09-17

**Status**: Draft

**Input**: User description: "obsidian-link-resolver is a CLI that take a link in the format of obsidian link (see: https://obsidian.md/help/links) and a context (file with the link) to return the target of that link. This can be as simple as the the file and a line, or optinaly the \"structured emplacement\" (the heading stack of the line in the file, with begin to end of those heading for sharded read later or any relevent type of content that is pointing). AI agent would be able to rely on the CLI (or MCP or skill wrapping it) to easely link information when readin obsidiant markdown file."

## Clarifications

### Session 2026-09-17

- Q: Should the resolver commit to a concrete common-case latency budget as an acceptance threshold, or only track latency for regressions? → A: Set a concrete warm-run budget (≤100 ms) as an acceptance threshold, plus regression tracking.
- Q: Should note-name and heading matching be case-insensitive, case-sensitive, or case-insensitive with a case-sensitive tie-breaker? → A: Case-insensitive for both note names and heading/block text (match Obsidian default).
- Q: What vault size should the resolver be designed and performance-tested against as its "representative vault"? → A: Medium — up to ~5,000 notes.

### Session 2026-09-19

- Q: Which language-agnostic integration surface(s) must the resolver guarantee for cross-technology consumption (at minimum .NET and Node.js)? → A: CLI machine-mode protocol + documented versioned JSON schema data contract + a C-compatible ABI/FFI boundary for in-process embedding (chosen to support many consecutive calls without per-call process spawn).
- Q: When a `vX.Y.Z` tag triggers the test-gated release pipeline, which distribution artifacts must it produce? → A: Prebuilt native CLI binaries plus the C-compatible shared library (FFI/ABI) for Linux (x64+arm64), macOS (x64+arm64), and Windows (x64).
- Q: Should the ≤100 ms warm-run performance budget be part of the blocking release test gate? → A: Yes — a warm-run p50 above ≤100 ms fails the `vX.Y.Z` release pipeline and blocks artifact publication.
- Q: How is a note name matched to a target file, and what happens when a bare name matches multiple notes? → A: Match the note-name portion case-insensitively against each note's basename (path-qualified names match the exact vault-relative path); exactly one match resolves, zero yields unresolved, two or more yields ambiguous with no silent selection.
- Q: When a path-qualified note name matches nothing at that exact path but a same-named note exists elsewhere, does the resolver fall back? → A: No — it reports unresolved and never falls back to a bare-name match.
- Q: When a note has duplicate headings (or duplicate block ids) with the same text/id, which one is targeted? → A: The first occurrence in document order (top to bottom), and the resolver reports which one (by line) was chosen.
- Q: For a nested heading path `#Parent#Child`, how are multiple matching parents or children disambiguated? → A: Resolve left-to-right, choosing the first match at each level in document order; if no child matches inside the selected parent's section, report sub-target-not-found.
- Q: What is the complete outcome set referenced by both the outcome and exit-status requirements? → A: Exactly five mutually exclusive outcomes — resolved, unresolved, sub-target-not-found, ambiguous, and error.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Resolve a link to its target file and location (Priority: P1)

An AI agent (or a developer) is reading an Obsidian markdown note and encounters a link such as `[[Project Plan#Milestones]]`. Given that link text and the path of the note that contains it, the caller asks the resolver where the link points. The resolver returns the target note's file path and, when the link points to a heading or block, the exact line where that target begins.

**Why this priority**: This is the core value of the tool. Without the ability to turn a link plus its context into a concrete file-and-location target, none of the downstream capabilities (structured reading, agent linking) are possible. This story alone delivers a usable MVP: an agent can follow any Obsidian link to the right file and line.

**Independent Test**: Can be fully tested by providing a sample vault, a context file path, and a link string, then asserting that the returned target file path and line match the known location of the referenced note/heading/block.

**Acceptance Scenarios**:

1. **Given** a link `[[Project Plan]]` and a context file within a vault that contains exactly one note named `Project Plan`, **When** the resolver runs, **Then** it returns the target file path of `Project Plan` and identifies the start of the file as the target location.
2. **Given** a link `[[Project Plan#Milestones]]` where `Project Plan` contains a `## Milestones` heading, **When** the resolver runs, **Then** it returns the target file path and the line number of the `## Milestones` heading.
3. **Given** a link `[[Project Plan#^abc123]]` where the block id `^abc123` exists, **When** the resolver runs, **Then** it returns the target file path and the line number of the block that carries that id.
4. **Given** a same-file link `[[#Overview]]` and a context file that contains an `## Overview` heading, **When** the resolver runs, **Then** it returns the context file itself and the line of the `## Overview` heading.
5. **Given** a link that cannot be resolved (target note does not exist), **When** the resolver runs, **Then** it reports an unresolved result with a clear reason and a non-success exit code.

---

### User Story 2 - Return the structured emplacement of the target (Priority: P2)

When a caller needs more than a single line, the resolver optionally returns the "structured emplacement" of the target location: the stack of headings that contain the target (from the top-level heading down to the most specific one) and, for each heading in that stack, the line range (begin/end) of its section. This lets an agent later read exactly the relevant slice of a large note without loading the whole file.

**Why this priority**: This is a strong enhancement over a bare line number because it enables sharded/targeted reading of large notes, reducing the amount of content an agent must load. It builds directly on P1 but is not required for a minimal viable tool.

**Independent Test**: Can be tested by resolving a link into a note with nested headings and asserting that the returned heading stack and each heading's begin/end line range match the note structure.

**Acceptance Scenarios**:

1. **Given** a link that resolves to a line nested under `# Design` → `## API` → `### Auth`, **When** structured emplacement is requested, **Then** the resolver returns the ordered heading stack (`Design`, `API`, `Auth`) with the begin and end line of each section.
2. **Given** a link that resolves to the start of a note with no headings, **When** structured emplacement is requested, **Then** the resolver returns an empty heading stack and the whole-file line range as the containing section.
3. **Given** a link that resolves to a heading, **When** structured emplacement is requested, **Then** the section range for the resolved heading spans from its heading line up to (but excluding) the next heading of equal or higher level.

---

### User Story 3 - Machine-friendly output for agents and wrappers (Priority: P3)

An automated caller (an AI agent, an MCP server, or a skill wrapper) invokes the resolver and needs to parse the result programmatically without ambiguity. The resolver provides a compact, deterministic machine-readable output mode that distinguishes resolved, unresolved, and ambiguous outcomes, alongside a human-readable mode for interactive use.

**Why this priority**: Agent integration is the ultimate purpose of the tool, but it depends on the resolution logic (P1) and benefits from structured emplacement (P2). A deterministic output contract makes wrapping the CLI in an MCP server or skill reliable.

**Independent Test**: Can be tested by invoking the resolver in machine mode against known inputs and asserting the output is a single, parseable, deterministic record with stable field names and the correct outcome status.

**Acceptance Scenarios**:

1. **Given** any resolvable link, **When** the resolver runs in machine mode, **Then** it emits a single compact machine-readable record containing at least the target file path, target line, and outcome status.
2. **Given** an ambiguous link (a note name matching multiple notes), **When** the resolver runs in machine mode, **Then** it reports an ambiguous outcome and lists the candidate targets.
3. **Given** identical inputs run twice, **When** the resolver runs in machine mode, **Then** the primary result records are byte-for-byte identical (no non-deterministic fields in the result).

---

### Edge Cases

- **Ambiguous note name**: Multiple notes share the same name resolvable from the context. The resolver reports an ambiguous outcome and lists candidates rather than silently guessing.
- **Broken link**: The referenced note does not exist anywhere in the vault. The resolver reports unresolved with a reason.
- **Missing heading/block**: The note exists but the referenced heading or block id is absent. The resolver reports that the note resolved but the sub-target did not, including the resolved file path.
- **Aliased link**: A link like `[[Project Plan|Plan]]` is resolved by its target (`Project Plan`), and the alias display text does not change the target.
- **Embed link**: An embed such as `![[Note#Section]]` resolves to the same target as the equivalent non-embed link; the embed nature is reported but does not alter target resolution.
- **Markdown-style link**: A standard markdown link (e.g., `[text](Some%20Note.md#Heading)`) that points within the vault is resolved to the same kind of target as the equivalent wikilink.
- **Non-markdown target**: A link/embed to an attachment (image, PDF, etc.) resolves to the attachment file path with no heading/block line and no heading stack.
- **Duplicate headings**: A note contains two headings with the same text; the resolver selects the first occurrence in document order and reports which one (by line) was chosen (FR-002a).
- **Duplicate or malformed block id**: A note carries the same block id on multiple lines, or a link references a syntactically malformed block token; the resolver targets the first matching line in document order, and treats a malformed or absent block reference as sub-target-not-found (FR-002c).
- **Nested heading path**: A link like `[[Note#Parent#Child]]` targets the `Child` heading nested under `Parent`; when multiple parents or children match, the first at each level in document order is chosen, and a child that is absent within the selected parent's section yields sub-target-not-found (FR-002b).
- **Path-qualified note name**: A link like `[[folder/subfolder/Note#Heading]]` includes a folder path (relative to the vault root) before the note name; the resolver uses that path to disambiguate and target the specific note under that folder. If no note exists at that exact vault-relative path, the resolver reports unresolved and does not fall back to a bare-name match elsewhere in the vault (FR-005a).
- **Self-reference without note name**: `[[#Heading]]` or `[[#^block]]` resolves within the context file.
- **Vault root explicitly provided**: A caller supplies the vault root directly; the resolver uses it as-is and does not attempt auto-detection.
- **Vault root auto-detected**: No vault root is provided; the resolver locates it by finding an `.obsidian` directory in the context file's own directory or one of its parent directories, using the nearest match as the vault root.
- **Context file outside a vault**: Neither an explicit vault root is given nor is an `.obsidian` directory found in the context file's directory or any parent; the resolver reports that the vault could not be determined.
- **Case and whitespace differences**: Link text differs from the target only by case or surrounding whitespace; the resolver matches case-insensitively (after trimming surrounding whitespace), consistent with Obsidian's default behavior.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST accept, as input, an Obsidian link string and the path of the context file that contains the link.
- **FR-002**: The system MUST resolve wikilink-style internal links, including plain note links (`[[Note]]`), aliased links (`[[Note|Alias]]`), heading links (`[[Note#Heading]]`), nested heading links (`[[Note#Parent#Child]]`), block links (`[[Note#^blockid]]`), same-file links (`[[#Heading]]`, `[[#^blockid]]`), and embeds (`![[...]]`).
- **FR-002a**: When a heading reference matches multiple headings with the same text in the target note, the system MUST select the first such heading in document order (top to bottom) and MUST report which occurrence (by its line) was chosen.
- **FR-002b**: For a nested heading path (`#Parent#Child`), the system MUST resolve left-to-right: it selects the first heading matching `Parent` in document order, then, within that parent's section, selects the first deeper-level heading matching `Child`. When multiple parents or children match, the first at each level in document order is chosen deterministically; if no matching child exists within the selected parent's section, the system MUST report sub-target-not-found.
- **FR-002c**: When a block-id reference matches multiple lines carrying the same block id in the target note, the system MUST select the first such line in document order. A reference to a block id that is absent or syntactically malformed (not a valid `^id` token) MUST be reported as sub-target-not-found (the note resolved but the block could not be located).
- **FR-003**: The system MUST resolve markdown-style links that point to targets within the vault to the same target representation as the equivalent wikilink.
- **FR-004**: The system MUST determine the vault root used for note-name resolution in the following order: (a) use an explicit caller-provided vault root when given; otherwise (b) auto-detect it by locating an `.obsidian` directory in the context file's own directory or, failing that, in the nearest ancestor directory that contains one.
- **FR-004a**: When neither an explicit vault root is provided nor an `.obsidian` directory is found in the context file's directory or any ancestor, the system MUST report that the vault could not be determined rather than guessing a root.
- **FR-005**: The system MUST resolve a note name to a target file using the following deterministic rule (which matches Obsidian's default "shortest path when possible" behavior): the note-name portion of the link is matched, case-insensitively (FR-005b), against the basename (filename without the `.md` extension) of every markdown note in the vault. A bare note name (no folder prefix) matches every note whose basename equals the name; a path-qualified name (FR-005a) matches only the note at that exact vault-relative path. Exactly one match resolves to that note; zero matches yields unresolved; two or more matches yields ambiguous with the full candidate list and no silent selection (FR-010, FR-011).
- **FR-005a**: The system MUST support path-qualified note names, where a link includes a folder path (relative to the vault root) before the note name (e.g., `[[folder/subfolder/Note]]`), and MUST resolve to the note located at that path relative to the vault root. When no note exists at that exact vault-relative path, the system MUST report unresolved and MUST NOT fall back to matching the bare note name elsewhere in the vault.
- **FR-005b**: The system MUST match note names, folder path segments, and heading/block reference text case-insensitively, consistent with Obsidian's default link resolution behavior.
- **FR-006**: For a resolved target, the system MUST return the target file path and, when the link points to a heading or block, the line where that target begins.
- **FR-007**: When the link has no heading or block component, the system MUST return the target file with the beginning of the file as the target location.
- **FR-008**: The system MUST, on request, return the structured emplacement of the target: the ordered stack of containing headings (outermost to innermost) and, for each, the begin and end line of its section.
- **FR-009**: The system MUST compute each heading section's end as the line immediately before the next heading of equal or higher level (or end of file if none follows).
- **FR-010**: The system MUST distinguish and report the following outcomes: resolved, unresolved (target note not found), sub-target-not-found (note found but heading/block missing), ambiguous (multiple candidate targets), and error (usage error, vault could not be determined, or I/O failure). These five outcomes are the complete, mutually exclusive outcome set referenced by the exit-status contract (FR-017) and success criteria (SC-002).
- **FR-011**: For an ambiguous outcome, the system MUST list the candidate targets rather than silently selecting one.
- **FR-012**: The system MUST preserve the alias/display text of a link as informational output without letting it influence target resolution.
- **FR-013**: The system MUST identify whether a link is an embed and report that attribute without changing the resolved target.
- **FR-014**: The system MUST resolve links to non-markdown attachments to the attachment's file path, with no heading/block line and an empty heading stack.
- **FR-015**: The system MUST provide a deterministic, compact machine-readable output mode with stable field names for programmatic callers, and a human-readable mode for interactive use.
- **FR-016**: The system MUST exclude non-deterministic values (such as timestamps) from the primary result record so identical inputs produce identical result output.
- **FR-017**: The system MUST communicate outcome via a distinct, documented exit status so callers can branch on all five outcomes — resolved (success), unresolved, sub-target-not-found, ambiguous, and error — without parsing text.
- **FR-018**: The system MUST emit primary results on standard output and diagnostics/errors on standard error.
- **FR-019**: The system MUST report a clear, actionable reason for every non-resolved outcome (e.g., which note, heading, or block was not found, or why the vault could not be determined).
- **FR-020**: The system MUST return line references that are stable and unambiguous with respect to a defined convention (documented as 1-based line numbers).
- **FR-021**: The system MUST expose stable, language-agnostic integration surfaces so it can be consumed from other technology stacks (at minimum .NET and Node.js) regardless of implementation language: (a) the deterministic machine-mode CLI protocol (stdout results + documented exit statuses), (b) a documented, versioned JSON schema data contract for the result record, and (c) a C-compatible ABI/FFI boundary that allows the resolver to be embedded in-process by a host application.
- **FR-021a**: The in-process (FFI/ABI) integration surface MUST support many consecutive resolutions within a single host process without incurring a separate process spawn per call, and MAY reuse a loaded vault index across consecutive resolutions in the same host process.
- **FR-021b**: The system MUST NOT introduce hard dependencies that preclude .NET or Node.js integration through the surfaces in FR-021 without a documented justification and migration path.

### Key Entities *(include if feature involves data)*

- **Obsidian Link**: The reference being resolved. Attributes: raw link text, target note name (optional for self-references), optional folder path prefix (relative to vault root), heading path (optional, may be nested), block id (optional), alias/display text (optional), embed flag, link style (wikilink or markdown).
- **Context File**: The markdown file that contains the link. Used to resolve same-file references and relative/vault-wide name resolution. Attributes: file path, associated vault.
- **Vault**: The collection of notes and attachments within which names are resolved. Attributes: root location; used to enumerate candidate targets for a note name.
- **Resolution Target**: The result of resolving a link. Attributes: target file path, target line (optional), outcome status, candidate list (for ambiguous), reason (for non-resolved).
- **Structured Emplacement**: The positional context of a target within its note. Attributes: ordered heading stack (each with heading text, level, begin line, end line); overall section range for the target.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a defined corpus covering every documented Obsidian link form (plain, aliased, heading, nested heading, block, same-file, embed, markdown-style, attachment), the resolver returns the correct target for 100% of cases.
- **SC-002**: For any input, the caller can determine the outcome (resolved, unresolved, sub-target-not-found, ambiguous, error) solely from the exit status, without parsing output text, in 100% of cases.
- **SC-003**: Running the resolver twice on identical inputs in machine mode yields identical primary result records in 100% of cases.
- **SC-004**: A caller can request and receive the structured emplacement (heading stack with begin/end line ranges) for any resolvable target that lands inside a note, and the ranges exactly match the note's heading structure for 100% of a defined test corpus.
- **SC-005**: A typical single link resolution (warm run) against a representative vault (up to ~5,000 notes) completes within a ≤100 ms acceptance threshold, and common-case latency is measured and tracked in CI to prevent regressions. This budget is a blocking release gate: a warm-run p50 above ≤100 ms fails the `vX.Y.Z` release pipeline and prevents artifact publication.
- **SC-006**: An integrator can wrap the CLI in an MCP server or skill and parse its machine-mode output with no custom text scraping, relying only on the documented fields and exit statuses.
- **SC-007**: An integrator can consume the resolver from both a .NET and a Node.js host — via the documented JSON schema contract and/or the C-compatible ABI/FFI boundary — and execute many consecutive resolutions within a single host process without a separate process spawn per call, relying only on the documented integration surfaces.
- **SC-008**: Each `vX.Y.Z` tagged release produces, only after the full test suite passes, prebuilt native CLI binaries and the C-compatible shared library (FFI/ABI) for Linux (x64 and arm64), macOS (x64 and arm64), and Windows (x64), so integrators can obtain both the CLI and the in-process embedding surface for every supported platform without building from source.

## Assumptions

- **Vault detection**: The vault root is identified either (a) explicitly, when the caller provides it, or (b) by auto-detection, walking up from the context file's directory to find the nearest directory containing an `.obsidian` directory. If neither yields a root, the resolver reports that the vault could not be determined.
- **Path-qualified links**: A folder path preceding the note name in a link is interpreted as relative to the vault root, consistent with Obsidian's handling of path-qualified links.
- **Name resolution semantics**: Note-name resolution follows Obsidian's documented behavior, including shortest-path resolution and vault-wide matching; where Obsidian's behavior is configuration-dependent, the most common default is assumed.
- **Line numbering**: Line references use 1-based numbering; this convention is documented in the output contract.
- **Read-only operation**: The resolver only reads the vault and context file; it never modifies notes or vault contents.
- **Vault scale**: The resolver is designed and performance-tested for vaults of up to ~5,000 notes as the representative corpus; larger vaults should still function but are outside the guaranteed latency budget for the first version.
- **Scope of output**: The tool returns the location and structure of the target; retrieving or slicing the actual target content is a downstream concern (though the emplacement ranges are provided to enable it).
- **Encoding**: Notes are UTF-8 encoded markdown; other encodings are out of scope for the first version.
- **Single link per invocation**: Each invocation resolves one link in the context of one file; batch resolution is out of scope for the first version.
- **MCP/skill wrappers**: The CLI is the primary deliverable; MCP servers and skills are wrappers built on top of it and are out of scope for this specification.
- **Interoperability**: The implementation language is unconstrained provided the integration surfaces in FR-021 are preserved. Because many consecutive calls are expected, an in-process C-compatible ABI/FFI boundary is required in addition to the CLI protocol and JSON schema contract, so host applications (at minimum .NET and Node.js) can avoid per-call process spawn and may reuse a loaded vault index. Ready-made per-language binding packages are not required for the first version, but the ABI/FFI and data contract MUST make such bindings feasible without changes to core behavior.
