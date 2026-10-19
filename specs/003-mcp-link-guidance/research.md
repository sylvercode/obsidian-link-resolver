# Research: MCP Link Guidance

## Decision 1: Keep link parsing and resolution unchanged

**Decision**: Update only the MCP tool description and initialization instructions. Preserve the existing `resolve_obsidian_link` schema, resolver call path, and result behavior.

**Rationale**: The feature spec assigns syntax recognition to model-facing guidance and explicitly leaves parsing and resolution outcomes to the resolver. The resolver already parses Wikilinks and Markdown links; existing resolver unit/integration tests include Markdown links with heading targets.

**Alternatives considered**: Add or alter parser behavior was rejected because it is out of scope and not needed to communicate supported examples.

## Decision 2: Put detailed guidance on the resolver tool

**Decision**: Expand the tool's discoverable description with `[[Note]]`, `[[Note#Heading]]`, `[text](Note.md)`, and `[text](Note.md#Heading)`, preserving source text, source-file context, and startup directory boundaries. Keep initialization instructions to one concise purpose statement.

**Rationale**: `ResolverServer::resolve_obsidian_link` owns the tool description, and `ResolverServer::get_info` owns initialization instructions. The existing metadata contract test already initializes the server, inspects the discovered tool, and submits a real request.

**Alternatives considered**: Duplicating the syntax list in initialization instructions was rejected because it competes with the detailed tool guidance. Adding a new tool or skill was rejected by FR-006.

## Decision 3: Describe configured Markdown folders without implying Obsidian or discovery

**Decision**: State that a configured folder of Markdown files can be resolved without the Obsidian application, while access is limited to directories allowed at server startup. Do not imply that the server discovers links or searches documents.

**Rationale**: The MCP server accepts an explicit link and context path and enforces its configured roots; it does not scan Markdown documents to find links.

**Alternatives considered**: Describing vault discovery or automatic link extraction was rejected because neither is part of the current tool contract or this feature.

## Validation and Dependencies

No new dependency, persistent data, API field, or runtime capability is required. Validate the metadata contract and stdio behavior with `cargo test -p obsidian-link-resolver-mcp`; validate unchanged parsing/resolution with `cargo test -p obsidian-link-resolver`.
