# Research: MCP Link Generation

**Feature**: [MCP Link Generation](spec.md)  
**Date**: 2026-10-18

## Decisions

### Use the Existing Rust MCP and Resolver Crates

**Decision**: Register generation in `obsidian-link-resolver-mcp`; keep formatting and request validation in a focused MCP module and reuse `obsidian-link-resolver` parsing, vault-index, and resolution APIs. Add no dependency.

**Rationale**: The server already uses `rmcp` 3.5.1, a process-scoped `VaultCache`, typed `serde`/`schemars` requests, and MCP stdio contract tests. The resolver already models Wikilink/Markdown styles, embeds, aliases, same-file targets, headings, blocks, and attachments. A second parser or cache would duplicate behavior and increase compatibility risk.

**Alternatives considered**: Add generation to the resolver library; rejected for the initial plan because generation is an MCP-facing capability and current output choices do not need to expand the library API. Add a new formatter dependency; rejected because a small, well-tested destination encoder is sufficient and no dependency is otherwise needed.

### Use Vault-Relative Paths in Both Formats

**Decision**: Request targets are exact vault-relative paths. Generated Wikilink and Markdown destinations both use vault-relative paths with `/` separators; Markdown destination components are percent-encoded as required by Obsidian. Note links may omit `.md` in Wikilinks; Markdown output retains the note extension.

**Rationale**: This is the accepted clarification in [spec.md](spec.md#clarifications), matches the current vault index, and follows Obsidian's documented internal-link behavior. The official guide says folder paths start at the vault root for both Wikilink and Markdown links: [Internal links](https://obsidian.md/help/links).

**Alternatives considered**: Source-relative Markdown paths; rejected by the user after checking documented Obsidian behavior and existing resolver semantics. Bare note names; rejected because they can be ambiguous in nested vaults.

### Reuse Parsing and Resolution for Existing-Target Verification

**Decision**: Render a candidate, parse/resolve it using the existing library against the cached vault, and report `validated` only when the resolved canonical vault-relative path and optional subtarget match the requested destination exactly. Keep future-target syntax/path checks distinct from destination validation.

**Rationale**: `resolve_with_vault` already exposes the required status distinctions, identifies attachments, checks headings and blocks, and preserves embed metadata. This is the strongest available local validation oracle without changing resolver behavior.

**Alternatives considered**: Mark a candidate valid based only on formatting; rejected because it cannot prove destination identity. Change existing resolution semantics; rejected because Feature 004 is additive and the spec assumes the resolver remains authoritative.

### Keep Vault Boundaries at Startup-Configured Roots

**Decision**: Select only a configured canonical vault root. For an existing source/target, canonicalize and verify containment. For a planned source that does not exist, canonicalize the nearest existing path prefix, verify it remains under the configured root, and permit only a normal, non-escaping missing tail; reject dangling or escaping symlinks. Target paths remain vault-relative and are joined beneath that root.

**Rationale**: The current MCP canonicalizes existing context paths and only accepts roots passed with `--vault`; Feature 004 additionally allows a planned source file. Canonicalizing the existing prefix preserves the allow-list boundary without requiring the planned file to exist.

**Alternatives considered**: Lexical `..` checks alone; rejected because symlinks can escape a root. Requiring the source file to exist; rejected because it conflicts with the planned-source use case.

### Use Obsidian's Documented Link and Embed Syntax

**Decision**: Wikilinks use `[[...]]`, aliases use `|`, subtargets use heading/block selectors, same-file shorthand omits the target path, and embeds use the `!` prefix. Markdown links use `[label](destination)` and Markdown embeds use `![label](destination)`. Escape/encode destinations so parsing round-trips; reject values that cannot be represented faithfully.

**Rationale**: The official guides document internal links, heading/block references, display text, and the `!` embed prefix: [Internal links](https://obsidian.md/help/links), [Embed files](https://obsidian.md/help/embeds). Accepted attachment types are documented in [Accepted file formats](https://obsidian.md/help/file-formats) and already enumerated by `vault.rs`.

**Alternatives considered**: Introduce plugin-specific attachment formats; rejected because plugin extensions are not represented by the resolver's index or this feature's scope. Mutate note content to insert block IDs; rejected by the explicit read-only requirement.

## Local Integration Findings

- The MCP server is a binary crate with its tool router and typed request models in `crates/obsidian-link-resolver-mcp/src/mcp.rs`.
- The server already canonicalizes configured roots, uses a shared `VaultCache`, and returns structured JSON MCP content.
- The resolver's `Link` model supports Wikilink/Markdown, embeds, same-file references, headings, blocks, and display text. The parser percent-decodes Markdown destinations.
- `Vault` indexes supported Markdown and attachment paths in vault-relative slash form. Attachment extension and filename rules are already centralized in `crates/obsidian-link-resolver/src/vault.rs`.
- Contract tests live under `crates/obsidian-link-resolver-mcp/tests/contract/`; end-to-end stdio tests live under `tests/integration/`; `StdioClient` is in `tests/support/mcp_client.rs`.
- `mcp_warm` already enforces a 100 ms p50 budget over a 5,000-note fixture. Add a generation sample to this existing benchmark rather than inventing a second threshold.

## Remaining Risk

The official Obsidian guides document embed-by-prefix behavior and accepted file formats, but the repository has no live Obsidian application test. Contract tests should verify the emitted Markdown and Wikilink forms round-trip through the local parser; an Obsidian smoke check remains useful during acceptance testing, especially for non-image Markdown embeds.
