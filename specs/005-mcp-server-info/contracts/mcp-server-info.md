# MCP Contract: Server Info

## Tool

`get_server_info`

Returns the running server's version and the vault roots configured at startup. It is additive; `resolve_obsidian_link` and `generate_obsidian_link` retain their existing names, schemas, and behavior.

## Discovery (`tools/list`)

| Property | Value |
|---|---|
| `name` | `get_server_info` |
| `description` | States that the tool returns the server version and the startup-configured vault roots as canonical paths, and that it takes no arguments. Must contain the phrases "version", "configured at startup", and "no arguments" (case-insensitive). |
| `inputSchema` | `{"type":"object","properties":{}}` — no properties, no `required`, no `additionalProperties: false`. |

## Input

None. The tool accepts a call with `arguments` omitted, `{}`, or any object. Supplied arguments are ignored; the call never fails or returns a partial result because of them.

## Output

Successful calls return `isError: false` with `structuredContent`:

| Field | Type | Description |
|---|---|---|
| `version` | string | Semantic version of the server build; equal to the handshake `serverInfo.version`. |
| `vaults` | array of object | Configured vaults in startup order, each distinct canonical path once. |
| `vaults[].path` | string | Canonical absolute path exactly as resolved by the OS (Windows `\\?\` prefix preserved). |

Example:

```json
{"version":"2.0.0","vaults":[{"path":"/home/user/vaults/work"},{"path":"/home/user/vaults/notes"}]}
```

The `content` array contains one `text` block whose value is the compact JSON serialization of `structuredContent` (same parity as the other tools).

No error outcomes are defined for valid protocol calls. Repeated calls on the same process return byte-identical `structuredContent`.

## Handshake (`initialize` result)

| Field | Value |
|---|---|
| `serverInfo.name` | `obsidian-link-resolver-mcp` |
| `serverInfo.version` | Same string as the tool's `version` |
| `instructions` | Unchanged: `Resolve Obsidian links.` |

## Compatibility and Verification

- Additive only: no change to existing tool names, input schemas, result fields, or statuses; `specs/002-mcp-wrapper/contracts/mcp-contract.json` is not modified and no `MIGRATION.md` entry is required.
- Future fields are added to `ServerInfo` or to each `vaults[]` object; existing fields and shapes are never renamed, retyped, or removed within a major version.
- Contract tests assert: tool discovery and schema; description phrases; `initialize` `serverInfo` name/version; `version` equals the crate's `CARGO_PKG_VERSION`; calls with no, empty, and extra arguments succeed with identical content; text/structured parity.
- Integration tests assert: one, two, and three vaults in startup order; duplicate and symlink-alias roots reported once; paths equal `fs::canonicalize` of the fixtures; no files created or modified in any vault; the tool responds before any resolve call has indexed a vault; existing tools behave identically with duplicate `--vault` roots.
- Benchmark (`mcp_warm`) reports `server_info_first_call_ms` (single cold sample, gated at 100 ms) and `server_info_p50_ms` (gated at 50 ms, the SC-004 budget).

## References

- [MCP `initialize` result and `Implementation`](https://modelcontextprotocol.io/specification/2025-03-26/basic/lifecycle)
- [MCP tools: `tools/list` and `tools/call`](https://modelcontextprotocol.io/specification/2025-03-26/server/tools)
- Existing server implementation: `crates/obsidian-link-resolver-mcp/src/mcp.rs`
