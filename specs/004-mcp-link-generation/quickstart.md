# Quickstart: MCP Link Generation

This guide describes the focused validation to run after implementation. It does not add or modify vault content through the MCP tool.

## Prerequisites

- Rust stable with the workspace's Rust 2021 toolchain.
- A local test vault (the repository fixtures or a temporary vault with `.obsidian/`, notes, and supported attachments).
- The MCP server started with the vault root explicitly allowed using `--vault <DIR>`.

## Contract and Integration Tests

Run the MCP crate tests:

```sh
cargo test -p obsidian-link-resolver-mcp
```

The contract tests should verify tool registration, schema defaults/constraints, strict argument handling, and stable structured output. Integration tests should cover both formats, existing and future targets, exact note/heading/block validation, attachments, embeds, same-file shorthand, ambiguity, malformed/unrepresentable input, and source/target paths that attempt to escape configured roots.

Run the existing MCP warm benchmark to guard the 100 ms p50 budget, including a generation case:

```sh
cargo bench -p obsidian-link-resolver-mcp --bench mcp_warm
```

## MCP Smoke Call

Initialize the stdio MCP server, then call `generate_obsidian_link` with:

```json
{
  "context_path": "/vault/notes/Context.md",
  "target_path": "projects/Plan.md",
  "intent": "existing",
  "format": "markdown",
  "output": "link"
}
```

Expected behavior: the returned destination is vault-relative and the structured result reports `validated` only if it resolves to `projects/Plan.md` exactly. A future target with `intent: "future"` must be returned as `unverified`, with syntax and path checks reported separately from destination validation. No vault files are created or modified.

For an embed, set `output` to `embed`. For same-file shorthand, set `same_file` to `true`, set `target_path` to the contextual note's vault-relative path, and supply exactly one of `heading` or `block_id`.
