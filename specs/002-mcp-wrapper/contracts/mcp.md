# MCP Contract: Obsidian Link Resolver

This contract adds one MCP stdio tool while preserving the existing resolver semantics and [result schema](../../001-link-resolver/contracts/result.schema.json).

## Contract Versioning

The MCP tool contract follows the project's semantic version. Within one major version, releases preserve argument names and requiredness, result field names and meanings, and outcome semantics. Additive optional arguments and fields are compatible. Any breaking contract change requires a new major version and migration notes. The first published MCP contract establishes the comparison baseline if no MCP contract has previously shipped; subsequent changes are checked against the latest published contract in the same major version.

## Server

- Executable: `obsidian-link-resolver-mcp`.
- Transport: MCP stdio. The client launches the local process and communicates using the standard MCP lifecycle and JSON-RPC messages over stdin/stdout.
- stdout is reserved exclusively for protocol messages. Diagnostics and watcher warnings go to stderr.
- Optional launch argument: `--vault <DIR>` supplies the default vault root. A `vault_root` in a tool request overrides it. Without either value, the resolver detects the nearest ancestor `.obsidian` directory from `context_path`.
- Optional launch flag: `--diagnostics` enables structured JSON-lines operational events on stderr. Diagnostics are disabled by default. Events cover server lifecycle, request processing/outcomes/failures, and cache reuse/refresh/watcher outcomes.
- The process remains alive for multiple tool calls and owns its vault cache until shutdown.

When `--diagnostics` is present, `structured-logger` 1.0.5 emits JSON-lines records to stderr through its synchronous JSON writer. Configure it with `default-features = false` to disable panic-message/backtrace logging, an explicit `INFO` level, a fixed MCP diagnostics target routed to stderr, and a sink as the default writer for all other targets. Do not initialize the logger when the flag is absent.

Each record includes the logger envelope (`timestamp`, `level`, `target`, and static `message`) plus structured diagnostic fields: `event`, optional process-local `request_seq`, optional resolver `status`, optional fixed `failure_kind`, and optional `duration_ms`. The event values are `server_started`, `server_stopped`, `request_started`, `request_completed`, `request_failed`, `cache_hit`, `cache_miss`, `cache_refresh_started`, `cache_refreshed`, `cache_refresh_failed`, and `watcher_unavailable`. Failure kinds are `invalid_arguments`, `root_selection`, `cache_refresh`, `watch_setup`, `protocol`, `transport`, and `internal`. A resolver result with status `error` is still a `request_completed` event; `request_failed` indicates the request did not produce a resolver result. Diagnostic records MUST NOT contain link text, context/vault paths, note contents, serialized tool arguments, or raw error strings. Messages and targets MUST remain fixed and MUST NOT interpolate untrusted data. Diagnostics MUST NOT be written to stdout or added to tool responses. When `--diagnostics` is absent, no optional diagnostic records are emitted.

## Tool: `resolve_obsidian_link`

**Description**: Resolve an Obsidian link using the existing Obsidian Link Resolver. Provide the raw link and the path of the file containing it. A simple target line/range is sufficient when you need one point target; request `with_emplacement` when reading a precise section or shard, since the result includes the containing heading stack, section range, and structured block when available. The tool preserves the resolver's resolved, unresolved, sub-target-not-found, ambiguous, and error outcomes.

### Input

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["link", "context_path"],
  "properties": {
    "link": { "type": "string", "minLength": 1 },
    "context_path": { "type": "string", "minLength": 1 },
    "vault_root": { "type": "string", "minLength": 1 },
    "with_emplacement": { "type": "boolean", "default": false }
  }
}
```

`link` and `context_path` are required. `vault_root` and `with_emplacement` are optional. Relative/absolute path interpretation and vault containment checks follow the current resolver.

### Output

`structuredContent` is one `ResolutionTarget` object validated against `../../001-link-resolver/contracts/result.schema.json`. A text content item contains that same record serialized as compact JSON for clients that consume text. No wrapper-only status or nondeterministic fields are added to the result record.

The five resolver statuses are normal tool outcomes. `unresolved`, `sub_target_not_found`, `ambiguous`, and resolver-level `error` include their existing reason/candidate fields as defined by the schema. Malformed tool input and MCP protocol/transport failures use MCP error handling instead.

### Stable Semantics

- Link parsing, note matching, path-qualified references, same-file links, heading paths, block IDs, and attachments are delegated to the existing resolver.
- Paths remain vault-relative and forward-slash normalized. Candidate order and primary result fields remain deterministic.
- `with_emplacement` requests the existing structured emplacement. Attachments do not gain heading/emplacement data.
- Repeated requests for the same canonical vault root reuse the process cache. Watch events mark it dirty for refresh before the next resolution; a background full rescan runs at least every 60 seconds per cached root, independently of requests. If watcher setup is unavailable, periodic scans still run. A failed refresh returns an error rather than a result from a known-stale index.
- Identical requests produce identical primary result records while the relevant vault contents remain unchanged. Results may change after relevant filesystem changes are reflected by cache invalidation or periodic refresh.

## Client Registration Shape

Client configuration uses its standard local stdio server registration mechanism and launches the installed `obsidian-link-resolver-mcp` executable. The exact JSON nesting is client-specific; the command and arguments are the executable path and optional `--vault` default. The request-level `vault_root` remains available for clients that reuse one process across vaults.
