# Quickstart: MCP Link Guidance Validation

## Prerequisites

- Rust toolchain and Cargo available in the repository environment.
- The existing MCP server crate and resolver crate.

## Automated Validation

Run the focused packages:

```sh
cargo test -p obsidian-link-resolver -p obsidian-link-resolver-mcp
```

Expected result: both packages pass. MCP metadata contract coverage confirms the five forms, verbatim-link guidance, source-file context, startup access boundary, and concise initialization purpose. Stdio integration coverage confirms unchanged submissions from a configured root reach the resolver and an unconfigured root remains inaccessible. Existing resolver tests continue to verify parsing and outcomes.

## Protocol Acceptance Scenario

1. Start the MCP server with a directory of Markdown notes configured as an allowed root. No Obsidian application is required.
2. Initialize an MCP client and inspect its instructions; confirm they state the resolver purpose briefly without listing link syntax.
3. List tools and inspect `resolve_obsidian_link`; confirm it identifies `[[Note]]`, `[[Note#Heading]]`, `[[Note#^blockid]]`, `[text](Note.md)`, and `[text](Note.md#Heading)`, and says to pass the source link unchanged with its source file as context.
4. Submit each of those five link strings unchanged, with the Markdown file containing it as `context_path` under the configured root. Confirm requests reach the existing resolver and outcomes follow its current behavior.
5. Repeat with a source Markdown file under a directory not configured at startup. Confirm it remains inaccessible, even if the agent can read the file, and the guidance does not imply link discovery or document search.

See [the MCP link guidance contract](contracts/mcp-link-guidance.md) and [the data model](data-model.md) for the unchanged request boundary and metadata responsibilities.
